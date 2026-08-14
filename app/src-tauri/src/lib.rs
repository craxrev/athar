//! lore's desktop shell.
//!
//! The window is a reader. The collector process owns every write; this opens the
//! same SQLite file read-only, so a scan can never be blocked by the app being
//! open, and the app can never corrupt the archive.

use std::sync::Mutex;

use lore_core::{api, config::Config, paths};
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

/// Just "is a collector working", cheap enough to ask often.
///
/// The full status counts every row in the archive, so it can only be polled
/// slowly — and a warm scan finishes in seconds, falling between two of those
/// polls. This reads four rows of `meta`, and only checks the recorded process
/// for life when the mark says a run is open, so an idle archive costs nothing.
#[tauri::command]
fn collector_run(archive: State<Archive>) -> Reply<Option<String>> {
    with_conn(&archive, |c, _| {
        Ok(lore_core::db::current_run(c).map(|r| r.action))
    })
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

/// The collector shipped with this window: its bundled sidecar, or the CLI
/// built beside it in a development tree.
///
/// Nothing is installed anywhere. The binary the window runs is the one it was
/// built with, so it can never be an older build than the window, and a machine
/// with no scheduler of its own is not a machine lore cannot archive on.
fn collector() -> Option<std::path::PathBuf> {
    lore_core::collector::beside(&std::env::current_exe().ok()?)
}

#[derive(Serialize)]
struct Paths {
    config_path: String,
    db_path: String,
}

/// Where the two files a user might want to look at actually live. No log among
/// them: the collector's output now comes back through the window that ran it.
#[tauri::command]
fn paths() -> Reply<Paths> {
    Ok(Paths {
        config_path: paths::config_file()?.to_string_lossy().to_string(),
        db_path: paths::db_file()?.to_string_lossy().to_string(),
    })
}

/// Runs the collector as a separate process.
///
/// The window never writes to the archive itself — it asks the collector binary
/// to, which keeps the single-writer rule intact even when the request comes from
/// a button.
///
/// `async` is load-bearing rather than stylistic: a synchronous command runs on
/// the main thread, so waiting here would freeze the window for the length of a
/// scan — minutes, on a run that finds real work.
#[tauri::command(async)]
fn run_collector(action: String) -> Reply<String> {
    let binary = collector().ok_or_else(|| Failure {
        message: "no collector ships with this build of the window".into(),
    })?;
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
    let builder = tauri::Builder::default();

    // Registered before anything else, as this plugin requires. A second launch
    // raises the window already open instead of opening another: two windows
    // share one config file, each holds its own copy in memory, and whichever
    // saves last overwrites the other's edits without either one noticing.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }));

    builder
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
            collector_run,
            projects,
            summary,
            timeline,
            lanes,
            session,
            commit_files,
            paths,
            read_config,
            write_config,
            run_collector
        ])
        .run(tauri::generate_context!())
        .expect("error while running lore");
}
