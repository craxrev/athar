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
    /// What the configuration asks for.
    pub interval_mins: u64,
    /// What `launchd` was actually told, read back from the installed file.
    ///
    /// The two drift apart the moment the interval is edited without reinstalling:
    /// the schedule is baked in at install time and is never re-read, so config
    /// alone cannot say how often scans really happen.
    pub scheduled_interval_mins: Option<u64>,
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
    let binary_stale = match (collector_source(), binary.exists()) {
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
        scheduled_interval_mins: scheduled_interval(&plist_path),
        plist: plist_path,
    })
}

/// The interval the installed schedule really runs, for callers that have no
/// `AgentStatus` to hand. `None` when nothing is installed.
pub fn installed_interval_mins() -> Option<u64> {
    scheduled_interval(&paths::launch_agent_file().ok()?)
}

/// Reads the interval back out of the installed schedule.
///
/// Parsed rather than remembered: a value held anywhere else describes what was
/// asked for, and the question here is what is actually running.
fn scheduled_interval(plist_path: &Path) -> Option<u64> {
    let body = fs::read_to_string(plist_path).ok()?;
    let after = body.split("<key>StartInterval</key>").nth(1)?;
    let open = after.find("<integer>")? + "<integer>".len();
    let close = after[open..].find("</integer>")? + open;
    let seconds: u64 = after[open..close].trim().parse().ok()?;
    Some(seconds / 60)
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

/// The collector binary to schedule.
///
/// Not simply the running process: the desktop window can ask for an install too,
/// and its own executable is the app. Copying that produced a schedule that
/// launched a *window* every interval and archived nothing, so the binary is
/// identified by name rather than assumed.
fn collector_source() -> Result<PathBuf> {
    let current = std::env::current_exe().context("locating the running binary")?;
    if current.file_name().and_then(|n| n.to_str()) == Some(COLLECTOR_BIN) {
        return Ok(current);
    }
    // A sibling of the running binary: how both the dev build tree and a bundle
    // that ships the collector alongside the app are laid out.
    if let Some(sibling) = current.parent().map(|d| d.join(COLLECTOR_BIN)) {
        if sibling.is_file() {
            return Ok(sibling);
        }
    }
    bail!(
        "no `{COLLECTOR_BIN}` binary beside {} — install the agent with `{COLLECTOR_BIN} agent install`",
        current.display()
    )
}

const COLLECTOR_BIN: &str = "lore";

fn install_binary() -> Result<PathBuf> {
    let current = collector_source()?;
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
    fn a_binary_that_is_not_the_collector_is_refused() {
        // The test harness is not named `lore`, and has no `lore` beside it, so
        // this exercises the case that scheduled the window as a collector.
        let refused = collector_source();
        assert!(
            refused.is_err(),
            "a non-collector binary must not be installable as one"
        );
        let message = refused.unwrap_err().to_string();
        assert!(message.contains("agent install"), "got: {message}");
    }

    #[test]
    fn the_installed_interval_can_be_read_back() {
        let dir = std::env::temp_dir().join(format!("lore-agent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("agent.plist");
        std::fs::write(
            &file,
            plist(Path::new("/tmp/lore"), Path::new("/tmp/lore.log"), 900),
        )
        .unwrap();

        assert_eq!(scheduled_interval(&file), Some(15));
        // A schedule that was never installed cannot claim an interval.
        assert_eq!(scheduled_interval(&dir.join("missing.plist")), None);
        std::fs::remove_dir_all(&dir).ok();
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
