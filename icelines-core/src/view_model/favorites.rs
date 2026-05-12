use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::model::TeamAbbr;
use crate::name::normalize_name;
use crate::view_model::context::{
    EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
};
use crate::view_model::mutation::MutationResultView;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavoriteMutationIntent {
    pub kind: String,
    pub key: String,
    pub entity_ref: String,
    pub redirect_to: String,
}

impl FavoriteMutationIntent {
    pub fn resolve(
        key: &str,
        kind_hint: Option<&str>,
        return_to: Option<&str>,
        referer_path: Option<&str>,
    ) -> Result<Self, String> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err("Empty key - pass a player name or team abbrev.".to_string());
        }

        let (kind, key) = match kind_hint {
            Some("team") => ("team".to_string(), trimmed.to_uppercase()),
            Some("player") => ("player".to_string(), normalize_name(trimmed)),
            _ => match TeamAbbr::parse(trimmed) {
                Ok(abbr) => ("team".to_string(), abbr.0),
                Err(_) => ("player".to_string(), normalize_name(trimmed)),
            },
        };
        let entity_ref = format!("{kind}:{key}");
        let redirect_to = return_to
            .or(referer_path)
            .filter(|path| path.starts_with('/') && !path.starts_with("//"))
            .unwrap_or("/favorites")
            .to_string();

        Ok(Self {
            kind,
            key,
            entity_ref,
            redirect_to,
        })
    }

    pub fn result_view(
        &self,
        context: ViewContext,
        operation: impl Into<String>,
        applied: bool,
    ) -> MutationResultView {
        let operation = operation.into();
        let message = if applied {
            format!("{operation} favorite {}", self.entity_ref)
        } else {
            format!("No favorite change needed for {}", self.entity_ref)
        };
        if applied {
            MutationResultView::applied(
                context,
                operation,
                self.entity_ref.clone(),
                message,
                Some(self.redirect_to.clone()),
            )
        } else {
            MutationResultView::noop(
                context,
                operation,
                self.entity_ref.clone(),
                message,
                Some(self.redirect_to.clone()),
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchlistView {
    pub context: ViewContext,
    pub group: String,
    pub rows: Vec<WatchlistMemberRow>,
    pub player_count: usize,
    pub team_count: usize,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl WatchlistView {
    pub fn from_members(
        mut context: ViewContext,
        group: String,
        members: Vec<FavoriteMemberInput>,
        notes: HashMap<String, WatchNoteInput>,
    ) -> Self {
        context
            .source_state
            .push(SourceState::complete(SourceKind::Watchlist));

        let rows: Vec<WatchlistMemberRow> = members
            .into_iter()
            .map(|member| {
                let note = notes.get(&format!("{}:{}", member.kind, member.key));
                WatchlistMemberRow {
                    kind: member.kind,
                    key: member.key,
                    reason: note.map(|note| note.reason.clone()),
                    source: note.map(|note| note.source.clone()),
                    updated_at: note.map(|note| note.updated_at.clone()),
                }
            })
            .collect();
        let player_count = rows.iter().filter(|row| row.kind == "player").count();
        let team_count = rows.iter().filter(|row| row.kind == "team").count();
        let empty_state = if rows.is_empty() {
            Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No watchlist entries".to_string(),
                detail: Some("Add players or teams to populate this watchlist.".to_string()),
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
pub struct WatchNoteInput {
    pub reason: String,
    pub source: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchlistMemberRow {
    pub kind: String,
    pub key: String,
    pub reason: Option<String>,
    pub source: Option<String>,
    pub updated_at: Option<String>,
}
