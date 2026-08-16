//! Projections over the archive.
//!
//! Everything here is recomputed from `raw_records` by [`rebuild`] and is never a
//! source of truth. When an adapter improves, these are rebuilt rather than
//! migrated — which is why the raw record is archived first, and why athar can
//! reinterpret history whose source files no longer exist.
//!
//! Two projections carry the product:
//!
//!   - **Activity blocks.** A contiguous stretch of work on one project, ended by
//!     an idle gap. This is how hours are accounted for with no timer running.
//!   - **Commit links.** Which session a commit came out of, and on what
//!     evidence. The tier is recorded rather than smoothed away, because a link
//!     athar witnessed in a transcript and one it inferred from timing are not the
//!     same claim.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use rusqlite::Connection;
use serde_json::Value;

use crate::config::Config;
use crate::db;

/// How close a recorded `git commit` call must be to a commit's own timestamp for
/// the link to count as witnessed rather than inferred.
const WITNESS_WINDOW_MS: i64 = 5 * 60 * 1000;

#[derive(Debug, Default, Clone, Copy)]
pub struct DeriveStats {
    pub projects: usize,
    pub folded_paths: usize,
    pub blocks: usize,
    pub sessions: usize,
    pub commits: usize,
    pub links_certain: usize,
    pub links_strong: usize,
    pub links_weak: usize,
    pub commits_unlinked: usize,
}

pub fn rebuild(conn: &mut Connection, config: &Config) -> Result<DeriveStats> {
    let gap_ms = (config.idle_gap_mins as i64) * 60_000;
    let mut stats = DeriveStats::default();

    // Raw records keep the exact path they were recorded with; the projection
    // folds each one to its project. Because this happens at rebuild, changing
    // the configured roots is `athar rebuild`, never a re-ingest.
    let canonical = canonical_projects(conn, config)?;
    stats.projects = canonical.values().collect::<HashSet<_>>().len();

    let sessions = read_sessions(conn, &canonical)?;
    let session_files = read_session_files(conn)?;
    let commit_calls = read_commit_calls(conn)?;
    let commits = read_commits(conn, &canonical)?;
    let blocks = compute_blocks(conn, gap_ms, &canonical)?;

    let links = link_commits(&sessions, &session_files, &commit_calls, &commits, gap_ms);
    for link in links.values() {
        match link.tier {
            Tier::Certain => stats.links_certain += 1,
            Tier::Strong => stats.links_strong += 1,
            Tier::Weak => stats.links_weak += 1,
        }
    }
    stats.folded_paths = canonical.iter().filter(|(raw, to)| raw != to).count();
    stats.commits_unlinked = commits.len() - links.len();
    stats.blocks = blocks.len();
    stats.sessions = sessions.len();
    stats.commits = commits.len();

    write_all(conn, &blocks, &sessions, &session_files, &commits, &links)?;
    Ok(stats)
}

/// Maps every recorded project path to the project it belongs to, interning any
/// canonical path that is not itself already recorded.
fn canonical_projects(conn: &Connection, config: &Config) -> Result<HashMap<i64, i64>> {
    let mut rows = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT id, path FROM projects")?;
        let mut q = stmt.query([])?;
        while let Some(row) = q.next()? {
            rows.push((row.get::<_, i64>(0)?, row.get::<_, String>(1)?));
        }
    }

    let mut out = HashMap::new();
    let mut interned: HashMap<String, i64> = HashMap::new();
    for (id, path) in rows {
        let on_disk = std::path::Path::new(&path).exists();
        let remembered = remembered_fold(conn, &path)?;

        // A path still on disk is re-folded every time, so changing the roots is
        // a rebuild. A path whose folder is gone keeps the decision made while
        // athar could still see its repository.
        let canonical = match (on_disk, remembered) {
            (false, Some(stored)) => stored,
            _ => {
                let computed = config
                    .canonical_project(std::path::Path::new(&path))
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone());
                remember_fold(conn, &path, &computed, on_disk)?;
                computed
            }
        };

        let canonical_id = if canonical == path {
            id
        } else {
            match interned.get(&canonical) {
                Some(existing) => *existing,
                None => {
                    let new_id = db::project_id(conn, &canonical)?;
                    interned.insert(canonical.clone(), new_id);
                    new_id
                }
            }
        };
        out.insert(id, canonical_id);
    }

    Ok(out)
}

fn remembered_fold(conn: &Connection, raw: &str) -> Result<Option<String>> {
    use rusqlite::OptionalExtension;
    Ok(conn
        .query_row(
            "SELECT canonical_path FROM project_map WHERE raw_path = ?1",
            [raw],
            |r| r.get(0),
        )
        .optional()?)
}

fn remember_fold(conn: &Connection, raw: &str, canonical: &str, from_disk: bool) -> Result<()> {
    conn.execute(
        "INSERT INTO project_map (raw_path, canonical_path, decided_at_ms, from_disk)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(raw_path) DO UPDATE SET
             canonical_path = excluded.canonical_path,
             decided_at_ms = excluded.decided_at_ms,
             from_disk = excluded.from_disk",
        (
            raw,
            canonical,
            chrono::Utc::now().timestamp_millis(),
            from_disk as i64,
        ),
    )?;
    Ok(())
}

/// Translates a recorded project id to the project it belongs to.
fn fold(canonical: &HashMap<i64, i64>, id: Option<i64>) -> Option<i64> {
    id.map(|raw| canonical.get(&raw).copied().unwrap_or(raw))
}

// ── Sessions ─────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
pub struct Session {
    pub id: String,
    pub project_id: Option<i64>,
    pub started_ms: Option<i64>,
    pub ended_ms: Option<i64>,
    pub title: Option<String>,
    pub prompts: i64,
    pub replies: i64,
    pub tool_calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub models: HashSet<String>,
    pub has_transcript: bool,
    /// Prompts seen in `history.jsonl`. Kept apart from `prompts` because the
    /// same prompt is usually in the transcript too, and counting both would
    /// double every session that still has one.
    history_prompts: i64,
}

fn read_sessions(
    conn: &Connection,
    canonical: &HashMap<i64, i64>,
) -> Result<HashMap<String, Session>> {
    let mut out: HashMap<String, Session> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT session_id, kind, ts_ms, project_id, json
           FROM raw_records
          WHERE session_id IS NOT NULL
          ORDER BY ts_ms",
    )?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let kind: String = row.get(1)?;
        let ts: Option<i64> = row.get(2)?;
        let project_id: Option<i64> = row.get(3)?;
        let body: String = row.get(4)?;

        let s = out.entry(id.clone()).or_insert_with(|| Session {
            id: id.clone(),
            ..Default::default()
        });
        if s.project_id.is_none() {
            s.project_id = fold(canonical, project_id);
        }
        if let Some(ts) = ts {
            s.started_ms = Some(s.started_ms.map_or(ts, |v: i64| v.min(ts)));
            s.ended_ms = Some(s.ended_ms.map_or(ts, |v: i64| v.max(ts)));
        }

        let value: Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => continue,
        };

        match kind.as_str() {
            "ai-title" => {
                if let Some(t) = value.get("aiTitle").and_then(Value::as_str) {
                    s.title = Some(t.to_string());
                }
            }
            "prompt_history" => {
                s.history_prompts += 1;
            }
            "user" => {
                s.has_transcript = true;
                // A user record whose content is only tool results is the
                // harness answering the model, not a person typing.
                if is_human_turn(&value) {
                    s.prompts += 1;
                }
            }
            "assistant" => {
                s.has_transcript = true;
                if let Some(model) = value.pointer("/message/model").and_then(Value::as_str) {
                    // `<synthetic>` marks a message the harness generated itself,
                    // not a model that ran. Recording it as a model is a lie.
                    if !model.starts_with('<') {
                        s.models.insert(model.to_string());
                    }
                }
                if let Some(usage) = value.pointer("/message/usage") {
                    s.input_tokens += num(usage, "input_tokens");
                    s.output_tokens += num(usage, "output_tokens");
                    s.cache_read_tokens += num(usage, "cache_read_input_tokens");
                }
                let mut had_text = false;
                for block in blocks_of(&value) {
                    match block.get("type").and_then(Value::as_str) {
                        Some("text") => had_text = true,
                        Some("tool_use") => s.tool_calls += 1,
                        _ => {}
                    }
                }
                if had_text {
                    s.replies += 1;
                }
            }
            _ => {}
        }
    }

    // A session whose transcript Claude Code has deleted still has its prompts,
    // in the history file athar archived separately. Without this those sessions
    // reported nothing typed at all — on this machine, 936 of them, holding seven
    // thousand prompts between them. Where a transcript survives it is the better
    // record and the history is the same prompts again, so it is not added.
    for session in out.values_mut() {
        if !session.has_transcript {
            session.prompts = session.history_prompts;
        }
    }

    Ok(out)
}

/// True when a person typed this turn, rather than the harness returning results.
fn is_human_turn(value: &Value) -> bool {
    if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
        return false;
    }
    match value.pointer("/message/content") {
        Some(Value::String(_)) => true,
        Some(Value::Array(items)) => items.iter().any(|b| {
            matches!(
                b.get("type").and_then(Value::as_str),
                Some("text") | Some("image")
            )
        }),
        _ => false,
    }
}

fn blocks_of(value: &Value) -> impl Iterator<Item = &Value> {
    value
        .pointer("/message/content")
        .and_then(Value::as_array)
        .map(|v| v.iter())
        .unwrap_or_default()
}

fn num(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0)
}

// ── Files a session touched ──────────────────────────────────────────────────

#[derive(Debug, Default, Clone, Copy)]
pub struct FileTouch {
    pub writes: i64,
    pub reads: i64,
}

type SessionFiles = HashMap<String, HashMap<String, FileTouch>>;

fn read_session_files(conn: &Connection) -> Result<SessionFiles> {
    let mut out: SessionFiles = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT session_id, json FROM raw_records
          WHERE kind = 'assistant' AND session_id IS NOT NULL",
    )?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let body: String = row.get(1)?;
        let Ok(value) = serde_json::from_str::<Value>(&body) else {
            continue;
        };

        for block in blocks_of(&value) {
            if block.get("type").and_then(Value::as_str) != Some("tool_use") {
                continue;
            }
            let name = block.get("name").and_then(Value::as_str).unwrap_or("");
            let Some(path) = block
                .pointer("/input/file_path")
                .and_then(Value::as_str)
                .filter(|p| p.starts_with('/'))
            else {
                continue;
            };

            let entry = out
                .entry(id.clone())
                .or_default()
                .entry(path.to_string())
                .or_default();
            match name {
                "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => entry.writes += 1,
                "Read" => entry.reads += 1,
                _ => {}
            }
        }
    }

    Ok(out)
}

/// Every recorded `git commit` invocation: the transcript witnessing the AI
/// commit, which is the only way a link can be certain rather than inferred.
fn read_commit_calls(conn: &Connection) -> Result<Vec<(String, i64)>> {
    let mut out = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT session_id, ts_ms, json FROM raw_records
          WHERE kind = 'assistant' AND session_id IS NOT NULL
            AND ts_ms IS NOT NULL AND json LIKE '%git commit%'",
    )?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let ts: i64 = row.get(1)?;
        let body: String = row.get(2)?;
        let Ok(value) = serde_json::from_str::<Value>(&body) else {
            continue;
        };
        for block in blocks_of(&value) {
            if block.get("type").and_then(Value::as_str) != Some("tool_use") {
                continue;
            }
            let command = block
                .pointer("/input/command")
                .and_then(Value::as_str)
                .unwrap_or("");
            if command.contains("git commit") {
                out.push((id.clone(), ts));
                break;
            }
        }
    }

    Ok(out)
}

// ── Commits ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Commit {
    pub sha: String,
    pub project_id: i64,
    pub project_path: String,
    pub ts_ms: i64,
    pub message: String,
    pub unreachable: bool,
    pub insertions: i64,
    pub deletions: i64,
    /// Absolute paths, so a commit can be compared with what a session wrote,
    /// each with the lines it changed.
    pub files: Vec<CommitFile>,
}

#[derive(Debug, Clone)]
pub struct CommitFile {
    pub path: String,
    pub added: Option<i64>,
    pub deleted: Option<i64>,
}

fn read_commits(conn: &Connection, canonical: &HashMap<i64, i64>) -> Result<Vec<Commit>> {
    let mut out = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT r.json, r.project_id, p.path, r.ts_ms
           FROM raw_records r JOIN projects p ON p.id = r.project_id
          WHERE r.kind = 'commit' AND r.ts_ms IS NOT NULL",
    )?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let body: String = row.get(0)?;
        let project_id: i64 = row.get(1)?;
        let project_path: String = row.get(2)?;
        let ts_ms: i64 = row.get(3)?;
        let Ok(value) = serde_json::from_str::<Value>(&body) else {
            continue;
        };
        let Some(sha) = value.get("sha").and_then(Value::as_str) else {
            continue;
        };

        let mut insertions = 0;
        let mut deletions = 0;
        let mut files = Vec::new();
        if let Some(list) = value.get("files").and_then(Value::as_array) {
            for f in list {
                insertions += f.get("added").and_then(Value::as_i64).unwrap_or(0);
                deletions += f.get("deleted").and_then(Value::as_i64).unwrap_or(0);
                if let Some(p) = f.get("path").and_then(Value::as_str) {
                    files.push(CommitFile {
                        path: format!("{project_path}/{p}"),
                        added: f.get("added").and_then(Value::as_i64),
                        deleted: f.get("deleted").and_then(Value::as_i64),
                    });
                }
            }
        }

        out.push(Commit {
            sha: sha.to_string(),
            // Folded for grouping; `project_path` stays the repository, because
            // the commit's file paths are relative to it.
            project_id: fold(canonical, Some(project_id)).unwrap_or(project_id),
            project_path: project_path.clone(),
            ts_ms,
            message: value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            unreachable: value.get("unreachable").and_then(Value::as_bool) == Some(true),
            insertions,
            deletions,
            files,
        });
    }

    Ok(out)
}

// ── Activity blocks ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Block {
    pub project_id: i64,
    pub started_ms: i64,
    pub ended_ms: i64,
    pub records: i64,
    pub sessions: i64,
    pub commits: i64,
    pub file_changes: i64,
}

/// Clusters every timestamped record by project, breaking a block wherever the
/// gap between consecutive records exceeds the idle threshold.
fn compute_blocks(
    conn: &Connection,
    gap_ms: i64,
    canonical: &HashMap<i64, i64>,
) -> Result<Vec<Block>> {
    let mut stmt = conn.prepare(
        // Ordered by the folded project so a session run from a subdirectory
        // clusters with the rest of that project's work rather than forming its
        // own lane.
        "SELECT project_id, ts_ms, kind, session_id
           FROM raw_records
          WHERE project_id IS NOT NULL AND ts_ms IS NOT NULL",
    )?;
    let mut rows = stmt.query([])?;

    let mut out: Vec<Block> = Vec::new();
    let mut current: Option<Block> = None;
    let mut current_project = -1i64;
    let mut seen_sessions: HashSet<String> = HashSet::new();

    let mut records: Vec<(i64, i64, String, Option<String>)> = Vec::new();
    while let Some(row) = rows.next()? {
        let raw: i64 = row.get(0)?;
        records.push((
            canonical.get(&raw).copied().unwrap_or(raw),
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
        ));
    }
    records.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    for (project_id, ts, kind, session_id) in records {
        let project_id = project_id;
        let ts = ts;

        let start_new = match &current {
            None => true,
            Some(b) => project_id != current_project || ts - b.ended_ms > gap_ms,
        };

        if start_new {
            if let Some(b) = current.take() {
                out.push(b);
            }
            seen_sessions.clear();
            current_project = project_id;
            current = Some(Block {
                project_id,
                started_ms: ts,
                ended_ms: ts,
                records: 0,
                sessions: 0,
                commits: 0,
                file_changes: 0,
            });
        }

        let block = current.as_mut().expect("block just created");
        block.ended_ms = ts;
        block.records += 1;
        match kind.as_str() {
            "commit" => block.commits += 1,
            "file_change" => block.file_changes += 1,
            _ => {}
        }
        if let Some(sid) = session_id {
            if seen_sessions.insert(sid) {
                block.sessions += 1;
            }
        }
    }
    if let Some(b) = current.take() {
        out.push(b);
    }

    Ok(out)
}

// ── Commit links ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Weak,
    Strong,
    Certain,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Certain => "certain",
            Tier::Strong => "strong",
            Tier::Weak => "weak",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Link {
    pub session_id: String,
    pub tier: Tier,
    pub shared_files: i64,
    distance_ms: i64,
}

/// Attributes each commit to at most one session, keeping the strongest evidence
/// available and, among equals, the nearest in time.
fn link_commits(
    sessions: &HashMap<String, Session>,
    session_files: &SessionFiles,
    commit_calls: &[(String, i64)],
    commits: &[Commit],
    gap_ms: i64,
) -> HashMap<String, Link> {
    // Sessions grouped by project, so a commit only considers its own project.
    let mut by_project: HashMap<i64, Vec<&Session>> = HashMap::new();
    for s in sessions.values() {
        if let Some(pid) = s.project_id {
            by_project.entry(pid).or_default().push(s);
        }
    }

    let mut calls_by_session: HashMap<&str, Vec<i64>> = HashMap::new();
    for (id, ts) in commit_calls {
        calls_by_session.entry(id.as_str()).or_default().push(*ts);
    }

    let mut out: HashMap<String, Link> = HashMap::new();

    for commit in commits {
        let Some(candidates) = by_project.get(&commit.project_id) else {
            continue;
        };

        let mut best: Option<Link> = None;
        for session in candidates {
            let (Some(start), Some(end)) = (session.started_ms, session.ended_ms) else {
                continue;
            };
            // A commit belongs to a session's stretch, allowing one idle gap
            // afterwards for a commit made right after the conversation ended.
            if commit.ts_ms < start - gap_ms || commit.ts_ms > end + gap_ms {
                continue;
            }

            let shared = session_files
                .get(&session.id)
                .map(|files| {
                    commit
                        .files
                        .iter()
                        .filter(|f| files.get(&f.path).is_some_and(|t| t.writes > 0))
                        .count() as i64
                })
                .unwrap_or(0);

            let witnessed = calls_by_session
                .get(session.id.as_str())
                .is_some_and(|times| {
                    times
                        .iter()
                        .any(|t| (commit.ts_ms - t).abs() <= WITNESS_WINDOW_MS)
                });

            let tier = if witnessed {
                Tier::Certain
            } else if shared > 0 {
                Tier::Strong
            } else {
                Tier::Weak
            };

            let distance_ms = if commit.ts_ms < start {
                start - commit.ts_ms
            } else if commit.ts_ms > end {
                commit.ts_ms - end
            } else {
                0
            };

            let candidate = Link {
                session_id: session.id.clone(),
                tier,
                shared_files: shared,
                distance_ms,
            };
            let better = match &best {
                None => true,
                Some(b) => {
                    (candidate.tier, -candidate.distance_ms, candidate.shared_files)
                        > (b.tier, -b.distance_ms, b.shared_files)
                }
            };
            if better {
                best = Some(candidate);
            }
        }

        if let Some(link) = best {
            out.insert(commit.sha.clone(), link);
        }
    }

    out
}

// ── Writing ──────────────────────────────────────────────────────────────────

fn write_all(
    conn: &mut Connection,
    blocks: &[Block],
    sessions: &HashMap<String, Session>,
    session_files: &SessionFiles,
    commits: &[Commit],
    links: &HashMap<String, Link>,
) -> Result<()> {
    let tx = conn.transaction()?;

    // Derived tables are replaced wholesale; nothing here is migrated.
    for table in [
        "commit_links",
        "commit_files",
        "commits",
        "session_files",
        "sessions",
        "blocks",
    ] {
        tx.execute(&format!("DELETE FROM {table}"), [])?;
    }

    // Blocks first, so sessions and commits can be assigned to one.
    let mut block_ids: Vec<(i64, i64, i64, i64)> = Vec::with_capacity(blocks.len());
    {
        let mut insert = tx.prepare(
            "INSERT INTO blocks
                 (project_id, started_ms, ended_ms, records, sessions, commits, file_changes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for b in blocks {
            insert.execute((
                b.project_id,
                b.started_ms,
                b.ended_ms,
                b.records,
                b.sessions,
                b.commits,
                b.file_changes,
            ))?;
            block_ids.push((tx.last_insert_rowid(), b.project_id, b.started_ms, b.ended_ms));
        }
    }

    let find_block = |project_id: Option<i64>, ts: Option<i64>| -> Option<i64> {
        let (pid, ts) = (project_id?, ts?);
        block_ids
            .iter()
            .find(|(_, bp, start, end)| *bp == pid && ts >= *start && ts <= *end)
            .map(|(id, _, _, _)| *id)
    };

    {
        let mut insert = tx.prepare(
            "INSERT INTO sessions
                 (session_id, project_id, block_id, started_ms, ended_ms, title,
                  prompts, replies, tool_calls, input_tokens, output_tokens,
                  cache_read_tokens, models, has_transcript)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )?;
        let mut insert_file = tx.prepare(
            "INSERT OR IGNORE INTO session_files (session_id, path, writes, reads)
             VALUES (?1, ?2, ?3, ?4)",
        )?;

        for s in sessions.values() {
            let mut models: Vec<&str> = s.models.iter().map(String::as_str).collect();
            models.sort_unstable();
            insert.execute((
                &s.id,
                s.project_id,
                find_block(s.project_id, s.started_ms),
                s.started_ms,
                s.ended_ms,
                &s.title,
                s.prompts,
                s.replies,
                s.tool_calls,
                s.input_tokens,
                s.output_tokens,
                s.cache_read_tokens,
                models.join(","),
                s.has_transcript as i64,
            ))?;

            if let Some(files) = session_files.get(&s.id) {
                for (path, touch) in files {
                    insert_file.execute((&s.id, path, touch.writes, touch.reads))?;
                }
            }
        }
    }

    {
        let mut insert = tx.prepare(
            "INSERT OR IGNORE INTO commits
                 (sha, project_id, block_id, ts_ms, message, unreachable,
                  file_count, insertions, deletions)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        let mut insert_file = tx.prepare(
            "INSERT OR IGNORE INTO commit_files (sha, path, added, deleted)
             VALUES (?1, ?2, ?3, ?4)",
        )?;
        let mut insert_link = tx.prepare(
            "INSERT OR IGNORE INTO commit_links (sha, session_id, tier, shared_files)
             VALUES (?1, ?2, ?3, ?4)",
        )?;

        for c in commits {
            insert.execute((
                &c.sha,
                c.project_id,
                find_block(Some(c.project_id), Some(c.ts_ms)),
                c.ts_ms,
                &c.message,
                c.unreachable as i64,
                c.files.len() as i64,
                c.insertions,
                c.deletions,
            ))?;
            for file in &c.files {
                insert_file.execute((&c.sha, &file.path, file.added, file.deleted))?;
            }
            if let Some(link) = links.get(&c.sha) {
                insert_link.execute((
                    &c.sha,
                    &link.session_id,
                    link.tier.as_str(),
                    link.shared_files,
                ))?;
            }
        }
    }

    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// No roots configured, so no path is folded — these tests exercise blocks and
    /// links, and `config.rs` owns the folding rule's own tests.
    fn test_config() -> Config {
        Config {
            idle_gap_mins: 20,
            ..Default::default()
        }
    }

    fn temp_db() -> Connection {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "athar-derive-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        db::open_writable(&dir.join("athar.db")).unwrap()
    }

    const MIN: i64 = 60_000;

    struct Fixture {
        conn: Connection,
        origin: i64,
        project: i64,
        line: i64,
    }

    impl Fixture {
        fn new(project_path: &str) -> Self {
            let conn = temp_db();
            let origin = db::origin_cursor(&conn, "claude", "/t.jsonl").unwrap().id;
            let project = db::project_id(&conn, project_path).unwrap();
            Self { conn, origin, project, line: 0 }
        }

        fn add(&mut self, kind: &str, ts: i64, session: Option<&str>, body: Value) {
            self.line += 1;
            self.conn
                .execute(
                    "INSERT INTO raw_records
                         (origin_id, line_no, ts_ms, kind, session_id, project_id,
                          json, bytes_original)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
                    (
                        self.origin,
                        self.line,
                        ts,
                        kind,
                        session,
                        self.project,
                        body.to_string(),
                    ),
                )
                .unwrap();
        }

        fn prompt(&mut self, ts: i64, session: &str, text: &str) {
            self.add(
                "user",
                ts,
                Some(session),
                json!({ "message": { "role": "user", "content": text } }),
            );
        }

        fn tool(&mut self, ts: i64, session: &str, name: &str, input: Value) {
            self.add(
                "assistant",
                ts,
                Some(session),
                json!({ "message": {
                    "model": "claude-opus-5",
                    "usage": { "input_tokens": 10, "output_tokens": 20, "cache_read_input_tokens": 5 },
                    "content": [{ "type": "tool_use", "name": name, "input": input }]
                }}),
            );
        }

        /// Point subsequent records at a different project.
        fn project(&mut self, path: &str) {
            self.project = db::project_id(&self.conn, path).unwrap();
        }

        /// A filesystem save, as the file collector records one.
        fn file_change(&mut self, ts: i64, path: &str) {
            self.line += 1;
            self.conn
                .execute(
                    "INSERT INTO raw_records
                         (origin_id, ext_id, ts_ms, kind, project_id, json,
                          bytes_original, truncated)
                     VALUES (?1, ?2, ?3, 'file_change', ?4, ?5, 0, 0)",
                    (
                        self.origin,
                        format!("{path}@{ts}"),
                        ts,
                        self.project,
                        json!({
                            "path": path, "mtime_ms": ts, "size": 10, "state": "dirty"
                        })
                        .to_string(),
                    ),
                )
                .unwrap();
        }

        fn commit(&mut self, ts: i64, sha: &str, message: &str, files: &[&str]) {
            let files: Vec<Value> = files
                .iter()
                .map(|p| json!({ "path": p, "added": 3, "deleted": 1 }))
                .collect();
            self.line += 1;
            self.conn
                .execute(
                    "INSERT INTO raw_records
                         (origin_id, ext_id, ts_ms, kind, project_id, json, bytes_original)
                     VALUES (?1, ?2, ?3, 'commit', ?4, ?5, 0)",
                    (
                        self.origin,
                        sha,
                        ts,
                        self.project,
                        json!({
                            "sha": sha, "message": message, "files": files,
                            "unreachable": false
                        })
                        .to_string(),
                    ),
                )
                .unwrap();
        }
    }

    #[test]
    fn splits_blocks_on_an_idle_gap_and_keeps_continuous_work_together() {
        let mut f = Fixture::new("/w/proj");
        let base = 1_780_000_000_000;
        f.prompt(base, "s1", "morning");
        f.prompt(base + 5 * MIN, "s1", "still going");
        f.prompt(base + 10 * MIN, "s1", "and again");
        // A three-hour break ends the block.
        f.prompt(base + 190 * MIN, "s2", "back after lunch");

        let stats = rebuild(&mut f.conn, &test_config()).unwrap();
        assert_eq!(stats.blocks, 2);

        let spans: Vec<(i64, i64)> = f
            .conn
            .prepare("SELECT started_ms, ended_ms FROM blocks ORDER BY started_ms")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(spans[0], (base, base + 10 * MIN));
        assert_eq!(spans[1], (base + 190 * MIN, base + 190 * MIN));
    }

    #[test]
    fn a_commit_the_ai_ran_is_certain() {
        let mut f = Fixture::new("/w/proj");
        let base = 1_780_000_000_000;
        f.prompt(base, "s1", "please commit");
        f.tool(
            base + MIN,
            "s1",
            "Bash",
            json!({ "command": "git commit -q -m \"feat: thing\"" }),
        );
        f.commit(base + MIN + 2_000, "abc123", "feat: thing", &["src/a.rs"]);

        let stats = rebuild(&mut f.conn, &test_config()).unwrap();
        assert_eq!(stats.links_certain, 1);

        let (session, tier): (String, String) = f
            .conn
            .query_row("SELECT session_id, tier FROM commit_links", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(session, "s1");
        assert_eq!(tier, "certain");
    }

    #[test]
    fn a_hand_made_commit_of_ai_written_files_is_strong_not_certain() {
        let mut f = Fixture::new("/w/proj");
        let base = 1_780_000_000_000;
        f.prompt(base, "s1", "write the parser");
        f.tool(
            base + MIN,
            "s1",
            "Write",
            json!({ "file_path": "/w/proj/src/parser.rs", "content": "code" }),
        );
        // Committed by hand in a terminal: nothing witnessed it, but the files
        // are exactly what the session wrote.
        f.commit(base + 3 * MIN, "def456", "feat: parser", &["src/parser.rs"]);

        let stats = rebuild(&mut f.conn, &test_config()).unwrap();
        assert_eq!(stats.links_certain, 0);
        assert_eq!(stats.links_strong, 1);

        let (tier, shared): (String, i64) = f
            .conn
            .query_row("SELECT tier, shared_files FROM commit_links", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(tier, "strong");
        assert_eq!(shared, 1);
    }

    #[test]
    fn a_commit_of_files_no_session_touched_is_only_weak() {
        let mut f = Fixture::new("/w/proj");
        let base = 1_780_000_000_000;
        f.prompt(base, "s1", "explain something");
        f.commit(base + 2 * MIN, "aaa111", "chore: hand edit", &["docs/notes.md"]);

        let stats = rebuild(&mut f.conn, &test_config()).unwrap();
        assert_eq!(stats.links_weak, 1);
        let tier: String = f
            .conn
            .query_row("SELECT tier FROM commit_links", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tier, "weak");
    }

    #[test]
    fn a_commit_far_from_any_session_stays_unlinked() {
        let mut f = Fixture::new("/w/proj");
        let base = 1_780_000_000_000;
        f.prompt(base, "s1", "hello");
        f.commit(base + 600 * MIN, "bbb222", "chore: much later", &["a.txt"]);

        let stats = rebuild(&mut f.conn, &test_config()).unwrap();
        assert_eq!(stats.commits_unlinked, 1);
        let links: i64 = f
            .conn
            .query_row("SELECT count(*) FROM commit_links", [], |r| r.get(0))
            .unwrap();
        assert_eq!(links, 0);
    }

    #[test]
    fn summarizes_a_session_with_its_title_tokens_and_files() {
        let mut f = Fixture::new("/w/proj");
        let base = 1_780_000_000_000;
        f.prompt(base, "s1", "do the thing");
        f.tool(
            base + MIN,
            "s1",
            "Edit",
            json!({ "file_path": "/w/proj/src/a.rs" }),
        );
        f.tool(
            base + 2 * MIN,
            "s1",
            "Read",
            json!({ "file_path": "/w/proj/src/b.rs" }),
        );
        f.add("ai-title", base + 3 * MIN, Some("s1"), json!({ "aiTitle": "Do the thing" }));
        // A tool result is the harness replying, not a prompt.
        f.add(
            "user",
            base + 4 * MIN,
            Some("s1"),
            json!({ "message": { "content": [{ "type": "tool_result", "content": "ok" }] } }),
        );

        rebuild(&mut f.conn, &test_config()).unwrap();

        let (title, prompts, tools, input, output): (String, i64, i64, i64, i64) = f
            .conn
            .query_row(
                "SELECT title, prompts, tool_calls, input_tokens, output_tokens FROM sessions",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(title, "Do the thing");
        assert_eq!(prompts, 1, "the tool result must not count as a prompt");
        assert_eq!(tools, 2);
        assert_eq!(input, 20);
        assert_eq!(output, 40);

        let writes: i64 = f
            .conn
            .query_row(
                "SELECT writes FROM session_files WHERE path='/w/proj/src/a.rs'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(writes, 1);
    }

    /// The defect: Lanes carries bars, not blocks, so selecting one re-ran the
    /// whole range query to find a single row and rendered whatever it found.
    /// Reading one block has to answer exactly what the range answers for it,
    /// or the pane and the timeline would describe the same block differently.
    #[test]
    fn one_block_reads_the_same_as_the_range_does() {
        let mut f = Fixture::new("/w/proj");
        let base = 1_780_000_000_000;
        f.prompt(base, "s1", "hi");
        f.tool(base + MIN, "s1", "Write", json!({ "file_path": "/w/proj/a.rs" }));
        f.commit(base + 2 * MIN, "ccc333", "feat: a", &["a.rs"]);
        // A second block, an idle gap later, so a wrong id cannot pass by being
        // the only row in the table.
        f.prompt(base + 90 * MIN, "s2", "again");

        let config = test_config();
        rebuild(&mut f.conn, &config).unwrap();

        let range =
            crate::api::timeline(&f.conn, &config, base - MIN, base + 200 * MIN, None, None, None)
                .unwrap();
        assert_eq!(range.len(), 2, "the idle gap should have split the work");

        for want in &range {
            let one = crate::api::block(&f.conn, &config, want.id)
                .unwrap()
                .expect("a block the range returned must be readable on its own");
            assert_eq!(
                serde_json::to_value(&one).unwrap(),
                serde_json::to_value(want).unwrap(),
                "block {} differs between the single read and the range",
                want.id
            );
        }

        // A selection can outlive the block it names, and `None` is the answer
        // the pane needs in order to say so rather than show the last one.
        assert!(crate::api::block(&f.conn, &config, 9_999).unwrap().is_none());
    }

    /// The digest prints these four beneath "across projects" with no caveat, so
    /// they have to add up. They split `project_ms` and not `elapsed_ms` for
    /// exactly that reason: elapsed merges overlapping blocks, so splitting it
    /// would total more than the whole.
    #[test]
    fn the_evidence_split_sums_to_the_counted_time() {
        let mut f = Fixture::new("/w/proj");
        let base = 1_780_000_000_000;
        // One block per class, each an idle gap apart so they stay separate.
        f.prompt(base, "s1", "hi");
        f.prompt(base + 3 * MIN, "s1", "more");
        f.commit(base + 90 * MIN, "ccc333", "feat: a", &["a.rs"]);
        f.commit(base + 95 * MIN, "ddd444", "feat: b", &["b.rs"]);

        let config = test_config();
        rebuild(&mut f.conn, &config).unwrap();

        let from = base - MIN;
        let to = base + 300 * MIN;
        let s = crate::api::summary(&f.conn, &config, from, to, None, None).unwrap();
        let e = &s.by_evidence;

        assert_eq!(
            e.sessions + e.commits + e.saves + e.bare,
            s.project_ms,
            "the four parts must account for the counted time exactly"
        );
        assert!(e.sessions > 0, "the prompt block is evidenced by a session");
        assert!(e.commits > 0, "the commit block has no session to claim it");
        assert_eq!(e.saves, 0);

        // And the classifier the digest uses is the one the lane bars are
        // stamped with — not a second copy that can drift from it.
        assert_eq!(crate::api::evidence_of(1, 4, 9), "sessions");
        assert_eq!(crate::api::evidence_of(0, 2, 9), "commits");
        assert_eq!(crate::api::evidence_of(0, 0, 1), "saves");
        assert_eq!(crate::api::evidence_of(0, 0, 0), "bare");
    }

    /// The digest is narrowed the same way the timeline is. Unfiltered, it printed
    /// a confident total for every project while the view below showed one.
    #[test]
    fn the_summary_narrows_to_the_filtered_project() {
        let mut f = Fixture::new("/w/alpha");
        let base = 1_780_000_000_000;
        f.prompt(base, "s1", "alpha work");
        f.prompt(base + 3 * MIN, "s1", "more");
        // A second project, an idle gap away, so the two never share a block.
        f.project("/w/beta");
        f.prompt(base + 90 * MIN, "s2", "beta work");
        f.prompt(base + 93 * MIN, "s2", "more beta");

        let config = test_config();
        rebuild(&mut f.conn, &config).unwrap();
        let (from, to) = (base - MIN, base + 300 * MIN);

        let all = crate::api::summary(&f.conn, &config, from, to, None, None).unwrap();
        let alpha =
            crate::api::summary(&f.conn, &config, from, to, Some("/w/alpha"), None).unwrap();
        let beta = crate::api::summary(&f.conn, &config, from, to, Some("/w/beta"), None).unwrap();

        assert_eq!(all.sessions, 2, "both projects are in range unfiltered");
        assert_eq!(alpha.sessions, 1, "the filter has to reach the census too");
        assert_eq!(beta.sessions, 1);
        assert_eq!(alpha.projects, 1);
        assert_eq!(
            alpha.project_ms + beta.project_ms,
            all.project_ms,
            "the parts of a partition must still sum to the whole"
        );
        assert_eq!(
            alpha.by_evidence.sessions + beta.by_evidence.sessions,
            all.by_evidence.sessions,
            "the evidence split narrows with everything else"
        );

        // A filter admitting no project reports zero, not everything.
        let none =
            crate::api::summary(&f.conn, &config, from, to, Some("/w/nope"), None).unwrap();
        assert_eq!(none.blocks, 0);
        assert_eq!(none.sessions, 0);
        assert_eq!(none.project_ms, 0);
    }

    /// The defect: a file the assistant wrote *and* the filesystem later recorded
    /// counted twice in the denominator, once on each side, so the printed share
    /// fell as the two sources agreed more.
    #[test]
    fn ai_share_counts_each_file_once() {
        let mut f = Fixture::new("/w/proj");
        let base = 1_780_000_000_000;
        f.prompt(base, "s1", "hi");
        f.tool(base + MIN, "s1", "Write", json!({ "file_path": "/w/proj/a.rs" }));
        // The same file, seen again as a filesystem change: one file, two sources.
        f.file_change(base + 2 * MIN, "/w/proj/a.rs");

        let config = test_config();
        rebuild(&mut f.conn, &config).unwrap();
        let s = crate::api::summary(&f.conn, &config, base - MIN, base + 60 * MIN, None, None)
            .unwrap();

        assert_eq!(
            s.ai_share,
            Some(100.0),
            "one distinct file, written by the assistant — added rather than unioned \
             this reported 50%"
        );
    }

    #[test]
    fn rebuilding_twice_produces_the_same_result() {
        let mut f = Fixture::new("/w/proj");
        let base = 1_780_000_000_000;
        f.prompt(base, "s1", "hi");
        f.tool(base + MIN, "s1", "Write", json!({ "file_path": "/w/proj/a.rs" }));
        f.commit(base + 2 * MIN, "ccc333", "feat: a", &["a.rs"]);

        let first = rebuild(&mut f.conn, &test_config()).unwrap();
        let second = rebuild(&mut f.conn, &test_config()).unwrap();
        assert_eq!(first.blocks, second.blocks);
        assert_eq!(first.sessions, second.sessions);
        assert_eq!(first.commits, second.commits);
        assert_eq!(first.links_strong, second.links_strong);

        let counts: (i64, i64, i64) = f
            .conn
            .query_row(
                "SELECT (SELECT count(*) FROM blocks),
                        (SELECT count(*) FROM commits),
                        (SELECT count(*) FROM commit_links)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(counts, (1, 1, 1));
    }

    #[test]
    fn a_deleted_project_keeps_the_grouping_decided_while_it_existed() {
        use crate::config::Root;

        let dir = std::env::temp_dir().join(format!("athar-fold-{}", std::process::id()));
        let root = dir.join("freelance");
        let northwind = root.join("clients/northwind");
        std::fs::create_dir_all(northwind.join(".git")).unwrap();
        std::fs::create_dir_all(root.join("clients/acme/.git")).unwrap();

        let config = Config {
            idle_gap_mins: 20,
            roots: vec![Root {
                path: root.clone(),
                category: "freelance".into(),
            }],
            ..Default::default()
        };

        let mut conn = temp_db();
        let origin = db::origin_cursor(&conn, "claude", "/t.jsonl").unwrap().id;
        // Recorded from a subdirectory of the repository, as a session would be.
        let deep = northwind.join("src");
        let project = db::project_id(&conn, &deep.to_string_lossy()).unwrap();
        conn.execute(
            "INSERT INTO raw_records (origin_id, line_no, ts_ms, kind, session_id, project_id, json, bytes_original)
             VALUES (?1, 1, 1780000000000, 'user', 's1', ?2, '{}', 0)",
            (origin, project),
        )
        .unwrap();

        rebuild(&mut conn, &config).unwrap();
        let folded: String = conn
            .query_row(
                "SELECT p.path FROM blocks b JOIN projects p ON p.id = b.project_id",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(folded, northwind.to_string_lossy(), "folds into its repository");

        // The project is deleted. Its repository can no longer be seen, so a
        // recomputed fold would land on `clients` and merge this history with
        // acme's. The remembered decision must hold instead.
        std::fs::remove_dir_all(&northwind).unwrap();
        rebuild(&mut conn, &config).unwrap();
        let after: String = conn
            .query_row(
                "SELECT p.path FROM blocks b JOIN projects p ON p.id = b.project_id",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            after,
            northwind.to_string_lossy(),
            "a deleted project must not be re-folded into its parent folder"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// The prompts of a session Claude Code has deleted are still countable: the
    /// history file survives, and reporting zero would understate the archive at
    /// exactly the point where it is the only copy left.
    #[test]
    fn a_transcript_less_session_counts_its_history_prompts() {
        let mut f = Fixture::new("/w/proj");
        let base = 1_780_000_000_000;
        for (i, text) in ["first", "second", "third"].iter().enumerate() {
            f.add(
                "prompt_history",
                base + i as i64 * MIN,
                Some("gone"),
                json!({ "display": text, "timestamp": base }),
            );
        }
        rebuild(&mut f.conn, &test_config()).unwrap();

        let (prompts, transcript): (i64, i64) = f
            .conn
            .query_row(
                "SELECT prompts, has_transcript FROM sessions WHERE session_id='gone'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(prompts, 3, "history is the only record of what was asked");
        assert_eq!(transcript, 0);
    }

    /// And where a transcript survives, the history is the same prompts again.
    #[test]
    fn a_transcript_session_does_not_count_history_twice() {
        let mut f = Fixture::new("/w/proj");
        let base = 1_780_000_000_000;
        f.prompt(base, "kept", "do the thing");
        f.add(
            "prompt_history",
            base,
            Some("kept"),
            json!({ "display": "do the thing", "timestamp": base }),
        );
        rebuild(&mut f.conn, &test_config()).unwrap();

        let prompts: i64 = f
            .conn
            .query_row(
                "SELECT prompts FROM sessions WHERE session_id='kept'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(prompts, 1, "one prompt recorded twice is still one prompt");
    }

    #[test]
    fn a_session_surviving_only_as_prompt_history_is_marked() {
        let mut f = Fixture::new("/w/proj");
        let base = 1_780_000_000_000;
        f.add(
            "prompt_history",
            base,
            Some("gone"),
            json!({ "display": "an old prompt", "timestamp": base }),
        );
        f.prompt(base + MIN, "kept", "a live one");

        rebuild(&mut f.conn, &test_config()).unwrap();
        let gone: i64 = f
            .conn
            .query_row(
                "SELECT has_transcript FROM sessions WHERE session_id='gone'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let kept: i64 = f
            .conn
            .query_row(
                "SELECT has_transcript FROM sessions WHERE session_id='kept'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(gone, 0);
        assert_eq!(kept, 1);
    }
}
