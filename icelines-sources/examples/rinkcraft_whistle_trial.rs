//! Versioned positive-net trial on a sealed retrospective result; no live promotion.
use icelines_core::view_model::rinkcraft_whistle::{positive_gate, record};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs};
fn count(v: &Value, k: &str) -> Result<u32, String> {
    v[k].as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| format!("invalid count {k}"))
}
fn q(v: &Value) -> Result<bool, String> {
    let a = v["percentiles"]
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or("two ranks required")?;
    let mut yes = true;
    for r in a {
        let p = r
            .as_f64()
            .filter(|p| p.is_finite() && (0.0..=100.0).contains(p))
            .ok_or("invalid percentile")?;
        yes &= p >= 60.;
    }
    if v["qualifies"].as_bool() != Some(yes) {
        return Err("inconsistent relative qualification".into());
    }
    Ok(yes)
}
fn perturb(d: u32, t: u32, label: &str) -> Result<(u32, u32), String> {
    match label {
        "drawn+1" => Ok((d.checked_add(1).ok_or("count overflow")?, t)),
        "drawn-1" => Ok((d.checked_sub(1).ok_or("negative drawn count")?, t)),
        "taken+1" => Ok((d, t.checked_add(1).ok_or("count overflow")?)),
        "taken-1" => Ok((d, t.checked_sub(1).ok_or("negative taken count")?)),
        _ => Err("unknown count variant".into()),
    }
}
fn run(base: &Value, base_hash: &str, protocol_hash: &str) -> Result<Value, String> {
    if base["schema"] != "rinkcraft.feat-robustness.v1"
        || base["untouched_holdout"] != false
        || base["evaluation_season"] != 20252026
        || base["development_seasons"] != json!([20232024, 20242025])
    {
        return Err("wrong retrospective baseline".into());
    }
    let rows: Vec<_> = base["results"]
        .as_array()
        .ok_or("missing results")?
        .iter()
        .filter(|r| r["candidate_id"] == "penalty-balance")
        .collect();
    if rows.len() != 2 {
        return Err("two whistle groups required".into());
    }
    let mut groups = BTreeSet::new();
    let mut results = Vec::new();
    for row in rows {
        let group = row["group"]
            .as_str()
            .filter(|g| matches!(*g, "Forwards" | "Defense"))
            .ok_or("invalid group")?;
        if !groups.insert(group) {
            return Err("duplicate group".into());
        }
        let list = row["players"].as_array().ok_or("missing players")?;
        if row["eligible"].as_u64() != Some(list.len() as u64) {
            return Err("incomplete evaluation cohort".into());
        }
        let mut players = Vec::new();
        let mut ids = BTreeSet::new();
        let (
            mut old_awards,
            mut new_awards,
            mut zero,
            mut negative,
            mut old_loss,
            mut new_loss,
            mut new_gain,
            mut newly_sensitive,
        ) = (0, 0, 0, 0, 0, 0, 0, 0);
        for p in list {
            let actor = count(p, "player_id")?;
            let gp = count(p, "games")?;
            if actor == 0 || gp < 20 || !ids.insert(actor) {
                return Err("duplicate actor or invalid exposure".into());
            }
            let d = count(&p["counts"], "drawn")?;
            let t = count(&p["counts"], "taken")?;
            let record = record(Some(d), Some(t)).expect("counts supplied");
            let old =
                q(&json!({"percentiles":p["primary_percentiles"],"qualifies":p["qualifies"]}))?;
            let gate = positive_gate(d, t);
            let trial = old && gate;
            old_awards += usize::from(old);
            new_awards += usize::from(trial);
            zero += usize::from(old && d == t);
            negative += usize::from(old && d < t);
            let mut changes = Vec::new();
            let mut labels = BTreeSet::new();
            let mut old_sensitive = false;
            let mut trial_sensitive = false;
            for v in p["count_variants"].as_array().ok_or("missing variants")? {
                let label = v["change"].as_str().ok_or("missing change label")?;
                if !labels.insert(label) {
                    return Err("duplicate change".into());
                }
                let (dd, tt) = perturb(d, t, label)?;
                let was = q(v)?;
                let now = was && positive_gate(dd, tt);
                old_sensitive |= was != old;
                trial_sensitive |= now != trial;
                let expected = [(dd as f64 - tt as f64) / gp as f64, dd as f64 / gp as f64];
                let vals = v["values"]
                    .as_array()
                    .filter(|a| a.len() == 2)
                    .ok_or("variant values missing")?;
                for (v, w) in vals.iter().zip(expected) {
                    if v.as_f64()
                        .is_none_or(|x| !x.is_finite() || (x - w).abs() > 1e-12)
                    {
                        return Err("count/value variant mismatch".into());
                    }
                }
                changes.push(json!({"change":label,"drawn":dd,"taken":tt,"relative_qualifies":was,"positive_gate":positive_gate(dd,tt),"trial_qualifies":now}));
            }
            let feasible = 2 + usize::from(d > 0) + usize::from(t > 0);
            if labels.len() != feasible {
                return Err("missing feasible count variant".into());
            }
            old_loss += usize::from(old && old_sensitive);
            new_loss += usize::from(trial && trial_sensitive);
            new_gain += usize::from(!trial && trial_sensitive);
            newly_sensitive += usize::from(trial && trial_sensitive && !old_sensitive);
            let mut refs = Vec::new();
            let mut modes = BTreeSet::new();
            for v in p["reference_variants"]
                .as_array()
                .ok_or("missing references")?
            {
                let mode = v["mode"].as_str().ok_or("missing reference mode")?;
                if ![
                    "actor_weighted",
                    "row_weighted",
                    "development_2023",
                    "development_2024",
                ]
                .contains(&mode)
                    || !modes.insert(mode)
                {
                    return Err("invalid reference variant".into());
                }
                let was = q(v)?;
                if mode == "actor_weighted" && was != old {
                    return Err("primary reference disagreement".into());
                }
                refs.push(
                    json!({"mode":mode,"relative_qualifies":was,"trial_qualifies":was&&gate}),
                );
            }
            if modes.len() != 4 {
                return Err("incomplete reference variants".into());
            }
            players.push(json!({"player_id":actor,"games":gp,"record":record,"relative_qualifies":old,"positive_gate":gate,"trial_qualifies":trial,"relative_one_event_sensitive":old_sensitive,"trial_one_event_sensitive":trial_sensitive,"count_variants":changes,"reference_variants":refs}));
        }
        if row["awards_p55_p60_p65"][1].as_u64() != Some(old_awards as u64)
            || row["winners_one_event_loss"].as_u64() != Some(old_loss as u64)
        {
            return Err("baseline summary mismatch".into());
        }
        let refs:Vec<_>=["actor_weighted","row_weighted","development_2023","development_2024"].iter().map(|mode|{let count=|key:&str|players.iter().filter(|p|p["reference_variants"].as_array().unwrap().iter().any(|v|v["mode"]==*mode&&v[key]==true)).count();json!({"mode":mode,"relative_awards":count("relative_qualifies"),"trial_awards":count("trial_qualifies")})}).collect();
        results.push(json!({"group":group,"eligible":players.len(),"relative_awards":old_awards,"positive_awards":new_awards,"excluded_zero":zero,"excluded_negative":negative,"relative_winner_one_event_loss":old_loss,"trial_winner_one_event_loss":new_loss,"trial_nonwinner_one_event_gain":new_gain,"newly_sensitive_retained_winners":newly_sensitive,"reference_variants":refs,"players":players}));
    }
    Ok(
        json!({"schema":"rinkcraft.positive-whistle-trial.v1","rule_id":"positive-whistle-balance.proposal.0.1","status":"retrospective_proposal_not_live","baseline_sha256":base_hash,"protocol_sha256":protocol_hash,"evaluation_season":20252026,"results":results,"notice":"Existing relative awards and reference populations remain unchanged. Positive net is an extra trial gate, not actual PP creation, a capability score, error probability or independent validation. Rule originated after inspecting prior results."}),
    )
}
fn main() {
    let a: Vec<_> = env::args().collect();
    let f = || -> Result<(), String> {
        if a.len() != 3 {
            return Err("Usage: TRIAL_INPUT OUTPUT".into());
        }
        let i: Value = serde_json::from_slice(&fs::read(&a[1]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if i["schema"] != "rinkcraft.positive-whistle-input.v1" {
            return Err("wrong input schema".into());
        }
        let read = |key: &str| -> Result<(Vec<u8>, String), String> {
            let b = fs::read(i[key]["path"].as_str().ok_or("missing input path")?)
                .map_err(|e| e.to_string())?;
            let h = format!("{:x}", Sha256::digest(&b));
            if i[key]["sha256"].as_str() != Some(h.as_str()) {
                return Err("input hash mismatch".into());
            }
            Ok((b, h))
        };
        let (b, bh) = read("baseline")?;
        let (_, ph) = read("protocol")?;
        let base: Value = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
        fs::write(
            &a[2],
            serde_json::to_vec(&run(&base, &bh, &ph)?).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!("Positive-net trial evaluated separately from existing awards.");
        Ok(())
    };
    if let Err(e) = f() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gate_is_strict() {
        assert!(!positive_gate(0, 0));
        assert!(!positive_gate(10, 10));
        assert!(positive_gate(11, 10));
    }
    #[test]
    fn variant_counts_are_coupled_and_bounded() {
        assert_eq!(perturb(3, 4, "drawn+1").unwrap(), (4, 4));
        assert_eq!(perturb(3, 4, "taken-1").unwrap(), (3, 3));
        assert!(perturb(0, 0, "drawn-1").is_err());
        assert!(perturb(3, 4, "random").is_err());
    }
    #[test]
    fn rank_decision_is_verified() {
        assert!(q(&json!({"percentiles":[60,70],"qualifies":true})).unwrap());
        assert!(q(&json!({"percentiles":[60,59],"qualifies":true})).is_err());
    }
}
