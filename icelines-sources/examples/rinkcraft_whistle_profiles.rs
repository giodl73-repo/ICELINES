//! Add signed-record diagnostics to frozen scope fixtures without rewriting numbers.
use icelines_core::view_model::rinkcraft_whistle::{record, RELATIVE_NAME};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{env, fs};
fn run(bytes: &[u8], reports: &[u8]) -> Result<Value, String> {
    let parsed = icelines_sources::nhl::achievements::parse(reports)?;
    let source: Value = serde_json::from_slice(reports).map_err(|e| e.to_string())?;
    let reports_hash = format!("{:x}", Sha256::digest(reports));
    let input: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let mut output = serde_json::Map::new();
    for (key, p) in input.as_object().ok_or("fixture map required")? {
        let actor = p["request"]["player_id"].as_u64().ok_or("actor missing")?;
        let mut patches = Vec::new();
        for (scope, frame) in [
            ("season", p),
            ("current", &p["scope_variants"]["current"]),
            ("career", &p["scope_variants"]["career"]),
        ] {
            let w = &frame["profile_window"];
            let patterns = &frame["experimental_patterns"];
            if w["kind"] != scope
                || w["player_id"].as_u64() != Some(actor)
                || patterns["player_id"].as_u64() != Some(actor)
                || patterns["season"] != w["season"]
                || patterns["season_type"] != w["season_type"]
                || patterns["group"] != frame["numerical_evidence"]["group"]
                || patterns["schema"] != "rinkcraft.scoped-patterns.v1"
            {
                return Err("incompatible profile/pattern scope".into());
            }
            let list = patterns["patterns"]
                .as_array()
                .ok_or("missing pattern list")?;
            let rows: Vec<_> = list
                .iter()
                .filter(|p| p["id"] == "penalty-balance")
                .collect();
            if rows.len() > 1 || scope == "career" && !rows.is_empty() {
                return Err("duplicated or invalid career whistle pattern".into());
            }
            if let Some(pattern) = rows.first() {
                let source_row = if scope == "season" {
                    if patterns["source_sha256"].as_str() != Some(reports_hash.as_str())
                        || w["season"].as_u64() != Some(parsed.season as u64)
                    {
                        return Err("season report does not match original reference source".into());
                    }
                    let original = parsed
                        .players
                        .iter()
                        .find(|p| p.player_id as u64 == actor)
                        .ok_or("source actor missing")?;
                    if original.group != patterns["group"].as_str().ok_or("missing group")?
                        || pattern["opportunities"]["games"].as_u64() != Some(original.games as u64)
                    {
                        return Err("season source GP/group mismatch".into());
                    }
                    Some(
                        source["penalties"]["data"]
                            .as_array()
                            .ok_or("missing penalty report")?
                            .iter()
                            .find(|r| r["playerId"].as_u64() == Some(actor))
                            .ok_or("missing source penalty actor")?,
                    )
                } else {
                    None
                };
                let read = |key: &str| -> Result<Option<u32>, String> {
                    let existing = &pattern["opportunities"][key];
                    let raw_key = if key == "penalties_drawn" {
                        "penaltiesDrawn"
                    } else {
                        "penalties"
                    };
                    let v = if let Some(row) = source_row {
                        let raw = &row[raw_key];
                        if !existing.is_null() && existing != raw {
                            return Err("existing/source count conflict".into());
                        }
                        raw
                    } else {
                        existing
                    };
                    if v.is_null() {
                        return Ok(None);
                    }
                    v.as_u64()
                        .and_then(|n| u32::try_from(n).ok())
                        .map(Some)
                        .ok_or_else(|| format!("invalid count {key}"))
                };
                let drawn = read("penalties_drawn")?;
                let taken = read("penalties_taken")?;
                patches.push(json!({"scope":scope,"player_id":actor,"season":w["season"],"season_type":w["season_type"],"as_of":w["as_of"],"group":patterns["group"],"source_sha256":patterns["source_sha256"],"id":"penalty-balance","name":RELATIVE_NAME,"opportunities":{"penalties_drawn":drawn,"penalties_taken":taken},"verified_season_reports_sha256":if scope=="season"{Some(&reports_hash)}else{None},"whistle_record":record(drawn,taken)}));
            }
        }
        output.insert(key.clone(), Value::Array(patches));
    }
    Ok(
        json!({"schema":"rinkcraft.whistle-profile-patches.v1","input_sha256":format!("{:x}",Sha256::digest(bytes)),"patches":output}),
    )
}
fn main() {
    let a: Vec<_> = env::args().collect();
    let f = || -> Result<(), String> {
        if a.len() != 4 {
            return Err(
                "Usage: THREE_SCOPE_FIXTURES VERIFIED_SEASON_REPORTS OUTPUT_PATCHES".into(),
            );
        }
        let b = fs::read(&a[1]).map_err(|e| e.to_string())?;
        fs::write(
            &a[3],
            serde_json::to_vec(&run(&b, &fs::read(&a[2]).map_err(|e| e.to_string())?)?)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!("Projected signed whistle records without changing qualification.");
        Ok(())
    };
    if let Err(e) = f() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
