use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::Season;
use crate::season_stats::SeasonType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViewWindow {
    pub season: Season,
    pub season_type: SeasonType,
}

impl ViewWindow {
    pub fn new(season: Season, season_type: SeasonType) -> Self {
        Self {
            season,
            season_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewContext {
    pub window: ViewWindow,
    pub generated_at: Option<DateTime<Utc>>,
    pub source_state: Vec<SourceState>,
    pub completeness: Completeness,
    pub data_generation: Option<String>,
}

impl ViewContext {
    pub fn new(window: ViewWindow) -> Self {
        Self {
            window,
            generated_at: None,
            source_state: Vec::new(),
            completeness: Completeness::Complete,
            data_generation: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    Complete,
    Partial,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceState {
    pub source: SourceKind,
    pub state: Completeness,
    pub provenance: Option<SourceProvenance>,
    pub fetched_at: Option<DateTime<Utc>>,
    pub stale_reason: Option<String>,
    pub message: Option<String>,
}

impl SourceState {
    pub fn complete(source: SourceKind) -> Self {
        Self {
            source,
            state: Completeness::Complete,
            provenance: None,
            fetched_at: None,
            stale_reason: None,
            message: None,
        }
    }

    pub fn missing(source: SourceKind) -> Self {
        Self {
            source,
            state: Completeness::Unavailable,
            provenance: None,
            fetched_at: None,
            stale_reason: None,
            message: Some("source window is not loaded".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Roster,
    Schedule,
    Scores,
    Playoffs,
    Favorites,
    Watchlist,
    Career,
    Home,
    Docs,
    GameLog,
    Boxscore,
    PlayByPlay,
    Shifts,
    Transactions,
    Contracts,
    Standings,
    FantasyImport,
    Snapshot,
    Bundle,
    Cache,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceProvenance {
    Bundled,
    InstalledBundle,
    Snapshot { id: String },
    Cache { key: String },
    LiveFetch { path: String },
    Derived { from: Vec<SourceKind> },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FilterKey(pub String);

impl From<&str> for FilterKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Eq,
    NotEq,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    In,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedFilter {
    pub key: FilterKey,
    pub op: Option<FilterOp>,
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SortKey(pub String);

impl From<&str> for SortKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortState {
    pub key: SortKey,
    pub label: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmptyState {
    pub kind: EmptyKind,
    pub title: String,
    pub detail: Option<String>,
    pub recovery: Vec<RecoveryAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmptyKind {
    NoRows,
    NoMatch,
    MissingSource,
    UnsupportedWindow,
    BadFilter,
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewWarning {
    pub kind: WarningKind,
    pub source: Option<SourceKind>,
    pub message: String,
    pub recovery: Vec<RecoveryAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningKind {
    PartialSource,
    StaleSource,
    MissingSource,
    EstimatedDeployment,
    DuplicateName,
    UnsupportedFilter,
    RendererProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryAction {
    pub label: String,
    pub action: RecoveryActionKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryActionKind {
    ClearFilter { key: Option<FilterKey> },
    ChangeWindow { window: ViewWindow },
    InstallData { source: SourceKind },
    RefreshSource { source: SourceKind },
    OpenRoute { route: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportContext {
    pub kind: ReportKind,
    pub view_context: ViewContext,
    pub report_id: String,
    pub title: String,
    pub sections: Vec<ReportSectionRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    Scouting,
    Poach,
    Weekly,
    Team,
    Leaders,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportSectionRef {
    pub id: String,
    pub title: String,
}
