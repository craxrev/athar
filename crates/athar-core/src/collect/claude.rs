//! Claude Code collector.
//!
//! Reads transcripts that Claude Code has already written to disk, on a
//! schedule, and copies them somewhere permanent. Nothing is hooked into the
//! harness and nothing watches live, so work done while athar was not running is
//! still archived — the evidence outlives the moment.
//!
//! This matters because the source deletes itself: `cleanupPeriodDays` defaults
//! to 30 days, after which Claude Code removes its own session files at startup.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde_json::Value;

use crate::db;
use crate::truncate;

pub const SOURCE_TRANSCRIPT: &str = "claude";
pub const SOURCE_HISTORY: &str = "claude_history";

#[derive(Debug, Default, Clone, Copy)]
pub struct ScanStats {
    pub files_seen: usize,
    pub files_read: usize,
    pub files_reset: usize,
    pub lines_read: u64,
    pub inserted: u64,
    pub duplicates: u64,
    pub dropped: u64,
    pub unparsed: u64,
    pub truncated: u64,
    pub bytes_read: u64,
}

impl ScanStats {
    fn merge(&mut self, other: ScanStats) {
        self.files_seen += other.files_seen;
        self.files_read += other.files_read;
        self.files_reset += other.files_reset;
        self.lines_read += other.lines_read;
        self.inserted += other.inserted;
        self.duplicates += other.duplicates;
        self.dropped += other.dropped;
        self.unparsed += other.unparsed;
        self.truncated += other.truncated;
        self.bytes_read += other.bytes_read;
    }
}

/// Every transcript under `<claude>/projects`, including sub-agent transcripts,
/// plus the long-lived prompt history file.
pub fn discover(claude_dir: &Path) -> Result<Vec<(&'static str, PathBuf)>> {
    let mut found = Vec::new();

    let projects = claude_dir.join("projects");
    if projects.is_dir() {
        let mut stack = vec![projects];
        while let Some(dir) = stack.pop() {
            let entries = match fs::read_dir(&dir) {
                Ok(e) => e,
                // A directory that vanished mid-scan is normal, not an error.
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                match entry.file_type() {
                    Ok(t) if t.is_dir() => stack.push(path),
                    Ok(t) if t.is_file() => {
                        if path.extension().is_some_and(|e| e == "jsonl") {
                            found.push((SOURCE_TRANSCRIPT, path));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let history = claude_dir.join("history.jsonl");
    if history.is_file() {
        found.push((SOURCE_HISTORY, history));
    }

    found.sort();
    Ok(found)
}

pub fn scan(conn: &mut Connection, claude_dir: &Path) -> Result<ScanStats> {
    let mut total = ScanStats::default();
    for (source, path) in discover(claude_dir)? {
        total.files_seen += 1;
        total.merge(ingest_file(conn, source, &path)?);
    }
    Ok(total)
}

pub fn ingest_file(conn: &mut Connection, source: &str, path: &Path) -> Result<ScanStats> {
    let mut stats = ScanStats::default();
    let key = path.to_string_lossy().to_string();

    let meta = fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
    let size = meta.len();
    let inode = meta.ino();
    let mtime_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default();

    let mut cursor = db::origin_cursor(conn, source, &key)?;

    // Rotation or truncation invalidates line numbering, so that one file is
    // re-read from the start. Transcripts are append-only in normal operation,
    // making this the rare path.
    let rotated = cursor.inode.is_some_and(|i| i != inode) || size < cursor.byte_offset;
    if rotated {
        db::forget_origin(conn, cursor.id)?;
        cursor.byte_offset = 0;
        cursor.line_no = 0;
        stats.files_reset += 1;
    } else if size == cursor.size && mtime_ms == cursor.mtime_ms && cursor.byte_offset >= size {
        return Ok(stats); // unchanged since the last scan
    }

    if cursor.byte_offset >= size {
        return Ok(stats);
    }

    let start_offset = cursor.byte_offset;
    let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    file.seek(SeekFrom::Start(start_offset))?;
    let mut buf = Vec::with_capacity((size - start_offset) as usize);
    file.read_to_end(&mut buf)?;
    stats.files_read += 1;
    stats.bytes_read += buf.len() as u64;

    // A final line without its newline is still being written; leave it for the
    // next scan rather than archiving half a record.
    let complete_len = match buf.iter().rposition(|b| *b == b'\n') {
        Some(idx) => idx + 1,
        None => return Ok(stats),
    };

    // Project paths repeat on nearly every line of a transcript.
    let mut project_ids: HashMap<String, i64> = HashMap::new();

    let tx = conn.transaction()?;
    {
        let mut insert = tx.prepare(
            "INSERT OR IGNORE INTO raw_records
                 (origin_id, line_no, ts_ms, kind, session_id, project_id,
                  json, bytes_original, truncated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;

        let mut line_no = cursor.line_no;
        for raw in buf[..complete_len].split(|b| *b == b'\n') {
            if raw.is_empty() {
                continue;
            }
            line_no += 1;
            stats.lines_read += 1;

            let text = String::from_utf8_lossy(raw);
            let record = Record::parse(&text, source);

            if truncate::is_dropped_kind(&record.kind) {
                stats.dropped += 1;
                continue;
            }
            if record.unparsed {
                stats.unparsed += 1;
            }
            if record.truncated {
                stats.truncated += 1;
            }

            let project_id = match &record.project_path {
                Some(p) => {
                    let id = match project_ids.get(p) {
                        Some(id) => *id,
                        None => {
                            let id = db::project_id(&tx, p)?;
                            project_ids.insert(p.clone(), id);
                            id
                        }
                    };
                    Some(id)
                }
                None => None,
            };

            let changed = insert.execute((
                cursor.id,
                line_no as i64,
                record.ts_ms,
                &record.kind,
                &record.session_id,
                project_id,
                &record.json,
                record.bytes_original as i64,
                record.truncated as i64,
            ))?;
            if changed == 1 {
                stats.inserted += 1;
            } else {
                stats.duplicates += 1;
            }
        }

        cursor.inode = Some(inode);
        cursor.size = size;
        cursor.mtime_ms = mtime_ms;
        cursor.byte_offset = start_offset + complete_len as u64;
        cursor.line_no = line_no;
        db::set_origin_cursor(&tx, &cursor)?;
    }
    tx.commit()?;

    Ok(stats)
}

struct Record {
    kind: String,
    ts_ms: Option<i64>,
    session_id: Option<String>,
    project_path: Option<String>,
    json: String,
    bytes_original: usize,
    truncated: bool,
    unparsed: bool,
}

impl Record {
    fn parse(text: &str, source: &str) -> Self {
        let bytes_original = text.len();
        // `history.jsonl` rows carry no `type` field; naming them here keeps
        // 9,883 prompts from landing in the archive as `_unknown`.
        let default_kind = if source == SOURCE_HISTORY {
            "prompt_history"
        } else {
            "_unknown"
        };

        let mut value: Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => {
                // Unreadable lines are archived rather than discarded: a format
                // athar does not understand yet is still evidence, and the source
                // file will not exist in 30 days to re-read.
                let mut v = Value::String(text.to_string());
                let truncated = truncate::apply(&mut v);
                return Self {
                    kind: "_unparsed".into(),
                    ts_ms: None,
                    session_id: None,
                    project_path: None,
                    json: v.to_string(),
                    bytes_original,
                    truncated,
                    unparsed: true,
                };
            }
        };

        let kind = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or(default_kind)
            .to_string();

        let ts_ms = match value.get("timestamp") {
            Some(Value::String(s)) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.timestamp_millis()),
            // `history.jsonl` stores epoch milliseconds.
            Some(Value::Number(n)) => n.as_i64(),
            _ => None,
        };

        let session_id = value
            .get("sessionId")
            .and_then(Value::as_str)
            .map(str::to_string);

        // Transcripts record the working directory as `cwd`; the prompt history
        // file calls the same thing `project`.
        let project_path = value
            .get("cwd")
            .or_else(|| value.get("project"))
            .and_then(Value::as_str)
            .map(str::to_string);

        let truncated = truncate::apply(&mut value);

        Self {
            kind,
            ts_ms,
            session_id,
            project_path,
            json: value.to_string(),
            bytes_original,
            truncated,
            unparsed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn transcript(dir: &Path, name: &str, lines: &[&str]) -> PathBuf {
        let projects = dir.join("projects").join("-some-project");
        fs::create_dir_all(&projects).unwrap();
        let path = projects.join(name);
        let mut f = File::create(&path).unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
        path
    }

    /// Tests run in parallel, so a wall-clock name is not unique enough — two
    /// tests sharing a directory share a database and each other's rows.
    fn temp() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "athar-test-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }

    fn count(conn: &Connection) -> i64 {
        conn.query_row("SELECT count(*) FROM raw_records", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn archives_records_and_resumes_from_the_offset() {
        let dir = temp();
        let path = transcript(
            &dir,
            "s1.jsonl",
            &[
                r#"{"type":"user","uuid":"a","timestamp":"2026-08-01T10:00:00.000Z","sessionId":"s1","cwd":"/w","message":{"role":"user","content":"hi"}}"#,
                r#"{"type":"assistant","uuid":"b","timestamp":"2026-08-01T10:00:05.000Z","sessionId":"s1","cwd":"/w","message":{"model":"claude-opus-5","content":[{"type":"text","text":"yes"}]}}"#,
            ],
        );

        let mut conn = db::open_writable(&dir.join("athar.db")).unwrap();
        let first = ingest_file(&mut conn, SOURCE_TRANSCRIPT, &path).unwrap();
        assert_eq!(first.inserted, 2);
        assert_eq!(count(&conn), 2);

        // Rescanning an unchanged file reads nothing at all.
        let second = ingest_file(&mut conn, SOURCE_TRANSCRIPT, &path).unwrap();
        assert_eq!(second.bytes_read, 0);
        assert_eq!(second.inserted, 0);

        // Appending archives only the new line.
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(
            f,
            r#"{{"type":"user","uuid":"c","timestamp":"2026-08-01T10:01:00.000Z","sessionId":"s1","cwd":"/w","message":{{"role":"user","content":"more"}}}}"#
        )
        .unwrap();
        let third = ingest_file(&mut conn, SOURCE_TRANSCRIPT, &path).unwrap();
        assert_eq!(third.inserted, 1);
        assert_eq!(count(&conn), 3);

        // The project path was interned once, not stored per record.
        let projects: i64 = conn
            .query_row("SELECT count(*) FROM projects", [], |r| r.get(0))
            .unwrap();
        assert_eq!(projects, 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn leaves_a_half_written_line_for_the_next_scan() {
        let dir = temp();
        let projects = dir.join("projects").join("-p");
        fs::create_dir_all(&projects).unwrap();
        let path = projects.join("s.jsonl");
        fs::write(
            &path,
            "{\"type\":\"user\",\"uuid\":\"a\",\"sessionId\":\"s\"}\n{\"type\":\"user\",\"uuid\":\"b\"",
        )
        .unwrap();

        let mut conn = db::open_writable(&dir.join("athar.db")).unwrap();
        let stats = ingest_file(&mut conn, SOURCE_TRANSCRIPT, &path).unwrap();
        assert_eq!(stats.inserted, 1);

        // Completing the line archives it without duplicating the first.
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, ",\"sessionId\":\"s\"}}").unwrap();
        let stats = ingest_file(&mut conn, SOURCE_TRANSCRIPT, &path).unwrap();
        assert_eq!(stats.inserted, 1);
        assert_eq!(count(&conn), 2);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn drops_transient_kinds_but_archives_unknown_ones() {
        let dir = temp();
        let path = transcript(
            &dir,
            "s2.jsonl",
            &[
                r#"{"type":"attachment","uuid":"a"}"#,
                r#"{"type":"queue-operation","uuid":"b"}"#,
                r#"{"type":"some-future-kind","uuid":"c","payload":{"x":1}}"#,
            ],
        );

        let mut conn = db::open_writable(&dir.join("athar.db")).unwrap();
        let stats = ingest_file(&mut conn, SOURCE_TRANSCRIPT, &path).unwrap();
        assert_eq!(stats.dropped, 2);
        assert_eq!(stats.inserted, 1);

        let kind: String = conn
            .query_row("SELECT kind FROM raw_records", [], |r| r.get(0))
            .unwrap();
        assert_eq!(kind, "some-future-kind");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn archives_lines_it_cannot_parse() {
        let dir = temp();
        let path = transcript(&dir, "s3.jsonl", &["not json at all"]);
        let mut conn = db::open_writable(&dir.join("athar.db")).unwrap();
        let stats = ingest_file(&mut conn, SOURCE_TRANSCRIPT, &path).unwrap();
        assert_eq!(stats.unparsed, 1);
        assert_eq!(stats.inserted, 1);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn re_reads_a_rotated_file_without_duplicating_it() {
        let dir = temp();
        let path = transcript(
            &dir,
            "s4.jsonl",
            &[
                r#"{"type":"user","uuid":"a","sessionId":"s"}"#,
                r#"{"type":"user","uuid":"b","sessionId":"s"}"#,
            ],
        );
        let mut conn = db::open_writable(&dir.join("athar.db")).unwrap();
        ingest_file(&mut conn, SOURCE_TRANSCRIPT, &path).unwrap();
        assert_eq!(count(&conn), 2);

        // Replaced with a shorter file: the old line numbering is meaningless.
        fs::write(&path, "{\"type\":\"user\",\"uuid\":\"z\"}\n").unwrap();
        let stats = ingest_file(&mut conn, SOURCE_TRANSCRIPT, &path).unwrap();
        assert_eq!(stats.files_reset, 1);
        assert_eq!(count(&conn), 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reads_epoch_millis_from_prompt_history() {
        let dir = temp();
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("history.jsonl");
        fs::write(
            &path,
            "{\"display\":\"hello\",\"timestamp\":1786616253369,\"project\":\"/w\",\"sessionId\":\"s\"}\n",
        )
        .unwrap();

        let mut conn = db::open_writable(&dir.join("athar.db")).unwrap();
        ingest_file(&mut conn, SOURCE_HISTORY, &path).unwrap();
        let (ts, kind, project): (i64, String, String) = conn
            .query_row(
                "SELECT r.ts_ms, r.kind, p.path
                   FROM raw_records r JOIN projects p ON p.id = r.project_id",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(ts, 1786616253369);
        assert_eq!(kind, "prompt_history");
        assert_eq!(project, "/w");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discovers_transcripts_including_sub_agents() {
        let dir = temp();
        transcript(&dir, "main.jsonl", &["{}"]);
        let sub = dir.join("projects").join("-some-project").join("subagents");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("agent-1.jsonl"), "{}\n").unwrap();
        fs::write(dir.join("history.jsonl"), "{}\n").unwrap();

        let found = discover(&dir).unwrap();
        assert_eq!(found.len(), 3);
        assert!(found.iter().any(|(s, _)| *s == SOURCE_HISTORY));

        fs::remove_dir_all(&dir).ok();
    }
}
