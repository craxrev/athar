//! Read-only queries backing the desktop app.
//!
//! Every function here reads derived tables and returns serializable values. The
//! collector is the sole writer; the app opens the same database read-only, so a
//! scan running in the background can never be blocked by the window being open.
//!
//! Coverage is uneven by design and these types carry that unevenness rather than
//! flattening it: a commit link knows whether it was witnessed or inferred, a
//! session knows whether its transcript still exists, and a file change knows
//! that its count is a floor.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;

use crate::config::Config;

#[derive(Debug, Serialize)]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub category: String,
    pub last_activity_ms: Option<i64>,
    pub blocks: i64,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub elapsed_ms: i64,
    pub project_ms: i64,
    pub blocks: i64,
    pub projects: i64,
    pub sessions: i64,
    pub commits: i64,
    pub file_changes: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    /// Share of touched files the AI wrote, as a percentage of files seen in the
    /// range. Absent when there is nothing to compare.
    pub ai_share: Option<f64>,
    /// `project_ms` split by what evidences each block. Sums to `project_ms`.
    pub by_evidence: crate::stats::EvidenceMs,
}

#[derive(Debug, Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub started_ms: Option<i64>,
    pub ended_ms: Option<i64>,
    pub prompts: i64,
    pub replies: i64,
    pub tool_calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub models: Vec<String>,
    pub files_written: i64,
    /// False when only the prompt history survives; Claude Code deleted the rest.
    pub has_transcript: bool,
    /// When this session was first seen inside the block being listed. A resumed
    /// session's `started_ms` can be days earlier, so that is the wrong value to
    /// order a block's contents by.
    pub first_seen_ms: Option<i64>,
    /// True when this block is not where the session began. A conversation that
    /// pauses past the idle gap spans several blocks; repeating its full row in
    /// each one reads as several conversations, so later blocks mark it as a
    /// continuation instead.
    pub continued: bool,
}

#[derive(Debug, Serialize)]
pub struct CommitSummary {
    pub sha: String,
    pub short: String,
    pub ts_ms: i64,
    pub subject: String,
    pub insertions: i64,
    pub deletions: i64,
    pub file_count: i64,
    /// A commit no ref can reach. Git will collect it; athar already kept it.
    pub unreachable: bool,
    /// `certain` | `strong` | `weak` | absent.
    pub tier: Option<String>,
    pub session_id: Option<String>,
    pub shared_files: i64,
}

#[derive(Debug, Serialize)]
pub struct CommitFile {
    /// Relative to the repository, which is how a commit names its own files.
    pub path: String,
    pub name: String,
    pub added: Option<i64>,
    pub deleted: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct FileChangeSummary {
    pub path: String,
    pub ts_ms: i64,
    /// `dirty` | `untracked` | `no-repo`.
    pub state: String,
}

#[derive(Debug, Serialize)]
pub struct BlockDetail {
    pub id: i64,
    /// Archived records inside the span, including kinds the timeline does not
    /// itemise. Lets an otherwise empty block explain itself.
    pub records: i64,
    pub project_path: String,
    pub project: String,
    pub category: String,
    pub started_ms: i64,
    pub ended_ms: i64,
    pub sessions: Vec<SessionSummary>,
    pub commits: Vec<CommitSummary>,
    pub file_changes: Vec<FileChangeSummary>,
    /// The same stamp the lane bar carries, so the pane can say in a sentence
    /// what the bar said in a shape. Without it the evidence code was learnable
    /// only by hovering a bar.
    pub evidence: &'static str,
}

/// Re-exported so the one rule keeps one name. It lives in `stats` because the
/// block walk that splits the digest lives there, and `api` depends on `stats`
/// rather than the other way round — the alternative was a second copy of the
/// rule, which is the thing this stamp exists to prevent.
pub use crate::stats::evidence_of;

#[derive(Debug, Serialize)]
pub struct Bar {
    pub block_id: i64,
    pub started_ms: i64,
    pub ended_ms: i64,
    pub sessions: i64,
    pub commits: i64,
    pub file_changes: i64,
    /// Stamped here rather than derived where it is drawn, so the timeline and
    /// the digest can never classify the same block differently.
    pub evidence: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Lane {
    pub project_path: String,
    pub project: String,
    pub category: String,
    pub total_ms: i64,
    pub bars: Vec<Bar>,
}

#[derive(Debug, Serialize)]
pub struct Turn {
    pub role: String,
    pub ts_ms: Option<i64>,
    /// The turn as a tree of markdown nodes, parsed here so the window never
    /// builds markup out of archived text.
    pub blocks: Vec<crate::markdown::Block>,
    /// True when the archived text is a head of a longer original.
    pub truncated: bool,
    pub tools: Vec<ToolCall>,
}

#[derive(Debug, Serialize)]
pub struct ToolCall {
    pub name: String,
    /// A file path or the head of a command — whatever identifies the call.
    pub target: String,
    pub failed: bool,
}

#[derive(Debug, Serialize)]
pub struct SessionDetail {
    pub session: SessionSummary,
    pub project_path: String,
    pub project: String,
    pub category: String,
    pub files: Vec<TouchedFile>,
    pub commits: Vec<CommitSummary>,
    pub turns: Vec<Turn>,
}

#[derive(Debug, Serialize)]
pub struct TouchedFile {
    pub path: String,
    pub name: String,
    pub writes: i64,
    pub reads: i64,
}

#[derive(Debug, Serialize)]
pub struct CollectorStatus {
    /// When a collector last finished, whatever it found.
    ///
    /// Distinct from [`Self::last_archived_ms`]: a scan that reads every source
    /// and finds nothing new advances this and not that. Reporting the other one
    /// as "last scan" told you the collector had stopped running when it hadn't.
    pub last_scan_ms: Option<i64>,
    /// When something was last written into the archive.
    pub last_archived_ms: Option<i64>,
    pub records: i64,
    pub sessions: i64,
    pub commits: i64,
    pub file_changes: i64,
    pub origins: i64,
    pub earliest_ms: Option<i64>,
    pub latest_ms: Option<i64>,
    /// How often the window scans while it is open.
    pub scan_interval_mins: u64,
    /// `scan` or `rebuild` while a collector is working, whoever started it.
    pub running: Option<String>,
    pub roots: Vec<String>,
    /// Sessions whose transcript the source has already deleted, and which now
    /// exist only here.
    pub sessions_only_in_athar: i64,
}

/// Opens the archive read-only. A missing database is an error the caller shows
/// as an empty state, not a crash.
pub fn open_readonly(path: &Path) -> Result<Connection> {
    use rusqlite::OpenFlags;
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )?;
    Ok(conn)
}

fn leaf(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

fn category(config: &Config, path: &str) -> String {
    config
        .category_of(Path::new(path))
        .unwrap_or("uncategorized")
        .to_string()
}

pub fn projects(conn: &Connection, config: &Config) -> Result<Vec<ProjectInfo>> {
    let mut stmt = conn.prepare(
        "SELECT p.path, max(b.ended_ms), count(b.id)
           FROM projects p LEFT JOIN blocks b ON b.project_id = p.id
          GROUP BY p.path
          HAVING count(b.id) > 0
          ORDER BY max(b.ended_ms) DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        let path: String = r.get(0)?;
        Ok(ProjectInfo {
            name: leaf(&path),
            category: category(config, &path),
            last_activity_ms: r.get(1)?,
            blocks: r.get(2)?,
            path,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// The projects a filter admits, as ids, or `None` when nothing is narrowed.
///
/// Category is computed from the configured roots, so it cannot be expressed in
/// SQL. Resolving both filters to an id set once means every figure in the digest
/// is narrowed the same way — the alternative was a digest describing the whole
/// range above a timeline showing a slice of it.
fn scope_ids(
    conn: &Connection,
    config: &Config,
    project_filter: Option<&str>,
    category_filter: Option<&str>,
) -> Result<Option<Vec<i64>>> {
    if project_filter.is_none() && category_filter.is_none() {
        return Ok(None);
    }
    let mut stmt = conn.prepare("SELECT id, path FROM projects")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = Vec::new();
    for row in rows {
        let (id, path) = row?;
        if project_filter.is_some_and(|p| p != path) {
            continue;
        }
        if category_filter.is_some_and(|c| c != category(config, &path)) {
            continue;
        }
        out.push(id);
    }
    Ok(Some(out))
}

pub fn summary(
    conn: &Connection,
    config: &Config,
    from_ms: i64,
    to_ms: i64,
    project_filter: Option<&str>,
    category_filter: Option<&str>,
) -> Result<Summary> {
    let scope = scope_ids(conn, config, project_filter, category_filter)?;
    let scope = scope.as_deref();
    let base = crate::stats::range_summary(conn, from_ms, to_ms, scope)?;
    let within = |column: &str| crate::stats::in_scope(column, scope);

    let (sessions, input_tokens, output_tokens): (i64, i64, i64) = conn.query_row(
        &format!(
            "SELECT count(*), coalesce(sum(input_tokens),0), coalesce(sum(output_tokens),0)
               FROM sessions WHERE started_ms < ?2 AND ended_ms >= ?1 {}",
            within("project_id")
        ),
        [from_ms, to_ms],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;

    let commits: i64 = conn.query_row(
        &format!(
            "SELECT count(*) FROM commits WHERE ts_ms >= ?1 AND ts_ms < ?2 {}",
            within("project_id")
        ),
        [from_ms, to_ms],
        |r| r.get(0),
    )?;

    let file_changes: i64 = conn.query_row(
        &format!(
            "SELECT count(*) FROM raw_records
              WHERE kind = 'file_change' AND ts_ms >= ?1 AND ts_ms < ?2 {}",
            within("project_id")
        ),
        [from_ms, to_ms],
        |r| r.get(0),
    )?;

    // Files the AI wrote, against every distinct file seen changing in the range.
    //
    // The two sides are unioned rather than added. Added, a file that the
    // assistant wrote *and* that the filesystem later recorded counted twice in
    // the denominator — once on each side — so the printed share was not a share
    // of distinct files at all, and drifted lower the more the two sources agreed.
    let (ai_files, all_files): (i64, i64) = conn.query_row(
        &format!(
            "WITH ai AS (
                 SELECT DISTINCT sf.path AS path
                   FROM session_files sf JOIN sessions s ON s.session_id = sf.session_id
                  WHERE sf.writes > 0 AND s.started_ms < ?2 AND s.ended_ms >= ?1 {}
             ),
             human AS (
                 SELECT DISTINCT json_extract(json, '$.path') AS path
                   FROM raw_records
                  WHERE kind = 'file_change' AND ts_ms >= ?1 AND ts_ms < ?2 {}
             )
             SELECT (SELECT count(*) FROM ai),
                    (SELECT count(*) FROM (SELECT path FROM ai UNION SELECT path FROM human))",
            within("s.project_id"),
            within("project_id")
        ),
        [from_ms, to_ms],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let ai_share = if all_files > 0 {
        Some((ai_files as f64) * 100.0 / (all_files as f64))
    } else {
        None
    };

    Ok(Summary {
        elapsed_ms: base.elapsed_ms,
        project_ms: base.project_ms,
        blocks: base.blocks,
        projects: base.projects,
        sessions,
        commits,
        file_changes,
        input_tokens,
        output_tokens,
        ai_share,
        by_evidence: base.by_evidence,
    })
}

/// One block with everything that fell inside it.
///
/// Shared by the range query and the single-block one so the two can never
/// drift: a block selected from Lanes and the same block read from Stream are
/// the same object, assembled once.
fn detail_of(
    conn: &Connection,
    config: &Config,
    block: crate::stats::BlockRow,
) -> Result<BlockDetail> {
    Ok(BlockDetail {
        id: block.id,
        records: block.records,
        project: leaf(&block.project),
        category: category(config, &block.project),
        started_ms: block.started_ms,
        ended_ms: block.ended_ms,
        sessions: sessions_in_block(conn, block.id)?,
        commits: commits_in_block(conn, block.id)?,
        file_changes: file_changes_in_block(conn, block.id)?,
        evidence: evidence_of(block.sessions, block.commits, block.file_changes),
        project_path: block.project,
    })
}

/// A single block, for a selection made in a view that does not carry one.
pub fn block(conn: &Connection, config: &Config, id: i64) -> Result<Option<BlockDetail>> {
    match crate::stats::block(conn, id)? {
        Some(row) => Ok(Some(detail_of(conn, config, row)?)),
        None => Ok(None),
    }
}

/// The Stream view: blocks in range, each with everything that fell inside it.
pub fn timeline(
    conn: &Connection,
    config: &Config,
    from_ms: i64,
    to_ms: i64,
    project_filter: Option<&str>,
    category_filter: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<BlockDetail>> {
    let mut out = Vec::new();
    for block in crate::stats::blocks_between(conn, from_ms, to_ms, limit)? {
        // Matched on the full path: leaf names collide across trees, and two
        // projects called `profile-next` are not the same project.
        if project_filter.is_some_and(|p| p != block.project) {
            continue;
        }
        if category_filter.is_some_and(|c| c != category(config, &block.project)) {
            continue;
        }

        out.push(detail_of(conn, config, block)?);
    }
    Ok(out)
}

/// The Lanes view: one row per project, its blocks as bars along the range.
pub fn lanes(
    conn: &Connection,
    config: &Config,
    from_ms: i64,
    to_ms: i64,
    category_filter: Option<&str>,
) -> Result<Vec<Lane>> {
    let mut stmt = conn.prepare(
        "SELECT p.path, b.id, b.started_ms, b.ended_ms, b.sessions, b.commits, b.file_changes
           FROM blocks b JOIN projects p ON p.id = b.project_id
          WHERE b.started_ms < ?2 AND b.ended_ms >= ?1
          ORDER BY p.path, b.started_ms",
    )?;
    let rows = stmt.query_map([from_ms, to_ms], |r| {
        let (sessions, commits, file_changes): (i64, i64, i64) = (r.get(4)?, r.get(5)?, r.get(6)?);
        Ok((
            r.get::<_, String>(0)?,
            Bar {
                block_id: r.get(1)?,
                started_ms: r.get(2)?,
                ended_ms: r.get(3)?,
                sessions,
                commits,
                file_changes,
                evidence: evidence_of(sessions, commits, file_changes),
            },
        ))
    })?;

    let mut grouped: HashMap<String, Vec<Bar>> = HashMap::new();
    for row in rows {
        let (path, bar) = row?;
        grouped.entry(path).or_default().push(bar);
    }

    let mut out: Vec<Lane> = grouped
        .into_iter()
        .filter_map(|(path, bars)| {
            let cat = category(config, &path);
            if category_filter.is_some_and(|c| c != cat) {
                return None;
            }
            let total_ms = bars
                .iter()
                .map(|b| (b.ended_ms.min(to_ms) - b.started_ms.max(from_ms)).max(0))
                .sum();
            Some(Lane {
                project: leaf(&path),
                category: cat,
                total_ms,
                bars,
                project_path: path,
            })
        })
        .collect();

    // Grouped by category, then busiest first inside it. Sorting globally by
    // activity would reshuffle every lane whenever the range changes.
    out.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then(b.total_ms.cmp(&a.total_ms))
            .then(a.project.cmp(&b.project))
    });
    Ok(out)
}

fn session_row(conn: &Connection, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<Vec<SessionSummary>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params, |r| {
        let id: String = r.get(0)?;
        let title: Option<String> = r.get(1)?;
        let models: String = r.get(8)?;
        Ok(SessionSummary {
            title: title.filter(|t| !t.is_empty()).unwrap_or_else(|| "Untitled session".into()),
            started_ms: r.get(2)?,
            ended_ms: r.get(3)?,
            prompts: r.get(4)?,
            replies: r.get(5)?,
            tool_calls: r.get(6)?,
            input_tokens: r.get(7)?,
            models: models
                .split(',')
                .filter(|m| !m.is_empty())
                .map(str::to_string)
                .collect(),
            output_tokens: r.get(9)?,
            has_transcript: r.get::<_, i64>(10)? == 1,
            files_written: r.get(11)?,
            continued: r.get::<_, i64>(12).unwrap_or(0) == 1,
            first_seen_ms: r.get(13).unwrap_or(None),
            id,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

const SESSION_COLUMNS: &str = "
    s.session_id, s.title, s.started_ms, s.ended_ms, s.prompts, s.replies,
    s.tool_calls, s.input_tokens, s.models, s.output_tokens, s.has_transcript,
    (SELECT count(*) FROM session_files f
      WHERE f.session_id = s.session_id AND f.writes > 0)";

/// Adds the continuation flag, for the block-scoped query only.
const SESSION_COLUMNS_IN_BLOCK: &str = "
    s.session_id, s.title, s.started_ms, s.ended_ms, s.prompts, s.replies,
    s.tool_calls, s.input_tokens, s.models, s.output_tokens, s.has_transcript,
    (SELECT count(*) FROM session_files f
      WHERE f.session_id = s.session_id AND f.writes > 0),
    CASE WHEN s.started_ms < b.started_ms THEN 1 ELSE 0 END,
    (SELECT min(rr.ts_ms) FROM raw_records rr
      WHERE rr.session_id = s.session_id
        AND rr.ts_ms BETWEEN b.started_ms AND b.ended_ms)";

/// Sessions that overlap the block, not merely those that started in it.
///
/// A conversation with a long pause in the middle spans two blocks: the gap ends
/// the block, but the session carries on. Matching on `block_id` alone attributed
/// it to the first block only and left the rest of the day looking empty.
pub fn sessions_in_block(conn: &Connection, block_id: i64) -> Result<Vec<SessionSummary>> {
    session_row(
        conn,
        &format!(
            "SELECT {SESSION_COLUMNS_IN_BLOCK} FROM sessions s
               JOIN blocks b ON b.id = ?1
              WHERE s.project_id = b.project_id
                AND s.started_ms <= b.ended_ms
                AND s.ended_ms   >= b.started_ms
              ORDER BY s.started_ms"
        ),
        &[&block_id],
    )
}

pub fn commits_in_block(conn: &Connection, block_id: i64) -> Result<Vec<CommitSummary>> {
    let mut stmt = conn.prepare(
        "SELECT c.sha, c.ts_ms, c.message, c.insertions, c.deletions, c.file_count,
                c.unreachable, l.tier, l.session_id, coalesce(l.shared_files, 0)
           FROM commits c LEFT JOIN commit_links l ON l.sha = c.sha
          WHERE c.block_id = ?1 ORDER BY c.ts_ms",
    )?;
    let rows = stmt.query_map([block_id], |r| {
        let sha: String = r.get(0)?;
        let message: String = r.get(2)?;
        Ok(CommitSummary {
            short: sha.chars().take(7).collect(),
            ts_ms: r.get(1)?,
            subject: message.lines().next().unwrap_or("").to_string(),
            insertions: r.get(3)?,
            deletions: r.get(4)?,
            file_count: r.get(5)?,
            unreachable: r.get::<_, i64>(6)? == 1,
            tier: r.get(7)?,
            session_id: r.get(8)?,
            shared_files: r.get(9)?,
            sha,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn file_changes_in_block(conn: &Connection, block_id: i64) -> Result<Vec<FileChangeSummary>> {
    let mut stmt = conn.prepare(
        "SELECT json_extract(r.json,'$.path'), r.ts_ms, json_extract(r.json,'$.state')
           FROM raw_records r
           JOIN blocks b ON b.id = ?1 AND b.project_id = r.project_id
          WHERE r.kind = 'file_change'
            AND r.ts_ms BETWEEN b.started_ms AND b.ended_ms
          ORDER BY r.ts_ms",
    )?;
    let rows = stmt.query_map([block_id], |r| {
        Ok(FileChangeSummary {
            path: r.get::<_, Option<String>>(0)?.unwrap_or_default(),
            ts_ms: r.get(1)?,
            state: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// One session in full: its summary, what it touched, what it produced, and the
/// conversation itself reconstructed from the archive.
pub fn session(conn: &Connection, config: &Config, session_id: &str) -> Result<Option<SessionDetail>> {
    let mut found = session_row(
        conn,
        &format!("SELECT {SESSION_COLUMNS} FROM sessions s WHERE s.session_id = ?1"),
        &[&session_id],
    )?;
    let Some(summary) = found.pop() else {
        return Ok(None);
    };

    let project_path: String = conn
        .query_row(
            "SELECT coalesce(p.path, '') FROM sessions s
               LEFT JOIN projects p ON p.id = s.project_id
              WHERE s.session_id = ?1",
            [session_id],
            |r| r.get(0),
        )
        .unwrap_or_default();

    let mut files_stmt = conn.prepare(
        "SELECT path, writes, reads FROM session_files
          WHERE session_id = ?1 ORDER BY writes DESC, path",
    )?;
    let files = files_stmt
        .query_map([session_id], |r| {
            let path: String = r.get(0)?;
            Ok(TouchedFile {
                name: leaf(&path),
                writes: r.get(1)?,
                reads: r.get(2)?,
                path,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut commits_stmt = conn.prepare(
        "SELECT c.sha, c.ts_ms, c.message, c.insertions, c.deletions, c.file_count,
                c.unreachable, l.tier, l.session_id, l.shared_files
           FROM commit_links l JOIN commits c ON c.sha = l.sha
          WHERE l.session_id = ?1 ORDER BY c.ts_ms",
    )?;
    let commits = commits_stmt
        .query_map([session_id], |r| {
            let sha: String = r.get(0)?;
            let message: String = r.get(2)?;
            Ok(CommitSummary {
                short: sha.chars().take(7).collect(),
                ts_ms: r.get(1)?,
                subject: message.lines().next().unwrap_or("").to_string(),
                insertions: r.get(3)?,
                deletions: r.get(4)?,
                file_count: r.get(5)?,
                unreachable: r.get::<_, i64>(6)? == 1,
                tier: r.get(7)?,
                session_id: r.get(8)?,
                shared_files: r.get(9)?,
                sha,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(SessionDetail {
        project: leaf(&project_path),
        category: category(config, &project_path),
        turns: turns(conn, session_id)?,
        session: summary,
        project_path,
        files,
        commits,
    }))
}

/// The conversation, both sides, rebuilt from archived records.
///
/// A typed prompt is recorded twice by Claude Code: once in the session transcript
/// and once in its own `history.jsonl`. Rendering both showed every prompt twice.
///
/// Neither source can simply be dropped. The transcript is the fuller record — it
/// holds pasted content in full, where the history keeps only a placeholder — but
/// Claude Code deletes transcripts after about thirty days, and for the large
/// majority of archived sessions the history is the only surviving record of what
/// was asked. So the transcript wins where it exists, and the history supplies
/// what the transcript never had: slash commands, which are not messages.
pub fn turns(conn: &Connection, session_id: &str) -> Result<Vec<Turn>> {
    let mut stmt = conn.prepare(
        "SELECT kind, ts_ms, json FROM raw_records
          WHERE session_id = ?1 AND kind IN ('user','assistant','prompt_history')
          ORDER BY ts_ms, line_no",
    )?;
    let mut rows = stmt.query([session_id])?;
    let mut out = Vec::new();
    // Held back as raw text until the transcript has been read, since whether to
    // keep one depends on what the transcript turned out to contain.
    let mut from_history: Vec<(Option<i64>, String)> = Vec::new();
    let mut transcript_prompts: std::collections::HashSet<String> = Default::default();
    let mut saw_transcript = false;

    while let Some(row) = rows.next()? {
        let kind: String = row.get(0)?;
        let ts_ms: Option<i64> = row.get(1)?;
        let body: String = row.get(2)?;
        let Ok(value) = serde_json::from_str::<Value>(&body) else {
            continue;
        };

        if kind == "prompt_history" {
            if let Some(text) = value.get("display").and_then(Value::as_str) {
                from_history.push((ts_ms, text.to_string()));
            }
            continue;
        }
        saw_transcript = true;

        // Harness bookkeeping is not part of the conversation.
        if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
            continue;
        }

        let mut text = String::new();
        let mut truncated = false;
        let mut tools = Vec::new();

        match value.pointer("/message/content") {
            Some(Value::String(s)) => text.push_str(s),
            Some(Value::Array(blocks)) => {
                for block in blocks {
                    match block.get("type").and_then(Value::as_str) {
                        Some("text") => {
                            if let Some((t, cut)) = read_text(block.get("text")) {
                                if !text.is_empty() {
                                    text.push_str("\n\n");
                                }
                                text.push_str(&t);
                                truncated |= cut;
                            }
                        }
                        Some("tool_use") => {
                            let name = block.get("name").and_then(Value::as_str).unwrap_or("tool");
                            let target = block
                                .pointer("/input/file_path")
                                .and_then(Value::as_str)
                                .map(str::to_string)
                                .or_else(|| {
                                    read_text(block.pointer("/input/command")).map(|(t, _)| t)
                                })
                                .unwrap_or_default();
                            tools.push(ToolCall {
                                name: name.to_string(),
                                target,
                                failed: false,
                            });
                        }
                        Some("tool_result") => {
                            // Results belong to the call that produced them.
                            if block.get("is_error").and_then(Value::as_bool) == Some(true) {
                                if let Some(last) = tools.last_mut() {
                                    last.failed = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        // The harness records several kinds of turn as tagged markup. Resolving it
        // here makes a slash command read as it was typed — which also lets it
        // match the history's copy — and drops the blocks that were only ever
        // addressed to the model.
        match readable(&text) {
            Some(resolved) => text = resolved,
            None if tools.is_empty() => continue,
            None => text.clear(),
        }

        if text.trim().is_empty() && tools.is_empty() {
            continue;
        }
        if kind == "user" {
            transcript_prompts.insert(text.trim().to_string());
        }
        out.push(Turn {
            role: if kind == "user" { "user" } else { "assistant" }.into(),
            ts_ms,
            blocks: crate::markdown::parse(&text),
            truncated,
            tools,
        });
    }

    for (ts_ms, text) in from_history {
        // With no transcript, the history is the whole conversation.
        let keep = if saw_transcript {
            // A placeholder for content the transcript holds in full, and a prompt
            // the transcript already carries, are both the same prompt twice.
            !text.starts_with("[Pasted text") && !transcript_prompts.contains(text.trim())
        } else {
            true
        };
        if keep {
            out.push(Turn {
                role: "user".into(),
                ts_ms,
                blocks: crate::markdown::parse(&text),
                truncated: false,
                tools: Vec::new(),
            });
        }
    }

    // Merged rather than appended: a slash command belongs where it was typed.
    out.sort_by_key(|t| t.ts_ms.unwrap_or(i64::MAX));
    Ok(out)
}

/// Wrappers whose contents are real: a command you ran, and what it printed.
const KEEP_WRAPPERS: &[&str] = &[
    "local-command-stdout",
    "bash-input",
    "bash-stdout",
    "bash-stderr",
];

/// Wrappers addressed to the model rather than to a reader. The caveat is the same
/// paragraph every time, a task notification is scheduling bookkeeping, and fork
/// boilerplate is an instruction to a subagent. None of it is conversation.
const DROP_WRAPPERS: &[&str] = &["local-command-caveat", "task-notification", "fork-boilerplate"];

/// What a transcript turn actually said, with the harness's markup resolved.
///
/// Claude Code records several kinds of turn as tagged blocks. Rendered verbatim
/// they read as XML in the middle of a conversation; dropped wholesale they would
/// take real content with them. So each wrapper is either unwrapped or discarded,
/// and `None` means the turn was bookkeeping with nothing left once it went.
///
/// Only a turn that *begins* with a wrapper is touched, so a message discussing
/// this markup is never rewritten into it.
fn readable(text: &str) -> Option<String> {
    let trimmed = text.trim_start();
    if !trimmed.starts_with('<') {
        return Some(strip_ansi(text));
    }
    if let Some(command) = as_command(text) {
        return Some(command);
    }

    let tag = trimmed
        .strip_prefix('<')?
        .split('>')
        .next()?
        .to_string();
    if !KEEP_WRAPPERS.contains(&tag.as_str()) && !DROP_WRAPPERS.contains(&tag.as_str()) {
        return Some(strip_ansi(text));
    }

    let mut kept: Vec<String> = Vec::new();
    for wrapper in KEEP_WRAPPERS {
        let open = format!("<{wrapper}>");
        let close = format!("</{wrapper}>");
        let mut rest = text;
        while let Some(at) = rest.find(&open) {
            let from = at + open.len();
            let Some(to) = rest[from..].find(&close) else {
                break;
            };
            let inner = rest[from..from + to].trim();
            if !inner.is_empty() {
                kept.push(strip_ansi(inner));
            }
            rest = &rest[from + to + close.len()..];
        }
    }

    if kept.is_empty() {
        None
    } else {
        Some(kept.join("\n"))
    }
}

/// Removes terminal colour codes. A command's output is archived exactly as it was
/// printed, escapes included, and those are noise in a reader.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        // A CSI sequence is ESC '[' then parameters then one byte in @..~. The
        // bracket is itself inside that range, so it has to be stepped over before
        // the search for the terminator begins.
        if chars.clone().next() == Some('[') {
            chars.next();
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        } else {
            chars.next();
        }
    }
    out
}

/// The command a transcript turn represents, when its content is only the markup
/// Claude Code records for one.
///
/// `<command-name>/model</command-name>` and its siblings are how an invocation is
/// stored; nobody typed that. Turns carrying anything else are left alone, so a
/// message that merely mentions a command is not rewritten into one.
fn as_command(text: &str) -> Option<String> {
    let between = |tag: &str| -> Option<String> {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        let start = text.find(&open)? + open.len();
        let end = text[start..].find(&close)? + start;
        Some(text[start..end].trim().to_string())
    };

    let name = between("command-name")?;
    if !text.trim_start().starts_with("<command-") {
        return None;
    }
    let args = between("command-args").unwrap_or_default();
    Some(if args.is_empty() {
        name
    } else {
        format!("{name} {args}")
    })
}

/// Reads a string that may have been shortened on archive, reporting which.
fn read_text(value: Option<&Value>) -> Option<(String, bool)> {
    match value? {
        Value::String(s) => Some((s.clone(), false)),
        Value::Object(map) if map.contains_key("_lore_trunc") => Some((
            map.get("head")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            true,
        )),
        _ => None,
    }
}

/// The files a commit touched, from the archive rather than from git.
///
/// Read from `commit_files`, so it answers for a repository that has since been
/// deleted and for the 622 commits git will garbage-collect — which is why this
/// exists instead of a live diff.
pub fn commit_files(conn: &Connection, sha: &str) -> Result<Vec<CommitFile>> {
    let mut stmt = conn.prepare(
        "SELECT cf.path, cf.added, cf.deleted, coalesce(p.path, '')
           FROM commit_files cf
           JOIN commits c ON c.sha = cf.sha
           LEFT JOIN projects p ON p.id = c.project_id
          WHERE cf.sha = ?1
          ORDER BY coalesce(cf.added, 0) + coalesce(cf.deleted, 0) DESC, cf.path",
    )?;
    let rows = stmt.query_map([sha], |r| {
        let stored: String = r.get(0)?;
        let project: String = r.get(3)?;
        // Stored absolute so a commit can be matched against what a session
        // wrote; shown relative, because that is how the commit names it.
        let relative = stored
            .strip_prefix(&format!("{project}/"))
            .unwrap_or(&stored)
            .to_string();
        Ok(CommitFile {
            name: leaf(&relative),
            added: r.get(1)?,
            deleted: r.get(2)?,
            path: relative,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn status(conn: &Connection, config: &Config) -> Result<CollectorStatus> {
    let one = |sql: &str| -> Result<i64> { Ok(conn.query_row(sql, [], |r| r.get(0))?) };

    let (earliest, latest): (Option<i64>, Option<i64>) = conn.query_row(
        "SELECT min(ts_ms), max(ts_ms) FROM raw_records WHERE ts_ms IS NOT NULL",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    Ok(CollectorStatus {
        // A run in progress has not finished, so the previous finish is what the
        // window should keep showing until this one records its own.
        last_scan_ms: conn
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM meta WHERE key = 'run_finished_ms'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .ok()
            .filter(|ms| *ms > 0),
        last_archived_ms: conn
            .query_row("SELECT max(updated_at_ms) FROM origins", [], |r| r.get(0))?,
        records: one("SELECT count(*) FROM raw_records")?,
        sessions: one("SELECT count(*) FROM sessions")?,
        commits: one("SELECT count(*) FROM commits")?,
        file_changes: one("SELECT count(*) FROM raw_records WHERE kind='file_change'")?,
        origins: one("SELECT count(*) FROM origins")?,
        sessions_only_in_athar: one("SELECT count(*) FROM sessions WHERE has_transcript = 0")?,
        earliest_ms: earliest,
        latest_ms: latest,
        scan_interval_mins: config.scan_interval_mins,
        running: crate::db::current_run(conn).map(|r| r.action),
        roots: config
            .roots
            .iter()
            .map(|r| r.path.to_string_lossy().to_string())
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::markdown::{Block, Span};

    /// Turns carry a markdown tree now. These tests are about which turns survive
    /// and what they say, so they read the tree back as plain text.
    fn plain(turn: &Turn) -> String {
        fn spans(list: &[Span], out: &mut String) {
            for span in list {
                match span {
                    Span::Text { text } | Span::Code { text } => out.push_str(text),
                    Span::Strong { spans: inner }
                    | Span::Em { spans: inner }
                    | Span::Link { spans: inner, .. } => spans(inner, out),
                }
            }
        }
        fn walk(list: &[Block], out: &mut String) {
            for block in list {
                match block {
                    Block::Paragraph { spans: s } | Block::Heading { spans: s, .. } => {
                        spans(s, out)
                    }
                    Block::Code { text, .. } => out.push_str(text),
                    Block::Quote { blocks } => walk(blocks, out),
                    Block::List { items, .. } => items.iter().for_each(|i| walk(i, out)),
                    Block::Table { .. } | Block::Rule => {}
                }
            }
        }
        let mut out = String::new();
        walk(&turn.blocks, &mut out);
        out
    }

    fn temp_db() -> Connection {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "athar-api-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        db::open_writable(&dir.join("athar.db")).unwrap()
    }

    /// `line_no` is per-origin, so transcript and history get their own.
    fn record(conn: &Connection, origin: i64, line: i64, ts: i64, kind: &str, json: &str) {
        conn.execute(
            "INSERT INTO raw_records (origin_id, line_no, ts_ms, kind, session_id, json, bytes_original)
             VALUES (?1, ?2, ?3, ?4, 's', ?5, length(?5))",
            rusqlite::params![origin, line, ts, kind, json],
        )
        .unwrap();
    }

    fn origins(conn: &Connection) -> (i64, i64) {
        let a = db::origin_cursor(conn, "claude", "/t/s.jsonl").unwrap().id;
        let b = db::origin_cursor(conn, "claude", "/t/history.jsonl").unwrap().id;
        (a, b)
    }

    fn user(text: &str) -> String {
        format!(
            r#"{{"type":"user","message":{{"role":"user","content":[{{"type":"text","text":{}}}]}}}}"#,
            serde_json::to_string(text).unwrap()
        )
    }

    /// The defect: a typed prompt lives in the transcript *and* in history, so the
    /// reader showed every prompt twice.
    #[test]
    fn a_prompt_in_both_sources_is_one_turn() {
        let conn = temp_db();
        let (t, h) = origins(&conn);

        record(&conn, t, 1, 1000, "user", &user("fix the footer"));
        record(&conn, t, 2, 1100, "assistant",
            r#"{"message":{"role":"assistant","content":[{"type":"text","text":"done"}]}}"#);
        // The same prompt as the transcript carries, plus two the transcript never had.
        record(&conn, h, 1, 1000, "prompt_history", r#"{"display":"fix the footer"}"#);
        record(&conn, h, 2, 1050, "prompt_history", r#"{"display":"/model"}"#);
        record(&conn, h, 3, 1060, "prompt_history", r#"{"display":"[Pasted text #1 +8 lines]"}"#);

        let turns = turns(&conn, "s").unwrap();
        let texts: Vec<String> = turns.iter().map(plain).collect();

        assert_eq!(
            texts,
            vec!["fix the footer".to_string(), "/model".into(), "done".into()],
            "the duplicate prompt and the paste placeholder both go; the command stays"
        );
        // And the command lands where it was typed, not appended at the end.
        assert!(turns[1].ts_ms.unwrap() < turns[2].ts_ms.unwrap());
    }

    /// A slash command is recorded as markup in the transcript and as typed text in
    /// the history, so it was showing twice — once unreadably.
    #[test]
    fn a_slash_command_is_one_readable_turn() {
        let conn = temp_db();
        let (t, h) = origins(&conn);

        record(&conn, t, 1, 1000, "user", &user(
            "<command-name>/model</command-name>\n            <command-message>model</command-message>\n            <command-args></command-args>"));
        record(&conn, t, 2, 2000, "user", &user(
            "<command-name>/impeccable</command-name>\n<command-args>doctor</command-args>"));
        record(&conn, h, 1, 1000, "prompt_history", r#"{"display":"/model"}"#);
        record(&conn, h, 2, 2000, "prompt_history", r#"{"display":"/impeccable doctor"}"#);

        let texts: Vec<String> = turns(&conn, "s").unwrap().into_iter().map(|t| plain(&t)).collect();
        assert_eq!(texts, vec!["/model", "/impeccable doctor"]);
    }

    /// The harness records shell runs, command output and its own notifications as
    /// tagged blocks in the user's turn. Some of that is content; some is addressed
    /// to the model and is not conversation at all.
    #[test]
    fn harness_markup_is_unwrapped_or_dropped() {
        let conn = temp_db();
        let (t, _) = origins(&conn);

        record(&conn, t, 1, 1000, "user", &user("<bash-input>git status</bash-input>"));
        record(&conn, t, 2, 1100, "user", &user(
            "<bash-stdout>nothing to commit</bash-stdout><bash-stderr></bash-stderr>"));
        // Colour codes are archived exactly as printed; they are noise to read.
        record(&conn, t, 3, 1200, "user", &user(
            "<local-command-stdout>Set model to \u{1b}[1mFable 5\u{1b}[22m</local-command-stdout>"));
        record(&conn, t, 4, 1300, "user", &user(
            "<local-command-caveat>Caveat: DO NOT respond to these messages.</local-command-caveat>"));
        record(&conn, t, 5, 1400, "user", &user("<task-notification>\n<task-id>abc</task-id>\n</task-notification>"));
        record(&conn, t, 6, 1500, "user", &user("a real message"));

        let texts: Vec<String> = turns(&conn, "s").unwrap().into_iter().map(|t| plain(&t)).collect();
        assert_eq!(
            texts,
            vec![
                "git status",
                "nothing to commit",
                "Set model to Fable 5",
                "a real message",
            ],
            "content is unwrapped and stripped; bookkeeping goes entirely"
        );
    }

    /// Mentioning the markup is not invoking it.
    #[test]
    fn prose_about_a_command_is_left_alone() {
        let conn = temp_db();
        let (t, _) = origins(&conn);
        let prose = "why does <command-name>/model</command-name> show up twice?";
        record(&conn, t, 1, 1000, "user", &user(prose));

        let texts: Vec<String> = turns(&conn, "s").unwrap().into_iter().map(|t| plain(&t)).collect();
        assert_eq!(texts, vec![prose]);
    }

    /// 936 of this machine's 1,034 sessions have no transcript left: Claude Code
    /// deleted it, and history is the only record of what was asked.
    #[test]
    fn without_a_transcript_history_is_the_whole_conversation() {
        let conn = temp_db();
        let (_, h) = origins(&conn);
        record(&conn, h, 1, 1000, "prompt_history", r#"{"display":"first"}"#);
        record(&conn, h, 2, 2000, "prompt_history", r#"{"display":"[Pasted text #1 +8 lines]"}"#);
        record(&conn, h, 3, 3000, "prompt_history", r#"{"display":"second"}"#);

        let texts: Vec<String> = turns(&conn, "s").unwrap().into_iter().map(|t| plain(&t)).collect();
        assert_eq!(
            texts,
            vec!["first", "[Pasted text #1 +8 lines]", "second"],
            "nothing may be dropped when it is the only source"
        );
    }
}
