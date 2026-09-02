use icelines_core::{
    build_card_comparison_set, parse_card_document, CardDocumentView, CardSectionView, MetricValue,
};

const NYR_CARD: &str = include_str!("../../examples/team-prognosis-card-alp-2026-27.json");
const SEA_CARD: &str = include_str!("../../examples/team-prognosis-card-brv-2026-27.json");

fn decimal(value: &MetricValue) -> f64 {
    match value {
        MetricValue::Decimal(value) => *value,
        other => panic!("expected decimal metric, got {other:?}"),
    }
}

fn bridge(card: &CardDocumentView) -> &icelines_core::ScenarioBridgeSectionView {
    card.pages
        .iter()
        .flat_map(|page| &page.sections)
        .find_map(|section| match section {
            CardSectionView::ScenarioBridge(bridge) => Some(bridge),
            _ => None,
        })
        .expect("card should contain the internal-ceiling bridge")
}

#[test]
fn canonical_team_cards_are_sealed_and_reconcile_baseline_to_ceiling() {
    for (json, team, path_label, raw_strength) in [
        (NYR_CARD, "NYR", "+15 Path", 15.4855),
        (SEA_CARD, "SEA", "+17 Path", 17.0669),
    ] {
        let card: CardDocumentView = serde_json::from_str(json).unwrap();
        let calculated = card.calculate_fingerprint().unwrap();
        assert_eq!(card.fingerprint, calculated, "{team} fixture seal mismatch");
        card.validate().unwrap();
        assert_eq!(card.pages.len(), 2);
        assert_eq!(card.context.joins.team_ids, [team]);
        assert_eq!(
            card.context.joins.scenario_comparison_key.as_deref(),
            Some("development-variance")
        );
        let bridge = bridge(&card);
        assert!(bridge.title.contains(path_label));
        let points = &bridge.metrics[0];
        let comparison = points.comparison.as_ref().expect("bridge comparison");
        assert!(
            (decimal(&points.metric.value)
                - decimal(&comparison.baseline)
                - decimal(&comparison.delta))
            .abs()
                < 1e-9
        );
        assert!((decimal(&bridge.metrics[3].metric.value) - raw_strength).abs() < 1e-9);
    }
}

#[test]
fn alpha_card_places_sample_player_and_cross_team_comparison_is_core_aligned() {
    let alpha = parse_card_document(NYR_CARD).unwrap();
    let bravo = parse_card_document(SEA_CARD).unwrap();
    let alpha_json = serde_json::to_string(&alpha).unwrap();
    assert!(alpha_json.contains("player:8481789"));
    assert!(alpha_json.contains("Sample Player 392"));

    let comparison = build_card_comparison_set(vec![alpha, bravo]).unwrap();
    assert!(comparison.warnings.is_empty());
    assert_eq!(comparison.documents.len(), 2);
    let points = comparison
        .aligned_metrics
        .iter()
        .find(|metric| metric.metric_key == "baseline_points")
        .expect("baseline points should align");
    assert_eq!(points.values.len(), 2);
    assert_eq!(points.deltas_from_first.len(), 2);
}
