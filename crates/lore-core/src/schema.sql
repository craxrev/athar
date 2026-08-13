-- lore schema.
--
-- Two tiers, one direction:
--
--   source -> adapter -> raw_records -> projector -> derived tables
--
-- `raw_records` is the archive: append-only, immutable, and the reason lore
-- survives Claude Code's 30-day cleanup and git's garbage collection. Every
-- derived table is rebuildable from it, so adapters can improve without needing
-- source material that no longer exists.

CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Everything lore reads from: a transcript file, or a git repository.
--
-- For line-based origins the cursor is a byte offset, so a rescan never
-- re-parses what it already archived; `inode` and `size` detect rotation or
-- truncation, which forces a full re-read of that one origin. Repositories
-- ignore those columns and are made incremental by record identity instead.
--
-- Paths live here rather than on every record: absolute paths repeated 71k times
-- cost more than the records themselves.
CREATE TABLE IF NOT EXISTS origins (
    id            INTEGER PRIMARY KEY,
    source        TEXT    NOT NULL,
    path          TEXT    NOT NULL,
    inode         INTEGER,
    size          INTEGER NOT NULL DEFAULT 0,
    mtime_ms      INTEGER NOT NULL DEFAULT 0,
    byte_offset   INTEGER NOT NULL DEFAULT 0,
    line_no       INTEGER NOT NULL DEFAULT 0,
    updated_at_ms INTEGER NOT NULL DEFAULT 0,
    UNIQUE (source, path)
);

CREATE TABLE IF NOT EXISTS projects (
    id   INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE
);

-- One archived record. `json` keeps the original shape so unknown kinds are
-- never dropped; oversized blobs are shortened in place.
--
-- Identity depends on the origin's shape, and both forms below make rescanning
-- idempotent without a per-row hash:
--
--   line-based (transcripts) — (origin_id, line_no). Line numbering is per file
--       and a rotated file is forgotten wholesale.
--   addressed (git)          — (origin_id, ext_id), where ext_id is the commit
--       sha. Re-reading a repository re-offers the same shas.
CREATE TABLE IF NOT EXISTS raw_records (
    id             INTEGER PRIMARY KEY,
    origin_id      INTEGER NOT NULL REFERENCES origins (id) ON DELETE CASCADE,
    line_no        INTEGER NOT NULL DEFAULT 0,
    ext_id         TEXT,
    ts_ms          INTEGER,
    kind           TEXT    NOT NULL,
    session_id     TEXT,
    project_id     INTEGER REFERENCES projects (id),
    json           TEXT    NOT NULL,
    bytes_original INTEGER NOT NULL,
    truncated      INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS raw_records_line
    ON raw_records (origin_id, line_no) WHERE line_no > 0;
CREATE UNIQUE INDEX IF NOT EXISTS raw_records_ext
    ON raw_records (origin_id, ext_id) WHERE ext_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS raw_records_ts      ON raw_records (ts_ms);
CREATE INDEX IF NOT EXISTS raw_records_session ON raw_records (session_id, ts_ms);
CREATE INDEX IF NOT EXISTS raw_records_kind    ON raw_records (kind, ts_ms);
CREATE INDEX IF NOT EXISTS raw_records_project ON raw_records (project_id, ts_ms);

-- ── Derived tables ────────────────────────────────────────────────────────────
--
-- Everything below is a projection of `raw_records` and is rebuilt wholesale by
-- `lore rebuild`. Nothing here is a source of truth: as adapters improve, these
-- are recomputed rather than migrated. That is the whole point of archiving the
-- raw record first.

-- A contiguous stretch of work on one project, ended by an idle gap.
-- This is how time is accounted for without a timer running.
CREATE TABLE IF NOT EXISTS blocks (
    id           INTEGER PRIMARY KEY,
    project_id   INTEGER NOT NULL REFERENCES projects (id),
    started_ms   INTEGER NOT NULL,
    ended_ms     INTEGER NOT NULL,
    records      INTEGER NOT NULL DEFAULT 0,
    sessions     INTEGER NOT NULL DEFAULT 0,
    commits      INTEGER NOT NULL DEFAULT 0,
    file_changes INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS blocks_time    ON blocks (started_ms);
CREATE INDEX IF NOT EXISTS blocks_project ON blocks (project_id, started_ms);

CREATE TABLE IF NOT EXISTS sessions (
    session_id         TEXT PRIMARY KEY,
    project_id         INTEGER REFERENCES projects (id),
    block_id           INTEGER REFERENCES blocks (id),
    started_ms         INTEGER,
    ended_ms           INTEGER,
    title              TEXT,
    prompts            INTEGER NOT NULL DEFAULT 0,
    replies            INTEGER NOT NULL DEFAULT 0,
    tool_calls         INTEGER NOT NULL DEFAULT 0,
    input_tokens       INTEGER NOT NULL DEFAULT 0,
    output_tokens      INTEGER NOT NULL DEFAULT 0,
    cache_read_tokens  INTEGER NOT NULL DEFAULT 0,
    models             TEXT,
    -- False when the session survives only as prompt history, its transcript
    -- already deleted by Claude Code. 936 of 1,032 sessions are in this state.
    has_transcript     INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS sessions_time ON sessions (started_ms);

-- Files a session touched, by absolute path so commits can be matched against it.
CREATE TABLE IF NOT EXISTS session_files (
    session_id TEXT NOT NULL,
    path       TEXT NOT NULL,
    writes     INTEGER NOT NULL DEFAULT 0,
    reads      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (session_id, path)
);

CREATE TABLE IF NOT EXISTS commits (
    sha         TEXT PRIMARY KEY,
    project_id  INTEGER NOT NULL REFERENCES projects (id),
    block_id    INTEGER REFERENCES blocks (id),
    ts_ms       INTEGER NOT NULL,
    message     TEXT,
    unreachable INTEGER NOT NULL DEFAULT 0,
    file_count  INTEGER NOT NULL DEFAULT 0,
    insertions  INTEGER NOT NULL DEFAULT 0,
    deletions   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS commits_time ON commits (ts_ms);

CREATE TABLE IF NOT EXISTS commit_files (
    sha  TEXT NOT NULL,
    path TEXT NOT NULL,
    PRIMARY KEY (sha, path)
);

-- Which session a commit came out of, and on what evidence.
--
--   certain — the transcript records the AI running `git commit`
--   strong  — the commit's files are files this session wrote
--   weak    — only time and project coincide; the user likely committed by hand
--
-- The tier is stored rather than hidden because a link lore inferred and a link
-- lore witnessed are not the same claim.
CREATE TABLE IF NOT EXISTS commit_links (
    sha          TEXT PRIMARY KEY,
    session_id   TEXT NOT NULL,
    tier         TEXT NOT NULL,
    shared_files INTEGER NOT NULL DEFAULT 0
);

-- The project each recorded path was folded into, and when that was decided.
--
-- Folding reads the filesystem to find the shallowest repository in a path's
-- chain. That evidence disappears when a project is deleted, so the decision is
-- remembered: a path that still exists is re-folded on every rebuild and follows
-- changes to the configured roots, while a path whose folder is gone keeps the
-- grouping decided when lore could still see it. Without this, deleting a
-- project silently re-folded its history into the parent folder and merged it
-- with unrelated siblings.
CREATE TABLE IF NOT EXISTS project_map (
    raw_path       TEXT PRIMARY KEY,
    canonical_path TEXT    NOT NULL,
    decided_at_ms  INTEGER NOT NULL,
    from_disk      INTEGER NOT NULL DEFAULT 1
);
