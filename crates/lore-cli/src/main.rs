//! `lore` — the collector, driven from the command line.
//!
//! The CLI exists so ingestion can be iterated on and verified without the UI
//! in the way. The desktop app reads the same database, read-only.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use lore_core::{
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
    /// Remove archived records of kinds lore no longer keeps.
    ///
    /// Reports by default. The archive is append-only, so removing from it is a
    /// deliberate act and has to be asked for.
    Prune {
        /// Actually delete. Without this, nothing is written.
        #[arg(long)]
        apply: bool,
    },
    /// Exercise every query the desktop app makes, against the real archive.
    /// A failure here is a failure the app can only show as a stuck window.
    Check {
        /// Read this session rather than the first one found. Useful for timing
        /// the longest conversation in the archive rather than a typical one.
        #[arg(long)]
        session: Option<String>,
    },
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
        Command::Prune { apply } => prune(apply),
        Command::Check { session } => check(session.as_deref()),
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

/// Said rather than failed: the schedule fires this every interval, and a run
/// that declines because another is working is the system behaving, not an error
/// worth filling the log with.
fn already_running(existing: &db::Run) -> String {
    let seconds = (chrono::Utc::now().timestamp_millis() - existing.started_ms).max(0) / 1000;
    format!(
        "a {} has been running for {seconds}s (pid {}) — leaving it to finish",
        existing.action, existing.pid
    )
}

fn scan() -> Result<()> {
    let config = Config::load()?;
    let mut conn = db::open_default()?;
    let started = std::time::Instant::now();
    // Announced before any work: a scan runs for minutes, and until this the
    // window had no way to tell a busy collector from an idle one. Claiming also
    // refuses to start beside a collector that is already working.
    if let Some(existing) = db::claim_run(&mut conn, "scan")? {
        println!("{}", already_running(&existing));
        return Ok(());
    }

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

    db::mark_run_end(&conn)?;
    println!("\nfinished in {:.1}s", started.elapsed().as_secs_f64());
    Ok(())
}

fn rebuild() -> Result<()> {
    let config = Config::load()?;
    let mut conn = db::open_default()?;
    let started = std::time::Instant::now();
    if let Some(existing) = db::claim_run(&mut conn, "rebuild")? {
        println!("{}", already_running(&existing));
        return Ok(());
    }
    let d = derive::rebuild(&mut conn, &config)?;
    db::mark_run_end(&conn)?;
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
    let sum = stats::range_summary(&conn, from, to, None)?;

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

fn check(session: Option<&str>) -> Result<()> {
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
    step!("summary", api::summary(&conn, &config, week, now, None, None));
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
    let chosen = session.map(str::to_string).or_else(|| {
        blocks
            .iter()
            .find(|b| !b.sessions.is_empty())
            .map(|b| b.sessions[0].id.clone())
    });
    if let Some(id) = chosen {
        let detail = step!("session", api::session(&conn, &config, &id));
        if let Some(d) = detail {
            // The payload matters: every turn crosses the IPC boundary as a
            // markdown tree, and a long session is eight hundred of them.
            let bytes = serde_json::to_string(&d.turns).map(|s| s.len()).unwrap_or(0);
            println!(
                "        {} turns, {:.2} MB of turns over IPC",
                d.turns.len(),
                bytes as f64 / 1e6
            );
        }
    }

    println!("\nall queries answered");
    Ok(())
}

/// Removes records whose kind lore has stopped archiving.
///
/// These are already skipped on ingest, so this is only about the bytes stored
/// before that decision. Derived tables never read these kinds, so nothing needs
/// rebuilding afterwards — and the origin cursors are left alone, so a later scan
/// does not re-read the lines this removes.
fn prune(apply: bool) -> Result<()> {
    let conn = db::open_default()?;

    let mut total_rows = 0i64;
    let mut total_bytes = 0i64;
    println!("{:<24}{:>9}{:>11}", "kind", "records", "stored");
    for kind in lore_core::truncate::DROPPED_KINDS {
        let (rows, bytes): (i64, Option<i64>) = conn.query_row(
            "SELECT count(*), sum(length(json)) FROM raw_records WHERE kind = ?1",
            [kind],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if rows == 0 {
            continue;
        }
        println!("{:<24}{:>9}{:>10.1}M", kind, rows, bytes.unwrap_or(0) as f64 / 1e6);
        total_rows += rows;
        total_bytes += bytes.unwrap_or(0);
    }

    if total_rows == 0 {
        println!("\nnothing to remove");
        return Ok(());
    }
    println!(
        "\n{total_rows} records, {:.1} MB",
        total_bytes as f64 / 1e6
    );

    if !apply {
        println!("reporting only — run `lore prune --apply` to remove them");
        return Ok(());
    }

    let mut removed = 0usize;
    for kind in lore_core::truncate::DROPPED_KINDS {
        removed += conn.execute("DELETE FROM raw_records WHERE kind = ?1", [kind])?;
    }
    // Space is returned to the filesystem rather than left as free pages: the
    // point of this command is the disk it gives back.
    conn.execute_batch("VACUUM")?;
    println!("removed {removed} records");
    Ok(())
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
