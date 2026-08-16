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
