//! Phase Foster.0.8 — first-run setup wizard.
//!
//! Three-question flow that resolves the user's preferences for the
//! capability matrix and writes them to `~/.icelines/config.toml`
//! via `Config::save_sync`. Defaults match the spec: `transactions=
//! favorites`, `boxscores=favorites`, `policy=eager`. The wizard is
//! intentionally short — questions map 1:1 to a config setting and
//! the matrix carries everything else at its default.
//!
//! Modes:
//! - Interactive (default) — prompts on stdin/stdout
//! - `--accept-defaults` — writes defaults, prints a summary, no prompts
//! - `--dry-run` — prints what would be written, never touches disk
//! - `--reset` — re-runs the wizard even if config.toml already exists

use std::io::{BufRead, Write};

use anyhow::{Context, Result};

use crate::config::{BannerMode, CapabilityMatrix, CapabilityMode, Config, SyncConfig, SyncPolicy};

pub async fn run(accept_defaults: bool, dry_run: bool, reset: bool) -> Result<()> {
    run_with_io(
        accept_defaults,
        dry_run,
        reset,
        &mut std::io::stdin().lock(),
        &mut std::io::stdout(),
    )
}

/// Inner entrypoint with explicit stdin/stdout — lets tests drive
/// the prompt flow against a `Cursor<Vec<u8>>` without a real tty.
pub fn run_with_io<R: BufRead, W: Write>(
    accept_defaults: bool,
    dry_run: bool,
    _reset: bool,
    stdin: &mut R,
    stdout: &mut W,
) -> Result<()> {
    let cfg = Config::load().unwrap_or_else(|_| default_config());

    let sync = if accept_defaults {
        writeln!(stdout, "icelines setup — accepting defaults").ok();
        SyncConfig::default()
    } else {
        prompt_flow(stdin, stdout)?
    };

    print_summary(stdout, &sync)?;

    if dry_run {
        writeln!(stdout, "(dry run — no config written)").ok();
        return Ok(());
    }

    let mut new_cfg = cfg;
    new_cfg.sync = sync;
    new_cfg.save_sync().context("save sync config")?;
    writeln!(stdout, "Setup complete. Run `icelines tui` to start.").ok();
    Ok(())
}

fn default_config() -> Config {
    Config {
        csv_path: None,
        cache_dir: std::path::PathBuf::new(),
        season: None,
        live: None,
        dashboards: None,
        reports: Default::default(),
        sync: SyncConfig::default(),
        ai: crate::ai::AiConfig::default(),
    }
}

fn prompt_flow<R: BufRead, W: Write>(stdin: &mut R, stdout: &mut W) -> Result<SyncConfig> {
    writeln!(stdout, "icelines setup — three quick questions.\n").ok();
    let mut sync = SyncConfig::default();
    let mut caps = CapabilityMatrix::default();

    // Q1 — transactions scope.
    let q1 = ask_choice(
        stdin,
        stdout,
        "1. Track all NHL transactions, or just your favorites?",
        &[
            ("favorites", "Favorites only"),
            ("league", "Whole league"),
            ("off", "Skip"),
        ],
        0,
    )?;
    let mode = parse_mode(&q1)?;
    caps.set(crate::config::Capability::Transactions, mode)
        .context("set transactions")?;

    // Q2 — boxscores scope (deeper stats).
    let q2 = ask_choice(
        stdin,
        stdout,
        "2. Pull deeper stats (boxscores) for your favorites?",
        &[
            ("favorites", "Yes (favorites only, ~5 MB/wk)"),
            ("off", "No"),
        ],
        0,
    )?;
    let mode = parse_mode(&q2)?;
    caps.set(crate::config::Capability::Boxscores, mode)
        .context("set boxscores")?;

    // Q3 — sync policy.
    let q3 = ask_choice(
        stdin,
        stdout,
        "3. Refresh on app launch?",
        &[
            ("eager", "Eager (refresh in background, non-blocking)"),
            ("lazy", "Lazy (only on demand)"),
            ("off", "Off (manual `icelines fetch sync`)"),
        ],
        0,
    )?;
    sync.policy = match q3.as_str() {
        "eager" => SyncPolicy::Eager,
        "lazy" => SyncPolicy::Lazy,
        "off" => SyncPolicy::Off,
        other => anyhow::bail!("unrecognized policy '{other}'"),
    };

    sync.capabilities = caps;
    sync.banner = BannerMode::Summary;
    Ok(sync)
}

fn parse_mode(s: &str) -> Result<CapabilityMode> {
    CapabilityMode::parse(s).map_err(|e| anyhow::anyhow!(e))
}

fn ask_choice<R: BufRead, W: Write>(
    stdin: &mut R,
    stdout: &mut W,
    question: &str,
    choices: &[(&str, &str)],
    default_idx: usize,
) -> Result<String> {
    writeln!(stdout, "{question}").ok();
    for (i, (key, label)) in choices.iter().enumerate() {
        let marker = if i == default_idx {
            "[default]"
        } else {
            "         "
        };
        writeln!(stdout, "  {i}. {key:<10} {marker} — {label}").ok();
    }
    write!(
        stdout,
        "Choose [0-{}] (Enter = default): ",
        choices.len() - 1
    )
    .ok();
    stdout.flush().ok();

    let mut line = String::new();
    stdin.read_line(&mut line).context("read stdin")?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(choices[default_idx].0.to_string());
    }
    let n: usize = trimmed
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid choice '{trimmed}' — expected a number"))?;
    if n >= choices.len() {
        anyhow::bail!("choice {n} out of range (max {})", choices.len() - 1);
    }
    Ok(choices[n].0.to_string())
}

fn print_summary<W: Write>(stdout: &mut W, sync: &SyncConfig) -> Result<()> {
    writeln!(stdout, "\n── Resolved configuration ──").ok();
    writeln!(
        stdout,
        "  sync.policy             = {}",
        sync.policy.as_str()
    )
    .ok();
    writeln!(
        stdout,
        "  sync.banner             = {}",
        sync.banner.as_str()
    )
    .ok();
    writeln!(
        stdout,
        "  sync.season_transition  = {}",
        sync.season_transition.as_str()
    )
    .ok();
    writeln!(
        stdout,
        "  sync.capabilities.stats           = {}",
        sync.capabilities.stats.as_str()
    )
    .ok();
    writeln!(
        stdout,
        "  sync.capabilities.scores_schedule = {}",
        sync.capabilities.scores_schedule.as_str()
    )
    .ok();
    writeln!(
        stdout,
        "  sync.capabilities.transactions    = {}",
        sync.capabilities.transactions.as_str()
    )
    .ok();
    writeln!(
        stdout,
        "  sync.capabilities.boxscores       = {}",
        sync.capabilities.boxscores.as_str()
    )
    .ok();
    writeln!(
        stdout,
        "  sync.capabilities.shifts          = {}  (locked)",
        sync.capabilities.shifts.as_str()
    )
    .ok();
    writeln!(
        stdout,
        "  sync.capabilities.career_history  = {}",
        sync.capabilities.career_history.as_str()
    )
    .ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn l1_foster08_accept_defaults_writes_default_sync() {
        let mut out = Vec::new();
        let mut input = Cursor::new(Vec::new());
        // dry_run = true so we don't touch real disk.
        run_with_io(true, true, false, &mut input, &mut out).expect("ok");
        let s = String::from_utf8_lossy(&out);
        assert!(s.contains("accepting defaults"));
        assert!(s.contains("transactions    = favorites"));
        assert!(s.contains("boxscores       = favorites"));
        assert!(s.contains("shifts          = off"));
        assert!(s.contains("(dry run"));
    }

    #[test]
    fn l1_foster08_interactive_default_path_takes_defaults_on_empty_input() {
        // All three questions: empty (Enter) → default choice (idx 0)
        let mut input = Cursor::new(b"\n\n\n".to_vec());
        let mut out = Vec::new();
        run_with_io(false, true, false, &mut input, &mut out).expect("ok");
        let s = String::from_utf8_lossy(&out);
        // Default of every question is the first choice; for Q1 that's
        // favorites, Q2 favorites, Q3 eager. So the resolved config
        // is the spec default.
        assert!(s.contains("transactions    = favorites"));
        assert!(s.contains("boxscores       = favorites"));
        assert!(s.contains("policy             = eager"));
    }

    #[test]
    fn l1_foster08_interactive_invalid_choice_errors_clearly() {
        // First question gets "9" which is out of range (only 3 choices).
        let mut input = Cursor::new(b"9\n".to_vec());
        let mut out = Vec::new();
        let err = run_with_io(false, true, false, &mut input, &mut out)
            .expect_err("must error on out-of-range");
        let msg = err.to_string();
        assert!(msg.contains("out of range"), "got: {msg}");
    }
}
