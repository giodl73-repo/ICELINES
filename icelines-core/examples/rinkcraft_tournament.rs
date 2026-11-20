//! One local command: draft/verified request -> seeded tournament and HTML report.
use icelines_core::view_model::rinkcraft_task_season::tournament as t;
use std::{fs, path::Path};
fn write_result(q: &t::Request, result: &t::ResultSet, out: &Path) -> Result<(), String> {
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    fs::write(
        out.join("request.json"),
        serde_json::to_vec_pretty(q).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        out.join("tournament.json"),
        serde_json::to_vec_pretty(result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(out.join("index.html"), t::report::html(result)).map_err(|e| e.to_string())?;
    Ok(())
}
fn read_request(path: &str) -> Result<t::Request, String> {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if value["schema"] == "rinkcraft.tournament-draft.0.1"
        || value["schema"] == "rinkcraft.tournament-draft.0.2"
    {
        t::build(serde_json::from_value(value).map_err(|e| e.to_string())?)
    } else {
        serde_json::from_value(value).map_err(|e| e.to_string())
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() == 4 && args[0] == "study" {
        let base = read_request(&args[1])?;
        let plan: t::study::Plan =
            serde_json::from_slice(&fs::read(&args[2]).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let result = t::study::run(&base, &plan)?;
        let out = Path::new(&args[3]);
        fs::create_dir_all(out).map_err(|e| e.to_string())?;
        for (v, r) in &result.variants {
            write_result(&r.request, r, &out.join(&v.id))?;
            println!("{}: {}", v.id, r.illustrative.champion);
        }
        fs::write(
            out.join("comparison.json"),
            serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::write(out.join("index.html"), t::study::html(&result)).map_err(|e| e.to_string())?;
        return Ok(());
    }
    if args.len() != 2 {
        return Err("Usage: rinkcraft_tournament INPUT OUTPUT | study BASE PLAN OUTPUT_DIR".into());
    }
    let q = read_request(&args[0])?;
    let result = t::run(&q)?;
    let out = Path::new(&args[1]);
    write_result(&q, &result, out)?;
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
