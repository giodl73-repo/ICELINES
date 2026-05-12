use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::identity::PlayerId;
use crate::model::{Position, Season, TeamAbbr};
use crate::name::normalize_name;
use crate::scheme::Scheme;
use crate::season_stats::SeasonType;
use crate::stats_repository::{PlayerView, StatsRepository};
use crate::view_model::context::{
    AppliedFilter, Completeness, EmptyKind, EmptyState, ReportContext, ReportKind,
    ReportSectionRef, SortDirection, SortKey, SortState, SourceKind, SourceState, ViewContext,
    ViewWarning, ViewWindow,
};
use crate::view_model::tokens::{MetricUnit, SemanticToken};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoachBoardView {
    pub context: ViewContext,
    pub query: PoachQuery,
    pub scoring_scheme: String,
    pub scoring_categories: Vec<String>,
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
            scoring_categories: query.effective_categories(),
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
        let mut context = poach_context_for_sources(query.season, query.season_type, has_window);
        if has_window && query.availability_imported {
            mark_source_complete(&mut context.source_state, SourceKind::FantasyImport);
        }
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
    pub availability_imported: bool,
    pub availability_by_player_key: BTreeMap<String, AvailabilityState>,
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
            availability_imported: false,
            availability_by_player_key: BTreeMap::new(),
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

        if !self.matches_availability(self.availability_for_player(player)) {
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

    pub fn effective_categories(&self) -> Vec<String> {
        if !self.categories.is_empty() {
            return self
                .categories
                .iter()
                .map(|c| normalize_category(c))
                .collect();
        }
        Scheme::builtin_named(&self.scoring_scheme)
            .map(|scheme| {
                scheme
                    .skater_category_keys()
                    .into_iter()
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_else(|| {
                vec![
                    "hits".to_string(),
                    "blocks".to_string(),
                    "shots".to_string(),
                ]
            })
    }

    pub fn with_imported_availability<I, S>(mut self, rostered_player_keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.availability_imported = true;
        self.availability_by_player_key = rostered_player_keys
            .into_iter()
            .map(|key| {
                (
                    normalize_name(key.as_ref()),
                    AvailabilityState::ImportedRostered,
                )
            })
            .collect();
        self
    }

    pub fn with_imported_league_availability<I, S, U, T>(
        mut self,
        rostered_player_keys: I,
        user_rostered_player_keys: U,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
        U: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        self.availability_imported = true;
        self.availability_by_player_key = rostered_player_keys
            .into_iter()
            .map(|key| {
                (
                    normalize_name(key.as_ref()),
                    AvailabilityState::ImportedRostered,
                )
            })
            .collect();
        for key in user_rostered_player_keys {
            self.availability_by_player_key.insert(
                normalize_name(key.as_ref()),
                AvailabilityState::RosteredByUser,
            );
        }
        self
    }

    pub fn availability_for_player(&self, player: &PlayerView<'_>) -> AvailabilityState {
        if !self.availability_imported {
            return AvailabilityState::Unknown;
        }
        self.availability_by_player_key
            .get(&player.identity.name_normalized)
            .copied()
            .unwrap_or(AvailabilityState::ImportedAvailable)
    }

    fn matches_availability(&self, state: AvailabilityState) -> bool {
        match self.availability_filter {
            PoachAvailabilityFilter::Any => true,
            PoachAvailabilityFilter::Available => {
                matches!(
                    state,
                    AvailabilityState::Available | AvailabilityState::ImportedAvailable
                )
            }
            PoachAvailabilityFilter::NotOnUserRoster => {
                !matches!(state, AvailabilityState::RosteredByUser)
            }
            PoachAvailabilityFilter::Watched => matches!(state, AvailabilityState::Watched),
            PoachAvailabilityFilter::ImportedAvailable => {
                matches!(state, AvailabilityState::ImportedAvailable)
            }
            PoachAvailabilityFilter::Unknown => matches!(state, AvailabilityState::Unknown),
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

pub fn watch_rules_view_with_persisted(
    mut context: ViewContext,
    persisted_rules: Vec<WatchRule>,
) -> WatchRulesView {
    context.completeness = Completeness::Partial;
    context.source_state = vec![
        SourceState::complete(SourceKind::Roster),
        SourceState::missing(SourceKind::Shifts),
        SourceState::missing(SourceKind::Schedule),
        SourceState::missing(SourceKind::FantasyImport),
    ];

    let mut view = default_watch_rules_view(context);
    view.rules.extend(persisted_rules);
    view
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
pub struct WatchAlertsView {
    pub context: ViewContext,
    pub alerts: Vec<WatchAlertRow>,
    pub source_state: Vec<SourceState>,
    pub warnings: Vec<ViewWarning>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchAlertRow {
    pub player_id: PlayerId,
    pub display_name: String,
    pub trigger: WatchAlertTrigger,
    pub severity: WatchAlertSeverity,
    pub reason: String,
    pub unsupported_sources: Vec<SourceKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchAlertTrigger {
    WatchedAvailable,
    WatchedDeploymentSignal,
    UserRosterDropRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchAlertSeverity {
    Info,
    Opportunity,
    Warning,
}

pub fn evaluate_watch_alerts(
    board: &PoachBoardView,
    watched_player_keys: &[String],
) -> WatchAlertsView {
    let watched: std::collections::BTreeSet<String> = watched_player_keys
        .iter()
        .map(|key| normalize_name(key))
        .collect();
    let mut alerts = Vec::new();

    for row in &board.rows {
        let key = normalize_name(&row.display_name);
        let is_watched = watched.contains(&key);

        if is_watched
            && matches!(
                row.availability,
                AvailabilityState::Available | AvailabilityState::ImportedAvailable
            )
        {
            alerts.push(WatchAlertRow {
                player_id: row.player_id,
                display_name: row.display_name.clone(),
                trigger: WatchAlertTrigger::WatchedAvailable,
                severity: WatchAlertSeverity::Opportunity,
                reason: format!(
                    "{} is available and has poach score {:.1}.",
                    row.display_name, row.score.final_score
                ),
                unsupported_sources: Vec::new(),
            });
        }

        if is_watched && !matches!(row.deployment, DeploymentSignal::Unknown) {
            alerts.push(WatchAlertRow {
                player_id: row.player_id,
                display_name: row.display_name.clone(),
                trigger: WatchAlertTrigger::WatchedDeploymentSignal,
                severity: WatchAlertSeverity::Info,
                reason: format!(
                    "{} has deployment evidence on the poach board.",
                    row.display_name
                ),
                unsupported_sources: vec![SourceKind::Shifts],
            });
        }

        if row.availability == AvailabilityState::RosteredByUser && row.risk_summary.is_some() {
            alerts.push(WatchAlertRow {
                player_id: row.player_id,
                display_name: row.display_name.clone(),
                trigger: WatchAlertTrigger::UserRosterDropRisk,
                severity: WatchAlertSeverity::Warning,
                reason: format!(
                    "{} is on your roster and carries risk: {}.",
                    row.display_name,
                    row.risk_summary.as_deref().unwrap_or("risk signal")
                ),
                unsupported_sources: Vec::new(),
            });
        }
    }

    alerts.sort_by(|a, b| {
        severity_rank(b.severity)
            .cmp(&severity_rank(a.severity))
            .then_with(|| a.display_name.cmp(&b.display_name))
            .then_with(|| format!("{:?}", a.trigger).cmp(&format!("{:?}", b.trigger)))
    });

    WatchAlertsView {
        context: board.context.clone(),
        alerts,
        source_state: board.source_state.clone(),
        warnings: board.warnings.clone(),
    }
}

fn severity_rank(severity: WatchAlertSeverity) -> u8 {
    match severity {
        WatchAlertSeverity::Warning => 3,
        WatchAlertSeverity::Opportunity => 2,
        WatchAlertSeverity::Info => 1,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoachReportView {
    pub context: ReportContext,
    pub scoring_scheme: String,
    pub scoring_categories: Vec<String>,
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

fn mark_source_complete(source_state: &mut [SourceState], source: SourceKind) {
    if let Some(state) = source_state.iter_mut().find(|state| state.source == source) {
        state.state = Completeness::Complete;
    }
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
        availability: query.availability_for_player(player),
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
    let categories = query.effective_categories();
    let wants_hits = categories.iter().any(|c| c == "hits");
    let wants_blocks = categories.iter().any(|c| c == "blocks");
    let wants_shots = categories.iter().any(|c| c == "shots");

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

fn normalize_category(value: &str) -> String {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "shot" | "sog" | "shots_on_goal" => "shots".to_string(),
        "blocked_shots" | "blk" => "blocks".to_string(),
        "hit" => "hits".to_string(),
        other => other.to_string(),
    }
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
        scoring_categories: board.scoring_categories,
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
    weekly_poach_report_from_board_with_watched(board, league, section_limit, &[])
}

pub fn weekly_poach_report_from_board_with_watched(
    board: PoachBoardView,
    league: &str,
    section_limit: u16,
    watched_player_keys: &[String],
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
            rows: rows_matching_watchlist(&rows, watched_player_keys, limit),
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
        scoring_categories: board.scoring_categories,
        window: board.window,
        source_state: board.source_state,
        warnings: board.warnings,
        omissions,
        sections,
    }
}

fn rows_matching_watchlist(
    rows: &[PoachPlayerRow],
    watched_player_keys: &[String],
    limit: usize,
) -> Vec<PoachPlayerRow> {
    if watched_player_keys.is_empty() {
        return Vec::new();
    }
    let watched: std::collections::HashSet<&str> =
        watched_player_keys.iter().map(String::as_str).collect();
    rows.iter()
        .filter(|row| watched.contains(normalize_name(&row.display_name).as_str()))
        .take(limit)
        .cloned()
        .collect()
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
        assert_eq!(json["scoring_categories"][0], "hits");
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
            scoring_categories: board.scoring_categories.clone(),
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
        assert_eq!(json["scoring_categories"][0], "hits");
        assert_eq!(json["sections"][0]["rows"][0]["player_id"], 8482109);
        assert_eq!(json["omissions"][0], "ownership import unavailable");
    }

    #[test]
    fn weekly_report_populates_watched_player_alerts_from_keys() {
        let board = fixture_board();
        let watched = vec!["hidden category fit".to_string()];

        let report =
            weekly_poach_report_from_board_with_watched(board, "Main League", 20, &watched);
        let section = report
            .sections
            .iter()
            .find(|section| section.id == "watched_player_alerts")
            .expect("watched section exists");

        assert_eq!(section.rows.len(), 1);
        assert_eq!(section.rows[0].display_name, "Hidden Category Fit");
    }

    #[test]
    fn watch_rules_builder_merges_persisted_rules_and_source_state() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let persisted = WatchRule {
            id: "custom-rule".to_string(),
            label: "Custom rule".to_string(),
            enabled: true,
            trigger: WatchRuleTrigger::CategoryThreshold {
                category: "shots".to_string(),
                threshold: 250.0,
            },
            last_fired: None,
            unsupported_sources: Vec::new(),
        };

        let view = watch_rules_view_with_persisted(context, vec![persisted]);

        assert_eq!(view.context.completeness, Completeness::Partial);
        assert!(view.context.source_state.iter().any(|state| {
            state.source == SourceKind::FantasyImport && state.state == Completeness::Unavailable
        }));
        assert!(view.rules.iter().any(|rule| rule.id == "custom-rule"));
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
        assert_eq!(
            board.scoring_categories,
            vec!["hits".to_string(), "blocks".to_string()]
        );
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
    fn poach_query_uses_builtin_scheme_categories_when_no_override() {
        let yahoo = PoachQuery::new(Season(20242025), SeasonType::Regular, "yahoo-standard");
        let simple = PoachQuery::new(Season(20242025), SeasonType::Regular, "simple-pts");

        assert!(yahoo.effective_categories().contains(&"hits".to_string()));
        assert!(yahoo.effective_categories().contains(&"blocks".to_string()));
        assert_eq!(simple.effective_categories(), vec!["goals", "assists"]);
    }

    #[test]
    fn poach_builder_marks_imported_rostered_and_available_players() {
        let (identity, stats) = crate::fixtures::stat_catalog_variants::skater_modern();
        let repo = crate::fixtures::test_repo_with(identity, stats);
        let query = PoachQuery::new(Season(20242025), SeasonType::Regular, "yahoo-standard")
            .with_imported_availability(["connor mcdavid"]);

        let board = PoachBoardView::from_repository(&repo, query);

        assert_eq!(board.rows.len(), 1);
        assert_eq!(
            board.rows[0].availability,
            AvailabilityState::ImportedRostered
        );
        assert!(board.source_state.iter().any(|state| {
            state.source == SourceKind::FantasyImport && state.state == Completeness::Complete
        }));
    }

    #[test]
    fn poach_builder_filters_to_imported_available_players() {
        let (identity_a, stats_a) = crate::fixtures::stat_catalog_variants::skater_modern();
        let identity_b = crate::fixtures::identity(8479999)
            .name("Free Agent Fit", "free agent fit")
            .build();
        let stats_b = crate::fixtures::stats(8479999, 20242025, "SEA")
            .realtime(40, 30, 10, 5)
            .build();
        let mut repo = crate::fixtures::test_repo_with(identity_a, stats_a);
        repo.upsert_identity(identity_b).unwrap();
        repo.upsert_stats(stats_b).unwrap();
        let mut query = PoachQuery::new(Season(20242025), SeasonType::Regular, "yahoo-standard")
            .with_imported_availability(["connor mcdavid"]);
        query.availability_filter = PoachAvailabilityFilter::ImportedAvailable;

        let board = PoachBoardView::from_repository(&repo, query);

        assert_eq!(board.rows.len(), 1);
        assert_eq!(board.rows[0].display_name, "Free Agent Fit");
        assert_eq!(
            board.rows[0].availability,
            AvailabilityState::ImportedAvailable
        );
    }

    #[test]
    fn poach_builder_marks_user_roster_separately_from_imported_rostered() {
        let (identity_a, stats_a) = crate::fixtures::stat_catalog_variants::skater_modern();
        let identity_b = crate::fixtures::identity(8479999)
            .name("Leon Draisaitl", "leon draisaitl")
            .build();
        let stats_b = crate::fixtures::stats(8479999, 20242025, "EDM")
            .realtime(25, 20, 210, 40)
            .build();
        let mut repo = crate::fixtures::test_repo_with(identity_a, stats_a);
        repo.upsert_identity(identity_b).unwrap();
        repo.upsert_stats(stats_b).unwrap();
        let query = PoachQuery::new(Season(20242025), SeasonType::Regular, "yahoo-standard")
            .with_imported_league_availability(
                ["connor mcdavid", "leon draisaitl"],
                ["connor mcdavid"],
            );

        let board = PoachBoardView::from_repository(&repo, query);
        let mcdavid = board
            .rows
            .iter()
            .find(|row| row.display_name == "Connor McDavid")
            .expect("mcdavid row");
        let draisaitl = board
            .rows
            .iter()
            .find(|row| row.display_name == "Leon Draisaitl")
            .expect("draisaitl row");

        assert_eq!(mcdavid.availability, AvailabilityState::RosteredByUser);
        assert_eq!(draisaitl.availability, AvailabilityState::ImportedRostered);
    }

    #[test]
    fn poach_builder_not_on_user_roster_keeps_other_rostered_players() {
        let (identity_a, stats_a) = crate::fixtures::stat_catalog_variants::skater_modern();
        let identity_b = crate::fixtures::identity(8479999)
            .name("Leon Draisaitl", "leon draisaitl")
            .build();
        let stats_b = crate::fixtures::stats(8479999, 20242025, "EDM")
            .realtime(25, 20, 210, 40)
            .build();
        let mut repo = crate::fixtures::test_repo_with(identity_a, stats_a);
        repo.upsert_identity(identity_b).unwrap();
        repo.upsert_stats(stats_b).unwrap();
        let mut query = PoachQuery::new(Season(20242025), SeasonType::Regular, "yahoo-standard")
            .with_imported_league_availability(
                ["connor mcdavid", "leon draisaitl"],
                ["connor mcdavid"],
            );
        query.availability_filter = PoachAvailabilityFilter::NotOnUserRoster;

        let board = PoachBoardView::from_repository(&repo, query);
        let names: Vec<_> = board
            .rows
            .iter()
            .map(|row| row.display_name.as_str())
            .collect();

        assert!(!names.contains(&"Connor McDavid"));
        assert!(names.contains(&"Leon Draisaitl"));
    }

    #[test]
    fn watch_alerts_flag_watched_available_and_user_roster_risk() {
        let (identity_a, stats_a) = crate::fixtures::stat_catalog_variants::skater_modern();
        let watched_key = identity_a.full_name.clone();
        let (mut identity_b, stats_b) = crate::fixtures::stat_catalog_variants::low_gp();
        identity_b.full_name = "Risky Roster".to_string();
        identity_b.name_normalized = "risky roster".to_string();
        let risk_key = identity_b.name_normalized.clone();
        let mut repo = crate::fixtures::test_repo_with(identity_a, stats_a);
        repo.upsert_identity(identity_b).unwrap();
        repo.upsert_stats(stats_b).unwrap();

        let query = PoachQuery::new(Season(20242025), SeasonType::Regular, "yahoo-standard")
            .with_imported_league_availability([risk_key.as_str()], [risk_key.as_str()]);
        let board = PoachBoardView::from_repository(&repo, query);
        let alerts = evaluate_watch_alerts(&board, &[watched_key]);
        let triggers: Vec<_> = alerts.alerts.iter().map(|alert| alert.trigger).collect();

        assert!(triggers.contains(&WatchAlertTrigger::WatchedAvailable));
        assert!(triggers.contains(&WatchAlertTrigger::WatchedDeploymentSignal));
        assert!(triggers.contains(&WatchAlertTrigger::UserRosterDropRisk));
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
