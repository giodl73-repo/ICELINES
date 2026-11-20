//! Linked personnel changes between adjacent supplied team games, not coach decisions.
use crate::{
    center_lines::GameCapture, deployment_history::candidate, observed_units::reconstruct_complete,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Separate descriptive summary; does not alter the canonical team history.
pub fn summarize(report: &Value) -> Result<Value> {
    if report["schema"] != "icelines.team-deployment-history.0.1-experimental" {
        return Err("Unsupported team history schema".into());
    }
    let days = report["days"]
        .as_array()
        .ok_or("Missing team history days")?;
    let mut output = json!({"schema":"icelines.team-deployment-summary.0.1-experimental","team_id":report["team_id"],
        "expected_games":report["expected_games"],"resolved_games":report["resolved_games"],
        "first_date":days.first().map(|d|&d["date"]),"last_date":days.last().map(|d|&d["date"])});
    for (players, changes, out) in [
        ("forwards", "forward_changes", "forward"),
        ("defense", "defense_changes", "defense"),
    ] {
        let mut compared = 0u64;
        let mut changed = 0u64;
        let mut adjacent = 0u64;
        let mut unchanged = 0u64;
        let mut standard = 0u64;
        let mut primary_seconds = 0u64;
        let mut roster_games = 0u64;
        let mut ties = 0u64;
        let mut max_streak = 0u64;
        let mut longest = Vec::new();
        let mut entries = 0usize;
        let mut exits = 0usize;
        let mut face_games = 0u64;
        for day in days {
            let c = &day[changes];
            if c["status"] == "comparable" {
                let n = c["comparable_continuing_players"]
                    .as_u64()
                    .ok_or("Missing comparison denominator")?;
                let k = c["changed_players"]
                    .as_u64()
                    .ok_or("Missing changed count")?;
                if k > n {
                    return Err("Changed players exceed comparable players".into());
                }
                compared += n;
                changed += k;
                // Zero-comparable games are not classified as stable.
                if n > 0 {
                    adjacent += 1;
                    if k == 0 {
                        unchanged += 1;
                    }
                }
                entries += c["newly_box_listed"].as_array().map_or(0, Vec::len);
                exits += c["no_longer_box_listed"].as_array().map_or(0, Vec::len);
            }
            if day["status"] != "ok" {
                continue;
            }
            for p in day[players].as_array().ok_or("Missing player rows")? {
                roster_games += 1;
                if p["primary_tied"] == true {
                    ties += 1;
                }
                if p["game_ev_faceoffs"].as_u64().is_some() {
                    face_games += 1;
                }
                let s = p["standard_unit_seconds"]
                    .as_u64()
                    .ok_or("Missing standard exposure")?;
                let top = p["primary"]["seconds"].as_u64().unwrap_or(0);
                if top > s {
                    return Err("Primary exceeds standard exposure".into());
                }
                standard += s;
                primary_seconds += top;
                if let Some(streak) = p["consecutive_primary_games"].as_u64() {
                    if streak > max_streak {
                        max_streak = streak;
                        longest.clear();
                    }
                    if streak == max_streak {
                        longest.push(json!({"player_id":p["id"],"name":p["name"],"end_date":day["date"],"primary":p["primary"]}));
                    }
                }
            }
        }
        output[out] = json!({"changed_player_comparisons":changed,"comparable_player_comparisons":compared,
            "primary_change_fraction":(compared>0).then(||changed as f64/compared as f64),
            "adjacent_games_with_comparable_players":adjacent,"adjacent_games_without_primary_change":unchanged,
            "box_player_games":roster_games,"tied_primary_player_games":ties,"game_faceoff_player_games_available":face_games,
            "box_entries_across_comparable_games":entries,"box_exits_across_comparable_games":exits,
            "standard_unit_player_seconds":standard,"most_used_unit_player_seconds":primary_seconds,
            "exposure_weighted_primary_concentration":(standard>0).then(||primary_seconds as f64/standard as f64),
            "longest_trailing_primary_run_games":max_streak,"longest_run_records":longest});
    }
    output["limits"]=json!(["Descriptive selected-window deployment, not team quality, coaching intent or chemistry",
        "Change fraction counts dependent per-player adjacent-game comparisons, not independent line changes or an accuracy probability",
        "Forward trios and defense pairs have different combinatorial opportunities; do not interpret a direct position comparison as quality",
        "Concentration uses standard-unit exposure; pair/trio player seconds count two/three participants, not elapsed team time",
        "Primary ties remain in concentration as equal maximal exposure but are excluded from unambiguous change and continuity",
        "Availability, injuries, opponents, score and tactical systems are not adjusted; no significance or causal claims",
        "Game faceoff coverage is separate and may differ by team; missing counts are not zero draws",
        "Longest run is trailing through observed dates, not exclusive assignment or future persistence"]);
    Ok(output)
}
fn read(path: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
fn ids(unit: &Value) -> Option<Vec<u64>> {
    let mut ids: Vec<_> = unit["members"]
        .as_array()?
        .iter()
        .filter_map(|m| m["id"].as_u64())
        .collect();
    ids.sort_unstable();
    Some(ids)
}
fn primary(player: &Value) -> Option<Vec<u64>> {
    if player["primary_tied"] == true {
        return None;
    }
    ids(&player["primary"])
}
fn annotate_streaks(day: &mut Value, previous: Option<&Value>) {
    if day["status"] != "ok" {
        return;
    }
    let prev = previous.filter(|p| {
        p["status"] == "ok" && p["season"] == day["season"] && p["game_type"] == day["game_type"]
    });
    for field in ["forwards", "defense"] {
        if let Some(rows) = day[field].as_array_mut() {
            for p in rows {
                let count = primary(p).map(|current| {
                    let old = prev
                        .and_then(|d| d[field].as_array())
                        .and_then(|r| r.iter().find(|r| r["id"] == p["id"]));
                    old.filter(|r| primary(r).as_ref() == Some(&current))
                        .and_then(|r| r["consecutive_primary_games"].as_u64())
                        .unwrap_or(0)
                        + 1
                });
                p["consecutive_primary_games"] = json!(count);
            }
        }
    }
}
fn player_rows(units: &Value, roster: &[Value], field: &str) -> Vec<Value> {
    roster.iter().map(|p|{
        let pid=p["playerId"].as_u64().unwrap();
        let rows:Vec<_>=units[field].as_array().unwrap().iter().filter(|u|u["members"].as_array().unwrap().iter().any(|m|m["id"].as_u64()==Some(pid))).collect();
        let seconds:u64=rows.iter().map(|r|r["seconds"].as_u64().unwrap()).sum();
        let no_c:u64=rows.iter().filter(|r|!r["members"].as_array().unwrap().iter().any(|m|m["position"]=="C")).map(|r|r["seconds"].as_u64().unwrap()).sum();
        json!({"id":pid,"name":p["name"]["default"],"listed_position":p["position"],"standard_unit_seconds":seconds,
            "primary":rows.first(),"primary_tied":rows.len()>1 && rows[0]["seconds"]==rows[1]["seconds"],
            "primary_share_of_standard_unit_time":(seconds>0).then(||rows[0]["seconds"].as_f64().unwrap()/seconds as f64),
            "no_listed_center_trio_seconds":if field=="five_on_five_trios"{json!(no_c)}else{Value::Null}})
    }).collect()
}
/// Connected sets of changed players whose old/new units share any participant.
/// These are evidence groups, not disjoint actual lines or inferred causal moves.
fn groups(changes: &[Value]) -> Vec<Value> {
    let mut components: Vec<(BTreeSet<u64>, Vec<Value>)> = Vec::new();
    for change in changes {
        let mut members: BTreeSet<_> = ids(&change["before"])
            .unwrap()
            .into_iter()
            .chain(ids(&change["after"]).unwrap())
            .collect();
        let mut rows = vec![change.clone()];
        let mut i = 0;
        while i < components.len() {
            if !members.is_disjoint(&components[i].0) {
                let (other, mut r) = components.remove(i);
                members.extend(other);
                rows.append(&mut r);
                i = 0;
            } else {
                i += 1;
            }
        }
        rows.sort_by_key(|r| r["player_id"].as_u64());
        components.push((members, rows));
    }
    components.sort_by(|a, b| a.0.cmp(&b.0));
    components.into_iter().map(|(members,rows)|{
        let unique:BTreeSet<_>=rows.iter().map(|r|(ids(&r["before"]).unwrap(),ids(&r["after"]).unwrap())).collect();
        json!({"participant_ids":members,"changed_player_count":rows.len(),"distinct_primary_unit_transitions":unique.len(),"changes":rows})
    }).collect()
}
fn compare(before: &Value, after: &Value, field: &str) -> Value {
    if before["status"] != "ok" || after["status"] != "ok" {
        return json!({"status":"not_comparable_missing_game"});
    }
    if before["season"] != after["season"] || before["game_type"] != after["game_type"] {
        return json!({"status":"context_boundary"});
    }
    let old: BTreeMap<_, _> = before[field]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["id"].as_u64().unwrap(), p))
        .collect();
    let new: BTreeMap<_, _> = after[field]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["id"].as_u64().unwrap(), p))
        .collect();
    let mut changes = Vec::new();
    let mut comparable = 0;
    let mut uncomparable = Vec::new();
    for (pid, p) in &new {
        if let Some(prev) = old.get(pid) {
            match (primary(prev), primary(p)) {
                (Some(a), Some(b)) => {
                    comparable += 1;
                    if a != b {
                        changes.push(json!({"player_id":pid,"name":p["name"],"before":prev["primary"],"after":p["primary"],
                        "before_share":prev["primary_share_of_standard_unit_time"],"after_share":p["primary_share_of_standard_unit_time"]}));
                    }
                }
                _ => uncomparable.push(
                    json!({"id":pid,"name":p["name"],"reason":"tied_or_missing_standard_primary"}),
                ),
            }
        }
    }
    let entered: Vec<_> = new
        .iter()
        .filter(|(id, _)| !old.contains_key(id))
        .map(|(_, p)| (*p).clone())
        .collect();
    let left: Vec<_> = old
        .iter()
        .filter(|(id, _)| !new.contains_key(id))
        .map(|(_, p)| (*p).clone())
        .collect();
    json!({"status":"comparable","previous_game_id":before["game_id"],"comparable_continuing_players":comparable,
        "changed_players":changes.len(),"linked_change_groups":groups(&changes),"uncomparable_continuing_players":uncomparable,
        "newly_box_listed":entered,"no_longer_box_listed":left})
}
pub fn build(mut captures: Vec<GameCapture>, team: u64, faceoffs: Option<&Path>) -> Result<Value> {
    let mut faces = BTreeMap::new();
    if let Some(path) = faceoffs {
        let raw = read(path.to_str().ok_or("Non UTF-8 faceoff path")?)?;
        let rows = raw["data"].as_array().ok_or("Missing game faceoff data")?;
        if raw["total"]
            .as_u64()
            .is_some_and(|n| n != rows.len() as u64)
        {
            return Err("Incomplete game faceoff capture".into());
        }
        for r in rows {
            let key = (
                r["gameId"].as_u64().ok_or("Need game-level faceoffs")?,
                r["playerId"].as_u64().ok_or("Missing faceoff player")?,
            );
            let record = (
                r["gameDate"]
                    .as_str()
                    .ok_or("Missing faceoff date")?
                    .to_owned(),
                r["evFaceoffs"].as_u64().ok_or("Missing EV faceoffs")?,
            );
            if faces.insert(key, record).is_some() {
                return Err("Duplicate player/game faceoffs".into());
            }
        }
    }
    captures.retain(|g| g.home_id == team || g.away_id == team);
    captures.sort_by(|a, b| {
        a.game_date
            .cmp(&b.game_date)
            .then(a.game_id.cmp(&b.game_id))
    });
    let mut seen = BTreeSet::new();
    let mut days = Vec::new();
    for g in captures {
        if !seen.insert(g.game_id) {
            return Err("Duplicate team game".into());
        }
        let result = (|| -> Result<Value> {
            let b = read(g.box_path.as_deref().ok_or("Missing box capture")?)?;
            let s = read(g.shifts_path.as_deref().ok_or("Missing shift capture")?)?;
            if b["id"].as_u64() != Some(g.game_id)
                || b["gameDate"].as_str() != Some(&g.game_date)
                || b["homeTeam"]["id"].as_u64() != Some(g.home_id)
                || b["awayTeam"]["id"].as_u64() != Some(g.away_id)
            {
                return Err("Game metadata mismatch".into());
            }
            let units = reconstruct_complete(&b, &s, team)?;
            let season = b["season"].as_u64().ok_or("Missing source season")?;
            let game_type = b["gameType"].as_u64().ok_or("Missing source game type")?;
            let side = if g.home_id == team {
                "homeTeam"
            } else {
                "awayTeam"
            };
            let roster = &b["playerByGameStats"][side];
            let mut forwards = player_rows(
                &units,
                roster["forwards"].as_array().unwrap(),
                "five_on_five_trios",
            );
            for p in &mut forwards {
                let draws = match faces.get(&(g.game_id, p["id"].as_u64().unwrap())) {
                    Some((date, n)) => {
                        if date != &g.game_date {
                            return Err("Faceoff date mismatch".into());
                        }
                        Some(*n)
                    }
                    None => None,
                };
                p["game_ev_faceoffs"] = json!(draws);
                p["center_review_flag"] = json!(candidate(
                    p["listed_position"].as_str(),
                    draws,
                    p["no_listed_center_trio_seconds"].as_u64().unwrap()
                ));
            }
            Ok(
                json!({"status":"ok","game_id":g.game_id,"date":g.game_date,"season":season,"game_type":game_type,
                "forwards":forwards,"defense":player_rows(&units,roster["defense"].as_array().unwrap(),"five_on_five_pairs"),
                "goalie_box_records":roster["goalies"].as_array().unwrap().iter().map(|p|json!({"id":p["playerId"],"name":p["name"]["default"],"box_toi_including_overtime":p["toi"]})).collect::<Vec<_>>(),
                "units":units}),
            )
        })();
        let mut day = match result {
            Ok(d) => d,
            Err(e) => {
                json!({"status":"missing_or_invalid_evidence","game_id":g.game_id,"date":g.game_date,"error":e.to_string()})
            }
        };
        annotate_streaks(&mut day, days.last());
        for (field, out) in [
            ("forwards", "forward_changes"),
            ("defense", "defense_changes"),
        ] {
            day[out] = if let Some(prev) = days.last() {
                compare(prev, &day, field)
            } else {
                json!({"status":"first_supplied_game"})
            };
        }
        days.push(day);
    }
    let resolved = days.iter().filter(|d| d["status"] == "ok").count();
    Ok(
        json!({"schema":"icelines.team-deployment-history.0.1-experimental","team_id":team,"expected_games":days.len(),"resolved_games":resolved,
        "limits":["Adjacent supplied team games; not a complete schedule unless manifest is complete",
            "Primary is each player's most-used standard regulation 5v5 trio/pair; choices need not form disjoint mutual units",
            "Primary shares divide by standard-unit time, unlike player-history shares over all player 5v5 time",
            "Linked groups join changes sharing any old/new participant; they are not independent events, simultaneous changes or causal decisions",
            "No longer/newly box-listed does not establish injury, scratch, transaction or reason",
            "Missing games and season/game-type boundaries break comparisons; ties stay unresolved",
            "Possible-center flags reuse exploratory 120-second/three-EV-draw review gates, not confirmed duties",
            "Draws include all EV strengths/overtime; overlaps are regulation-only; no coach intent, chemistry, capability or PP-station inference",
            "Complete source units retain D pairs and special teams; goalie box TOI is separate and includes overtime"],"days":days}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unit(ids: &[u64]) -> Value {
        json!({"seconds":100,"members":ids.iter().map(|id|json!({"id":id})).collect::<Vec<_>>()})
    }
    fn change(pid: u64, a: &[u64], b: &[u64]) -> Value {
        json!({"player_id":pid,"before":unit(a),"after":unit(b)})
    }
    fn day(ids: &[u64]) -> Value {
        json!({"status":"ok","game_id":1,"season":1,"game_type":2,"forwards":[{"id":1,"name":"A","primary":unit(ids),"primary_tied":false}]})
    }
    #[test]
    fn transitive_changes_group_and_deduplicate_unit_edges() {
        let c = vec![
            change(1, &[1, 2, 3], &[1, 2, 4]),
            change(2, &[1, 2, 3], &[1, 2, 4]),
            change(5, &[5, 6, 7], &[3, 5, 6]),
            change(9, &[9, 10, 11], &[9, 10, 12]),
        ];
        let g = groups(&c);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0]["changed_player_count"], 3);
        assert_eq!(g[0]["distinct_primary_unit_transitions"], 2);
        let mut reversed = c.clone();
        reversed.reverse();
        assert_eq!(g, groups(&reversed));
    }
    #[test]
    fn ties_and_missing_break_comparison() {
        let a = day(&[1, 2, 3]);
        let mut b = day(&[1, 2, 4]);
        b["forwards"][0]["primary_tied"] = json!(true);
        assert_eq!(
            compare(&a, &b, "forwards")["comparable_continuing_players"],
            0
        );
        b["status"] = json!("missing_or_invalid_evidence");
        assert_eq!(
            compare(&a, &b, "forwards")["status"],
            "not_comparable_missing_game"
        );
    }
    #[test]
    fn stable_primary_is_not_change_and_roster_absence_is_separate() {
        let a = day(&[1, 2, 3]);
        assert_eq!(compare(&a, &a, "forwards")["changed_players"], 0);
        let mut b = a.clone();
        b["forwards"][0]["id"] = json!(2);
        let r = compare(&a, &b, "forwards");
        assert_eq!(r["changed_players"], 0);
        assert_eq!(r["newly_box_listed"][0]["id"], 2);
        assert_eq!(r["no_longer_box_listed"][0]["id"], 1);
    }
    #[test]
    fn season_and_game_type_are_boundaries() {
        let a = day(&[1, 2, 3]);
        for field in ["season", "game_type"] {
            let mut b = a.clone();
            b[field] = json!(9);
            assert_eq!(compare(&a, &b, "forwards")["status"], "context_boundary");
        }
    }
    #[test]
    fn no_standard_exposure_has_no_primary_or_invented_share() {
        let u = json!({"five_on_five_trios":[]});
        let roster = vec![json!({"playerId":1,"position":"L","name":{"default":"A"}})];
        let p = player_rows(&u, &roster, "five_on_five_trios");
        assert!(p[0]["primary"].is_null());
        assert!(p[0]["primary_share_of_standard_unit_time"].is_null());
    }
    #[test]
    fn changes_retain_before_and_after_exposure() {
        let a = day(&[1, 2, 3]);
        let b = day(&[1, 2, 4]);
        let r = compare(&a, &b, "forwards");
        assert_eq!(r["changed_players"], 1);
        assert_eq!(
            r["linked_change_groups"][0]["changes"][0]["before"]["seconds"],
            100
        );
    }
    #[test]
    fn streaks_are_trailing_and_break_on_ties_missing_and_context() {
        let mut a = day(&[1, 2, 3]);
        annotate_streaks(&mut a, None);
        assert_eq!(a["forwards"][0]["consecutive_primary_games"], 1);
        let mut b = day(&[1, 2, 3]);
        annotate_streaks(&mut b, Some(&a));
        assert_eq!(b["forwards"][0]["consecutive_primary_games"], 2);
        let mut changed = day(&[1, 2, 4]);
        annotate_streaks(&mut changed, Some(&b));
        assert_eq!(changed["forwards"][0]["consecutive_primary_games"], 1);
        let mut tied = day(&[1, 2, 3]);
        tied["forwards"][0]["primary_tied"] = json!(true);
        annotate_streaks(&mut tied, Some(&b));
        assert!(tied["forwards"][0]["consecutive_primary_games"].is_null());
        let mut after = day(&[1, 2, 3]);
        annotate_streaks(&mut after, Some(&json!({"status":"missing"})));
        assert_eq!(after["forwards"][0]["consecutive_primary_games"], 1);
        after["season"] = json!(9);
        annotate_streaks(&mut after, Some(&b));
        assert_eq!(after["forwards"][0]["consecutive_primary_games"], 1);
    }
    fn summary_fixture() -> Value {
        let mut d = day(&[1, 2, 3]);
        d["date"] = json!("2000-01-01");
        d["defense"] = json!([]);
        d["forwards"][0]["standard_unit_seconds"] = json!(200);
        d["forwards"][0]["consecutive_primary_games"] = json!(1);
        d["forward_changes"] =
            json!({"status":"comparable","comparable_continuing_players":2,"changed_players":1});
        d["defense_changes"] = json!({"status":"first_supplied_game"});
        json!({"schema":"icelines.team-deployment-history.0.1-experimental","team_id":1,"expected_games":1,"resolved_games":1,"days":[d]})
    }
    #[test]
    fn summaries_use_explicit_dependent_counts_and_standard_exposure() {
        let r = summarize(&summary_fixture()).unwrap();
        assert_eq!(r["forward"]["primary_change_fraction"], 0.5);
        assert_eq!(r["forward"]["exposure_weighted_primary_concentration"], 0.5);
        assert_eq!(r["forward"]["game_faceoff_player_games_available"], 0);
        assert_eq!(r["forward"]["longest_trailing_primary_run_games"], 1);
    }
    #[test]
    fn zero_comparisons_are_unknown_not_perfect_stability() {
        let mut f = summary_fixture();
        f["days"][0]["forward_changes"]["comparable_continuing_players"] = json!(0);
        f["days"][0]["forward_changes"]["changed_players"] = json!(0);
        let r = summarize(&f).unwrap();
        assert!(r["forward"]["primary_change_fraction"].is_null());
        assert_eq!(r["forward"]["adjacent_games_without_primary_change"], 0);
        assert!(r["defense"]["exposure_weighted_primary_concentration"].is_null());
    }
    #[test]
    fn invalid_schema_and_impossible_counts_are_rejected() {
        assert!(summarize(&json!({"schema":"other"})).is_err());
        let mut f = summary_fixture();
        f["days"][0]["forward_changes"]["changed_players"] = json!(3);
        assert!(summarize(&f).is_err());
        let mut f = summary_fixture();
        f["days"][0]["forwards"][0]["standard_unit_seconds"] = json!(99);
        assert!(summarize(&f).is_err());
    }
}
