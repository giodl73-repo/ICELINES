use icelines_core::view_model::{
    rinkcraft_attribute_calibration as audit,
    rinkcraft_skater_context::Context,
    rinkcraft_stat_scores::{self as s, Catalog, Player},
};
use std::collections::BTreeMap;
fn fixture() -> Catalog {
    let players = ["Forwards", "Defense"]
        .into_iter()
        .enumerate()
        .flat_map(|(g, group)| {
            (0..20).map(move |i| Player {
                player_id: 1 + i + g as u32 * 100,
                group: group.into(),
                games: 20,
                minutes: 600.,
                metrics: BTreeMap::from([
                    ("goals".into(), 1.),
                    ("shots".into(), 4.),
                    ("assists".into(), 2.),
                ]),
            })
        })
        .collect::<Vec<_>>();
    let mut profiles = s::build(20262027, "all strengths · rates per60", &players).unwrap();
    for (p, input) in profiles.iter_mut().zip(&players) {
        let primary = if p.group == "Defense" {
            0.
        } else {
            (p.player_id - 1) as f64
        };
        let mut v = input.metrics.clone();
        v.extend(BTreeMap::from([
            ("primary".into(), primary),
            ("secondary".into(), 20. - primary),
            ("primary-rate".into(), primary / 10.),
            ("quality-attempts".into(), 20.),
            ("quality-goals".into(), 1.),
            ("ixg".into(), 2.),
            ("finishing".into(), -1.),
            ("pp-share".into(), ((p.player_id - 1) % 100) as f64 / 40.),
        ]));
        p.skater_context = Some(Context::new(v, "Synthetic test sources".into(), vec![]));
    }
    Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles,
    }
}
#[test]
fn primary_credit_discriminates_equal_totals_and_position_references_stay_separate() {
    let c = fixture();
    let before = serde_json::to_string(&c).unwrap();
    let r = audit::compare(&c, 20262027, &BTreeMap::new()).unwrap();
    let p = &r.candidates[0];
    assert_eq!(p.rows[0].baseline, Some(10));
    assert_eq!(p.rows[19].baseline, Some(10));
    assert_eq!(p.rows[0].candidate, Some(1));
    assert_eq!(p.rows[19].candidate, Some(20));
    assert!(p.rows[20..].iter().all(|r| r.candidate == Some(10)));
    assert_eq!(p.groups[0].paired_players, 20);
    assert_eq!(p.groups[0].pp_pairs, 20);
    assert!(p.groups[0].baseline_pp_correlation.is_none());
    assert!((p.groups[0].candidate_pp_correlation.unwrap() - 1.).abs() < 1e-10);
    assert_eq!(before, serde_json::to_string(&c).unwrap());
}
#[test]
fn signed_residuals_zero_and_missing_opportunities_remain_distinct() {
    let mut c = fixture();
    let v = &mut c.profiles[0].skater_context.as_mut().unwrap().values;
    v.insert("quality-goals".into(), 2.);
    v.insert("finishing".into(), 0.);
    c.profiles[1]
        .skater_context
        .as_mut()
        .unwrap()
        .values
        .remove("quality-attempts");
    let r = audit::compare(&c, 20262027, &BTreeMap::new()).unwrap();
    let s = &r.candidates[2];
    assert_eq!(s.rows[0].component_value, Some(0.));
    assert_eq!(s.rows[1].component_value, None);
    assert!(s.rows[20].component_value.unwrap() < 0.);
    assert!(s.rows[20].candidate.is_some());
    assert_eq!(s.groups[0].reference_players, 19);
    assert!(s.rows[0].candidate.is_none());
}
#[test]
fn neutral_prior_pulls_small_samples_toward_zero_without_inventing_opportunities() {
    let mut c = fixture();
    for (i, n) in [(0, 1.), (1, 100.)] {
        let v = &mut c.profiles[i].skater_context.as_mut().unwrap().values;
        v.insert("quality-attempts".into(), n);
        v.insert("quality-goals".into(), n);
        v.insert("ixg".into(), n * 0.2);
        v.insert("finishing".into(), n * 0.8);
    }
    let r = audit::compare(&c, 20262027, &BTreeMap::new()).unwrap();
    assert!(
        (r.candidates[2].rows[0].component_value.unwrap()
            - r.candidates[2].rows[1].component_value.unwrap())
        .abs()
            < 1e-10
    );
    assert!(
        r.candidates[3].rows[0].component_value.unwrap()
            < r.candidates[3].rows[1].component_value.unwrap()
    );
    assert!(
        r.candidates[4].rows[0].component_value.unwrap()
            < r.candidates[3].rows[0].component_value.unwrap()
    );
}
#[test]
fn incomplete_source_populations_and_incompatible_baselines_fail_explicitly() {
    let mut c = fixture();
    c.profiles[0]
        .skater_context
        .as_mut()
        .unwrap()
        .values
        .insert("quality-goals".into(), 30.);
    assert!(audit::compare(&c, 20262027, &BTreeMap::new()).is_err());
    let mut c = fixture();
    c.profiles[0]
        .skater_context
        .as_mut()
        .unwrap()
        .values
        .insert("primary-rate".into(), 99.);
    assert!(audit::compare(&c, 20262027, &BTreeMap::new()).is_err());
    let mut c = fixture();
    c.profiles[0].scores[3].parts[0].weight = 0.8;
    assert!(audit::compare(&c, 20262027, &BTreeMap::new()).is_err());
}
