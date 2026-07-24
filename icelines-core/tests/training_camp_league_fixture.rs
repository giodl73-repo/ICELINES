use std::collections::BTreeSet;

use icelines_core::{
    TrainingCampAuthorityStatus, TrainingCampLeagueForecastView,
    TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA,
};

const EPSILON: f64 = 1e-9;

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= EPSILON
}

#[test]
fn league_training_camp_reference_reconciles_all_32_teams() {
    let league: TrainingCampLeagueForecastView = serde_json::from_str(include_str!(
        "../../examples/icecast-league-training-camp-2026-27.json"
    ))
    .expect("league camp reference must remain valid UI-neutral JSON");

    assert_eq!(league.schema, TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA);
    assert_eq!(league.season, 20262027);
    assert_eq!(league.teams_requested, 32);
    assert_eq!(league.teams_simulated, 32);
    assert_eq!(league.teams_degraded, 0);
    assert_eq!(league.teams_failed, 0);
    assert_eq!(league.teams.len(), 32);

    let teams = league
        .teams
        .iter()
        .map(|team| team.team.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(teams.len(), 32, "team abbreviations must be unique");
    let mut league_player_ids = BTreeSet::new();

    for team in &league.teams {
        assert_eq!(
            team.authority_status,
            TrainingCampAuthorityStatus::ConfirmedPool
        );
        assert!(team.error.is_none(), "{} unexpectedly failed", team.team);
        let forecast = team
            .forecast
            .as_ref()
            .unwrap_or_else(|| panic!("{} is missing its forecast", team.team));
        assert_eq!(forecast.team, team.team);
        assert!(forecast.valid_trials > 0);
        assert_eq!(
            forecast.valid_trials + forecast.incomplete_trials,
            forecast.trials,
            "{} trial accounting does not reconcile",
            team.team
        );
        assert_eq!(forecast.opening_roster_size, 23);
        assert_eq!(forecast.dressed_roster_size, 20);
        assert_eq!(
            forecast.players.len(),
            team.current_roster_candidates
                + team.sourced_overlay_candidates
                + team.fallback_candidates,
            "{} candidate audit does not reconcile",
            team.team
        );

        let make_sum = forecast
            .players
            .iter()
            .map(|player| player.make_probability)
            .sum::<f64>();
        let dressed_sum = forecast
            .players
            .iter()
            .map(|player| player.dressed_probability)
            .sum::<f64>();
        let scratch_sum = forecast
            .players
            .iter()
            .map(|player| player.healthy_scratch_probability)
            .sum::<f64>();
        assert!(close(make_sum, 23.0), "{} active sum={make_sum}", team.team);
        assert!(
            close(dressed_sum, 20.0),
            "{} dressed sum={dressed_sum}",
            team.team
        );
        assert!(
            close(scratch_sum, 3.0),
            "{} scratch sum={scratch_sum}",
            team.team
        );

        let mut player_ids = BTreeSet::new();
        for player in &forecast.players {
            assert!(
                player_ids.insert(player.player_id),
                "{} repeats player {}",
                team.team,
                player.player_id
            );
            assert!(
                league_player_ids.insert(player.player_id),
                "player {} appears in more than one team camp pool",
                player.player_id
            );
            assert!(close(player.make_probability + player.cut_probability, 1.0));
            assert!(close(
                player.make_probability
                    + player.unavailable_probability
                    + player.selection_loss_probability,
                1.0
            ));
            assert!(close(
                player.dressed_probability + player.healthy_scratch_probability,
                player.make_probability
            ));
            assert!(player.waiver_exposure_probability <= player.cut_probability + EPSILON);
            if player.waiver_exempt {
                assert!(close(player.waiver_exposure_probability, 0.0));
            } else {
                assert!(close(
                    player.waiver_exposure_probability,
                    player.selection_loss_probability
                ));
            }
        }

        let sourced = forecast
            .players
            .iter()
            .filter(|player| {
                player
                    .source_league
                    .starts_with("Sourced organizational candidate")
            })
            .collect::<Vec<_>>();
        assert_eq!(sourced.len(), team.sourced_overlay_candidates);
        assert!(sourced
            .iter()
            .all(|player| player.source_league.contains("https://")));

        let modal = forecast
            .modal_opening_roster_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(modal.len(), 23, "{} modal roster is malformed", team.team);
        assert!(modal.iter().all(|player_id| player_ids.contains(player_id)));

        for branch in &forecast.most_common_rosters {
            assert_eq!(branch.forward_ids.len(), 14);
            assert_eq!(branch.defense_ids.len(), 7);
            assert_eq!(branch.goalie_ids.len(), 2);
            assert!(branch.probability > 0.0 && branch.probability <= 1.0);
            assert!(branch.trials > 0 && branch.trials <= forecast.valid_trials);
        }
    }
}
