use serde::{Deserialize, Serialize};

use crate::model::{GpStatus, MIN_GP};
use crate::projection::per_game_sigma;
use crate::stats_repository::PlayerView;
use crate::view_model::{ViewContext, ViewWindow};

const REGULAR_SEASON_PACE_GAMES: u32 = 82;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlayerScoringPaceMetric {
    Goals,
    Points,
    Shots,
}

impl PlayerScoringPaceMetric {
    pub fn label(self) -> &'static str {
        match self {
            Self::Goals => "Goals",
            Self::Points => "Points",
            Self::Shots => "Shots",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerScoringPaceSampleStatus {
    ZeroGames,
    BelowThreshold,
    Eligible,
}

impl PlayerScoringPaceSampleStatus {
    pub fn from_gp(gp: u32) -> Self {
        match GpStatus::from_gp(gp) {
            GpStatus::Unfetched | GpStatus::Zero => Self::ZeroGames,
            GpStatus::BelowThreshold(_) => Self::BelowThreshold,
            GpStatus::Eligible(_) => Self::Eligible,
        }
    }

    pub fn is_eligible(self) -> bool {
        matches!(self, Self::Eligible)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerScoringPaceRow {
    pub metric: PlayerScoringPaceMetric,
    pub label: String,
    pub current_total: u32,
    pub games_played: u32,
    pub sample_status: PlayerScoringPaceSampleStatus,
    pub per_game: Option<f64>,
    pub pace_82: Option<f64>,
    pub projected_finish: Option<f64>,
    pub confidence_low: Option<f64>,
    pub confidence_high: Option<f64>,
    pub remaining_games: Option<u32>,
}

impl PlayerScoringPaceRow {
    pub fn new(
        metric: PlayerScoringPaceMetric,
        current_total: u32,
        games_played: u32,
        remaining_games: Option<u32>,
    ) -> Self {
        let sample_status = PlayerScoringPaceSampleStatus::from_gp(games_played);
        let per_game = sample_status
            .is_eligible()
            .then_some(current_total as f64 / games_played as f64);
        let pace_82 = per_game.map(|value| value * REGULAR_SEASON_PACE_GAMES as f64);
        let projected_finish = per_game
            .zip(remaining_games)
            .map(|(value, remaining)| current_total as f64 + value * remaining as f64);
        let confidence_band = projected_finish.zip(per_game).zip(remaining_games).map(
            |((projected, value), remaining)| {
                let band = per_game_sigma(value, games_played) * remaining as f64;
                ((projected - band).max(0.0), projected + band)
            },
        );
        Self {
            metric,
            label: metric.label().to_string(),
            current_total,
            games_played,
            sample_status,
            per_game,
            pace_82,
            projected_finish,
            confidence_low: confidence_band.map(|(low, _)| low),
            confidence_high: confidence_band.map(|(_, high)| high),
            remaining_games,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerScoringPaceView {
    pub context: ViewContext,
    pub player_id: u32,
    pub player_name: String,
    pub team: String,
    pub position: String,
    pub games_played: u32,
    pub sample_status: PlayerScoringPaceSampleStatus,
    pub min_games: u32,
    pub pace_games: u32,
    pub remaining_games: Option<u32>,
    pub shot_pct: Option<f64>,
    pub rows: Vec<PlayerScoringPaceRow>,
}

impl PlayerScoringPaceView {
    pub fn from_player(
        context: ViewContext,
        player: &PlayerView<'_>,
        remaining_games: Option<u32>,
    ) -> Self {
        let games_played = player.gp();
        let sample_status = PlayerScoringPaceSampleStatus::from_gp(games_played);
        let shot_pct =
            (player.shots() > 0).then_some(player.goals() as f64 / player.shots() as f64);
        let rows = vec![
            PlayerScoringPaceRow::new(
                PlayerScoringPaceMetric::Goals,
                player.goals(),
                games_played,
                remaining_games,
            ),
            PlayerScoringPaceRow::new(
                PlayerScoringPaceMetric::Points,
                player.points(),
                games_played,
                remaining_games,
            ),
            PlayerScoringPaceRow::new(
                PlayerScoringPaceMetric::Shots,
                player.shots(),
                games_played,
                remaining_games,
            ),
        ];
        Self {
            context,
            player_id: player.id().0,
            player_name: player.full_name().to_string(),
            team: player.team_display().to_string(),
            position: player.position().abbreviation().to_string(),
            games_played,
            sample_status,
            min_games: MIN_GP,
            pace_games: REGULAR_SEASON_PACE_GAMES,
            remaining_games,
            shot_pct,
            rows,
        }
    }

    pub fn context_for_player(player: &PlayerView<'_>) -> ViewContext {
        ViewContext::new(ViewWindow::new(player.season(), player.season_type()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::identity::PlayerId;
    use crate::model::{PaceScore, Position, Season, TeamAbbr};
    use crate::season_stats::{SeasonStatsBuilder, SeasonType, StatTotals, TeamStint};

    #[test]
    fn l0_player_scoring_pace_known_goal_value_and_finish() {
        let view = fixture_view(40, 20, 15, 35, 100, Some(42));
        let goals = row(&view, PlayerScoringPaceMetric::Goals);

        assert_close(goals.pace_82, 41.0);
        assert_close(goals.projected_finish, 41.0);
        assert!(goals.confidence_low.is_some());
        assert!(goals.confidence_high.is_some());
        assert!(goals.confidence_low.unwrap() < goals.projected_finish.unwrap());
        assert!(goals.confidence_high.unwrap() > goals.projected_finish.unwrap());
        assert_close(goals.per_game, 0.5);
        assert_eq!(goals.sample_status, PlayerScoringPaceSampleStatus::Eligible);
        assert_close(view.shot_pct, 0.2);
    }

    #[test]
    fn l0_player_scoring_pace_points_and_shots_rows_use_same_contract() {
        let view = fixture_view(40, 20, 30, 50, 120, Some(42));
        let points = row(&view, PlayerScoringPaceMetric::Points);
        let shots = row(&view, PlayerScoringPaceMetric::Shots);

        assert_close(points.pace_82, 102.5);
        assert_close(shots.pace_82, 246.0);
        assert_close(points.projected_finish, 102.5);
        assert_close(shots.projected_finish, 246.0);
    }

    #[test]
    fn l0_player_scoring_pace_below_threshold_nulls_rates() {
        let view = fixture_view(9, 3, 3, 6, 30, Some(73));

        assert_eq!(
            view.sample_status,
            PlayerScoringPaceSampleStatus::BelowThreshold
        );
        for row in &view.rows {
            assert_eq!(
                row.sample_status,
                PlayerScoringPaceSampleStatus::BelowThreshold
            );
            assert_eq!(row.per_game, None);
            assert_eq!(row.pace_82, None);
            assert_eq!(row.projected_finish, None);
            assert_eq!(row.confidence_low, None);
            assert_eq!(row.confidence_high, None);
        }
    }

    #[test]
    fn l0_player_scoring_pace_exact_min_gp_is_eligible() {
        let view = fixture_view(10, 3, 5, 8, 40, Some(72));
        let goals = row(&view, PlayerScoringPaceMetric::Goals);

        assert_eq!(view.sample_status, PlayerScoringPaceSampleStatus::Eligible);
        assert_close(goals.pace_82, 24.6);
    }

    #[test]
    fn l0_player_scoring_pace_zero_gp_nulls_rates_without_dividing() {
        let view = fixture_view(0, 0, 0, 0, 0, Some(82));

        assert_eq!(view.sample_status, PlayerScoringPaceSampleStatus::ZeroGames);
        for row in &view.rows {
            assert_eq!(row.per_game, None);
            assert_eq!(row.pace_82, None);
            assert_eq!(row.projected_finish, None);
            assert_eq!(row.confidence_low, None);
            assert_eq!(row.confidence_high, None);
        }
        assert_eq!(view.shot_pct, None);
    }

    #[test]
    fn l0_player_scoring_pace_missing_remaining_keeps_pace_but_nulls_finish() {
        let view = fixture_view(40, 20, 15, 35, 100, None);
        let goals = row(&view, PlayerScoringPaceMetric::Goals);

        assert_close(goals.pace_82, 41.0);
        assert_eq!(goals.projected_finish, None);
        assert_eq!(goals.confidence_low, None);
        assert_eq!(goals.confidence_high, None);
        assert_eq!(goals.remaining_games, None);
    }

    #[test]
    fn l0_player_scoring_pace_confidence_band_widens_for_small_samples() {
        let high_gp = fixture_view(60, 30, 30, 60, 180, Some(22));
        let low_gp = fixture_view(10, 5, 5, 10, 30, Some(72));
        let high = row(&high_gp, PlayerScoringPaceMetric::Points);
        let low = row(&low_gp, PlayerScoringPaceMetric::Points);

        assert!(
            band_width(low) > band_width(high),
            "low-sample projection should carry wider confidence band"
        );
    }

    #[test]
    fn l0_player_scoring_pace_zero_shots_has_no_conversion() {
        let view = fixture_view(40, 0, 10, 10, 0, Some(42));
        let shots = row(&view, PlayerScoringPaceMetric::Shots);

        assert_eq!(view.shot_pct, None);
        assert_close(shots.pace_82, 0.0);
    }

    fn fixture_view(
        gp: u32,
        goals: u32,
        assists: u32,
        points: u32,
        shots: u32,
        remaining_games: Option<u32>,
    ) -> PlayerScoringPaceView {
        let player_id = PlayerId(8478402);
        let identity = fixtures::identity(player_id.0).build();
        let stats = SeasonStatsBuilder::new(
            player_id,
            Season(20252026),
            SeasonType::Regular,
            Position::Center,
        )
        .add_team_stint(TeamStint {
            team: TeamAbbr("EDM".to_string()),
            started: Some("2025-10-07".to_string()),
            ended: None,
            gp,
            goals,
            assists,
            points,
            goalie: None,
        })
        .with_totals(StatTotals {
            gp,
            goals,
            assists,
            points,
            shots,
            shooting_pct: (shots > 0).then_some(goals as f32 / shots as f32),
            pace_score: (gp >= MIN_GP).then(|| PaceScore {
                pace_82: points as f64 / gp as f64 * REGULAR_SEASON_PACE_GAMES as f64,
                goals_per_82: goals as f64 / gp as f64 * REGULAR_SEASON_PACE_GAMES as f64,
                raw_points: points,
                gp,
            }),
            ..Default::default()
        })
        .build();
        let repo = fixtures::test_repo_with(identity, stats);
        let player = repo
            .view(player_id, Season(20252026), SeasonType::Regular)
            .expect("fixture player view");
        PlayerScoringPaceView::from_player(
            ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            &player,
            remaining_games,
        )
    }

    fn row(view: &PlayerScoringPaceView, metric: PlayerScoringPaceMetric) -> &PlayerScoringPaceRow {
        view.rows
            .iter()
            .find(|row| row.metric == metric)
            .expect("metric row")
    }

    fn assert_close(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("value present");
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {expected}, got {actual}"
        );
    }

    fn band_width(row: &PlayerScoringPaceRow) -> f64 {
        row.confidence_high.expect("high band") - row.confidence_low.expect("low band")
    }
}
