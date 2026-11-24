use icelines_core::view_model::rinkcraft_stat_scores::Player;
use icelines_sources::rinkcraft_skater_context as c;
use serde_json::json;
use std::collections::BTreeMap;
#[test]
fn assists_follow_official_order_and_exclude_shootout_and_wrong_actors() {
    let game = json!({"id":2026020001_u64,"season":20262027,"gameType":2,"gameState":"OFF","rosterSpots":[{"playerId":1,"teamId":10},{"playerId":2,"teamId":10}],
 "plays":[{"eventId":1,"typeDescKey":"goal","periodDescriptor":{"periodType":"REG"},"details":{"eventOwnerTeamId":10,"assist1PlayerId":2,"assist2PlayerId":1}},
 {"eventId":2,"typeDescKey":"goal","periodDescriptor":{"periodType":"SO"},"details":{"assist1PlayerId":2}}]});
    let parse = |v: &serde_json::Value| {
        c::goal_assists(&serde_json::to_vec(v).unwrap(), 2026020001, 20262027)
    };
    let out = parse(&game).unwrap();
    assert_eq!(out[&1].secondary, 1);
    assert_eq!(out[&2].primary, 1);
    let mut bad = game.clone();
    bad["plays"][0]["details"]["assist1PlayerId"] = json!(99);
    assert!(parse(&bad).is_err());
    let mut bad = game.clone();
    bad["gameType"] = json!(3);
    assert!(parse(&bad).is_err());
}
#[test]
fn shooting_uses_same_archive_goals_and_xg_excludes_empty_nets_and_misses_are_attempts() {
    let csv="shotID,game_id,season,isPlayoffGame,period,shotOnEmptyNet,shooterPlayerId,goal,xGoal,shotDistance,shotWasOnGoal\n1,20001,2026,0,1,0,1,1,0.3,12,1\n2,20001,2026,0,1,0,1,0,0.1,24,0\n3,20001,2026,0,1,1,1,1,0.9,45,1\n4,30001,2026,1,1,0,1,1,0.5,15,1\n";
    let s = c::shooting(csv.as_bytes(), 20262027).unwrap();
    assert_eq!(s[&1].attempts, 2);
    assert_eq!(s[&1].goals, 1);
    assert!((s[&1].xg - 0.4).abs() < 1e-10);
    assert!(c::shooting(csv.replace(",2026,", ",2025,").as_bytes(), 20262027).is_err());
}
#[test]
fn contexts_keep_pp_and_ev_denominators_zero_and_unavailable_assist_details_distinct() {
    let p = Player {
        player_id: 1,
        group: "Forwards".into(),
        games: 10,
        minutes: 100.,
        metrics: BTreeMap::from([
            ("goals".into(), 1.),
            ("shots".into(), 6.),
            ("assists".into(), 3.),
            ("giveaways".into(), 1.),
            ("drawn".into(), 0.),
            ("taken".into(), 0.),
        ]),
    };
    let summary = vec![
        json!({"playerId":1,"gamesPlayed":10,"assists":5,"evPoints":4,"evGoals":2,"ppPoints":3,"ppGoals":0}),
    ];
    let rt = vec![json!({"playerId":1,"gamesPlayed":10,"hits":10})];
    let toi = vec![
        json!({"playerId":1,"gamesPlayed":10,"evTimeOnIce":4800,"ppTimeOnIce":600,"shTimeOnIce":0}),
    ];
    let assists = BTreeMap::from([(
        1,
        c::Assists {
            primary: 4,
            secondary: 1,
        },
    )]);
    let result = c::build(&[p.clone()], &summary, &rt, &toi, Some(&assists), None).unwrap();
    let v = &result[&1];
    v.validate().unwrap();
    assert_eq!(v.values["ev-assists"], 1.5);
    assert_eq!(v.values["pp-assists"], 18.);
    assert_eq!(v.values["primary"], 4.);
    assert_eq!(v.values["sh-seconds"], 0.);
    assert!(v
        .rows
        .iter()
        .any(|r| r.label == "Penalty-kill deployment" && r.value == "0:00 /GP"));
    assert!(v
        .rows
        .iter()
        .any(|r| r.label.starts_with("Expected goals (") && r.value == "Unavailable"));
    let mismatch = BTreeMap::new();
    let result = c::build(&[p], &summary, &rt, &toi, Some(&mismatch), None).unwrap();
    assert!(!result[&1].values.contains_key("primary"));
}
