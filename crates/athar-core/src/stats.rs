use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

/// What kind of record backs a block's span, strongest present first.
///
/// A block's start and end are the timestamps of its first and last record, so
/// *what its width means* changes with what those records are: a session
/// brackets continuous work, commits are exact points with the idle-gap rule
/// filling between them, and a file save is a point whose coverage is a floor.
/// Drawn identically, a three-hour conversation and two file saves make the same
/// claim, and only one of them has earned it.
///
/// A session counts whether or not its transcript survived. Prompt timestamps
/// are exact, so the span is evidenced even where the content is gone — that
/// absence is a different axis, and `prompts only` already carries it.
pub fn evidence_of(sessions: i64, commits: i64, file_changes: i64) -> &'static str {
    if sessions > 0 {
        "sessions"
    } else if commits > 0 {
        "commits"
    } else if file_changes > 0 {
        "saves"
    } else {
        // Records the timeline does not itemise — harness state, prompt history.
        // Real, and not a claim about any of the three above.
        "bare"
    }
}

/// Counted time split by what evidences it.
///
/// This splits `project_ms` — the sum across projects — and not `elapsed_ms`,
/// which merges overlapping blocks so that a per-class split of it would total
/// more than the whole. Every block has exactly one class, so these four add up
/// to `project_ms` exactly, and the digest can print them without a caveat.
#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct EvidenceMs {
    pub sessions: i64,
    pub commits: i64,
    pub saves: i64,
    pub bare: i64,
}

impl EvidenceMs {
    fn add(&mut self, class: &str, ms: i64) {
        match class {
            "sessions" => self.sessions += ms,
            "commits" => self.commits += ms,
            "saves" => self.saves += ms,
            _ => self.bare += ms,
        }
    }
}

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
/// A `project_id IN (…)` fragment, or nothing when the filter is absent. The ids
/// come from our own table and are integers, so interpolation carries no injection
/// surface; rusqlite cannot bind a list without the `rarray` feature.
pub fn in_scope(column: &str, scope: Option<&[i64]>) -> String {
    match scope {
        None => String::new(),
        // An empty set is a real answer: a filter that admits no project should
        // report zero, not everything.
        Some(ids) if ids.is_empty() => format!("AND {column} IS NULL AND 1 = 0"),
        Some(ids) => {
            let list: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
            format!("AND {column} IN ({})", list.join(","))
        }
    }
}

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

/// One block, by id.
///
/// The Lanes view carries bars rather than blocks, so selecting one had nothing
/// in hand and re-ran the whole range query to find a single row — the full
/// block table, on the widest range, for one click. `None` is a real answer: a
/// rebuild renumbers derived rows, so a selection can outlive the block it names.
pub fn block(conn: &Connection, id: i64) -> Result<Option<BlockRow>> {
    let mut stmt = conn.prepare(
        "SELECT b.id, p.path, b.started_ms, b.ended_ms, b.sessions, b.commits,
                b.file_changes, b.records
           FROM blocks b JOIN projects p ON p.id = b.project_id
          WHERE b.id = ?1",
    )?;
    let mut rows = stmt.query_map([id], |r| {
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
    Ok(rows.next().transpose()?)
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
    pub by_evidence: EvidenceMs,
}

/// `scope` is a pre-resolved list of project ids, or `None` for everything.
/// Category is derived from the configured roots rather than stored, so it cannot
/// be a `WHERE` clause — resolving both filters to concrete ids once is what keeps
/// every figure in the digest on the same footing as the timeline beneath it.
pub fn range_summary(
    conn: &Connection,
    from_ms: i64,
    to_ms: i64,
    scope: Option<&[i64]>,
) -> Result<RangeSummary> {
    let mut stmt = conn.prepare(&format!(
        "SELECT started_ms, ended_ms, project_id, sessions, commits, file_changes
           FROM blocks
          WHERE started_ms < ?2 AND ended_ms >= ?1 {}
          ORDER BY started_ms",
        in_scope("project_id", scope)
    ))?;
    let rows = stmt.query_map([from_ms, to_ms], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
            evidence_of(r.get(3)?, r.get(4)?, r.get(5)?),
        ))
    })?;

    let mut summary = RangeSummary::default();
    let mut projects = std::collections::HashSet::new();
    let mut merged: Option<(i64, i64)> = None;

    for row in rows {
        let (start, end, project_id, class) = row?;
        let start = start.max(from_ms);
        let end = end.min(to_ms);
        if end < start {
            continue;
        }
        summary.blocks += 1;
        summary.project_ms += end - start;
        // Clamped to the range first, so the split reports the time this range
        // actually holds rather than the whole of a block that overhangs it.
        summary.by_evidence.add(class, end - start);
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
