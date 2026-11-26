use icelines_sources::rinkcraft_shot_methods::parse_game;
use serde_json::{json, Value};
fn event(id: u32, kind: &str, actor: u32, team: u32, shot: Value) -> Value {
    let mut d = json!({"eventOwnerTeamId":team,"shotType":shot});
    d[if kind == "goal" {
        "scoringPlayerId"
    } else {
        "shootingPlayerId"
    }] = json!(actor);
    if kind == "goal" {
        d["homeScore"] = json!(1);
        d["awayScore"] = json!(0);
    }
    json!({"eventId":id,"sortOrder":id,"typeDescKey":kind,"periodDescriptor":{"number":1,"periodType":"REG"},"timeInPeriod":format!("01:{id:02}"),"details":d})
}
fn fixture() -> Value {
    let mut blocked = event(4, "blocked-shot", 11, 1, json!("slap"));
    blocked["details"]["blockingPlayerId"] = json!(21);
    blocked["details"]["reason"] = json!("blocked");
    json!({"id":2026020001u64,"season":20262027,"gameType":2,"gameState":"OFF","homeTeam":{"id":1,"score":1},"awayTeam":{"id":2,"score":0},"rosterSpots":[{"playerId":11,"teamId":1,"positionCode":"C"},{"playerId":12,"teamId":1,"positionCode":"D"},{"playerId":21,"teamId":2,"positionCode":"C"},{"playerId":22,"teamId":2,"positionCode":"G"}],"plays":[event(1,"shot-on-goal",11,1,json!("wrist")),event(2,"missed-shot",11,1,json!("snap")),event(3,"goal",11,1,json!("wrist")),blocked,event(5,"shot-on-goal",22,2,json!("wrist")),event(6,"shot-on-goal",12,1,json!("unrecognized")),event(7,"missed-shot",12,1,Value::Null)]})
}
fn run(
    v: &Value,
) -> Result<
    std::collections::BTreeMap<
        u32,
        std::collections::BTreeMap<
            String,
            icelines_core::view_model::rinkcraft_shot_methods::Counts,
        >,
    >,
    String,
> {
    parse_game(&serde_json::to_vec(v).unwrap(), 2026020001, 20262027)
}
#[test]
fn counts_unblocked_types_and_goals_without_adding_blocks_or_goalie_techniques() {
    let r = run(&fixture()).unwrap();
    assert_eq!(r[&11]["wrist"].attempts, 2);
    assert_eq!(r[&11]["wrist"].on_goal, 2);
    assert_eq!(r[&11]["wrist"].goals, 1);
    assert_eq!(r[&11]["snap"].on_goal, 0);
    assert!(!r[&11].contains_key("slap"));
    assert!(!r.contains_key(&22));
    assert_eq!(r[&12]["unknown"].attempts, 2);
    assert!(r[&21].is_empty());
}
#[test]
fn overtime_is_included_and_shootout_attempts_are_excluded() {
    let mut v = fixture();
    let mut winner = v["plays"].as_array_mut().unwrap().remove(2);
    winner["eventId"] = json!(8);
    winner["sortOrder"] = json!(8);
    winner["periodDescriptor"] = json!({"number":4,"periodType":"OT"});
    winner["timeInPeriod"] = json!("01:00");
    v["plays"].as_array_mut().unwrap().push(winner);
    let mut so = event(9, "goal", 11, 1, json!("backhand"));
    so["periodDescriptor"] = json!({"number":5,"periodType":"SO"});
    v["plays"].as_array_mut().unwrap().push(so);
    assert_eq!(run(&v).unwrap()[&11]["wrist"].goals, 1);
    assert!(!run(&v).unwrap()[&11].contains_key("backhand"));
}
#[test]
fn rejects_wrong_scope_actor_team_clock_and_duplicate_events() {
    for edit in 0..6 {
        let mut v = fixture();
        match edit {
            0 => v["id"] = json!(2026020002u64),
            1 => v["plays"][0]["details"]["shootingPlayerId"] = json!(999),
            2 => v["plays"][0]["details"]["eventOwnerTeamId"] = json!(2),
            3 => v["plays"][0]["timeInPeriod"] = json!("20:01"),
            4 => {
                let e = v["plays"][0].clone();
                v["plays"].as_array_mut().unwrap().push(e);
            }
            _ => {
                v["plays"][0]["periodDescriptor"] = json!({"number":4,"periodType":"OT"});
                v["plays"][0]["timeInPeriod"] = json!("06:00");
            }
        }
        assert!(run(&v).is_err());
    }
}
