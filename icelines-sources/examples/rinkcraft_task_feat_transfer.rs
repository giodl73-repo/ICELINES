//! Same-season-reference descriptive task transfer, with unavailable winners audited.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
};
const YEARS: [u64; 3] = [20232024, 20242025, 20252026];
#[derive(Clone)]
struct Measurement {
    gp: u64,
    opps: u64,
    valid: bool,
}
struct Case {
    id: String,
    group: String,
    gp: u64,
    opps: u64,
    reference: bool,
    measurements: BTreeMap<u64, Measurement>,
    players: BTreeMap<u64, bool>,
    winners: BTreeSet<u64>,
    before_validation: u64,
}
struct Window {
    season: u64,
    cases: BTreeMap<(String, String), Case>,
    population: BTreeMap<u64, (String, u64)>,
}
fn n(v: &Value, k: &str) -> Result<u64, String> {
    v[k].as_u64().ok_or_else(|| format!("missing count {k}"))
}
fn ratio(a: usize, b: usize) -> Option<f64> {
    if b == 0 {
        None
    } else {
        Some(a as f64 / b as f64)
    }
}
fn read(spec: &Value) -> Result<(Value, String), String> {
    let b = fs::read(spec["path"].as_str().ok_or("missing path")?).map_err(|e| e.to_string())?;
    let h = format!("{:x}", Sha256::digest(&b));
    if spec["sha256"].as_str() != Some(h.as_str()) {
        return Err("input checksum mismatch".into());
    }
    Ok((serde_json::from_slice(&b).map_err(|e| e.to_string())?, h))
}
fn load(entry: &Value, year: u64) -> Result<(Window, Value, String), String> {
    if entry["season"] != year {
        return Err("wrong temporal order".into());
    }
    let (screen, sh) = read(&entry["screen"])?;
    let (mut rules, rh) = read(&entry["rules"])?;
    if screen["schema"] != "rinkcraft.task-batch-two-screen.v1"
        || screen["season"] != year
        || rules["season"] != year
        || screen["input_sha256"]["rules"].as_str() != Some(rh.as_str())
    {
        return Err("screen/rule binding mismatch".into());
    }
    rules
        .as_object_mut()
        .ok_or("invalid rules")?
        .remove("season");
    let normalized_hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&rules).map_err(|e| e.to_string())?)
    );
    let mut cases = BTreeMap::new();
    let mut population = BTreeMap::new();
    for row in screen["results"]
        .as_array()
        .filter(|a| a.len() == 7)
        .ok_or("seven case groups required")?
    {
        let id = row["id"].as_str().ok_or("missing rule id")?;
        let group = row["group"].as_str().ok_or("missing group")?;
        let rule = rules["rules"]
            .as_array()
            .ok_or("missing rules")?
            .iter()
            .find(|r| r["id"] == id)
            .ok_or("unknown rule")?;
        if !matches!(
            (rule["branch"].as_str(), group),
            (Some("skater"), "Forwards" | "Defense") | (Some("goalie"), "Goalies")
        ) {
            return Err("rule/group mismatch".into());
        }
        let (gp, opps) = (n(rule, "minimum_gp")?, n(rule, "minimum_opportunities")?);
        let primary = row["scenarios"]
            .as_array()
            .ok_or("missing scenarios")?
            .iter()
            .filter(|s| s["exposure_multiplier"] == 1.)
            .collect::<Vec<_>>();
        if primary.len() != 1 {
            return Err("primary scenario missing/duplicate".into());
        }
        let primary = primary[0];
        if primary["minimum_gp"] != gp
            || primary["minimum_opportunities"].as_f64() != Some(opps as f64)
        {
            return Err("primary exposure mismatch".into());
        }
        let reference = primary["minimum_peers_met"]
            .as_bool()
            .ok_or("missing reference status")?;
        let mut measurements = BTreeMap::new();
        for m in row["measurements"]
            .as_array()
            .ok_or("missing measurements")?
        {
            let actor = n(m, "player_id")?;
            let mgp = n(m, "games")?;
            let valid = m["issues"]
                .as_array()
                .ok_or("missing source quality")?
                .is_empty();
            if actor == 0
                || measurements
                    .insert(
                        actor,
                        Measurement {
                            gp: mgp,
                            opps: n(m, "opportunities")?,
                            valid,
                        },
                    )
                    .is_some()
            {
                return Err("duplicate measurement actor".into());
            }
            if let Some(old) = population.insert(actor, (group.to_owned(), mgp)) {
                if old != (group.to_owned(), mgp) {
                    return Err("actor GP/group inconsistent across tasks".into());
                }
            }
        }
        if row["observed"].as_u64() != Some(measurements.len() as u64) {
            return Err("observed cohort incomplete".into());
        }
        let expected: BTreeSet<_> = measurements
            .iter()
            .filter(|(_, m)| m.gp >= gp && m.opps >= opps && m.valid)
            .map(|(a, _)| *a)
            .collect();
        let exposure = measurements
            .values()
            .filter(|m| m.gp >= gp && m.opps >= opps)
            .count();
        if primary["eligible_before_validation"].as_u64() != Some(exposure as u64)
            || primary["measured"].as_u64() != Some(expected.len() as u64)
            || reference != (expected.len() >= 20)
        {
            return Err("primary cohort/reference counts disagree".into());
        }
        let mut players = BTreeMap::new();
        let mut winners = BTreeSet::new();
        for p in primary["players"]
            .as_array()
            .ok_or("missing primary players")?
        {
            let actor = n(p, "player_id")?;
            let m = measurements
                .get(&actor)
                .ok_or("player absent from measurement")?;
            if n(p, "games")? != m.gp || n(p, "opportunities")? != m.opps {
                return Err("primary count mismatch".into());
            }
            let q = if reference {
                let ranks = p["percentiles"]
                    .as_array()
                    .filter(|a| a.len() == 2)
                    .ok_or("missing ranks")?;
                let mut q = true;
                for r in ranks {
                    let x = r
                        .as_f64()
                        .filter(|x| x.is_finite() && (0.0..=100.0).contains(x))
                        .ok_or("invalid percentile")?;
                    q &= x >= 60.;
                }
                if p["qualifies_p55_p60_p65"][1].as_bool() != Some(q) {
                    return Err("award does not match ranks".into());
                }
                q
            } else {
                if !p["percentiles"].is_null() || !p["qualifies_p55_p60_p65"][1].is_null() {
                    return Err("unranked case assigned award".into());
                }
                false
            };
            if players.insert(actor, q).is_some() {
                return Err("duplicate primary actor".into());
            }
            if q {
                winners.insert(actor);
            }
        }
        if players.keys().copied().collect::<BTreeSet<_>>() != expected {
            return Err("primary actor set differs".into());
        }
        if reference && primary["awards_p55_p60_p65"][1].as_u64() != Some(winners.len() as u64) {
            return Err("summary award count differs".into());
        }
        let key = (id.to_owned(), group.to_owned());
        if cases
            .insert(
                key,
                Case {
                    id: id.into(),
                    group: group.into(),
                    gp,
                    opps,
                    reference,
                    measurements,
                    players,
                    winners,
                    before_validation: exposure as u64,
                },
            )
            .is_some()
        {
            return Err("duplicate case".into());
        }
    }
    Ok((
        Window {
            season: year,
            cases,
            population,
        },
        json!({"season":year,"screen_sha256":sh,"rules_sha256":rh,"normalized_rule_sha256":normalized_hash}),
        normalized_hash,
    ))
}
fn status(
    actor: u64,
    group: &str,
    pop: &BTreeMap<u64, (String, u64)>,
    case: &Case,
) -> Result<&'static str, String> {
    let Some((actual, _)) = pop.get(&actor) else {
        return Ok("absent");
    };
    if actual != group {
        return Ok("group_changed");
    }
    let m = case
        .measurements
        .get(&actor)
        .ok_or("observed actor missing case measurement")?;
    if m.gp < case.gp || m.opps < case.opps {
        return Ok("insufficient_exposure");
    }
    if !m.valid {
        return Ok("source_conflict");
    }
    if !case.reference {
        return Ok("insufficient_reference");
    }
    if !case.players.contains_key(&actor) {
        return Err("comparable actor missing primary row".into());
    }
    Ok("comparable")
}
fn run(input: &Value) -> Result<Value, String> {
    if input["schema"] != "rinkcraft.task-transfer-input.v1" || input["untouched_holdout"] != false
    {
        return Err("invalid protocol or holdout claim".into());
    }
    let p = fs::read(
        input["protocol"]["path"]
            .as_str()
            .ok_or("missing protocol path")?,
    )
    .map_err(|e| e.to_string())?;
    let protocol_hash = format!("{:x}", Sha256::digest(&p));
    if input["protocol"]["sha256"].as_str() != Some(protocol_hash.as_str()) {
        return Err("protocol checksum mismatch".into());
    }
    let entries = input["windows"]
        .as_array()
        .filter(|a| a.len() == 3)
        .ok_or("three explicit windows required")?;
    let mut windows = Vec::new();
    let mut sources = Vec::new();
    let mut hashes = BTreeSet::new();
    for (e, y) in entries.iter().zip(YEARS) {
        let (w, s, h) = load(e, y)?;
        windows.push(w);
        sources.push(s);
        hashes.insert(h);
    }
    if hashes.len() != 1 {
        return Err("task definitions differ across seasons".into());
    }
    let mut annual = Vec::new();
    let mut actors = BTreeSet::new();
    let mut player_seasons = 0;
    for w in &windows {
        actors.extend(w.population.keys().copied());
        player_seasons += w.population.len();
        for c in w.cases.values() {
            annual.push(json!({"season":w.season,"id":c.id,"group":c.group,"observed":c.measurements.len(),"exposure_qualified":c.before_validation,"measured":c.players.len(),"source_conflicts":c.before_validation-c.players.len() as u64,"reference_met":c.reference,"awards":if c.reference{Some(c.winners.len())}else{None}}));
        }
    }
    let mut support: BTreeMap<String, (BTreeSet<u64>, usize)> = BTreeMap::new();
    for w in &windows {
        for c in w.cases.values() {
            let entry = support.entry(c.id.clone()).or_default();
            entry.0.extend(c.players.keys().copied());
            entry.1 += c.players.len();
        }
    }
    let task_support:Vec<_>=support.into_iter().map(|(id,(actors,rows))|json!({"id":id,"unique_measured_actors":actors.len(),"measured_player_seasons":rows})).collect();
    let mut transitions = Vec::new();
    for pair in windows.windows(2) {
        let (before, after) = (&pair[0], &pair[1]);
        if before.cases.keys().ne(after.cases.keys()) {
            return Err("case coverage changes across seasons".into());
        }
        for (key, a) in &before.cases {
            let b = &after.cases[key];
            let mut common = Vec::new();
            if a.reference && b.reference {
                for actor in a.players.keys() {
                    if b.players.contains_key(actor) {
                        common.push(*actor);
                    }
                }
            }
            let common_set: BTreeSet<_> = common.iter().copied().collect();
            let prior_common = a.winners.intersection(&common_set).count();
            let next_common = b.winners.intersection(&common_set).count();
            let retained = common
                .iter()
                .filter(|id| a.winners.contains(id) && b.winners.contains(id))
                .count();
            let mut statuses = BTreeMap::from([
                ("absent", 0_usize),
                ("group_changed", 0),
                ("insufficient_exposure", 0),
                ("source_conflict", 0),
                ("insufficient_reference", 0),
                ("comparable", 0),
            ]);
            let mut audits = Vec::new();
            for actor in &a.winners {
                let st = status(*actor, &a.group, &after.population, b)?;
                *statuses.get_mut(st).ok_or("unknown status")? += 1;
                audits.push(json!({"player_id":actor,"next_status":st,"next_qualifies":if st=="comparable"{b.players.get(actor).copied()}else{None}}));
            }
            if statuses["comparable"] != prior_common {
                return Err("winner audit/common mismatch".into());
            }
            let persistence = ratio(retained, prior_common);
            let prevalence = ratio(next_common, common.len());
            let relative = match (persistence, prevalence) {
                (Some(a), Some(b)) if b > 0. => Some(a / b),
                _ => None,
            };
            transitions.push(json!({"from":before.season,"to":after.season,"id":a.id,"group":a.group,"common_comparable":common.len(),"prior_awards_total":a.winners.len(),"prior_awards_comparable":prior_common,"next_awards_common":next_common,"retained":retained,"retention_fraction":persistence,"next_award_prevalence_common":prevalence,"descriptive_retention_to_prevalence_ratio":relative,"prior_winner_statuses":statuses,"prior_winner_audit":audits,"common_players":common.iter().map(|id|json!({"player_id":id,"before":a.winners.contains(id),"after":b.winners.contains(id)})).collect::<Vec<_>>()}));
        }
    }
    Ok(
        json!({"schema":"rinkcraft.task-transfer.v1","reference_mode":"same_season_descriptive","untouched_holdout":false,"protocol_sha256":protocol_hash,"sources":sources,"unique_observed_actors":actors.len(),"player_seasons":player_seasons,"annual":annual,"task_support":task_support,"transitions":transitions,"limits":"Retrospective descriptive transfer with each season's own references, not frozen-norm prediction, independent players, calibrated probabilities, causal effects or latent ability. Conflict/survivor selection remains. No live promotion or card change."}),
    )
}
fn main() {
    let a: Vec<_> = env::args().collect();
    let f = || -> Result<(), String> {
        if a.len() != 3 {
            return Err("Usage: TASK_TRANSFER_INPUT OUTPUT".into());
        }
        let input: Value = serde_json::from_slice(&fs::read(&a[1]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        fs::write(
            &a[2],
            serde_json::to_vec(&run(&input)?).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!("Task temporal transfer completed with explicit source-conflict attrition.");
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
    fn case() -> Case {
        Case {
            id: "x".into(),
            group: "Goalies".into(),
            gp: 15,
            opps: 500,
            reference: true,
            measurements: BTreeMap::from([(
                1,
                Measurement {
                    gp: 20,
                    opps: 600,
                    valid: false,
                },
            )]),
            players: BTreeMap::new(),
            winners: BTreeSet::new(),
            before_validation: 1,
        }
    }
    #[test]
    fn conflict_is_not_nonqualification() {
        let c = case();
        let pop = BTreeMap::from([(1, ("Goalies".into(), 20))]);
        assert_eq!(status(1, "Goalies", &pop, &c).unwrap(), "source_conflict");
    }
    #[test]
    fn exposure_precedes_conflict_for_status() {
        let mut c = case();
        c.measurements.get_mut(&1).unwrap().opps = 100;
        let pop = BTreeMap::from([(1, ("Goalies".into(), 20))]);
        assert_eq!(
            status(1, "Goalies", &pop, &c).unwrap(),
            "insufficient_exposure"
        );
    }
    #[test]
    fn group_change_and_absence_are_separate() {
        let c = case();
        assert_eq!(
            status(1, "Goalies", &BTreeMap::new(), &c).unwrap(),
            "absent"
        );
        assert_eq!(
            status(
                1,
                "Forwards",
                &BTreeMap::from([(1, ("Goalies".into(), 20))]),
                &c
            )
            .unwrap(),
            "group_changed"
        );
    }
    #[test]
    fn missing_reference_is_not_a_measured_false_award() {
        let mut c = case();
        c.measurements.get_mut(&1).unwrap().valid = true;
        c.reference = false;
        let pop = BTreeMap::from([(1, ("Goalies".into(), 20))]);
        assert_eq!(
            status(1, "Goalies", &pop, &c).unwrap(),
            "insufficient_reference"
        );
        c.reference = true;
        c.players.insert(1, false);
        assert_eq!(status(1, "Goalies", &pop, &c).unwrap(), "comparable");
        assert_eq!(c.players.get(&1), Some(&false));
    }
    #[test]
    fn zero_denominators_remain_null() {
        assert_eq!(ratio(0, 0), None);
        assert_eq!(ratio(1, 4), Some(0.25));
    }
}
