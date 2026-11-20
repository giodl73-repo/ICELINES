//! Project record markers only from reconciled career fields.
use icelines_core::view_model::rinkcraft_record_milestones::{self as milestones, Input};
use serde_json::Value;
use std::{collections::BTreeMap, env, fs};
fn main() {
    let a: Vec<_> = env::args().collect();
    let run = || -> Result<(), String> {
        if a.len() != 3 {
            return Err("Usage: THREE_SCOPE_FIXTURES OUTPUT_MILESTONE_PATCHES".into());
        }
        let profiles: Value = serde_json::from_slice(&fs::read(&a[1]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let mut out = BTreeMap::new();
        for (key, profile) in profiles
            .as_object()
            .ok_or("explicit fixture map required")?
        {
            let career = &profile["scope_variants"]["career"];
            let w = &career["profile_window"];
            let reconciliation = &career["career_reconciliation"];
            if w["kind"] != "career"
                || w["coverage"] != "career_reconciled"
                || reconciliation["state"] != "core_counts_reconciled"
            {
                return Err("record markers require reconciled career scope".into());
            }
            let actor = w["player_id"]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or("invalid actor")?;
            if career["request"]["player_id"].as_u64() != Some(actor as u64)
                || profile["request"]["player_id"].as_u64() != Some(actor as u64)
            {
                return Err("record actor mismatch".into());
            }
            let mut counts = BTreeMap::new();
            for field in reconciliation["fields"]
                .as_array()
                .ok_or("missing career quality fields")?
            {
                let key = field["key"].as_str().ok_or("missing record key")?;
                if matches!(
                    field["state"].as_str(),
                    Some("reconciled" | "complete_report_windows")
                ) {
                    if let Some(value) = reconciliation["counts"][key].as_u64() {
                        counts.insert(key.into(), value);
                    }
                }
            }
            counts.insert(
                "time_seconds".into(),
                reconciliation["time_seconds"]
                    .as_u64()
                    .ok_or("career exact time not verified")?,
            );
            let input = Input {
                player_id: actor,
                first_season: w["first_season"]
                    .as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or("missing career start")?,
                last_season: w["season"]
                    .as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or("missing career end")?,
                season_type: w["season_type"]
                    .as_str()
                    .ok_or("missing record game type")?
                    .into(),
                as_of: w["as_of"].as_str().ok_or("missing record date")?.into(),
                group: career["numerical_evidence"]["group"]
                    .as_str()
                    .ok_or("missing record group")?
                    .into(),
                verified_counts: counts,
                source_input_sha256: career["deployment_evidence"]["source_sha256"]
                    .as_str()
                    .ok_or("missing record source hash")?
                    .into(),
            };
            out.insert(
                key.clone(),
                milestones::evaluate(&input, &milestones::registry())?,
            );
        }
        fs::write(&a[2], serde_json::to_vec(&out).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        println!("Projected literal record tracks, separate from comparative career feats.");
        Ok(())
    };
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
