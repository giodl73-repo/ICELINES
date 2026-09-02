use std::collections::BTreeSet;

use icelines_core::{
    TrainingCampExposureBoardView, TrainingCampExposureLane, TrainingCampTradeProtection,
    TrainingCampTransactionAuthorityStatus, TRAINING_CAMP_EXPOSURE_BOARD_SCHEMA,
};

const EPSILON: f64 = 1e-9;

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= EPSILON
}

#[test]
fn sourced_alpha_bravo_context_is_protected_and_team_scoped() {
    let board: TrainingCampExposureBoardView = serde_json::from_str(include_str!(
        "../../examples/icecast-league-bubble-sourced-alp-brv-2026-27.json"
    ))
    .expect("sourced Bubble reference must remain valid UI-neutral JSON");
    let alpha = board.teams.iter().find(|team| team.team == "NYR").unwrap();
    let transaction_review = alpha
        .rows
        .iter()
        .find(|row| row.display_name == "Sample Player 195")
        .unwrap();
    assert_eq!(transaction_review.cap_hit, Some(3_000_000));
    assert_eq!(
        transaction_review.trade_protection,
        TrainingCampTradeProtection::ModifiedNoTrade
    );
    assert_eq!(
        transaction_review.lane,
        TrainingCampExposureLane::TransactionReview
    );

    let protected_player = alpha
        .rows
        .iter()
        .find(|row| row.display_name == "Sample Player 235")
        .unwrap();
    assert_eq!(
        protected_player.trade_protection,
        TrainingCampTradeProtection::NoMove
    );
    assert_eq!(
        protected_player.lane,
        TrainingCampExposureLane::ContractProtected
    );

    let bravo = board.teams.iter().find(|team| team.team == "SEA").unwrap();
    let sourced_player = bravo
        .rows
        .iter()
        .find(|row| row.display_name == "Sample Player 189")
        .unwrap();
    assert_eq!(
        sourced_player.transaction_authority_status,
        TrainingCampTransactionAuthorityStatus::Sourced
    );

    let montreal = board.teams.iter().find(|team| team.team == "MTL").unwrap();
    assert!(montreal
        .rows
        .iter()
        .all(|row| row.display_name != "Joe Veleno"));
    assert!(montreal
        .authority_warnings
        .iter()
        .all(|warning| !warning.contains("multiple league camp pools")));
    assert!(board.teams.iter().all(|team| team
        .authority_warnings
        .iter()
        .all(|warning| { !warning.contains("multiple league camp pools") })));
}

#[test]
fn bubble_reference_ranks_all_32_teams_without_turning_injuries_into_waivers() {
    let board: TrainingCampExposureBoardView = serde_json::from_str(include_str!(
        "../../examples/icecast-league-bubble-2026-27.json"
    ))
    .expect("Bubble reference must remain valid UI-neutral JSON");

    assert_eq!(board.schema, TRAINING_CAMP_EXPOSURE_BOARD_SCHEMA);
    assert_eq!(board.season, 20262027);
    assert_eq!(board.teams.len(), 32);
    assert_eq!(
        board
            .teams
            .iter()
            .map(|team| team.team.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        32
    );

    for team in &board.teams {
        assert_eq!(team.rows.len(), 10, "{} must retain its top 10", team.team);
        assert!(team.valid_trials > 0 && team.valid_trials <= team.trials);
        for (index, row) in team.rows.iter().enumerate() {
            assert_eq!(row.rank, index + 1);
            assert_eq!(
                row.transaction_authority_status,
                TrainingCampTransactionAuthorityStatus::NoRead
            );
            assert!(!matches!(
                row.lane,
                TrainingCampExposureLane::TransactionReview | TrainingCampExposureLane::WaiverWatch
            ));
            assert!(close(
                row.active_probability
                    + row.unavailable_probability
                    + row.selection_loss_probability,
                1.0
            ));
            let expected_score = row.selection_loss_probability * 0.55
                + row.healthy_scratch_probability * 0.30
                + row.prospect_displacement_probability * 0.15;
            assert!(close(row.exposure_score, expected_score.clamp(0.0, 1.0)));
            if row.waiver_exempt {
                assert!(close(row.waiver_exposure_probability, 0.0));
                assert!(close(
                    row.development_assignment_probability,
                    row.selection_loss_probability
                ));
            } else {
                assert!(close(
                    row.waiver_exposure_probability,
                    row.selection_loss_probability
                ));
                assert!(close(row.development_assignment_probability, 0.0));
            }
        }
        assert!(team
            .rows
            .windows(2)
            .all(|rows| rows[0].exposure_score >= rows[1].exposure_score));
    }

    let alpha = board.teams.iter().find(|team| team.team == "NYR").unwrap();
    let scratch_candidate = alpha
        .rows
        .iter()
        .find(|row| row.lane == TrainingCampExposureLane::HealthyScratchRotation)
        .unwrap();
    assert_eq!(
        scratch_candidate.lane,
        TrainingCampExposureLane::HealthyScratchRotation
    );
    assert!(
        scratch_candidate.healthy_scratch_probability
            > scratch_candidate.selection_loss_probability
    );
    assert!(scratch_candidate.unavailable_probability > 0.0);
    assert!(board
        .disclosures
        .iter()
        .any(|line| line.contains("not a trade prediction")));
    assert!(board
        .disclosures
        .iter()
        .any(|line| line.contains("no-read fallback")));
}
