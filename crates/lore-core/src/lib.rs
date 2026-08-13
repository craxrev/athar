//! lore — a permanent record of developer work, reconstructed from evidence.
//!
//! The sources lore reads all destroy their own history. Claude Code deletes
//! session transcripts after 30 days by default; git garbage-collects
//! unreachable commits from deleted branches and pre-rebase history; a saved
//! file keeps only its last modified time. lore reads what they leave behind, on
//! a schedule, and keeps it after they erase it.
//!
//! The mechanism is archival, not observation: no hooks in the user's harnesses,
//! no live filesystem watcher, no process that must be running at the moment
//! work happens. Work done while lore was off is still captured, because the
//! evidence outlives the moment. The one requirement is running at least once
//! inside each source's retention window.

pub mod agent;
pub mod api;
pub mod collect;
pub mod config;
pub mod db;
pub mod derive;
pub mod paths;
pub mod stats;
pub mod truncate;

pub use config::Config;

/// Bumped whenever the collector's output changes shape.
///
/// The collector records this on every scan; a reader compares it with its own to
/// know whether the archive was written by the build it expects. Comparing
/// binaries cannot work across processes — the window and the collector are
/// different executables by design.
pub const COLLECTOR_REVISION: &str = "2026-08-13.3";
