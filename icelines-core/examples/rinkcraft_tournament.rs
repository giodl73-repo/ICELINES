//! One local command: draft/verified request -> seeded tournament and HTML report.
use icelines_core::view_model::rinkcraft_task_season::tournament as t;
use std::{fs, path::Path};
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("Usage: rinkcraft_tournament INPUT_JSON OUTPUT_DIR".into());
    }
    let raw = fs::read(&args[0]).map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    let q = if value["schema"] == "rinkcraft.tournament-draft.0.1" {
        t::build(serde_json::from_value(value).map_err(|e| e.to_string())?)?
    } else {
        serde_json::from_value::<t::Request>(value).map_err(|e| e.to_string())?
    };
    let result = t::run(&q)?;
    let out = Path::new(&args[1]);
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    fs::write(
        out.join("request.json"),
        serde_json::to_vec_pretty(&q).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        out.join("tournament.json"),
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(out.join("index.html"), t::report::html(&result)).map_err(|e| e.to_string())?;
    println!(
        "{} teams / {} characters / {} games; champion: {}; {} title-frequency trials",
        q.teams.len(),
        q.teams
            .iter()
            .map(|t| t.roster.members.len())
            .sum::<usize>(),
        result.illustrative.games.len(),
        result.illustrative.champion,
        q.batch_trials
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
