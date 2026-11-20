//! Frozen-reference, identity-aware retrospective feat transfer. No live scoring.
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs};
const IDS: [&str; 3] = ["accuracy-ledger", "pp-efficiency", "penalty-balance"];
const YEARS: [u32; 3] = [20232024, 20242025, 20252026];
#[derive(Clone)]
struct Observation {
    id: u32,
    name: String,
    group: String,
    values: Vec<f64>,
    annual: bool,
}
struct Window {
    season: u32,
    population: BTreeMap<u32, (String, u32)>,
    candidates: BTreeMap<String, Vec<Observation>>,
    definitions: BTreeMap<String, Value>,
}
fn weighted_rank(value: f64, reference: &[(f64, f64)]) -> Option<f64> {
    let total: f64 = reference.iter().map(|(_, w)| w).sum();
    if total <= 0. {
        return None;
    }
    let lower: f64 = reference
        .iter()
        .filter(|(v, _)| *v < value)
        .map(|(_, w)| w)
        .sum();
    let tied: f64 = reference
        .iter()
        .filter(|(v, _)| *v == value)
        .map(|(_, w)| w)
        .sum();
    Some(100. * (lower + 0.5 * tied) / total)
}
fn ratio(n: usize, d: usize) -> Option<f64> {
    if d == 0 {
        None
    } else {
        Some(n as f64 / d as f64)
    }
}
fn load(entry: &Value, expected: u32) -> Result<Window, String> {
    if entry["season"] != expected {
        return Err("incorrect temporal split/order".into());
    }
    let read = |key: &str| -> Result<Vec<u8>, String> {
        fs::read(entry[key].as_str().ok_or("missing explicit input path")?)
            .map_err(|e| e.to_string())
    };
    let input = icelines_sources::nhl::achievements::parse(&read("reports")?)?;
    if input.season != expected {
        return Err("report window mismatch".into());
    }
    let screen: Value = serde_json::from_slice(&read("screen")?).map_err(|e| e.to_string())?;
    if screen["season"] != expected
        || screen["season_type"] != "regular"
        || screen["protocol"] != "rinkcraft.feat-screen.0.1-exploratory"
    {
        return Err("incompatible screen window/protocol".into());
    }
    let population: BTreeMap<_, _> = input
        .players
        .iter()
        .map(|p| (p.player_id, (p.group.clone(), p.games)))
        .collect();
    let mut candidates = BTreeMap::new();
    let mut definitions = BTreeMap::new();
    for id in IDS {
        let matching: Vec<_> = screen["results"]
            .as_array()
            .ok_or("missing results")?
            .iter()
            .filter(|r| r["definition"]["id"] == id)
            .collect();
        if matching.len() != 1 {
            return Err("candidate missing or duplicated".into());
        }
        let row = matching[0];
        let (expected_keys, expected_minimums) = match id {
            "accuracy-ledger" => (["conversion", "shots_pg"], json!({"shots":100})),
            "pp-efficiency" => (
                ["pp_points_per60", "pp_seconds"],
                json!({"pp_seconds":7200}),
            ),
            "penalty-balance" => (["net_penalties_pg", "drawn_pg"], json!({})),
            _ => unreachable!(),
        };
        if row["definition"]["minimums"] != expected_minimums {
            return Err("opportunity gates differ from frozen protocol".into());
        }
        let primary = &row["primary"];
        if primary["minimum_gp"] != 20
            || primary["threshold"] != 60.
            || primary["exposure_multiplier"] != 1.
        {
            return Err("incompatible primary policy".into());
        }
        let metrics = row["definition"]["metrics"]
            .as_array()
            .ok_or("missing definition metrics")?;
        if metrics.len() != 2 || metrics.iter().any(|m| m["direction"] != "higher") {
            return Err("expected two higher-favored frozen metrics".into());
        }
        if metrics
            .iter()
            .zip(expected_keys)
            .any(|(m, k)| m["key"] != k)
        {
            return Err("metric keys differ from frozen protocol".into());
        }
        let mut observations = Vec::new();
        let mut ids = BTreeSet::new();
        let mut groups = BTreeSet::new();
        for g in primary["groups"].as_array().ok_or("missing groups")? {
            let group = g["group"].as_str().ok_or("missing group")?;
            if !matches!(group, "Forwards" | "Defense") || !groups.insert(group) {
                return Err("invalid or repeated group".into());
            }
            let players = g["players"].as_array().ok_or("missing players")?;
            if g["eligible"].as_u64() != Some(players.len() as u64) {
                return Err("inconsistent eligible count".into());
            }
            for p in players {
                let player = p["player_id"]
                    .as_u64()
                    .and_then(|v| u32::try_from(v).ok())
                    .ok_or("bad actor")?;
                let (actual, gp) = population.get(&player).ok_or("actor absent from source")?;
                if actual != group
                    || *gp < 20
                    || !ids.insert(player)
                    || p["games"].as_u64() != Some(*gp as u64)
                {
                    return Err("actor group/GP/uniqueness mismatch".into());
                }
                let conditions = p["conditions"].as_array().ok_or("missing conditions")?;
                if conditions.len() != metrics.len() {
                    return Err("condition arity mismatch".into());
                }
                let mut values = Vec::new();
                let mut floor = 100_f64;
                for (c, m) in conditions.iter().zip(metrics) {
                    if c["key"] != m["key"] || c["direction"] != "higher" {
                        return Err("condition keys differ from definition".into());
                    }
                    values.push(
                        c["value"]
                            .as_f64()
                            .filter(|v| v.is_finite())
                            .ok_or("invalid observation")?,
                    );
                    floor = floor.min(
                        c["percentile"]
                            .as_f64()
                            .filter(|v| (0.0..=100.0).contains(v))
                            .ok_or("invalid rank")?,
                    );
                }
                let annual = players.len() >= 20 && floor >= 60.;
                if p["qualifies"].as_bool() != Some(annual) {
                    return Err("inconsistent annual decision".into());
                }
                observations.push(Observation {
                    id: player,
                    name: p["name"].as_str().ok_or("missing name")?.into(),
                    group: group.into(),
                    values,
                    annual,
                });
            }
        }
        if groups.len() != 2 {
            return Err("both F/D groups required".into());
        }
        definitions.insert(id.into(), row["definition"].clone());
        candidates.insert(id.into(), observations);
    }
    Ok(Window {
        season: expected,
        population,
        candidates,
        definitions,
    })
}
fn analyze(windows: &[Window]) -> Result<Value, String> {
    if windows.len() != 3 || windows.iter().zip(YEARS).any(|(w, y)| w.season != y) {
        return Err("expected fixed three-season order".into());
    }
    let mut results = Vec::new();
    for id in IDS {
        if windows[1..]
            .iter()
            .any(|w| w.definitions[id] != windows[0].definitions[id])
        {
            return Err("definitions changed across seasons".into());
        }
        let mut references = Vec::new();
        let mut frozen: BTreeMap<String, Vec<Vec<(f64, f64)>>> = BTreeMap::new();
        for group in ["Forwards", "Defense"] {
            let mut appearances: BTreeMap<u32, usize> = BTreeMap::new();
            for w in &windows[..2] {
                for p in w.candidates[id].iter().filter(|p| p.group == group) {
                    *appearances.entry(p.id).or_default() += 1;
                }
            }
            let mut refs = vec![Vec::new(), Vec::new()];
            let mut observations = Vec::new();
            for w in &windows[..2] {
                for p in w.candidates[id].iter().filter(|p| p.group == group) {
                    let weight = 1. / appearances[&p.id] as f64;
                    for (v, r) in p.values.iter().zip(&mut refs) {
                        r.push((*v, weight));
                    }
                    observations.push(json!({"season":w.season,"player_id":p.id,"weight":weight,"values":p.values}));
                }
            }
            references.push(json!({"group":group,"unique_actors":appearances.len(),"player_seasons":observations.len(),"repeated_actors":appearances.values().filter(|n|**n>1).count(),"minimum_reference_met":appearances.len()>=20,"observations":observations}));
            frozen.insert(group.into(), refs);
        }
        let mut seasons = Vec::new();
        let mut awards: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
        for w in windows {
            let mut group_results = Vec::new();
            let mut winner_ids = BTreeSet::new();
            for group in ["Forwards", "Defense"] {
                let refs = &frozen[group];
                let reference_n = references.iter().find(|r| r["group"] == group).unwrap()
                    ["unique_actors"]
                    .as_u64()
                    .unwrap();
                let mut records = Vec::new();
                for p in w.candidates[id].iter().filter(|p| p.group == group) {
                    let ranks: Vec<_> = p
                        .values
                        .iter()
                        .zip(refs)
                        .map(|(v, r)| weighted_rank(*v, r))
                        .collect();
                    let floor = ranks
                        .iter()
                        .copied()
                        .collect::<Option<Vec<_>>>()
                        .map(|v| v.into_iter().fold(100_f64, f64::min));
                    let qualifies = reference_n >= 20 && floor.is_some_and(|v| v >= 60.);
                    if qualifies {
                        winner_ids.insert(p.id);
                    }
                    records.push(json!({"player_id":p.id,"name":p.name,"values":p.values,"frozen_percentiles":ranks,"annual_qualifies":p.annual,"frozen_qualifies":qualifies,"frozen_p55":reference_n>=20&&floor.is_some_and(|v|v>=55.),"frozen_p65":reference_n>=20&&floor.is_some_and(|v|v>=65.)}));
                }
                let annual = records
                    .iter()
                    .filter(|r| r["annual_qualifies"] == true)
                    .count();
                let count = records
                    .iter()
                    .filter(|r| r["frozen_qualifies"] == true)
                    .count();
                let both = records
                    .iter()
                    .filter(|r| r["annual_qualifies"] == true && r["frozen_qualifies"] == true)
                    .count();
                group_results.push(json!({"group":group,"eligible":records.len(),"annual_earned":annual,"frozen_earned":count,"both_earned":both,"annual_frozen_jaccard":ratio(both,annual+count-both),"frozen_p55":records.iter().filter(|r|r["frozen_p55"]==true).count(),"frozen_p65":records.iter().filter(|r|r["frozen_p65"]==true).count(),"players":records}));
            }
            awards.insert(w.season, winner_ids);
            seasons.push(json!({"season":w.season,"role":if w.season==YEARS[2]{"retrospective_evaluation"}else{"development"},"groups":group_results}));
        }
        let mut persistence = Vec::new();
        for pair in windows.windows(2) {
            let a = &pair[0];
            let b = &pair[1];
            for group in ["Forwards", "Defense"] {
                for mode in ["annual", "frozen"] {
                    let previous: BTreeMap<_, _> = a.candidates[id]
                        .iter()
                        .filter(|p| p.group == group)
                        .map(|p| (p.id, p))
                        .collect();
                    let next: BTreeMap<_, _> = b.candidates[id]
                        .iter()
                        .filter(|p| p.group == group)
                        .map(|p| (p.id, p))
                        .collect();
                    let common: BTreeSet<_> = previous
                        .keys()
                        .filter(|id| next.contains_key(id))
                        .copied()
                        .collect();
                    let win = |p: &Observation, year: u32| {
                        if mode == "annual" {
                            p.annual
                        } else {
                            awards[&year].contains(&p.id)
                        }
                    };
                    let prior_winners: BTreeSet<_> = previous
                        .values()
                        .filter(|p| win(p, a.season))
                        .map(|p| p.id)
                        .collect();
                    let comparable_winners = prior_winners.intersection(&common).count();
                    let retained = common
                        .iter()
                        .filter(|pid| prior_winners.contains(pid) && win(next[pid], b.season))
                        .count();
                    let next_winners = common.iter().filter(|pid| win(next[pid], b.season)).count();
                    let retained_rate = ratio(retained, comparable_winners);
                    let base = ratio(next_winners, common.len());
                    let mut absent = 0;
                    let mut switched = 0;
                    let mut ineligible = 0;
                    for actor in prior_winners.difference(&common) {
                        match b.population.get(actor) {
                            None => absent += 1,
                            Some((g, _)) if g != group => switched += 1,
                            _ => ineligible += 1,
                        }
                    }
                    persistence.push(json!({"from":a.season,"to":b.season,"group":group,"mode":mode,"common_eligible":common.len(),"prior_winners":prior_winners.len(),"comparable_prior_winners":comparable_winners,"retained_winners":retained,"retention_fraction":retained_rate,"next_awards_in_common":next_winners,"next_award_fraction_in_common":base,"descriptive_retention_ratio":retained_rate.zip(base).filter(|(_,b)|*b>0.).map(|(a,b)|a/b),"prior_winners_absent":absent,"prior_winners_group_changed":switched,"prior_winners_ineligible":ineligible}));
                }
            }
        }
        results.push(json!({"candidate_id":id,"definition":windows[0].definitions[id],"frozen_references":references,"seasons":seasons,"persistence":persistence}));
    }
    let populations: Vec<_> = windows.iter().map(|w| {
        let groups: Vec<_> = ["Forwards", "Defense", "Goalies"].iter().map(|g| {
            json!({"group":g,"observed":w.population.values().filter(|(v,_)|v==g).count(),"gp_eligible":w.population.values().filter(|(v,gp)|v==g&&*gp>=if *g=="Goalies"{15}else{20}).count()})
        }).collect();
        json!({"season":w.season,"groups":groups})
    }).collect();
    let unique: BTreeSet<_> = windows.iter().flat_map(|w| w.population.keys()).collect();
    Ok(
        json!({"protocol":"rinkcraft.feat-longitudinal.0.1-retrospective","development_seasons":[20232024,20242025],"evaluation_season":20252026,"untouched_holdout":false,"population":populations,"unique_observed_actors":unique.len(),"player_seasons":windows.iter().map(|w|w.population.len()).sum::<usize>(),"results":results,"limits":["Retrospective previously inspected seasons; not confirmatory/prospective holdout","Frozen reference weights each development actor equally within candidate/group; observations remain dependent","Eligibility survival, scorer/deployment/context effects and multiple comparisons unresolved","Award persistence is not a calibrated skill probability or causal effect","No live candidate promotion, capability grade or simulation bonus"]}),
    )
}
fn main() {
    let args: Vec<_> = env::args().collect();
    let run = || -> Result<(), String> {
        if args.len() != 3 {
            return Err("Usage: LONGITUDINAL_INPUT_MANIFEST OUTPUT_JSON".into());
        }
        let manifest: Value =
            serde_json::from_slice(&fs::read(&args[1]).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if manifest["untouched_holdout"] != false {
            return Err("prior exposure requires retrospective designation".into());
        }
        let entries = manifest["windows"].as_array().ok_or("missing windows")?;
        if entries.len() != 3 {
            return Err("three windows required".into());
        }
        let windows = entries
            .iter()
            .zip(YEARS)
            .map(|(e, y)| load(e, y))
            .collect::<Result<Vec<_>, _>>()?;
        fs::write(
            &args[2],
            serde_json::to_vec(&analyze(&windows)?).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!("Three candidate feats evaluated across three seasons with frozen development references.");
        Ok(())
    };
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weighted_ties_and_empty() {
        assert_eq!(
            weighted_rank(2., &[(1., 0.5), (2., 0.5), (2., 1.)]),
            Some(62.5)
        );
        assert_eq!(weighted_rank(0., &[]), None);
        assert_eq!(ratio(0, 0), None);
    }
    #[test]
    fn repeated_players_have_one_total_weight() {
        let appearances: BTreeMap<_, _> = [(1, 2), (2, 1)].into();
        let rows = [(1, 0.), (1, 10.), (2, 20.)];
        let refs: Vec<_> = rows
            .iter()
            .map(|(id, v)| (*v, 1. / appearances[id] as f64))
            .collect();
        assert_eq!(refs.iter().map(|(_, w)| w).sum::<f64>(), 2.);
        assert_eq!(weighted_rank(20., &refs), Some(75.));
    }
    #[test]
    fn temporal_order_rejected() {
        assert!(analyze(&[]).is_err());
    }
    #[test]
    fn evaluation_values_do_not_enter_reference() {
        fn window(season: u32) -> Window {
            let observations: Vec<_> = (1..=25)
                .map(|id| Observation {
                    id,
                    name: id.to_string(),
                    group: "Forwards".into(),
                    values: vec![id as f64, id as f64],
                    annual: id >= 16,
                })
                .collect();
            Window {
                season,
                population: (1..=25).map(|id| (id, ("Forwards".into(), 40))).collect(),
                candidates: IDS
                    .iter()
                    .map(|id| (id.to_string(), observations.clone()))
                    .collect(),
                definitions: IDS
                    .iter()
                    .map(|id| (id.to_string(), json!({"id":id})))
                    .collect(),
            }
        }
        let mut windows: Vec<_> = YEARS.into_iter().map(window).collect();
        let before = analyze(&windows).unwrap();
        for observations in windows[2].candidates.values_mut() {
            for p in observations {
                p.values = vec![10000., 10000.];
            }
        }
        let after = analyze(&windows).unwrap();
        for (a, b) in before["results"]
            .as_array()
            .unwrap()
            .iter()
            .zip(after["results"].as_array().unwrap())
        {
            assert_eq!(a["frozen_references"], b["frozen_references"]);
            assert_ne!(
                a["seasons"][2]["groups"][0]["frozen_earned"],
                b["seasons"][2]["groups"][0]["frozen_earned"]
            );
        }
    }
}
