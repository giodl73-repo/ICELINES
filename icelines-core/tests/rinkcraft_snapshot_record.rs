use icelines_core::view_model::{
    rinkcraft_skater_context::{attach_records, Context},
    rinkcraft_snapshot_record::estimate,
    rinkcraft_stat_scores::{self as scores, Player},
};
use std::collections::BTreeMap;
fn profile() -> scores::Profile {
    let mut p = scores::build(
        20262027,
        "all strengths",
        &[Player {
            player_id: 1,
            group: "Forwards".into(),
            games: 20,
            minutes: 400.,
            metrics: BTreeMap::new(),
        }],
    )
    .unwrap()
    .remove(0);
    p.skater_context = Some(Context::new(
        BTreeMap::from([
            ("shots".into(), 9.),
            ("assists".into(), 3.),
            ("goals".into(), 1.5),
            ("hits".into(), 6.),
            ("blocks".into(), 0.),
            ("quality-goals".into(), 999.),
        ]),
        "Synthetic".into(),
        vec![],
    ));
    p
}
#[test]
fn season_pace_restores_hits_blocks_and_preserves_non_count_rates() {
    let p = profile();
    let r = estimate(&p, None).unwrap();
    assert_eq!(r.target_games, 84);
    assert_eq!(r.metrics[0].value, Some(252.));
    assert_eq!(r.metrics[3].per_game, Some(2.));
    assert_eq!(r.metrics[3].value, Some(168.));
    assert_eq!(r.metrics[4].value, Some(0.));
    assert_eq!(r.metrics[5].value, Some(20.));
    assert!(!r.metrics[5].projected_total);
    assert_eq!(r.metrics[6].value, Some(1.5 / 9.));
    assert!(!r.metrics[6].projected_total);
    assert_eq!(r.metrics[2].value, Some(42.));
    assert_eq!(estimate(&p, Some(70)).unwrap().metrics[3].value, Some(140.));
}
#[test]
fn missing_zero_shots_and_inactive_windows_do_not_create_false_totals() {
    let mut p = profile();
    let v = &mut p.skater_context.as_mut().unwrap().values;
    v.remove("hits");
    v.insert("shots".into(), 0.);
    v.insert("goals".into(), 0.);
    let r = estimate(&p, None).unwrap();
    assert_eq!(r.metrics[3].value, None);
    assert_eq!(r.metrics[0].value, Some(0.));
    assert_eq!(r.metrics[6].value, None);
    p.games = 0;
    assert!(estimate(&p, None).is_none());
    p.games = 20;
    p.minutes = 0.;
    assert!(estimate(&p, None).is_none());
    p.minutes = 400.;
    p.group = "Goalies".into();
    assert!(estimate(&p, None).is_none());
}
#[test]
fn attachment_preserves_grades_other_seasons_and_validates_record_window() {
    let p = profile();
    let before = p.scores.clone();
    let mut prior = p.clone();
    prior.season = 20252026;
    let prior_before = prior.clone();
    let mut ps = vec![p, prior];
    attach_records(&mut ps, 20262027);
    assert_eq!(ps[0].scores, before);
    assert_eq!(ps[1], prior_before);
    assert!(ps[0].estimated_record.is_some());
    let mut c = scores::Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles: ps,
    };
    c.validate().unwrap();
    c.profiles[0]
        .estimated_record
        .as_mut()
        .unwrap()
        .observed_games = Some(19);
    assert!(c.validate().is_err());
    c.profiles[0]
        .estimated_record
        .as_mut()
        .unwrap()
        .observed_games = Some(20);
    c.profiles[0].estimated_record.as_mut().unwrap().season = 20252026;
    assert!(c.validate().is_err());
}
