use serde::{Deserialize, Serialize};

use crate::view_model::{Completeness, ScheduledGameInput, SourceKind, SourceState, ViewContext};

const REGULAR_SEASON_OUTLOOK_GAMES: u32 = 82;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TeamScoringOutlookMetric {
    GoalsFor,
    GoalsAgainst,
}

impl TeamScoringOutlookMetric {
    pub fn label(self) -> &'static str {
        match self {
            Self::GoalsFor => "Goals for",
            Self::GoalsAgainst => "Goals against",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamScoringOutlookSourceStatus {
    MissingSource,
    PartialSource,
    Loaded,
}

impl TeamScoringOutlookSourceStatus {
    pub fn from_flags(source_loaded: bool, source_partial: bool) -> Self {
        if !source_loaded {
            Self::MissingSource
        } else if source_partial {
            Self::PartialSource
        } else {
            Self::Loaded
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamScoringOutlookSampleStatus {
    NoGames,
    HasGames,
}

impl TeamScoringOutlookSampleStatus {
    pub fn from_games(games_played: u32) -> Self {
        if games_played == 0 {
            Self::NoGames
        } else {
            Self::HasGames
        }
    }

    pub fn has_games(self) -> bool {
        matches!(self, Self::HasGames)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamScoringOutlookRow {
    pub metric: TeamScoringOutlookMetric,
    pub label: String,
    pub current_total: u32,
    pub games_played: u32,
    pub sample_status: TeamScoringOutlookSampleStatus,
    pub source_status: TeamScoringOutlookSourceStatus,
    pub per_game: Option<f64>,
    pub pace_82: Option<f64>,
    pub projected_finish: Option<f64>,
    pub remaining_games: Option<u32>,
}

impl TeamScoringOutlookRow {
    pub fn new(
        metric: TeamScoringOutlookMetric,
        current_total: u32,
        games_played: u32,
        remaining_games: Option<u32>,
        source_status: TeamScoringOutlookSourceStatus,
    ) -> Self {
        let sample_status = TeamScoringOutlookSampleStatus::from_games(games_played);
        let per_game = sample_status
            .has_games()
            .then_some(current_total as f64 / games_played as f64);
        let pace_82 = per_game.map(|value| value * REGULAR_SEASON_OUTLOOK_GAMES as f64);
        let projected_finish = per_game
            .zip(remaining_games)
            .map(|(value, remaining)| current_total as f64 + value * remaining as f64);
        Self {
            metric,
            label: metric.label().to_string(),
            current_total,
            games_played,
            sample_status,
            source_status,
            per_game,
            pace_82,
            projected_finish,
            remaining_games,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamScoringOutlookRecentForm {
    pub label: String,
    pub games_loaded: u32,
    pub source_status: TeamScoringOutlookSourceStatus,
    pub goals_for: u32,
    pub goals_against: u32,
    pub goal_differential: i32,
    pub goals_for_per_game: Option<f64>,
    pub goals_against_per_game: Option<f64>,
}

impl TeamScoringOutlookRecentForm {
    fn from_games(
        games: &[TeamScoringOutlookGame],
        source_status: TeamScoringOutlookSourceStatus,
    ) -> Self {
        let recent: Vec<&TeamScoringOutlookGame> = games.iter().rev().take(10).collect();
        let games_loaded = recent.len() as u32;
        let goals_for: u32 = recent.iter().map(|game| game.goals_for).sum();
        let goals_against: u32 = recent.iter().map(|game| game.goals_against).sum();
        let goals_for_per_game =
            (games_loaded > 0).then_some(goals_for as f64 / games_loaded as f64);
        let goals_against_per_game =
            (games_loaded > 0).then_some(goals_against as f64 / games_loaded as f64);
        Self {
            label: "recent pressure - last 10 games".to_string(),
            games_loaded,
            source_status,
            goals_for,
            goals_against,
            goal_differential: goals_for as i32 - goals_against as i32,
            goals_for_per_game,
            goals_against_per_game,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamScoringOutlookView {
    pub context: ViewContext,
    pub team: String,
    pub games_played: u32,
    pub remaining_games: Option<u32>,
    pub source_status: TeamScoringOutlookSourceStatus,
    pub sample_status: TeamScoringOutlookSampleStatus,
    pub pace_games: u32,
    pub rows: Vec<TeamScoringOutlookRow>,
    pub recent_form: TeamScoringOutlookRecentForm,
}

impl TeamScoringOutlookView {
    pub fn from_schedule_games(
        mut context: ViewContext,
        team: impl Into<String>,
        source_loaded: bool,
        source_partial: bool,
        games: Vec<ScheduledGameInput>,
        remaining_games: Option<u32>,
    ) -> Self {
        let team = team.into().trim().to_ascii_uppercase();
        let source_status =
            TeamScoringOutlookSourceStatus::from_flags(source_loaded, source_partial);
        context
            .source_state
            .push(schedule_source_state(source_status));
        if source_status == TeamScoringOutlookSourceStatus::MissingSource {
            context.completeness = Completeness::Unavailable;
        } else if source_status == TeamScoringOutlookSourceStatus::PartialSource {
            context.completeness = Completeness::Partial;
        }

        let final_games = if source_loaded {
            final_regular_team_games(&team, games)
        } else {
            Vec::new()
        };
        let games_played = final_games.len() as u32;
        let goals_for: u32 = final_games.iter().map(|game| game.goals_for).sum();
        let goals_against: u32 = final_games.iter().map(|game| game.goals_against).sum();
        let sample_status = TeamScoringOutlookSampleStatus::from_games(games_played);
        let rows = vec![
            TeamScoringOutlookRow::new(
                TeamScoringOutlookMetric::GoalsFor,
                goals_for,
                games_played,
                remaining_games,
                source_status,
            ),
            TeamScoringOutlookRow::new(
                TeamScoringOutlookMetric::GoalsAgainst,
                goals_against,
                games_played,
                remaining_games,
                source_status,
            ),
        ];
        let recent_form = TeamScoringOutlookRecentForm::from_games(&final_games, source_status);

        Self {
            context,
            team,
            games_played,
            remaining_games,
            source_status,
            sample_status,
            pace_games: REGULAR_SEASON_OUTLOOK_GAMES,
            rows,
            recent_form,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TeamScoringOutlookGame {
    date: String,
    game_id: u64,
    goals_for: u32,
    goals_against: u32,
}

fn final_regular_team_games(
    team: &str,
    games: Vec<ScheduledGameInput>,
) -> Vec<TeamScoringOutlookGame> {
    let mut final_games: Vec<TeamScoringOutlookGame> = games
        .into_iter()
        .filter_map(|game| {
            if game.game_type != 2 || !is_final_state(game.game_state.as_deref()) {
                return None;
            }
            let (goals_for, goals_against) = if game.away_abbrev.eq_ignore_ascii_case(team) {
                (game.away_score, game.home_score)
            } else if game.home_abbrev.eq_ignore_ascii_case(team) {
                (game.home_score, game.away_score)
            } else {
                return None;
            };
            Some(TeamScoringOutlookGame {
                date: game.date,
                game_id: game.game_id,
                goals_for: u32::from(goals_for?),
                goals_against: u32::from(goals_against?),
            })
        })
        .collect();
    final_games.sort_by(|a, b| a.date.cmp(&b.date).then(a.game_id.cmp(&b.game_id)));
    final_games
}

fn is_final_state(state: Option<&str>) -> bool {
    matches!(state, Some("FINAL" | "OFF"))
}

fn schedule_source_state(source_status: TeamScoringOutlookSourceStatus) -> SourceState {
    match source_status {
        TeamScoringOutlookSourceStatus::Loaded => SourceState::complete(SourceKind::Schedule),
        TeamScoringOutlookSourceStatus::PartialSource => SourceState {
            source: SourceKind::Schedule,
            state: Completeness::Partial,
            provenance: None,
            fetched_at: None,
            stale_reason: None,
            message: Some("loaded schedule/score window is partial".to_string()),
        },
        TeamScoringOutlookSourceStatus::MissingSource => SourceState::missing(SourceKind::Schedule),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Season;
    use crate::season_stats::SeasonType;
    use crate::view_model::ViewWindow;

    #[test]
    fn l0_team_scoring_outlook_known_goals_for_and_against_pace() {
        let view = fixture_view(true, false, ten_final_games(20, 16), Some(72));
        let goals_for = row(&view, TeamScoringOutlookMetric::GoalsFor);
        let goals_against = row(&view, TeamScoringOutlookMetric::GoalsAgainst);

        // 20 goals for / 10 GP * 82 = 164.0 GF per 82.
        assert_close(goals_for.pace_82, 164.0);
        // Projected finish = current 20 + (20 / 10) * 72 remaining = 164.0.
        assert_close(goals_for.projected_finish, 164.0);
        // 16 goals against / 10 GP * 82 = 131.2 GA per 82.
        assert_close(goals_against.pace_82, 131.2);
        assert_close(goals_against.projected_finish, 131.2);
        assert_eq!(view.games_played, 10);
        assert_eq!(view.recent_form.goal_differential, 4);
    }

    #[test]
    fn l0_team_scoring_outlook_missing_remaining_keeps_pace_but_nulls_finish() {
        let view = fixture_view(true, false, ten_final_games(20, 16), None);
        let goals_for = row(&view, TeamScoringOutlookMetric::GoalsFor);

        assert_close(goals_for.pace_82, 164.0);
        assert_eq!(goals_for.projected_finish, None);
        assert_eq!(goals_for.remaining_games, None);
    }

    #[test]
    fn l0_team_scoring_outlook_zero_games_loaded_has_null_rates() {
        let view = fixture_view(true, false, Vec::new(), Some(82));

        assert_eq!(view.source_status, TeamScoringOutlookSourceStatus::Loaded);
        assert_eq!(view.sample_status, TeamScoringOutlookSampleStatus::NoGames);
        for row in &view.rows {
            assert_eq!(row.per_game, None);
            assert_eq!(row.pace_82, None);
            assert_eq!(row.projected_finish, None);
        }
    }

    #[test]
    fn l0_team_scoring_outlook_loaded_zero_goals_are_real_zero_rate() {
        let games = vec![
            game(1, "2025-10-01", "EDM", "CGY", Some(0), Some(1), "FINAL"),
            game(2, "2025-10-02", "VAN", "EDM", Some(2), Some(0), "FINAL"),
        ];
        let view = fixture_view(true, false, games, Some(80));
        let goals_for = row(&view, TeamScoringOutlookMetric::GoalsFor);

        assert_eq!(goals_for.current_total, 0);
        assert_close(goals_for.pace_82, 0.0);
        assert_close(goals_for.projected_finish, 0.0);
        assert_eq!(view.recent_form.goals_for, 0);
        assert_close(view.recent_form.goals_for_per_game, 0.0);
    }

    #[test]
    fn l0_team_scoring_outlook_partial_source_marks_context_partial() {
        let view = fixture_view(true, true, ten_final_games(20, 16), Some(72));

        assert_eq!(
            view.source_status,
            TeamScoringOutlookSourceStatus::PartialSource
        );
        assert_eq!(view.context.completeness, Completeness::Partial);
        assert_eq!(view.context.source_state[0].state, Completeness::Partial);
        for row in &view.rows {
            assert_eq!(
                row.source_status,
                TeamScoringOutlookSourceStatus::PartialSource
            );
        }
    }

    #[test]
    fn l0_team_scoring_outlook_missing_source_does_not_fabricate_zero_pace() {
        let view = fixture_view(false, false, ten_final_games(20, 16), Some(72));

        assert_eq!(
            view.source_status,
            TeamScoringOutlookSourceStatus::MissingSource
        );
        assert_eq!(view.context.completeness, Completeness::Unavailable);
        assert_eq!(
            view.context.source_state[0].state,
            Completeness::Unavailable
        );
        assert_eq!(view.games_played, 0);
        for row in &view.rows {
            assert_eq!(row.current_total, 0);
            assert_eq!(row.pace_82, None);
            assert_eq!(row.projected_finish, None);
        }
    }

    #[test]
    fn l0_team_scoring_outlook_ignores_non_final_and_non_regular_games() {
        let games = vec![
            game(1, "2025-10-01", "EDM", "CGY", Some(4), Some(2), "FINAL"),
            game(2, "2025-10-02", "EDM", "VAN", Some(9), Some(9), "LIVE"),
            playoff_game(3, "2026-05-01", "EDM", "LAK", Some(5), Some(1)),
        ];
        let view = fixture_view(true, false, games, Some(81));
        let goals_for = row(&view, TeamScoringOutlookMetric::GoalsFor);

        assert_eq!(view.games_played, 1);
        assert_eq!(goals_for.current_total, 4);
        assert_close(goals_for.pace_82, 328.0);
    }

    fn fixture_view(
        source_loaded: bool,
        source_partial: bool,
        games: Vec<ScheduledGameInput>,
        remaining_games: Option<u32>,
    ) -> TeamScoringOutlookView {
        TeamScoringOutlookView::from_schedule_games(
            ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            "edm",
            source_loaded,
            source_partial,
            games,
            remaining_games,
        )
    }

    fn ten_final_games(goals_for: u8, goals_against: u8) -> Vec<ScheduledGameInput> {
        (0..10)
            .map(|idx| {
                let team_score = if idx == 0 { goals_for } else { 0 };
                let opponent_score = if idx == 0 { goals_against } else { 0 };
                game(
                    (idx + 1) as u64,
                    &format!("2025-10-{:02}", idx + 1),
                    "EDM",
                    "CGY",
                    Some(team_score),
                    Some(opponent_score),
                    "FINAL",
                )
            })
            .collect()
    }

    fn playoff_game(
        game_id: u64,
        date: &str,
        away: &str,
        home: &str,
        away_score: Option<u8>,
        home_score: Option<u8>,
    ) -> ScheduledGameInput {
        ScheduledGameInput {
            game_type: 3,
            ..game(game_id, date, away, home, away_score, home_score, "FINAL")
        }
    }

    fn game(
        game_id: u64,
        date: &str,
        away: &str,
        home: &str,
        away_score: Option<u8>,
        home_score: Option<u8>,
        state: &str,
    ) -> ScheduledGameInput {
        ScheduledGameInput {
            game_id,
            date: date.to_string(),
            game_type: 2,
            away_abbrev: away.to_string(),
            away_name: away.to_string(),
            home_abbrev: home.to_string(),
            home_name: home.to_string(),
            start_time_utc: format!("{date}T23:00:00Z"),
            away_score,
            home_score,
            game_state: Some(state.to_string()),
            last_period: None,
            series_game: None,
            away_wins: None,
            home_wins: None,
        }
    }

    fn row(
        view: &TeamScoringOutlookView,
        metric: TeamScoringOutlookMetric,
    ) -> &TeamScoringOutlookRow {
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
}
