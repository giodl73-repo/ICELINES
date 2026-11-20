//! Descriptive deployment changes, not coaching intent, chemistry or player grades.
use crate::player_line_history::{self, Appearance};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn same_context(a: &Value, b: &Value) -> bool {
    ["teamId", "season", "gameTypeId"]
        .iter()
        .all(|k| a["appearance"][k] == b["appearance"][k])
}
fn partners(day: &Value, player: u64) -> BTreeMap<u64, (String, u64)> {
    let mut out = BTreeMap::new();
    if let Some(trios) = day["units"]["five_on_five_trios"].as_array() {
        for trio in trios {
            for m in trio["members"].as_array().unwrap() {
                let id = m["id"].as_u64().unwrap();
                if id != player {
                    let p = out
                        .entry(id)
                        .or_insert((m["name"].as_str().unwrap().to_owned(), 0));
                    p.1 += trio["seconds"].as_u64().unwrap();
                }
            }
        }
    }
    out
}
fn trio_ids(day: &Value) -> Option<Vec<u64>> {
    if day["status"] != "ok" || day["primary_trio_tied"] == true {
        return None;
    }
    let mut ids: Vec<_> = day["primary_trio"]["members"]
        .as_array()?
        .iter()
        .filter_map(|m| m["id"].as_u64())
        .collect();
    ids.sort_unstable();
    (ids.len() == 3).then_some(ids)
}
pub(crate) fn candidate(
    position: Option<&str>,
    draws: Option<u64>,
    no_center: u64,
) -> &'static str {
    if !matches!(position, Some("L" | "R")) {
        return "not_a_winger_transition_test";
    }
    if no_center >= 120 && draws.is_some_and(|n| n >= 3) {
        "review_possible_center_stint"
    } else if no_center >= 120 {
        "review_no_listed_center_exposure"
    } else {
        "no_threshold_flag"
    }
}

fn window(days: &[Value], player: u64) -> Value {
    let valid: Vec<_> = days.iter().filter(|d| d["status"] == "ok").collect();
    let mut pp = 0;
    let mut ev = 0;
    let mut no_c = 0;
    let mut draws = 0;
    let mut face_games = 0;
    let mut partners_all = BTreeMap::<u64, (String, u64)>::new();
    let mut trios = BTreeMap::<Vec<u64>, (Value, u64)>::new();
    for d in &valid {
        ev += d["focused_five_on_five_seconds"].as_u64().unwrap_or(0);
        pp += d["units"]["focused_state_seconds"]["5v4"]
            .as_u64()
            .unwrap_or(0);
        no_c += d["deployment"]["no_listed_center_trio_seconds"]
            .as_u64()
            .unwrap_or(0);
        if let Some(n) = d["deployment"]["game_ev_faceoffs"].as_u64() {
            draws += n;
            face_games += 1;
        }
        for (id, (name, seconds)) in partners(d, player) {
            partners_all.entry(id).or_insert((name, 0)).1 += seconds;
        }
        for t in d["units"]["five_on_five_trios"].as_array().unwrap() {
            let ids = t["members"]
                .as_array()
                .unwrap()
                .iter()
                .map(|m| m["id"].as_u64().unwrap())
                .collect();
            trios.entry(ids).or_insert((t["members"].clone(), 0)).1 +=
                t["seconds"].as_u64().unwrap();
        }
    }
    let mut p: Vec<_> = partners_all
        .into_iter()
        .map(|(id, (name, seconds))| json!({"id":id,"name":name,"seconds":seconds}))
        .collect();
    p.sort_by_key(|r| {
        (
            std::cmp::Reverse(r["seconds"].as_u64().unwrap()),
            r["id"].as_u64().unwrap(),
        )
    });
    let mut t: Vec<_> = trios
        .into_values()
        .map(|(members, seconds)| json!({"members":members,"seconds":seconds}))
        .collect();
    t.sort_by(|a, b| {
        b["seconds"]
            .as_u64()
            .cmp(&a["seconds"].as_u64())
            .then_with(|| a["members"].to_string().cmp(&b["members"].to_string()))
    });
    let n = valid.len();
    json!({"appearances":days.len(),"resolved_games":n,"first_date":days.first().map(|d|&d["appearance"]["gameDate"]),
        "last_date":days.last().map(|d|&d["appearance"]["gameDate"]),"five_on_five_seconds":ev,"five_on_four_seconds":pp,
        "five_on_five_seconds_per_resolved_game":(n>0).then(||ev as f64/n as f64),
        "five_on_four_seconds_per_resolved_game":(n>0).then(||pp as f64/n as f64),
        "no_listed_center_trio_seconds":no_c,"ev_faceoffs_in_available_games":draws,"faceoff_games_available":face_games,
        "partners":p,"trios":t})
}

/// Leaves the legacy appearance history unchanged; optional draws must be game-level.
pub fn build(appearances: Vec<Appearance>, player: u64, faceoffs: Option<&Path>) -> Result<Value> {
    let mut faces = BTreeMap::new();
    if let Some(path) = faceoffs {
        let raw: Value = serde_json::from_slice(&std::fs::read(path)?)?;
        let rows = raw["data"].as_array().ok_or("Missing game faceoff data")?;
        if raw["total"]
            .as_u64()
            .is_some_and(|n| n != rows.len() as u64)
        {
            return Err("Incomplete game faceoff capture".into());
        }
        for row in rows
            .iter()
            .filter(|r| r["playerId"].as_u64() == Some(player))
        {
            let game = row["gameId"]
                .as_u64()
                .ok_or("Faceoffs must be game-level")?;
            let date = row["gameDate"].as_str().ok_or("Missing faceoff date")?;
            let n = row["evFaceoffs"].as_u64().ok_or("Missing EV faceoffs")?;
            if faces.insert(game, (date.to_owned(), n)).is_some() {
                return Err("Duplicate player/game faceoffs".into());
            }
        }
    }
    let mut history = player_line_history::build(appearances, player)?;
    let days = history["days"].as_array_mut().unwrap();
    for i in 0..days.len() {
        if days[i]["status"] != "ok" {
            continue;
        }
        let day = &days[i];
        let a = &day["appearance"];
        let game = a["gameId"].as_u64().unwrap();
        let boxscore: Value =
            serde_json::from_slice(&std::fs::read(a["boxPath"].as_str().unwrap())?)?;
        let side = if boxscore["homeTeam"]["id"] == a["teamId"] {
            "homeTeam"
        } else {
            "awayTeam"
        };
        let stats = ["forwards", "defense", "goalies"]
            .iter()
            .filter_map(|g| boxscore["playerByGameStats"][side][g].as_array())
            .flatten()
            .find(|p| p["playerId"].as_u64() == Some(player))
            .ok_or("Missing player box record")?;
        let no_center: u64 = day["units"]["five_on_five_trios"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| {
                !t["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|m| m["position"] == "C")
            })
            .map(|t| t["seconds"].as_u64().unwrap())
            .sum();
        let draws = if let Some((date, n)) = faces.get(&game) {
            if a["gameDate"].as_str() != Some(date.as_str()) {
                return Err("Faceoff game/date mismatch".into());
            }
            Some(*n)
        } else {
            None
        };
        let partner_rows: Vec<_> = partners(day, player)
            .into_iter()
            .map(|(id, (name, seconds))| json!({"id":id,"name":name,"seconds":seconds}))
            .collect();
        let comparison = if i > 0 && same_context(&days[i - 1], day) {
            match (trio_ids(&days[i - 1]), trio_ids(day)) {
                (Some(old), Some(new)) => {
                    json!({"status":"comparable","previous_game_id":days[i-1]["appearance"]["gameId"],
                    "primary_trio_changed":old!=new,"shared_primary_members":old.iter().filter(|p|new.contains(p)).count()})
                }
                _ => json!({"status":"not_comparable_missing_or_tied_primary"}),
            }
        } else {
            json!({"status":"first_appearance_or_context_boundary"})
        };
        days[i]["deployment"] = json!({"listed_position":stats["position"],"game_ev_faceoffs":draws,
            "no_listed_center_trio_seconds":no_center,"center_review_flag":candidate(stats["position"].as_str(),draws,no_center),
            "partner_seconds":partner_rows,"previous_appearance_comparison":comparison,
            "box_record_all_strengths_including_overtime":{"toi":stats["toi"],"goals":stats["goals"],"assists":stats["assists"],"shots_on_goal":stats["sog"]},
            "individual_xg":null,"unit_xgf":null,"unit_xga":null});
        // Trailing appearances only, never season totals or future evidence.
        let mut context_start = i;
        while context_start > 0 && same_context(&days[context_start - 1], &days[i]) {
            context_start -= 1;
        }
        let recent_start = (i + 1).saturating_sub(5).max(context_start);
        let prior_start = recent_start.saturating_sub(5).max(context_start);
        let recent = window(&days[recent_start..=i], player);
        let prior = window(&days[prior_start..recent_start], player);
        let delta = |key: &str| {
            recent[key]
                .as_f64()
                .zip(prior[key].as_f64())
                .map(|(r, p)| r - p)
        };
        let change = json!({"five_on_five_seconds_per_game":delta("five_on_five_seconds_per_resolved_game"),
            "five_on_four_seconds_per_game":delta("five_on_four_seconds_per_resolved_game")});
        days[i]["deployment"]["trailing_five_appearances"] = recent;
        days[i]["deployment"]["preceding_five_appearances"] = prior;
        days[i]["deployment"]["opportunity_change"] = change;
    }
    let comparable = days
        .iter()
        .filter(|d| d["deployment"]["previous_appearance_comparison"]["status"] == "comparable")
        .count();
    let changed = days
        .iter()
        .filter(|d| {
            d["deployment"]["previous_appearance_comparison"]["primary_trio_changed"] == true
        })
        .count();
    let flags: Vec<_> = days.iter().filter(|d| d["deployment"]["center_review_flag"].as_str().is_some_and(|f|f.starts_with("review_")))
        .map(|d|json!({"game_id":d["appearance"]["gameId"],"date":d["appearance"]["gameDate"],"flag":d["deployment"]["center_review_flag"]})).collect();
    history["schema"] = json!("icelines.deployment-history.0.1-experimental");
    history["deployment_summary"] = json!({"comparable_adjacent_appearance_pairs":comparable,"primary_trio_changes":changed,"center_review_games":flags});
    history["deployment_policy"] = json!({"trailing_window_appearances":5,"winger_center_review_min_no_listed_center_seconds":120,
        "winger_center_review_min_game_ev_faceoffs":3,"threshold_status":"authored exploratory review gates, not validated NHL definitions",
        "limits":["No coaching intent, confirmed center assignment, promotion/demotion or causal chemistry inferred",
            "No listed center means game roster positions, not observed tactical duties; a listed C can play wing",
            "Game EV faceoffs include all EV strengths and overtime; overlap is regulation 5v5 only",
            "Taking faceoffs may be relief, handedness or ejection cover rather than playing center",
            "Trailing windows use supplied appearances, not calendar days or team games; missing rows stay in the window",
            "PP exposure is 5v4 only; no PP station or PP1/PP2 label inferred",
            "Box results include all strengths and overtime; no unit attribution or rate against regulation-only exposure",
            "Expected goals and opponent-adjusted unit results are unavailable in these inputs and stay null",
            "No flag is not evidence of no role change; thresholds can miss changes with a listed C or few restarts"]});
    Ok(history)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn winger_flags_need_both_signals_and_preserve_missing_draws() {
        assert_eq!(
            candidate(Some("L"), Some(3), 120),
            "review_possible_center_stint"
        );
        assert_eq!(
            candidate(Some("L"), Some(2), 120),
            "review_no_listed_center_exposure"
        );
        assert_eq!(
            candidate(Some("R"), None, 120),
            "review_no_listed_center_exposure"
        );
        assert_eq!(candidate(Some("L"), Some(12), 119), "no_threshold_flag");
        assert_eq!(
            candidate(Some("C"), Some(12), 800),
            "not_a_winger_transition_test"
        );
    }
    fn day(ids: &[u64], seconds: u64) -> Value {
        json!({"status":"ok","appearance":{"teamId":1,"season":1,"gameTypeId":2,"gameDate":"2000-01-01"},
            "focused_five_on_five_seconds":seconds,"primary_trio_tied":false,
            "primary_trio":{"members":ids.iter().map(|id|json!({"id":id})).collect::<Vec<_>>()},
            "units":{"five_on_five_trios":[{"seconds":seconds,"members":ids.iter().map(|id|json!({"id":id,"name":format!("P{id}")})).collect::<Vec<_>>()}]}})
    }
    #[test]
    fn partner_seconds_count_two_teammates_and_not_self() {
        let p = partners(&day(&[1, 2, 3], 100), 1);
        assert_eq!(p.values().map(|(_, s)| s).sum::<u64>(), 200);
        assert!(!p.contains_key(&1));
    }
    #[test]
    fn missing_and_tied_primaries_are_not_transitions() {
        let mut d = day(&[1, 2, 3], 100);
        d["primary_trio_tied"] = json!(true);
        assert!(trio_ids(&d).is_none());
        d["status"] = json!("missing_or_invalid_evidence");
        assert!(trio_ids(&d).is_none());
    }
    #[test]
    fn windows_keep_missing_exposure_out_of_the_denominator() {
        let w = window(
            &[
                day(&[1, 2, 3], 100),
                json!({"status":"missing_or_invalid_evidence"}),
            ],
            1,
        );
        assert_eq!(w["appearances"], 2);
        assert_eq!(w["resolved_games"], 1);
        assert_eq!(w["five_on_five_seconds_per_resolved_game"], 100.0);
        assert_eq!(w["faceoff_games_available"], 0);
        assert!(window(&[], 1)["five_on_five_seconds_per_resolved_game"].is_null());
    }
    #[test]
    fn changes_do_not_bridge_team_season_or_phase() {
        let a = day(&[1, 2, 3], 100);
        for k in ["teamId", "season", "gameTypeId"] {
            let mut b = a.clone();
            b["appearance"][k] = json!(99);
            assert!(!same_context(&a, &b));
        }
    }

    #[test]
    fn actual_build_keeps_trailing_windows_causal_and_legacy_history_exact() {
        let dir = std::env::temp_dir().join(format!(
            "icelines-deployment-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut appearances = Vec::new();
        let mut faces = Vec::new();
        for game in 1..=7u64 {
            let date = format!("2000-01-{game:02}");
            let mut b = json!({"id":game,"gameDate":date,"homeTeam":{"id":1},"awayTeam":{"id":2},"playerByGameStats":{}});
            let mut shifts = Vec::new();
            for (side, team, base) in [("homeTeam", 1u64, 0u64), ("awayTeam", 2, 10)] {
                let f:Vec<_>=(1..=3).map(|n|json!({"playerId":base+n,"position":"L","name":{"default":format!("P{}",base+n)},"toi":"60:00","goals":0,"assists":0,"sog":0})).collect();
                let d:Vec<_>=(4..=5).map(|n|json!({"playerId":base+n,"position":"D","name":{"default":format!("P{}",base+n)}})).collect();
                b["playerByGameStats"][side] = json!({"forwards":f,"defense":d,"goalies":[{"playerId":base+6,"position":"G","name":{"default":"G"}}]});
                for period in 1..=3 {
                    for n in 1..=6 {
                        shifts.push(json!({"gameId":game,"teamId":team,"playerId":base+n,"period":period,"typeCode":517,"startTime":"00:00","endTime":"20:00"}));
                    }
                }
            }
            let bp = dir.join(format!("{game}-box.json"));
            let sp = dir.join(format!("{game}-shifts.json"));
            std::fs::write(&bp, b.to_string()).unwrap();
            std::fs::write(&sp, json!({"data":shifts}).to_string()).unwrap();
            appearances.push(Appearance {
                game_id: game,
                game_date: date.clone(),
                season: 20002001,
                game_type_id: 2,
                team_id: 1,
                team_abbrev: "X".into(),
                opponent: "Y".into(),
                box_path: Some(bp.to_string_lossy().into_owned()),
                shifts_path: Some(sp.to_string_lossy().into_owned()),
            });
            faces.push(json!({"gameId":game,"gameDate":date,"playerId":1,"evFaceoffs":if game==7 {99}else{3}}));
        }
        let fp = dir.join("faces.json");
        std::fs::write(&fp, json!({"data":faces,"total":7}).to_string()).unwrap();
        let r = build(appearances.clone(), 1, Some(&fp)).unwrap();
        assert_eq!(
            r["days"][4]["deployment"]["trailing_five_appearances"]
                ["ev_faceoffs_in_available_games"],
            15
        );
        assert_eq!(
            r["days"][6]["deployment"]["trailing_five_appearances"]["appearances"],
            5
        );
        assert_eq!(
            r["days"][6]["deployment"]["preceding_five_appearances"]["appearances"],
            2
        );
        assert_eq!(
            r["days"][0]["deployment"]["center_review_flag"],
            "review_possible_center_stint"
        );
        assert!(r["days"][0]["deployment"]["individual_xg"].is_null());
        let legacy = player_line_history::build(appearances.clone(), 1).unwrap();
        let mut stripped = r.clone();
        stripped["schema"] = legacy["schema"].clone();
        for key in ["deployment_summary", "deployment_policy"] {
            stripped.as_object_mut().unwrap().remove(key);
        }
        for d in stripped["days"].as_array_mut().unwrap() {
            d.as_object_mut().unwrap().remove("deployment");
        }
        assert_eq!(stripped, legacy);
        // Keep an unavailable selected appearance; do not bridge its change or replace it.
        appearances[5].shifts_path = None;
        let missing = build(appearances, 1, Some(&fp)).unwrap();
        assert_eq!(
            missing["days"][6]["deployment"]["trailing_five_appearances"]["appearances"],
            5
        );
        assert_eq!(
            missing["days"][6]["deployment"]["trailing_five_appearances"]["resolved_games"],
            4
        );
        assert_eq!(
            missing["days"][6]["deployment"]["previous_appearance_comparison"]["status"],
            "not_comparable_missing_or_tied_primary"
        );
        // Duplicate records must not silently win, even if their values agree.
        faces.push(faces[0].clone());
        std::fs::write(&fp, json!({"data":faces,"total":8}).to_string()).unwrap();
        assert!(build(Vec::new(), 1, Some(&fp)).is_err());
        std::fs::write(&fp, json!({"data":[],"total":2}).to_string()).unwrap();
        assert!(build(Vec::new(), 1, Some(&fp)).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
