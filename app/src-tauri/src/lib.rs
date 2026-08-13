//! lore's desktop shell.
//!
//! The window is a reader. The collector process owns every write; this opens the
//! same SQLite file read-only, so a scan can never be blocked by the app being
//! open, and the app can never corrupt the archive.

use std::sync::Mutex;

use lore_core::{agent, api, config::Config, paths};
use rusqlite::Connection;
use serde::Serialize;
use tauri::{Manager, State};

/// Held open for the process lifetime: the archive is local and read-only, so a
/// single connection behind a mutex is simpler and faster than a pool.
struct Archive {
    conn: Mutex<Option<Connection>>,
    config: Config,
}

/// Every command returns this, so the UI can render a real failure instead of a
/// silent empty state.
#[derive(Serialize)]
struct Failure {
    message: String,
}

impl From<anyhow::Error> for Failure {
    fn from(err: anyhow::Error) -> Self {
        Failure {
            message: format!("{err:#}"),
        }
    }
}

type Reply<T> = Result<T, Failure>;

fn with_conn<T, F>(archive: &Archive, f: F) -> Reply<T>
where
    F: FnOnce(&Connection, &Config) -> anyhow::Result<T>,
{
    let guard = archive.conn.lock().map_err(|_| Failure {
        message: "archive lock poisoned".into(),
    })?;
    let conn = guard.as_ref().ok_or_else(|| Failure {
        message: "no archive yet — run `lore scan` to build one".into(),
    })?;
    f(conn, &archive.config).map_err(Into::into)
}

#[tauri::command]
fn status(archive: State<Archive>) -> Reply<api::CollectorStatus> {
    with_conn(&archive, api::status)
}

#[tauri::command]
fn projects(archive: State<Archive>) -> Reply<Vec<api::ProjectInfo>> {
    with_conn(&archive, api::projects)
}

#[tauri::command]
fn summary(archive: State<Archive>, from_ms: i64, to_ms: i64) -> Reply<api::Summary> {
    with_conn(&archive, |c, _| api::summary(c, from_ms, to_ms))
}

#[tauri::command]
fn timeline(
    archive: State<Archive>,
    from_ms: i64,
    to_ms: i64,
    project: Option<String>,
    category: Option<String>,
    limit: Option<usize>,
) -> Reply<Vec<api::BlockDetail>> {
    with_conn(&archive, |c, cfg| {
        api::timeline(
            c,
            cfg,
            from_ms,
            to_ms,
            project.as_deref(),
            category.as_deref(),
            limit,
        )
    })
}

#[tauri::command]
fn lanes(
    archive: State<Archive>,
    from_ms: i64,
    to_ms: i64,
    category: Option<String>,
) -> Reply<Vec<api::Lane>> {
    with_conn(&archive, |c, cfg| {
        api::lanes(c, cfg, from_ms, to_ms, category.as_deref())
    })
}

#[tauri::command]
fn commit_files(archive: State<Archive>, sha: String) -> Reply<Vec<api::CommitFile>> {
    with_conn(&archive, |c, _| api::commit_files(c, &sha))
}

#[tauri::command]
fn session(archive: State<Archive>, id: String) -> Reply<Option<api::SessionDetail>> {
    with_conn(&archive, |c, cfg| api::session(c, cfg, &id))
}

/// The configuration, as the settings surface reads and writes it.
#[tauri::command]
fn read_config() -> Reply<Config> {
    Config::load().map_err(Into::into)
}

#[tauri::command]
fn write_config(config: Config) -> Reply<Config> {
    config.validate()?;
    config.save_over(&paths::config_file()?)?;
    Ok(config)
}

#[tauri::command]
fn agent_state(archive: State<Archive>) -> Reply<AgentView> {
    let config = Config::load().unwrap_or_default();
    let s = agent::status(&config)?;
    // Compare recorded revisions, not binaries: the window and the collector are
    // different executables, so `current_exe` can never match.
    let written_by = archive
        .conn
        .lock()
        .ok()
        .and_then(|g| g.as_ref().and_then(lore_core::db::collector_revision));
    let stale = written_by
        .as_deref()
        .map(|v| v != lore_core::COLLECTOR_REVISION)
        .unwrap_or(true);
    Ok(AgentView {
        installed: s.installed,
        loaded: s.loaded,
        binary_stale: stale,
        log: s.log.to_string_lossy().to_string(),
        config_path: paths::config_file()?.to_string_lossy().to_string(),
        db_path: paths::db_file()?.to_string_lossy().to_string(),
    })
}

#[derive(Serialize)]
struct AgentView {
    installed: bool,
    loaded: bool,
    binary_stale: bool,
    log: String,
    config_path: String,
    db_path: String,
}

#[tauri::command]
fn install_agent() -> Reply<()> {
    let config = Config::load()?;
    agent::install(&config)?;
    Ok(())
}

/// Runs the collector as a separate process.
///
/// The window never writes to the archive itself — it asks the collector binary
/// to, which keeps the single-writer rule intact even when the request comes from
/// a button.
#[tauri::command]
fn run_collector(action: String) -> Reply<String> {
    let binary = paths::installed_binary()?;
    if !binary.exists() {
        return Err(Failure {
            message: "the collector is not installed — install the agent first".into(),
        });
    }
    let verb = match action.as_str() {
        "scan" => "scan",
        "rebuild" => "rebuild",
        other => {
            return Err(Failure {
                message: format!("unknown action: {other}"),
            })
        }
    };
    let out = std::process::Command::new(&binary)
        .arg(verb)
        .output()
        .map_err(|e| Failure {
            message: format!("running {verb}: {e}"),
        })?;
    if !out.status.success() {
        return Err(Failure {
            message: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config = Config::load().unwrap_or_default();
            // A missing archive is an empty state, not a startup failure: the
            // window should open and say what to do about it.
            let conn = paths::db_file()
                .ok()
                .filter(|p| p.exists())
                .and_then(|p| api::open_readonly(&p).ok());
            app.manage(Archive {
                conn: Mutex::new(conn),
                config,
            });
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            status,
            projects,
            summary,
            timeline,
            lanes,
            session,
            commit_files,
            read_config,
            write_config,
            agent_state,
            install_agent,
            run_collector
        ])
        .run(tauri::generate_context!())
        .expect("error while running lore");
}
