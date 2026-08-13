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

/// Claude Code's own directory. A fixed, standard location — never configured.
pub fn claude_dir() -> Result<PathBuf> {
    Ok(home()?.join(".claude"))
}
