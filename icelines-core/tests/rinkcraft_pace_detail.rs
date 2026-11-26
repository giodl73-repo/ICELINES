use icelines_core::view_model::rinkcraft_stat_scores::{
    estimate_record, EstimatedRecord, RecordMetric,
};
fn metric(unit: &str, value: Option<f64>) -> RecordMetric {
    RecordMetric {
        key: "hits".into(),
        label: "Hits".into(),
        unit: unit.into(),
        value,
    }
}
#[test]
fn whole_observed_counts_explain_fractional_pace_without_using_display_rates() {
    let r = estimate_record(
        20262027,
        24,
        400.,
        None,
        &[
            metric("PerGame", Some(17. / 24.)),
            metric("Count", Some(0.)),
            metric("Pct", Some(0.1)),
        ],
    );
    assert_eq!(r.observed_games, Some(24));
    assert_eq!(r.metrics[0].recorded_count, Some(17));
    assert_eq!(r.metrics[0].value, Some(59.5));
    assert_eq!(
        r.metrics[0].calculation.as_deref(),
        Some("17 / 24 GP x 84 games = 59.50")
    );
    assert_eq!(r.metrics[1].recorded_count, Some(0));
    assert!(r.metrics[2].recorded_count.is_none());
    assert!(r.metrics[2].calculation.is_none());
    assert!(r.has_calculations());
}
#[test]
fn rounded_rate_inputs_do_not_invent_observed_whole_counts() {
    let r = estimate_record(
        20262027,
        24,
        400.,
        Some(70),
        &[metric("PerGame", Some(0.71))],
    );
    assert_eq!(r.metrics[0].recorded_count, None);
    assert!(r.metrics[0]
        .calculation
        .as_ref()
        .unwrap()
        .contains("full-precision rate used"));
    assert_eq!(r.target_games, 70);
    let r = estimate_record(20262027, 0, 0., None, &[metric("Count", Some(17.))]);
    assert!(r.metrics[0].calculation.is_none());
    assert!(r.metrics[0].recorded_count.is_none());
}
#[test]
fn older_records_keep_optional_detail_absent() {
    let r = estimate_record(20262027, 20, 400., None, &[metric("Count", Some(10.))]);
    let mut json = serde_json::to_value(&r).unwrap();
    json.as_object_mut().unwrap().remove("observed_games");
    for m in json["metrics"].as_array_mut().unwrap() {
        m.as_object_mut().unwrap().remove("recorded_count");
        m.as_object_mut().unwrap().remove("calculation");
    }
    let old: EstimatedRecord = serde_json::from_value(json).unwrap();
    assert!(old.observed_games.is_none());
    assert!(!old.has_calculations());
    assert_eq!(old.metrics[0].value, r.metrics[0].value);
}
