use std::path::PathBuf;

use icelines_core::{
    build_trade_lineup_scenario, TeamSeasonForecastMovementView, TeamSeasonScenario,
    TeamSeasonScenarioDevelopmentCalibrationView, TeamSeasonScenarioEventKind,
    TeamSeasonScenarioProbabilityAuthorityStatus, TradeLineupScenarioInput,
    TradeMarketEvaluationView, TradeScoutView, TRADE_MARKET_EVALUATION_SCHEMA, TRADE_SCOUT_SCHEMA,
};
use serde::de::DeserializeOwned;

fn fixture<T: DeserializeOwned>(name: &str) -> T {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join(name);
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|error| {
        panic!("read Wright/Schneider fixture {}: {error}", path.display())
    }))
    .unwrap_or_else(|error| panic!("parse Wright/Schneider fixture {}: {error}", path.display()))
}

#[test]
fn wright_schneider_market_is_ready_fair_and_mutually_useful() {
    let market: TradeMarketEvaluationView =
        fixture("icecast-nyr-wright-trade-market-evaluation-2026-27.json");
    assert_eq!(market.schema, TRADE_MARKET_EVALUATION_SCHEMA);
    assert_eq!(market.proposals.len(), 1);

    let proposal = &market.proposals[0];
    assert_eq!(proposal.buyer, "NYR");
    assert_eq!(proposal.seller, "SEA");
    assert!(proposal.transaction_ready);
    assert!(proposal.mutually_beneficial);
    assert!(proposal.fairness_score > 0.98);
    assert!((proposal.feasibility_probability - 0.16).abs() < 1e-12);
    assert_eq!(proposal.buyer_cap_space_delta, Some(4.613_334));
    assert_eq!(proposal.seller_cap_space_delta, Some(-4.613_334));

    let nyr: TradeLineupScenarioInput =
        fixture("icecast-nyr-trade-lineup-wright-for-schneider-2026-27.json");
    let sea: TradeLineupScenarioInput =
        fixture("icecast-sea-trade-lineup-schneider-for-wright-2026-27.json");
    let nyr = build_trade_lineup_scenario(nyr).expect("recompute Rangers lineup impact");
    let sea = build_trade_lineup_scenario(sea).expect("recompute Kraken lineup impact");
    let attached = proposal
        .lineup_impact
        .as_ref()
        .expect("attached lineup impact");

    assert_eq!(attached.buyer, nyr);
    assert_eq!(attached.seller, sea);
    assert!((attached.buyer.strength_delta + 1.6).abs() < 1e-9);
    assert!((attached.seller.strength_delta - 0.5).abs() < 1e-9);

    let season = proposal
        .season_forecast_impact
        .as_ref()
        .expect("attached paired season impact");
    assert_eq!(season.buyer.team, "NYR");
    assert_eq!(season.seller.team, "SEA");
    assert!((season.buyer.average_points_delta + 0.4591).abs() < 1e-9);
    assert!((season.buyer.playoff_probability_delta + 0.021).abs() < 1e-9);
    assert!((season.buyer.stanley_cup_probability_delta + 0.0057).abs() < 1e-9);
    assert!((season.seller.average_points_delta - 0.147).abs() < 1e-9);
}

#[test]
fn wright_scout_stops_at_schneider_without_inventing_a_sweetener() {
    let scout: TradeScoutView = fixture("icecast-nyr-wright-trade-scout-board-2026-27.json");
    assert_eq!(scout.schema, TRADE_SCOUT_SCHEMA);
    assert_eq!(scout.candidates.len(), 1);

    let candidate = &scout.candidates[0];
    assert_eq!(candidate.label, "Shane Wright");
    let ladder = &candidate.negotiation;
    for package in [
        &ladder.opening_offer,
        &ladder.fair_midpoint,
        &ladder.maximum_acceptable,
    ] {
        assert_eq!(package.assets_to_seller, ["Braden Schneider"]);
        assert!(package.market_value <= ladder.walk_away_market_value);
    }
}

#[test]
fn wright_breakout_sensitivity_is_paired_and_monotone_for_nyr() {
    let cases = [
        (
            "icecast-nyr-wright-sealed-trade-movement-2026-27.json",
            25.3,
            -0.4591,
            -0.021,
            -0.0057,
        ),
        (
            "icecast-nyr-wright-moderate-breakout-movement-2026-27.json",
            30.0,
            0.861,
            0.0335,
            0.0082,
        ),
        (
            "icecast-nyr-wright-strong-breakout-movement-2026-27.json",
            35.0,
            2.2526,
            0.0847,
            0.0198,
        ),
        (
            "icecast-nyr-wright-star-breakout-movement-2026-27.json",
            40.0,
            3.6424,
            0.1277,
            0.038,
        ),
        (
            "icecast-nyr-wright-cup-threshold-movement-2026-27.json",
            43.2,
            4.568,
            0.1576,
            0.0483,
        ),
    ];
    let mut previous_points_delta = f64::NEG_INFINITY;
    for (name, player_score, points, playoffs, cup) in cases {
        let movement: TeamSeasonForecastMovementView = fixture(name);
        assert_eq!(movement.trials, 10_000);
        assert_eq!(movement.seed, 20_262_027);
        let nyr = movement
            .teams
            .iter()
            .find(|row| row.team == "NYR")
            .expect("Rangers movement row");
        assert!((nyr.average_points_delta - points).abs() < 1e-9);
        assert!((nyr.playoff_probability_delta - playoffs).abs() < 1e-9);
        assert!((nyr.stanley_cup_probability_delta - cup).abs() < 1e-9);
        assert!(nyr.average_points_delta > previous_points_delta);
        previous_points_delta = nyr.average_points_delta;

        let label = movement.later_label.as_deref().expect("scenario label");
        assert!(label.contains(&player_score.to_string()));
    }
}

#[test]
fn balanced_breakout_portfolio_separates_expected_path_from_all_hit_ceiling() {
    let movement: TeamSeasonForecastMovementView =
        fixture("icecast-nyr-wright-balanced-breakout-portfolio-movement-2026-27.json");
    let nyr = movement
        .teams
        .iter()
        .find(|row| row.team == "NYR")
        .expect("Rangers movement row");
    assert!((nyr.average_points_delta - 1.1561).abs() < 1e-9);
    assert!((nyr.playoff_probability_delta - 0.0425).abs() < 1e-9);
    assert!((nyr.stanley_cup_probability_delta - 0.0067).abs() < 1e-9);

    let scenario: TeamSeasonScenario =
        fixture("icecast-nyr-wright-balanced-breakout-portfolio-scenario-2026-27.json");
    let breakout_events = scenario
        .events
        .iter()
        .filter(|event| event.kind == TeamSeasonScenarioEventKind::Form)
        .collect::<Vec<_>>();
    assert_eq!(breakout_events.len(), 5);
    assert!(breakout_events
        .iter()
        .all(|event| event.correlation_key.is_none()));
    let all_hit_probability = breakout_events
        .iter()
        .map(|event| event.occurrence_probability)
        .product::<f64>();
    assert!((all_hit_probability - 0.0021).abs() < 1e-12);
    assert!(
        (breakout_events
            .iter()
            .map(|event| event.strength_delta)
            .sum::<f64>()
            - 17.9)
            .abs()
            < 1e-12
    );
}

#[test]
fn balanced_portfolio_exposes_historical_and_uncalibrated_probability_authority() {
    let calibrated: TeamSeasonScenarioDevelopmentCalibrationView =
        fixture("icecast-nyr-wright-balanced-breakout-portfolio-calibrated-2026-27.json");
    assert_eq!(calibrated.source_transitions, 11_156);
    assert_eq!(calibrated.calibrated_events, 4);
    assert_eq!(calibrated.uncalibrated_events, 1);

    let smits = calibrated
        .probability_authority
        .iter()
        .find(|row| row.event_id == "nyr-smits-defense-hit")
        .expect("Smits probability authority");
    assert_eq!(
        smits.status,
        TeamSeasonScenarioProbabilityAuthorityStatus::UncalibratedScenarioAssumption
    );
    assert_eq!(smits.applied_probability, 0.20);
    assert!(smits.cohort.is_none());

    let historical = calibrated
        .probability_authority
        .iter()
        .filter(|row| {
            row.status == TeamSeasonScenarioProbabilityAuthorityStatus::HistoricalDevelopmentCohort
        })
        .collect::<Vec<_>>();
    assert_eq!(historical.len(), 4);
    assert!(historical.iter().all(|row| row
        .cohort
        .as_ref()
        .is_some_and(|cohort| cohort.sample_size >= 140)));
    let all_hit_probability = calibrated
        .scenario
        .events
        .iter()
        .filter(|event| event.kind == TeamSeasonScenarioEventKind::Form)
        .map(|event| event.occurrence_probability)
        .product::<f64>();
    assert!((all_hit_probability - 0.001_757_117_370_794_63).abs() < 1e-15);

    let movement: TeamSeasonForecastMovementView =
        fixture("icecast-nyr-wright-balanced-breakout-portfolio-calibrated-movement-2026-27.json");
    let nyr = movement
        .teams
        .iter()
        .find(|row| row.team == "NYR")
        .expect("Rangers calibrated movement row");
    assert!((nyr.average_points_delta - 1.1538).abs() < 1e-9);
    assert!((nyr.playoff_probability_delta - 0.0425).abs() < 1e-9);
    assert!((nyr.stanley_cup_probability_delta - 0.0065).abs() < 1e-9);
}
