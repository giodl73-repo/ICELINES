use icelines_core::{
    fixtures,
    identity::PlayerId,
    model::{Position, Season},
    season_stats::SeasonType,
    stats_repository::StatsRepository,
    view_model::{
        rinkcraft_goalie_puck as puck,
        rinkcraft_profile::{build_rinkcraft_stat_profile, RinkcraftProfileRequest},
    },
};
use serde_json::{json, Value};
fn ledger() -> Value {
    json!({"schema_version":1,"season":20262027,"season_type":"regular","teams":["NYR"],
 "expected_games":1,"captured_games":1,"event_count":2,"goalies":[{"player_id":1,"counts":{"giveaway":999}}],
 "captures":[{"url":"https://api-web.nhle.com/v1/gamecenter/2026020001/play-by-play","sha256":"a".repeat(64),"captured_at":"2026-11-23T00:00:00Z","status":200}],
 "games":[{"game_id":2026020001_u64,"goalies_on_roster":[{"player_id":1,"name":"Goalie","team_id":10},{"player_id":2,"name":"Backup","team_id":10}],
 "events":[{"game_id":2026020001_u64,"event_id":1,"player_id":1,"team_id":10,"period":1,"time_in_period":"01:00","kind":"giveaway","penalty_description":null,"following_opponent_attempt":{"event_id":3,"kind":"shot-on-goal","seconds_after":5}},
 {"game_id":2026020001_u64,"event_id":2,"player_id":1,"team_id":10,"period":2,"time_in_period":"02:00","kind":"penalty","penalty_description":"delaying-game-puck-over-glass","following_opponent_attempt":null}]}]})
}
fn project(v: Value, id: u32) -> Result<Option<puck::Evidence>, String> {
    puck::project(&v.to_string(), id, 20262027)
}
#[test]
fn derives_counts_preserves_zero_and_never_uses_summary_or_roster_as_gp() {
    let e = project(ledger(), 1).unwrap().unwrap();
    assert_eq!(e.metrics[3].count, 1);
    assert_eq!(e.metrics[0].count, 0);
    assert_eq!(e.penalty_types[0].label, "delaying game puck over glass");
    assert!(e.plays[0]
        .following_attempt
        .as_ref()
        .unwrap()
        .contains("not causation"));
    let backup = project(ledger(), 2).unwrap().unwrap();
    assert!(backup.metrics.iter().all(|m| m.count == 0));
    assert!(backup.plays.is_empty());
    assert!(project(ledger(), 99).unwrap().is_none());
    assert!(puck::project(&ledger().to_string(), 1, 20252026)
        .unwrap()
        .is_none());
}
#[test]
fn refuses_incomplete_unknown_duplicate_or_misattributed_records() {
    for (key, v) in [
        ("captured_games", json!(0)),
        ("event_count", json!(3)),
        ("schema_version", json!(2)),
    ] {
        let mut l = ledger();
        l[key] = v;
        assert!(project(l, 1).is_err());
    }
    for (key, v) in [
        ("player_id", json!(99)),
        ("team_id", json!(99)),
        ("kind", json!("completed-pass")),
    ] {
        let mut l = ledger();
        l["games"][0]["events"][0][key] = v;
        assert!(project(l, 1).is_err());
    }
    let mut l = ledger();
    l["captures"] = json!([]);
    assert!(project(l, 1).is_err());
    let mut l = ledger();
    let g = l["games"][0].clone();
    l["games"].as_array_mut().unwrap().push(g);
    l["expected_games"] = json!(2);
    l["captured_games"] = json!(2);
    l["event_count"] = json!(4);
    assert!(project(l, 1).is_err());
}
#[test]
fn profile_attachment_replaces_stale_data_and_gates_goalie_season_and_type() {
    let mut repo = StatsRepository::with_lru_cap(2);
    repo.upsert_identity(fixtures::identity(1).build()).unwrap();
    let mut row = fixtures::stats(1, 20262027, "AAA").build();
    row.position = Position::Goalie;
    repo.upsert_stats(row).unwrap();
    let mut p = build_rinkcraft_stat_profile(
        &repo,
        RinkcraftProfileRequest {
            player_id: PlayerId(1),
            season_type: SeasonType::Regular,
            first_season: Some(Season(20262027)),
            last_season: Some(Season(20262027)),
        },
    )
    .unwrap();
    let source = ledger().to_string();
    puck::attach(&mut p, &source).unwrap();
    assert!(p.goalie_puck_events.is_some());
    p.timeline[0].position = Position::Center;
    puck::attach(&mut p, &source).unwrap();
    assert!(p.goalie_puck_events.is_none());
    p.timeline[0].position = Position::Goalie;
    p.request.season_type = SeasonType::Playoff;
    puck::attach(&mut p, &source).unwrap();
    assert!(p.goalie_puck_events.is_none());
}
