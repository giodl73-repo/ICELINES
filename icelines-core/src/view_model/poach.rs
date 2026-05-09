use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::identity::PlayerId;
use crate::model::{Position, Season, TeamAbbr};
use crate::season_stats::SeasonType;
use crate::view_model::context::{
    AppliedFilter, EmptyState, ReportContext, ReportKind, ReportSectionRef, SortState, SourceKind,
    SourceState, ViewContext, ViewWarning, ViewWindow,
};
use crate::view_model::tokens::{MetricUnit, SemanticToken};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoachBoardView {
    pub context: ViewContext,
    pub query: PoachQuery,
    pub scoring_scheme: String,
    pub window: PoachWindow,
    pub applied_filters: Vec<AppliedFilter>,
    pub sort: Option<SortState>,
    pub rows: Vec<PoachPlayerRow>,
    pub source_state: Vec<SourceState>,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
    pub confidence_summary: ConfidenceSummary,
}

impl PoachBoardView {
    pub fn new(context: ViewContext, query: PoachQuery, scoring_scheme: impl Into<String>) -> Self {
        let window = query.window;
        Self {
            context,
            query,
            scoring_scheme: scoring_scheme.into(),
            window,
            applied_filters: Vec::new(),
            sort: None,
            rows: Vec::new(),
            source_state: Vec::new(),
            warnings: Vec::new(),
            empty_state: None,
            confidence_summary: ConfidenceSummary::default(),
        }
    }

    pub fn contract_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for row in &self.rows {
            errors.extend(row.contract_errors());
        }

        errors
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoachPlayerRow {
    pub player_id: PlayerId,
    pub display_name: String,
    pub team: TeamAbbr,
    pub position: Position,
    pub availability: AvailabilityState,
    pub recommendation_kinds: Vec<RecommendationKind>,
    pub score: PoachScore,
    pub confidence: PoachConfidence,
    pub components: Vec<PoachScoreComponent>,
    pub deployment: DeploymentSignal,
    pub schedule_summary: String,
    pub category_fit_summary: String,
    pub risk_summary: Option<String>,
    pub explanations: Vec<PoachExplanation>,
    pub tokens: Vec<SemanticToken>,
}

impl PoachPlayerRow {
    pub fn contract_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.explanations.is_empty() {
            errors.push(format!(
                "poach row {} has score without explanation",
                self.player_id.0
            ));
        }

        let recomputed = self.score.recompute_final();
        if (self.score.final_score - recomputed).abs() > f64::EPSILON {
            errors.push(format!(
                "poach row {} final score {} does not match recomputed score {}",
                self.player_id.0, self.score.final_score, recomputed
            ));
        }

        for kind in PoachComponentKind::ALL {
            if self.component(kind).is_none() {
                errors.push(format!(
                    "poach row {} missing component {:?}",
                    self.player_id.0, kind
                ));
            }
        }

        for component in &self.components {
            if component.value < component.range.start || component.value > component.range.end {
                errors.push(format!(
                    "poach row {} component {:?} value {} outside {}..{}",
                    self.player_id.0,
                    component.kind,
                    component.value,
                    component.range.start,
                    component.range.end
                ));
            }
        }

        if matches!(self.deployment, DeploymentSignal::Unknown)
            && self
                .component(PoachComponentKind::DeploymentTrend)
                .is_some_and(|component| component.value < 0.0)
        {
            errors.push(format!(
                "poach row {} penalizes unknown deployment",
                self.player_id.0
            ));
        }

        if matches!(self.availability, AvailabilityState::Unknown)
            && self
                .component(PoachComponentKind::AvailabilityGap)
                .is_some_and(|component| component.value < 0.0)
        {
            errors.push(format!(
                "poach row {} penalizes unknown availability",
                self.player_id.0
            ));
        }

        errors
    }

    pub fn component(&self, kind: PoachComponentKind) -> Option<&PoachScoreComponent> {
        self.components
            .iter()
            .find(|component| component.kind == kind)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoachScore {
    pub final_score: f64,
    pub opportunity_delta: f64,
    pub deployment_trend: f64,
    pub category_fit: f64,
    pub schedule_value: f64,
    pub availability_gap: f64,
    pub roster_need_fit: f64,
    pub risk_discount: f64,
}

impl PoachScore {
    pub fn recompute_final(&self) -> f64 {
        let positive = self.opportunity_delta
            + self.deployment_trend
            + self.category_fit
            + self.schedule_value
            + self.availability_gap
            + self.roster_need_fit;
        (positive.clamp(0.0, 100.0) - self.risk_discount).clamp(0.0, 100.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoachScoreComponent {
    pub kind: PoachComponentKind,
    pub label: String,
    pub value: f64,
    pub range: ScoreRange,
    pub status: ComponentStatus,
    pub unit: MetricUnit,
    pub source: Option<SourceKind>,
    pub token: Option<SemanticToken>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScoreRange {
    pub start: f64,
    pub end: f64,
}

impl ScoreRange {
    pub fn new(start: f64, end: f64) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoachComponentKind {
    OpportunityDelta,
    DeploymentTrend,
    CategoryFit,
    ScheduleValue,
    AvailabilityGap,
    RosterNeedFit,
    RiskDiscount,
}

impl PoachComponentKind {
    pub const ALL: [Self; 7] = [
        Self::OpportunityDelta,
        Self::DeploymentTrend,
        Self::CategoryFit,
        Self::ScheduleValue,
        Self::AvailabilityGap,
        Self::RosterNeedFit,
        Self::RiskDiscount,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStatus {
    Measured,
    Estimated,
    Deferred,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum DeploymentSignal {
    Actual {
        source: SourceKind,
        fetched_at: Option<DateTime<Utc>>,
        label: String,
    },
    Estimated {
        proxy: String,
        generated_at: Option<DateTime<Utc>>,
    },
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityState {
    Available,
    RosteredByUser,
    Watched,
    ImportedRostered,
    ImportedAvailable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationKind {
    Stream,
    Stash,
    CategoryFit,
    ScheduleEdge,
    DeploymentRiser,
    GoalieStream,
    Risk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoachConfidence {
    High,
    Medium,
    Low,
    DataLimited,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoachExplanation {
    pub component: PoachComponentKind,
    pub status: ComponentStatus,
    pub impact: ExplanationImpact,
    pub token: SemanticToken,
    pub message: String,
    pub source: Option<SourceKind>,
    pub freshness: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationImpact {
    Positive,
    Negative,
    Neutral,
    Omission,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoachQuery {
    pub season: Season,
    pub season_type: SeasonType,
    pub scoring_scheme: String,
    pub window: PoachWindow,
    pub categories: Vec<String>,
    pub positions: Vec<Position>,
    pub teams: Vec<TeamAbbr>,
    pub availability_filter: PoachAvailabilityFilter,
    pub candidate_kind: PoachCandidateKind,
    pub schedule_filter: Option<PoachScheduleFilter>,
    pub min_confidence: Option<PoachConfidence>,
    pub limit: Option<u16>,
    pub sort: Option<String>,
}

impl PoachQuery {
    pub fn new(season: Season, season_type: SeasonType, scoring_scheme: impl Into<String>) -> Self {
        Self {
            season,
            season_type,
            scoring_scheme: scoring_scheme.into(),
            window: PoachWindow::Days14,
            categories: Vec::new(),
            positions: Vec::new(),
            teams: Vec::new(),
            availability_filter: PoachAvailabilityFilter::Any,
            candidate_kind: PoachCandidateKind::All,
            schedule_filter: None,
            min_confidence: None,
            limit: None,
            sort: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoachWindow {
    Days7,
    Days14,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoachAvailabilityFilter {
    Any,
    Available,
    NotOnUserRoster,
    Watched,
    ImportedAvailable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoachCandidateKind {
    All,
    Streamer,
    Stash,
    CategorySpecialist,
    DeploymentRiser,
    GoalieStreamer,
    WatchAlert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoachScheduleFilter {
    AnyGames,
    OffNights,
    BackToBack,
    FantasyPlayoffWeek,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ConfidenceSummary {
    pub high: u16,
    pub medium: u16,
    pub low: u16,
    pub data_limited: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchRulesView {
    pub context: ViewContext,
    pub rules: Vec<WatchRule>,
    pub warnings: Vec<ViewWarning>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchRule {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub trigger: WatchRuleTrigger,
    pub last_fired: Option<DateTime<Utc>>,
    pub unsupported_sources: Vec<SourceKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WatchRuleTrigger {
    PlayerPromoted {
        player_id: Option<PlayerId>,
        evidence: DeploymentSignal,
    },
    CategoryThreshold {
        category: String,
        threshold: f64,
    },
    GoalieBackToBackStart {
        team: Option<TeamAbbr>,
    },
    AvailabilityChanged {
        player_id: Option<PlayerId>,
        state: AvailabilityState,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoachReportView {
    pub context: ReportContext,
    pub scoring_scheme: String,
    pub window: PoachWindow,
    pub source_state: Vec<SourceState>,
    pub warnings: Vec<ViewWarning>,
    pub omissions: Vec<String>,
    pub sections: Vec<PoachReportSection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoachReportSection {
    pub id: String,
    pub title: String,
    pub rows: Vec<PoachPlayerRow>,
}

pub fn poach_context(season: Season, season_type: SeasonType) -> ViewContext {
    let mut context = ViewContext::new(ViewWindow::new(season, season_type));
    context.source_state = vec![
        SourceState::complete(SourceKind::Roster),
        SourceState::complete(SourceKind::Schedule),
        SourceState::complete(SourceKind::FantasyImport),
    ];
    context
}

pub fn poach_report_context(context: ViewContext, report_id: impl Into<String>) -> ReportContext {
    ReportContext {
        kind: ReportKind::Poach,
        view_context: context,
        report_id: report_id.into(),
        title: "Fantasy Poacher".to_string(),
        sections: vec![
            ReportSectionRef {
                id: "top_adds".to_string(),
                title: "Top Adds".to_string(),
            },
            ReportSectionRef {
                id: "source_omissions".to_string(),
                title: "Source Omissions".to_string(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::{MetricUnit, SortDirection, SortKey};

    #[test]
    fn poach_contract_fixture_serializes_context_score_and_explanations() {
        let board = fixture_board();
        let json = serde_json::to_value(&board).expect("serialize poach board");
        let row = &json["rows"][0];

        assert_eq!(json["context"]["window"]["season"], 20252026);
        assert_eq!(json["context"]["window"]["season_type"], "regular");
        assert_eq!(json["query"]["window"], "days14");
        assert_eq!(json["scoring_scheme"], "yahoo-standard");
        assert_eq!(json["sort"]["key"], "poach_score");
        assert_eq!(json["sort"]["direction"], "desc");

        assert_eq!(row["player_id"], 8482109);
        assert_eq!(row["display_name"], "Hidden Category Fit");
        assert_eq!(row["availability"], "unknown");
        assert_eq!(row["confidence"], "data_limited");
        assert_eq!(row["score"]["final_score"], 62.0);
        assert_eq!(row["components"][0]["kind"], "opportunity_delta");
        assert_eq!(row["components"][0]["status"], "estimated");
        assert_eq!(row["components"][2]["kind"], "category_fit");
        assert_eq!(row["components"][2]["status"], "measured");
        assert_eq!(row["deployment"]["kind"], "unknown");
        assert_eq!(row["explanations"][0]["component"], "category_fit");
        assert_eq!(row["explanations"][0]["impact"], "positive");
        assert_eq!(row["tokens"][0], "category_fit");
    }

    #[test]
    fn poach_fixture_satisfies_contract_invariants() {
        let board = fixture_board();

        assert_eq!(board.contract_errors(), Vec::<String>::new());
        for row in &board.rows {
            assert!(!row.explanations.is_empty());
            assert_eq!(row.score.final_score, row.score.recompute_final());
        }
    }

    #[test]
    fn unknown_deployment_and_availability_are_not_negative_evidence() {
        let row = fixture_unknown_source_row();

        assert!(matches!(row.deployment, DeploymentSignal::Unknown));
        assert_eq!(row.availability, AvailabilityState::Unknown);
        assert_eq!(
            row.component(PoachComponentKind::DeploymentTrend)
                .expect("deployment component")
                .value,
            0.0
        );
        assert_eq!(
            row.component(PoachComponentKind::AvailabilityGap)
                .expect("availability component")
                .value,
            0.0
        );
        assert_eq!(row.contract_errors(), Vec::<String>::new());
    }

    #[test]
    fn poach_report_uses_report_context_and_sections() {
        let board = fixture_board();
        let report = PoachReportView {
            context: poach_report_context(board.context.clone(), "poach-weekly-fixture"),
            scoring_scheme: board.scoring_scheme.clone(),
            window: board.window,
            source_state: board.source_state.clone(),
            warnings: Vec::new(),
            omissions: vec!["ownership import unavailable".to_string()],
            sections: vec![PoachReportSection {
                id: "top_adds".to_string(),
                title: "Top Adds".to_string(),
                rows: board.rows.clone(),
            }],
        };

        let json = serde_json::to_value(&report).expect("serialize poach report");

        assert_eq!(json["context"]["kind"], "poach");
        assert_eq!(json["context"]["report_id"], "poach-weekly-fixture");
        assert_eq!(json["sections"][0]["id"], "top_adds");
        assert_eq!(json["sections"][0]["rows"][0]["player_id"], 8482109);
        assert_eq!(json["omissions"][0], "ownership import unavailable");
    }

    fn fixture_board() -> PoachBoardView {
        let season = Season(20252026);
        let season_type = SeasonType::Regular;
        let mut query = PoachQuery::new(season, season_type, "yahoo-standard");
        query.categories = vec!["hits".to_string(), "blocks".to_string()];
        query.availability_filter = PoachAvailabilityFilter::Available;
        query.candidate_kind = PoachCandidateKind::CategorySpecialist;
        query.limit = Some(25);
        query.sort = Some("poach_score".to_string());

        let mut board =
            PoachBoardView::new(poach_context(season, season_type), query, "yahoo-standard");
        board.sort = Some(SortState {
            key: SortKey::from("poach_score"),
            label: "Poach Score".to_string(),
            direction: SortDirection::Desc,
        });
        board.source_state = board.context.source_state.clone();
        board.rows = vec![fixture_unknown_source_row()];
        board.confidence_summary.data_limited = 1;
        board
    }

    fn fixture_unknown_source_row() -> PoachPlayerRow {
        PoachPlayerRow {
            player_id: PlayerId(8482109),
            display_name: "Hidden Category Fit".to_string(),
            team: TeamAbbr("SEA".to_string()),
            position: Position::LeftWing,
            availability: AvailabilityState::Unknown,
            recommendation_kinds: vec![RecommendationKind::CategoryFit],
            score: PoachScore {
                final_score: 62.0,
                opportunity_delta: 12.0,
                deployment_trend: 0.0,
                category_fit: 25.0,
                schedule_value: 15.0,
                availability_gap: 0.0,
                roster_need_fit: 10.0,
                risk_discount: 0.0,
            },
            confidence: PoachConfidence::DataLimited,
            components: vec![
                component(
                    PoachComponentKind::OpportunityDelta,
                    "Opportunity",
                    12.0,
                    0.0,
                    20.0,
                    ComponentStatus::Estimated,
                    Some(SourceKind::Roster),
                    Some(SemanticToken::Rising),
                ),
                component(
                    PoachComponentKind::DeploymentTrend,
                    "Deployment",
                    0.0,
                    0.0,
                    15.0,
                    ComponentStatus::Unavailable,
                    None,
                    Some(SemanticToken::SourceUnavailable),
                ),
                component(
                    PoachComponentKind::CategoryFit,
                    "Category Fit",
                    25.0,
                    0.0,
                    25.0,
                    ComponentStatus::Measured,
                    Some(SourceKind::Roster),
                    Some(SemanticToken::CategoryFit),
                ),
                component(
                    PoachComponentKind::ScheduleValue,
                    "Schedule",
                    15.0,
                    0.0,
                    15.0,
                    ComponentStatus::Measured,
                    Some(SourceKind::Schedule),
                    Some(SemanticToken::ScheduleEdge),
                ),
                component(
                    PoachComponentKind::AvailabilityGap,
                    "Availability",
                    0.0,
                    0.0,
                    10.0,
                    ComponentStatus::Unavailable,
                    None,
                    Some(SemanticToken::SourceUnavailable),
                ),
                component(
                    PoachComponentKind::RosterNeedFit,
                    "Roster Need",
                    10.0,
                    0.0,
                    15.0,
                    ComponentStatus::Estimated,
                    Some(SourceKind::FantasyImport),
                    Some(SemanticToken::SupportingEvidence),
                ),
                component(
                    PoachComponentKind::RiskDiscount,
                    "Risk",
                    0.0,
                    0.0,
                    30.0,
                    ComponentStatus::Estimated,
                    Some(SourceKind::Transactions),
                    Some(SemanticToken::Risk),
                ),
            ],
            deployment: DeploymentSignal::Unknown,
            schedule_summary: "4 games in next 14 days".to_string(),
            category_fit_summary: "Hits and blocks fit active scheme".to_string(),
            risk_summary: None,
            explanations: vec![
                PoachExplanation {
                    component: PoachComponentKind::CategoryFit,
                    status: ComponentStatus::Measured,
                    impact: ExplanationImpact::Positive,
                    token: SemanticToken::CategoryFit,
                    message: "Strong hits/blocks value under yahoo-standard".to_string(),
                    source: Some(SourceKind::Roster),
                    freshness: None,
                },
                PoachExplanation {
                    component: PoachComponentKind::DeploymentTrend,
                    status: ComponentStatus::Unavailable,
                    impact: ExplanationImpact::Omission,
                    token: SemanticToken::SourceUnavailable,
                    message: "Line and PP data unavailable; no deployment penalty applied"
                        .to_string(),
                    source: None,
                    freshness: None,
                },
            ],
            tokens: vec![SemanticToken::CategoryFit, SemanticToken::ScheduleEdge],
        }
    }

    fn component(
        kind: PoachComponentKind,
        label: &str,
        value: f64,
        start: f64,
        end: f64,
        status: ComponentStatus,
        source: Option<SourceKind>,
        token: Option<SemanticToken>,
    ) -> PoachScoreComponent {
        PoachScoreComponent {
            kind,
            label: label.to_string(),
            value,
            range: ScoreRange::new(start, end),
            status,
            unit: MetricUnit::Score,
            source,
            token,
        }
    }
}
