use icelines_core::view_model::{
    rinkcraft_history::{HistoryPoint, HistoryRow, HistorySeason},
    rinkcraft_history_changes::build,
    rinkcraft_numbers::Measurement,
};
fn season(year: u32) -> HistorySeason {
    HistorySeason {
        season: year,
        group: Some("Defense".into()),
        situation: Some("5on5".into()),
        games: Some(20),
        minutes: Some(400.),
        state: "available".into(),
        sources: vec![],
    }
}
fn measurement(value: f64, rank: Option<f64>) -> Measurement {
    Measurement {
        key: "test".into(),
        label: "Test".into(),
        domain: "Output".into(),
        interpretation: "Recorded".into(),
        unit: "Per60".into(),
        value: Some(value),
        percentile: rank,
        numerator: Some(value),
        denominator: Some(1.),
        scale: 60.,
        measured_peers: 25,
        reference_sha256: "a".repeat(64),
        state: if rank.is_some() {
            "ranked"
        } else {
            "below_minimum"
        }
        .into(),
        formula: "n / d * 60".into(),
    }
}
fn row(a: Measurement, b: Measurement) -> HistoryRow {
    HistoryRow {
        key: "test".into(),
        label: "Test".into(),
        points: vec![
            HistoryPoint {
                season: 20242025,
                state: a.state.clone(),
                measurement: Some(a),
            },
            HistoryPoint {
                season: 20252026,
                state: b.state.clone(),
                measurement: Some(b),
            },
        ],
        ranked_seasons: 2,
        upper_quintile_seasons: Some(0),
        comparability: String::new(),
    }
}
#[test]
fn lower_rate_can_have_higher_annual_rank_and_zero_is_not_missing() {
    let r = row(measurement(1., Some(40.)), measurement(0., Some(60.)));
    let c = build(&[season(20252026), season(20242025)], &[r]);
    let r = &c[0].rows[0];
    assert_eq!(r.state, "descriptive");
    assert_eq!(r.to_value, Some(0.));
    assert_eq!(r.difference, Some(-1.));
    assert_eq!(r.percentile_point_difference, Some(20.));
    assert_eq!(c[0].from_season, 20242025);
}
#[test]
fn changed_definition_or_context_withholds_difference() {
    for kind in 0..5 {
        let mut a = measurement(1., Some(40.));
        let mut b = measurement(2., Some(50.));
        let s = season(20242025);
        let mut t = season(20252026);
        match kind {
            0 => t.group = Some("Forwards".into()),
            1 => t.situation = Some("all".into()),
            2 => a.unit = "Count".into(),
            3 => b.formula = "other".into(),
            _ => b.scale = 1.,
        }
        let c = build(&[s, t], &[row(a, b)]);
        let r = &c[0].rows[0];
        assert_eq!(r.state, "incompatible");
        assert_eq!(r.difference, None);
        assert_eq!(r.from_value, None);
        assert_eq!(r.percentile_point_difference, None);
    }
}
#[test]
fn missing_player_metric_or_value_never_bridges_or_becomes_zero() {
    for kind in 0..3 {
        let a = season(20242025);
        let mut b = season(20252026);
        let mut r = row(measurement(1., Some(40.)), measurement(2., Some(50.)));
        match kind {
            0 => b.state = "missing_player".into(),
            1 => r.points[1].measurement = None,
            _ => r.points[1].measurement.as_mut().unwrap().value = None,
        }
        let c = build(&[a, b], &[r]);
        assert_eq!(c[0].rows[0].state, "unavailable");
        assert_eq!(c[0].rows[0].difference, None);
    }
}
#[test]
fn unranked_exposure_keeps_raw_difference_without_peer_difference() {
    let c = build(
        &[season(20242025), season(20252026)],
        &[row(measurement(1., Some(40.)), measurement(2., None))],
    );
    assert_eq!(c[0].rows[0].difference, Some(1.));
    assert_eq!(c[0].rows[0].percentile_point_difference, None);
    assert!(c[0].rows[0].note.contains("unranked"));
    assert!(build(&[season(20242025)], &[]).is_empty());
}
