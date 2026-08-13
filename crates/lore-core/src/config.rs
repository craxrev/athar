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

    /// The project a path belongs to.
    ///
    /// A recorded path is often deeper than the project: a session's working
    /// directory can be any subdirectory, and dependency trees contain their own
    /// repositories. Walking down from the configured root, the project is the
    /// **shallowest git repository** in the chain — which folds
    /// `profile-next/node_modules/pdfjs-dist` and `ProFile-iOS/ios` back into the
    /// repository they belong to. When no repository exists in the chain, the
    /// top-level folder under the root is the project, which is the case for
    /// research directories that were never version-controlled.
    ///
    /// Both halves are needed: a blanket top-level rule would merge
    /// `freelance/beecoop/malek` and `freelance/beecoop/colocqui` into `beecoop`,
    /// which is a folder of separate client projects rather than a project.
    ///
    /// Returns `None` for a path under no configured root; that path is its own
    /// project and stays uncategorized, because lore has no basis to fold it.
    pub fn canonical_project(&self, path: &Path) -> Option<PathBuf> {
        let root = self
            .roots
            .iter()
            .filter(|r| path.starts_with(&r.path))
            .max_by_key(|r| r.path.as_os_str().len())?;

        // The root itself may be a repository — a single monorepo pointed at
        // directly — in which case everything under it is that one project.
        if root.path.join(".git").exists() {
            return Some(root.path.clone());
        }

        let relative = path.strip_prefix(&root.path).ok()?;
        let mut cursor = root.path.clone();
        let mut top_level: Option<PathBuf> = None;

        for component in relative.components() {
            cursor.push(component);
            if top_level.is_none() {
                top_level = Some(cursor.clone());
            }
            if cursor.join(".git").exists() {
                return Some(cursor);
            }
        }

        Some(top_level.unwrap_or_else(|| root.path.clone()))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "lore-config-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }

    fn repo(path: &Path) {
        fs::create_dir_all(path.join(".git")).unwrap();
    }

    fn config_for(root: &Path) -> Config {
        Config {
            roots: vec![Root {
                path: root.to_path_buf(),
                category: "research".into(),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn folds_a_subdirectory_into_its_repository() {
        let root = temp_root();
        repo(&root.join("profile-next"));
        fs::create_dir_all(root.join("profile-next/node_modules/pdfjs-dist")).unwrap();
        let config = config_for(&root);

        // A dependency's own repository is not a project.
        assert_eq!(
            config.canonical_project(&root.join("profile-next/node_modules/pdfjs-dist")),
            Some(root.join("profile-next"))
        );
        // Neither is a working directory inside the repository.
        assert_eq!(
            config.canonical_project(&root.join("profile-next/app/src")),
            Some(root.join("profile-next"))
        );
    }

    #[test]
    fn keeps_sibling_repositories_under_a_grouping_folder_apart() {
        let root = temp_root();
        fs::create_dir_all(root.join("beecoop")).unwrap();
        repo(&root.join("beecoop/malek"));
        repo(&root.join("beecoop/colocqui"));
        repo(&root.join("beecoop/coinsence/html_i4c"));
        let config = config_for(&root);

        // `beecoop` is a folder of client projects, not a project.
        assert_eq!(
            config.canonical_project(&root.join("beecoop/malek/src")),
            Some(root.join("beecoop/malek"))
        );
        assert_eq!(
            config.canonical_project(&root.join("beecoop/colocqui")),
            Some(root.join("beecoop/colocqui"))
        );
        assert_eq!(
            config.canonical_project(&root.join("beecoop/coinsence/html_i4c/x")),
            Some(root.join("beecoop/coinsence/html_i4c"))
        );
    }

    #[test]
    fn uses_the_top_level_folder_when_nothing_is_version_controlled() {
        let root = temp_root();
        fs::create_dir_all(root.join("ooredoo-tz/ghidra_out/deep")).unwrap();
        let config = config_for(&root);
        assert_eq!(
            config.canonical_project(&root.join("ooredoo-tz/ghidra_out/deep")),
            Some(root.join("ooredoo-tz"))
        );
    }

    #[test]
    fn leaves_a_path_outside_every_root_alone() {
        let root = temp_root();
        let config = config_for(&root);
        assert_eq!(config.canonical_project(Path::new("/Users/someone")), None);
    }

    #[test]
    fn a_root_that_is_itself_a_repository_absorbs_its_subdirectories() {
        let root = temp_root();
        repo(&root);
        fs::create_dir_all(root.join("packages/web")).unwrap();
        let config = config_for(&root);
        assert_eq!(
            config.canonical_project(&root.join("packages/web")),
            Some(root.clone())
        );
    }
}
