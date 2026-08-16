//! Git collector.
//!
//! Reads commit metadata — never content. Measured on real repositories,
//! printing every patch as text came to 5.9 MB for a 194-commit repo and 221 MB
//! for a 2,182-commit repo, against 133 KB and 2.4 MB of metadata. Git already
//! stores the code, compressed, two directories away; athar asks it for a diff
//! when someone opens a commit.
//!
//! Two things make this worth archiving rather than querying live:
//!
//!   - Git garbage-collects unreachable commits about 30 days after a branch is
//!     deleted or a history rewritten. Reading `--reflog` catches that work while
//!     it still exists, and athar then keeps it permanently. Records carry
//!     `unreachable: true` so a deleted branch's commits stay identifiable.
//!   - Repositories get deleted, and their history goes with them.
//!
//! The collector shells out to `git` rather than linking a library: `git` is
//! present by definition wherever there are repositories to read, and it handles
//! packed refs, worktrees and reflog expiry correctly without athar reimplementing
//! any of it.

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::db;
use crate::truncate;

pub const SOURCE: &str = "git";

/// Bumped whenever the archived record's shape changes.
///
/// It is part of every repository's fingerprint, so improving this adapter re-reads
/// history that is still on disk instead of leaving old records in their old shape.
/// A repository that no longer exists keeps whatever was archived from it — the
/// evidence cannot be re-read, and athar does not discard it.
const ADAPTER_VERSION: u32 = 2;

/// Commits touching more files than this store a marker instead of the full list.
///
/// Set from the measured shape of real history: the largest genuine commit in
/// these repositories touches 1,168 files, while the outliers above this line are
/// vendored dependency drops — a single "remove pods from source" commit moves
/// 17,349 files under `ios/`. Removing the cap entirely would triple the per-file
/// table for five commits of no evidentiary value; 500 was low enough to truncate
/// real work. Whatever is dropped is counted in `files_omitted`, never silently.
const MAX_FILES_PER_COMMIT: usize = 2_000;

/// Field and record separators for `git log --format`. Neither appears in commit
/// messages in practice, which lets a message keep its newlines.
const RS: char = '\u{1e}';
const US: char = '\u{1f}';

#[derive(Debug, Default, Clone, Copy)]
pub struct GitStats {
    pub repos_seen: usize,
    pub repos_read: usize,
    pub repos_unchanged: usize,
    pub repos_without_identity: usize,
    pub commits_inserted: u64,
    pub commits_known: u64,
    pub commits_refreshed: u64,
    pub commits_unreachable: u64,
    pub commits_foreign: u64,
    pub errors: usize,
}

impl GitStats {
    fn merge(&mut self, other: GitStats) {
        self.repos_seen += other.repos_seen;
        self.repos_read += other.repos_read;
        self.repos_unchanged += other.repos_unchanged;
        self.repos_without_identity += other.repos_without_identity;
        self.commits_inserted += other.commits_inserted;
        self.commits_known += other.commits_known;
        self.commits_refreshed += other.commits_refreshed;
        self.commits_unreachable += other.commits_unreachable;
        self.commits_foreign += other.commits_foreign;
        self.errors += other.errors;
    }
}

/// Repository roots under the configured roots.
///
/// A directory containing `.git` is a repository and is not descended into, so a
/// repo's own subdirectories are not re-scanned. Excluded directory names are
/// pruned, which matters because dependency trees contain their own `.git`
/// directories that are nobody's work.
pub fn discover(roots: &[PathBuf], exclude: &[String]) -> Vec<PathBuf> {
    let excluded: HashSet<&str> = exclude.iter().map(String::as_str).collect();
    let mut found = Vec::new();

    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            if dir.join(".git").exists() {
                found.push(dir);
                continue;
            }
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                if !entry.file_type().is_ok_and(|t| t.is_dir()) {
                    continue;
                }
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with('.') || excluded.contains(name.as_ref()) {
                    continue;
                }
                stack.push(entry.path());
            }
        }
    }

    found.sort();
    found
}

pub fn scan(conn: &mut Connection, config: &Config) -> Result<GitStats> {
    let roots: Vec<PathBuf> = config.roots.iter().map(|r| r.path.clone()).collect();
    let mut total = GitStats::default();

    for repo in discover(&roots, &config.exclude) {
        total.repos_seen += 1;
        match ingest_repo(conn, &repo, &config.identities) {
            Ok(s) => total.merge(s),
            Err(_) => total.errors += 1,
        }
    }

    Ok(total)
}

pub fn ingest_repo(conn: &mut Connection, repo: &Path, extra_identities: &[String]) -> Result<GitStats> {
    let mut stats = GitStats::default();
    let key = repo.to_string_lossy().to_string();

    let identities = identities(repo, extra_identities);
    if identities.is_empty() {
        // Archiving every commit in a clone would fill the record with other
        // people's work, so a repository with no known identity is left alone.
        stats.repos_without_identity += 1;
        return Ok(stats);
    }

    // Skipping unchanged repositories keeps a rescan cheap: reading a large
    // history costs far more than fingerprinting its refs. The identity set is
    // part of the fingerprint, so declaring another address of your own re-reads
    // every repository instead of silently leaving that work unarchived.
    let fingerprint = fingerprint(repo, &identities)?;
    let meta_key = format!("git:refs:{key}");
    if read_meta(conn, &meta_key)?.as_deref() == Some(fingerprint.as_str()) {
        stats.repos_unchanged += 1;
        return Ok(stats);
    }

    let reachable = reachable_shas(repo)?;
    let commits = read_commits(repo)?;

    let origin = db::origin_cursor(conn, SOURCE, &key)?;
    let project_id = db::project_id(conn, &key)?;

    let tx = conn.transaction()?;
    {
        let mut insert = tx.prepare(
            "INSERT OR IGNORE INTO raw_records
                 (origin_id, ext_id, ts_ms, kind, project_id, json,
                  bytes_original, truncated)
             VALUES (?1, ?2, ?3, 'commit', ?4, ?5, ?6, ?7)",
        )?;
        let mut refresh = tx.prepare(
            "UPDATE raw_records
                SET json = ?3, bytes_original = ?4, truncated = ?5, ts_ms = ?6
              WHERE origin_id = ?1 AND ext_id = ?2 AND json <> ?3",
        )?;

        for commit in commits {
            if !identities.contains(&commit.author_email.to_lowercase()) {
                stats.commits_foreign += 1;
                continue;
            }

            let unreachable = !reachable.contains(&commit.sha);
            if unreachable {
                stats.commits_unreachable += 1;
            }

            let mut value = commit.to_json(unreachable);
            let bytes_original = value.to_string().len();
            let truncated = truncate::apply(&mut value);

            let changed = insert.execute((
                origin.id,
                &commit.sha,
                commit.authored_at_ms,
                project_id,
                value.to_string(),
                bytes_original as i64,
                truncated as i64,
            ))?;
            if changed == 1 {
                stats.commits_inserted += 1;
            } else {
                // Already archived. Refresh it only if this adapter produces a
                // different record than the one on file.
                let updated = refresh.execute((
                    origin.id,
                    &commit.sha,
                    value.to_string(),
                    bytes_original as i64,
                    truncated as i64,
                    commit.authored_at_ms,
                ))?;
                if updated == 1 {
                    stats.commits_refreshed += 1;
                } else {
                    stats.commits_known += 1;
                }
            }
        }

        write_meta(&tx, &meta_key, &fingerprint)?;
    }
    tx.commit()?;

    stats.repos_read += 1;
    Ok(stats)
}

/// The identities whose commits count as the user's own. Repository scope
/// includes global config, so a per-repo work address is picked up automatically.
pub fn identities(repo: &Path, extra: &[String]) -> HashSet<String> {
    let mut set: HashSet<String> = extra.iter().map(|e| e.to_lowercase()).collect();
    if let Ok(out) = git(repo, &["config", "--get-all", "user.email"]) {
        for line in out.lines() {
            let line = line.trim();
            if !line.is_empty() {
                set.insert(line.to_lowercase());
            }
        }
    }
    set
}

/// A cheap summary of everything that could change what gets archived: every
/// ref, the size of the HEAD reflog (which grows on checkouts and commits
/// alike), and the identity set that decides which commits are the user's own.
fn fingerprint(repo: &Path, identities: &HashSet<String>) -> Result<String> {
    let refs = git(repo, &["for-each-ref", "--format=%(objectname) %(refname)"])?;
    let reflog_size = ["logs/HEAD", "../logs/HEAD"]
        .iter()
        .map(|p| repo.join(".git").join(p))
        .find_map(|p| std::fs::metadata(p).ok().map(|m| m.len()))
        .unwrap_or(0);

    let mut sorted: Vec<&str> = identities.iter().map(String::as_str).collect();
    sorted.sort_unstable();

    let mut hasher = Sha256::new();
    hasher.update(ADAPTER_VERSION.to_le_bytes());
    hasher.update(refs.as_bytes());
    hasher.update(reflog_size.to_le_bytes());
    for id in sorted {
        hasher.update(id.as_bytes());
        hasher.update(b"\x00");
    }
    Ok(truncate::hex(&hasher.finalize())[..32].to_string())
}

fn reachable_shas(repo: &Path) -> Result<HashSet<String>> {
    let out = git(repo, &["rev-list", "--all"])?;
    Ok(out.lines().map(str::to_string).collect())
}

#[derive(Debug)]
struct Commit {
    sha: String,
    parents: Vec<String>,
    authored_at_ms: i64,
    committed_at_ms: i64,
    author_name: String,
    author_email: String,
    refs_at_scan: String,
    branch_from_reflog: Option<String>,
    message: String,
    files: Vec<FileChange>,
    files_omitted: usize,
}

#[derive(Debug)]
struct FileChange {
    path: String,
    added: Option<u64>,
    deleted: Option<u64>,
}

impl Commit {
    fn to_json(&self, unreachable: bool) -> Value {
        let files: Vec<Value> = self
            .files
            .iter()
            .map(|f| json!({ "path": f.path, "added": f.added, "deleted": f.deleted }))
            .collect();

        let mut value = json!({
            "sha": self.sha,
            "parents": self.parents,
            // Author time is when the work was written. Committer time changes on
            // rebase and amend, so both are kept and the timeline uses author.
            "authored_at_ms": self.authored_at_ms,
            "committed_at_ms": self.committed_at_ms,
            "author_name": self.author_name,
            "author_email": self.author_email,
            // What pointed at this commit when athar scanned — not the branch it
            // was made on, which a commit object does not record.
            "refs_at_scan": self.refs_at_scan,
            // The branch it was actually made on, recovered from the reflog. Only
            // available while the reflog still holds the entry, and only on the
            // machine where the commit happened.
            "branch": self.branch_from_reflog,
            "message": self.message,
            "files": files,
            // A commit no ref can reach: a deleted branch or rewritten history.
            // Git will collect it; athar has already kept it.
            "unreachable": unreachable,
        });
        if self.files_omitted > 0 {
            value["files_omitted"] = json!(self.files_omitted);
        }
        value
    }
}

/// The branch each commit was actually made on, from the per-branch reflogs.
///
/// A commit object records no branch, so this is the only place the answer exists
/// — and only while the reflog still holds the entry, on the machine where the
/// commit happened. Absent for anything older than the reflog window, which is
/// honest: athar does not guess a branch it cannot see.
fn reflog_branches(repo: &Path) -> HashMap<String, String> {
    let Ok(out) = git(repo, &["reflog", "--all", "--format=%H\x1f%gD\x1f%gs"]) else {
        return HashMap::new();
    };

    let mut out_map = HashMap::new();
    for line in out.lines() {
        let mut parts = line.split('\u{1f}');
        let (Some(sha), Some(selector), Some(subject)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        // Only entries that created a commit, and only on a real branch.
        if !subject.starts_with("commit") {
            continue;
        }
        let Some(rest) = selector.strip_prefix("refs/heads/") else {
            continue;
        };
        let branch = rest.split('@').next().unwrap_or(rest).trim();
        if branch.is_empty() {
            continue;
        }
        // The most recent entry for a sha wins; earlier ones are rewrites.
        out_map.entry(sha.to_string()).or_insert_with(|| branch.to_string());
    }
    out_map
}

/// Reads every commit from every ref *and* the reflog, so work on deleted
/// branches and pre-rebase history is archived while it still exists.
fn read_commits(repo: &Path) -> Result<Vec<Commit>> {
    let reflog_branches = reflog_branches(repo);
    let format = format!(
        "{RS}%H{US}%P{US}%at{US}%ct{US}%an{US}%ae{US}%D{US}%B{US}"
    );
    let out = git(
        repo,
        &[
            "log",
            "--all",
            "--reflog",
            "--no-abbrev",
            "--numstat",
            &format!("--format={format}"),
        ],
    )?;

    let mut commits = Vec::new();
    for chunk in out.split(RS) {
        if chunk.trim().is_empty() {
            continue;
        }
        let mut fields = chunk.split(US);
        let mut next = || fields.next().unwrap_or_default().to_string();

        let sha = next();
        let parents = next();
        let authored = next();
        let committed = next();
        let author_name = next();
        let author_email = next();
        let refs_at_scan = next();
        let message = next();
        let numstat = next();

        if sha.len() < 40 {
            continue;
        }

        let mut files = Vec::new();
        let mut files_omitted = 0;
        for line in numstat.lines() {
            let mut parts = line.split('\t');
            let (Some(a), Some(d), Some(path)) = (parts.next(), parts.next(), parts.next()) else {
                continue;
            };
            if files.len() >= MAX_FILES_PER_COMMIT {
                files_omitted += 1;
                continue;
            }
            files.push(FileChange {
                path: path.to_string(),
                // Binary files report `-` rather than a count.
                added: a.parse().ok(),
                deleted: d.parse().ok(),
            });
        }

        let branch_from_reflog = reflog_branches.get(&sha).cloned();
        commits.push(Commit {
            sha,
            parents: parents.split_whitespace().map(str::to_string).collect(),
            authored_at_ms: authored.trim().parse::<i64>().unwrap_or_default() * 1000,
            committed_at_ms: committed.trim().parse::<i64>().unwrap_or_default() * 1000,
            author_name,
            author_email: author_email.trim().to_string(),
            refs_at_scan,
            branch_from_reflog,
            message: message.trim_end().to_string(),
            files,
            files_omitted,
        });
    }

    Ok(commits)
}

fn read_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
        .optional()?)
}

fn write_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        (key, value),
    )?;
    Ok(())
}

fn git<S: AsRef<OsStr>>(repo: &Path, args: &[S]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .with_context(|| format!("running git in {}", repo.display()))?;
    if !out.status.success() {
        bail!(
            "git failed in {}: {}",
            repo.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Repositories keyed by path, for callers that need the category too.
pub fn by_category(config: &Config) -> HashMap<String, Vec<PathBuf>> {
    let roots: Vec<PathBuf> = config.roots.iter().map(|r| r.path.clone()).collect();
    let mut out: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for repo in discover(&roots, &config.exclude) {
        let category = config.category_of(&repo).unwrap_or("uncategorized").to_string();
        out.entry(category).or_default().push(repo);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "athar-git-test-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    fn init_repo(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        git(dir, &["init", "-q", "-b", "main"]).unwrap();
        git(dir, &["config", "user.email", "me@example.com"]).unwrap();
        git(dir, &["config", "user.name", "Me"]).unwrap();
        git(dir, &["config", "commit.gpgsign", "false"]).unwrap();
    }

    fn commit(dir: &Path, file: &str, body: &str, message: &str) {
        std::fs::write(dir.join(file), body).unwrap();
        git(dir, &["add", "-A"]).unwrap();
        git(dir, &["commit", "-q", "-m", message]).unwrap();
    }

    #[test]
    fn archives_commits_with_metadata_but_no_content() {
        let dir = temp();
        let repo = dir.join("repo");
        init_repo(&repo);
        commit(&repo, "a.txt", "one\ntwo\n", "feat: add a");

        let mut conn = db::open_writable(&dir.join("athar.db")).unwrap();
        let stats = ingest_repo(&mut conn, &repo, &[]).unwrap();
        assert_eq!(stats.commits_inserted, 1);

        let json: String = conn
            .query_row("SELECT json FROM raw_records WHERE kind='commit'", [], |r| r.get(0))
            .unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["message"], "feat: add a");
        assert_eq!(v["files"][0]["path"], "a.txt");
        assert_eq!(v["files"][0]["added"], 2);
        assert_eq!(v["unreachable"], false);
        // The file's contents appear nowhere.
        assert!(!json.contains("one"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rescanning_an_unchanged_repo_reads_nothing() {
        let dir = temp();
        let repo = dir.join("repo");
        init_repo(&repo);
        commit(&repo, "a.txt", "x", "one");

        let mut conn = db::open_writable(&dir.join("athar.db")).unwrap();
        ingest_repo(&mut conn, &repo, &[]).unwrap();
        let second = ingest_repo(&mut conn, &repo, &[]).unwrap();
        assert_eq!(second.repos_unchanged, 1);
        assert_eq!(second.commits_inserted, 0);

        // A new commit is picked up, and the old one is not duplicated.
        commit(&repo, "b.txt", "y", "two");
        let third = ingest_repo(&mut conn, &repo, &[]).unwrap();
        assert_eq!(third.commits_inserted, 1);
        let total: i64 = conn
            .query_row("SELECT count(*) FROM raw_records", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 2);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn keeps_commits_from_a_deleted_branch_and_marks_them_unreachable() {
        let dir = temp();
        let repo = dir.join("repo");
        init_repo(&repo);
        commit(&repo, "a.txt", "x", "base");

        git(&repo, &["checkout", "-q", "-b", "throwaway"]).unwrap();
        commit(&repo, "b.txt", "y", "work that gets abandoned");
        git(&repo, &["checkout", "-q", "main"]).unwrap();
        git(&repo, &["branch", "-qD", "throwaway"]).unwrap();

        let mut conn = db::open_writable(&dir.join("athar.db")).unwrap();
        let stats = ingest_repo(&mut conn, &repo, &[]).unwrap();
        assert_eq!(stats.commits_inserted, 2);
        assert_eq!(stats.commits_unreachable, 1);

        let msg: String = conn
            .query_row(
                "SELECT json_extract(json,'$.message') FROM raw_records
                  WHERE json_extract(json,'$.unreachable') = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(msg, "work that gets abandoned");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ignores_other_peoples_commits_in_a_clone() {
        let dir = temp();
        let repo = dir.join("repo");
        init_repo(&repo);
        commit(&repo, "mine.txt", "x", "mine");

        // A commit from someone else, as a clone of any shared project contains.
        git(&repo, &["config", "user.email", "someone@else.test"]).unwrap();
        commit(&repo, "theirs.txt", "y", "theirs");
        git(&repo, &["config", "user.email", "me@example.com"]).unwrap();

        let mut conn = db::open_writable(&dir.join("athar.db")).unwrap();
        let stats = ingest_repo(&mut conn, &repo, &[]).unwrap();
        assert_eq!(stats.commits_inserted, 1);
        assert_eq!(stats.commits_foreign, 1);

        // Declaring that address as the user's own re-reads the repository, even
        // though no ref changed, and archives the commit that was skipped.
        let stats = ingest_repo(&mut conn, &repo, &["someone@else.test".to_string()]).unwrap();
        assert_eq!(stats.repos_unchanged, 0, "identity change must invalidate the fingerprint");
        assert_eq!(stats.commits_inserted, 1);
        let total: i64 = conn
            .query_row("SELECT count(*) FROM raw_records", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 2);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn recovers_the_branch_a_commit_was_made_on() {
        let dir = temp();
        let repo = dir.join("repo");
        init_repo(&repo);
        commit(&repo, "a.txt", "x", "on main");
        git(&repo, &["checkout", "-q", "-b", "feature/side"]).unwrap();
        commit(&repo, "b.txt", "y", "on the feature branch");

        let mut conn = db::open_writable(&dir.join("athar.db")).unwrap();
        ingest_repo(&mut conn, &repo, &[]).unwrap();

        // A commit object records no branch; the reflog is the only witness.
        let branch: String = conn
            .query_row(
                "SELECT json_extract(json,'$.branch') FROM raw_records
                  WHERE json_extract(json,'$.message') = 'on the feature branch'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(branch, "feature/side");

        let main_branch: String = conn
            .query_row(
                "SELECT json_extract(json,'$.branch') FROM raw_records
                  WHERE json_extract(json,'$.message') = 'on main'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(main_branch, "main");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn refreshes_a_record_when_the_adapter_changes_but_not_otherwise() {
        let dir = temp();
        let repo = dir.join("repo");
        init_repo(&repo);
        commit(&repo, "a.txt", "x", "only commit");

        let mut conn = db::open_writable(&dir.join("athar.db")).unwrap();
        ingest_repo(&mut conn, &repo, &[]).unwrap();

        // An unchanged repository read by the same adapter refreshes nothing.
        conn.execute("DELETE FROM meta WHERE key LIKE 'git:refs:%'", []).unwrap();
        let again = ingest_repo(&mut conn, &repo, &[]).unwrap();
        assert_eq!(again.commits_refreshed, 0);
        assert_eq!(again.commits_known, 1);

        // A record left in an older shape is brought up to date in place.
        conn.execute(
            "UPDATE raw_records SET json = json_remove(json, '$.branch')",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM meta WHERE key LIKE 'git:refs:%'", []).unwrap();
        let refreshed = ingest_repo(&mut conn, &repo, &[]).unwrap();
        assert_eq!(refreshed.commits_refreshed, 1);

        let total: i64 = conn
            .query_row("SELECT count(*) FROM raw_records", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 1, "refreshing must not duplicate the record");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn keeps_multiline_commit_messages_intact() {
        let dir = temp();
        let repo = dir.join("repo");
        init_repo(&repo);
        std::fs::write(repo.join("a.txt"), "x").unwrap();
        git(&repo, &["add", "-A"]).unwrap();
        git(&repo, &["commit", "-q", "-m", "subject line", "-m", "body line one\nbody line two"]).unwrap();

        let mut conn = db::open_writable(&dir.join("athar.db")).unwrap();
        ingest_repo(&mut conn, &repo, &[]).unwrap();
        let msg: String = conn
            .query_row("SELECT json_extract(json,'$.message') FROM raw_records", [], |r| r.get(0))
            .unwrap();
        assert!(msg.starts_with("subject line"));
        assert!(msg.contains("body line one\nbody line two"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discovery_finds_repos_and_skips_excluded_trees() {
        let dir = temp();
        let root = dir.join("root");
        init_repo(&root.join("project-a"));
        init_repo(&root.join("nested").join("project-b"));
        // A dependency tree with its own repository is nobody's work.
        init_repo(&root.join("project-a").join("node_modules").join("dep"));
        std::fs::create_dir_all(root.join("plain-folder")).unwrap();

        let found = discover(&[root.clone()], &["node_modules".to_string()]);
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|p| p.ends_with("project-a")));
        assert!(found.iter().any(|p| p.ends_with("project-b")));

        std::fs::remove_dir_all(&dir).ok();
    }
}
