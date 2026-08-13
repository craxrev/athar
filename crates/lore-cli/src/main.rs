//! `lore` — the collector, driven from the command line.
//!
//! The CLI exists so ingestion can be iterated on and verified without the UI
//! in the way. The desktop app reads the same database, read-only.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use lore_core::{collect::claude, config::Config, db, paths, stats};

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
    /// Inspect or create the configuration file.
    #[command(subcommand)]
    Config(ConfigCommand),
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
        Command::Config(cmd) => config(cmd),
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

    if !config.sources.claude.enabled {
        println!("claude source disabled in config; nothing to do");
        return Ok(());
    }

    let claude_dir = config.claude_dir()?;
    if !claude_dir.is_dir() {
        println!("no Claude Code directory at {}", claude_dir.display());
        return Ok(());
    }

    let started = std::time::Instant::now();
    let s = claude::scan(&mut conn, &claude_dir)
        .with_context(|| format!("scanning {}", claude_dir.display()))?;
    let elapsed = started.elapsed();

    println!("claude    {} files seen, {} read", s.files_seen, s.files_read);
    println!("archived  {} records ({} lines read)", s.inserted, s.lines_read);
    if s.duplicates > 0 {
        println!("already   {} records (rescan is a no-op)", s.duplicates);
    }
    println!("dropped   {} transient records", s.dropped);
    println!("truncated {} records with oversized blobs", s.truncated);
    if s.unparsed > 0 {
        println!("unparsed  {} lines archived verbatim", s.unparsed);
    }
    if s.files_reset > 0 {
        println!("reset     {} rotated files re-read", s.files_reset);
    }
    println!(
        "read      {} in {:.1}s",
        human_bytes(s.bytes_read as i64),
        elapsed.as_secs_f64()
    );
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
