use std::path::PathBuf;

use anyhow::{anyhow, Result};
use directories::{BaseDirs, ProjectDirs};

fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("", "", "lore").ok_or_else(|| anyhow!("no home directory for this user"))
}

/// `~/Library/Application Support/lore/config.toml` on macOS.
///
/// Config lives outside the repository on purpose: scan roots are personal
/// paths and must never be committable, even by accident.
pub fn config_file() -> Result<PathBuf> {
    Ok(project_dirs()?.config_dir().join("config.toml"))
}

pub fn data_dir() -> Result<PathBuf> {
    Ok(project_dirs()?.data_dir().to_path_buf())
}

pub fn db_file() -> Result<PathBuf> {
    Ok(data_dir()?.join("lore.db"))
}

pub fn home() -> Result<PathBuf> {
    Ok(BaseDirs::new()
        .ok_or_else(|| anyhow!("no home directory for this user"))?
        .home_dir()
        .to_path_buf())
}

/// Where `launchd` looks for user agents.
pub fn launch_agent_file() -> Result<PathBuf> {
    Ok(home()?
        .join("Library/LaunchAgents")
        .join("dev.lore.collector.plist"))
}

pub fn log_file() -> Result<PathBuf> {
    Ok(home()?.join("Library/Logs/lore/collector.log"))
}

/// The collector's own copy of the binary. `launchd` must not point at a build
/// directory, where a rebuild can replace the binary mid-scan and a clean can
/// remove it outright.
pub fn installed_binary() -> Result<PathBuf> {
    Ok(data_dir()?.join("bin/lore"))
}

/// Claude Code's own directory. A fixed, standard location — never configured.
pub fn claude_dir() -> Result<PathBuf> {
    Ok(home()?.join(".claude"))
}
