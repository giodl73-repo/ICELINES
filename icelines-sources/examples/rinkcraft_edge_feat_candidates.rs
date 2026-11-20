//! Calibrate one observational feat family from a frozen exposure report.
//! This command never awards production feats or updates player cards.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::Path,
};

const CUTOFF: f64 = 90.;
const MIN_PEERS: u64 = 20;

fn classify(
    eligible: bool,
    peak: Option<(f64, u64)>,
    rate: Option<(f64, u64)>,
) -> Result<(&'static str, Option<&'static str>), String> {
    use icelines_core::view_model::rinkcraft_seasonal_records::{classify_pair, Family};
    let d = classify_pair(Family::Skating, eligible, peak, rate)?;
    let state = match d.state.as_str() {
        "assessed" => "assessed",
        "unassessed" => "unassessed",
        "insufficient_exposure" => "insufficient_exposure",
        _ => unreachable!("shared decision state"),
    };
    let candidate = match d.candidate.as_deref() {
        Some("two-gears") => Some("two-gears"),
        Some("top-gear") => Some("top-gear"),
        Some("burst-engine") => Some("burst-engine"),
        None => None,
        _ => unreachable!("shared skating family"),
    };
    Ok((state, candidate))
}

fn evaluate(input: &Value) -> Result<Value, String> {
    if input["schema"] != "rinkcraft.edge-exposure.v1" {
        return Err("unsupported exposure report".into());
    }
    let mut scenarios = Vec::new();
    let mut settings = BTreeSet::new();
    for scenario in input["scenarios"].as_array().ok_or("missing scenarios")? {
        let gp = scenario["min_games"].as_u64().ok_or("missing GP cutoff")?;
        if ![5, 10, 20].contains(&gp) || scenario["min_minutes"] != 100 || !settings.insert(gp) {
            return Err("unexpected sensitivity settings".into());
        }
        let mut rows = Vec::new();
        let mut ids = BTreeSet::new();
        let mut census: BTreeMap<(u64, String, String), usize> = BTreeMap::new();
        for row in scenario["rows"].as_array().ok_or("missing rows")? {
            let record = &row["record"];
            let id = record["player_id"].as_u64().ok_or("missing actor")?;
            let season = record["season"].as_u64().ok_or("missing season")?;
            let group = record["group"].as_str().ok_or("missing group")?;
            if !ids.insert((season, id)) || !["Forwards", "Defense"].contains(&group) {
                return Err("duplicate or non-skater row".into());
            }
            let eligible = row["eligible"].as_bool().ok_or("missing eligibility")?;
            let games = record["games"].as_u64().ok_or("missing games")?;
            let minutes = record["minutes"].as_f64().ok_or("missing minutes")?;
            if !minutes.is_finite() || minutes <= 0. || eligible != (games >= gp && minutes >= 100.)
            {
                return Err("exposure eligibility mismatch".into());
            }
            let mut comparisons = BTreeMap::new();
            for c in row["comparisons"].as_array().ok_or("missing comparisons")? {
                let key = c["metric"].as_str().ok_or("missing metric")?;
                let p = c["percentile"].as_f64().ok_or("missing percentile")?;
                let n = c["peers"].as_u64().ok_or("missing peer count")?;
                if comparisons.insert(key, (p, n)).is_some() {
                    return Err("duplicate metric".into());
                }
            }
            if !eligible && !comparisons.is_empty() {
                return Err("ineligible actor has ranks".into());
            }
            let (state, candidate) = classify(
                eligible,
                comparisons.get("peak_mph").copied(),
                comparisons.get("bursts_per_60").copied(),
            )?;
            *census
                .entry((season, group.into(), candidate.unwrap_or(state).into()))
                .or_default() += 1;
            rows.push(json!({"player_id":id,"season":season,"group":group,"name":record["name"],"state":state,"candidate":candidate,"measurements":row["comparisons"],"edge_sha256":record["edge_sha256"]}));
        }
        let census:Vec<_>=census.into_iter().map(|((season,group,state),count)|json!({"season":season,"group":group,"outcome":state,"count":count})).collect();
        scenarios.push(json!({"min_games":gp,"min_minutes":100,"rows":rows,"census":census}));
    }
    if settings != BTreeSet::from([5, 10, 20]) {
        return Err("incomplete sensitivity scenarios".into());
    }
    let rules = json!({
        "status":"research_candidates_not_awarded","family":"recorded-skating-speed","placements":["distinctive_features"],
        "cutoff_percentile":CUTOFF,"minimum_measured_peers":MIN_PEERS,
        "reference":"Same-season, same-position IceLines exposure midranks; all strengths.",
        "candidates":[
            {"id":"top-gear","name":"Top Gear","formula":"Peak-speed percentile >= 90 and burst-rate percentile < 90.","meaning":"Recorded a top-decile season maximum in this reference."},
            {"id":"burst-engine","name":"Burst Engine","formula":"Burst-rate percentile >= 90 and peak-speed percentile < 90.","meaning":"Recorded top-decile 20-plus-mph bursts per 60 official TOI minutes."},
            {"id":"two-gears","name":"Two Gears","formula":"Peak-speed percentile >= 90 and burst-rate percentile >= 90.","meaning":"Both recorded measures reached the top decile; replaces the two single candidates."}
        ],
        "unknown_policy":"Both metrics and their minimum peer counts are required to classify the family; missing or ineligible is never failure.",
        "interpretation":"One family, at most one candidate per actor/window. Two correlated observations are not independent evidence. No acceleration, endurance, pursuit, puck-control or overall Skating claim. No negative feat for not qualifying.",
        "threshold_status":"Authored hypotheses, not externally validated cutoffs. No production registry changes."
    });
    Ok(
        json!({"schema":"rinkcraft.edge-feat-candidates.v1","rules_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&rules).unwrap())),"rules":rules,"scenarios":scenarios}),
    )
}

fn run(input: &Path, expected_hash: &str, output: &Path) -> Result<(), String> {
    if output.exists() {
        return Err("new output file required".into());
    }
    let bytes = fs::read(input).map_err(|e| e.to_string())?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if hash != expected_hash {
        return Err("exposure report checksum mismatch".into());
    }
    let source: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let mut result = evaluate(&source)?;
    result["exposure_report_sha256"] = json!(hash);
    result["catalog_sha256"] = source["catalog_sha256"].clone();
    fs::write(
        output,
        serde_json::to_vec(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!("Three exposure settings evaluated; no production feat awards.");
    Ok(())
}
fn main() {
    let a: Vec<_> = env::args().collect();
    if a.len() != 4 {
        eprintln!("usage: rinkcraft_edge_feat_candidates REPORT_JSON SHA256 NEW_OUTPUT_JSON");
        std::process::exit(2);
    }
    if let Err(e) = run(Path::new(&a[1]), &a[2], Path::new(&a[3])) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_threshold_and_exclusive_combination() {
        assert_eq!(
            classify(true, Some((90., 20)), Some((90., 20))).unwrap(),
            ("assessed", Some("two-gears"))
        );
        assert_eq!(
            classify(true, Some((90., 20)), Some((89.99, 20)))
                .unwrap()
                .1,
            Some("top-gear")
        );
        assert_eq!(
            classify(true, Some((89.99, 20)), Some((90., 20)))
                .unwrap()
                .1,
            Some("burst-engine")
        );
        assert_eq!(
            classify(true, Some((89.99, 20)), Some((89.99, 20)))
                .unwrap()
                .1,
            None
        );
    }
    #[test]
    fn unknown_exposure_and_reference_limits_are_not_failures() {
        assert_eq!(
            classify(false, None, None).unwrap().0,
            "insufficient_exposure"
        );
        assert_eq!(
            classify(true, Some((99., 20)), None).unwrap().0,
            "unassessed"
        );
        assert_eq!(
            classify(true, Some((99., 19)), Some((99., 20))).unwrap().0,
            "unassessed"
        );
        assert!(classify(true, Some((f64::NAN, 20)), Some((99., 20))).is_err());
        assert!(classify(true, Some((101., 20)), Some((99., 20))).is_err());
        assert!(classify(true, Some((90., 0)), Some((99., 20))).is_err());
    }
}
