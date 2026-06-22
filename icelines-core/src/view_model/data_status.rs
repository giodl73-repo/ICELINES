use serde::{Deserialize, Serialize};

use crate::freshness::{FetchSource, Freshness, Ttl};
use crate::view_model::context::{
    Completeness, EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataStatusView {
    pub context: ViewContext,
    pub root: String,
    pub active_shard: Option<String>,
    pub stale_only: bool,
    pub authority_notes: Vec<DataAuthorityNote>,
    pub rows: Vec<DataStatusRow>,
    pub total: usize,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl DataStatusView {
    pub fn from_entries(
        mut context: ViewContext,
        root: impl Into<String>,
        active_shard: Option<String>,
        stale_only: bool,
        entries: Vec<DataStatusEntryInput>,
    ) -> Self {
        context
            .source_state
            .push(SourceState::complete(SourceKind::Cache));

        let rows: Vec<DataStatusRow> = entries.into_iter().map(data_status_row).collect();
        let total = rows.len();
        let empty_state = if total == 0 {
            Some(EmptyState {
                kind: if active_shard.is_some() || stale_only {
                    EmptyKind::NoMatch
                } else {
                    EmptyKind::MissingSource
                },
                title: if active_shard.is_some() {
                    "No manifest entries".to_string()
                } else if stale_only {
                    "No stale manifest entries".to_string()
                } else {
                    "Manifest is empty".to_string()
                },
                detail: Some(if stale_only {
                    "No entries matched the stale-only filter.".to_string()
                } else {
                    "Run `icelines setup --accept-defaults` then `icelines fetch sync` to populate data."
                        .to_string()
                }),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        if total == 0 {
            context.completeness = Completeness::Unavailable;
        }

        Self {
            context,
            root: root.into(),
            active_shard,
            stale_only,
            authority_notes: default_authority_notes(),
            rows,
            total,
            warnings: Vec::new(),
            empty_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataAuthorityNote {
    pub key: String,
    pub source: String,
    pub coverage_state: String,
    pub covered_metrics: Vec<String>,
    pub blocked_metrics: Vec<String>,
    pub limitations: Vec<String>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataStatusEntryInput {
    pub source: FetchSource,
    pub kind: String,
    pub key: String,
    pub freshness: Freshness,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataStatusRow {
    pub source: String,
    pub kind: String,
    pub key: String,
    pub freshness: String,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataMutationOperation {
    Install,
    Remove,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataMutationIntent {
    pub operation: DataMutationOperation,
    pub target: String,
    pub force: bool,
}

impl DataMutationIntent {
    pub fn resolve(
        operation: DataMutationOperation,
        target: impl Into<String>,
        force: bool,
    ) -> Result<Self, String> {
        let target = target.into();
        if target.trim().is_empty() {
            return Err("data mutation target is required".to_string());
        }
        Ok(Self {
            operation,
            target,
            force,
        })
    }

    pub fn result_view(&self, context: ViewContext, changed: bool) -> crate::MutationResultView {
        let operation = match self.operation {
            DataMutationOperation::Install => "data_install",
            DataMutationOperation::Remove => "data_remove",
            DataMutationOperation::Verify => "data_verify",
        };
        let message = if changed {
            format!("{operation} {}", self.target)
        } else {
            format!("No data change needed for {}", self.target)
        };
        if changed {
            crate::MutationResultView::applied(
                context,
                operation,
                self.target.clone(),
                message,
                None,
            )
        } else {
            crate::MutationResultView::noop(context, operation, self.target.clone(), message, None)
        }
    }
}

fn data_status_row(input: DataStatusEntryInput) -> DataStatusRow {
    DataStatusRow {
        source: source_label(input.source).to_string(),
        kind: input.kind,
        key: input.key,
        freshness: freshness_label(&input.freshness),
        stale: input.freshness.is_stale(&crate::freshness::SystemClock),
    }
}

pub fn source_label(source: FetchSource) -> &'static str {
    match source {
        FetchSource::Bundle => "Bundle",
        FetchSource::Setup => "Setup",
        FetchSource::Live => "Live",
        FetchSource::DataInstall => "DataInstall",
        FetchSource::Manual => "Manual",
    }
}

pub fn freshness_label(freshness: &Freshness) -> String {
    match freshness.ttl {
        Ttl::Static => "static".to_string(),
        Ttl::After(duration) => {
            let secs = duration.as_secs();
            if secs < 3600 {
                format!("ttl {}m", secs / 60)
            } else if secs < 86400 {
                format!("ttl {}h", secs / 3600)
            } else {
                format!("ttl {}d", secs / 86400)
            }
        }
    }
}

fn default_authority_notes() -> Vec<DataAuthorityNote> {
    vec![DataAuthorityNote {
        key: "moneypuck_skater_snapshot".to_string(),
        source: "MoneyPuck skater snapshot".to_string(),
        coverage_state: "optional_snapshot".to_string(),
        covered_metrics: vec![
            "individual_expected_goals".to_string(),
            "individual_expected_goals_per_60".to_string(),
            "on_ice_expected_goals_for".to_string(),
            "on_ice_expected_goals_against".to_string(),
            "expected_goals_for_pct".to_string(),
            "corsi_for_pct".to_string(),
            "fenwick_for_pct".to_string(),
        ],
        blocked_metrics: vec![
            "goalie_xga".to_string(),
            "goalie_gsax".to_string(),
            "goalie_high_danger_save_pct".to_string(),
            "skater_high_danger_chance_pct".to_string(),
            "zone_entries".to_string(),
            "deployment_recommendations".to_string(),
        ],
        limitations: vec![
            "optional_snapshot_not_live_fetch_status".to_string(),
            "missing_snapshot_values_are_absent_not_zero".to_string(),
            "xg_is_not_prediction_or_betting_advice".to_string(),
            "does_not_cover_high_danger_or_zone_entries".to_string(),
        ],
        label: "MoneyPuck skater xG is an optional snapshot source; missing values stay absent, and goalie/high-danger/zone-entry/deployment claims remain blocked."
            .to_string(),
    }]
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::Utc;

    use super::*;
    use crate::model::Season;
    use crate::season_stats::SeasonType;
    use crate::view_model::{ViewContext, ViewWindow};

    #[test]
    fn data_status_view_projects_entries_and_empty_state() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = DataStatusView::from_entries(
            context,
            "/tmp/icelines",
            Some("stats".to_string()),
            false,
            vec![DataStatusEntryInput {
                source: FetchSource::Bundle,
                kind: "Stats".to_string(),
                key: "20252026/regular".to_string(),
                freshness: Freshness {
                    fetched_at: Utc::now(),
                    source: FetchSource::Bundle,
                    ttl: Ttl::Static,
                },
            }],
        );

        assert_eq!(view.root, "/tmp/icelines");
        assert_eq!(view.total, 1);
        assert_eq!(view.rows[0].source, "Bundle");
        assert_eq!(view.rows[0].freshness, "static");
        assert_eq!(view.context.source_state[0].source, SourceKind::Cache);
        assert_eq!(view.authority_notes[0].key, "moneypuck_skater_snapshot");
        assert!(view.empty_state.is_none());
    }

    #[test]
    fn data_status_view_exposes_moneypuck_snapshot_authority_note() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = DataStatusView::from_entries(context, "/tmp/icelines", None, false, Vec::new());
        let note = view
            .authority_notes
            .iter()
            .find(|note| note.key == "moneypuck_skater_snapshot")
            .expect("MoneyPuck authority note");

        assert_eq!(note.coverage_state, "optional_snapshot");
        assert!(note
            .covered_metrics
            .contains(&"on_ice_expected_goals_against".to_string()));
        assert!(note
            .blocked_metrics
            .contains(&"goalie_high_danger_save_pct".to_string()));
        assert!(note
            .limitations
            .contains(&"missing_snapshot_values_are_absent_not_zero".to_string()));
    }

    #[test]
    fn data_status_view_empty_manifest_is_unavailable() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = DataStatusView::from_entries(context, "/tmp/icelines", None, false, Vec::new());

        assert_eq!(view.context.completeness, Completeness::Unavailable);
        assert_eq!(
            view.empty_state.as_ref().map(|state| state.kind),
            Some(EmptyKind::MissingSource)
        );
    }

    #[test]
    fn freshness_label_buckets_by_unit() {
        let mk = |ttl| Freshness {
            fetched_at: Utc::now(),
            source: FetchSource::Live,
            ttl,
        };

        assert_eq!(freshness_label(&mk(Ttl::Static)), "static");
        assert_eq!(
            freshness_label(&mk(Ttl::After(Duration::from_secs(60)))),
            "ttl 1m"
        );
        assert_eq!(
            freshness_label(&mk(Ttl::After(Duration::from_secs(3600)))),
            "ttl 1h"
        );
        assert_eq!(
            freshness_label(&mk(Ttl::After(Duration::from_secs(86400)))),
            "ttl 1d"
        );
    }

    #[test]
    fn data_mutation_intent_projects_result_view() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let intent = DataMutationIntent::resolve(DataMutationOperation::Install, "20252026", false)
            .expect("valid data mutation");
        let view = intent.result_view(context, true);

        assert_eq!(view.operation, "data_install");
        assert_eq!(view.target, "20252026");
        assert_eq!(view.status, crate::MutationStatus::Applied);
    }
}
