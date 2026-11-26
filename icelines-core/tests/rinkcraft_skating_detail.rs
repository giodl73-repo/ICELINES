use icelines_core::view_model::{
    rinkcraft_skater_context::Context, rinkcraft_skating_detail as detail,
    rinkcraft_stat_scores as scores,
};
use std::collections::BTreeMap;
fn player(metrics: BTreeMap<String, f64>) -> scores::Profile {
    scores::build(
        20262027,
        "all strengths",
        &[scores::Player {
            player_id: 1,
            group: "Defense".into(),
            games: 4,
            minutes: 80.,
            metrics,
        }],
    )
    .unwrap()
    .remove(0)
}
fn sample() -> scores::Profile {
    player(BTreeMap::from([
        ("peak-speed".into(), 22.1234),
        ("bursts".into(), 6.5),
    ]))
}
#[test]
fn existing_measurements_fill_the_detail_without_a_ten_game_gate_or_score_change() {
    let mut p = sample();
    p.skater_context = Some(Context::new(
        BTreeMap::from([("edge-distance".into(), 12.345)]),
        "Neutral test".into(),
        vec![],
    ));
    let scores = p.scores.clone();
    let r = detail::build(&p).unwrap();
    assert_eq!(r.headline, "22.12 mph");
    assert_eq!(r.secondary, "20+ mph bursts 6.50 /60");
    assert_eq!(r.rows[2].value, "12.35 miles");
    assert_eq!(p.scores, scores);
    detail::validate(&p, &r).unwrap();
}
#[test]
fn a_burst_only_record_does_not_fabricate_peak_speed_and_measured_zero_is_retained() {
    let p = player(BTreeMap::from([("bursts".into(), 4.)]));
    let r = detail::build(&p).unwrap();
    assert_eq!(r.headline, "4.00 /60");
    assert_eq!(r.rows[0].value, "Unavailable");
    assert!(r.secondary.contains("peak unavailable"));
    let p = player(BTreeMap::from([
        ("peak-speed".into(), 0.),
        ("bursts".into(), 0.),
    ]));
    let r = detail::build(&p).unwrap();
    assert_eq!(r.headline, "0.00 mph");
    assert_eq!(r.rows[1].value, "0.00 /60");
}
#[test]
fn missing_wrong_unit_scope_and_goalie_inputs_stay_unavailable() {
    assert!(detail::build(&player(BTreeMap::new())).is_none());
    for edit in 0..5 {
        let mut p = player(BTreeMap::from([("peak-speed".into(), 22.)]));
        match edit {
            0 => p.group = "Goalies".into(),
            1 => p.scope = "5v5".into(),
            2 => p.scores[0].parts[0].unit = "km/h".into(),
            3 => p.games = 0,
            _ => p.minutes = f64::NAN,
        };
        assert!(detail::build(&p).is_none());
    }
}
#[test]
fn canonical_text_and_snapshot_binding_reject_stale_detail() {
    let p = sample();
    let r = detail::build(&p).unwrap();
    for edit in 0..5 {
        let mut bad = r.clone();
        match edit {
            0 => bad.player_id = 2,
            1 => bad.games = 3,
            2 => bad.season = 20252026,
            3 => bad.headline = "99.00 mph".into(),
            _ => bad.rows[0].value = "99.00 mph".into(),
        };
        assert!(detail::validate(&p, &bad).is_err());
    }
}
#[test]
fn optional_record_roundtrips_and_old_catalogs_omit_it() {
    let mut p = sample();
    let old = serde_json::to_value(&p).unwrap();
    assert!(old.get("skating_detail").is_none());
    p.skating_detail = detail::build(&p);
    let c = scores::Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles: vec![p],
    };
    c.validate().unwrap();
    let decoded = scores::Catalog::from_json(&serde_json::to_string(&c).unwrap()).unwrap();
    assert_eq!(
        decoded.profiles[0].skating_detail,
        c.profiles[0].skating_detail
    );
}
