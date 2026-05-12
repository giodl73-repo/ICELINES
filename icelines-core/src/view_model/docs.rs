use serde::{Deserialize, Serialize};

use crate::view_model::context::{EmptyState, SourceKind, SourceState, ViewContext, ViewWarning};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocsView {
    pub context: ViewContext,
    pub source_path: String,
    pub title: String,
    pub markdown: String,
    pub rendered_html: String,
    pub markdown_bytes: usize,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl DocsView {
    pub fn rendered(
        mut context: ViewContext,
        source_path: impl Into<String>,
        title: impl Into<String>,
        markdown: &str,
        rendered_html: impl Into<String>,
    ) -> Self {
        context
            .source_state
            .push(SourceState::complete(SourceKind::Docs));

        Self {
            context,
            source_path: source_path.into(),
            title: title.into(),
            markdown: markdown.to_string(),
            rendered_html: rendered_html.into(),
            markdown_bytes: markdown.len(),
            warnings: Vec::new(),
            empty_state: None,
        }
    }
}
