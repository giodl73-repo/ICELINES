use icelines_core::view_model::{
    rinkcraft_defending_calibration as study,
    rinkcraft_skater_context::Context,
    rinkcraft_stat_scores::{self as scores, Catalog, Player},
};
use std::collections::BTreeMap;
fn catalog() -> Catalog {
    let players = ["Forwards", "Defense"]
        .into_iter()
        .enumerate()
        .flat_map(|(g, group)| {
            (0..21).map(move |i| Player {
                player_id: 1 + i + g as u32 * 100,
                group: group.into(),
                games: 60,
                minutes: 1000.,
                metrics: BTreeMap::from([
                    ("blocks".into(), 3.),
                    ("takeaways".into(), 1.),
                    ("taken".into(), 0.5),
                ]),
            })
        })
        .collect::<Vec<_>>();
    let mut profiles = scores::build(20232024, "synthetic all strengths", &players).unwrap();
    for p in &mut profiles {
        let x = ((p.player_id - 1) % 100) as f64 + if p.group == "Defense" { 100. } else { 0. };
        p.skater_context = Some(Context::new(
            BTreeMap::from([
                ("def-5v5-seconds".into(), 36000.),
                ("def-5v5-xga60".into(), x),
                ("def-5v5-off-xga60".into(), 10.),
                ("def-5v5-fa60".into(), x * 10.),
                ("def-pk-share".into(), 0.),
                ("def-5v5-dz-share".into(), 0.5),
            ]),
            "Synthetic fixture".into(),
            vec![],
        ));
    }
    Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles,
    }
}
#[test]
fn distinguishes_outcomes_from_activity_and_keeps_position_references_separate() {
    let c = catalog();
    let original = serde_json::to_vec(&c).unwrap();
    let r = study::compare(&c, 20232024, &BTreeMap::new()).unwrap();
    assert_eq!(r.candidates.len(), 4);
    assert_eq!(r.schema, "rinkcraft.attribute-calibration.v2");
    for candidate in &r.candidates {
        assert!(!candidate.outcome.as_ref().unwrap().higher_is_better);
        assert_eq!(candidate.rows[0].baseline, Some(10));
        assert_eq!(candidate.rows[0].minutes, 600.);
        assert_eq!(candidate.rows[0].production_rate, Some(0.));
        assert!(candidate.rows[0].input_detail.contains("PK time 0.0%"));
    }
    assert_eq!(r.candidates[0].rows[0].candidate, Some(20));
    assert_eq!(r.candidates[0].rows[20].candidate, Some(1));
    assert_eq!(r.candidates[0].rows[21].candidate, Some(20));
    assert_eq!(r.candidates[3].rows[0].candidate, Some(15));
    assert_eq!(original, serde_json::to_vec(&c).unwrap());
}
#[test]
fn missing_off_ice_values_do_not_become_zero_and_reference_minimum_is_enforced() {
    let mut c = catalog();
    c.profiles[0]
        .skater_context
        .as_mut()
        .unwrap()
        .values
        .remove("def-5v5-off-xga60");
    let r = study::compare(&c, 20232024, &BTreeMap::new()).unwrap();
    assert_eq!(r.candidates[0].rows[0].candidate, Some(20));
    assert_eq!(r.candidates[2].rows[0].candidate, None);
    assert_eq!(r.candidates[2].groups[0].reference_players, 20);
    c.profiles[1]
        .skater_context
        .as_mut()
        .unwrap()
        .values
        .remove("def-5v5-off-xga60");
    let r = study::compare(&c, 20232024, &BTreeMap::new()).unwrap();
    assert!(r.candidates[2].rows[..21]
        .iter()
        .all(|r| r.candidate.is_none()));
    c.profiles[2]
        .skater_context
        .as_mut()
        .unwrap()
        .values
        .insert("def-pk-share".into(), 1.1);
    assert!(study::compare(&c, 20232024, &BTreeMap::new()).is_err());
}
#[test]
fn absent_exposure_is_flagged_and_cannot_produce_an_estimate() {
    let mut c = catalog();
    c.profiles[0]
        .skater_context
        .as_mut()
        .unwrap()
        .values
        .remove("def-5v5-seconds");
    let r = study::compare(&c, 20232024, &BTreeMap::new()).unwrap();
    for candidate in r.candidates {
        let row = &candidate.rows[0];
        assert!(row.exposure_missing);
        assert!(!row.reference_eligible);
        assert_eq!(row.candidate, None);
    }
}
