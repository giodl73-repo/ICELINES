use serde::{Deserialize, Serialize};

use crate::view_model::context::{
    EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyLeagueView {
    pub context: ViewContext,
    pub active_league: Option<String>,
    pub leagues: Vec<FantasyLeagueRow>,
    pub teams: Vec<FantasyLeagueTeamRow>,
    pub user_team: Option<String>,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl FantasyLeagueView {
    pub fn from_rows(
        mut context: ViewContext,
        active_league: Option<String>,
        leagues: Vec<FantasyLeagueInput>,
        teams: Vec<FantasyLeagueTeamInput>,
    ) -> Self {
        context
            .source_state
            .push(SourceState::complete(SourceKind::FantasyImport));

        let rows = leagues
            .into_iter()
            .map(|league| FantasyLeagueRow {
                name: league.name,
                scoring_scheme: league.scoring_scheme,
                is_active: league.is_active,
                team_count: league.team_count,
            })
            .collect::<Vec<_>>();
        let team_rows = teams
            .into_iter()
            .map(|team| FantasyLeagueTeamRow {
                name: team.name,
                owner: team.owner,
                is_user_team: team.is_user_team,
                player_count: team.player_count,
            })
            .collect::<Vec<_>>();
        let user_team = team_rows
            .iter()
            .find(|team| team.is_user_team)
            .map(|team| team.name.clone());
        let empty_state = if rows.is_empty() {
            Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No fantasy leagues".to_string(),
                detail: Some(
                    "Create or import a fantasy league to populate this view.".to_string(),
                ),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Self {
            context,
            active_league,
            leagues: rows,
            teams: team_rows,
            user_team,
            warnings: Vec::new(),
            empty_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyLeagueInput {
    pub name: String,
    pub scoring_scheme: String,
    pub is_active: bool,
    pub team_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyLeagueTeamInput {
    pub name: String,
    pub owner: String,
    pub is_user_team: bool,
    pub player_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyLeagueRow {
    pub name: String,
    pub scoring_scheme: String,
    pub is_active: bool,
    pub team_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyLeagueTeamRow {
    pub name: String,
    pub owner: String,
    pub is_user_team: bool,
    pub player_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Season;
    use crate::season_stats::SeasonType;
    use crate::view_model::{ViewContext, ViewWindow};

    #[test]
    fn fantasy_league_view_marks_active_and_user_team() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = FantasyLeagueView::from_rows(
            context,
            Some("Office League".to_string()),
            vec![FantasyLeagueInput {
                name: "Office League".to_string(),
                scoring_scheme: "yahoo-standard".to_string(),
                is_active: true,
                team_count: 2,
            }],
            vec![
                FantasyLeagueTeamInput {
                    name: "My Team".to_string(),
                    owner: "Me".to_string(),
                    is_user_team: true,
                    player_count: 3,
                },
                FantasyLeagueTeamInput {
                    name: "Rival".to_string(),
                    owner: "Them".to_string(),
                    is_user_team: false,
                    player_count: 2,
                },
            ],
        );

        assert_eq!(view.active_league.as_deref(), Some("Office League"));
        assert_eq!(view.user_team.as_deref(), Some("My Team"));
        assert_eq!(view.leagues[0].scoring_scheme, "yahoo-standard");
        assert_eq!(view.teams.len(), 2);
        assert!(view.empty_state.is_none());
    }
}
