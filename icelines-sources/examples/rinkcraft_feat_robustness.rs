//! Retrospective count-boundary/reference stress tests, never live feat scoring.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
};
const YEARS: [u32; 3] = [20232024, 20242025, 20252026];
const IDS: [&str; 3] = ["accuracy-ledger", "pp-efficiency", "penalty-balance"];
#[derive(Clone, Debug)]
struct Actor {
    id: u32,
    group: String,
    gp: u32,
    goals: u32,
    shots: u32,
    pp_points: u32,
    pp_seconds: u32,
    drawn: u32,
    taken: u32,
}
fn count(v: &Value, k: &str) -> Result<u32, String> {
    v[k].as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| format!("invalid integer count {k}"))
}
fn rows(v: &Value, year: u32) -> Result<BTreeMap<u32, &Value>, String> {
    let a = v["data"].as_array().ok_or("missing report rows")?;
    if v["total"].as_u64() != Some(a.len() as u64) {
        return Err("incomplete report".into());
    }
    let mut out = BTreeMap::new();
    for r in a {
        let id = count(r, "playerId")?;
        if id == 0 || count(r, "seasonId")? != year || out.insert(id, r).is_some() {
            return Err("wrong year or duplicate actor".into());
        }
    }
    Ok(out)
}
fn load(entry: &Value, year: u32) -> Result<(Vec<Actor>, Value), String> {
    if entry["season"] != year {
        return Err("wrong temporal order".into());
    }
    let read = |key: &str| -> Result<Vec<u8>, String> {
        let bytes = fs::read(entry[key]["path"].as_str().ok_or("missing input path")?)
            .map_err(|e| e.to_string())?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        if entry[key]["sha256"].as_str() != Some(hash.as_str()) {
            return Err("input hash mismatch".into());
        }
        Ok(bytes)
    };
    let bytes = read("reports")?;
    let parsed = icelines_sources::nhl::achievements::parse(&bytes)?;
    if parsed.season != year {
        return Err("mixed report season".into());
    }
    let raw: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let time: Value = serde_json::from_slice(&read("timeonice")?).map_err(|e| e.to_string())?;
    let sk = rows(&raw["skaters"], year)?;
    let pen = rows(&raw["penalties"], year)?;
    let toi = rows(&time, year)?;
    if sk.keys().ne(toi.keys()) || sk.keys().ne(pen.keys()) {
        return Err("actor cohorts differ".into());
    }
    let mut actors = Vec::new();
    let mut population: BTreeMap<String, usize> = BTreeMap::new();
    for p in parsed.players {
        *population.entry(p.group.clone()).or_default() += 1;
        if p.group == "Goalies" {
            continue;
        }
        let s = sk[&p.player_id];
        let t = toi[&p.player_id];
        let q = pen[&p.player_id];
        if count(t, "gamesPlayed")? != p.games
            || count(q, "gamesPlayed")? != p.games
            || t["positionCode"] != s["positionCode"]
            || p.games == 0
        {
            return Err("joined GP/position mismatch".into());
        }
        let total = count(t, "timeOnIce")?;
        let pp = count(t, "ppTimeOnIce")?;
        let sh = count(t, "shTimeOnIce")?;
        let ev = count(t, "evTimeOnIce")?;
        if pp as u64 + sh as u64 + ev as u64 != total as u64 {
            return Err("TOI partition mismatch".into());
        }
        let a = Actor {
            id: p.player_id,
            group: p.group,
            gp: p.games,
            goals: count(s, "goals")?,
            shots: count(s, "shots")?,
            pp_points: count(s, "ppPoints")?,
            pp_seconds: pp,
            drawn: count(q, "penaltiesDrawn")?,
            taken: count(q, "penalties")?,
        };
        if a.goals > a.shots
            || (a.pp_seconds == 0 && a.pp_points > 0)
            || q["netPenalties"].as_i64() != Some(a.drawn as i64 - a.taken as i64)
        {
            return Err("inconsistent count accounting".into());
        }
        actors.push(a);
    }
    Ok((
        actors,
        json!({"season":year,"observed":population,"source_urls":parsed.source_urls,"reports_sha256":entry["reports"]["sha256"],"timeonice_sha256":entry["timeonice"]["sha256"]}),
    ))
}
fn eligible(a: &Actor, id: &str) -> bool {
    a.gp >= 20
        && match id {
            "accuracy-ledger" => a.shots >= 100,
            "pp-efficiency" => a.pp_seconds >= 7200,
            "penalty-balance" => true,
            _ => false,
        }
}
fn values(a: &Actor, id: &str) -> [f64; 2] {
    match id {
        "accuracy-ledger" => [
            a.goals as f64 / a.shots as f64,
            a.shots as f64 / a.gp as f64,
        ],
        "pp-efficiency" => [
            a.pp_points as f64 * 3600. / a.pp_seconds as f64,
            a.pp_seconds as f64,
        ],
        "penalty-balance" => [
            (a.drawn as f64 - a.taken as f64) / a.gp as f64,
            a.drawn as f64 / a.gp as f64,
        ],
        _ => unreachable!(),
    }
}
#[derive(Clone)]
struct Ref {
    id: u32,
    season: u32,
    values: [f64; 2],
    weight: f64,
}
fn reference(dev: &[Vec<Actor>], id: &str, group: &str, mode: &str) -> Vec<Ref> {
    let mut all = Vec::new();
    for (i, year) in dev.iter().enumerate() {
        if mode == "development_2023" && i != 0 || mode == "development_2024" && i != 1 {
            continue;
        }
        for a in year.iter().filter(|a| a.group == group && eligible(a, id)) {
            all.push(Ref {
                id: a.id,
                season: YEARS[i],
                values: values(a, id),
                weight: 1.,
            });
        }
    }
    if mode == "actor_weighted" {
        let mut n: BTreeMap<u32, usize> = BTreeMap::new();
        for a in &all {
            *n.entry(a.id).or_default() += 1;
        }
        for a in &mut all {
            a.weight = 1. / n[&a.id] as f64;
        }
    }
    all
}
fn ranks(v: [f64; 2], reference: &[Ref]) -> [f64; 2] {
    let total: f64 = reference.iter().map(|a| a.weight).sum();
    std::array::from_fn(|i| {
        100. * reference
            .iter()
            .map(|a| {
                if a.values[i] < v[i] {
                    a.weight
                } else if a.values[i] == v[i] {
                    0.5 * a.weight
                } else {
                    0.
                }
            })
            .sum::<f64>()
            / total
    })
}
fn award(r: [f64; 2], n: usize, threshold: f64) -> bool {
    n >= 20 && r.iter().all(|p| *p >= threshold)
}
fn changes(a: &Actor, id: &str) -> Vec<(String, Actor)> {
    let mut out = Vec::new();
    let fields: &[&str] = match id {
        "accuracy-ledger" => &["goals"],
        "pp-efficiency" => &["pp_points"],
        "penalty-balance" => &["drawn", "taken"],
        _ => unreachable!(),
    };
    for field in fields {
        for delta in [-1, 1] {
            let mut b = a.clone();
            let slot = match *field {
                "goals" => &mut b.goals,
                "pp_points" => &mut b.pp_points,
                "drawn" => &mut b.drawn,
                "taken" => &mut b.taken,
                _ => unreachable!(),
            };
            if let Some(value) = slot.checked_add_signed(delta) {
                *slot = value;
                if b.goals <= b.shots {
                    out.push((format!("{field}{delta:+}"), b));
                }
            }
        }
    }
    out
}
fn run(input: &Value) -> Result<Value, String> {
    if input["schema"] != "rinkcraft.feat-robustness-input.v1"
        || input["untouched_holdout"] != false
        || !input["seed"].is_null()
        || !input["trial_count"].is_null()
    {
        return Err("wrong protocol or stochastic/holdout claim".into());
    }
    let protocol = fs::read(
        input["protocol"]["path"]
            .as_str()
            .ok_or("missing protocol")?,
    )
    .map_err(|e| e.to_string())?;
    let protocol_sha = format!("{:x}", Sha256::digest(&protocol));
    if input["protocol"]["sha256"].as_str() != Some(protocol_sha.as_str()) {
        return Err("protocol hash mismatch".into());
    }
    let windows = input["windows"]
        .as_array()
        .filter(|a| a.len() == 3)
        .ok_or("three explicit windows required")?;
    let mut all = Vec::new();
    let mut populations = Vec::new();
    for (entry, year) in windows.iter().zip(YEARS) {
        let (a, p) = load(entry, year)?;
        all.push(a);
        populations.push(p);
    }
    let dev = &all[..2];
    let eval = &all[2];
    let mut results = Vec::new();
    for id in IDS {
        for group in ["Forwards", "Defense"] {
            let modes = [
                "actor_weighted",
                "row_weighted",
                "development_2023",
                "development_2024",
            ];
            let references: Vec<_> = modes.iter().map(|m| reference(dev, id, group, m)).collect();
            let supports: Vec<_> = references
                .iter()
                .map(|r| r.iter().map(|a| a.id).collect::<BTreeSet<_>>().len())
                .collect();
            if supports.iter().any(|n| *n < 20) {
                return Err("insufficient reference support".into());
            }
            let mut players = Vec::new();
            let mut awards = [0_usize; 3];
            let mut lost = 0;
            let mut gained = 0;
            let mut disagreements = [0_usize; 4];
            let mut alternative_awards = [0_usize; 4];
            for a in eval.iter().filter(|a| a.group == group && eligible(a, id)) {
                let v = values(a, id);
                let pr = ranks(v, &references[0]);
                let primary = award(pr, supports[0], 60.);
                let thresholds: Vec<_> = [55., 60., 65.]
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        let q = award(pr, supports[0], *t);
                        awards[i] += usize::from(q);
                        json!({"threshold":t,"qualifies":q})
                    })
                    .collect();
                let variants: Vec<_> = changes(a, id)
                    .iter()
                    .map(|(name, b)| {
                        let vv = values(b, id);
                        let rr = ranks(vv, &references[0]);
                        let q = award(rr, supports[0], 60.);
                        json!({"change":name,"values":vv,"percentiles":rr,"qualifies":q})
                    })
                    .collect();
                let fragile = variants
                    .iter()
                    .any(|r| r["qualifies"].as_bool() != Some(primary));
                lost += usize::from(primary && fragile);
                gained += usize::from(!primary && fragile);
                let alternatives: Vec<_> = references
                    .iter()
                    .enumerate()
                    .map(|(i, r)| {
                        let rr = ranks(v, r);
                        let q = award(rr, supports[i], 60.);
                        alternative_awards[i] += usize::from(q);
                        disagreements[i] += usize::from(q != primary);
                        json!({"mode":modes[i],"percentiles":rr,"qualifies":q})
                    })
                    .collect();
                players.push(json!({"player_id":a.id,"games":a.gp,"counts":{"goals":a.goals,"shots":a.shots,"pp_points":a.pp_points,"pp_seconds":a.pp_seconds,"drawn":a.drawn,"taken":a.taken},"values":v,"primary_percentiles":pr,"qualifies":primary,"one_event_sensitive":fragile,"thresholds":thresholds,"count_variants":variants,"reference_variants":alternatives}));
            }
            // Supplementary semantic audit added after inspecting primary results;
            // it never changes qualification or the frozen sensitivity comparisons.
            let net_signs = if id == "penalty-balance" {
                let mut signs =
                    BTreeMap::from([("negative", 0_usize), ("zero", 0), ("positive", 0)]);
                for a in eval.iter().filter(|a| a.group == group && eligible(a, id)) {
                    if award(ranks(values(a, id), &references[0]), supports[0], 60.) {
                        let key = match a.drawn.cmp(&a.taken) {
                            std::cmp::Ordering::Less => "negative",
                            std::cmp::Ordering::Equal => "zero",
                            std::cmp::Ordering::Greater => "positive",
                        };
                        *signs.get_mut(key).expect("defined sign") += 1;
                    }
                }
                Some(signs)
            } else {
                None
            };
            let n = players.len();
            let reference_rows:Vec<_>=references.iter().enumerate().map(|(i,r)|json!({"mode":modes[i],"unique_actors":supports[i],"player_seasons":r.len(),"observations":r.iter().map(|a|json!({"player_id":a.id,"season":a.season,"values":a.values,"weight":a.weight})).collect::<Vec<_>>(),"evaluation_awards":alternative_awards[i],"evaluation_disagreements":disagreements[i]})).collect();
            results.push(json!({"candidate_id":id,"group":group,"eligible":n,"awards_p55_p60_p65":awards,"winners_one_event_loss":lost,"nonwinners_one_event_gain":gained,"one_event_sensitive":lost+gained,"winner_sensitive_fraction":if awards[1]==0{None}else{Some(lost as f64/awards[1] as f64)},"posthoc_awarded_net_signs":net_signs,"references":reference_rows,"players":players}));
        }
    }
    Ok(
        json!({"schema":"rinkcraft.feat-robustness.v1","protocol_sha256":protocol_sha,"development_seasons":YEARS[..2],"evaluation_season":YEARS[2],"untouched_holdout":false,"population":populations,"results":results,"limits":"Retrospective deterministic sensitivity, not confidence intervals, latent skill, corrected game histories or automatic live promotion. Existing feat/card rules unchanged. Three candidates are skater-only; goalie population retained."}),
    )
}
fn main() {
    let a: Vec<_> = env::args().collect();
    let result = || -> Result<(), String> {
        if a.len() != 3 {
            return Err("Usage: ROBUSTNESS_INPUT OUTPUT".into());
        }
        let v: Value = serde_json::from_slice(&fs::read(&a[1]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let output = run(&v)?;
        fs::write(
            &a[2],
            serde_json::to_vec(&output).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!("Round-two count/reference stress test complete; no live feat changes.");
        Ok(())
    };
    if let Err(e) = result() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn actor() -> Actor {
        Actor {
            id: 1,
            group: "Forwards".into(),
            gp: 40,
            goals: 10,
            shots: 100,
            pp_points: 20,
            pp_seconds: 7200,
            drawn: 10,
            taken: 4,
        }
    }
    #[test]
    fn exposure_and_group_boundaries_stay_separate() {
        let mut a = actor();
        assert!(eligible(&a, "accuracy-ledger"));
        a.shots = 99;
        assert!(!eligible(&a, "accuracy-ledger"));
        a.shots = 100;
        a.pp_seconds = 7199;
        assert!(!eligible(&a, "pp-efficiency"));
        a.pp_seconds = 7200;
        assert!(eligible(&a, "pp-efficiency"));
        a.gp = 19;
        assert!(!eligible(&a, "penalty-balance"));
        let a = actor();
        assert!(reference(
            &[vec![a.clone()], vec![a]],
            "accuracy-ledger",
            "Defense",
            "actor_weighted"
        )
        .is_empty());
    }
    #[test]
    fn drawn_changes_both_derived_conditions() {
        let a = actor();
        let c = changes(&a, "penalty-balance");
        let b = &c.iter().find(|(k, _)| k == "drawn+1").unwrap().1;
        assert_eq!(values(b, "penalty-balance"), [7. / 40., 11. / 40.]);
    }
    #[test]
    fn no_negative_or_impossible_counts() {
        let mut a = actor();
        a.goals = 0;
        a.pp_points = 0;
        a.drawn = 0;
        a.taken = 0;
        assert_eq!(changes(&a, "accuracy-ledger").len(), 1);
        assert_eq!(changes(&a, "pp-efficiency").len(), 1);
        assert_eq!(changes(&a, "penalty-balance").len(), 2);
        a.goals = a.shots;
        assert_eq!(changes(&a, "accuracy-ledger").len(), 1);
    }
    #[test]
    fn actor_weight_is_one_across_development() {
        let a = actor();
        let refs = reference(
            &[vec![a.clone()], vec![a]],
            "accuracy-ledger",
            "Forwards",
            "actor_weighted",
        );
        assert_eq!(refs.len(), 2);
        assert_eq!(refs.iter().map(|a| a.weight).sum::<f64>(), 1.);
    }
    #[test]
    fn ties_and_all_conditions_count() {
        let r = vec![
            Ref {
                id: 1,
                season: YEARS[0],
                values: [1., 2.],
                weight: 1.,
            },
            Ref {
                id: 2,
                season: YEARS[0],
                values: [3., 2.],
                weight: 1.,
            },
        ];
        assert_eq!(ranks([1., 2.], &r), [25., 50.]);
        assert!(!award([80., 59.], 100, 60.));
        assert!(!award([80., 80.], 19, 60.));
    }
    #[test]
    fn development_references_cannot_access_evaluation() {
        let dev = vec![vec![actor()], vec![actor()]];
        let before = reference(&dev, "pp-efficiency", "Forwards", "actor_weighted");
        let mut evaluation = actor();
        evaluation.pp_points = 999;
        let after = reference(&dev, "pp-efficiency", "Forwards", "actor_weighted");
        assert_eq!(
            before.iter().map(|r| r.values).collect::<Vec<_>>(),
            after.iter().map(|r| r.values).collect::<Vec<_>>()
        );
        assert_eq!(values(&evaluation, "pp-efficiency")[0], 499.5);
    }
}
