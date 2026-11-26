use icelines_sources::nhl::achievements::goalie_input;
use serde_json::{json, Value};
fn source() -> Vec<String> {
    vec!["https://api.nhle.com/stats/rest/en/goalie/summary?isAggregate=false&isGame=false&cayenneExp=seasonId=20262027%20and%20gameTypeId=2".into()]
}
fn sample() -> Value {
    json!({"total":1,"data":[{"playerId":1,"seasonId":20262027,"goalieFullName":"Example Goalie","gamesPlayed":20,"gamesStarted":18,"shutouts":1,"shotsAgainst":100,"saves":90,"goalsAgainst":10}]})
}
#[test]
fn explicit_goalie_only_report_preserves_counts_and_rejects_scope_or_accounting_errors() {
    let v = sample();
    let p = goalie_input(&serde_json::to_vec(&v).unwrap(), 20262027, source()).unwrap();
    assert_eq!(p.players.len(), 1);
    assert_eq!(p.players[0].counts["saves"], 90.);
    let mut additional = v.clone();
    additional["data"][0]["goalsAgainst"] = 11.into();
    let p = goalie_input(
        &serde_json::to_vec(&additional).unwrap(),
        20262027,
        source(),
    )
    .unwrap();
    assert_eq!(p.players[0].counts["saves"], 90.);
    assert_eq!(p.players[0].counts["goals_against"], 11.);
    for change in 0..4 {
        let mut b = v.clone();
        match change {
            0 => b["total"] = 2.into(),
            1 => b["data"][0]["seasonId"] = 20252026.into(),
            2 => b["data"][0]["saves"] = 101.into(),
            _ => b["data"][0]["gamesStarted"] = 21.into(),
        };
        assert!(goalie_input(&serde_json::to_vec(&b).unwrap(), 20262027, source()).is_err());
    }
    assert!(goalie_input(
        &serde_json::to_vec(&v).unwrap(),
        20262027,
        vec![source()[0].replace("gameTypeId=2", "gameTypeId=3")]
    )
    .is_err());
}
