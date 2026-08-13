-- lore schema.
--
-- Two tiers, one direction:
--
--   source file -> adapter -> raw_records -> projector -> derived tables
--
-- `raw_records` is the archive: append-only, immutable, and the reason lore
-- survives Claude Code's 30-day cleanup and git's garbage collection. Every
-- derived table is rebuildable from it, so adapters can improve without needing
-- source files that no longer exist.

CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Every file lore has read, with its read cursor. Transcripts are append-only,
-- so a byte offset means a rescan never re-parses what it already archived;
-- `inode` and `size` detect rotation or truncation, which forces a full re-read
-- of that one file.
--
-- Paths live here rather than on every record: absolute paths repeated 71k times
-- cost more than the records themselves.
CREATE TABLE IF NOT EXISTS files (
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

-- One row per source line, with oversized blobs shortened in place. `json` keeps
-- the original record shape so unknown kinds are never dropped.
--
-- Identity is (file_id, line_no): line numbering is per file and a rotated file
-- is forgotten wholesale, so rescanning is idempotent without a separate hash.
CREATE TABLE IF NOT EXISTS raw_records (
    id             INTEGER PRIMARY KEY,
    file_id        INTEGER NOT NULL REFERENCES files (id) ON DELETE CASCADE,
    line_no        INTEGER NOT NULL,
    ts_ms          INTEGER,
    kind           TEXT    NOT NULL,
    session_id     TEXT,
    project_id     INTEGER REFERENCES projects (id),
    json           TEXT    NOT NULL,
    bytes_original INTEGER NOT NULL,
    truncated      INTEGER NOT NULL DEFAULT 0,
    UNIQUE (file_id, line_no)
);

CREATE INDEX IF NOT EXISTS raw_records_ts      ON raw_records (ts_ms);
CREATE INDEX IF NOT EXISTS raw_records_session ON raw_records (session_id, ts_ms);
CREATE INDEX IF NOT EXISTS raw_records_kind    ON raw_records (kind, ts_ms);
CREATE INDEX IF NOT EXISTS raw_records_project ON raw_records (project_id, ts_ms);
