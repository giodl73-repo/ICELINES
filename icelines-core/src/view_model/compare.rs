use serde::{Deserialize, Serialize};

use crate::identity::PlayerId;
use crate::model::Season;
use crate::season_stats::SeasonType;
use crate::stats_repository::StatsRepository;
use crate::view_model::context::{
    Completeness, EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
    ViewWindow,
};
use crate::view_model::player_card::PlayerCardView;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompareView {
    pub context: ViewContext,
    pub a: Option<PlayerCardView>,
    pub b: Option<PlayerCardView>,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl CompareView {
    pub fn from_repository(
        repo: &StatsRepository,
        a: Option<PlayerId>,
        b: Option<PlayerId>,
        season: Season,
        season_type: SeasonType,
    ) -> Self {
        let has_window = repo.has_window(season, season_type);
        let mut context = ViewContext::new(ViewWindow::new(season, season_type));
        if !has_window {
            context.completeness = Completeness::Unavailable;
            context
                .source_state
                .push(SourceState::missing(SourceKind::Roster));
        }

        let a = a.and_then(|id| PlayerCardView::from_repository(repo, id, season, season_type));
        let b = b.and_then(|id| PlayerCardView::from_repository(repo, id, season, season_type));
        let empty_state = if a.is_none() && b.is_none() {
            Some(EmptyState {
                kind: if has_window {
                    EmptyKind::NoRows
                } else {
                    EmptyKind::MissingSource
                },
                title: "No comparable players".to_string(),
                detail: Some("No resolved player ids produced compare cards.".to_string()),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Self {
            context,
            a,
            b,
            warnings: Vec::new(),
            empty_state,
        }
    }
}
