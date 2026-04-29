use crate::cli::SchemeSubcommand;
use crate::commands::scheme_dialects::{detect_platform, dialect_for, matched_stats, Platform};
use anyhow::Context;
use icelines_core::scheme::Scheme;

pub async fn run(cmd: SchemeSubcommand) -> anyhow::Result<()> {
    match cmd {
        SchemeSubcommand::List => run_list().await,
        SchemeSubcommand::Show { name, source } => run_show(&name, source).await,
        SchemeSubcommand::FromCsv { path, name, platform } =>
            run_from_csv(&path, name.as_deref(), platform.as_deref()).await,
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
    // TODO Phase 2: also load user schemes from ~/.icelines/schemes/
    Ok(())
}

pub async fn run_show(name: &str, source: bool) -> anyhow::Result<()> {
    let scheme = find_builtin(name)
        .with_context(|| format!("scheme '{name}' not found — run `icelines scheme list`"))?;

    if source {
        // Phase 8f.5: --source emits the scheme as pretty JSON for
        // copy/paste, diffing, or piping into another tool.
        let json = serde_json::to_string_pretty(&scheme)
            .context("serializing scheme to JSON")?;
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
        let parsed = Platform::parse(p).ok_or_else(|| anyhow::anyhow!(
            "unknown platform '{p}' — supported: yahoo, espn, sleeper, fantrax"
        ))?;
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
    println!("Platform: {} ({} signature{} detected)",
        dialect.platform.name(),
        dialect.signatures.iter()
            .filter(|s| header.contains(*s))
            .count(),
        if dialect.signatures.iter().filter(|s| header.contains(*s)).count() == 1 { "" } else { "s" },
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
