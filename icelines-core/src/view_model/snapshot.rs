use serde::{Deserialize, Serialize};

use crate::view_model::context::{
    Completeness, EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotView {
    pub context: ViewContext,
    pub active: Option<String>,
    pub rows: Vec<SnapshotRow>,
    pub selected: Option<SnapshotRow>,
    pub total: usize,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl SnapshotView {
    pub fn from_entries(
        mut context: ViewContext,
        active: Option<String>,
        entries: Vec<SnapshotEntryInput>,
        selected_name: Option<&str>,
    ) -> Self {
        context
            .source_state
            .push(SourceState::complete(SourceKind::Snapshot));

        let rows: Vec<SnapshotRow> = entries
            .into_iter()
            .map(|entry| snapshot_row(entry, active.as_deref()))
            .collect();
        let selected = selected_name
            .and_then(|name| rows.iter().find(|row| row.name == name))
            .cloned();
        let total = rows.len();
        if total == 0 {
            context.completeness = Completeness::Unavailable;
        }

        let empty_state = if total == 0 {
            Some(EmptyState {
                kind: EmptyKind::MissingSource,
                title: "No snapshots".to_string(),
                detail: Some("Run `icelines fetch all` to create a snapshot.".to_string()),
                recovery: Vec::new(),
            })
        } else if selected_name.is_some() && selected.is_none() {
            Some(EmptyState {
                kind: EmptyKind::NotFound,
                title: "Snapshot not found".to_string(),
                detail: selected_name.map(|name| format!("Snapshot '{name}' was not found.")),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Self {
            context,
            active,
            rows,
            selected,
            total,
            warnings: Vec::new(),
            empty_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotEntryInput {
    pub name: String,
    pub season: String,
    pub tier: String,
    pub date: String,
    pub created_at: String,
    pub parent_key: Option<String>,
    pub file_count: usize,
    pub sealed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRow {
    pub name: String,
    pub season: String,
    pub tier: String,
    pub date: String,
    pub created_at: String,
    pub parent_key: Option<String>,
    pub file_count: usize,
    pub sealed: bool,
    pub sealed_label: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotMutationOperation {
    Create,
    Activate,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotMutationIntent {
    pub operation: SnapshotMutationOperation,
    pub name: String,
}

impl SnapshotMutationIntent {
    pub fn resolve(
        operation: SnapshotMutationOperation,
        name: impl Into<String>,
    ) -> Result<Self, String> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err("snapshot name is required".to_string());
        }
        Ok(Self { operation, name })
    }

    pub fn result_view(&self, context: ViewContext, changed: bool) -> crate::MutationResultView {
        let operation = match self.operation {
            SnapshotMutationOperation::Create => "snapshot_create",
            SnapshotMutationOperation::Activate => "snapshot_activate",
            SnapshotMutationOperation::Remove => "snapshot_remove",
        };
        let message = if changed {
            format!("{operation} {}", self.name)
        } else {
            format!("No snapshot change needed for {}", self.name)
        };
        if changed {
            crate::MutationResultView::applied(context, operation, self.name.clone(), message, None)
        } else {
            crate::MutationResultView::noop(context, operation, self.name.clone(), message, None)
        }
    }
}

fn snapshot_row(input: SnapshotEntryInput, active: Option<&str>) -> SnapshotRow {
    let is_active = active == Some(input.name.as_str());
    SnapshotRow {
        sealed_label: if input.sealed { "yes" } else { "draft" }.to_string(),
        is_active,
        name: input.name,
        season: input.season,
        tier: input.tier,
        date: input.date,
        created_at: input.created_at,
        parent_key: input.parent_key,
        file_count: input.file_count,
        sealed: input.sealed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Season;
    use crate::season_stats::SeasonType;
    use crate::view_model::{ViewContext, ViewWindow};

    #[test]
    fn snapshot_view_marks_active_and_selected_rows() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = SnapshotView::from_entries(
            context,
            Some("stats-2026-05-10".to_string()),
            vec![SnapshotEntryInput {
                name: "stats-2026-05-10".to_string(),
                season: "20252026".to_string(),
                tier: "Stats".to_string(),
                date: "2026-05-10".to_string(),
                created_at: "2026-05-10T12:00:00Z".to_string(),
                parent_key: Some("rosters-2026-05-10".to_string()),
                file_count: 4,
                sealed: true,
            }],
            Some("stats-2026-05-10"),
        );

        assert_eq!(view.total, 1);
        assert!(view.rows[0].is_active);
        assert_eq!(view.rows[0].sealed_label, "yes");
        assert_eq!(
            view.selected.as_ref().map(|row| row.name.as_str()),
            Some("stats-2026-05-10")
        );
        assert_eq!(view.context.source_state[0].source, SourceKind::Snapshot);
    }

    #[test]
    fn snapshot_view_empty_manifest_is_unavailable() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = SnapshotView::from_entries(context, None, Vec::new(), None);

        assert_eq!(view.context.completeness, Completeness::Unavailable);
        assert_eq!(
            view.empty_state.as_ref().map(|state| state.kind),
            Some(EmptyKind::MissingSource)
        );
    }

    #[test]
    fn snapshot_mutation_intent_projects_result_view() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let intent =
            SnapshotMutationIntent::resolve(SnapshotMutationOperation::Activate, "stats-latest")
                .expect("valid snapshot mutation");
        let view = intent.result_view(context, true);

        assert_eq!(view.operation, "snapshot_activate");
        assert_eq!(view.target, "stats-latest");
        assert_eq!(view.status, crate::MutationStatus::Applied);
    }
}
