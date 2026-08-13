//! The background collector, as a macOS user agent.
//!
//! lore's whole premise depends on running regularly: Claude Code deletes session
//! transcripts after 30 days, git collects unreachable commits after roughly the
//! same, and a file's mtime only ever remembers its last save. An archive that is
//! scanned by hand is an archive with holes, so scanning belongs to `launchd`
//! rather than to remembering.
//!
//! The agent runs `lore scan`, which reads every source and rebuilds the derived
//! tables. It is the sole writer; the desktop app opens the same database
//! read-only, so a scan and an open window never contend.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::config::Config;
use crate::paths;

pub const LABEL: &str = "dev.lore.collector";

#[derive(Debug)]
pub struct AgentStatus {
    pub plist: PathBuf,
    pub installed: bool,
    pub loaded: bool,
    pub binary: PathBuf,
    pub binary_present: bool,
    pub log: PathBuf,
    pub interval_mins: u64,
    /// True when the installed copy differs from the binary now running. The
    /// agent keeps using its own copy, so a collector change is not scheduled
    /// until `lore agent install` runs again.
    pub binary_stale: bool,
}

/// Installs a copy of the running binary and registers the agent against it.
///
/// The copy matters: pointing `launchd` at a build directory means a rebuild can
/// replace the binary mid-scan, and a `cargo clean` silently stops the collector.
pub fn install(config: &Config) -> Result<AgentStatus> {
    let binary = install_binary()?;
    let plist_path = paths::launch_agent_file()?;
    let log = paths::log_file()?;

    if let Some(parent) = log.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    if let Some(parent) = plist_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }

    let interval = config.scan_interval_mins.max(1) * 60;
    fs::write(&plist_path, plist(&binary, &log, interval))
        .with_context(|| format!("writing {}", plist_path.display()))?;

    // Reloading is two steps because `launchd` will not re-read a plist in place.
    let _ = bootout();
    bootstrap(&plist_path)?;

    status(config)
}

pub fn uninstall(config: &Config) -> Result<AgentStatus> {
    let plist_path = paths::launch_agent_file()?;
    let _ = bootout();
    if plist_path.exists() {
        fs::remove_file(&plist_path)
            .with_context(|| format!("removing {}", plist_path.display()))?;
    }
    // The installed binary is left in place: removing the schedule should not
    // remove the tool that can still be run by hand.
    status(config)
}

pub fn status(config: &Config) -> Result<AgentStatus> {
    let plist_path = paths::launch_agent_file()?;
    let binary = paths::installed_binary()?;
    let binary_stale = match (std::env::current_exe(), binary.exists()) {
        (Ok(current), true) if current != binary => {
            let same = std::fs::read(&current)
                .ok()
                .zip(std::fs::read(&binary).ok())
                .map(|(a, b)| a == b)
                .unwrap_or(false);
            !same
        }
        _ => false,
    };

    Ok(AgentStatus {
        installed: plist_path.exists(),
        loaded: is_loaded(),
        binary_present: binary.exists(),
        binary_stale,
        binary,
        log: paths::log_file()?,
        interval_mins: config.scan_interval_mins,
        plist: plist_path,
    })
}

/// Runs the collector once, right now, through `launchd` rather than in this
/// process — so the run happens in exactly the environment the schedule uses.
pub fn run_now() -> Result<()> {
    let out = Command::new("launchctl")
        .args(["kickstart", &format!("gui/{}/{LABEL}", uid()?)])
        .output()
        .context("running launchctl kickstart")?;
    if !out.status.success() {
        bail!(
            "launchctl kickstart failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

fn install_binary() -> Result<PathBuf> {
    let current = std::env::current_exe().context("locating the running binary")?;
    let target = paths::installed_binary()?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    if current != target {
        fs::copy(&current, &target).with_context(|| {
            format!("copying {} to {}", current.display(), target.display())
        })?;
    }
    Ok(target)
}

fn uid() -> Result<String> {
    let out = Command::new("id").arg("-u").output().context("reading uid")?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn bootstrap(plist_path: &Path) -> Result<()> {
    let out = Command::new("launchctl")
        .args(["bootstrap", &format!("gui/{}", uid()?)])
        .arg(plist_path)
        .output()
        .context("running launchctl bootstrap")?;
    if out.status.success() {
        return Ok(());
    }

    // `bootstrap` is unavailable on older systems; `load -w` is the equivalent.
    let fallback = Command::new("launchctl")
        .args(["load", "-w"])
        .arg(plist_path)
        .output()
        .context("running launchctl load")?;
    if !fallback.status.success() {
        bail!(
            "could not register the agent: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

fn bootout() -> Result<()> {
    let out = Command::new("launchctl")
        .args(["bootout", &format!("gui/{}/{LABEL}", uid()?)])
        .output()?;
    if !out.status.success() {
        bail!("not loaded");
    }
    Ok(())
}

fn is_loaded() -> bool {
    Command::new("launchctl")
        .args(["list", LABEL])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn plist(binary: &Path, log: &Path, interval_secs: u64) -> String {
    let binary = escape(&binary.to_string_lossy());
    let log = escape(&log.to_string_lossy());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>{LABEL}</string>
	<key>ProgramArguments</key>
	<array>
		<string>{binary}</string>
		<string>scan</string>
	</array>
	<key>StartInterval</key>
	<integer>{interval_secs}</integer>
	<!-- Scan on login too: the interval alone would miss a machine that is
	     asleep more often than it is awake. -->
	<key>RunAtLoad</key>
	<true/>
	<key>StandardOutPath</key>
	<string>{log}</string>
	<key>StandardErrorPath</key>
	<string>{log}</string>
	<!-- Reading a few hundred megabytes of transcripts must never be felt. -->
	<key>ProcessType</key>
	<string>Background</string>
	<key>Nice</key>
	<integer>5</integer>
	<key>LowPriorityIO</key>
	<true/>
</dict>
</plist>
"#
    )
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plist_carries_the_schedule_and_the_binary() {
        let body = plist(Path::new("/tmp/lore"), Path::new("/tmp/lore.log"), 3600);
        assert!(body.contains("<string>/tmp/lore</string>"));
        assert!(body.contains("<string>scan</string>"));
        assert!(body.contains("<integer>3600</integer>"));
        assert!(body.contains("<key>RunAtLoad</key>"));
        assert!(body.contains("<string>Background</string>"));
    }

    #[test]
    fn paths_with_xml_significant_characters_stay_valid() {
        let body = plist(
            Path::new("/tmp/a&b/lore"),
            Path::new("/tmp/<log>/lore.log"),
            60,
        );
        assert!(body.contains("/tmp/a&amp;b/lore"));
        assert!(body.contains("/tmp/&lt;log&gt;/lore.log"));
        assert!(!body.contains("/tmp/a&b/lore"));
    }
}
