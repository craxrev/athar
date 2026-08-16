use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use crate::paths;

const SCHEMA: &str = include_str!("schema.sql");

/// An origin athar has read, and where it stopped.
#[derive(Debug, Clone, Copy)]
pub struct OriginCursor {
    pub id: i64,
    pub inode: Option<u64>,
    pub size: u64,
    pub mtime_ms: i64,
    pub byte_offset: u64,
    pub line_no: u64,
}

/// Opens the collector's database. The collector is the sole writer; the app
/// opens the same file read-only.
pub fn open_writable(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating data directory {}", parent.display()))?;
    }
    let conn =
        Connection::open(path).with_context(|| format!("opening database at {}", path.display()))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    // Records are removed only by cascade from `origins`, when a file was rotated.
    conn.pragma_update(None, "foreign_keys", "ON")?;
    drop_outdated_derived(&conn)?;
    conn.execute_batch(SCHEMA).context("applying schema")?;
    Ok(conn)
}

/// Drops a derived table whose shape has changed, so the schema below recreates
/// it and the next rebuild refills it. Derived tables are projections; replacing
/// one costs a rebuild, and migrating one would be pretending it held truth.
fn drop_outdated_derived(conn: &Connection) -> Result<()> {
    let needs_drop = |table: &str, column: &str| -> Result<bool> {
        let exists: i64 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name = ?1",
            [table],
            |r| r.get(0),
        )?;
        if exists == 0 {
            return Ok(false);
        }
        let has_column: i64 = conn.query_row(
            &format!("SELECT count(*) FROM pragma_table_info('{table}') WHERE name = ?1"),
            [column],
            |r| r.get(0),
        )?;
        Ok(has_column == 0)
    };

    if needs_drop("commit_files", "added")? {
        conn.execute("DROP TABLE commit_files", [])?;
    }
    Ok(())
}

pub fn open_default() -> Result<Connection> {
    open_writable(&paths::db_file()?)
}

/// Returns the origin's row, creating it on first sight.
pub fn origin_cursor(conn: &Connection, source: &str, path: &str) -> Result<OriginCursor> {
    conn.execute(
        "INSERT OR IGNORE INTO origins (source, path) VALUES (?1, ?2)",
        (source, path),
    )?;
    let cursor = conn.query_row(
        "SELECT id, inode, size, mtime_ms, byte_offset, line_no
           FROM origins WHERE source = ?1 AND path = ?2",
        (source, path),
        |r| {
            Ok(OriginCursor {
                id: r.get(0)?,
                inode: r.get::<_, Option<i64>>(1)?.map(|v| v as u64),
                size: r.get::<_, i64>(2)? as u64,
                mtime_ms: r.get(3)?,
                byte_offset: r.get::<_, i64>(4)? as u64,
                line_no: r.get::<_, i64>(5)? as u64,
            })
        },
    )?;
    Ok(cursor)
}

pub fn set_origin_cursor(conn: &Connection, cursor: &OriginCursor) -> Result<()> {
    conn.execute(
        "UPDATE origins
            SET inode = ?2, size = ?3, mtime_ms = ?4,
                byte_offset = ?5, line_no = ?6, updated_at_ms = ?7
          WHERE id = ?1",
        (
            cursor.id,
            cursor.inode.map(|v| v as i64),
            cursor.size as i64,
            cursor.mtime_ms,
            cursor.byte_offset as i64,
            cursor.line_no as i64,
            chrono::Utc::now().timestamp_millis(),
        ),
    )?;
    Ok(())
}

/// Drops everything archived from one origin and resets its cursor. Used only
/// when a file was rotated or truncated, which invalidates its line numbering.
pub fn forget_origin(conn: &Connection, origin_id: i64) -> Result<usize> {
    let removed = conn.execute("DELETE FROM raw_records WHERE origin_id = ?1", [origin_id])?;
    conn.execute(
        "UPDATE origins SET byte_offset = 0, line_no = 0 WHERE id = ?1",
        [origin_id],
    )?;
    Ok(removed)
}

/// A scan or rebuild in progress, as recorded by the process doing it.
#[derive(Debug, Clone)]
pub struct Run {
    pub action: String,
    pub started_ms: i64,
    pub pid: i32,
}

/// Claims the right to run, or reports the collector that already holds it.
///
/// Returns `Some(existing)` when another collector is working, and the caller
/// should stop. The window scans on its own timer, its buttons can fire one, and
/// a terminal can too — and two collectors against one archive means a blocked
/// writer at best.
///
/// The check and the claim share one immediate transaction, so two collectors
/// starting together cannot both see an idle archive.
pub fn claim_run(conn: &mut Connection, action: &str) -> Result<Option<Run>> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    if let Some(existing) = current_run(&tx) {
        return Ok(Some(existing));
    }
    mark_run_start(&tx, action)?;
    tx.commit()?;
    Ok(None)
}

/// Records that a collector run has begun.
///
/// The process doing the work is the only thing that knows it is happening — a
/// window that spawned it, another window, and a terminal all need the same
/// answer. It goes in the archive, which the window reads read-only.
fn mark_run_start(conn: &Connection, action: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    let pid = std::process::id() as i64;
    for (key, value) in [
        ("run_action", action.to_string()),
        ("run_started_ms", now.to_string()),
        // Presence is what marks a run open, not a comparison of timestamps: a run
        // that starts in the same millisecond the last one ended is indistinguishable
        // under ordering, and would read as idle while it worked.
        ("run_open_pid", pid.to_string()),
    ] {
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;
    }
    Ok(())
}

pub fn mark_run_end(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('run_finished_ms', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [chrono::Utc::now().timestamp_millis().to_string()],
    )?;
    conn.execute("DELETE FROM meta WHERE key = 'run_open_pid'", [])?;
    Ok(())
}

/// The run happening right now, if there is one.
///
/// A collector killed mid-scan never writes its finish, so the mark alone would
/// claim a scan forever. The recorded process is checked for life before the mark
/// is believed.
pub fn current_run(conn: &Connection) -> Option<Run> {
    let value = |key: &str| -> Option<String> {
        conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .ok()
    };
    // No open row, no run. The row is written at the start and deleted at the end,
    // so its absence is unambiguous where a timestamp comparison was not.
    let pid: i32 = value("run_open_pid")?.parse().ok()?;
    if !process_alive(pid) {
        return None;
    }
    Some(Run {
        action: value("run_action").unwrap_or_else(|| "scan".into()),
        started_ms: value("run_started_ms")?.parse().unwrap_or(0),
        pid,
    })
}

/// Signal 0 tests for a process without touching it.
fn process_alive(pid: i32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Interns a project path, returning its id.
pub fn project_id(conn: &Connection, path: &str) -> Result<i64> {
    if let Some(id) = conn
        .query_row("SELECT id FROM projects WHERE path = ?1", [path], |r| {
            r.get::<_, i64>(0)
        })
        .optional()?
    {
        return Ok(id);
    }
    conn.execute("INSERT OR IGNORE INTO projects (path) VALUES (?1)", [path])?;
    Ok(conn.query_row("SELECT id FROM projects WHERE path = ?1", [path], |r| {
        r.get(0)
    })?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> Connection {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "athar-db-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        open_writable(&dir.join("athar.db")).unwrap()
    }

    #[test]
    fn a_second_collector_is_refused_while_one_is_working() {
        let mut conn = temp_db();

        assert!(claim_run(&mut conn, "scan").unwrap().is_none(), "first claim");

        // The claim records this very process, which is alive, so the second
        // caller sees a genuinely running collector rather than a stale mark.
        let refused = claim_run(&mut conn, "scan").unwrap().expect("second claim");
        assert_eq!(refused.action, "scan");
        assert_eq!(refused.pid, std::process::id() as i32);

        mark_run_end(&conn).unwrap();
        assert!(
            claim_run(&mut conn, "rebuild").unwrap().is_none(),
            "a finished run must not block the next one"
        );
    }

    /// The window reads this to say when a scan last completed. A run in flight
    /// must not erase that answer, and neither must one that dies without
    /// finishing — otherwise the footer reports "never scanned" on a live archive.
    #[test]
    fn an_open_run_keeps_the_previous_finish() {
        let mut conn = temp_db();

        claim_run(&mut conn, "scan").unwrap();
        mark_run_end(&conn).unwrap();
        let first: i64 = conn
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM meta WHERE key='run_finished_ms'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(first > 0, "a completed run records its finish");

        // A second run opens; the first run's finish is still the last one known.
        mark_run_start(&conn, "scan").unwrap();
        let during: i64 = conn
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM meta WHERE key='run_finished_ms'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(during, first, "an open run must not erase the last finish");
        assert!(current_run(&conn).is_some(), "and it still reads as running");
    }

    #[test]
    fn a_run_whose_process_died_does_not_block_the_next() {
        let conn = temp_db();
        mark_run_start(&conn, "scan").unwrap();
        // PID 1 is launchd and always alive, so a pid that cannot exist is used:
        // the mark stays open, and only the liveness check can clear it.
        conn.execute(
            "UPDATE meta SET value = '2147483647' WHERE key = 'run_open_pid'",
            [],
        )
        .unwrap();
        assert!(
            current_run(&conn).is_none(),
            "an open mark from a dead process must not claim a run forever"
        );
    }
}
