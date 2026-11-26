use icelines_core::view_model::{
    rinkcraft_production_mix as mix, rinkcraft_skater_context::Context,
    rinkcraft_stat_scores as scores,
};
use std::collections::BTreeMap;
fn sample() -> scores::Profile {
    let mut p = scores::build(
        20262027,
        "all strengths",
        &[scores::Player {
            player_id: 1,
            group: "Forwards".into(),
            games: 4,
            minutes: 120.,
            metrics: BTreeMap::new(),
        }],
    )
    .unwrap()
    .remove(0);
    p.skater_context = Some(Context::new(
        BTreeMap::from([
            ("goals".into(), 1.),
            ("assists".into(), 3.),
            ("primary".into(), 4.),
            ("secondary".into(), 2.),
        ]),
        "Neutral fixture".into(),
        vec![],
    ));
    p
}
#[test]
fn point_shares_use_matched_rates_and_assist_subtypes_use_points_denominator() {
    let p = sample();
    let before = p.clone();
    let r = mix::build(&p).unwrap();
    assert_eq!(r.headline, "25.0% goal share");
    assert_eq!(r.rows[0].share, "25.0% of points");
    assert_eq!(r.assist_detail[0].rate, "2.00 /60");
    assert_eq!(r.assist_detail[0].share, "50.0% of points");
    assert_eq!(r.assist_detail[1].share, "25.0% of points");
    assert_eq!(p, before);
    mix::validate(&p, &r).unwrap();
}
#[test]
fn zero_missing_conflicting_and_nonfinite_observations_remain_distinct() {
    let mut p = sample();
    let v = &mut p.skater_context.as_mut().unwrap().values;
    for k in ["goals", "assists", "primary", "secondary"] {
        v.insert(k.into(), 0.);
    }
    let r = mix::build(&p).unwrap();
    assert_eq!(r.headline, "No recorded points");
    assert!(r.rows.iter().all(|r| !r.share.contains('%')));
    let mut p = sample();
    p.skater_context.as_mut().unwrap().values.remove("goals");
    assert!(mix::build(&p).is_none());
    for (key, value) in [("primary", 5.), ("secondary", 2.5)] {
        let mut p = sample();
        p.skater_context
            .as_mut()
            .unwrap()
            .values
            .insert(key.into(), value);
        assert!(mix::build(&p).unwrap().assist_detail.is_empty());
    }
    for bad in [-1., f64::NAN, f64::INFINITY] {
        let mut p = sample();
        p.skater_context
            .as_mut()
            .unwrap()
            .values
            .insert("goals".into(), bad);
        assert!(mix::build(&p).is_none());
    }
}
#[test]
fn scope_binding_and_optional_roundtrip_do_not_change_grades() {
    for change in 0..4 {
        let mut p = sample();
        match change {
            0 => p.scope = "5v5".into(),
            1 => p.group = "Goalies".into(),
            2 => p.games = 0,
            _ => p.minutes = 0.,
        };
        assert!(mix::build(&p).is_none());
    }
    let mut p = sample();
    p.production_mix = mix::build(&p);
    let cat = scores::Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles: vec![p.clone()],
    };
    scores::Catalog::from_json(&serde_json::to_string(&cat).unwrap()).unwrap();
    for change in 0..3 {
        let mut r = p.production_mix.clone().unwrap();
        match change {
            0 => r.player_id = 2,
            1 => r.season = 20252026,
            _ => r.headline = "Wizard".into(),
        };
        assert!(mix::validate(&p, &r).is_err());
    }
    p.production_mix = None;
    assert!(!serde_json::to_string(&p)
        .unwrap()
        .contains("production_mix"));
}
