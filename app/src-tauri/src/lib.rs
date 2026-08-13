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
) -> Reply<Vec<api::BlockDetail>> {
    with_conn(&archive, |c, cfg| {
        api::timeline(c, cfg, from_ms, to_ms, project.as_deref(), category.as_deref())
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
fn session(archive: State<Archive>, id: String) -> Reply<Option<api::SessionDetail>> {
    with_conn(&archive, |c, cfg| api::session(c, cfg, &id))
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
        .invoke_handler(tauri::generate_handler![
            status, projects, summary, timeline, lanes, session
        ])
        .run(tauri::generate_context!())
        .expect("error while running lore");
}
