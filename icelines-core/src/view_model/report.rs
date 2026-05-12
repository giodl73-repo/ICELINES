use serde::{Deserialize, Serialize};

use crate::view_model::context::{
    ReportContext, ReportKind, ReportSectionRef, SourceKind, SourceState, ViewContext, ViewWarning,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    Terminal,
    Markdown,
    Json,
    Csv,
}

impl ReportFormat {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Markdown => "markdown",
            Self::Json => "json",
            Self::Csv => "csv",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportView {
    pub context: ReportContext,
    pub format: ReportFormat,
    pub rendered_body: String,
    pub source_state: Vec<SourceState>,
    pub warnings: Vec<ViewWarning>,
}

impl ReportView {
    pub fn rendered(
        view_context: ViewContext,
        kind: ReportKind,
        report_id: impl Into<String>,
        title: impl Into<String>,
        format: ReportFormat,
        sections: Vec<ReportSectionRef>,
        rendered_body: impl Into<String>,
    ) -> Self {
        let source_state = vec![SourceState::complete(SourceKind::Roster)];
        Self {
            context: ReportContext {
                kind,
                view_context,
                report_id: report_id.into(),
                title: title.into(),
                sections,
            },
            format,
            rendered_body: rendered_body.into(),
            source_state,
            warnings: Vec::new(),
        }
    }
}

pub fn scouting_report_sections() -> Vec<ReportSectionRef> {
    [
        ("bio", "Bio"),
        ("current-season", "Current Season"),
        ("career-trajectory", "Career Trajectory"),
        ("peer-group-rank", "Peer Group Rank"),
        ("linemates", "Linemates"),
        ("depth-chart-position", "Depth Chart Position"),
        ("cross-team-value", "Cross-Team Value"),
        ("fit-interpretation", "Fit Interpretation"),
    ]
    .into_iter()
    .map(|(id, title)| ReportSectionRef {
        id: id.to_string(),
        title: title.to_string(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Season;
    use crate::season_stats::SeasonType;
    use crate::view_model::{ViewContext, ViewWindow};

    #[test]
    fn report_view_wraps_rendered_body_and_sections() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = ReportView::rendered(
            context,
            ReportKind::Scouting,
            "scouting-8478402",
            "Scouting Report",
            ReportFormat::Markdown,
            scouting_report_sections(),
            "# Scouting Report\n",
        );

        assert_eq!(view.context.kind, ReportKind::Scouting);
        assert_eq!(view.format.label(), "markdown");
        assert_eq!(view.context.sections.len(), 8);
        assert_eq!(view.source_state[0].source, SourceKind::Roster);
        assert!(view.rendered_body.contains("Scouting Report"));
    }
}
