//! Collectors read artifacts other systems have already written, on a schedule.
//!
//! Claude Code is the only implementation. The shape below exists because the
//! sources differ in what they know: transcripts and git carry exact timestamps
//! in their own data, while a file's mtime is exact but incomplete between
//! scans. A collector says which it is so the interface can be honest about it.

pub mod claude;
