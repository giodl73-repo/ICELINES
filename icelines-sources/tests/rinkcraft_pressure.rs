use icelines_sources::rinkcraft_pressure::parse_game;
use serde_json::{json, Value};
fn draw(id: u32, period: u32, time: &str) -> Value {
    json!({"eventId":id,"sortOrder":id,"typeDescKey":"faceoff","periodDescriptor":{"number":period,"periodType":"REG"},"timeInPeriod":time,"details":{"eventOwnerTeamId":2,"winningPlayerId":1,"losingPlayerId":2}})
}
fn goal(id: u32, team: u32, away: u32, home: u32) -> Value {
    json!({"eventId":id,"sortOrder":id,"typeDescKey":"goal","periodDescriptor":{"number":3,"periodType":"REG"},"details":{"eventOwnerTeamId":team,"awayScore":away,"homeScore":home}})
}
fn game() -> Value {
    json!({"id":2026020001u64,"season":20262027,"gameType":2,"gameState":"OFF","awayTeam":{"id":3,"score":0},"homeTeam":{"id":2,"score":2},"rosterSpots":[{"playerId":1,"teamId":2,"positionCode":"C"},{"playerId":2,"teamId":3,"positionCode":"C"},{"playerId":3,"teamId":2,"positionCode":"D"}],"plays":[draw(1,1,"00:00"),goal(2,2,0,1),draw(3,3,"15:00"),goal(4,2,0,2),draw(5,3,"19:00")]})
}
fn parse(
    v: &Value,
) -> Result<std::collections::BTreeMap<u32, icelines_sources::rinkcraft_pressure::Counts>, String> {
    parse_game(&serde_json::to_vec(v).unwrap(), 2026020001, 20262027)
}
#[test]
fn draw_strength_is_player_relative_and_empty_nets_are_not_standard_advantage() {
    let mut v = game();
    for (code, home, away) in [
        ("1551", "even", "even"),
        ("1331", "even", "even"),
        ("1451", "advantage", "disadvantage"),
        ("1541", "disadvantage", "advantage"),
        ("1351", "advantage", "disadvantage"),
        ("0651", "other", "other"),
        ("1560", "other", "other"),
        ("bad", "unknown", "unknown"),
        ("2551", "unknown", "unknown"),
        ("15511", "unknown", "unknown"),
    ] {
        v["plays"][0]["situationCode"] = json!(code);
        let r = parse(&v).unwrap();
        assert_eq!(r[&1].strengths[home].won, if home=="unknown" {3} else {1});
        assert_eq!(r[&2].strengths[away].won, 0);
        for id in [1, 2] {
            assert_eq!(
                r[&id].strengths.values().map(|c| c.taken).sum::<u32>(),
                r[&id].taken
            );
            assert_eq!(
                r[&id].strengths.values().map(|c| c.won).sum::<u32>(),
                r[&id].won
            );
        }
    }
    v["plays"][0]
        .as_object_mut()
        .unwrap()
        .remove("situationCode");
    assert_eq!(parse(&v).unwrap()[&1].strengths["unknown"].taken, 3);
}
fn action(id: u32, kind: &str, details: Value) -> Value {
    json!({"eventId":id,"sortOrder":id,"typeDescKey":kind,"periodDescriptor":{"number":3,"periodType":"REG"},"timeInPeriod":"15:00","details":details})
}
#[test]
fn defensive_actions_use_blocker_team_exclude_teammates_and_keep_baseline() {
    let mut v = game();
    let block = |id| {
        action(
            id,
            "blocked-shot",
            json!({"blockingPlayerId":3,"shootingPlayerId":2,"eventOwnerTeamId":3,"reason":"blocked"}),
        )
    };
    v["plays"] = json!([
        block(1),
        goal(2, 2, 0, 1),
        block(3),
        action(4, "takeaway", json!({"playerId":3,"eventOwnerTeamId":2})),
        action(
            5,
            "blocked-shot",
            json!({"blockingPlayerId":3,"shootingPlayerId":1,"eventOwnerTeamId":2,"reason":"teammate-blocked"})
        ),
        goal(6, 2, 0, 2),
        block(7),
        action(8, "takeaway", json!({"playerId":2,"eventOwnerTeamId":3})),
        action(
            9,
            "blocked-shot",
            json!({"shootingPlayerId":2,"eventOwnerTeamId":3,"reason":"other-block"})
        )
    ]);
    let r = parse(&v).unwrap();
    assert_eq!(r[&3].actions.blocks, 3);
    assert_eq!(r[&3].actions.late_lead_blocks, 1);
    assert_eq!(r[&3].actions.late_lead_takeaways, 1);
    assert_eq!(r[&2].actions.takeaways, 1);
    assert_eq!(r[&2].actions.late_lead_takeaways, 0);
    assert_eq!(r[&1].roster_games, 1);
    for edit in 0..4 {
        let mut bad = v.clone();
        match edit {
            0 => bad["plays"][0]["details"]["blockingPlayerId"] = json!(999),
            1 => bad["plays"][0]["details"]["eventOwnerTeamId"] = json!(2),
            2 => bad["plays"][3]["details"]["eventOwnerTeamId"] = json!(3),
            _ => bad["plays"][0]["timeInPeriod"] = json!("20:01"),
        };
        assert!(parse(&bad).is_err());
    }
    v["plays"][2]["periodDescriptor"]["periodType"] = json!("OT");
    assert_eq!(parse(&v).unwrap()[&3].actions.late_lead_blocks, 0);
}
#[test]
fn late_context_uses_score_before_draw_and_retains_opponent_losses() {
    let r = parse(&game()).unwrap();
    assert_eq!(r[&1].taken, 3);
    assert_eq!(r[&1].won, 3);
    assert_eq!(r[&1].late_taken, 1);
    assert_eq!(r[&2].won, 0);
    assert_eq!(r[&2].late_taken, 1);
    assert_eq!(r[&3].taken, 0);
    assert_eq!(r[&1].late_states["leading"].won, 1);
    assert_eq!(r[&2].late_states["trailing"].taken, 1);
}
#[test]
fn state_is_team_relative_and_tied_draws_are_separate() {
    let mut v = game();
    v["plays"] = json!([
        goal(1, 2, 0, 1),
        goal(2, 3, 1, 1),
        draw(3, 3, "15:00"),
        goal(4, 3, 2, 1),
        draw(5, 3, "16:00"),
        goal(6, 2, 2, 2),
        draw(7, 3, "17:00"),
        goal(8, 2, 2, 3)
    ]);
    v["awayTeam"]["score"] = json!(2);
    v["homeTeam"]["score"] = json!(3);
    let r = parse(&v).unwrap();
    assert_eq!(r[&1].late_taken, 3);
    assert_eq!(r[&1].late_states["tied"].taken, 2);
    assert_eq!(r[&1].late_states["trailing"].won, 1);
    assert_eq!(r[&2].late_states["leading"].won, 0);
    assert_eq!(r[&2].late_states["leading"].taken, 1);
}
#[test]
fn rejects_wrong_scope_actors_duplicate_events_and_missing_score_sequence() {
    for change in 0..6 {
        let mut v = game();
        match change {
            0 => v["gameType"] = json!(3),
            1 => v["plays"][0]["details"]["losingPlayerId"] = json!(1),
            2 => {
                let e = v["plays"][0].clone();
                v["plays"].as_array_mut().unwrap().push(e);
            }
            3 => {
                v["plays"].as_array_mut().unwrap().remove(1);
            }
            4 => v["plays"][0]["timeInPeriod"] = json!("20:60"),
            _ => v["plays"][0]["details"]["eventOwnerTeamId"] = json!(99),
        };
        assert!(parse(&v).is_err(), "case {change}");
    }
}
#[test]
fn overtime_is_not_late_regulation_and_shootout_score_is_not_a_goal() {
    let mut v = game();
    v["plays"] = json!([goal(1, 2, 0, 1), goal(2, 3, 1, 1), draw(3, 4, "00:00")]);
    v["plays"][2]["periodDescriptor"]["periodType"] = json!("OT");
    v["awayTeam"]["score"] = json!(1);
    v["homeTeam"]["score"] = json!(2);
    v["gameOutcome"] = json!({"lastPeriodType":"SO"});
    let r = parse(&v).unwrap();
    assert_eq!(r[&1].taken, 1);
    assert_eq!(r[&1].late_taken, 0);
}
