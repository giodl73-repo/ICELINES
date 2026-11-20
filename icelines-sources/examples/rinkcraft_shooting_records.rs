//! Cohort calibration of shooting records; never a finishing capability estimate.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, env, fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Record {
    player_id: u64,
    season: u64,
    name: String,
    group: String,
    games: u64,
    minutes: f64,
    goals: Option<u32>,
    shots: Option<u32>,
    peak_mph: Option<f64>,
    shots_per_60: Option<f64>,
    conversion_pct: Option<f64>,
    edge_all_counts_match: Option<bool>,
    edge_all_goals: Option<u64>,
    edge_all_shots: Option<u64>,
    edge_captured_at: Option<String>,
    source_profile_sha256: String,
    edge_sha256: Option<String>,
}
fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn official_count(profile: &Value, key: &str) -> Result<Option<u32>, String> {
    let rows: Vec<_> = profile["latest_available"]["metrics"]
        .as_array()
        .ok_or("missing official metrics")?
        .iter()
        .filter(|m| m["key"] == key)
        .collect();
    if rows.len() > 1 {
        return Err("duplicate official count".into());
    }
    let Some(m) = rows.first() else {
        return Ok(None);
    };
    if m["status"] != "available" {
        return Ok(None);
    }
    if m["unit"] != "Count" {
        return Err("wrong count unit".into());
    }
    let v = m["value"].as_f64().ok_or("invalid official count")?;
    if !v.is_finite() || v < 0. || v.fract() != 0. || v > f64::from(u32::MAX) {
        return Err("invalid official count".into());
    }
    Ok(Some(v as u32))
}
fn conversion(goals: Option<u32>, shots: Option<u32>) -> Option<f64> {
    match (goals, shots) {
        (Some(g), Some(s)) if s > 0 && g <= s => Some(f64::from(g) * 100. / f64::from(s)),
        _ => None,
    }
}
fn metric(r: &Record, key: &str, min_shots: u32) -> Option<f64> {
    match key {
        "peak_mph" => r.peak_mph,
        "shots_per_60" => r.shots_per_60,
        "conversion_pct" if r.shots.is_some_and(|s| s >= min_shots) => r.conversion_pct,
        _ => None,
    }
}
fn midrank(sorted: &[f64], value: f64) -> f64 {
    let lower = sorted.partition_point(|x| *x < value);
    let upper = sorted.partition_point(|x| *x <= value);
    100. * (lower as f64 + 0.5 * (upper - lower) as f64) / sorted.len() as f64
}
fn family(
    peak: Option<(f64, usize)>,
    volume: Option<(f64, usize)>,
) -> (&'static str, Option<&'static str>) {
    let (Some((p, pn)), Some((v, vn))) = (peak, volume) else {
        return ("unassessed", None);
    };
    if pn < 20 || vn < 20 {
        return ("unassessed", None);
    }
    let candidate = match (p >= 90., v >= 90.) {
        (true, true) => Some("power-volume"),
        (true, false) => Some("shot-power"),
        (false, true) => Some("shot-volume"),
        _ => None,
    };
    ("assessed", candidate)
}
fn evaluate(records: &[Record], min_shots: u32) -> Value {
    let eligible = |r: &Record| r.games >= 10 && r.minutes >= 100.;
    let keys = ["peak_mph", "shots_per_60", "conversion_pct"];
    let mut pools: BTreeMap<(u64, String, &str), Vec<f64>> = BTreeMap::new();
    for r in records.iter().filter(|r| eligible(r)) {
        for key in keys {
            if let Some(v) = metric(r, key, min_shots) {
                pools
                    .entry((r.season, r.group.clone(), key))
                    .or_default()
                    .push(v);
            }
        }
    }
    for pool in pools.values_mut() {
        pool.sort_by(f64::total_cmp);
    }
    let mut census: BTreeMap<(u64, String, String), usize> = BTreeMap::new();
    let rows: Vec<_> = records
        .iter()
        .map(|r| {
            let mut ranks = BTreeMap::new();
            let mut measurements = Vec::new();
            for key in keys {
                let value = metric(r, key, min_shots);
                let pool = pools.get(&(r.season, r.group.clone(), key));
                let peers = pool.map_or(0, Vec::len);
                let percentile = if eligible(r) {
                    value.zip(pool).map(|(v, p)| midrank(p, v))
                } else {
                    None
                };
                if let Some(p) = percentile {
                    ranks.insert(key, (p, peers));
                }
                measurements.push(
                    json!({"metric":key,"value":value,"percentile":percentile,"peers":peers}),
                );
            }
            let (state, candidate) = if eligible(r) {
                family(
                    ranks.get("peak_mph").copied(),
                    ranks.get("shots_per_60").copied(),
                )
            } else {
                ("insufficient_exposure", None)
            };
            *census
                .entry((r.season, r.group.clone(), candidate.unwrap_or(state).into()))
                .or_default() += 1;
            json!({"record":r,"state":state,"candidate":candidate,"measurements":measurements})
        })
        .collect();
    let census:Vec<_>=census.into_iter().map(|((season,group,outcome),count)|json!({"season":season,"group":group,"outcome":outcome,"count":count})).collect();
    json!({"minimum_games":10,"minimum_minutes":100,"minimum_conversion_shots":min_shots,"rows":rows,"census":census})
}
fn run(base: &Path, exposure: &Path, expected_hash: &str, out: &Path) -> Result<(), String> {
    if out.exists() {
        return Err("new output file required".into());
    }
    let exposure_bytes = fs::read(exposure).map_err(|e| e.to_string())?;
    if digest(&exposure_bytes) != expected_hash {
        return Err("exposure checksum mismatch".into());
    }
    let exposure: Value = serde_json::from_slice(&exposure_bytes).map_err(|e| e.to_string())?;
    if exposure["schema"] != "rinkcraft.edge-exposure.v1" {
        return Err("unsupported input inventory".into());
    }
    let catalog_bytes = fs::read(base.join("catalog.json")).map_err(|e| e.to_string())?;
    if exposure["catalog_sha256"] != digest(&catalog_bytes) {
        return Err("catalog checksum mismatch".into());
    }
    let catalog: Value = serde_json::from_slice(&catalog_bytes).map_err(|e| e.to_string())?;
    let mut inventory = BTreeMap::new();
    for input in exposure["inputs"].as_array().ok_or("missing inventory")? {
        if inventory
            .insert(
                input["path"].as_str().ok_or("missing path")?.to_string(),
                input["sha256"].as_str().ok_or("missing hash")?.to_string(),
            )
            .is_some()
        {
            return Err("duplicate inventory path".into());
        }
    }
    let mut records = Vec::new();
    for window in catalog["windows"].as_array().ok_or("missing windows")? {
        let season = window["season"].as_u64().ok_or("missing season")?;
        for actor in window["players"].as_array().ok_or("missing actors")? {
            let id = actor["id"].as_u64().ok_or("missing actor")?;
            let path = format!("data/{season}/{id}.json");
            if actor["url"] != path {
                return Err("unexpected profile path".into());
            }
            let bytes = fs::read(base.join(&path)).map_err(|e| e.to_string())?;
            let hash = digest(&bytes);
            if inventory.remove(&path).as_deref() != Some(&hash) {
                return Err("profile checksum mismatch".into());
            }
            let profile: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let f = &profile["numerical_evidence"];
            if f["player_id"] != id
                || f["season"] != season
                || profile["latest_available"]["season"] != season
                || f["group"] != actor["group"]
                || f["games"] != actor["games"]
                || profile["latest_available"]["games"] != actor["games"]
                || f["situation"] != "all strengths"
            {
                return Err("profile/catalog binding mismatch".into());
            }
            let group = f["group"].as_str().ok_or("missing group")?;
            if group == "Goalies" {
                continue;
            }
            if !["Forwards", "Defense"].contains(&group) {
                return Err("unknown skater group".into());
            }
            let games = f["games"].as_u64().ok_or("missing games")?;
            let minutes = f["minutes"].as_f64().ok_or("missing minutes")?;
            if games == 0 || !minutes.is_finite() || minutes <= 0. {
                return Err("invalid exposure".into());
            }
            let goals = official_count(&profile, "goals")?;
            let shots = official_count(&profile, "shots")?;
            let edge = &f["edge_evidence"];
            let mut peak = None;
            let mut all_match = None;
            let mut edge_hash = None;
            let mut edge_goals = None;
            let mut edge_shots = None;
            let mut edge_captured_at = None;
            if !edge.is_null() {
                if edge["player_id"] != id
                    || edge["season"] != season
                    || edge["games"] != games
                    || edge["group"] != group
                    || edge["season_type"] != "regular"
                    || edge["strength_scope"] != "all strengths"
                {
                    return Err("EDGE scope mismatch".into());
                }
                let measures = edge["measures"].as_array().ok_or("missing EDGE measures")?;
                let peaks: Vec<_> = measures
                    .iter()
                    .filter(|m| m["key"] == "max-shot-speed")
                    .collect();
                if peaks.len() > 1 {
                    return Err("duplicate peak shot speed".into());
                }
                if let Some(p) = peaks.first() {
                    let v = p["value"].as_f64().ok_or("invalid peak speed")?;
                    if p["unit"] != "Mph" || !v.is_finite() || v <= 0. {
                        return Err("invalid peak shot speed".into());
                    }
                    peak = Some(v);
                }
                let all: Vec<_> = edge["locations"]
                    .as_array()
                    .ok_or("missing EDGE locations")?
                    .iter()
                    .filter(|r| r["code"] == "all")
                    .collect();
                if all.len() > 1 {
                    return Err("duplicate all-location row".into());
                }
                if let Some(a) = all.first() {
                    edge_goals = a["goals"].as_u64();
                    edge_shots = a["shots"].as_u64();
                }
                edge_captured_at = Some(
                    edge["captured_at"]
                        .as_str()
                        .ok_or("missing capture date")?
                        .into(),
                );
                if let (Some(a), Some(g), Some(s)) = (all.first(), goals, shots) {
                    if let (Some(eg), Some(es)) = (a["goals"].as_u64(), a["shots"].as_u64()) {
                        all_match = Some(eg == u64::from(g) && es == u64::from(s));
                    }
                }
                edge_hash = Some(
                    edge["source"]["sha256"]
                        .as_str()
                        .ok_or("missing EDGE hash")?
                        .to_string(),
                );
            }
            records.push(Record {
                player_id: id,
                season,
                name: f["name"].as_str().ok_or("missing name")?.into(),
                group: group.into(),
                games,
                minutes,
                goals,
                shots,
                peak_mph: peak,
                shots_per_60: shots.map(|s| f64::from(s) * 60. / minutes),
                conversion_pct: conversion(goals, shots),
                edge_all_counts_match: all_match,
                edge_all_goals: edge_goals,
                edge_all_shots: edge_shots,
                edge_captured_at,
                source_profile_sha256: hash,
                edge_sha256: edge_hash,
            });
        }
    }
    if !inventory.is_empty() {
        return Err("unused input inventory".into());
    }
    let scenarios: Vec<_> = [10, 25, 50].map(|n| evaluate(&records, n)).into();
    let rules = json!({"status":"research_candidates_not_awarded","family":"recorded-shot-power-volume","cutoff_percentile":90,"minimum_games":10,"minimum_minutes":100,"minimum_measured_peers":20,"candidates":[{"id":"shot-power","name":"Shot Power","formula":"Peak shot-speed P >= 90; shots-on-goal/60 P < 90."},{"id":"shot-volume","name":"Shot Volume","formula":"Shots-on-goal/60 P >= 90; peak shot-speed P < 90."},{"id":"power-volume","name":"Power & Volume","formula":"Both P >= 90; replaces the two individual candidates."}],"conversion_role":"Audit only. Conversion rank never awards this family or a finishing feat."});
    let result = json!({"schema":"rinkcraft.shooting-records.v1","rules_sha256":digest(&serde_json::to_vec(&rules).unwrap()),"rules":rules,"catalog_sha256":digest(&catalog_bytes),"exposure_report_sha256":expected_hash,"inputs":exposure["inputs"],"scenarios":scenarios,"notes":["All-strength recorded activity, not capability estimates, awards, class inference or simulation parameters.","Official goals/shots and EDGE maximums retain separate lineages. EDGE all-location totals are cross-checked without rewriting either source.","Shots are shots on goal, not all attempts. Official TOI is not equivalent shot opportunity time. Peak velocity is a sample maximum.","Conversion = official goals * 100 / official SOG, conditional on nonzero SOG and goals <= SOG. Zero goals remain zero; impossible or missing ratios stay unknown.","The 10/25/50-SOG eligibility settings are sensitivity hypotheses, not reliability validation. No xG, location adjustment, shot type, empty-net exclusion or technique adjustment is supplied.","P90 and the minimum peer count are authored research settings. Missing peak velocity makes the exclusive power/volume family unassessed, even when volume is observed.","No existing finishing model was replaced. That model requires verified xG/history/calibration inputs; this report supplies none of those."]});
    fs::write(out, serde_json::to_vec(&result).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    println!(
        "{} skater records; three conversion-exposure scenarios; no awards.",
        records.len()
    );
    Ok(())
}
fn main() {
    let a: Vec<_> = env::args().collect();
    if a.len() != 5 {
        eprintln!(
            "usage: rinkcraft_shooting_records BASE_DIRECTORY EXPOSURE_JSON SHA256 NEW_OUTPUT_JSON"
        );
        std::process::exit(2);
    }
    if let Err(e) = run(Path::new(&a[1]), Path::new(&a[2]), &a[3], Path::new(&a[4])) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conversion_distinguishes_zero_missing_and_invalid() {
        assert_eq!(conversion(Some(0), Some(10)), Some(0.));
        assert_eq!(conversion(Some(5), Some(20)), Some(25.));
        assert_eq!(conversion(Some(0), Some(0)), None);
        assert_eq!(conversion(Some(2), Some(1)), None);
        assert_eq!(conversion(None, Some(20)), None);
    }
    #[test]
    fn tie_midrank_and_exclusive_boundaries() {
        assert_eq!(midrank(&[1., 2., 2., 4.], 2.), 50.);
        assert_eq!(
            family(Some((90., 20)), Some((90., 20))).1,
            Some("power-volume")
        );
        assert_eq!(
            family(Some((90., 20)), Some((89.9, 20))).1,
            Some("shot-power")
        );
        assert_eq!(
            family(Some((89.9, 20)), Some((90., 20))).1,
            Some("shot-volume")
        );
        assert_eq!(family(None, Some((99., 400))).0, "unassessed");
        assert_eq!(family(Some((99., 19)), Some((99., 400))).0, "unassessed");
    }
    #[test]
    fn count_contract_rejects_fractional_or_wrong_unit() {
        let p = json!({"latest_available":{"metrics":[{"key":"shots","status":"available","unit":"Count","value":0}]}});
        assert_eq!(official_count(&p, "shots").unwrap(), Some(0));
        assert_eq!(official_count(&p, "goals").unwrap(), None);
        let mut bad = p.clone();
        bad["latest_available"]["metrics"][0]["value"] = json!(1.5);
        assert!(official_count(&bad, "shots").is_err());
        bad["latest_available"]["metrics"][0]["value"] = json!(1);
        bad["latest_available"]["metrics"][0]["unit"] = json!("PerGame");
        assert!(official_count(&bad, "shots").is_err());
    }
}
