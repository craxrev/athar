//! File-change collector.
//!
//! Whether a file is examined depends on its *git state*, not on whether its
//! project uses git:
//!
//!   - **Inside a repository** — only files git cannot already account for:
//!     dirty or untracked, from `git status`. Files matching HEAD are git's job.
//!     This needs no walk of tracked trees, and it removes mtime false positives
//!     by definition rather than by heuristic: `git checkout`, rebase, `npm
//!     install` and build steps rewrite modified-times but leave files *clean*.
//!     What it captures is the work git cannot see — an afternoon of editing that
//!     was never committed.
//!
//!   - **Outside a repository** — mtime is the only evidence in existence, so the
//!     tree is walked in full. This is not a marginal case: of the project
//!     directories measured under the configured roots, 43 have no git at all,
//!     so for most projects this collector is the entire record.
//!
//! Timestamps here are exact — an mtime is the real time of the last save. What
//! is incomplete is *coverage*: three saves inside one scan interval leave one
//! mtime, so a per-file change count is a floor, never a total. Record identity
//! is `(path, mtime)`, which makes rescanning idempotent while letting a file
//! accumulate one record per change lore actually witnessed.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde_json::json;

use crate::config::Config;
use crate::db;

pub const SOURCE: &str = "file";

#[derive(Debug, Default, Clone, Copy)]
pub struct FileStats {
    pub projects_seen: usize,
    pub repos: usize,
    pub non_git: usize,
    pub dirs_walked: u64,
    pub files_examined: u64,
    pub changes_recorded: u64,
    pub changes_known: u64,
    pub skipped_old: u64,
    pub errors: usize,
}

impl FileStats {
    fn merge(&mut self, other: FileStats) {
        self.projects_seen += other.projects_seen;
        self.repos += other.repos;
        self.non_git += other.non_git;
        self.dirs_walked += other.dirs_walked;
        self.files_examined += other.files_examined;
        self.changes_recorded += other.changes_recorded;
        self.changes_known += other.changes_known;
        self.skipped_old += other.skipped_old;
        self.errors += other.errors;
    }
}

/// How a file's change was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Tracked by git and differing from HEAD: edited but not committed.
    Dirty,
    /// Inside a repository but not tracked.
    Untracked,
    /// No repository at all — mtime is the only record that exists.
    NoRepo,
}

impl State {
    fn as_str(self) -> &'static str {
        match self {
            State::Dirty => "dirty",
            State::Untracked => "untracked",
            State::NoRepo => "no-repo",
        }
    }
}

#[derive(Debug)]
struct Change {
    relative: String,
    mtime_ms: i64,
    size: u64,
    state: State,
}

pub fn scan(conn: &mut Connection, config: &Config) -> Result<FileStats> {
    let cutoff_ms = cutoff(config.file_lookback_days);
    let excluded: HashSet<&str> = config.exclude.iter().map(String::as_str).collect();
    let mut total = FileStats::default();

    for root in &config.roots {
        if !root.path.is_dir() {
            continue;
        }
        // A project is a top-level entry under a root, which is how these are
        // organized and how the user refers to them.
        let Ok(entries) = std::fs::read_dir(&root.path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !entry.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || excluded.contains(name.as_ref()) {
                continue;
            }
            total.projects_seen += 1;
            match ingest_project(conn, &path, &excluded, cutoff_ms) {
                Ok(s) => total.merge(s),
                Err(_) => total.errors += 1,
            }
        }
    }

    Ok(total)
}

fn cutoff(lookback_days: u64) -> i64 {
    let now = chrono::Utc::now().timestamp_millis();
    now - (lookback_days as i64) * 86_400_000
}

/// Archives one project's changes. A repository nested inside a non-git tree is
/// handled as its own project, so its tracked files are never walked.
pub fn ingest_project(
    conn: &mut Connection,
    project: &Path,
    excluded: &HashSet<&str>,
    cutoff_ms: i64,
) -> Result<FileStats> {
    let mut stats = FileStats::default();
    let mut changes = Vec::new();
    let mut nested = Vec::new();

    if project.join(".git").exists() {
        stats.repos += 1;
        changes = git_changes(project, cutoff_ms, &mut stats)?;
    } else {
        stats.non_git += 1;
        walk(project, project, excluded, cutoff_ms, &mut changes, &mut nested, &mut stats);
    }

    write_changes(conn, project, &changes, &mut stats)?;

    for repo in nested {
        stats.projects_seen += 1;
        match ingest_project(conn, &repo, excluded, cutoff_ms) {
            Ok(s) => stats.merge(s),
            Err(_) => stats.errors += 1,
        }
    }

    Ok(stats)
}

fn write_changes(
    conn: &mut Connection,
    project: &Path,
    changes: &[Change],
    stats: &mut FileStats,
) -> Result<()> {
    if changes.is_empty() {
        return Ok(());
    }
    let key = project.to_string_lossy().to_string();
    let origin = db::origin_cursor(conn, SOURCE, &key)?;
    let project_id = db::project_id(conn, &key)?;

    let tx = conn.transaction()?;
    {
        let mut insert = tx.prepare(
            "INSERT OR IGNORE INTO raw_records
                 (origin_id, ext_id, ts_ms, kind, project_id, json,
                  bytes_original, truncated)
             VALUES (?1, ?2, ?3, 'file_change', ?4, ?5, ?6, 0)",
        )?;

        for change in changes {
            // Identity pairs the path with the mtime, so a rescan that sees the
            // same save is a no-op while a later save becomes a new record.
            let ext_id = format!("{}@{}", change.relative, change.mtime_ms);
            let value = json!({
                "path": change.relative,
                "mtime_ms": change.mtime_ms,
                "size": change.size,
                "state": change.state.as_str(),
                // The count of changes to this file is a floor: saves between two
                // scans leave only the most recent mtime behind.
                "coverage": "last-save-per-scan",
            });
            let body = value.to_string();
            let changed = insert.execute((
                origin.id,
                &ext_id,
                change.mtime_ms,
                project_id,
                &body,
                body.len() as i64,
            ))?;
            if changed == 1 {
                stats.changes_recorded += 1;
            } else {
                stats.changes_known += 1;
            }
        }
    }
    tx.commit()?;
    Ok(())
}

/// Dirty and untracked files, straight from git. Deletions are skipped: a removed
/// file has no mtime, so lore cannot say when it happened and will not guess.
fn git_changes(repo: &Path, cutoff_ms: i64, stats: &mut FileStats) -> Result<Vec<Change>> {
    let out = match git(
        repo,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--no-renames",
        ],
    ) {
        Ok(out) => out,
        Err(_) => {
            stats.errors += 1;
            return Ok(Vec::new());
        }
    };

    let mut changes = Vec::new();
    for entry in out.split('\0') {
        if entry.len() < 4 {
            continue;
        }
        let (code, path) = entry.split_at(3);
        let code = code.trim();
        if code.contains('D') {
            continue;
        }
        let state = if code == "??" {
            State::Untracked
        } else {
            State::Dirty
        };

        let absolute = repo.join(path);
        stats.files_examined += 1;
        let Some((mtime_ms, size)) = stat(&absolute) else {
            continue;
        };
        if mtime_ms < cutoff_ms {
            stats.skipped_old += 1;
            continue;
        }
        changes.push(Change {
            relative: path.to_string(),
            mtime_ms,
            size,
            state,
        });
    }

    Ok(changes)
}

#[allow(clippy::too_many_arguments)]
fn walk(
    dir: &Path,
    project: &Path,
    excluded: &HashSet<&str>,
    cutoff_ms: i64,
    changes: &mut Vec<Change>,
    nested_repos: &mut Vec<PathBuf>,
    stats: &mut FileStats,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        stats.errors += 1;
        return;
    };
    stats.dirs_walked += 1;

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if file_type.is_dir() {
            // Hidden directories are caches and tool state, not work.
            if name.starts_with('.') || excluded.contains(name.as_ref()) {
                continue;
            }
            if path.join(".git").exists() {
                nested_repos.push(path);
                continue;
            }
            walk(&path, project, excluded, cutoff_ms, changes, nested_repos, stats);
        } else if file_type.is_file() {
            stats.files_examined += 1;
            let Some((mtime_ms, size)) = stat(&path) else {
                continue;
            };
            if mtime_ms < cutoff_ms {
                stats.skipped_old += 1;
                continue;
            }
            let relative = path
                .strip_prefix(project)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            changes.push(Change {
                relative,
                mtime_ms,
                size,
                state: State::NoRepo,
            });
        }
    }
}

fn stat(path: &Path) -> Option<(i64, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis() as i64;
    Some((mtime, meta.len()))
}

fn git<S: AsRef<OsStr>>(repo: &Path, args: &[S]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .with_context(|| format!("running git in {}", repo.display()))?;
    if !out.status.success() {
        anyhow::bail!("git status failed in {}", repo.display());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Change counts per project, for reporting.
pub fn per_project(conn: &Connection, limit: i64) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT p.path, count(*) FROM raw_records r
           JOIN projects p ON p.id = r.project_id
          WHERE r.kind = 'file_change'
          GROUP BY p.path ORDER BY count(*) DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// A convenience for tests and callers that want one project's history.
pub fn history(conn: &Connection, project: &str) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT json_extract(r.json,'$.path'), r.ts_ms FROM raw_records r
           JOIN projects p ON p.id = r.project_id
          WHERE r.kind = 'file_change' AND p.path = ?1
          ORDER BY r.ts_ms",
    )?;
    let rows = stmt.query_map([project], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "lore-file-test-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }

    fn excludes() -> HashSet<&'static str> {
        ["node_modules", "target"].into_iter().collect()
    }

    fn init_repo(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        run(dir, &["init", "-q", "-b", "main"]);
        run(dir, &["config", "user.email", "me@example.com"]);
        run(dir, &["config", "user.name", "Me"]);
    }

    fn run(dir: &Path, args: &[&str]) {
        git(dir, args).unwrap();
    }

    fn count(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT count(*) FROM raw_records WHERE kind='file_change'",
            [],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn records_every_file_in_a_project_without_git() {
        let dir = temp();
        let project = dir.join("scratch");
        fs::create_dir_all(project.join("src")).unwrap();
        fs::write(project.join("notes.md"), "hello").unwrap();
        fs::write(project.join("src").join("main.c"), "int main(){}").unwrap();

        let mut conn = db::open_writable(&dir.join("lore.db")).unwrap();
        let stats = ingest_project(&mut conn, &project, &excludes(), 0).unwrap();
        assert_eq!(stats.non_git, 1);
        assert_eq!(stats.changes_recorded, 2);

        let paths: Vec<String> = history(&conn, &project.to_string_lossy())
            .unwrap()
            .into_iter()
            .map(|(p, _)| p)
            .collect();
        assert!(paths.contains(&"notes.md".to_string()));
        assert!(paths.contains(&"src/main.c".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn inside_a_repo_records_only_what_git_cannot_account_for() {
        let dir = temp();
        let repo = dir.join("repo");
        init_repo(&repo);
        fs::write(repo.join("committed.txt"), "v1").unwrap();
        run(&repo, &["add", "-A"]);
        run(&repo, &["commit", "-q", "-m", "first"]);

        // Clean and committed: git's job, not lore's.
        let mut conn = db::open_writable(&dir.join("lore.db")).unwrap();
        let stats = ingest_project(&mut conn, &repo, &excludes(), 0).unwrap();
        assert_eq!(stats.repos, 1);
        assert_eq!(stats.changes_recorded, 0);

        // Edited but never committed: invisible to git, visible only here.
        fs::write(repo.join("committed.txt"), "v2 uncommitted").unwrap();
        fs::write(repo.join("brand-new.txt"), "untracked").unwrap();
        let stats = ingest_project(&mut conn, &repo, &excludes(), 0).unwrap();
        assert_eq!(stats.changes_recorded, 2);

        let states: HashSet<String> = conn
            .prepare("SELECT json_extract(json,'$.state') FROM raw_records WHERE kind='file_change'")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert!(states.contains("dirty"));
        assert!(states.contains("untracked"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_checkout_that_rewrites_mtimes_records_nothing() {
        let dir = temp();
        let repo = dir.join("repo");
        init_repo(&repo);
        fs::write(repo.join("a.txt"), "main version").unwrap();
        run(&repo, &["add", "-A"]);
        run(&repo, &["commit", "-q", "-m", "main"]);
        run(&repo, &["checkout", "-q", "-b", "other"]);
        fs::write(repo.join("a.txt"), "other version").unwrap();
        run(&repo, &["add", "-A"]);
        run(&repo, &["commit", "-q", "-m", "other"]);

        let mut conn = db::open_writable(&dir.join("lore.db")).unwrap();
        ingest_project(&mut conn, &repo, &excludes(), 0).unwrap();
        assert_eq!(count(&conn), 0);

        // Switching branches rewrites the file and its mtime, but leaves it
        // clean. A heuristic would have to guess; git state simply knows.
        run(&repo, &["checkout", "-q", "main"]);
        let stats = ingest_project(&mut conn, &repo, &excludes(), 0).unwrap();
        assert_eq!(stats.changes_recorded, 0);
        assert_eq!(count(&conn), 0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_later_save_is_a_new_record_and_a_rescan_is_a_no_op() {
        let dir = temp();
        let project = dir.join("scratch");
        fs::create_dir_all(&project).unwrap();
        let file = project.join("notes.md");
        fs::write(&file, "first").unwrap();

        let mut conn = db::open_writable(&dir.join("lore.db")).unwrap();
        ingest_project(&mut conn, &project, &excludes(), 0).unwrap();
        assert_eq!(count(&conn), 1);

        // Same mtime seen again: nothing new.
        let again = ingest_project(&mut conn, &project, &excludes(), 0).unwrap();
        assert_eq!(again.changes_recorded, 0);
        assert_eq!(again.changes_known, 1);

        // A save with a later mtime accumulates a second record, so the file has
        // a history rather than a single current state.
        let later = chrono::Utc::now().timestamp_millis() + 5_000;
        fs::write(&file, "second").unwrap();
        set_mtime(&file, later);
        let stats = ingest_project(&mut conn, &project, &excludes(), 0).unwrap();
        assert_eq!(stats.changes_recorded, 1);
        assert_eq!(count(&conn), 2);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn skips_files_older_than_the_lookback_and_prunes_excluded_trees() {
        let dir = temp();
        let project = dir.join("scratch");
        fs::create_dir_all(project.join("node_modules").join("dep")).unwrap();
        fs::create_dir_all(project.join(".cache")).unwrap();
        fs::write(project.join("node_modules").join("dep").join("index.js"), "x").unwrap();
        fs::write(project.join(".cache").join("blob"), "x").unwrap();
        fs::write(project.join("fresh.txt"), "x").unwrap();
        let stale = project.join("ancient.txt");
        fs::write(&stale, "x").unwrap();
        set_mtime(&stale, chrono::Utc::now().timestamp_millis() - 400 * 86_400_000);

        let mut conn = db::open_writable(&dir.join("lore.db")).unwrap();
        let cutoff = cutoff(30);
        let stats = ingest_project(&mut conn, &project, &excludes(), cutoff).unwrap();

        assert_eq!(stats.changes_recorded, 1, "only the fresh file");
        assert_eq!(stats.skipped_old, 1);
        let paths: Vec<String> = history(&conn, &project.to_string_lossy())
            .unwrap()
            .into_iter()
            .map(|(p, _)| p)
            .collect();
        assert_eq!(paths, vec!["fresh.txt".to_string()]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_repo_nested_in_a_plain_tree_is_its_own_project() {
        let dir = temp();
        let project = dir.join("scratch");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("loose.txt"), "x").unwrap();
        let inner = project.join("inner-repo");
        init_repo(&inner);
        fs::write(inner.join("tracked.txt"), "v1").unwrap();
        run(&inner, &["add", "-A"]);
        run(&inner, &["commit", "-q", "-m", "first"]);

        let mut conn = db::open_writable(&dir.join("lore.db")).unwrap();
        let stats = ingest_project(&mut conn, &project, &excludes(), 0).unwrap();
        assert_eq!(stats.non_git, 1);
        assert_eq!(stats.repos, 1, "the nested repository was handled as a repo");

        // The nested repo's committed file was not walked as a loose file.
        let paths: Vec<String> = history(&conn, &project.to_string_lossy())
            .unwrap()
            .into_iter()
            .map(|(p, _)| p)
            .collect();
        assert_eq!(paths, vec!["loose.txt".to_string()]);

        fs::remove_dir_all(&dir).ok();
    }

    fn set_mtime(path: &Path, ms: i64) {
        let secs = ms / 1000;
        let out = Command::new("touch")
            .arg("-t")
            .arg(
                chrono::DateTime::from_timestamp(secs, 0)
                    .unwrap()
                    .format("%Y%m%d%H%M.%S")
                    .to_string(),
            )
            .arg(path)
            .output()
            .unwrap();
        assert!(out.status.success());
    }
}
