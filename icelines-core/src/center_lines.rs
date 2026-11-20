//! Center-usage line inference above observed complete trios; no coach-label claim.
use crate::{
    deployment_audit::{index, number, rows},
    observed_units::reconstruct_complete,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameCapture {
    pub game_id: u64,
    pub game_date: String,
    pub home_id: u64,
    pub away_id: u64,
    pub box_path: Option<String>,
    pub shifts_path: Option<String>,
}
#[derive(Clone, Copy, Default)]
struct Faceoffs {
    ev_per_game: f64,
    ev_seconds_per_game: f64,
}
#[derive(Clone, Copy)]
enum Anchor {
    Center(u64),
    UsageCenter(u64),
    Ambiguous,
    Unresolved,
}
fn choose(trio: &Value, faces: &BTreeMap<u64, Faceoffs>) -> Anchor {
    let has_center = trio["members"].as_array().unwrap().iter().any(|m| {
        m["position"] == "C"
            && m["id"]
                .as_u64()
                .and_then(|id| faces.get(&id))
                .map(|f| f.ev_per_game >= 1.0)
                .unwrap_or(false)
    });
    let mut candidates: Vec<_> = trio["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| {
            let id = m["id"].as_u64()?;
            let load = faces.get(&id)?.ev_per_game;
            let eligible = if has_center {
                m["position"] == "C" && load >= 1.0
            } else {
                load >= 3.0
            };
            eligible.then_some((id, load))
        })
        .collect();
    candidates.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    match candidates.as_slice() {
        [] => Anchor::Unresolved,
        [(id, _)] => Anchor::Center(*id),
        [(id, load), (_, second), ..] => {
            if second / load >= 0.75 {
                let mut usage: Vec<_> = candidates
                    .iter()
                    .filter(|(_, draws)| draws / load >= 0.75)
                    .map(|(pid, _)| (*pid, faces[pid].ev_seconds_per_game))
                    .collect();
                usage.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                if usage.len() > 1 && usage[0].1 > 0.0 && usage[0].1 > usage[1].1 {
                    Anchor::UsageCenter(usage[0].0)
                } else {
                    Anchor::Ambiguous
                }
            } else {
                Anchor::Center(*id)
            }
        }
    }
}
#[derive(Default, Clone)]
struct Center {
    seconds: u64,
    games: BTreeSet<u64>,
    trios: BTreeMap<Vec<u64>, (u64, Value)>,
    usage_resolved: u64,
}
#[derive(Default)]
struct Aggregate {
    centers: BTreeMap<u64, Center>,
    standard: u64,
    ambiguous: u64,
    unresolved: u64,
    nonstandard: u64,
    censored: u64,
    invalid: u64,
    usage_resolved: u64,
    usage_resolved_trios: BTreeMap<Vec<u64>, (u64, Value)>,
    ambiguous_trios: BTreeMap<Vec<u64>, (u64, Value)>,
    unresolved_trios: BTreeMap<Vec<u64>, (u64, Value)>,
    pairs: BTreeMap<Vec<u64>, (u64, Value)>,
    pp: BTreeMap<Vec<u64>, (u64, Value)>,
    pk: BTreeMap<Vec<u64>, (u64, Value)>,
}
fn merge(map: &mut BTreeMap<Vec<u64>, (u64, Value)>, unit: &Value) {
    let key = unit["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_u64().unwrap())
        .collect();
    let row = map
        .entry(key)
        .or_insert_with(|| (0, unit["members"].clone()));
    row.0 += unit["seconds"].as_u64().unwrap();
}
fn serialized(map: &BTreeMap<Vec<u64>, (u64, Value)>) -> Value {
    let mut rows: Vec<_> = map.values().collect();
    rows.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.to_string().cmp(&b.1.to_string()))
    });
    json!(rows
        .iter()
        .map(|(seconds, members)| json!({"seconds":seconds,"members":members}))
        .collect::<Vec<_>>())
}
fn aggregate(games: &[(u64, String, Value)], faces: &BTreeMap<u64, Faceoffs>) -> Aggregate {
    let mut a = Aggregate::default();
    for (game, _, u) in games {
        for (field, dest) in [
            ("five_on_five_pairs", &mut a.pairs),
            ("five_on_four_units", &mut a.pp),
            ("four_on_five_units", &mut a.pk),
        ] {
            if let Some(units) = u[field].as_array() {
                for unit in units {
                    merge(dest, unit);
                }
            }
        }
        let trios = u["five_on_five_trios"].as_array().unwrap();
        let standard: u64 = trios.iter().map(|t| t["seconds"].as_u64().unwrap()).sum();
        a.standard += standard;
        a.nonstandard += u["state_seconds"]["5v5"]
            .as_u64()
            .unwrap_or(0)
            .saturating_sub(standard);
        a.censored += u["state_seconds"]["censored_conflicting_shift"]
            .as_u64()
            .unwrap_or(0);
        a.invalid += u["state_seconds"]["invalid_occupancy"]
            .as_u64()
            .unwrap_or(0);
        for trio in trios {
            let seconds = trio["seconds"].as_u64().unwrap();
            let anchor = choose(trio, faces);
            match anchor {
                Anchor::Ambiguous => {
                    a.ambiguous += seconds;
                    merge(&mut a.ambiguous_trios, trio);
                }
                Anchor::Unresolved => {
                    a.unresolved += seconds;
                    merge(&mut a.unresolved_trios, trio);
                }
                Anchor::Center(pid) | Anchor::UsageCenter(pid) => {
                    let usage = matches!(anchor, Anchor::UsageCenter(_));
                    if usage {
                        a.usage_resolved += seconds;
                        merge(&mut a.usage_resolved_trios, trio);
                    }
                    let c = a.centers.entry(pid).or_default();
                    if usage {
                        c.usage_resolved += seconds;
                    }
                    c.seconds += seconds;
                    c.games.insert(*game);
                    let key = trio["members"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|m| m["id"].as_u64().unwrap())
                        .collect();
                    let row = c
                        .trios
                        .entry(key)
                        .or_insert_with(|| (0, trio["members"].clone()));
                    row.0 += seconds;
                }
            }
        }
    }
    a
}
fn established(centers: &BTreeMap<u64, Center>) -> Vec<u64> {
    let mut ids: Vec<_> = centers.keys().copied().collect();
    ids.sort_by(|a, b| {
        let ca = &centers[a];
        let cb = &centers[b];
        (cb.games.len() >= 3)
            .cmp(&(ca.games.len() >= 3))
            .then_with(|| {
                (cb.seconds as f64 / cb.games.len() as f64)
                    .total_cmp(&(ca.seconds as f64 / ca.games.len() as f64))
            })
            .then_with(|| cb.seconds.cmp(&ca.seconds))
            .then_with(|| a.cmp(b))
    });
    ids
}
fn window(
    name: &str,
    games: &[(u64, String, Value)],
    season: &Aggregate,
    faces: &BTreeMap<u64, Faceoffs>,
    names: &BTreeMap<u64, String>,
) -> Value {
    let a = aggregate(games, faces);
    let ranks = established(&season.centers);
    let mut centers: Vec<_> = a.centers.iter().collect();
    centers.sort_by_key(|(id, c)| {
        (
            (c.seconds as f64 / c.games.len() as f64) < 120.0,
            ranks.iter().position(|p| p == *id).unwrap_or(usize::MAX),
        )
    });
    let rows:Vec<_>=centers.into_iter().enumerate().map(|(i,(id,c))|{
        let mut trios:Vec<_>=c.trios.values().collect();trios.sort_by(|a,b|b.0.cmp(&a.0).then_with(||a.1.to_string().cmp(&b.1.to_string())));
        let combos:Vec<_>=trios.iter().map(|(seconds,members)|json!({"seconds":seconds,"members":members})).collect();
        let baseline=&season.centers[id];
        let incidental=(c.seconds as f64/c.games.len() as f64)<120.0;
        json!({"label":if incidental{"incidental / relief center".into()}else if i<4{format!("inferred L{}",i+1)}else{"additional center".into()},"basis":"season center-usage ordering among substantive window centers; not a coach-confirmed label",
            "center_id":id,"center":names.get(id),"centered_seconds":c.seconds,"games_with_anchor":c.games.len(),"usage_tiebreak_seconds":c.usage_resolved,
            "window_minutes_per_anchor_game":c.seconds as f64/c.games.len() as f64/60.0,"season_minutes_per_anchor_game":baseline.seconds as f64/baseline.games.len() as f64/60.0,
            "season_anchor_games":baseline.games.len(),"season_center_usage_rank":ranks.iter().position(|p|p==id).unwrap()+1,"incidental_window_usage":incidental,
            "sparse_season_sample":baseline.games.len()<3,"center_ev_faceoffs_per_game":faces[id].ev_per_game,
            "primary_trio_share_of_centered_time":trios.first().map(|r|r.0 as f64/c.seconds as f64),"trios":combos})
    }).collect();
    let assigned: u64 = a.centers.values().map(|c| c.seconds).sum();
    json!({"window":name,"games":games.len(),"first_date":games.first().map(|g|&g.1),"last_date":games.last().map(|g|&g.1),
        "standard_trio_seconds":a.standard,"center_assigned_seconds":assigned,"ambiguous_center_seconds":a.ambiguous,"unresolved_center_seconds":a.unresolved,
        "nonstandard_five_on_five_seconds":a.nonstandard,"censored_source_seconds":a.censored,"invalid_occupancy_seconds":a.invalid,"usage_tiebreak_seconds":a.usage_resolved,
        "center_assignment_share":if a.standard>0{Some(assigned as f64/a.standard as f64)}else{None},"lines":rows,
        "ambiguous_trios":serialized(&a.ambiguous_trios),"unresolved_trios":serialized(&a.unresolved_trios),
        "usage_resolved_trios":serialized(&a.usage_resolved_trios),
        "defense_pairs":serialized(&a.pairs),"five_on_four_units":serialized(&a.pp),"four_on_five_units":serialized(&a.pk)})
}
fn read(path: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn latest_ids(expected: &[u64], count: usize) -> BTreeSet<u64> {
    expected.iter().rev().take(count).copied().collect()
}

pub fn build(
    mut captures: Vec<GameCapture>,
    faceoff_file: &Path,
    team_file: &Path,
    toi_file: Option<&Path>,
) -> Result<Value> {
    let raw_faces = index(rows(faceoff_file)?, "playerId")?;
    let ev_toi = match toi_file {
        Some(p) => Some(index(rows(p)?, "playerId")?),
        None => None,
    };
    if let Some(toi) = &ev_toi {
        if raw_faces.keys().ne(toi.keys()) {
            return Err("Faceoff/EV TOI player coverage mismatch".into());
        }
    }
    let mut faces = BTreeMap::new();
    let mut names = BTreeMap::new();
    for (id, f) in raw_faces {
        let gp = number(&f, "gamesPlayed")?;
        if gp <= 0.0 {
            return Err("No faceoff reference games".into());
        }
        faces.insert(
            id,
            Faceoffs {
                ev_per_game: number(&f, "evFaceoffs")? / gp,
                ev_seconds_per_game: match &ev_toi {
                    Some(t) => number(&t[&id], "evTimeOnIcePerGame")?,
                    None => 0.0,
                },
            },
        );
        names.insert(
            id,
            f["skaterFullName"]
                .as_str()
                .ok_or("Missing player name")?
                .into(),
        );
    }
    let teams = index(rows(team_file)?, "teamId")?;
    let mut observed = BTreeMap::<u64, Vec<(u64, String, Value)>>::new();
    let mut failures = Vec::new();
    let mut ids = BTreeSet::new();
    captures.sort_by(|a, b| {
        a.game_date
            .cmp(&b.game_date)
            .then_with(|| a.game_id.cmp(&b.game_id))
    });
    for g in &captures {
        if !ids.insert(g.game_id) {
            return Err("Duplicate game capture".into());
        }
        let loaded = (|| -> Result<(Value, Value)> {
            let b = read(g.box_path.as_deref().ok_or("Missing box capture")?)?;
            let s = read(g.shifts_path.as_deref().ok_or("Missing shifts")?)?;
            if b["id"].as_u64() != Some(g.game_id)
                || b["gameDate"].as_str() != Some(&g.game_date)
                || b["homeTeam"]["id"].as_u64() != Some(g.home_id)
                || b["awayTeam"]["id"].as_u64() != Some(g.away_id)
            {
                return Err("Source game metadata mismatch".into());
            }
            Ok((b, s))
        })();
        for team in [g.home_id, g.away_id] {
            if !teams.contains_key(&team) {
                return Err("Unexpected team".into());
            }
            let result = match &loaded {
                Ok((b, s)) => reconstruct_complete(b, s, team),
                Err(e) => Err(e.to_string().into()),
            };
            match result{Ok(u)=>observed.entry(team).or_default().push((g.game_id,g.game_date.clone(),u)),Err(e)=>failures.push(json!({"team_id":team,"game_id":g.game_id,"date":g.game_date,"error":e.to_string()}))}
        }
    }
    let mut out = Vec::new();
    for (id, team) in teams {
        let games = observed.remove(&id).unwrap_or_default();
        let season = aggregate(&games, &faces);
        let expected: Vec<_> = captures
            .iter()
            .filter(|g| g.home_id == id || g.away_id == id)
            .collect();
        if expected.len() as f64 != number(&team, "gamesPlayed")? {
            return Err("Schedule/statistical team GP mismatch".into());
        }
        let expected_ids: Vec<_> = expected.iter().map(|g| g.game_id).collect();
        let mut windows = Vec::new();
        for (name, count) in [("season", usize::MAX), ("last5", 5), ("latest", 1)] {
            let selected = latest_ids(&expected_ids, count);
            let observed: Vec<_> = games
                .iter()
                .filter(|g| selected.contains(&g.0))
                .cloned()
                .collect();
            let mut w = window(name, &observed, &season, &faces, &names);
            w["expected_games"] = json!(selected.len());
            w["last_expected_date"] = json!(expected.last().map(|g| &g.game_date));
            windows.push(w);
        }
        out.push(json!({"team_id":id,"team":team["teamFullName"],"expected_games":team["gamesPlayed"],"resolved_games":games.len(),"windows":windows}));
    }
    Ok(
        json!({"schema":"icelines.center-lines.0.1-experimental","games_in_manifest":captures.len(),"teams":out,"failures":failures,
        "method":{"center_candidate":"prefer box C with >=1 EV draw/game; only if none, fallback forward with >=3; counts, not win percentage","multiple_center_ambiguity_ratio":0.75,
            "main_line_minimum_seconds_per_anchor_game":120,
            "comparable_center_tiebreak":"if EV TOI supplied, prefer greater season EV seconds/game; resolved time/trios explicitly marked, not coach confirmation",
            "ordering":"season inferred centered-5v5 seconds per anchor game; >=3 anchor games prioritized, sparse samples explicit; active-window centers retain season ordering",
            "trios":"joint observed combinations, not independently chosen wingers; all minutes retained in aggregate"},
        "limits":["Inferred usage-based line labels, not dated coach assignments","Regulation only; source-quality exclusions retained",
            "Season faceoff workload is a center proxy, not proof of the slot on every shift","Ambiguous and unresolved center time is not assigned",
            "Injuries/trades and true role promotions need dated overrides; season/last5/latest remain separate","PP/PK roles separate from line ranking"]}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_center_relief_does_not_displace_substantive_line() {
        let f = BTreeMap::from([(1, evidence(13.0)), (2, evidence(12.0))]);
        let mut short = trio(&[(1, "C"), (7, "L"), (8, "R")]);
        short["seconds"] = json!(15);
        let mut long = trio(&[(2, "C"), (9, "L"), (10, "R")]);
        long["seconds"] = json!(600);
        let games = vec![(
            1,
            "2000-01-01".into(),
            json!({"five_on_five_trios":[short,long],"state_seconds":{"5v5":615}}),
        )];
        let mut season = aggregate(&games, &f);
        season.centers.get_mut(&1).unwrap().seconds = 12000;
        let w = window("latest", &games, &season, &f, &BTreeMap::new());
        assert_eq!(w["lines"][0]["center_id"], 2);
        assert_eq!(w["lines"][1]["label"], "incidental / relief center");
    }
    fn evidence(ev_per_game: f64) -> Faceoffs {
        Faceoffs {
            ev_per_game,
            ..Default::default()
        }
    }
    #[test]
    fn comparable_centers_can_use_explicit_ev_toi_tiebreak() {
        let f = BTreeMap::from([
            (
                1,
                Faceoffs {
                    ev_per_game: 9.0,
                    ev_seconds_per_game: 920.0,
                },
            ),
            (
                2,
                Faceoffs {
                    ev_per_game: 7.2,
                    ev_seconds_per_game: 810.0,
                },
            ),
        ]);
        assert!(matches!(
            choose(&trio(&[(1, "C"), (2, "C"), (3, "R")]), &f),
            Anchor::UsageCenter(1)
        ));
    }
    #[test]
    fn one_listed_center_takes_precedence_over_high_draw_winger() {
        let f = BTreeMap::from([(1, evidence(13.0)), (2, evidence(12.0))]);
        assert!(matches!(
            choose(&trio(&[(1, "C"), (2, "R"), (3, "L")]), &f),
            Anchor::Center(1)
        ));
    }
    #[test]
    fn every_alternate_trio_is_retained() {
        let f = BTreeMap::from([(1, evidence(13.0))]);
        let games:Vec<_>=(10..15).map(|i|(i,"2000-01-01".into(),json!({"five_on_five_trios":[trio(&[(1,"C"),(i,"L"),(99,"R")])],"state_seconds":{"5v5":120}}))).collect();
        let a = aggregate(&games, &f);
        let w = window("season", &games, &a, &f, &BTreeMap::new());
        assert_eq!(w["lines"][0]["trios"].as_array().unwrap().len(), 5);
    }
    #[test]
    fn missing_latest_game_does_not_pull_an_older_game_into_window() {
        let expected = latest_ids(&[1, 2, 3, 4, 5, 6], 1);
        let resolved = [1, 2, 3, 4, 5];
        assert_eq!(resolved.iter().filter(|g| expected.contains(g)).count(), 0);
        let last5 = latest_ids(&[1, 2, 3, 4, 5, 6], 5);
        assert!(!last5.contains(&1));
        assert!(last5.contains(&6));
    }
    fn trio(codes: &[(u64, &str)]) -> Value {
        json!({"seconds":120,"members":codes.iter().map(|(id,p)|json!({"id":id,"position":p})).collect::<Vec<_>>()})
    }
    #[test]
    fn listed_center_playing_wing_does_not_displace_actual_draw_taker() {
        let f = BTreeMap::from([(1, evidence(0.125)), (2, evidence(13.0))]);
        assert!(matches!(
            choose(&trio(&[(1, "C"), (2, "C"), (3, "R")]), &f),
            Anchor::Center(2)
        ));
    }
    #[test]
    fn comparable_centers_remain_ambiguous() {
        let f = BTreeMap::from([(1, evidence(13.0)), (2, evidence(12.0))]);
        assert!(matches!(
            choose(&trio(&[(1, "C"), (2, "C"), (3, "R")]), &f),
            Anchor::Ambiguous
        ));
    }
    #[test]
    fn no_supported_center_is_not_fabricated() {
        assert!(matches!(
            choose(&trio(&[(1, "L"), (2, "L"), (3, "R")]), &BTreeMap::new()),
            Anchor::Unresolved
        ));
    }
    #[test]
    fn season_evidence_keeps_single_game_callup_below_established_center() {
        let mut c = BTreeMap::new();
        c.insert(
            1,
            Center {
                seconds: 1800,
                games: BTreeSet::from([1]),
                ..Default::default()
            },
        );
        c.insert(
            2,
            Center {
                seconds: 2400,
                games: BTreeSet::from([1, 2, 3]),
                ..Default::default()
            },
        );
        assert_eq!(established(&c), vec![2, 1]);
    }
    #[test]
    fn coverage_partitions_assigned_ambiguous_and_unresolved_time() {
        let f = BTreeMap::from([(1, evidence(13.0)), (2, evidence(12.0))]);
        let u = json!({"five_on_five_trios":[trio(&[(1,"C"),(4,"R"),(5,"L")]),trio(&[(1,"C"),(2,"C"),(5,"L")]),trio(&[(3,"L"),(4,"R"),(5,"L")])],"state_seconds":{"5v5":360}});
        let a = aggregate(&[(1, "2000-01-01".into(), u)], &f);
        assert_eq!(a.standard, 360);
        assert_eq!(
            a.centers.values().map(|c| c.seconds).sum::<u64>() + a.ambiguous + a.unresolved,
            a.standard
        );
    }
}
