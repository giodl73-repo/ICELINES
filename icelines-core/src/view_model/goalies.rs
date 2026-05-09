use serde::{Deserialize, Serialize};

use crate::identity::PlayerId;
use crate::model::{Season, TeamAbbr};
use crate::season_stats::SeasonType;
use crate::stats_repository::{PlayerView, StatsRepository};
use crate::view_model::context::{
    AppliedFilter, Completeness, EmptyKind, EmptyState, SortDirection, SortKey, SortState,
    SourceKind, SourceState, ViewContext, ViewWarning, ViewWindow,
};
use crate::view_model::team_depth::DeploymentEvidence;
use crate::view_model::tokens::{
    MetricCell, MetricUnit, MetricValue, SemanticToken, StatKey, ValuePrecision,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoaliesView {
    pub context: ViewContext,
    pub applied_filters: Vec<AppliedFilter>,
    pub sort: Option<SortState>,
    pub role_filter: Option<GoalieRoleFilter>,
    pub rows: Vec<GoalieRow>,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl GoaliesView {
    pub fn new(context: ViewContext) -> Self {
        Self {
            context,
            applied_filters: Vec::new(),
            sort: None,
            role_filter: None,
            rows: Vec::new(),
            warnings: Vec::new(),
            empty_state: None,
        }
    }

    pub fn from_repository(
        repo: &StatsRepository,
        season: Season,
        season_type: SeasonType,
    ) -> Self {
        let has_window = repo.has_window(season, season_type);
        let mut rows: Vec<PlayerView<'_>> = repo.goalies(season, season_type).collect();
        rows.sort_by(|a, b| {
            let a_save = a
                .stats
                .goalie
                .as_ref()
                .and_then(|g| g.save_pct)
                .unwrap_or(0.0);
            let b_save = b
                .stats
                .goalie
                .as_ref()
                .and_then(|g| g.save_pct)
                .unwrap_or(0.0);
            b_save.total_cmp(&a_save)
        });

        let mut view = Self::new(view_context(season, season_type, has_window));
        view.sort = Some(SortState {
            key: SortKey::from("save_pct"),
            label: "Save %".to_string(),
            direction: SortDirection::Desc,
        });
        view.rows = rows.into_iter().map(|goalie| goalie_row(&goalie)).collect();

        if view.rows.is_empty() {
            view.empty_state = Some(EmptyState {
                kind: if has_window {
                    EmptyKind::NoRows
                } else {
                    EmptyKind::MissingSource
                },
                title: if has_window {
                    "No goalies".to_string()
                } else {
                    "Missing goalie data".to_string()
                },
                detail: Some(if has_window {
                    "No goalie rows are loaded for this season/type.".to_string()
                } else {
                    "The requested season/type window is not loaded.".to_string()
                }),
                recovery: Vec::new(),
            });
        }

        view
    }

    pub fn from_player_views<'a>(
        context: ViewContext,
        rows: impl IntoIterator<Item = PlayerView<'a>>,
    ) -> Self {
        let mut view = Self::new(context);
        view.rows = rows.into_iter().map(|goalie| goalie_row(&goalie)).collect();
        if view.rows.is_empty() {
            view.empty_state = Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No goalies".to_string(),
                detail: Some("No goalie rows matched the current filters.".to_string()),
                recovery: Vec::new(),
            });
        }
        view
    }
}

fn view_context(season: Season, season_type: SeasonType, has_window: bool) -> ViewContext {
    let mut context = ViewContext::new(ViewWindow::new(season, season_type));
    if !has_window {
        context.completeness = Completeness::Unavailable;
        context
            .source_state
            .push(SourceState::missing(SourceKind::Roster));
    }
    context
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalieRow {
    pub player_id: PlayerId,
    pub display_name: String,
    pub team: TeamAbbr,
    pub role_signal: GoalieRoleSignal,
    pub metrics: Vec<MetricCell>,
    pub tokens: Vec<SemanticToken>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalieRoleSignal {
    pub label: String,
    pub evidence: DeploymentEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalieRoleFilter {
    Starter,
    Tandem,
    Backup,
    Unknown,
}

fn goalie_row(goalie: &PlayerView<'_>) -> GoalieRow {
    let stats = goalie.stats.goalie.as_ref();
    let starts = stats.map(|g| g.games_started).unwrap_or(0);
    let role_label = if starts >= 45 {
        "starter"
    } else if starts >= 25 {
        "tandem"
    } else if starts > 0 {
        "backup"
    } else {
        "unknown"
    };

    GoalieRow {
        player_id: goalie.id(),
        display_name: goalie.full_name().to_string(),
        team: goalie
            .team()
            .cloned()
            .unwrap_or_else(|| TeamAbbr("UNK".to_string())),
        role_signal: GoalieRoleSignal {
            label: role_label.to_string(),
            evidence: if starts > 0 {
                DeploymentEvidence::Actual
            } else {
                DeploymentEvidence::Unknown
            },
        },
        metrics: vec![
            metric_int("gp", "GP", goalie.gp() as i64, MetricUnit::Games),
            metric_int("starts", "GS", starts as i64, MetricUnit::Games),
            metric_int(
                "wins",
                "W",
                stats.map(|g| g.wins).unwrap_or(0) as i64,
                MetricUnit::Count,
            ),
            metric_int(
                "losses",
                "L",
                stats.map(|g| g.losses).unwrap_or(0) as i64,
                MetricUnit::Count,
            ),
            MetricCell {
                key: StatKey::from("ot_losses"),
                label: "OT".to_string(),
                value: stats
                    .and_then(|g| g.ot_losses)
                    .map(|v| MetricValue::Integer(v as i64))
                    .unwrap_or(MetricValue::Missing),
                unit: MetricUnit::Count,
                precision: ValuePrecision::Integer,
                token: None,
            },
            metric_decimal(
                "save_pct",
                "SV%",
                stats.and_then(|g| g.save_pct.map(|v| v as f64)),
                MetricUnit::Percentage,
                ValuePrecision::ThreeDecimals,
            ),
            metric_decimal(
                "gaa",
                "GAA",
                stats.and_then(|g| g.goals_against_average.map(|v| v as f64)),
                MetricUnit::Score,
                ValuePrecision::TwoDecimals,
            ),
            metric_int(
                "shutouts",
                "SO",
                stats.map(|g| g.shutouts).unwrap_or(0) as i64,
                MetricUnit::Count,
            ),
            metric_int(
                "saves",
                "Saves",
                stats.map(|g| g.saves).unwrap_or(0) as i64,
                MetricUnit::Count,
            ),
        ],
        tokens: vec![SemanticToken::SupportingEvidence],
    }
}

fn metric_int(key: &str, label: &str, value: i64, unit: MetricUnit) -> MetricCell {
    MetricCell {
        key: StatKey::from(key),
        label: label.to_string(),
        value: MetricValue::Integer(value),
        unit,
        precision: ValuePrecision::Integer,
        token: None,
    }
}

fn metric_decimal(
    key: &str,
    label: &str,
    value: Option<f64>,
    unit: MetricUnit,
    precision: ValuePrecision,
) -> MetricCell {
    MetricCell {
        key: StatKey::from(key),
        label: label.to_string(),
        value: value
            .map(MetricValue::Decimal)
            .unwrap_or(MetricValue::Missing),
        unit,
        precision,
        token: None,
    }
}
