use icelines_core::view_model::rinkcraft_achievements::raw_thresholds;
use icelines_core::view_model::rinkcraft_stat_scores::{estimate_record, RecordMetric};
#[test]
fn season_pace_respects_lengths_limits_rates_and_zero_games() {
    let metrics = vec![
        RecordMetric {
            key: "points".into(),
            label: "Points / GP".into(),
            unit: "PerGame".into(),
            value: Some(2.),
        },
        RecordMetric {
            key: "goals".into(),
            label: "Goals".into(),
            unit: "Count".into(),
            value: Some(10.),
        },
        RecordMetric {
            key: "sv".into(),
            label: "Save percentage".into(),
            unit: "Pct".into(),
            value: Some(0.91),
        },
    ];
    let current = estimate_record(20262027, 20, 200., None, &metrics);
    assert_eq!(current.target_games, 84);
    assert_eq!(current.metrics[0].value, Some(168.));
    assert_eq!(current.metrics[0].per_game, Some(2.));
    assert_eq!(current.metrics[1].value, Some(42.));
    assert_eq!(current.metrics[2].value, Some(0.91));
    assert!(!current.metrics[2].projected_total);
    assert_eq!(
        estimate_record(20252026, 20, 200., None, &metrics).metrics[0].value,
        Some(164.)
    );
    assert_eq!(
        estimate_record(20262027, 20, 200., Some(70), &metrics).metrics[0].value,
        Some(140.)
    );
    assert_eq!(
        estimate_record(20262027, 0, 0., None, &metrics).metrics[0].value,
        None
    );
}
#[test]
fn displayed_raw_boundaries_match_existing_percentile_rules_with_and_without_ties() {
    for peers in [
        (0..40).map(|i| i as f64 / 7.).collect::<Vec<_>>(),
        vec![0.; 40],
        (0..40).map(|i| (i / 8) as f64).collect(),
    ] {
        let cutoffs = raw_thresholds(&peers);
        for target in [60., 75., 90.] {
            let cutoff = &cutoffs[&format!("{target:.0}")];
            for value in peers.iter().copied().chain([-1., 0.1, 0.5, 2.1, 10., 100.]) {
                let rank = 100.
                    * (peers.iter().filter(|v| **v < value).count() as f64
                        + 0.5 * peers.iter().filter(|v| **v == value).count() as f64)
                    / peers.len() as f64;
                let raw_pass = if cutoff.inclusive {
                    value >= cutoff.value
                } else {
                    value > cutoff.value
                };
                assert_eq!(raw_pass, rank >= target, "value={value} target={target}");
            }
        }
    }
    assert!(raw_thresholds(&[0.; 19]).is_empty());
}
#[test]
fn per_sixty_record_uses_observed_ice_time_per_game() {
    let metric = RecordMetric {
        key: "shots".into(),
        label: "Shots / 60".into(),
        unit: "Per60".into(),
        value: Some(3.),
    };
    let record = estimate_record(20262027, 20, 200., None, &[metric]);
    assert_eq!(record.metrics[0].label, "Shots");
    assert_eq!(record.metrics[0].per_game, Some(0.5));
    assert_eq!(record.metrics[0].value, Some(42.));
}
