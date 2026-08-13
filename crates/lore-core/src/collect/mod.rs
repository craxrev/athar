//! Collectors read artifacts other systems have already written, on a schedule.
//!
//! The sources differ in what they know, and lore must not smooth that over:
//! transcripts and git carry exact timestamps in their own data, while a file's
//! mtime is exact but incomplete between scans.

pub mod claude;
pub mod file;
pub mod git;
