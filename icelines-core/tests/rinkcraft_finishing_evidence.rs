use icelines_core::view_model::{
    rinkcraft_finishing as f, rinkcraft_finishing_evidence as e,
    rinkcraft_history::NumericalHistory, rinkcraft_numbers as n,
};
use serde_json::json;
fn setup() -> (NumericalHistory, e::ResearchInput) {
    let sources: Vec<_> = [20232024, 20242025]
        .into_iter()
        .map(|season| e::SourceRecord {
            season,
            source: n::Source {
                url: format!("https://example.com/{season}"),
                sha256: format!("{season:064x}"),
                bytes: 100,
            },
        })
        .collect();
    let catalogs = sources
        .iter()
        .map(|s| {
            let (goals, xg) = if s.season == 20232024 {
                (10., 8.)
            } else {
                (8., 10.)
            };
            n::build(
                &[n::InputRow {
                    player_id: 1,
                    name: "Synthetic".into(),
                    group: "Forwards".into(),
                    games: 30,
                    seconds: 36000.,
                    values: [("goals".into(), goals), ("ixg".into(), xg)]
                        .into_iter()
                        .collect(),
                }],
                s.season,
                &[],
                vec![s.source.clone()],
            )
            .unwrap()
        })
        .collect();
    let input = e::ResearchInput {
        schema: "icelines.finishing-research-input.v1".into(),
        model_id: "neutral fixed model".into(),
        model_config_sha256: "a".repeat(64),
        model: f::Model {
            calibration_season: 20232024,
            shape: 4.,
            rate: 4.,
            forward_factor: 1.,
            defense_factor: 1.2,
        },
        interval_probability: 0.9,
        minimum_target_games: 10,
        minimum_target_minutes: 300.,
        sources,
        records: vec![e::Record {
            target: f::Target {
                player_id: 1,
                season: 20242025,
                position: f::Position::C,
                xg: 10.,
            },
            history: vec![f::HistoryRow {
                player_id: 1,
                season: 20232024,
                position: f::Position::C,
                goals: 10,
                xg: 8.,
            }],
        }],
    };
    (
        NumericalHistory {
            schema: icelines_core::view_model::rinkcraft_history::SCHEMA.into(),
            catalogs,
            finishing_research: Some(serde_json::to_value(&input).unwrap()),
        },
        input,
    )
}
fn read(h: &NumericalHistory) -> n::EvidenceCard {
    h.snapshot(1, None, None, Some(20242025))
        .unwrap()
        .card
        .unwrap()
}
#[test]
fn support_uses_residual_prior_and_each_historical_position() {
    let (mut h, mut input) = setup();
    // Unequal global shape/rate catches accidental use of the global prior.
    input.model.rate = 2.;
    input.records[0].history[0].position = f::Position::D;
    h.catalogs[0].profiles[0].group = "Defense".into();
    h.finishing_research = Some(serde_json::to_value(&input).unwrap());
    let card = read(&h);
    let f = card.finishing_research.unwrap();
    assert_eq!(f.status, "experimental");
    let s = f.support.unwrap();
    assert_eq!(s.prior_shape, 4.);
    assert_eq!(s.prior_rate, 4.);
    assert_eq!(s.rows[0].position_factor, 1.2);
    assert_eq!(s.rows[0].adjusted_xg, 9.6);
    assert_eq!(s.posterior_shape, 14.);
    assert_eq!(s.posterior_rate, 13.6);
    assert!((s.history_weight - 9.6 / 13.6).abs() < 1e-14);
    assert!((f.mean.unwrap() - s.posterior_shape / s.posterior_rate).abs() < 1e-14);
    assert!(
        (f.mean.unwrap() - ((1. - s.history_weight) + s.history_weight * 10. / 9.6)).abs() < 1e-14
    );
}

#[test]
fn absent_support_deserializes_and_unavailable_has_no_ledger() {
    let (mut h, _) = setup();
    let f = read(&h).finishing_research.unwrap();
    let mut legacy = serde_json::to_value(&f).unwrap();
    legacy.as_object_mut().unwrap().remove("support");
    assert!(serde_json::from_value::<e::Readout>(legacy)
        .unwrap()
        .support
        .is_none());
    h.finishing_research = Some(json!({"invalid": true}));
    assert!(read(&h).finishing_research.unwrap().support.is_none());
}
#[test]
fn exact_snapshot_is_source_bound_and_ordinary_metrics_unchanged() {
    let (mut h, _) = setup();
    let with = read(&h);
    let f = with.finishing_research.as_ref().unwrap();
    assert_eq!(f.status, "experimental");
    assert_eq!(f.history_seasons, vec![20232024]);
    assert_eq!(f.history_goals, Some(10));
    assert_eq!(f.mean, Some(14. / 12.));
    assert!(f.lower.unwrap() < f.mean.unwrap() && f.upper.unwrap() > f.mean.unwrap());
    // Target goals are visible in the statistics but cannot enter earlier-history inference.
    let mut altered = with.clone();
    altered
        .metrics
        .iter_mut()
        .find(|m| m.key == "goals-per-60")
        .unwrap()
        .numerator = Some(999.);
    let again = e::project(
        h.finishing_research.as_ref().unwrap(),
        &altered,
        &h.catalogs,
    );
    assert_eq!(again.mean, f.mean);
    assert_eq!(again.lower, f.lower);
    assert_eq!(again.upper, f.upper);
    h.finishing_research = None;
    let without = read(&h);
    assert_eq!(with.metrics, without.metrics);
    assert_eq!(with.facets, without.facets);
    assert!(without.finishing_research.is_none());
}
#[test]
fn invalid_research_does_not_hide_valid_statistics() {
    let (mut h, _) = setup();
    h.finishing_research = Some(json!({"unexpected":true}));
    let c = read(&h);
    assert!(!c.metrics.is_empty());
    assert_eq!(c.finishing_research.unwrap().status, "unavailable");
}
#[test]
fn source_and_history_mismatches_are_withheld() {
    let (h, input) = setup();
    for kind in 0..5 {
        let mut bad = input.clone();
        match kind {
            0 => bad.sources[0].source.sha256 = "f".repeat(64),
            1 => bad.records[0].history[0].goals += 1,
            2 => bad.records[0].target.xg += 1.,
            3 => bad.records[0].history[0].season = 20242025,
            _ => bad.records.push(bad.records[0].clone()),
        };
        let mut h = h.clone();
        h.finishing_research = Some(serde_json::to_value(bad).unwrap());
        let f = read(&h).finishing_research.unwrap();
        assert_eq!(f.status, "unavailable");
        assert!(f.mean.is_none());
    }
}
#[test]
fn no_cross_season_or_player_fallback_and_no_goalie_transfer() {
    let (mut h, mut input) = setup();
    input.records[0].target.season = 20252026;
    h.finishing_research = Some(serde_json::to_value(input).unwrap());
    assert_eq!(read(&h).finishing_research.unwrap().status, "missing");
    let mut card = read(&h);
    card.group = "Goalies".into();
    card.situation = "all".into();
    assert_eq!(
        e::project(&json!(null), &card, &h.catalogs).status,
        "not_applicable"
    );
}
#[test]
fn no_history_is_baseline_only_not_player_skill() {
    let (mut h, mut input) = setup();
    input.records[0].history.clear();
    h.catalogs[0].profiles[0].player_id = 2;
    h.finishing_research = Some(serde_json::to_value(input).unwrap());
    let f = read(&h).finishing_research.unwrap();
    assert_eq!(f.status, "baseline_only");
    assert_eq!(f.mean, Some(1.));
    assert!(f.lower.unwrap() < 1. && f.upper.unwrap() > 1.);
}

#[test]
fn omitted_available_history_and_out_of_qualification_are_withheld() {
    let (mut h, mut input) = setup();
    input.records[0].history.clear();
    h.finishing_research = Some(serde_json::to_value(input).unwrap());
    assert_eq!(read(&h).finishing_research.unwrap().status, "unavailable");
    let (mut h, mut input) = setup();
    input.minimum_target_minutes = 1000.;
    h.finishing_research = Some(serde_json::to_value(input).unwrap());
    let c = read(&h);
    assert!(!c.metrics.is_empty());
    assert_eq!(c.finishing_research.unwrap().status, "unavailable");
}
