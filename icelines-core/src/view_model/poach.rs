use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::identity::PlayerId;
use crate::model::{Position, Season, TeamAbbr};
use crate::season_stats::SeasonType;
use crate::stats_repository::{PlayerView, StatsRepository};
use crate::view_model::context::{
    AppliedFilter, Completeness, EmptyKind, EmptyState, ReportContext, ReportKind,
    ReportSectionRef, SortDirection, SortKey, SortState, SourceKind, SourceState, ViewContext,
    ViewWarning, ViewWindow,
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

    pub fn from_repository(repo: &StatsRepository, query: PoachQuery) -> Self {
        let has_window = repo.has_window(query.season, query.season_type);
        let context = poach_context_for_sources(query.season, query.season_type, has_window);
        let mut view = Self::new(context, query.clone(), query.scoring_scheme.clone());
        view.sort = Some(SortState {
            key: SortKey::from("poach_score"),
            label: "Poach Score".to_string(),
            direction: SortDirection::Desc,
        });
        view.source_state = view.context.source_state.clone();

        if !has_window {
            view.empty_state = Some(EmptyState {
                kind: EmptyKind::MissingSource,
                title: "Missing poacher source data".to_string(),
                detail: Some("The requested season/type window is not loaded.".to_string()),
                recovery: Vec::new(),
            });
            return view;
        }

        let mut rows: Vec<PoachPlayerRow> = repo
            .skaters(query.season, query.season_type)
            .filter(|player| query.matches_player(player))
            .map(|player| row_from_player(&player, &query))
            .collect();

        rows.sort_by(|a, b| {
            b.score
                .final_score
                .total_cmp(&a.score.final_score)
                .then_with(|| a.player_id.0.cmp(&b.player_id.0))
        });

        if let Some(limit) = query.limit {
            rows.truncate(limit as usize);
        }

        view.confidence_summary = ConfidenceSummary::from_rows(&rows);
        view.rows = rows;

        if view.rows.is_empty() {
            view.empty_state = Some(EmptyState {
                kind: EmptyKind::NoMatch,
                title: "No poach candidates".to_string(),
                detail: Some("No skaters matched the poacher filters.".to_string()),
                recovery: Vec::new(),
            });
        }

        view
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
        if (self.score.final_score - recomputed).abs() > 0.000_001 {
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

    fn matches_player(&self, player: &PlayerView<'_>) -> bool {
        if !self.positions.is_empty() && !self.positions.contains(&player.position()) {
            return false;
        }

        if !self.teams.is_empty()
            && !player
                .team()
                .is_some_and(|team| self.teams.iter().any(|query_team| query_team == team))
        {
            return false;
        }

        match self.candidate_kind {
            PoachCandidateKind::All
            | PoachCandidateKind::CategorySpecialist
            | PoachCandidateKind::DeploymentRiser
            | PoachCandidateKind::Streamer => true,
            PoachCandidateKind::Stash
            | PoachCandidateKind::GoalieStreamer
            | PoachCandidateKind::WatchAlert => true,
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

impl ConfidenceSummary {
    pub fn from_rows(rows: &[PoachPlayerRow]) -> Self {
        let mut summary = Self::default();
        for row in rows {
            match row.confidence {
                PoachConfidence::High => summary.high += 1,
                PoachConfidence::Medium => summary.medium += 1,
                PoachConfidence::Low => summary.low += 1,
                PoachConfidence::DataLimited => summary.data_limited += 1,
            }
        }
        summary
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchRulesView {
    pub context: ViewContext,
    pub rules: Vec<WatchRule>,
    pub warnings: Vec<ViewWarning>,
}

pub fn default_watch_rules_view(context: ViewContext) -> WatchRulesView {
    WatchRulesView {
        context,
        rules: vec![
            WatchRule {
                id: "category-hits-pace".to_string(),
                label: "Category specialist crosses hits threshold".to_string(),
                enabled: true,
                trigger: WatchRuleTrigger::CategoryThreshold {
                    category: "hits".to_string(),
                    threshold: 200.0,
                },
                last_fired: None,
                unsupported_sources: Vec::new(),
            },
            WatchRule {
                id: "category-blocks-pace".to_string(),
                label: "Category specialist crosses blocks threshold".to_string(),
                enabled: true,
                trigger: WatchRuleTrigger::CategoryThreshold {
                    category: "blocks".to_string(),
                    threshold: 120.0,
                },
                last_fired: None,
                unsupported_sources: Vec::new(),
            },
            WatchRule {
                id: "deployment-promotion".to_string(),
                label: "Player promotion from deployment signal".to_string(),
                enabled: true,
                trigger: WatchRuleTrigger::PlayerPromoted {
                    player_id: None,
                    evidence: DeploymentSignal::Unknown,
                },
                last_fired: None,
                unsupported_sources: vec![SourceKind::Shifts],
            },
            WatchRule {
                id: "goalie-back-to-back".to_string(),
                label: "Goalie back-to-back start candidate".to_string(),
                enabled: true,
                trigger: WatchRuleTrigger::GoalieBackToBackStart { team: None },
                last_fired: None,
                unsupported_sources: vec![SourceKind::Schedule],
            },
            WatchRule {
                id: "availability-change".to_string(),
                label: "Watched player becomes available".to_string(),
                enabled: true,
                trigger: WatchRuleTrigger::AvailabilityChanged {
                    player_id: None,
                    state: AvailabilityState::Unknown,
                },
                last_fired: None,
                unsupported_sources: vec![SourceKind::FantasyImport],
            },
        ],
        warnings: Vec::new(),
    }
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

fn poach_context_for_sources(
    season: Season,
    season_type: SeasonType,
    has_window: bool,
) -> ViewContext {
    let mut context = ViewContext::new(ViewWindow::new(season, season_type));
    if has_window {
        context.completeness = Completeness::Partial;
        context.source_state = vec![
            SourceState::complete(SourceKind::Roster),
            SourceState::missing(SourceKind::Schedule),
            SourceState::missing(SourceKind::FantasyImport),
            SourceState::missing(SourceKind::Shifts),
        ];
    } else {
        context.completeness = Completeness::Unavailable;
        context.source_state = vec![SourceState::missing(SourceKind::Roster)];
    }
    context
}

fn row_from_player(player: &PlayerView<'_>, query: &PoachQuery) -> PoachPlayerRow {
    let category_fit = category_fit_value(player, query);
    let opportunity_delta = opportunity_value(player);
    let deployment_trend = deployment_value(player);
    let schedule_value = 0.0;
    let availability_gap = 0.0;
    let roster_need_fit = 0.0;
    let risk_discount = risk_discount_value(player);
    let final_score = (opportunity_delta
        + deployment_trend
        + category_fit
        + schedule_value
        + availability_gap
        + roster_need_fit)
        .clamp(0.0, 100.0)
        - risk_discount;
    let final_score = final_score.clamp(0.0, 100.0);

    let score = PoachScore {
        final_score,
        opportunity_delta,
        deployment_trend,
        category_fit,
        schedule_value,
        availability_gap,
        roster_need_fit,
        risk_discount,
    };

    let confidence = if player.hits().is_some() || player.blocked_shots().is_some() {
        PoachConfidence::Medium
    } else {
        PoachConfidence::DataLimited
    };
    let mut tokens = vec![SemanticToken::CategoryFit];
    if risk_discount > 0.0 {
        tokens.push(SemanticToken::Risk);
    }

    PoachPlayerRow {
        player_id: player.id(),
        display_name: player.full_name().to_string(),
        team: player
            .team()
            .cloned()
            .unwrap_or_else(|| TeamAbbr("UNK".to_string())),
        position: player.position(),
        availability: AvailabilityState::Unknown,
        recommendation_kinds: vec![RecommendationKind::CategoryFit],
        score,
        confidence,
        components: vec![
            component(
                PoachComponentKind::OpportunityDelta,
                "Opportunity",
                opportunity_delta,
                ScoreRange::new(0.0, 20.0),
                ComponentStatus::Estimated,
                Some(SourceKind::Roster),
                Some(SemanticToken::Rising),
            ),
            component(
                PoachComponentKind::DeploymentTrend,
                "Deployment",
                deployment_trend,
                ScoreRange::new(0.0, 15.0),
                ComponentStatus::Estimated,
                Some(SourceKind::Roster),
                Some(SemanticToken::Rising),
            ),
            component(
                PoachComponentKind::CategoryFit,
                "Category Fit",
                category_fit,
                ScoreRange::new(0.0, 25.0),
                ComponentStatus::Measured,
                Some(SourceKind::Roster),
                Some(SemanticToken::CategoryFit),
            ),
            component(
                PoachComponentKind::ScheduleValue,
                "Schedule",
                schedule_value,
                ScoreRange::new(0.0, 15.0),
                ComponentStatus::Unavailable,
                None,
                Some(SemanticToken::SourceUnavailable),
            ),
            component(
                PoachComponentKind::AvailabilityGap,
                "Availability",
                availability_gap,
                ScoreRange::new(0.0, 10.0),
                ComponentStatus::Unavailable,
                None,
                Some(SemanticToken::SourceUnavailable),
            ),
            component(
                PoachComponentKind::RosterNeedFit,
                "Roster Need",
                roster_need_fit,
                ScoreRange::new(0.0, 15.0),
                ComponentStatus::Deferred,
                Some(SourceKind::FantasyImport),
                Some(SemanticToken::SupportingEvidence),
            ),
            component(
                PoachComponentKind::RiskDiscount,
                "Risk",
                risk_discount,
                ScoreRange::new(0.0, 30.0),
                ComponentStatus::Estimated,
                Some(SourceKind::Roster),
                Some(SemanticToken::Risk),
            ),
        ],
        deployment: DeploymentSignal::Estimated {
            proxy: "season pace and shot volume".to_string(),
            generated_at: None,
        },
        schedule_summary: "Schedule source unavailable".to_string(),
        category_fit_summary: category_fit_summary(player),
        risk_summary: (risk_discount > 0.0).then(|| "Low GP or partial source risk".to_string()),
        explanations: explanations_for_player(player, category_fit, risk_discount),
        tokens,
    }
}

fn opportunity_value(player: &PlayerView<'_>) -> f64 {
    player
        .pace_82()
        .map(|pace| (pace / 8.0).clamp(0.0, 20.0))
        .unwrap_or(0.0)
}

fn deployment_value(player: &PlayerView<'_>) -> f64 {
    player
        .shots_per_82()
        .map(|shots| (shots / 22.0).clamp(0.0, 15.0))
        .unwrap_or(0.0)
}

fn category_fit_value(player: &PlayerView<'_>, query: &PoachQuery) -> f64 {
    let wants_hits = query.categories.is_empty() || query.categories.iter().any(|c| c == "hits");
    let wants_blocks =
        query.categories.is_empty() || query.categories.iter().any(|c| c == "blocks");
    let wants_shots = query.categories.is_empty() || query.categories.iter().any(|c| c == "shots");

    let mut value: f64 = 0.0;
    if wants_hits {
        value += player.hits_per_82().unwrap_or(0.0) / 14.0;
    }
    if wants_blocks {
        value += player.blocked_shots_per_82().unwrap_or(0.0) / 10.0;
    }
    if wants_shots {
        value += player.shots_per_82().unwrap_or(0.0) / 24.0;
    }
    value.clamp(0.0, 25.0)
}

fn risk_discount_value(player: &PlayerView<'_>) -> f64 {
    match player.gp() {
        0 => 30.0,
        1..=9 => 12.0,
        _ => 0.0,
    }
}

fn category_fit_summary(player: &PlayerView<'_>) -> String {
    let hits = player
        .hits()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let blocks = player
        .blocked_shots()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    format!("Hits {hits}, blocks {blocks}, shots {}", player.shots())
}

fn explanations_for_player(
    player: &PlayerView<'_>,
    category_fit: f64,
    risk_discount: f64,
) -> Vec<PoachExplanation> {
    let mut explanations = vec![PoachExplanation {
        component: PoachComponentKind::CategoryFit,
        status: ComponentStatus::Measured,
        impact: ExplanationImpact::Positive,
        token: SemanticToken::CategoryFit,
        message: format!(
            "Category fit contributes {:.1} points from available hits, blocks, and shots.",
            category_fit
        ),
        source: Some(SourceKind::Roster),
        freshness: None,
    }];

    if player.hits().is_none() || player.blocked_shots().is_none() {
        explanations.push(PoachExplanation {
            component: PoachComponentKind::CategoryFit,
            status: ComponentStatus::Unavailable,
            impact: ExplanationImpact::Omission,
            token: SemanticToken::SourceUnavailable,
            message: "Realtime hits/blocks are unavailable; missing categories are not penalized."
                .to_string(),
            source: Some(SourceKind::Roster),
            freshness: None,
        });
    }

    explanations.push(PoachExplanation {
        component: PoachComponentKind::ScheduleValue,
        status: ComponentStatus::Unavailable,
        impact: ExplanationImpact::Omission,
        token: SemanticToken::SourceUnavailable,
        message: "Schedule extraction is not wired yet; no schedule penalty applied.".to_string(),
        source: Some(SourceKind::Schedule),
        freshness: None,
    });

    if risk_discount > 0.0 {
        explanations.push(PoachExplanation {
            component: PoachComponentKind::RiskDiscount,
            status: ComponentStatus::Estimated,
            impact: ExplanationImpact::Negative,
            token: SemanticToken::Risk,
            message: format!("Risk discount subtracts {:.1} points.", risk_discount),
            source: Some(SourceKind::Roster),
            freshness: None,
        });
    }

    explanations
}

fn component(
    kind: PoachComponentKind,
    label: &str,
    value: f64,
    range: ScoreRange,
    status: ComponentStatus,
    source: Option<SourceKind>,
    token: Option<SemanticToken>,
) -> PoachScoreComponent {
    PoachScoreComponent {
        kind,
        label: label.to_string(),
        value,
        range,
        status,
        unit: MetricUnit::Score,
        source,
        token,
    }
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

pub fn poach_report_from_board(board: PoachBoardView) -> PoachReportView {
    let omissions = report_omissions(&board.source_state);

    PoachReportView {
        context: poach_report_context(board.context.clone(), "poach-report"),
        scoring_scheme: board.scoring_scheme,
        window: board.window,
        source_state: board.source_state,
        warnings: board.warnings,
        omissions,
        sections: vec![PoachReportSection {
            id: "top_adds".to_string(),
            title: "Top Adds".to_string(),
            rows: board.rows,
        }],
    }
}

pub fn weekly_poach_report_from_board(
    board: PoachBoardView,
    league: &str,
    section_limit: u16,
) -> PoachReportView {
    let omissions = report_omissions(&board.source_state);
    let rows = board.rows;
    let limit = section_limit as usize;
    let sections = vec![
        PoachReportSection {
            id: "top_adds".to_string(),
            title: "Top Adds".to_string(),
            rows: take_rows(&rows, limit),
        },
        PoachReportSection {
            id: "category_specialists".to_string(),
            title: "Category Specialists".to_string(),
            rows: rows_matching_kind(&rows, RecommendationKind::CategoryFit, limit),
        },
        PoachReportSection {
            id: "deployment_risers".to_string(),
            title: "Deployment Risers".to_string(),
            rows: rows_matching_kind(&rows, RecommendationKind::DeploymentRiser, limit),
        },
        PoachReportSection {
            id: "risk_discounts".to_string(),
            title: "Risk Discounts".to_string(),
            rows: rows
                .iter()
                .filter(|row| row.risk_summary.is_some())
                .take(limit)
                .cloned()
                .collect(),
        },
        PoachReportSection {
            id: "watched_player_alerts".to_string(),
            title: "Watched Player Alerts".to_string(),
            rows: Vec::new(),
        },
    ];
    let mut context = poach_report_context(
        board.context.clone(),
        format!("weekly-{}", slug_or_default(league, "default")),
    );
    context.title = "Weekly Fantasy Prep".to_string();
    context.sections = sections
        .iter()
        .map(|section| ReportSectionRef {
            id: section.id.clone(),
            title: section.title.clone(),
        })
        .collect();

    PoachReportView {
        context,
        scoring_scheme: board.scoring_scheme,
        window: board.window,
        source_state: board.source_state,
        warnings: board.warnings,
        omissions,
        sections,
    }
}

fn report_omissions(source_state: &[SourceState]) -> Vec<String> {
    source_state
        .iter()
        .filter(|state| state.state != Completeness::Complete)
        .map(|state| format!("{:?}: {:?}", state.source, state.state).to_ascii_lowercase())
        .collect()
}

fn take_rows(rows: &[PoachPlayerRow], limit: usize) -> Vec<PoachPlayerRow> {
    rows.iter().take(limit).cloned().collect()
}

fn rows_matching_kind(
    rows: &[PoachPlayerRow],
    kind: RecommendationKind,
    limit: usize,
) -> Vec<PoachPlayerRow> {
    rows.iter()
        .filter(|row| row.recommendation_kinds.contains(&kind))
        .take(limit)
        .cloned()
        .collect()
}

fn slug_or_default(value: &str, default: &str) -> String {
    let slug = value
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() || ch == '-' || ch == '_' {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        default.to_string()
    } else {
        slug
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

    #[test]
    fn poach_builder_reads_repository_and_preserves_source_gaps() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::skater_modern();
        let repo = crate::fixtures::test_repo_with(identity, stats);
        let mut query = PoachQuery::new(Season(20242025), SeasonType::Regular, "yahoo-standard");
        query.categories = vec!["hits".to_string(), "blocks".to_string()];
        query.limit = Some(10);

        let board = PoachBoardView::from_repository(&repo, query);

        assert_eq!(board.rows.len(), 1);
        assert_eq!(board.context.completeness, Completeness::Partial);
        assert_eq!(board.source_state[0].source, SourceKind::Roster);
        assert_eq!(board.source_state[1].source, SourceKind::Schedule);
        assert_eq!(board.source_state[1].state, Completeness::Unavailable);
        assert_eq!(board.rows[0].display_name, "Connor McDavid");
        assert_eq!(board.rows[0].confidence, PoachConfidence::Medium);
        assert_eq!(
            board.rows[0]
                .component(PoachComponentKind::CategoryFit)
                .expect("category component")
                .status,
            ComponentStatus::Measured
        );
        assert_eq!(board.contract_errors(), Vec::<String>::new());
    }

    #[test]
    fn poach_builder_missing_window_is_unavailable_empty_state() {
        let repo = crate::stats_repository::StatsRepository::new();
        let query = PoachQuery::new(Season(19981999), SeasonType::Regular, "yahoo-standard");

        let board = PoachBoardView::from_repository(&repo, query);

        assert!(board.rows.is_empty());
        assert_eq!(board.context.completeness, Completeness::Unavailable);
        assert_eq!(
            board.empty_state.as_ref().map(|state| state.kind),
            Some(EmptyKind::MissingSource)
        );
        assert_eq!(board.source_state[0].source, SourceKind::Roster);
    }

    #[test]
    fn poach_builder_team_and_position_filters_lower_into_query() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::skater_modern();
        let repo = crate::fixtures::test_repo_with(identity, stats);
        let mut query = PoachQuery::new(Season(20242025), SeasonType::Regular, "yahoo-standard");
        query.teams = vec![TeamAbbr("TOR".to_string())];
        query.positions = vec![Position::Center];

        let board = PoachBoardView::from_repository(&repo, query);

        assert!(board.rows.is_empty());
        assert_eq!(
            board.empty_state.as_ref().map(|state| state.kind),
            Some(EmptyKind::NoMatch)
        );
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
                    ScoreRange::new(0.0, 20.0),
                    ComponentStatus::Estimated,
                    Some(SourceKind::Roster),
                    Some(SemanticToken::Rising),
                ),
                component(
                    PoachComponentKind::DeploymentTrend,
                    "Deployment",
                    0.0,
                    ScoreRange::new(0.0, 15.0),
                    ComponentStatus::Unavailable,
                    None,
                    Some(SemanticToken::SourceUnavailable),
                ),
                component(
                    PoachComponentKind::CategoryFit,
                    "Category Fit",
                    25.0,
                    ScoreRange::new(0.0, 25.0),
                    ComponentStatus::Measured,
                    Some(SourceKind::Roster),
                    Some(SemanticToken::CategoryFit),
                ),
                component(
                    PoachComponentKind::ScheduleValue,
                    "Schedule",
                    15.0,
                    ScoreRange::new(0.0, 15.0),
                    ComponentStatus::Measured,
                    Some(SourceKind::Schedule),
                    Some(SemanticToken::ScheduleEdge),
                ),
                component(
                    PoachComponentKind::AvailabilityGap,
                    "Availability",
                    0.0,
                    ScoreRange::new(0.0, 10.0),
                    ComponentStatus::Unavailable,
                    None,
                    Some(SemanticToken::SourceUnavailable),
                ),
                component(
                    PoachComponentKind::RosterNeedFit,
                    "Roster Need",
                    10.0,
                    ScoreRange::new(0.0, 15.0),
                    ComponentStatus::Estimated,
                    Some(SourceKind::FantasyImport),
                    Some(SemanticToken::SupportingEvidence),
                ),
                component(
                    PoachComponentKind::RiskDiscount,
                    "Risk",
                    0.0,
                    ScoreRange::new(0.0, 30.0),
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
        range: ScoreRange,
        status: ComponentStatus,
        source: Option<SourceKind>,
        token: Option<SemanticToken>,
    ) -> PoachScoreComponent {
        PoachScoreComponent {
            kind,
            label: label.to_string(),
            value,
            range,
            status,
            unit: MetricUnit::Score,
            source,
            token,
        }
    }
}
