use anyhow::Result;
use rusqlite::Connection;

#[derive(Debug, Default)]
pub struct Archive {
    pub records: i64,
    pub sessions: i64,
    pub projects: i64,
    pub files_tracked: i64,
    pub truncated: i64,
    pub earliest_ms: Option<i64>,
    pub latest_ms: Option<i64>,
    pub bytes_original: i64,
    pub bytes_stored: i64,
}

pub fn archive(conn: &Connection) -> Result<Archive> {
    let mut s = Archive::default();

    s.records = one(conn, "SELECT count(*) FROM raw_records")?;
    s.sessions = one(
        conn,
        "SELECT count(DISTINCT session_id) FROM raw_records WHERE session_id IS NOT NULL",
    )?;
    s.projects = one(conn, "SELECT count(*) FROM projects")?;
    s.files_tracked = one(conn, "SELECT count(*) FROM files WHERE line_no > 0")?;
    s.truncated = one(conn, "SELECT count(*) FROM raw_records WHERE truncated = 1")?;
    s.bytes_original = one(conn, "SELECT coalesce(sum(bytes_original), 0) FROM raw_records")?;
    s.bytes_stored = one(conn, "SELECT coalesce(sum(length(json)), 0) FROM raw_records")?;

    let (lo, hi): (Option<i64>, Option<i64>) = conn.query_row(
        "SELECT min(ts_ms), max(ts_ms) FROM raw_records WHERE ts_ms IS NOT NULL",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    s.earliest_ms = lo;
    s.latest_ms = hi;

    Ok(s)
}

pub fn by_kind(conn: &Connection) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT kind, count(*) FROM raw_records GROUP BY kind ORDER BY count(*) DESC",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Sessions started per day, which is how the timeline's density was measured.
pub fn sessions_per_day(conn: &Connection, limit: i64) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT day, count(*) FROM (
             SELECT session_id, date(min(ts_ms) / 1000, 'unixepoch', 'localtime') AS day
               FROM raw_records
              WHERE session_id IS NOT NULL AND ts_ms IS NOT NULL
              GROUP BY session_id
         )
         GROUP BY day ORDER BY day DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn one(conn: &Connection, sql: &str) -> Result<i64> {
    Ok(conn.query_row(sql, [], |r| r.get(0))?)
}
