//! Bind frozen descriptive observations to exact seasonal profile fixtures.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{env, fs};

fn read(spec: &Value) -> Result<(Value, String, Vec<u8>), String> {
    let bytes =
        fs::read(spec["path"].as_str().ok_or("input path missing")?).map_err(|e| e.to_string())?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if spec["sha256"].as_str() != Some(&hash) {
        return Err("input checksum differs".into());
    }
    Ok((
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?,
        hash,
        bytes,
    ))
}
fn attach(mut fixtures: Value, report: &Value, report_hash: &str) -> Result<Value, String> {
    if report["schema"] != "rinkcraft.feat-salary-contrast.v1" {
        return Err("unsupported observation report".into());
    }
    let rows = report["rows"]
        .as_array()
        .ok_or("sample missing")?
        .iter()
        .chain(report["benchmark"].as_array().ok_or("benchmark missing")?)
        .collect::<Vec<_>>();
    let mut ids = std::collections::BTreeSet::new();
    for row in &rows {
        let id = row["player_id"].as_u64().ok_or("actor missing")?;
        if !ids.insert(id) {
            return Err("duplicate observation actor".into());
        }
    }
    for p in fixtures
        .as_object_mut()
        .ok_or("fixture map required")?
        .values_mut()
    {
        let id = p["request"]["player_id"]
            .as_u64()
            .ok_or("fixture actor missing")?;
        let Some(row) = rows.iter().find(|r| r["player_id"].as_u64() == Some(id)) else {
            continue;
        };
        let a = &p["achievements"];
        if p["profile_window"]["kind"] != "season"
            || p["profile_window"]["player_id"].as_u64() != Some(id)
            || p["profile_window"]["season"] != report["season"]
            || p["profile_window"]["season_type"] != "regular"
            || p["request"]["first_season"] != report["season"]
            || p["request"]["last_season"] != report["season"]
            || p["request"]["season_type"] != "regular"
            || a["player_id"].as_u64() != Some(id)
            || a["season"] != report["season"]
            || a["season_type"] != "regular"
            || a["group"] != row["group"]
            || a["games"] != row["games"]
            || a["source_sha256"] != report["source_sha256"]["reports"]
            || p["player_name"] != row["name"]
        {
            return Err("observation/profile actor, window or lineage mismatch".into());
        }
        let lows = row["low_callouts"]
            .as_array()
            .ok_or("lower-tail observations missing")?;
        let positive = row["positive_total"]
            .as_u64()
            .ok_or("positive count missing")?;
        let caution = row["caution_count"]
            .as_u64()
            .ok_or("caution count missing")?;
        if row["observation_count"].as_u64() != Some(positive + caution + lows.len() as u64) {
            return Err("observation count mismatch".into());
        }
        p["achievements"]["profile_observations"] = json!({
            "schema":"rinkcraft.profile-observations.v1","policy":"lower-tail-observations.v0.1-exploratory",
            "player_id":id,"season":report["season"],"season_type":"regular","group":row["group"],"games":row["games"],
            "source_sha256":report["source_sha256"]["reports"],"analysis_sha256":report_hash,
            "positive_count":positive,"caution_count":caution,"low_callouts":lows,
            "coverage_gaps":["Entry denial","Controlled exits & retrievals","Gap & lane protection","Net-front coverage","Rotations & cover","Passing under pressure","PK prevention"],
            "note":"Overlapping observations, not independent abilities. P20 is an exploratory display cutoff. No automatic gameplay penalties."
        });
    }
    Ok(fixtures)
}
// Keep the original JSON number tokens: deserializing and rewriting every float
// can introduce tiny rounding differences in otherwise untouched profile scopes.
fn project(bytes: &[u8], report: &Value, hash: &str) -> Result<Vec<u8>, String> {
    use serde_json::value::{to_raw_value, RawValue};
    use std::collections::BTreeMap;
    type RawMap = BTreeMap<String, Box<RawValue>>;
    let original: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let annotated = attach(original, report, hash)?;
    let mut root: RawMap = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    for (key, raw_profile) in &mut root {
        let observation = &annotated[key]["achievements"]["profile_observations"];
        if observation.is_null() {
            continue;
        }
        let mut profile: RawMap =
            serde_json::from_str(raw_profile.get()).map_err(|e| e.to_string())?;
        let mut achievements: RawMap =
            serde_json::from_str(profile["achievements"].get()).map_err(|e| e.to_string())?;
        achievements.insert(
            "profile_observations".into(),
            to_raw_value(observation).map_err(|e| e.to_string())?,
        );
        profile.insert(
            "achievements".into(),
            to_raw_value(&achievements).map_err(|e| e.to_string())?,
        );
        *raw_profile = to_raw_value(&profile).map_err(|e| e.to_string())?;
    }
    serde_json::to_vec(&root).map_err(|e| e.to_string())
}
fn main() {
    let result = || -> Result<(), String> {
        let args: Vec<_> = env::args().collect();
        if args.len() != 3 {
            return Err("Usage: INPUT_MANIFEST OUTPUT_FIXTURES".into());
        }
        let manifest: Value =
            serde_json::from_slice(&fs::read(&args[1]).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let (_, _, fixtures) = read(&manifest["fixtures"])?;
        let (report, hash, _) = read(&manifest["observations"])?;
        let output = project(&fixtures, &report, &hash)?;
        fs::write(&args[2], output).map_err(|e| e.to_string())?;
        println!("Bound seasonal profile observations; other scopes unchanged.");
        Ok(())
    }();
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> (Value, Value) {
        (
            json!({"x":{"request":{"player_id":1,"first_season":20242025,"last_season":20242025,"season_type":"regular"},"player_name":"Example","profile_window":{"kind":"season","player_id":1,"season":20242025,"season_type":"regular"},"achievements":{"player_id":1,"season":20242025,"season_type":"regular","group":"Defense","games":60,"source_sha256":"source"},"scope_variants":{"current":{"untouched":true},"career":{"untouched":true}}}}),
            json!({"schema":"rinkcraft.feat-salary-contrast.v1","season":20242025,"source_sha256":{"reports":"source"},"rows":[{"player_id":1,"name":"Example","group":"Defense","games":60,"positive_total":4,"caution_count":0,"low_callouts":[],"observation_count":4}],"benchmark":[]}),
        )
    }
    #[test]
    fn attachment_preserves_other_scopes_and_existing_fields() {
        let (p, r) = inputs();
        let out = attach(p.clone(), &r, "hash").unwrap();
        assert_eq!(out["x"]["scope_variants"], p["x"]["scope_variants"]);
        assert_eq!(
            out["x"]["achievements"]["profile_observations"]["positive_count"],
            4
        );
    }
    #[test]
    fn keeps_original_number_tokens_in_other_scopes() {
        let (mut p, r) = inputs();
        p["x"]["scope_variants"]["current"]["value"] = json!("NUMBER_TOKEN");
        let bytes = serde_json::to_string(&p)
            .unwrap()
            .replace("\"NUMBER_TOKEN\"", "93.71134020618557");
        let result = String::from_utf8(project(bytes.as_bytes(), &r, "h").unwrap()).unwrap();
        assert!(result.contains("93.71134020618557"));
    }
    #[test]
    fn rejects_wrong_scope_lineage_and_duplicate_actor() {
        let (p, r) = inputs();
        for field in ["season", "games", "source_sha256", "player_id", "group"] {
            let mut bad = p.clone();
            bad["x"]["achievements"][field] = json!("wrong");
            assert!(attach(bad, &r, "h").is_err());
        }
        let mut bad = p.clone();
        bad["x"]["profile_window"]["kind"] = json!("career");
        assert!(attach(bad, &r, "h").is_err());
        let mut bad = r.clone();
        bad["benchmark"] = json!([r["rows"][0].clone()]);
        assert!(attach(p, &bad, "h").is_err());
    }
}
