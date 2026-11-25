use icelines_sources::rinkcraft_stat_inputs::{goalie_inputs, skater_inputs};
use serde_json::json;
const SHOTS:&str="shotID,game_id,season,isPlayoffGame,period,goalieIdForShot,shotOnEmptyNet,shotWasOnGoal,goal,shotRebound,shotAngleReboundRoyalRoad,shotGeneratedRebound,xGoal,xRebound\n0,20001,2025,0,1,1,0,1,0,0,1,1,0.02,0.1\n1,20001,2025,0,1,1,0,1,1,1,1,0,0.3,0.1\n2,20001,2025,0,1,1,0,0,0,0,0,0,0.01,0.1\n3,30001,2025,1,1,1,0,1,1,1,1,0,0.3,0.1\n4,20001,2025,0,5,1,0,1,1,0,0,0,0.3,0.1\n5,20001,2025,0,1,,1,1,1,0,0,0,0.3,0.1\n";
fn nhl() -> Vec<serde_json::Value> {
    vec![
        json!({"playerId":1,"seasonId":20252026,"gamesPlayed":20,"timeOnIce":12000,"saves":1,"shotsAgainst":2,"assists":0}),
    ]
}
fn summary() -> &'static str {
    "playerId,season,situation,games_played,ongoal\n1,2025,all,20,2\n"
}
#[test]
fn shot_sequences_separate_misses_fast_followups_and_cross_slot_opportunities() {
    let mut csv = SHOTS
        .lines()
        .enumerate()
        .map(|(i, line)| {
            format!(
                "{line},{}",
                if i == 0 {
                    "timeSinceLastEvent,lastEventCategory"
                } else {
                    "1,SHOT"
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    csv.push_str("\n6,20001,2025,0,1,1,0,0,0,1,0,0,0.3,0.1,2,MISS\n7,20001,2025,0,1,1,0,1,0,1,1,0,0.3,0.1,0,SHOT\n");
    let rows = goalie_inputs(20252026, &nhl(), summary(), csv.as_bytes()).unwrap();
    let data = rows[0].shot_context.clone().unwrap();
    assert_eq!(data.attempts, 5);
    assert_eq!(data.buckets["rebound"].attempts, 3);
    assert_eq!(data.buckets["rebound"].on_goal, 2);
    assert_eq!(data.buckets["rebound"].goals, 1);
    assert_eq!(data.buckets["after-miss"].attempts, 1);
    assert_eq!(data.buckets["after-miss"].on_goal, 0);
    assert_eq!(data.buckets["fast"].on_goal, 2);
    assert_eq!(data.buckets["cross"].on_goal, 2);
    let mut c = icelines_core::view_model::rinkcraft_goalie_shots::Context::new(
        data,
        "https://peter-tanner.com/moneypuck/downloads/shots_2025.zip".into(),
        "2026-11-23T00:00:00Z".into(),
        "a".repeat(64),
    );
    c.validate().unwrap();
    assert_eq!(c.headline[0].value, "1/2 saved · 50.0%");
    assert!(c.rows[6].value.contains("No on-goal opportunities"));
    c.rows[0].value = "Invented".into();
    assert!(c.validate().is_err());
    assert!(goalie_inputs(
        20252026,
        &nhl(),
        summary(),
        csv.replace(",2,MISS", ",4,MISS").as_bytes()
    )
    .is_err());
    assert!(
        goalie_inputs(20252026, &nhl(), summary(), SHOTS.as_bytes()).unwrap()[0]
            .shot_context
            .is_none()
    );
}
#[test]
fn goalie_sources_keep_sog_denominators_flags_competition_and_zero_separate() {
    let rows = goalie_inputs(20252026, &nhl(), summary(), SHOTS.as_bytes()).unwrap();
    let p = &rows[0].player;
    assert_eq!(p.group, "Goalies");
    assert_eq!(p.minutes, 200.);
    assert_eq!(p.metrics["goalie-save-fraction"], 0.5);
    assert_eq!(p.metrics["goalie-low-save"], 1.); // A missed attempt is not a goalie save.
    assert_eq!(p.metrics["goalie-high-save"], 0.);
    assert_eq!(p.metrics["goalie-rebound-save"], 0.);
    assert_eq!(p.metrics["goalie-cross-save"], 0.); // Both cross-slot AND rebound flags required.
    assert!((p.metrics["goalie-rebound-ratio"] - 1. / 0.3).abs() < 1e-10);
    assert_eq!(p.metrics["goalie-assists"], 0.);
    assert!(rows[0].notes.is_empty());
}
#[test]
fn distinct_provider_coverage_is_explicit_without_crossing_rate_denominators() {
    let rows = goalie_inputs(
        20252026,
        &nhl(),
        &summary().replace(",20,2", ",19,3"),
        SHOTS.as_bytes(),
    )
    .unwrap();
    assert_eq!(rows[0].notes.len(), 2);
    assert_eq!(rows[0].player.metrics["goalie-low-save"], 1.);
}
#[test]
fn bad_scope_duplicates_invalid_flags_and_inconsistent_counts_fail() {
    for shots in [
        SHOTS.replace(",2025,", ",2024,"),
        SHOTS.replace("0,20001,2025,0,1", "0,20001,2025,2,1"),
        format!("{SHOTS}0,20001,2025,0,1,1,0,1,0,0,0,0,0.1,0.1\n"),
    ] {
        assert!(goalie_inputs(20252026, &nhl(), summary(), shots.as_bytes()).is_err());
    }
    let mut bad = nhl();
    bad[0]["saves"] = json!(3);
    assert!(goalie_inputs(20252026, &bad, summary(), SHOTS.as_bytes()).is_err());
}
#[test]
fn no_rebound_opportunity_remains_unknown_and_missing_assists_are_not_zero() {
    let mut rows = nhl();
    rows[0].as_object_mut().unwrap().remove("assists");
    let shots = SHOTS.lines().take(2).collect::<Vec<_>>().join("\n");
    let result = goalie_inputs(20252026, &rows, summary(), shots.as_bytes()).unwrap();
    assert!(!result[0].player.metrics.contains_key("goalie-rebound-save"));
    assert!(!result[0].player.metrics.contains_key("goalie-assists"));
}
#[test]
fn skater_refresh_uses_all_strength_toi_and_rejects_mixed_gps() {
    let s = vec![
        json!({"playerId":1,"seasonId":20252026,"positionCode":"D","gamesPlayed":20,"goals":1,"shots":10,"assists":4}),
    ];
    let rt = vec![
        json!({"playerId":1,"seasonId":20252026,"gamesPlayed":20,"giveaways":0,"takeaways":3,"blockedShots":8}),
    ];
    let pen = vec![
        json!({"playerId":1,"seasonId":20252026,"gamesPlayed":20,"penaltiesDrawn":4,"penalties":5}),
    ];
    let mut time =
        vec![json!({"playerId":1,"seasonId":20252026,"gamesPlayed":20,"timeOnIce":12000})];
    let p = skater_inputs(20252026, &s, &rt, &pen, &time).unwrap();
    assert_eq!(p[0].metrics["assists"], 1.2);
    assert_eq!(p[0].metrics["giveaways"], 0.);
    assert_eq!(p[0].group, "Defense");
    time[0]["gamesPlayed"] = json!(21);
    assert!(skater_inputs(20252026, &s, &rt, &pen, &time).is_err());
}
