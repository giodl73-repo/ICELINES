use icelines_core::career_history::CareerGameType;
use icelines_sources::nhl::player_landing::{
    parse_career_history, parse_official_nhl_organization_fact,
};

fn mcdavid_landing() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../icelines-fetch/tests/fixtures/landing/mcdavid_8478402.json"
    ))
    .expect("frozen landing fixture")
}

#[test]
fn l0_player_landing_preserves_career_semantics() {
    let history = parse_career_history(8_478_402, &mcdavid_landing()).expect("parse fixture");
    let ohl = history
        .stints
        .iter()
        .find(|stint| {
            stint.season.0 == 20_142_015
                && stint.league.0 == "OHL"
                && stint.game_type == CareerGameType::Regular
        })
        .expect("McDavid 2014-15 OHL row");
    assert_eq!(ohl.team, "Erie");
    assert_eq!(ohl.gp, 47);
    assert_eq!(ohl.points, Some(120));
    assert!(history.stints.windows(2).all(|rows| {
        (rows[0].season.0, rows[0].sequence, rows[0].game_type as u8)
            <= (rows[1].season.0, rows[1].sequence, rows[1].game_type as u8)
    }));
}

#[test]
fn l0_player_landing_organization_fact_is_dated_and_player_scoped() {
    let fact =
        parse_official_nhl_organization_fact(8_478_402, "2026-07-31T12:00:00Z", &mcdavid_landing())
            .expect("parse organization fact");
    assert_eq!(fact.player_id, 8_478_402);
    assert_eq!(fact.current_team_abbrev.as_deref(), Some("EDM"));
    assert_eq!(fact.observed_at, "2026-07-31T12:00:00Z");
}

#[test]
fn l0_player_landing_toi_is_seconds_and_malformed_values_are_absent() {
    let raw = serde_json::json!({
        "seasonTotals": [
            {"season": 20252026, "gameTypeId": 2, "leagueAbbrev": "NHL", "gamesPlayed": 1, "avgToi": "21:52"},
            {"season": 20242025, "gameTypeId": 2, "leagueAbbrev": "NHL", "gamesPlayed": 1, "avgToi": "garbage"}
        ]
    });
    let history = parse_career_history(1, &raw).expect("parse synthetic TOI rows");
    assert_eq!(history.stints[1].avg_toi_sec, Some(21 * 60 + 52));
    assert_eq!(history.stints[0].avg_toi_sec, None);
}
