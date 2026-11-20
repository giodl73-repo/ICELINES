use icelines_core::view_model::{
    rinkcraft_capability_index::rules, rinkcraft_task_measurements::*,
};
use std::collections::BTreeMap;

fn fixture() -> Input {
    let r = rules();
    let components = r
        .attributes
        .iter()
        .filter(|a| a.branch == "skater")
        .flat_map(|a| a.components.iter().map(|c| c.id.clone()))
        .collect::<Vec<_>>();
    let tasks = components
        .iter()
        .map(|id| Task {
            component_id: id.clone(),
            opportunity_rule: "Synthetic registered task".into(),
            success_rule: "Synthetic task target met".into(),
            attribution_rule: "Synthetic actor resolved".into(),
            channels: [(
                "test_value".into(),
                Channel {
                    unit: "synthetic_unit".into(),
                    minimum: 0.,
                    maximum: 100.,
                    definition: "Synthetic numerical witness, no NHL measurement".into(),
                },
            )]
            .into_iter()
            .collect(),
        })
        .collect();
    let games = (0..9)
        .map(|i| Game {
            id: i + 1,
            date: format!("2025-01-{:02}", i + 1).parse().unwrap(),
            split: [Split::TrainEarly, Split::TrainLate, Split::HeldOut][i as usize / 3],
        })
        .collect::<Vec<_>>();
    let outcomes = [
        Outcome::Success,
        Outcome::TechnicalFailure,
        Outcome::ChoiceFailure,
        Outcome::OpponentWins,
        Outcome::TeammateFailure,
        Outcome::JointOrAmbiguous,
        Outcome::Untested,
        Outcome::Censored,
        Outcome::OutOfScope,
    ];
    let mut observations = Vec::new();
    for component in &components {
        for game in &games {
            for (i, outcome) in outcomes.iter().enumerate() {
                let resolved = i < 5;
                observations.push(Observation {
                    player_id: 1,
                    component_id: component.clone(),
                    game_id: game.id,
                    episode_id: format!("episode-{i}"),
                    opportunity_id: format!("opportunity-{i}"),
                    context_cell: "open".into(),
                    source_id: "synthetic".into(),
                    source_locator: format!("game/{}/row/{i}", game.id),
                    assessor: "test".into(),
                    start_seconds: i as f64,
                    end_seconds: i as f64 + 1.,
                    five_on_five_both_goalies: i != 8,
                    outcome: *outcome,
                    choice_coded_before_outcome: true,
                    measurements: if resolved {
                        [("test_value".into(), i as f64)].into_iter().collect()
                    } else {
                        BTreeMap::new()
                    },
                    missing_measurements: BTreeMap::new(),
                });
            }
        }
    }
    Input {
        schema: "rinkcraft.task-measurement-input.v1".into(),
        protocol: Protocol {
            id: "synthetic-complete-battery".into(),
            registry_sha256: r.source_registry_sha256,
            selection_rule: "Complete synthetic event census, no real players".into(),
            selection_status: "synthetic_test".into(),
            window: "synthetic 2025-01".into(),
            context_cells: [(
                "open".into(),
                ["pressure", "zone", "task_variant", "support", "duty"]
                    .into_iter()
                    .map(|k| (k.into(), "synthetic".into()))
                    .collect(),
            )]
            .into_iter()
            .collect(),
            games,
            players: vec![
                Player {
                    id: 1,
                    position: "C".into(),
                },
                Player {
                    id: 2,
                    position: "D".into(),
                },
            ],
            sources: vec![Source {
                id: "synthetic".into(),
                lineage: "synthetic-primary".into(),
                sha256: "a".repeat(64),
                url: "urn:rinkcraft:test".into(),
            }],
            tasks,
        },
        observations,
    }
}
fn run(input: &Input) -> Result<Report, String> {
    measure(&serde_json::to_string(input).unwrap())
}

#[test]
fn full_battery_counts_all_outcomes_preserves_zero_and_missing_players() {
    let input = fixture();
    let report = run(&input).unwrap();
    assert_eq!(report.observations, 23 * 9 * 9);
    assert_eq!(report.cells.len(), 23 * 3);
    assert_eq!(report.coverage.len(), 23 * 2);
    assert_eq!(report.linked_episodes, 9 * 9); // Not 23 independent replicas.
    for cell in &report.cells {
        assert_eq!(cell.counts.eligible, 24);
        assert_eq!(cell.counts.resolved, 15);
        assert_eq!(cell.counts.non_success, 12);
        assert_eq!(cell.counts.out_of_scope, 3);
        assert_eq!(cell.success_per_resolved, Some(0.2));
        assert!(cell.success_rate_game_jackknife_se.unwrap().abs() < 1e-14);
        let values = &cell.channels["test_value"];
        assert_eq!(values.measured, 15);
        assert_eq!(values.minimum, Some(0.));
        assert_eq!(values.median, Some(2.));
        assert_eq!(values.p90, Some(4.));
    }
    assert!(report
        .coverage
        .iter()
        .filter(|c| c.player_id == 2)
        .all(|c| c.resolved == 0 && c.measurement_status == "no_resolved_opportunities"));
    let encoded = serde_json::to_value(&report).unwrap();
    assert!(encoded.get("score").is_none());
}

#[test]
fn missing_channels_and_unresolved_are_never_zero_filled() {
    let mut input = fixture();
    input.observations[0].measurements.clear();
    assert!(run(&input).unwrap_err().contains("explicit missingness"));
    input.observations[0]
        .missing_measurements
        .insert("test_value".into(), "endpoint not measurable".into());
    let report = run(&input).unwrap();
    let cell = report
        .cells
        .iter()
        .find(|c| {
            c.component_id == input.observations[0].component_id && c.split == Split::TrainEarly
        })
        .unwrap();
    assert_eq!(cell.channels["test_value"].missing, 1);
    assert_eq!(cell.channels["test_value"].measured, 14);
    input.observations[0]
        .measurements
        .insert("test_value".into(), 0.);
    assert!(run(&input).is_err()); // Value and missingness cannot coexist.
    input.observations[0].measurements.clear();
    input.observations[5]
        .measurements
        .insert("test_value".into(), 0.);
    assert!(run(&input).unwrap_err().contains("Unresolved"));
}

#[test]
fn duplicate_temporal_scope_and_reading_guards() {
    let mut input = fixture();
    input.observations.push(input.observations[0].clone());
    assert!(run(&input)
        .unwrap_err()
        .contains("Duplicate actor-component"));
    input.observations.pop();
    input.protocol.games[8].date = input.protocol.games[0].date;
    assert!(run(&input).unwrap_err().contains("chronological"));
    input = fixture();
    input.observations[0].five_on_five_both_goalies = false;
    assert!(run(&input).unwrap_err().contains("manpower"));
    input = fixture();
    let reading = input
        .observations
        .iter_mut()
        .find(|r| r.component_id.starts_with("REA-"))
        .unwrap();
    reading.choice_coded_before_outcome = false;
    assert!(run(&input).unwrap_err().contains("pre-outcome"));
    input = fixture();
    input.protocol.tasks.pop();
    assert!(run(&input).unwrap_err().contains("23 task"));
    input = fixture();
    input.protocol.players[0].position = "G".into();
    assert!(run(&input).unwrap_err().contains("goalies"));
}

#[test]
fn empty_observations_report_absence_for_every_component() {
    let mut input = fixture();
    input.observations.clear();
    let report = run(&input).unwrap();
    assert_eq!(report.coverage.len(), 46);
    assert!(report
        .coverage
        .iter()
        .all(|c| c.resolved == 0 && c.splits_with_resolved.is_empty()));
    assert!(report.cells.is_empty());
}

#[test]
fn game_jackknife_is_sensitive_to_cluster_outcomes_and_not_row_order() {
    let mut input = fixture();
    let component = input.observations[0].component_id.clone();
    for r in &mut input.observations {
        if r.component_id == component && r.game_id == 1 && r.outcome == Outcome::TechnicalFailure {
            r.outcome = Outcome::Success;
        }
    }
    let report = run(&input).unwrap();
    let cell = report
        .cells
        .iter()
        .find(|c| c.component_id == component && c.split == Split::TrainEarly)
        .unwrap();
    // Per-game rates .4,.2,.2; leave-one-game rates .2,.3,.3.
    assert!((cell.success_rate_game_jackknife_se.unwrap() - 1. / 15.).abs() < 1e-12);
    input.observations.reverse();
    let reversed = run(&input).unwrap();
    assert_eq!(
        serde_json::to_value(&report.cells).unwrap(),
        serde_json::to_value(&reversed.cells).unwrap()
    );
}

#[test]
fn source_channels_and_sampling_frame_are_checked() {
    let mut input = fixture();
    input.observations[0].source_id = "unregistered-source".into();
    assert!(run(&input).is_err());
    input = fixture();
    input.observations[0]
        .measurements
        .insert("test_value".into(), 101.);
    assert!(run(&input).is_err());
    input = fixture();
    input.observations[0].game_id = 1000;
    assert!(run(&input).is_err());
    input = fixture();
    input.protocol.sources[0].sha256 = "not-a-sha256".into();
    assert!(run(&input).is_err());
    if let Ok(path) = std::env::var("ICELINES_TASK_TEST_FIXTURE") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        // Explicit test-only export. Never an observation collection command.
        file.write_all(&serde_json::to_vec_pretty(&fixture()).unwrap())
            .unwrap();
    }
}
