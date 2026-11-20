//! Read a frozen preview cohort; emit exposure comparisons without changing cards.
use icelines_core::view_model::{
    rinkcraft_edge::EdgeEvidence,
    rinkcraft_edge_exposure::{compare, record},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{env, fs, path::Path};

fn run(directory: &Path, expected_hash: &str, output: &Path) -> Result<(), String> {
    if output.exists() {
        return Err("new output file required".into());
    }
    let catalog_bytes = fs::read(directory.join("catalog.json")).map_err(|e| e.to_string())?;
    let hash = format!("{:x}", Sha256::digest(&catalog_bytes));
    if hash != expected_hash {
        return Err("catalog checksum mismatch".into());
    }
    let catalog: Value = serde_json::from_slice(&catalog_bytes).map_err(|e| e.to_string())?;
    if catalog["schema"] != "rinkcraft.public-preview.v1" {
        return Err("unsupported catalog".into());
    }
    let mut records = Vec::new();
    let mut inputs = Vec::new();
    let mut excluded = Vec::new();
    for window in catalog["windows"].as_array().ok_or("missing windows")? {
        let season = window["season"].as_u64().ok_or("missing season")?;
        for actor in window["players"].as_array().ok_or("missing players")? {
            let id = actor["id"].as_u64().ok_or("missing player id")?;
            let relative = format!("data/{season}/{id}.json");
            if actor["url"] != relative {
                return Err("unexpected profile path".into());
            }
            let bytes = fs::read(directory.join(&relative)).map_err(|e| e.to_string())?;
            inputs.push(json!({"path":relative,"sha256":format!("{:x}",Sha256::digest(&bytes))}));
            let profile: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let frame = &profile["numerical_evidence"];
            if frame["player_id"] != id
                || frame["season"] != season
                || frame["group"] != actor["group"]
                || frame["games"] != actor["games"]
                || frame["situation"] != "all strengths"
            {
                return Err("profile/catalog scope mismatch".into());
            }
            if frame["group"] == "Goalies" {
                continue;
            }
            if frame["edge_evidence"].is_null() {
                excluded.push(json!({"season":season,"player_id":id,"reason":frame["edge_capture_status"]["reason"]}));
                continue;
            }
            let edge: EdgeEvidence = serde_json::from_value(frame["edge_evidence"].clone())
                .map_err(|e| e.to_string())?;
            if u64::from(edge.player_id) != id
                || u64::from(edge.season) != season
                || frame["group"] != edge.group
                || frame["games"] != edge.games
                || edge.source.url
                    != format!("https://api-web.nhle.com/v1/edge/skater-detail/{id}/{season}/2")
            {
                return Err("EDGE/profile binding mismatch".into());
            }
            records.push(record(
                &edge,
                frame["name"].as_str().ok_or("missing name")?,
                frame["minutes"].as_f64().ok_or("missing minutes")?,
            )?);
        }
    }
    let mut scenarios = Vec::new();
    for min_games in [5, 10, 20] {
        let rows = compare(&records, min_games, 100.)?;
        scenarios.push(json!({"min_games":min_games,"min_minutes":100,"rows":rows}));
    }
    let result = json!({
        "schema":"rinkcraft.edge-exposure.v1",
        "status":"descriptive_sensitivity_analysis",
        "catalog_sha256":hash,"inputs":inputs,"excluded_skaters":excluded,
        "formulas":{"bursts_per_game":"bursts / GP","bursts_per_60":"bursts * 60 / official all-strength TOI minutes","percentile":"100 * (count below + 0.5 * count tied) / measured eligible same-season same-position-group peers"},
        "notes":["No attribute grades, feat awards or simulation parameters are produced.","Minimum exposure thresholds are authored sensitivity settings, not established reliability cutoffs.","All-strength official TOI is an exposure denominator, not tracked skating time or a count of comparable sprint opportunities.","Tracking coverage, role, strength, score, fatigue and rink effects remain uncontrolled.","Peak speed is a sample maximum; exposure normalization does not remove its opportunity dependence.","IceLines positional midranks are separate from NHL provider percentiles. No season pooling or goalie-skater comparisons."],
        "scenarios":scenarios
    });
    fs::write(
        output,
        serde_json::to_vec(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "{} skater records; {} inputs; output {}",
        records.len(),
        inputs.len(),
        output.display()
    );
    Ok(())
}
fn main() {
    let a: Vec<_> = env::args().collect();
    if a.len() != 4 {
        eprintln!("usage: rinkcraft_edge_exposure FROZEN_DIRECTORY CATALOG_SHA256 NEW_OUTPUT_JSON");
        std::process::exit(2);
    }
    if let Err(e) = run(Path::new(&a[1]), &a[2], Path::new(&a[3])) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
