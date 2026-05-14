use serde::{Deserialize, Serialize};

use crate::model::Season;
use crate::view_model::{Completeness, SourceKind, SourceState, ViewContext};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAwardSeasonRow {
    pub season: Season,
    pub game_type_id: u8,
    pub games_played: Option<u32>,
    pub goals: Option<u32>,
    pub assists: Option<u32>,
    pub points: Option<u32>,
    pub plus_minus: Option<i32>,
    pub pim: Option<u32>,
    pub hits: Option<u32>,
    pub blocked_shots: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAwardRow {
    pub trophy: String,
    pub seasons: Vec<PlayerAwardSeasonRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAwardsView {
    pub context: ViewContext,
    pub player_id: u32,
    pub player_name: String,
    pub awards: Vec<PlayerAwardRow>,
}

impl PlayerAwardsView {
    pub fn new(
        mut context: ViewContext,
        player_id: u32,
        player_name: impl Into<String>,
        mut awards: Vec<PlayerAwardRow>,
    ) -> Self {
        awards.sort_by(|a, b| a.trophy.cmp(&b.trophy));
        for award in &mut awards {
            award.seasons.sort_by(|a, b| b.season.0.cmp(&a.season.0));
        }
        context.source_state.push(SourceState {
            source: SourceKind::Career,
            state: Completeness::Complete,
            provenance: None,
            fetched_at: None,
            stale_reason: None,
            message: Some("NHL landing awards[]".to_string()),
        });
        Self {
            context,
            player_id,
            player_name: player_name.into(),
            awards,
        }
    }

    pub fn empty(mut context: ViewContext, player_id: u32, player_name: impl Into<String>) -> Self {
        context
            .source_state
            .push(SourceState::missing(SourceKind::Career));
        Self {
            context,
            player_id,
            player_name: player_name.into(),
            awards: Vec::new(),
        }
    }

    pub fn trophy_count(&self) -> usize {
        self.awards.len()
    }

    pub fn season_count(&self) -> usize {
        self.awards
            .iter()
            .map(|award| award.seasons.len())
            .sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Season;
    use crate::season_stats::SeasonType;
    use crate::view_model::{ViewContext, ViewWindow};

    #[test]
    fn l0_awards_view_sorts_trophies_and_seasons() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = PlayerAwardsView::new(
            context,
            1,
            "Test Player",
            vec![
                PlayerAwardRow {
                    trophy: "Ted Lindsay Award".to_string(),
                    seasons: vec![PlayerAwardSeasonRow {
                        season: Season(20212022),
                        game_type_id: 2,
                        games_played: None,
                        goals: None,
                        assists: None,
                        points: None,
                        plus_minus: None,
                        pim: None,
                        hits: None,
                        blocked_shots: None,
                    }],
                },
                PlayerAwardRow {
                    trophy: "Art Ross Trophy".to_string(),
                    seasons: vec![
                        PlayerAwardSeasonRow {
                            season: Season(20202021),
                            game_type_id: 2,
                            games_played: None,
                            goals: None,
                            assists: None,
                            points: None,
                            plus_minus: None,
                            pim: None,
                            hits: None,
                            blocked_shots: None,
                        },
                        PlayerAwardSeasonRow {
                            season: Season(20222023),
                            game_type_id: 2,
                            games_played: None,
                            goals: None,
                            assists: None,
                            points: None,
                            plus_minus: None,
                            pim: None,
                            hits: None,
                            blocked_shots: None,
                        },
                    ],
                },
            ],
        );
        assert_eq!(view.awards[0].trophy, "Art Ross Trophy");
        assert_eq!(view.awards[0].seasons[0].season, Season(20222023));
        assert_eq!(view.season_count(), 3);
    }
}
