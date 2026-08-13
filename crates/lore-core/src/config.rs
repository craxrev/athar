use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
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

    /// Directory names pruned during the walk, in addition to `.gitignore`.
    pub exclude: Vec<String>,

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
}
