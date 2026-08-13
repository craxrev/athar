//! `lore` — the collector, driven from the command line.
//!
//! The CLI exists so ingestion can be iterated on and verified without the UI
//! in the way. The desktop app reads the same database, read-only.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use lore_core::{
    agent,
    collect::{claude, file, git},
    derive,
    config::Config,
    db, paths, stats,
};

#[derive(Parser)]
#[command(name = "lore", about = "A permanent record of developer work", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read every source once and archive what is new.
    Scan,
    /// What the archive currently holds.
    Stats,
    /// Where config and data live, and whether the sources are readable.
    Doctor,
    /// Recompute every derived table from the archive.
    Rebuild,
    /// The timeline for one day, as the app's Stream view will show it.
    Day {
        /// A local date, `YYYY-MM-DD`. Defaults to today.
        date: Option<String>,
    },
    /// Inspect or create the configuration file.
    #[command(subcommand)]
    Config(ConfigCommand),
    /// Manage the background collector that keeps the archive current.
    #[command(subcommand)]
    Agent(AgentCommand),
    /// Exercise every query the desktop app makes, against the real archive.
    /// A failure here is a failure the app can only show as a stuck window.
    Check,
}

#[derive(Subcommand)]
enum AgentCommand {
    /// Install the agent and start scanning on the configured interval.
    Install,
    /// Stop and remove the agent. The archive and the binary are left alone.
    Uninstall,
    /// Whether the agent is installed and loaded.
    Status,
    /// Trigger one scan now, through the agent.
    Run,
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Print the config file's location.
    Path,
    /// Print the effective configuration, defaults included.
    Show,
    /// Write a starter config. Never overwrites an existing one.
    Init {
        /// A scanned root as `path=category`, repeatable.
        /// Example: --root ~/Developer/research=research
        #[arg(long = "root", value_name = "PATH=CATEGORY")]
        roots: Vec<String>,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Scan => scan(),
        Command::Stats => show_stats(),
        Command::Doctor => doctor(),
        Command::Rebuild => rebuild(),
        Command::Day { date } => day_view(date.as_deref()),
        Command::Config(cmd) => config(cmd),
        Command::Agent(cmd) => agent_command(cmd),
        Command::Check => check(),
    }
}

fn config(cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Path => {
            println!("{}", paths::config_file()?.display());
            Ok(())
        }
        ConfigCommand::Show => {
            print!("{}", toml::to_string_pretty(&Config::load()?)?);
            Ok(())
        }
        ConfigCommand::Init { roots } => {
            let mut cfg = Config::default();
            for spec in &roots {
                let (path, category) = spec
                    .split_once('=')
                    .with_context(|| format!("expected PATH=CATEGORY, got `{spec}`"))?;
                let path = expand_home(path)?;
                if !path.is_dir() {
                    eprintln!("warning: {} is not a directory", path.display());
                }
                cfg.roots.push(lore_core::config::Root {
                    path,
                    category: category.to_string(),
                });
            }

            let path = paths::config_file()?;
            cfg.save_new(&path)?;
            println!("wrote {}", path.display());
            if cfg.roots.is_empty() {
                println!("no roots set — the git and file sources need at least one");
            }
            for root in &cfg.roots {
                println!("  root {} [{}]", root.path.display(), root.category);
            }
            Ok(())
        }
    }
}

/// A leading `~` typed on the command line is not expanded by the shell when
/// quoted, and a config full of unexpanded tildes silently scans nothing.
fn expand_home(raw: &str) -> Result<std::path::PathBuf> {
    match raw.strip_prefix("~/") {
        Some(rest) => Ok(paths::home()?.join(rest)),
        None => Ok(std::path::PathBuf::from(raw)),
    }
}

fn scan() -> Result<()> {
    let config = Config::load()?;
    let mut conn = db::open_default()?;
    let started = std::time::Instant::now();

    if config.sources.claude.enabled {
        let claude_dir = config.claude_dir()?;
        if claude_dir.is_dir() {
            let s = claude::scan(&mut conn, &claude_dir)
                .with_context(|| format!("scanning {}", claude_dir.display()))?;
            println!("claude");
            println!("  files       {} seen, {} read", s.files_seen, s.files_read);
            println!("  archived    {} records ({} lines)", s.inserted, s.lines_read);
            if s.duplicates > 0 {
                println!("  already had {} records", s.duplicates);
            }
            println!("  dropped     {} transient records", s.dropped);
            println!("  truncated   {} oversized blobs", s.truncated);
            if s.unparsed > 0 {
                println!("  unparsed    {} lines archived verbatim", s.unparsed);
            }
            if s.files_reset > 0 {
                println!("  reset       {} rotated files re-read", s.files_reset);
            }
            println!("  read        {}", human_bytes(s.bytes_read as i64));
        } else {
            println!("claude      no directory at {}", claude_dir.display());
        }
    }

    if config.roots.is_empty() {
        println!("git         no roots configured — run `lore config init --root ...`");
    } else {
        let g = git::scan(&mut conn, &config)?;
        println!("git");
        println!(
            "  repos       {} found, {} read, {} unchanged",
            g.repos_seen, g.repos_read, g.repos_unchanged
        );
        println!("  archived    {} commits", g.commits_inserted);
        if g.commits_known > 0 {
            println!("  already had {} commits", g.commits_known);
        }
        if g.commits_refreshed > 0 {
            println!("  refreshed   {} commits re-read by a newer adapter", g.commits_refreshed);
        }
        println!(
            "  unreachable {} commits git will collect (deleted branches, rewrites)",
            g.commits_unreachable
        );
        if g.commits_foreign > 0 {
            println!("  skipped     {} commits by other authors", g.commits_foreign);
        }
        if g.repos_without_identity > 0 {
            println!(
                "  no identity {} repos skipped — set user.email or config identities",
                g.repos_without_identity
            );
        }
        if g.errors > 0 {
            println!("  errors      {} repos could not be read", g.errors);
        }
    }

    if !config.roots.is_empty() {
        let f = file::scan(&mut conn, &config)?;
        println!("files");
        println!(
            "  projects    {} ({} git, {} without git)",
            f.projects_seen, f.repos, f.non_git
        );
        println!("  examined    {} files", f.files_examined);
        println!("  archived    {} changes", f.changes_recorded);
        if f.changes_known > 0 {
            println!("  already had {} changes", f.changes_known);
        }
        println!(
            "  skipped     {} older than the {}-day lookback",
            f.skipped_old, config.file_lookback_days
        );
        if f.errors > 0 {
            println!("  errors      {} paths unreadable", f.errors);
        }
    }

    let d = derive::rebuild(&mut conn, &config)?;
    println!("derived");
    println!(
        "  blocks      {} activity blocks ({} min idle gap)",
        d.blocks, config.idle_gap_mins
    );
    println!("  sessions    {}", d.sessions);
    println!(
        "  projects    {} ({} recorded paths folded in)",
        d.projects, d.folded_paths
    );
    println!(
        "  links       {} certain, {} strong, {} weak, {} unlinked commits",
        d.links_certain, d.links_strong, d.links_weak, d.commits_unlinked
    );

    println!("\nfinished in {:.1}s", started.elapsed().as_secs_f64());
    Ok(())
}

fn rebuild() -> Result<()> {
    let config = Config::load()?;
    let mut conn = db::open_default()?;
    let started = std::time::Instant::now();
    let d = derive::rebuild(&mut conn, &config)?;
    println!(
        "{} projects ({} recorded paths folded into them)",
        d.projects, d.folded_paths
    );
    println!(
        "{} blocks, {} sessions, {} commits\n{} certain, {} strong, {} weak, {} unlinked",
        d.blocks, d.sessions, d.commits, d.links_certain, d.links_strong, d.links_weak,
        d.commits_unlinked
    );
    println!("rebuilt in {:.1}s", started.elapsed().as_secs_f64());
    Ok(())
}

fn day_view(date: Option<&str>) -> Result<()> {
    use chrono::{Local, NaiveDate, TimeZone};

    let day = match date {
        Some(d) => NaiveDate::parse_from_str(d, "%Y-%m-%d")
            .with_context(|| format!("expected YYYY-MM-DD, got `{d}`"))?,
        None => Local::now().date_naive(),
    };
    let from = Local
        .from_local_datetime(&day.and_hms_opt(0, 0, 0).unwrap())
        .earliest()
        .map(|d| d.timestamp_millis())
        .unwrap_or_default();
    let to = from + 86_400_000;

    let conn = db::open_default()?;
    let config = Config::load()?;
    let sum = stats::range_summary(&conn, from, to)?;

    println!(
        "{}   {} elapsed · {} across projects · {} blocks · {} projects",
        day.format("%a %d %b %Y"),
        duration(sum.elapsed_ms),
        duration(sum.project_ms),
        sum.blocks,
        sum.projects
    );

    let rows = stats::blocks_between(&conn, from, to, None)?;
    if rows.is_empty() {
        println!("\nnothing recorded. lore may not have been running, or nothing happened.");
        return Ok(());
    }

    for block in rows {
        let name = std::path::Path::new(&block.project)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| block.project.clone());
        let category = config
            .category_of(std::path::Path::new(&block.project))
            .unwrap_or("uncategorized");

        println!(
            "\n{}–{}  {}  [{}]  {}",
            clock(block.started_ms),
            clock(block.ended_ms),
            name,
            category,
            duration(block.ended_ms - block.started_ms)
        );

        for s in stats::sessions_in_block(&conn, block.id)? {
            let title = s.title.unwrap_or_else(|| "untitled session".into());
            let note = if s.has_transcript { "" } else { "  (transcript deleted)" };
            println!(
                "  ▸ {title}{note}\n      {} prompts · {} tools · {}k tokens · {}",
                s.prompts,
                s.tool_calls,
                (s.input_tokens + s.output_tokens) / 1000,
                if s.models.is_empty() { "—".into() } else { s.models.clone() }
            );
        }

        for c in stats::commits_in_block(&conn, block.id)? {
            let tier = match c.tier.as_deref() {
                Some("certain") => "· ai committed",
                Some("strong") => "· from session (files match)",
                Some("weak") => "· same session window",
                _ => "· unattributed",
            };
            let ghost = if c.unreachable { " ⚠ unreachable" } else { "" };
            println!(
                "  ● {}  {}  +{}/-{} {tier}{ghost}",
                &c.sha[..7],
                first_line(&c.message),
                c.insertions,
                c.deletions
            );
        }

        let changes = stats::file_changes_in_block(&conn, block.id)?;
        if !changes.is_empty() {
            let shown: Vec<String> = changes.iter().take(3).map(|(p, _, st)| format!("{p} ({st})")).collect();
            let more = changes.len().saturating_sub(shown.len());
            println!(
                "  ▨ {} file changes: {}{}",
                changes.len(),
                shown.join(", "),
                if more > 0 { format!(", +{more} more") } else { String::new() }
            );
        }
    }

    Ok(())
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or("")
}

fn clock(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_else(|| "--:--".into())
}

fn duration(ms: i64) -> String {
    let mins = ms / 60_000;
    if mins == 0 {
        // A block can hold a single record, which has no span. Printing `0m`
        // reads as nothing happening, when something did.
        "<1m".to_string()
    } else if mins < 60 {
        format!("{mins}m")
    } else {
        format!("{}h {:02}m", mins / 60, mins % 60)
    }
}

fn check() -> Result<()> {
    use lore_core::api;
    let config = Config::load()?;
    let conn = api::open_readonly(&paths::db_file()?)?;

    let now = chrono::Utc::now().timestamp_millis();
    let week = now - 7 * 86_400_000;

    macro_rules! step {
        ($label:expr, $body:expr) => {{
            let started = std::time::Instant::now();
            let value = $body;
            match value {
                Ok(v) => {
                    println!("  ok    {:<16} {:>6}ms", $label, started.elapsed().as_millis());
                    v
                }
                Err(e) => {
                    println!("  FAIL  {:<16} {e:#}", $label);
                    return Err(e);
                }
            }
        }};
    }

    println!("app read path:");
    step!("status", api::status(&conn, &config));
    step!("projects", api::projects(&conn, &config));
    step!("summary", api::summary(&conn, week, now));
    let lanes = step!("lanes", api::lanes(&conn, &config, week, now, None));
    let blocks = step!("timeline", api::timeline(&conn, &config, week, now, None, None, None));
    println!("        {} lanes, {} blocks", lanes.len(), blocks.len());

    // The range the app offers as "All time", which is the widest query it can be
    // asked for and therefore the one that has to be measured.
    let earliest = api::status(&conn, &config)?.earliest_ms.unwrap_or(week);
    let all = step!("timeline all", api::timeline(&conn, &config, earliest, now, None, None, None));
    println!("        {} blocks over all time (uncapped)", all.len());
    let capped = step!(
        "timeline capped",
        api::timeline(&conn, &config, earliest, now, None, None, Some(300))
    );
    println!("        {} blocks with the app's cap", capped.len());

    if let Some(block) = blocks.iter().find(|b| !b.commits.is_empty()) {
        let sha = block.commits[0].sha.clone();
        let files = step!("commit_files", api::commit_files(&conn, &sha));
        println!("        {} files for {}", files.len(), &sha[..7]);
    }
    if let Some(block) = blocks.iter().find(|b| !b.sessions.is_empty()) {
        let id = block.sessions[0].id.clone();
        let detail = step!("session", api::session(&conn, &config, &id));
        println!("        {} turns", detail.map(|d| d.turns.len()).unwrap_or(0));
    }

    println!("\nall queries answered");
    Ok(())
}

fn agent_command(cmd: AgentCommand) -> Result<()> {
    let config = Config::load()?;
    match cmd {
        AgentCommand::Install => {
            let s = agent::install(&config)?;
            println!("installed  {}", s.plist.display());
            println!("binary     {}", s.binary.display());
            println!("schedule   every {} min, and at login", s.interval_mins);
            println!("log        {}", s.log.display());
            println!(
                "state      {}",
                if s.loaded { "loaded" } else { "NOT loaded — see the log" }
            );
            if config.roots.is_empty() {
                println!("\nnote: no roots configured, so only Claude Code will be read.");
            }
            Ok(())
        }
        AgentCommand::Uninstall => {
            let s = agent::uninstall(&config)?;
            println!(
                "removed    {}",
                if s.installed { "failed — plist still present" } else { "agent" }
            );
            println!("kept       {} and the archive", s.binary.display());
            Ok(())
        }
        AgentCommand::Status => {
            let s = agent::status(&config)?;
            print_agent(&s);
            Ok(())
        }
        AgentCommand::Run => {
            agent::run_now()?;
            println!("scan triggered; follow it with:\n  tail -f {}", paths::log_file()?.display());
            Ok(())
        }
    }
}

fn print_agent(s: &lore_core::agent::AgentStatus) {
    println!(
        "agent      {}",
        match (s.installed, s.loaded) {
            (true, true) => "installed and loaded",
            (true, false) => "installed but NOT loaded",
            (false, _) => "not installed — run `lore agent install`",
        }
    );
    println!("plist      {}", s.plist.display());
    println!(
        "binary     {}{}",
        s.binary.display(),
        if !s.binary_present {
            "  (missing)"
        } else if s.binary_stale {
            "  (older than this build — run `lore agent install`)"
        } else {
            ""
        }
    );
    println!("schedule   every {} min, and at login", s.interval_mins);
    println!("log        {}", s.log.display());
}

fn show_stats() -> Result<()> {
    let conn = db::open_default()?;
    let a = stats::archive(&conn)?;

    println!("records     {}", a.records);
    println!("sessions    {}", a.sessions);
    println!("projects    {}", a.projects);
    println!("files read  {}", a.files_tracked);
    println!(
        "span        {} → {}",
        a.earliest_ms.map(day).unwrap_or_else(|| "—".into()),
        a.latest_ms.map(day).unwrap_or_else(|| "—".into())
    );
    println!(
        "stored      {} of {} original ({} records truncated)",
        human_bytes(a.bytes_stored),
        human_bytes(a.bytes_original),
        a.truncated
    );

    println!("\nby kind");
    for (kind, count) in stats::by_kind(&conn)? {
        println!("  {count:>7}  {kind}");
    }

    println!("\nsessions per day (most recent first)");
    for (d, count) in stats::sessions_per_day(&conn, 14)? {
        println!("  {d}  {count}");
    }

    Ok(())
}

fn doctor() -> Result<()> {
    let config = Config::load()?;
    let config_path = paths::config_file()?;
    let db_path = paths::db_file()?;
    let claude_dir = config.claude_dir()?;

    println!(
        "config      {} {}",
        config_path.display(),
        if config_path.exists() { "" } else { "(absent, using defaults)" }
    );
    println!(
        "database    {} {}",
        db_path.display(),
        if db_path.exists() { "" } else { "(not yet created)" }
    );
    println!("interval    every {} min", config.scan_interval_mins);
    println!(
        "claude      {} {}",
        claude_dir.display(),
        if claude_dir.is_dir() { "" } else { "(missing)" }
    );

    if claude_dir.is_dir() {
        let found = claude::discover(&claude_dir)?;
        println!("            {} transcript files discoverable", found.len());
    }

    print_agent(&agent::status(&config)?);

    for (category, repos) in git::by_category(&config) {
        println!("repos       {} in [{}]", repos.len(), category);
    }

    if config.roots.is_empty() {
        println!("roots       none configured — git and file sources need them");
    } else {
        for root in &config.roots {
            println!(
                "root        {} [{}]{}",
                root.path.display(),
                root.category,
                if root.path.is_dir() { "" } else { " (missing)" }
            );
        }
    }

    Ok(())
}

fn day(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "—".into())
}

fn human_bytes(n: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut unit = 0;
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{v:.1} {}", UNITS[unit])
    }
}
