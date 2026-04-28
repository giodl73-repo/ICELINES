use crate::cli::SchemeSubcommand;
use anyhow::Context;
use icelines_core::scheme::Scheme;

pub async fn run(cmd: SchemeSubcommand) -> anyhow::Result<()> {
    match cmd {
        SchemeSubcommand::List => run_list().await,
        SchemeSubcommand::Show { name } => run_show(&name).await,
        SchemeSubcommand::FromCsv { path, name } => run_from_csv(&path, name.as_deref()).await,
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

pub async fn run_show(name: &str) -> anyhow::Result<()> {
    let scheme = find_builtin(name)
        .with_context(|| format!("scheme '{name}' not found — run `icelines scheme list`"))?;

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

async fn run_from_csv(path: &str, name: Option<&str>) -> anyhow::Result<()> {
    // Detect scoreable columns from Yahoo CSV headers
    use std::io::BufRead;
    let file = std::fs::File::open(path).with_context(|| format!("opening {path}"))?;
    let mut reader = std::io::BufReader::new(file);
    let mut header = String::new();
    reader.read_line(&mut header)?;

    const STAT_COLS: &[(&str, &str)] = &[
        ("G (P)", "goals"),
        ("A (P)", "assists"),
        ("PPG (P)", "pp_goals"),
        ("PPA (P)", "pp_assists"),
        ("SHG (P)", "sh_goals"),
        ("SHA (P)", "sh_assists"),
        ("GWG (P)", "gwg"),
        ("HIT (P)", "hits"),
        ("BLK (P)", "blocks"),
        ("W (G)", "goalie_wins"),
        ("L (G)", "goalie_losses"),
        ("GA (G)", "goalie_ga"),
        ("SV (G)", "goalie_saves"),
        ("SHO (G)", "goalie_shutouts"),
    ];

    let detected: Vec<&str> = STAT_COLS
        .iter()
        .filter(|(col, _)| header.contains(col))
        .map(|(_, key)| *key)
        .collect();

    let scheme_name = name.unwrap_or("my-league");
    println!("Detected {} scoreable stats in '{path}':", detected.len());
    for k in &detected {
        println!("  {k}");
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
