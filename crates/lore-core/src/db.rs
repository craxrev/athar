use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use crate::paths;

const SCHEMA: &str = include_str!("schema.sql");

/// An origin lore has read, and where it stopped.
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

/// Records which collector build last wrote the archive.
pub fn stamp_collector(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('collector_revision', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [crate::COLLECTOR_REVISION],
    )?;
    Ok(())
}

pub fn collector_revision(conn: &Connection) -> Option<String> {
    conn.query_row(
        "SELECT value FROM meta WHERE key = 'collector_revision'",
        [],
        |r| r.get(0),
    )
    .ok()
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
