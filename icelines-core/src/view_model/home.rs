use serde::{Deserialize, Serialize};

use crate::identity::PlayerId;
use crate::model::{Position, Season, TeamAbbr};
use crate::season_stats::SeasonType;
use crate::stats_repository::{PlayerView, StatsRepository};
use crate::view_model::context::{
    EmptyState, SourceKind, SourceState, ViewContext, ViewWarning, ViewWindow,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeView {
    pub context: ViewContext,
    pub top_skaters: Vec<HomeSkaterRow>,
    pub top_goalies: Vec<HomeGoalieRow>,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl HomeView {
    pub fn from_repository(
        repo: &StatsRepository,
        season: Season,
        season_type: SeasonType,
        goalie_gp_floor: u32,
        limit: usize,
    ) -> Self {
        let mut context = ViewContext::new(ViewWindow::new(season, season_type));
        context
            .source_state
            .push(SourceState::complete(SourceKind::Home));

        let mut skaters: Vec<PlayerView<'_>> = repo.skaters(season, season_type).collect();
        skaters.sort_by(|a, b| {
            b.points()
                .cmp(&a.points())
                .then(b.goals().cmp(&a.goals()))
                .then(a.full_name().cmp(b.full_name()))
        });
        let top_skaters = skaters
            .into_iter()
            .take(limit)
            .map(home_skater_row)
            .collect();

        let mut goalies: Vec<PlayerView<'_>> = repo
            .goalies(season, season_type)
            .filter(|goalie| goalie.gp() >= goalie_gp_floor)
            .collect();
        goalies.sort_by(|a, b| {
            let a_save = a
                .stats
                .goalie
                .as_ref()
                .and_then(|stats| stats.save_pct)
                .unwrap_or(0.0);
            let b_save = b
                .stats
                .goalie
                .as_ref()
                .and_then(|stats| stats.save_pct)
                .unwrap_or(0.0);
            b_save
                .total_cmp(&a_save)
                .then(goalie_wins(b).cmp(&goalie_wins(a)))
                .then(a.full_name().cmp(b.full_name()))
        });
        let top_goalies = goalies
            .into_iter()
            .take(limit)
            .filter_map(home_goalie_row)
            .collect();

        Self {
            context,
            top_skaters,
            top_goalies,
            warnings: Vec::new(),
            empty_state: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeSkaterRow {
    pub player_id: PlayerId,
    pub display_name: String,
    pub position: Position,
    pub team: TeamAbbr,
    pub gp: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeGoalieRow {
    pub player_id: PlayerId,
    pub display_name: String,
    pub team: TeamAbbr,
    pub gp: u32,
    pub wins: u32,
    pub losses: u32,
    pub shutouts: u32,
    pub save_pct: Option<f32>,
    pub goals_against_average: Option<f32>,
}

fn home_skater_row(player: PlayerView<'_>) -> HomeSkaterRow {
    HomeSkaterRow {
        player_id: player.id(),
        display_name: player.full_name().to_string(),
        position: player.position(),
        team: player
            .team()
            .cloned()
            .unwrap_or_else(|| TeamAbbr("UNK".to_string())),
        gp: player.gp(),
        goals: player.goals(),
        assists: player.assists(),
        points: player.points(),
    }
}

fn home_goalie_row(goalie: PlayerView<'_>) -> Option<HomeGoalieRow> {
    let stats = goalie.stats.goalie.as_ref()?;
    Some(HomeGoalieRow {
        player_id: goalie.id(),
        display_name: goalie.full_name().to_string(),
        team: goalie
            .team()
            .cloned()
            .unwrap_or_else(|| TeamAbbr("UNK".to_string())),
        gp: goalie.gp(),
        wins: stats.wins,
        losses: stats.losses,
        shutouts: stats.shutouts,
        save_pct: stats.save_pct,
        goals_against_average: stats.goals_against_average,
    })
}

fn goalie_wins(goalie: &PlayerView<'_>) -> u32 {
    goalie
        .stats
        .goalie
        .as_ref()
        .map(|stats| stats.wins)
        .unwrap_or(0)
}
