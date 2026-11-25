use icelines_core::view_model::{
    rinkcraft_power_play_context as pp, rinkcraft_skater_context::Context,
    rinkcraft_stat_scores as scores,
};
use std::collections::BTreeMap;
fn player(share: f64, pp: Option<f64>, all: Option<f64>) -> scores::Profile {
    let mut p = scores::build(
        20262027,
        "all strengths",
        &[scores::Player {
            player_id: 1,
            group: "Defense".into(),
            games: 4,
            minutes: 100.,
            metrics: BTreeMap::new(),
        }],
    )
    .unwrap()
    .remove(0);
    let mut values = BTreeMap::from([("pp-share".into(), share)]);
    if let Some(v) = pp {
        values.insert("pp-assists".into(), v);
    }
    if let Some(v) = all {
        values.insert("assists".into(), v);
    }
    p.skater_context = Some(Context::new(values, "Neutral fixture".into(), vec![]));
    p
}
#[test]
fn matched_rates_keep_strength_denominators_and_derive_assist_composition() {
    let p = player(0.2, Some(6.), Some(2.));
    let before = p.clone();
    let r = pp::build(&p).unwrap();
    assert_eq!(r.time_share, "20.0% of own TOI");
    assert_eq!(r.assist_share, "60.0% of assists");
    assert_eq!(r.pp_assists, "6.00 /60");
    assert_eq!(r.outside_assists, "1.00 /60");
    assert_eq!(p, before);
    pp::validate(&p, &r).unwrap();
}
#[test]
fn zero_missing_conflicting_and_no_exposure_are_distinct() {
    let r = pp::build(&player(0., None, Some(2.))).unwrap();
    assert_eq!(r.pp_assists, "No PP time");
    assert_eq!(r.assist_share, "Assist share unavailable");
    assert_eq!(r.outside_assists, "Unavailable");
    let r = pp::build(&player(0.2, Some(0.), Some(0.))).unwrap();
    assert_eq!(r.assist_share, "No credited assists");
    assert_eq!(r.outside_assists, "0.00 /60");
    let r = pp::build(&player(1., Some(2.), Some(2.))).unwrap();
    assert_eq!(r.assist_share, "100.0% of assists");
    assert_eq!(r.outside_assists, "No time outside PP");
    for (s, p, a) in [
        (0.2, Some(20.), Some(1.)),
        (1., Some(2.), Some(3.)),
        (0., Some(2.), Some(3.)),
        (0.2, None, Some(2.)),
    ] {
        let r = pp::build(&player(s, p, a)).unwrap();
        assert_eq!(r.assist_share, "Assist share unavailable");
        assert_eq!(
            r.outside_assists,
            if s == 1. {
                "No time outside PP"
            } else {
                "Unavailable"
            }
        );
    }
}
#[test]
fn scope_binding_optional_roundtrip_and_invalid_inputs_preserve_missingness() {
    for share in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
        assert!(pp::build(&player(share, Some(1.), Some(2.))).is_none());
    }
    for change in 0..4 {
        let mut p = player(0.2, Some(1.), Some(2.));
        match change {
            0 => p.group = "Goalies".into(),
            1 => p.scope = "5v5".into(),
            2 => p.games = 0,
            _ => p.minutes = 0.,
        };
        assert!(pp::build(&p).is_none());
    }
    let mut p = player(0.2, Some(1.), Some(2.));
    p.power_play_context = pp::build(&p);
    let cat = scores::Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles: vec![p.clone()],
    };
    scores::Catalog::from_json(&serde_json::to_string(&cat).unwrap()).unwrap();
    let mut stale = p.power_play_context.clone().unwrap();
    stale.season = 20252026;
    assert!(pp::validate(&p, &stale).is_err());
    stale = p.power_play_context.clone().unwrap();
    stale.assist_share = "Wizard".into();
    assert!(pp::validate(&p, &stale).is_err());
    p.power_play_context = None;
    assert!(!serde_json::to_string(&p)
        .unwrap()
        .contains("power_play_context"));
}
