//! Offline exploratory screen. Does not alter the live feat registry or cards.
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs};

#[derive(Clone)]
struct Actor {
    id: u32,
    name: String,
    group: String,
    games: u32,
    m: BTreeMap<String, f64>,
}
fn rows(v: &Value, season: u32) -> Result<BTreeMap<u32, &Value>, String> {
    let a = v["data"].as_array().ok_or("missing data")?;
    if v["total"].as_u64() != Some(a.len() as u64) {
        return Err("incomplete report".into());
    }
    let mut out = BTreeMap::new();
    for r in a {
        let id = r["playerId"]
            .as_u64()
            .and_then(|x| u32::try_from(x).ok())
            .filter(|x| *x > 0)
            .ok_or("bad actor")?;
        if r["seasonId"].as_u64() != Some(season as u64) || out.insert(id, r).is_some() {
            return Err("mixed season or duplicate actor".into());
        }
    }
    Ok(out)
}
fn number(r: &Value, k: &str) -> Result<f64, String> {
    r[k].as_f64()
        .filter(|x| x.is_finite() && *x >= 0.)
        .ok_or_else(|| format!("missing/invalid {k}"))
}
fn rank(v: f64, reference: &[f64], lower: bool) -> f64 {
    let below = reference.iter().filter(|x| **x < v).count();
    let equal = reference.iter().filter(|x| **x == v).count();
    let p = 100. * (below as f64 + 0.5 * equal as f64) / reference.len() as f64;
    if lower {
        100. - p
    } else {
        p
    }
}
fn jaccard(a: &BTreeSet<u32>, b: &BTreeSet<u32>) -> Option<f64> {
    let n = a.union(b).count();
    if n == 0 {
        None
    } else {
        Some(a.intersection(b).count() as f64 / n as f64)
    }
}
fn eligible(p: &Actor, rule: &Value, gp: u32, exposure: f64) -> bool {
    p.games >= gp
        && rule["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| p.m.contains_key(m["key"].as_str().unwrap()))
        && rule["minimums"].as_object().unwrap().iter().all(|(k, v)| {
            p.m.get(k)
                .is_some_and(|x| *x >= v.as_f64().unwrap() * exposure)
        })
}
fn evaluate(actors: &[Actor], rule: &Value, gp: u32, exposure: f64, threshold: f64) -> Value {
    let metrics = rule["metrics"].as_array().unwrap();
    let mut groups = Vec::new();
    for group in ["Forwards", "Defense"] {
        let peers: Vec<_> = actors
            .iter()
            .filter(|p| p.group == group && eligible(p, rule, gp, exposure))
            .collect();
        let refs: Vec<Vec<f64>> = metrics
            .iter()
            .map(|m| {
                peers
                    .iter()
                    .map(|p| p.m[m["key"].as_str().unwrap()])
                    .collect()
            })
            .collect();
        let mut records = Vec::new();
        for p in &peers {
            let conditions: Vec<_> = metrics
                .iter()
                .zip(&refs)
                .map(|(m, r)| {
                    let k = m["key"].as_str().unwrap();
                    let v = p.m[k];
                    let pct = rank(v, r, m["direction"] == "lower");
                    json!({"key":k,"value":v,"percentile":pct,"direction":m["direction"]})
                })
                .collect();
            let qualifies = peers.len() >= 20
                && conditions
                    .iter()
                    .all(|c| c["percentile"].as_f64().unwrap() >= threshold);
            records.push(json!({"player_id":p.id,"name":p.name,"games":p.games,"qualifies":qualifies,"conditions":conditions}));
        }
        let earned = records.iter().filter(|r| r["qualifies"] == true).count();
        let ties:Vec<_>=refs.iter().zip(metrics).map(|(values,m)|{
            let mut sorted=values.clone();sorted.sort_by(f64::total_cmp);
            let mut max_tie=0;let mut run=0;let mut previous=None;
            for v in &sorted{if Some(*v)==previous{run+=1}else{run=1;}previous=Some(*v);max_tie=max_tie.max(run);}
            json!({"metric":m["key"],"zero_count":values.iter().filter(|v|**v==0.).count(),"largest_tie":max_tie})
        }).collect();
        groups.push(json!({"group":group,"eligible":peers.len(),"planning_target_met":peers.len()>=100,"percentile_floor_met":peers.len()>=20,"earned":earned,"award_fraction":if peers.is_empty(){None}else{Some(earned as f64/peers.len() as f64)},"ties":ties,"players":records}));
    }
    json!({"candidate_id":rule["id"],"minimum_gp":gp,"exposure_multiplier":exposure,"threshold":threshold,"groups":groups})
}
fn set(v: &Value, earned_only: bool) -> BTreeSet<u32> {
    v["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["players"].as_array().unwrap())
        .filter(|p| !earned_only || p["qualifies"] == true)
        .map(|p| p["player_id"].as_u64().unwrap() as u32)
        .collect()
}
fn run(args: &[String]) -> Result<(), String> {
    if args.len() != 6 {
        return Err(
            "Usage: REPORTS_JSON TOI_JSON CANDIDATES_JSON SCREEN_RULES_JSON OUTPUT_JSON".into(),
        );
    }
    let read = |i: usize| -> Result<Value, String> {
        serde_json::from_slice(&fs::read(&args[i]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    };
    let raw = fs::read(&args[1]).map_err(|e| e.to_string())?;
    let input = icelines_sources::nhl::achievements::parse(&raw)?;
    let source: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    let time = read(2)?;
    let candidates = read(3)?;
    let spec = read(4)?;
    let definitions = spec["rules"].as_array().ok_or("missing rules")?;
    if spec["minimum_gp"] != 20
        || spec["minimum_peers"] != 20
        || spec["planning_target_per_skater_group"] != 100
        || spec["primary_threshold"] != 60
        || spec["season"] != input.season
        || spec["threshold_sensitivity"] != json!([55, 65, 75, 90])
        || spec["exposure_multipliers"] != json!([0.5, 1.5])
        || spec["gp_sensitivity"] != json!([10, 40])
    {
        return Err("screen protocol settings differ from version 0.1".into());
    }
    let supported = [
        "gwg_pg",
        "ot_goals_pg",
        "shots",
        "shots_pg",
        "net_penalties_pg",
        "drawn_pg",
        "total_seconds",
        "games",
        "pp_seconds",
        "sh_seconds",
        "pp_seconds_pg",
        "sh_seconds_pg",
        "shifts_pg",
        "seconds_per_shift",
        "conversion",
        "pp_points_per60",
        "sh_points_per60",
    ];
    let all = candidates["candidates"]
        .as_array()
        .ok_or("missing candidates")?;
    if all.len() != 33 {
        return Err("expected frozen 33 candidate registry".into());
    }
    let ids: BTreeSet<_> = all
        .iter()
        .map(|c| c["id"].as_str().ok_or("bad candidate"))
        .collect::<Result<_, _>>()?;
    let mut seen = BTreeSet::new();
    for d in definitions {
        let id = d["id"].as_str().ok_or("bad screen id")?;
        if !ids.contains(id)
            || !seen.insert(id)
            || d["metrics"]
                .as_array()
                .is_none_or(|m| m.is_empty() || m.len() > 3)
            || !d["minimums"].is_object()
        {
            return Err("invalid screen rule".into());
        }
        for m in d["metrics"].as_array().unwrap() {
            if !matches!(m["direction"].as_str(), Some("higher" | "lower"))
                || !m["key"].as_str().is_some_and(|k| supported.contains(&k))
            {
                return Err("invalid metric direction".into());
            }
        }
        let metric_keys: BTreeSet<_> = d["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["key"].as_str().unwrap())
            .collect();
        if metric_keys.len() != d["metrics"].as_array().unwrap().len()
            || d["minimums"]
                .as_object()
                .unwrap()
                .keys()
                .any(|k| !supported.contains(&k.as_str()))
        {
            return Err("duplicate metric or unknown gate".into());
        }
        if d["minimums"]
            .as_object()
            .unwrap()
            .values()
            .any(|n| n.as_f64().is_none_or(|x| !x.is_finite() || x < 0.))
        {
            return Err("invalid exposure gate".into());
        }
    }
    let sk = rows(&source["skaters"], input.season)?;
    let pen = rows(&source["penalties"], input.season)?;
    let toi = rows(&time, input.season)?;
    if sk.keys().ne(toi.keys()) {
        return Err("TOI actor cohort mismatch".into());
    }
    let mut actors = Vec::new();
    for p in input.players.iter().filter(|p| p.group != "Goalies") {
        let s = sk[&p.player_id];
        let t = toi[&p.player_id];
        let q = pen[&p.player_id];
        if t["gamesPlayed"].as_u64() != Some(p.games as u64)
            || t["positionCode"] != s["positionCode"]
        {
            return Err("TOI GP/position mismatch".into());
        }
        let gp = p.games as f64;
        if gp == 0. {
            return Err("zero GP".into());
        }
        let n = |r, k| number(r, k);
        let total = n(t, "timeOnIce")?;
        let pp = n(t, "ppTimeOnIce")?;
        let sh = n(t, "shTimeOnIce")?;
        let ev = n(t, "evTimeOnIce")?;
        let shifts = n(t, "shifts")?;
        if (total - pp - sh - ev).abs() > 1. || shifts == 0. {
            return Err("TOI accounting/shift inconsistency".into());
        }
        let shots = n(s, "shots")?;
        let goals = n(s, "goals")?;
        let pppts = n(s, "ppPoints")?;
        let shpts = n(s, "shPoints")?;
        if goals > shots {
            return Err("goals exceed shots".into());
        }
        if (pp == 0. && pppts > 0.) || (sh == 0. && shpts > 0.) {
            return Err("special-team output without exposure".into());
        }
        let drawn = n(q, "penaltiesDrawn")?;
        let taken = n(q, "penalties")?;
        let reported_net = q["netPenalties"]
            .as_f64()
            .filter(|v| v.is_finite())
            .ok_or("missing/invalid signed net penalties")?;
        if (reported_net - (drawn - taken)).abs() > 0.001 {
            return Err("net penalty mismatch".into());
        }
        let mut m: BTreeMap<String, f64> = [
            ("gwg_pg", n(s, "gameWinningGoals")? / gp),
            ("ot_goals_pg", n(s, "otGoals")? / gp),
            ("shots", shots),
            ("shots_pg", shots / gp),
            ("net_penalties_pg", (drawn - taken) / gp),
            ("drawn_pg", drawn / gp),
            ("total_seconds", total),
            ("games", gp),
            ("pp_seconds", pp),
            ("sh_seconds", sh),
            ("pp_seconds_pg", pp / gp),
            ("sh_seconds_pg", sh / gp),
            ("shifts_pg", shifts / gp),
            ("seconds_per_shift", total / shifts),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v))
        .collect();
        if shots > 0. {
            m.insert("conversion".into(), goals / shots);
        }
        if pp > 0. {
            m.insert("pp_points_per60".into(), pppts * 3600. / pp);
        }
        if sh > 0. {
            m.insert("sh_points_per60".into(), shpts * 3600. / sh);
        }
        actors.push(Actor {
            id: p.player_id,
            name: p.name.clone(),
            group: p.group.clone(),
            games: p.games,
            m,
        });
    }
    let baseline = icelines_core::view_model::rinkcraft_achievements::evaluate(
        &input,
        &icelines_core::view_model::rinkcraft_achievements::default_rules(),
    )?;
    let mut active: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    for p in &baseline.profiles {
        for f in &p.feats {
            if f.state == "earned" {
                active.entry(f.id.clone()).or_default().insert(p.player_id);
            }
        }
    }
    let primary: Vec<_> = definitions
        .iter()
        .map(|d| evaluate(&actors, d, 20, 1., 60.))
        .collect();
    let mut results = Vec::new();
    for (d, p) in definitions.iter().zip(&primary) {
        let eligible_ids = set(p, false);
        let winners = set(p, true);
        let mut overlaps:Vec<_>=active.iter().map(|(id,a)|{let restricted=a.intersection(&eligible_ids).copied().collect();json!({"active_id":id,"jaccard":jaccard(&winners,&restricted),"common_eligible":eligible_ids.len(),"shared":winners.intersection(a).count()})}).collect();
        overlaps.sort_by(|a, b| {
            b["jaccard"]
                .as_f64()
                .unwrap_or(-1.)
                .total_cmp(&a["jaccard"].as_f64().unwrap_or(-1.))
        });
        let threshold: Vec<_> = [55., 65., 75., 90.]
            .iter()
            .map(|t| evaluate(&actors, d, 20, 1., *t))
            .collect();
        let exposure: Vec<_> = [0.5, 1.5]
            .iter()
            .map(|e| evaluate(&actors, d, 20, *e, 60.))
            .collect();
        let gp: Vec<_> = [10, 40]
            .iter()
            .map(|g| evaluate(&actors, d, *g, 1., 60.))
            .collect();
        results.push(json!({"definition":d,"primary":p,"top_active_overlaps":overlaps.into_iter().take(3).collect::<Vec<_>>(),"threshold_sensitivity":threshold,"exposure_sensitivity":exposure,"gp_sensitivity":gp}));
    }
    let mut pairwise = Vec::new();
    for (i, a) in primary.iter().enumerate() {
        for b in primary.iter().skip(i + 1) {
            let common: BTreeSet<_> = set(a, false)
                .intersection(&set(b, false))
                .copied()
                .collect();
            let wa = set(a, true).intersection(&common).copied().collect();
            let wb = set(b, true).intersection(&common).copied().collect();
            pairwise.push(json!({"a":a["candidate_id"],"b":b["candidate_id"],"common_eligible":common.len(),"jaccard":jaccard(&wa,&wb)}));
        }
    }
    let gates:Vec<_>=all.iter().map(|c|json!({"id":c["id"],"name":c["name"],"state":if seen.contains(c["id"].as_str().unwrap()){ "exploratory_computed" }else{"gated_not_computed"},"evidence_gate":c["evidence_gate"]})).collect();
    let populations:Vec<_>=["Forwards","Defense","Goalies"].iter().map(|g|json!({"group":g,"observed":input.players.iter().filter(|p|p.group==*g).count(),"gp_eligible":input.players.iter().filter(|p|p.group==*g&&p.games>=if *g=="Goalies"{15}else{20}).count()})).collect();
    let output = json!({"protocol":"rinkcraft.feat-screen.0.1-exploratory","season":input.season,"season_type":"regular","population":populations,"candidate_dispositions":gates,"results":results,"candidate_overlap":pairwise,"limits":["Single-season exploratory screen; no held-out evaluation or capability calibration","Same observed cohort defines and evaluates ranks; award frequency is descriptive","P60 threshold, exposure/GP gates and planning population targets are authored","Goalie candidate inputs unavailable; no skater substitutes","No live registry, card, class, grade or simulation change"]});
    fs::write(
        &args[5],
        serde_json::to_vec(&output).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "Screened {} candidates; gated {}; {} skaters",
        definitions.len(),
        all.len() - definitions.len(),
        actors.len()
    );
    Ok(())
}
fn main() {
    if let Err(e) = run(&env::args().collect::<Vec<_>>()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ties_direction_and_signed_values() {
        assert_eq!(rank(0., &[-1., 0., 0., 2.], false), 50.);
        assert_eq!(rank(0., &[-1., 0., 0., 2.], true), 50.);
        assert_eq!(rank(-1., &[-1., 0., 2.], true), 100. - 100. / 6.);
    }
    #[test]
    fn overlap_does_not_reward_two_empty_awards() {
        assert_eq!(jaccard(&BTreeSet::new(), &BTreeSet::new()), None);
        assert_eq!(jaccard(&[1, 2].into(), &[2, 3].into()), Some(1. / 3.));
    }
    #[test]
    fn missing_zero_exposure_and_population_floor() {
        let a = Actor {
            id: 1,
            name: "A".into(),
            group: "Forwards".into(),
            games: 20,
            m: [("rate".into(), 0.), ("shots".into(), 100.)].into(),
        };
        let rule =
            json!({"metrics":[{"key":"rate","direction":"higher"}],"minimums":{"shots":100}});
        assert!(eligible(&a, &rule, 20, 1.));
        assert!(!eligible(&a, &rule, 20, 1.5));
        let out = evaluate(&[a], &rule, 20, 1., 0.);
        assert_eq!(out["groups"][0]["earned"], 0);
        assert_eq!(out["groups"][0]["eligible"], 1);
    }
    #[test]
    fn rejects_partial_duplicate_mixed_and_missing_inputs() {
        assert!(rows(&json!({"total":2,"data":[]}), 20242025).is_err());
        let row = json!({"playerId":1,"seasonId":20242025});
        assert!(rows(&json!({"total":2,"data":[row.clone(),row]}), 20242025).is_err());
        assert!(rows(
            &json!({"total":1,"data":[{"playerId":1,"seasonId":20232024}]}),
            20242025
        )
        .is_err());
        assert!(number(&json!({"x":null}), "x").is_err());
    }
}
