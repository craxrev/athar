use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// How often the collector sweeps. Only the file source is sensitive to
    /// this: git and transcripts carry exact timestamps in their own data,
    /// but a file's mtime remembers just the last save, so anything between
    /// two scans of a non-git project is unrecorded.
    pub scan_interval_mins: u64,

    /// Scanned roots, each carrying its category. Categories come from the
    /// filesystem layout so projects never need tagging by hand.
    pub roots: Vec<Root>,

    /// Directory names pruned during the walk.
    pub exclude: Vec<String>,

    /// How long a pause ends an activity block. Work either side of a longer
    /// gap is two stretches, not one.
    pub idle_gap_mins: u64,

    /// How far back the file source looks on a first scan. An mtime records only
    /// the last save, so older ones say little beyond "untouched since"; without
    /// a bound, a first scan would archive every file ever written.
    pub file_lookback_days: u64,

    /// Extra email addresses whose commits count as the user's own. Repository
    /// and global git config are read automatically; this covers addresses used
    /// on another machine.
    pub identities: Vec<String>,

    pub sources: Sources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Root {
    pub path: PathBuf,
    pub category: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Sources {
    pub claude: ClaudeSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ClaudeSource {
    pub enabled: bool,
    /// Defaults to `~/.claude`; present only for tests and unusual installs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

impl Default for ClaudeSource {
    fn default() -> Self {
        Self {
            enabled: true,
            path: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan_interval_mins: 60,
            roots: Vec::new(),
            exclude: [
                "node_modules",
                "target",
                "build",
                "dist",
                ".next",
                ".git",
                "DerivedData",
                ".venv",
                "vendor",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            idle_gap_mins: 20,
            file_lookback_days: 30,
            identities: Vec::new(),
            sources: Sources::default(),
        }
    }
}

impl Config {
    /// Loads the config file, falling back to defaults when it does not exist.
    ///
    /// A missing file is not an error: the Claude source needs no configuration
    /// at all, so `lore scan` works on a fresh machine. Only the git and file
    /// sources need roots, and there is no safe default for those — guessing
    /// where someone keeps their code would be worse than asking.
    pub fn load() -> Result<Self> {
        let path = paths::config_file()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("reading config at {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing config at {}", path.display()))
    }

    pub fn claude_dir(&self) -> Result<PathBuf> {
        match &self.sources.claude.path {
            Some(p) => Ok(p.clone()),
            None => paths::claude_dir(),
        }
    }

    /// Writes the config, creating its directory. Refuses to clobber an existing
    /// file: scan roots are typed by hand and losing them is not recoverable.
    pub fn save_new(&self, path: &Path) -> Result<()> {
        if path.exists() {
            bail!("config already exists at {}", path.display());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let body = toml::to_string_pretty(self).context("serializing config")?;
        fs::write(path, format!("{CONFIG_HEADER}{body}"))
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// The category a path belongs to, taken from the root that contains it.
    /// Categories come from the filesystem layout so projects never need tagging.
    pub fn category_of(&self, path: &Path) -> Option<&str> {
        self.roots
            .iter()
            .filter(|r| path.starts_with(&r.path))
            // Longest match wins, so a nested root beats its parent.
            .max_by_key(|r| r.path.as_os_str().len())
            .map(|r| r.category.as_str())
    }
}

const CONFIG_HEADER: &str = "\
# lore configuration.
#
# This file lives outside any repository on purpose: scan roots are personal
# paths and must never become committable.
#
# Only the git and file sources need roots. Claude Code's directory is a fixed
# standard location and is never configured here.
#
# `scan_interval_mins` matters only to the file source: git and transcripts carry
# exact timestamps in their own data, but a file's mtime remembers just the last
# save, so anything between two scans of a non-git project goes unrecorded.

";
