use std::path::PathBuf;

use anyhow::{anyhow, Result};
use directories::{BaseDirs, ProjectDirs};

/// Overrides every path below, so a whole profile — config, archive and logs —
/// can live somewhere other than the real one.
///
/// This exists for a problem with no other answer: there is no way to see what
/// a first run looks like without destroying the archive, and the archive is a
/// permanent copy of history the sources have already deleted. Pointing this at
/// an empty directory gives a pristine install and leaves the real one alone.
///
/// ```text
/// ATHAR_HOME=/tmp/athar-firstrun npm run tauri dev
/// ATHAR_HOME=/tmp/athar-firstrun cargo run -p athar-cli -- stats
/// ```
///
/// The window spawns the collector as a child process, so it inherits this and
/// both halves of the app agree on which profile they are looking at.
pub const HOME_ENV: &str = "ATHAR_HOME";

fn override_root() -> Option<PathBuf> {
    match std::env::var_os(HOME_ENV) {
        Some(v) if !v.is_empty() => Some(PathBuf::from(v)),
        _ => None,
    }
}

fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("", "", "athar").ok_or_else(|| anyhow!("no home directory for this user"))
}

/// `~/Library/Application Support/athar/config.toml` on macOS.
///
/// Config lives outside the repository on purpose: scan roots are personal
/// paths and must never be committable, even by accident.
pub fn config_file() -> Result<PathBuf> {
    if let Some(root) = override_root() {
        return Ok(root.join("config.toml"));
    }
    Ok(project_dirs()?.config_dir().join("config.toml"))
}

pub fn data_dir() -> Result<PathBuf> {
    if let Some(root) = override_root() {
        return Ok(root);
    }
    Ok(project_dirs()?.data_dir().to_path_buf())
}

pub fn db_file() -> Result<PathBuf> {
    Ok(data_dir()?.join("athar.db"))
}

pub fn home() -> Result<PathBuf> {
    Ok(BaseDirs::new()
        .ok_or_else(|| anyhow!("no home directory for this user"))?
        .home_dir()
        .to_path_buf())
}

pub fn log_file() -> Result<PathBuf> {
    if let Some(root) = override_root() {
        return Ok(root.join("collector.log"));
    }
    Ok(home()?.join("Library/Logs/athar/collector.log"))
}

/// Claude Code's own directory. A fixed, standard location — never configured.
pub fn claude_dir() -> Result<PathBuf> {
    Ok(home()?.join(".claude"))
}

/* ---- the rename ---------------------------------------------------------- */

/// Where this app kept its profile when it was called `lore`.
fn legacy_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "lore")
}

/// Move a `lore` profile to its `athar` location, once.
///
/// The archive is the point of the product: the sources it was read from have
/// deleted their own copies, so a rename that abandoned it would destroy history
/// that cannot be rebuilt from anywhere. It is a directory rename on the same
/// volume, so it is atomic and costs nothing regardless of how large the archive
/// has grown.
///
/// Returns the path it moved from, so a caller can say so. Silent and idempotent
/// otherwise: nothing to move, or a destination that already exists, is the
/// normal case on every run after the first.
pub fn migrate_legacy_profile() -> Result<Option<PathBuf>> {
    // An overridden profile is a throwaway or a test one; it has no history to
    // inherit and must never adopt the real archive.
    if override_root().is_some() {
        return Ok(None);
    }

    let Some(old) = legacy_dirs() else {
        return Ok(None);
    };
    let new = project_dirs()?;

    // On macOS these are the same directory and the second move is a no-op; on
    // Linux config and data live apart and both have to travel.
    let mut moved = None;
    let pairs = [
        (old.data_dir().to_path_buf(), new.data_dir().to_path_buf()),
        (old.config_dir().to_path_buf(), new.config_dir().to_path_buf()),
    ];
    for (from, to) in pairs {
        if from == to || !from.exists() || to.exists() {
            continue;
        }
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&from, &to)?;
        moved.get_or_insert(from);
    }

    // The database carried the old name inside whichever directory it landed in.
    // Its write-ahead log and shared-memory files travel with it, or SQLite will
    // not find the tail of the last transaction.
    let dir = data_dir()?;
    for suffix in ["", "-wal", "-shm"] {
        let from = dir.join(format!("lore.db{suffix}"));
        let to = dir.join(format!("athar.db{suffix}"));
        if from.exists() && !to.exists() {
            std::fs::rename(&from, &to)?;
        }
    }

    Ok(moved)
}
