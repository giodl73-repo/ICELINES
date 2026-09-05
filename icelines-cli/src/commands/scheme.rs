use crate::cli::SchemeSubcommand;
use crate::commands::scheme_dialects::{detect_platform, dialect_for, matched_stats, Platform};
use anyhow::Context;
use icelines_core::scheme::Scheme;

pub async fn run(cmd: SchemeSubcommand) -> anyhow::Result<()> {
    match cmd {
        SchemeSubcommand::List => run_list().await,
        SchemeSubcommand::Show { name, source } => run_show(&name, source).await,
        SchemeSubcommand::FromCsv {
            path,
            name,
            platform,
        } => run_from_csv(&path, name.as_deref(), platform.as_deref()).await,
    }
}

async fn run_list() -> anyhow::Result<()> {
    println!("{:<20} {:<12} {:<40}", "Name", "Source", "Description");
    println!("{}", "─".repeat(60usize));
    for s in Scheme::all_builtins() {
        println!(
            "{:<20} {:<12} {}",
            s.name,
            format!("{:?}", s.source).to_lowercase(),
            s.description
        );
    }
    // Phase 8f.9: surface user schemes alongside builtins. Errors loading any
    // single file are warned about but don't break listing.
    match load_user_schemes() {
        Ok(user) => {
            for s in user {
                println!("{:<20} {:<12} {}", s.name, "user", s.description,);
            }
        }
        Err(e) => eprintln!("  warning: could not enumerate user schemes — {e}"),
    }
    Ok(())
}

pub async fn run_show(name: &str, source: bool) -> anyhow::Result<()> {
    let scheme = find_scheme(name)
        .with_context(|| format!(
            "scheme '{name}' not found — run `icelines scheme list`, or check ~/.icelines/schemes/{name}.toml"
        ))?;

    if source {
        // Phase 8f.5: --source emits the scheme as pretty JSON for
        // copy/paste, diffing, or piping into another tool.
        let json = serde_json::to_string_pretty(&scheme).context("serializing scheme to JSON")?;
        println!("{json}");
        return Ok(());
    }

    println!("Scheme:   {}", scheme.name);
    println!("Source:   {:?}", scheme.source);
    println!("Desc:     {}", scheme.description);
    println!();
    println!("Skater scoring:");
    let w = &scheme.skater;
    let fields: &[(&str, f32)] = &[
        ("goals", w.goals),
        ("assists", w.assists),
        ("pp_goals", w.pp_goals),
        ("pp_assists", w.pp_assists),
        ("sh_goals", w.sh_goals),
        ("sh_assists", w.sh_assists),
        ("gwg", w.gwg),
        ("ot_goals", w.ot_goals),
        ("hits", w.hits),
        ("blocks", w.blocks),
        ("shots_on_goal", w.shots_on_goal),
        ("plus_minus", w.plus_minus),
        ("takeaways", w.takeaways),
        ("giveaways", w.giveaways),
        ("faceoff_wins", w.faceoff_wins),
    ];
    for (k, v) in fields {
        if v.abs() > 0.0001 {
            println!("  {k:<18} {v:>6.2}");
        }
    }
    println!();
    println!("Goalie scoring:");
    let g = &scheme.goalie;
    let gfields: &[(&str, f32)] = &[
        ("wins", g.wins),
        ("losses", g.losses),
        ("saves", g.saves),
        ("goals_against", g.goals_against),
        ("shutouts", g.shutouts),
        ("save_pct", g.save_pct),
    ];
    for (k, v) in gfields {
        if v.abs() > 0.0001 {
            println!("  {k:<18} {v:>6.2}");
        }
    }
    Ok(())
}

async fn run_from_csv(
    path: &str,
    name: Option<&str>,
    platform: Option<&str>,
) -> anyhow::Result<()> {
    use std::io::BufRead;
    let file = std::fs::File::open(path).with_context(|| format!("opening {path}"))?;
    let mut reader = std::io::BufReader::new(file);
    let mut header = String::new();
    reader.read_line(&mut header)?;

    // Phase 8f.7: pick a dialect by explicit --platform, otherwise auto-detect.
    let dialect = if let Some(p) = platform {
        let parsed = Platform::parse(p).ok_or_else(|| {
            anyhow::anyhow!("unknown platform '{p}' — supported: yahoo, espn, sleeper, fantrax")
        })?;
        dialect_for(parsed)
    } else {
        match detect_platform(&header) {
            Some(d) => d,
            None => anyhow::bail!(
                "unrecognized CSV format — header has no Yahoo / ESPN / Sleeper / \
                 Fantrax signature columns.\n  Try `--platform <yahoo|espn|sleeper|fantrax>` \
                 to force a specific dialect."
            ),
        }
    };

    let stats = matched_stats(dialect, &header);
    let scheme_name = name.unwrap_or("my-league");
    println!(
        "Platform: {} ({} signature{} detected)",
        dialect.platform.name(),
        dialect
            .signatures
            .iter()
            .filter(|s| header.contains(*s))
            .count(),
        if dialect
            .signatures
            .iter()
            .filter(|s| header.contains(*s))
            .count()
            == 1
        {
            ""
        } else {
            "s"
        },
    );
    println!("Detected {} scoreable stat(s) in '{path}':", stats.len());
    for (col, key) in &stats {
        println!("  {col:<14} → {key}");
    }
    println!();
    println!("Template scheme name: '{scheme_name}'");
    println!("Edit data/schemes/{scheme_name}.toml and set weights for each stat.");
    println!("Then use: icelines scheme show {scheme_name}");
    Ok(())
}

fn find_builtin(name: &str) -> Option<Scheme> {
    Scheme::all_builtins().into_iter().find(|s| s.name == name)
}

// ── Phase 8f.9: user schemes from ~/.icelines/schemes/ ──────────────────────

/// Look up a scheme by name, preferring a user file when present so users can
/// override a builtin (e.g., a tweaked `yahoo-standard.toml`).
pub(crate) fn find_scheme(name: &str) -> Option<Scheme> {
    if let Some(s) = load_user_scheme(name) {
        return Some(s);
    }
    find_builtin(name)
}

/// Resolve `~/.icelines/schemes/{name}.toml` and parse it as a `Scheme`.
/// Returns `None` if the file is absent; `None` (with no signal to caller)
/// if the file is malformed — the caller falls back to the builtin lookup,
/// and a `scheme show` of a malformed user file will surface the parse error
/// via the dedicated `load_user_scheme_strict` path below.
fn load_user_scheme(name: &str) -> Option<Scheme> {
    let path = user_schemes_dir().ok()?.join(format!("{name}.toml"));
    if !path.exists() {
        return None;
    }
    let text = std::fs::read_to_string(&path).ok()?;
    toml::from_str(&text).ok()
}

/// Enumerate every `*.toml` file under `~/.icelines/schemes/` and parse it.
/// Failed parses are skipped silently to keep `scheme list` robust against
/// half-edited templates; `scheme show NAME` is the place for strict errors.
fn load_user_schemes() -> anyhow::Result<Vec<Scheme>> {
    let dir = user_schemes_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let parsed: Result<Scheme, _> = toml::from_str(&text);
        if let Ok(s) = parsed {
            out.push(s);
        } else {
            eprintln!("  warning: could not parse {}", path.display());
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn user_schemes_dir() -> anyhow::Result<std::path::PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    Ok(home.join(".icelines").join("schemes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Set HOME/USERPROFILE so `user_schemes_dir()` resolves into a tempdir.
    /// Returns the temp `~/.icelines/schemes/` ready to receive `.toml` files.
    fn isolate_home() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path().to_path_buf();
        // Set both env vars; production code falls back from HOME to USERPROFILE.
        std::env::set_var("HOME", &home);
        std::env::set_var("USERPROFILE", &home);
        let schemes = home.join(".icelines").join("schemes");
        std::fs::create_dir_all(&schemes).unwrap();
        (dir, schemes)
    }

    fn write_user_scheme(dir: &std::path::Path, name: &str, body: &str) {
        std::fs::write(dir.join(format!("{name}.toml")), body).unwrap();
    }

    /// A minimal valid scheme TOML matching the Scheme serde shape.
    fn minimal_scheme_toml(name: &str) -> String {
        format!(
            r#"
name = "{name}"
description = "test scheme"
source = "custom"

[skater]
goals = 4.0
assists = 3.0

[goalie]
wins = 5.0
"#
        )
    }

    #[test]
    fn l0_load_user_scheme_round_trips_known_name() {
        // NOTE: env vars are process-global; tests in the same binary run in
        // separate threads, so this test serializes lookups via a Mutex when
        // run alongside others. cargo runs tests in parallel by default but
        // toml parsing is stateless — the only contention is HOME.
        let _guard = crate::test_utils::home_env_lock();
        let (_keep, schemes) = isolate_home();
        write_user_scheme(&schemes, "my-league", &minimal_scheme_toml("my-league"));

        let s = load_user_scheme("my-league").expect("user scheme resolves");
        assert_eq!(s.name, "my-league");
        assert_eq!(s.description, "test scheme");
        assert_eq!(s.skater.goals, 4.0);
        assert_eq!(s.goalie.wins, 5.0);
    }

    #[test]
    fn l0_find_scheme_prefers_user_over_builtin() {
        let _guard = crate::test_utils::home_env_lock();
        let (_keep, schemes) = isolate_home();
        // Override the builtin yahoo-standard with a user scheme.
        write_user_scheme(
            &schemes,
            "yahoo-standard",
            &minimal_scheme_toml("yahoo-standard"),
        );
        let s = find_scheme("yahoo-standard").expect("must resolve");
        // Our user scheme has goals=4.0 — different from the real builtin (3.0).
        assert_eq!(s.skater.goals, 4.0, "user scheme should override builtin");
    }

    #[test]
    fn l0_find_scheme_falls_back_to_builtin_when_user_absent() {
        let _guard = crate::test_utils::home_env_lock();
        let (_keep, _schemes) = isolate_home();
        // No user file present — should still resolve the builtin.
        let s = find_scheme("yahoo-standard").expect("builtin must resolve");
        assert_eq!(s.name, "yahoo-standard");
        // Builtin yahoo-standard goals = 3.0 (matches scheme.rs:103).
        assert_eq!(s.skater.goals, 3.0);
    }

    #[test]
    fn l0_load_user_schemes_skips_malformed_files() {
        let _guard = crate::test_utils::home_env_lock();
        let (_keep, schemes) = isolate_home();
        write_user_scheme(&schemes, "valid", &minimal_scheme_toml("valid"));
        write_user_scheme(&schemes, "broken", "this is not valid toml [[[");

        let all = load_user_schemes().expect("listing must not fail");
        let names: Vec<&str> = all.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"valid"));
        assert!(
            !names.contains(&"broken"),
            "malformed toml should be skipped, got: {names:?}"
        );
    }

    #[test]
    fn l0_load_user_schemes_empty_dir_returns_empty() {
        let _guard = crate::test_utils::home_env_lock();
        let (_keep, _schemes) = isolate_home();
        let all = load_user_schemes().expect("must succeed on empty dir");
        assert!(all.is_empty(), "empty dir should produce empty list");
    }
}
