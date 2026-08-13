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
    s.files_tracked = one(conn, "SELECT count(*) FROM origins WHERE line_no > 0")?;
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

// ── Timeline queries ─────────────────────────────────────────────────────────
//
// These back the CLI's `day` view and, later, the app's Stream view. They read
// only derived tables, so they stay cheap regardless of archive size.

#[derive(Debug)]
pub struct BlockRow {
    pub id: i64,
    pub project: String,
    pub records: i64,
    pub started_ms: i64,
    pub ended_ms: i64,
    pub sessions: i64,
    pub commits: i64,
    pub file_changes: i64,
}

#[derive(Debug)]
pub struct SessionRow {
    pub id: String,
    pub title: Option<String>,
    pub started_ms: Option<i64>,
    pub ended_ms: Option<i64>,
    pub prompts: i64,
    pub tool_calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub models: String,
    pub has_transcript: bool,
}

#[derive(Debug)]
pub struct CommitRow {
    pub sha: String,
    pub ts_ms: i64,
    pub message: String,
    pub insertions: i64,
    pub deletions: i64,
    pub unreachable: bool,
    pub tier: Option<String>,
    pub session_id: Option<String>,
}

/// Blocks overlapping the range, oldest first.
///
/// `limit` keeps the most recent N, because a truncated view of history should
/// end at the far past rather than stop before yesterday. It applies here rather
/// than at the caller: every block kept costs three further queries, so trimming
/// afterwards would pay the whole price anyway.
pub fn blocks_between(
    conn: &Connection,
    from_ms: i64,
    to_ms: i64,
    limit: Option<usize>,
) -> Result<Vec<BlockRow>> {
    let mut stmt = conn.prepare(
        "SELECT b.id, p.path, b.started_ms, b.ended_ms, b.sessions, b.commits,
                b.file_changes, b.records
           FROM blocks b JOIN projects p ON p.id = b.project_id
          WHERE b.started_ms < ?2 AND b.ended_ms >= ?1
          ORDER BY b.started_ms DESC
          LIMIT ?3",
    )?;
    let cap = limit.unwrap_or(usize::MAX).min(i64::MAX as usize) as i64;
    let rows = stmt.query_map([from_ms, to_ms, cap], |r| {
        Ok(BlockRow {
            id: r.get(0)?,
            project: r.get(1)?,
            started_ms: r.get(2)?,
            ended_ms: r.get(3)?,
            sessions: r.get(4)?,
            commits: r.get(5)?,
            file_changes: r.get(6)?,
            records: r.get(7)?,
        })
    })?;
    let mut out = rows.collect::<Result<Vec<_>, _>>()?;
    out.reverse();
    Ok(out)
}

pub fn sessions_in_block(conn: &Connection, block_id: i64) -> Result<Vec<SessionRow>> {
    let mut stmt = conn.prepare(
        "SELECT session_id, title, started_ms, ended_ms, prompts, tool_calls,
                input_tokens, output_tokens, models, has_transcript
           FROM sessions WHERE block_id = ?1 ORDER BY started_ms",
    )?;
    let rows = stmt.query_map([block_id], |r| {
        Ok(SessionRow {
            id: r.get(0)?,
            title: r.get(1)?,
            started_ms: r.get(2)?,
            ended_ms: r.get(3)?,
            prompts: r.get(4)?,
            tool_calls: r.get(5)?,
            input_tokens: r.get(6)?,
            output_tokens: r.get(7)?,
            models: r.get(8)?,
            has_transcript: r.get::<_, i64>(9)? == 1,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn commits_in_block(conn: &Connection, block_id: i64) -> Result<Vec<CommitRow>> {
    let mut stmt = conn.prepare(
        "SELECT c.sha, c.ts_ms, c.message, c.insertions, c.deletions, c.unreachable,
                l.tier, l.session_id
           FROM commits c LEFT JOIN commit_links l ON l.sha = c.sha
          WHERE c.block_id = ?1 ORDER BY c.ts_ms",
    )?;
    let rows = stmt.query_map([block_id], |r| {
        Ok(CommitRow {
            sha: r.get(0)?,
            ts_ms: r.get(1)?,
            message: r.get(2)?,
            insertions: r.get(3)?,
            deletions: r.get(4)?,
            unreachable: r.get::<_, i64>(5)? == 1,
            tier: r.get(6)?,
            session_id: r.get(7)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// File changes inside a block's project and span. File changes are not assigned
/// a block id: they are evidence about a file, not an event in a conversation.
pub fn file_changes_in_block(conn: &Connection, block_id: i64) -> Result<Vec<(String, i64, String)>> {
    let mut stmt = conn.prepare(
        "SELECT json_extract(r.json,'$.path'), r.ts_ms, json_extract(r.json,'$.state')
           FROM raw_records r
           JOIN blocks b ON b.id = ?1 AND b.project_id = r.project_id
          WHERE r.kind = 'file_change'
            AND r.ts_ms BETWEEN b.started_ms AND b.ended_ms
          ORDER BY r.ts_ms",
    )?;
    let rows = stmt.query_map([block_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Tracked time over a range.
///
/// Two figures, because they answer different questions and only one of them is
/// wall-clock. Blocks on different projects legitimately overlap — switching
/// between two repos in the same half hour produces two blocks covering the same
/// minutes — so summing their durations overstates the day. `elapsed_ms` is the
/// union of the intervals, which is time actually spent; `project_ms` is the sum,
/// which is effort distributed across projects and can exceed the day.
#[derive(Debug, Default, Clone, Copy)]
pub struct RangeSummary {
    pub elapsed_ms: i64,
    pub project_ms: i64,
    pub blocks: i64,
    pub projects: i64,
}

pub fn range_summary(conn: &Connection, from_ms: i64, to_ms: i64) -> Result<RangeSummary> {
    let mut stmt = conn.prepare(
        "SELECT started_ms, ended_ms, project_id FROM blocks
          WHERE started_ms < ?2 AND ended_ms >= ?1
          ORDER BY started_ms",
    )?;
    let rows = stmt.query_map([from_ms, to_ms], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;

    let mut summary = RangeSummary::default();
    let mut projects = std::collections::HashSet::new();
    let mut merged: Option<(i64, i64)> = None;

    for row in rows {
        let (start, end, project_id) = row?;
        let start = start.max(from_ms);
        let end = end.min(to_ms);
        if end < start {
            continue;
        }
        summary.blocks += 1;
        summary.project_ms += end - start;
        projects.insert(project_id);

        merged = match merged {
            Some((ms, me)) if start <= me => Some((ms, me.max(end))),
            Some((ms, me)) => {
                summary.elapsed_ms += me - ms;
                Some((start, end))
            }
            None => Some((start, end)),
        };
    }
    if let Some((ms, me)) = merged {
        summary.elapsed_ms += me - ms;
    }
    summary.projects = projects.len() as i64;

    Ok(summary)
}
