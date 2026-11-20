//! Attach seasonal skating and optional shooting research; preserve existing JSON tokens.
use serde_json::{
    json,
    value::{to_raw_value, RawValue},
    Value,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, env, fs, path::Path};
type RawMap = BTreeMap<String, Box<RawValue>>;
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn attach(bytes: &[u8], key: &str, candidate: &Value) -> Result<Vec<u8>, String> {
    let mut root: RawMap = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let mut frame: RawMap =
        serde_json::from_str(root.get("numerical_evidence").ok_or("missing frame")?.get())
            .map_err(|e| e.to_string())?;
    if frame.contains_key(key) {
        return Err("candidate already attached".into());
    }
    frame.insert(
        key.into(),
        to_raw_value(candidate).map_err(|e| e.to_string())?,
    );
    root.insert(
        "numerical_evidence".into(),
        to_raw_value(&frame).map_err(|e| e.to_string())?,
    );
    serde_json::to_vec(&root).map_err(|e| e.to_string())
}
fn project(bytes: &[u8], candidate: &Value) -> Result<Vec<u8>, String> {
    attach(bytes, "edge_skating_candidate", candidate)
}
fn run(
    exposure: &Path,
    candidates: &Path,
    base: &Path,
    out: &Path,
    shooting: Option<&Path>,
) -> Result<(), String> {
    if out.exists() {
        return Err("new output directory required".into());
    }
    let exposure_bytes = fs::read(exposure).map_err(|e| e.to_string())?;
    let exposure: Value = serde_json::from_slice(&exposure_bytes).map_err(|e| e.to_string())?;
    let candidate_bytes = fs::read(candidates).map_err(|e| e.to_string())?;
    let report: Value = serde_json::from_slice(&candidate_bytes).map_err(|e| e.to_string())?;
    let candidate_report_hash = digest(&candidate_bytes);
    if exposure["schema"] != "rinkcraft.edge-exposure.v1"
        || report["schema"] != "rinkcraft.edge-feat-candidates.v1"
        || report["exposure_report_sha256"] != digest(&exposure_bytes)
        || report["rules"]["status"] != "research_candidates_not_awarded"
        || report["rules_sha256"] != digest(&serde_json::to_vec(&report["rules"]).unwrap())
    {
        return Err("candidate lineage mismatch".into());
    }
    let catalog_bytes = fs::read(base.join("catalog.json")).map_err(|e| e.to_string())?;
    if exposure["catalog_sha256"] != digest(&catalog_bytes)
        || report["catalog_sha256"] != exposure["catalog_sha256"]
    {
        return Err("catalog checksum mismatch".into());
    }
    let catalog: Value = serde_json::from_slice(&catalog_bytes).map_err(|e| e.to_string())?;
    let mut inventory = BTreeMap::new();
    for item in exposure["inputs"]
        .as_array()
        .ok_or("missing input inventory")?
    {
        if inventory
            .insert(
                item["path"].as_str().ok_or("missing path")?.to_string(),
                item["sha256"].as_str().ok_or("missing hash")?.to_string(),
            )
            .is_some()
        {
            return Err("duplicate inventory path".into());
        }
    }
    let scenarios: Vec<_> = report["scenarios"]
        .as_array()
        .ok_or("missing scenarios")?
        .iter()
        .filter(|s| s["min_games"] == 10 && s["min_minutes"] == 100)
        .collect();
    if scenarios.len() != 1 {
        return Err("exactly one 10-GP/100-minute scenario required".into());
    }
    let mut records = BTreeMap::new();
    for row in scenarios[0]["rows"].as_array().ok_or("missing rows")? {
        let key = (
            row["season"].as_u64().ok_or("missing season")?,
            row["player_id"].as_u64().ok_or("missing actor")?,
        );
        if records.insert(key, row).is_some() {
            return Err("duplicate candidate actor/window".into());
        }
    }
    let mut attached = 0;
    let mut qualified = 0;
    let mut shooting_rows = BTreeMap::new();
    let mut shooting_rules = Value::Null;
    let mut shooting_hash = String::new();
    if let Some(path) = shooting {
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        shooting_hash = digest(&bytes);
        let report: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if report["schema"] != "rinkcraft.shooting-records.v1"
            || report["exposure_report_sha256"] != digest(&exposure_bytes)
            || report["catalog_sha256"] != exposure["catalog_sha256"]
            || report["rules"]["status"] != "research_candidates_not_awarded"
            || report["rules_sha256"] != digest(&serde_json::to_vec(&report["rules"]).unwrap())
        {
            return Err("shooting lineage mismatch".into());
        }
        shooting_rules = report["rules_sha256"].clone();
        let selected: Vec<_> = report["scenarios"]
            .as_array()
            .ok_or("missing shooting scenarios")?
            .iter()
            .filter(|s| {
                s["minimum_conversion_shots"] == 25
                    && s["minimum_games"] == 10
                    && s["minimum_minutes"] == 100
            })
            .collect();
        if selected.len() != 1 {
            return Err("one 25-SOG audit scenario required".into());
        }
        for row in selected[0]["rows"]
            .as_array()
            .ok_or("missing shooting rows")?
        {
            let r = &row["record"];
            let key = (
                r["season"].as_u64().ok_or("missing season")?,
                r["player_id"].as_u64().ok_or("missing actor")?,
            );
            if shooting_rows.insert(key, row.clone()).is_some() {
                return Err("duplicate shooting row".into());
            }
        }
    }
    let mut shooting_attached = 0;
    let mut shooting_qualified = 0;
    for window in catalog["windows"].as_array().ok_or("missing windows")? {
        let season = window["season"].as_u64().ok_or("missing season")?;
        for actor in window["players"].as_array().ok_or("missing actors")? {
            let id = actor["id"].as_u64().ok_or("missing actor")?;
            let relative = format!("data/{season}/{id}.json");
            if actor["url"] != relative {
                return Err("unexpected profile path".into());
            }
            let bytes = fs::read(base.join(&relative)).map_err(|e| e.to_string())?;
            let hash = digest(&bytes);
            if inventory.remove(&relative).as_deref() != Some(&hash) {
                return Err("profile checksum mismatch".into());
            }
            let profile: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let frame = &profile["numerical_evidence"];
            if frame["player_id"] != id
                || frame["season"] != season
                || frame["group"] != actor["group"]
                || frame["games"] != actor["games"]
                || frame["situation"] != "all strengths"
            {
                return Err("profile/catalog binding mismatch".into());
            }
            let mut projected = if let Some(row) = records.remove(&(season, id)) {
                if row["group"] != frame["group"]
                    || row["edge_sha256"] != frame["edge_evidence"]["source"]["sha256"]
                    || frame["group"] == "Goalies"
                {
                    return Err("candidate/frame binding mismatch".into());
                }
                let mut candidate = row.clone();
                for (key, value) in [
                    ("schema", json!("icelines.edge-skating-candidate.v1")),
                    ("season_type", json!("regular")),
                    ("status", json!("research_candidate")),
                    ("games", frame["games"].clone()),
                    ("minutes", frame["minutes"].clone()),
                    ("min_games", json!(10)),
                    ("min_minutes", json!(100)),
                    ("rules_sha256", report["rules_sha256"].clone()),
                    ("candidate_report_sha256", json!(candidate_report_hash)),
                    ("source_profile_sha256", json!(hash)),
                ] {
                    candidate[key] = value;
                }
                attached += 1;
                if !candidate["candidate"].is_null() {
                    qualified += 1;
                }
                project(&bytes, &candidate)?
            } else {
                bytes
            };
            if let Some(mut row) = shooting_rows.remove(&(season, id)) {
                let r = &row["record"];
                if r["source_profile_sha256"] != hash
                    || r["group"] != frame["group"]
                    || r["games"] != frame["games"]
                    || r["edge_sha256"] != frame["edge_evidence"]["source"]["sha256"]
                    || frame["group"] == "Goalies"
                {
                    return Err("shooting/profile binding mismatch".into());
                }
                row["schema"] = json!("icelines.shooting-candidate.v1");
                row["status"] = json!("research_candidate");
                row["season_type"] = json!("regular");
                row["minimum_games"] = json!(10);
                row["minimum_minutes"] = json!(100);
                row["minimum_conversion_shots"] = json!(25);
                row["rules_sha256"] = shooting_rules.clone();
                row["candidate_report_sha256"] = json!(shooting_hash);
                shooting_attached += 1;
                if !row["candidate"].is_null() {
                    shooting_qualified += 1;
                }
                projected = attach(&projected, "edge_shooting_candidate", &row)?;
            }
            let destination = out.join(relative);
            fs::create_dir_all(destination.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::write(destination, projected).map_err(|e| e.to_string())?;
        }
    }
    if !records.is_empty() || !inventory.is_empty() || !shooting_rows.is_empty() {
        return Err("unconsumed actor or inventory records".into());
    }
    fs::write(out.join("catalog.json"), catalog_bytes).map_err(|e| e.to_string())?;
    fs::copy(
        base.join("edge-coverage.json"),
        out.join("edge-coverage.json"),
    )
    .map_err(|e| e.to_string())?;
    println!(
        "Attached {attached} research contexts; {qualified} candidate labels across four seasons."
    );
    println!("Shooting: {shooting_attached} contexts; {shooting_qualified} candidate labels.");
    Ok(())
}
fn main() {
    let a: Vec<_> = env::args().collect();
    if a.len() != 5 && a.len() != 6 {
        eprintln!("usage: rinkcraft_edge_candidate_profiles EXPOSURE_JSON CANDIDATE_JSON BASE_DIRECTORY NEW_DIRECTORY [SHOOTING_JSON]");
        std::process::exit(2);
    }
    if let Err(e) = run(
        Path::new(&a[1]),
        Path::new(&a[2]),
        Path::new(&a[3]),
        Path::new(&a[4]),
        a.get(5).map(Path::new),
    ) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_original_tokens_and_rejects_duplicate() {
        let bytes=br#"{"numerical_evidence":{"minutes":0.1234567890123456789,"unknown":null},"outside":1.234567890123456789}"#;
        let out = project(bytes, &json!({"state":"unassessed"})).unwrap();
        let text = String::from_utf8(out.clone()).unwrap();
        assert!(text.contains("0.1234567890123456789"));
        assert!(text.contains("1.234567890123456789"));
        assert!(text.contains("\"unknown\":null"));
        assert!(project(&out, &json!({})).is_err());
        let both = attach(
            &out,
            "edge_shooting_candidate",
            &json!({"state":"unassessed"}),
        )
        .unwrap();
        let root: Value = serde_json::from_slice(&both).unwrap();
        assert!(root["numerical_evidence"]
            .get("edge_skating_candidate")
            .is_some());
        assert!(root["numerical_evidence"]
            .get("edge_shooting_candidate")
            .is_some());
        assert!(attach(&both, "edge_shooting_candidate", &json!({})).is_err());
    }
}
