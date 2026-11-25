use icelines_sources::rinkcraft_pressure_exposure::analyze;
use serde_json::{json, Value};
fn goal(id: u32, time: &str, team: u32, away: u32, home: u32) -> Value {
    json!({"eventId":id,"sortOrder":id,"typeDescKey":"goal","periodDescriptor":{"number":3,"periodType":"REG"},"timeInPeriod":time,"details":{"eventOwnerTeamId":team,"awayScore":away,"homeScore":home}})
}
fn fixture() -> (Value, Value, Value) {
    let game = 2026020001u64;
    let mut p = json!({"id":game,"season":20262027,"gameType":2,"gameState":"OFF","homeTeam":{"id":1,"score":2},"awayTeam":{"id":2,"score":1},"rosterSpots":[],"plays":[goal(1,"15:00",1,0,1),goal(3,"16:00",2,1,1),goal(4,"18:00",1,1,2)]});
    let mut b = json!({"id":game,"season":20262027,"gameType":2,"gameState":"OFF","gameDate":"2026-10-01","homeTeam":{"id":1,"score":2},"awayTeam":{"id":2,"score":1},"playerByGameStats":{}});
    let mut rows = Vec::new();
    for (side, team, base) in [("homeTeam", 1, 0), ("awayTeam", 2, 10)] {
        let mut groups = json!({"forwards":[],"defense":[],"goalies":[]});
        for id in 1..=6 {
            let pid = base + id;
            let (group, pos) = if id == 6 {
                ("goalies", "G")
            } else if id > 3 {
                ("defense", "D")
            } else {
                ("forwards", "C")
            };
            groups[group].as_array_mut().unwrap().push(json!({"playerId":pid,"position":pos,"toi":"60:00","name":{"default":format!("Player {pid}")}}));
            p["rosterSpots"]
                .as_array_mut()
                .unwrap()
                .push(json!({"playerId":pid,"teamId":team,"positionCode":pos}));
            for period in 1..=3 {
                rows.push(json!({"gameId":game,"teamId":team,"playerId":pid,"period":period,"typeCode":517,"startTime":"00:00","endTime":"20:00"}));
            }
        }
        b["playerByGameStats"][side] = groups;
    }
    p["plays"].as_array_mut().unwrap().insert(1,json!({"eventId":2,"sortOrder":2,"typeDescKey":"blocked-shot","periodDescriptor":{"number":3,"periodType":"REG"},"timeInPeriod":"15:30","details":{"eventOwnerTeamId":2,"shootingPlayerId":11,"blockingPlayerId":5,"reason":"blocked"}}));
    let s = json!({"total":rows.len(),"data":rows});
    (p, b, s)
}
fn run(
    p: &Value,
    b: &Value,
    s: &Value,
) -> Result<icelines_sources::rinkcraft_pressure_exposure::GameExposure, String> {
    analyze(
        &serde_json::to_vec(p).unwrap(),
        &serde_json::to_vec(b).unwrap(),
        &serde_json::to_vec(s).unwrap(),
        2026020001,
        20262027,
    )
}
#[test]
fn score_boundaries_and_matched_counts_are_team_relative() {
    let (p, b, s) = fixture();
    let r = run(&p, &b, &s).unwrap();
    assert_eq!(r.players.len(), 10);
    assert!(r.excluded_players.is_empty());
    assert_eq!(r.players[&5].late_lead_seconds, 180);
    assert_eq!(r.players[&5].late_lead_blocks, 1);
    assert_eq!(r.players[&15].late_lead_seconds, 0);
    assert_eq!(r.players[&5].unresolved_team_seconds, 0);
    assert!(!r.players.contains_key(&6));
}
#[test]
fn partial_shift_exposure_and_identical_duplicates_do_not_inflate_time() {
    let (p, mut b, mut s) = fixture();
    let rows = s["data"].as_array_mut().unwrap();
    let first = rows[0].clone();
    rows.push(first);
    let i = rows
        .iter()
        .position(|r| r["playerId"] == 4 && r["period"] == 3)
        .unwrap();
    rows[i]["endTime"] = json!("15:00");
    let mut resumed = rows[i].clone();
    resumed["startTime"] = json!("17:00");
    resumed["endTime"] = json!("20:00");
    rows.push(resumed);
    s["total"] = json!(rows.len());
    b["playerByGameStats"]["homeTeam"]["defense"][0]["toi"] = json!("58:00");
    let r = run(&p, &b, &s).unwrap();
    assert_eq!(r.players[&4].late_lead_seconds, 120);
    assert_eq!(r.players[&5].late_lead_seconds, 180);
}
#[test]
fn conflicting_clock_is_unresolved_and_bad_toi_is_not_zero_exposure() {
    let (p, mut b, s) = fixture();
    b["playerByGameStats"]["homeTeam"]["defense"][0]["toi"] = json!("59:58");
    let r = run(&p, &b, &s).unwrap();
    assert!(!r.players.contains_key(&4));
    assert!(r.excluded_players.contains_key(&4));
    assert_eq!(r.players[&5].late_lead_seconds, 180);
    let (p, b, mut s) = fixture();
    let mut conflict = s["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["playerId"] == 4 && r["period"] == 3)
        .unwrap()
        .clone();
    conflict["endTime"] = json!("19:59");
    s["data"].as_array_mut().unwrap().push(conflict);
    s["total"] = json!(s["data"].as_array().unwrap().len());
    let r = run(&p, &b, &s).unwrap();
    assert!(!r.players.contains_key(&4));
    assert_eq!(r.players[&5].late_lead_seconds, 0);
    assert_eq!(r.players[&5].unresolved_team_seconds, 180);
}
#[test]
fn rejects_wrong_game_incomplete_native_source_and_bad_clocks() {
    for edit in 0..6 {
        let (mut p, mut b, mut s) = fixture();
        match edit {
            0 => b["id"] = json!(2026020002u64),
            1 => s["total"] = json!(1),
            2 => s["adapter"] = json!("derived"),
            3 => s["data"][0]["teamId"] = json!(99),
            4 => p["plays"][0]["timeInPeriod"] = json!("20:01"),
            _ => p["plays"][0]["timeInPeriod"] = json!("19:00"),
        };
        assert!(run(&p, &b, &s).is_err(), "case {edit}");
    }
}
#[test]
fn overtime_toi_is_validated_but_never_added_to_regulation_context() {
    let (mut p, mut b, mut s) = fixture();
    let tie = goal(5, "19:00", 2, 2, 2);
    let mut winner = goal(6, "01:00", 1, 2, 3);
    winner["periodDescriptor"] = json!({"number":4,"periodType":"OT"});
    p["plays"].as_array_mut().unwrap().extend([tie, winner]);
    p["awayTeam"]["score"] = json!(2);
    p["homeTeam"]["score"] = json!(3);
    b["awayTeam"]["score"] = json!(2);
    b["homeTeam"]["score"] = json!(3);
    let extra = s["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["period"] == 1)
        .cloned()
        .map(|mut r| {
            r["period"] = json!(4);
            r["endTime"] = json!("01:00");
            r
        })
        .collect::<Vec<_>>();
    s["data"].as_array_mut().unwrap().extend(extra);
    s["total"] = json!(s["data"].as_array().unwrap().len());
    for side in ["homeTeam", "awayTeam"] {
        for group in ["forwards", "defense", "goalies"] {
            for r in b["playerByGameStats"][side][group].as_array_mut().unwrap() {
                r["toi"] = json!("61:00");
            }
        }
    }
    assert_eq!(run(&p, &b, &s).unwrap().players[&5].late_lead_seconds, 120);
}
