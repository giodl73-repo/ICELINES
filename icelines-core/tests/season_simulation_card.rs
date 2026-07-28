use std::fs;

use icelines_core::season_stats::SeasonType;
use icelines_core::{
    build_season_simulation_card, CardKind, CardSectionView, EvidenceLabel, Season,
    SeasonSimulationCardInput, TeamSeasonForecastView, TeamSeasonReplayCheckpointTeamRow,
    TeamSeasonReplayCheckpointView, ViewContext, ViewWindow,
};

fn forecast() -> TeamSeasonForecastView {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../examples/icecast-nyr-development-variance-10000-result.json"
    );
    serde_json::from_slice(&fs::read(path).expect("read sealed league result"))
        .expect("parse sealed league result")
}

fn card(
    forecast: TeamSeasonForecastView,
    team: &str,
    name: &str,
) -> icelines_core::CardDocumentView {
    build_season_simulation_card(SeasonSimulationCardInput {
        forecast,
        focus_team: team.to_string(),
        team_name: name.to_string(),
        view: ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular)),
        evidence_at: None,
        calendar_fingerprint: Some("2026-27-nhl-schedule".to_string()),
    })
    .expect("build season simulation card")
}

#[test]
fn focused_cards_share_the_sealed_league_run() {
    let run = forecast();
    let nyr = card(run.clone(), "NYR", "New York Rangers");
    let sea = card(run, "SEA", "Seattle Kraken");

    assert_eq!(nyr.card_kind, CardKind::SeasonSimulation);
    assert_eq!(nyr.pages.len(), 2);
    assert_eq!(sea.pages.len(), 2);
    assert_eq!(
        nyr.context.simulation.parameter_fingerprint,
        sea.context.simulation.parameter_fingerprint
    );
    assert_eq!(nyr.context.simulation.seed, sea.context.simulation.seed);
    assert_eq!(nyr.context.simulation.trials, Some(10_000));
    assert_eq!(nyr.provenance[0].fingerprint, sea.provenance[0].fingerprint);
    assert_ne!(nyr.fingerprint, sea.fingerprint);
    assert!(nyr.pages[1].sections.iter().any(|section| {
        matches!(section, CardSectionView::MetricStrip(strip) if strip.id == "event-path-upside")
    }));
}

#[test]
fn changing_any_league_row_changes_the_run_fingerprint() {
    let run = forecast();
    let original = card(run.clone(), "NYR", "New York Rangers");
    let mut changed = run;
    changed
        .teams
        .iter_mut()
        .find(|row| row.team == "SEA")
        .unwrap()
        .average_points += 0.1;
    let changed = card(changed, "NYR", "New York Rangers");

    assert_ne!(
        original.context.simulation.parameter_fingerprint,
        changed.context.simulation.parameter_fingerprint
    );
}

#[test]
fn sealed_nyr_and_sea_fixtures_parse_and_share_the_run() {
    let nyr = icelines_core::parse_card_document(include_str!(
        "../../examples/season-simulation-card-nyr-2026-27.json"
    ))
    .unwrap();
    let sea = icelines_core::parse_card_document(include_str!(
        "../../examples/season-simulation-card-sea-2026-27.json"
    ))
    .unwrap();
    assert_eq!(
        nyr.context.simulation.parameter_fingerprint,
        sea.context.simulation.parameter_fingerprint
    );
    assert_ne!(nyr.fingerprint, sea.fingerprint);
}

#[test]
fn completed_2024_replay_preserves_confirmed_actuals_and_calibration() {
    let nyr = icelines_core::parse_card_document(include_str!(
        "../../examples/season-simulation-card-nyr-2024-25.json"
    ))
    .unwrap();
    assert_eq!(nyr.context.view.window.season.0, 20242025);
    assert_eq!(nyr.context.simulation.trials, Some(1_000));
    let strips = nyr.pages[1]
        .sections
        .iter()
        .filter_map(|section| match section {
            CardSectionView::MetricStrip(strip) => Some(strip),
            _ => None,
        })
        .collect::<Vec<_>>();
    let actual = strips
        .iter()
        .find(|strip| strip.id == "actual-team-result")
        .unwrap();
    assert_eq!(actual.metrics[0].display_text, "39");
    assert_eq!(actual.metrics[3].display_text, "85");
    assert!(actual
        .metrics
        .iter()
        .all(|metric| metric.evidence_label == EvidenceLabel::Confirmed));
    let calibration = strips
        .iter()
        .find(|strip| strip.id == "replay-calibration")
        .unwrap();
    assert!(calibration.metrics.iter().any(|metric| {
        metric.metric.key.0 == "calibration_intercept" && metric.display_text == "-0.378"
    }));
    assert!(calibration.metrics.iter().any(|metric| {
        metric.metric.key.0 == "calibration_slope" && metric.display_text == "4.417"
    }));
    assert!(calibration.metrics.iter().any(|metric| {
        metric.metric.key.0 == "calibration_slope_ci95_lower" && metric.display_text == "2.380"
    }));
    assert!(calibration.metrics.iter().any(|metric| {
        metric.metric.key.0 == "calibration_slope_ci95_upper" && metric.display_text == "6.454"
    }));
    assert!(strips.iter().any(|strip| strip.id == "replay-best-blend"));
}

#[test]
fn point_in_time_card_names_its_cutoff_and_has_a_unique_document_id() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../examples/icecast-2024-25-replay-1000-result.json"
    );
    let mut forecast: TeamSeasonForecastView =
        serde_json::from_slice(&fs::read(path).expect("read historical league result"))
            .expect("parse historical league result");
    let cutoff = chrono::NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
    forecast.as_of_date = Some(cutoff);
    forecast.replay_checkpoint = Some(TeamSeasonReplayCheckpointView {
        as_of_date: cutoff,
        league_completed_games: 800,
        league_remaining_games: 512,
        teams: vec![TeamSeasonReplayCheckpointTeamRow {
            team: "NYR".to_string(),
            completed_games: 50,
            remaining_games: 32,
            wins: 24,
            losses: 20,
            overtime_losses: 6,
            standings_points: 54,
            expected_remaining_wins: 16.5,
            expected_remaining_losses: 12.0,
            expected_remaining_overtime_losses: 3.5,
            expected_remaining_points: 36.5,
        }],
    });
    let document = build_season_simulation_card(SeasonSimulationCardInput {
        forecast,
        focus_team: "NYR".to_string(),
        team_name: "New York Rangers".to_string(),
        view: ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular)),
        evidence_at: None,
        calendar_fingerprint: Some("2024-25-nhl-schedule".to_string()),
    })
    .expect("build as-of season card");

    assert_eq!(
        document.document_id,
        "season-simulation:NYR:20242025:through:2025-01-31"
    );
    assert!(document
        .subtitle
        .as_deref()
        .unwrap()
        .contains("through 2025-01-31"));
    let titles = document.pages[1]
        .sections
        .iter()
        .filter_map(|section| match section {
            CardSectionView::MetricStrip(strip) => strip.title.as_deref(),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(titles.contains(&"Actual team result through 2025-01-31"));
    assert!(titles.contains(&"Calibration through 2025-01-31"));
    let checkpoint = document.pages[0]
        .sections
        .iter()
        .find_map(|section| match section {
            CardSectionView::MetricStrip(strip) if strip.id == "as-of-checkpoint" => Some(strip),
            _ => None,
        })
        .expect("scoreboard checkpoint strip");
    assert_eq!(
        checkpoint.title.as_deref(),
        Some("Actual checkpoint through 2025-01-31")
    );
    assert_eq!(checkpoint.metrics[0].display_text, "50");
    assert_eq!(checkpoint.metrics[4].display_text, "54");
    assert_eq!(checkpoint.metrics[5].display_text, "32");
    let remainder = document.pages[0]
        .sections
        .iter()
        .find_map(|section| match section {
            CardSectionView::MetricStrip(strip) if strip.id == "projected-remainder" => Some(strip),
            _ => None,
        })
        .expect("scoreboard projected remainder strip");
    assert_eq!(
        remainder.title.as_deref(),
        Some("Expected rest of season after 2025-01-31")
    );
    assert_eq!(remainder.metrics[0].display_text, "16.5");
    assert_eq!(remainder.metrics[3].display_text, "36.5");
}
