use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::view_model::context::{
    EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FavoritesView {
    pub context: ViewContext,
    pub group: String,
    pub rows: Vec<FavoriteMemberRow>,
    pub player_count: usize,
    pub team_count: usize,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl FavoritesView {
    pub fn from_members(
        mut context: ViewContext,
        group: String,
        members: Vec<FavoriteMemberInput>,
        stat_lines: HashMap<String, String>,
    ) -> Self {
        context
            .source_state
            .push(SourceState::complete(SourceKind::Favorites));

        let rows: Vec<FavoriteMemberRow> = members
            .into_iter()
            .map(|member| FavoriteMemberRow {
                kind: member.kind,
                key: member.key.clone(),
                stat_line: stat_lines.get(&member.key).cloned(),
            })
            .collect();
        let player_count = rows.iter().filter(|row| row.kind == "player").count();
        let team_count = rows.iter().filter(|row| row.kind == "team").count();
        let empty_state = if rows.is_empty() {
            Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No favorites".to_string(),
                detail: Some("Add players or teams to populate this group.".to_string()),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Self {
            context,
            group,
            rows,
            player_count,
            team_count,
            warnings: Vec::new(),
            empty_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FavoriteMemberInput {
    pub kind: String,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FavoriteMemberRow {
    pub kind: String,
    pub key: String,
    pub stat_line: Option<String>,
}
