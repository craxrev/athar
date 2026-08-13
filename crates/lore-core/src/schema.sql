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
