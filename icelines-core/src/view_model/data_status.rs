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
            rows,
            total,
            warnings: Vec::new(),
            empty_state,
        }
    }
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
        assert!(view.empty_state.is_none());
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
}
