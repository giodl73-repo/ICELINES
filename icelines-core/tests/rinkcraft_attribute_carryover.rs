use icelines_core::view_model::{
    rinkcraft_attribute_calibration::{Candidate, Report, Row},
    rinkcraft_attribute_carryover as study,
};
fn report(season: u32, next: bool) -> Report {
    let mut rows = (0..4)
        .map(|i| {
            let baseline = [1, 7, 15, 20][i];
            Row {
                player_id: i as u32 + 1,
                name: format!("Player {i}"),
                group: "Forwards".into(),
                games: if i == 3 { 20 } else { 60 },
                minutes: 600.,
                reference_eligible: true,
                baseline: Some(baseline),
                candidate: Some(if next { baseline } else { 21 - baseline }),
                delta: None,
                component_value: Some(i as f64),
                component_percentile: None,
                reference_players: 40,
                pp_share: None,
                input_detail: String::new(),
                production_rate: Some(i as f64),
            }
        })
        .collect::<Vec<_>>();
    if next {
        rows.reverse();
    }
    Report {
        schema: "rinkcraft.attribute-calibration.v1".into(),
        season,
        candidates: vec![Candidate {
            id: "test".into(),
            label: "Test".into(),
            attribute: "Passing".into(),
            formula: "Frozen test formula".into(),
            prior_attempts: 0.,
            component_unit: "credits/60".into(),
            groups: vec![],
            rows,
        }],
        notes: vec![],
    }
}
#[test]
fn pairs_by_identity_uses_common_future_outcomes_and_both_season_thresholds() {
    let a = report(20232024, false);
    let b = report(20242025, true);
    let history = study::compare(&[b.clone(), a.clone()]).unwrap();
    let row = &history.comparisons[0];
    assert_eq!(row.matched_players, 4);
    assert_eq!(row.baseline_repeatability, Some(1.));
    assert_eq!(row.candidate_repeatability, Some(-1.));
    assert!(row.baseline_next_production.unwrap() > 0.99);
    assert!(row.candidate_next_production.unwrap() < -0.99);
    assert_eq!(row.baseline_mean_absolute_change, Some(0.));
    assert_eq!(history.comparisons[1].matched_players, 3);
    let mut b = b;
    for r in &mut b.candidates[0].rows {
        r.production_rate = r.production_rate.map(|v| 3. - v);
    }
    let changed = study::compare(&[a, b]).unwrap();
    assert_eq!(
        changed.comparisons[0].baseline_repeatability,
        row.baseline_repeatability
    );
    assert!(changed.comparisons[0].baseline_next_production.unwrap() < -0.99);
}
#[test]
fn missing_changes_and_attrition_are_excluded_from_both_sides() {
    let a = report(20232024, false);
    let mut b = report(20242025, true);
    b.candidates[0].rows[0].group = "Defense".into();
    b.candidates[0].rows[1].candidate = None;
    b.candidates[0].rows[2].games = 9;
    let h = study::compare(&[a, b]).unwrap();
    let row = &h.comparisons[0];
    assert_eq!(row.no_eligible_same_group_next, 2);
    assert_eq!(row.unavailable_pairs, 1);
    assert_eq!(row.matched_players, 1);
    assert_eq!(row.baseline_repeatability, None);
    assert_eq!(row.candidate_next_production, None);
}
#[test]
fn refuses_gaps_duplicates_and_recipe_drift() {
    let a = report(20232024, false);
    assert!(study::compare(&[a.clone(), a.clone()]).is_err());
    assert!(study::compare(&[a.clone(), report(20252026, true)]).is_err());
    let mut b = report(20242025, true);
    b.candidates[0].formula.push_str(" changed");
    assert!(study::compare(&[a.clone(), b]).is_err());
    let mut b = report(20242025, true);
    let duplicate = b.candidates[0].rows[0].clone();
    b.candidates[0].rows.push(duplicate);
    assert!(study::compare(&[a, b]).is_err());
}
