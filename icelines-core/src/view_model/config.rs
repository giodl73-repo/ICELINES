use serde::{Deserialize, Serialize};

use crate::season_stats::SeasonType;
use crate::view_model::context::{
    EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigView {
    pub context: ViewContext,
    pub rows: Vec<ConfigEntryRow>,
    pub selected_key: Option<String>,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl ConfigView {
    pub fn from_entries(
        mut context: ViewContext,
        entries: Vec<ConfigEntryInput>,
        selected_key: Option<String>,
    ) -> Self {
        context
            .source_state
            .push(SourceState::complete(SourceKind::Cache));
        let rows: Vec<ConfigEntryRow> = entries
            .into_iter()
            .map(|entry| ConfigEntryRow {
                selected: selected_key.as_deref() == Some(entry.key.as_str()),
                key: entry.key,
                value: entry.value,
            })
            .collect();
        let empty_state = if rows.is_empty() {
            Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No config entries".to_string(),
                detail: Some("No configuration keys are available.".to_string()),
                recovery: Vec::new(),
            })
        } else if selected_key.is_some() && !rows.iter().any(|row| row.selected) {
            Some(EmptyState {
                kind: EmptyKind::NotFound,
                title: "Config key not found".to_string(),
                detail: selected_key
                    .as_ref()
                    .map(|key| format!("Config key '{key}' was not found.")),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Self {
            context,
            rows,
            selected_key,
            warnings: Vec::new(),
            empty_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigEntryInput {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigEntryRow {
    pub key: String,
    pub value: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeasonTypeMutationIntent {
    pub active_season_type: String,
    pub redirect_to: String,
}

impl SeasonTypeMutationIntent {
    pub fn resolve(kind: &str, referer: Option<&str>) -> Self {
        let active_season_type = SeasonType::parse_lossy(kind).label().to_string();
        let redirect_to = safe_redirect_from_referer(referer).unwrap_or_else(|| "/".to_string());

        Self {
            active_season_type,
            redirect_to,
        }
    }

    pub fn result_view(&self, context: ViewContext) -> crate::MutationResultView {
        crate::MutationResultView::applied(
            context,
            "set_season_type",
            self.active_season_type.clone(),
            format!("Active season type set to {}", self.active_season_type),
            Some(self.redirect_to.clone()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigMutationIntent {
    pub key: String,
    pub value: Option<String>,
    pub reset: bool,
}

impl ConfigMutationIntent {
    pub fn set(key: &str, value: &str) -> Result<Self, String> {
        let key = key.trim();
        if key.is_empty() {
            return Err("config key is required".to_string());
        }
        Ok(Self {
            key: key.to_string(),
            value: Some(value.to_string()),
            reset: false,
        })
    }

    pub fn reset(key: &str) -> Result<Self, String> {
        let key = key.trim();
        if key.is_empty() {
            return Err("config key is required".to_string());
        }
        Ok(Self {
            key: key.to_string(),
            value: None,
            reset: true,
        })
    }

    pub fn result_view(&self, context: ViewContext, changed: bool) -> crate::MutationResultView {
        let operation = if self.reset {
            "config_reset"
        } else {
            "config_set"
        };
        let message = if changed {
            format!("{operation} {}", self.key)
        } else {
            format!("No config change needed for {}", self.key)
        };
        if changed {
            crate::MutationResultView::applied(context, operation, self.key.clone(), message, None)
        } else {
            crate::MutationResultView::noop(context, operation, self.key.clone(), message, None)
        }
    }
}

fn safe_redirect_from_referer(referer: Option<&str>) -> Option<String> {
    let referer = referer?;
    if is_safe_relative_path(referer) {
        return Some(referer.to_string());
    }

    let (_, after_scheme) = referer.split_once("://")?;
    let (host, path) = match after_scheme.split_once('/') {
        Some((host, path)) => (host, format!("/{path}")),
        None => (after_scheme, "/".to_string()),
    };
    if is_local_host(host) && is_safe_relative_path(&path) {
        Some(path)
    } else {
        None
    }
}

fn is_local_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let host_name = if let Some(value) = host.strip_prefix('[') {
        value.split_once(']').map(|(addr, _)| addr).unwrap_or("")
    } else {
        host.split(':').next().unwrap_or("")
    };
    matches!(host_name, "127.0.0.1" | "localhost" | "::1")
}

fn is_safe_relative_path(path: &str) -> bool {
    path.starts_with('/') && !path.starts_with("//")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Season;
    use crate::season_stats::SeasonType;
    use crate::view_model::{ViewContext, ViewWindow};

    #[test]
    fn config_view_marks_selected_key() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = ConfigView::from_entries(
            context,
            vec![
                ConfigEntryInput {
                    key: "season".to_string(),
                    value: "20252026".to_string(),
                },
                ConfigEntryInput {
                    key: "theme".to_string(),
                    value: "ascii".to_string(),
                },
            ],
            Some("theme".to_string()),
        );

        assert_eq!(view.rows.len(), 2);
        assert!(view.rows[1].selected);
        assert_eq!(view.context.source_state[0].source, SourceKind::Cache);
        assert!(view.empty_state.is_none());
    }

    #[test]
    fn config_mutation_intent_projects_result_view() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let intent = ConfigMutationIntent::set("theme", "ascii").expect("valid config mutation");
        let view = intent.result_view(context, true);

        assert_eq!(view.operation, "config_set");
        assert_eq!(view.target, "theme");
        assert_eq!(view.status, crate::MutationStatus::Applied);
    }
}
