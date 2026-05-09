use serde::{Deserialize, Serialize};

use crate::identity::PlayerId;
use crate::model::{Position, Season, TeamAbbr};
use crate::season_stats::SeasonType;
use crate::stats_repository::{PlayerView, StatsRepository};
use crate::view_model::context::{
    AppliedFilter, Completeness, EmptyKind, EmptyState, SortDirection, SortKey, SortState,
    SourceKind, SourceState, ViewContext, ViewWarning, ViewWindow,
};
use crate::view_model::tokens::{
    MetricCell, MetricUnit, MetricValue, SemanticToken, StatKey, ValuePrecision,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeadersView {
    pub context: ViewContext,
    pub kind: LeaderKind,
    pub applied_filters: Vec<AppliedFilter>,
    pub sort: Option<SortState>,
    pub rows: Vec<LeaderRow>,
    pub empty_state: Option<EmptyState>,
    pub warnings: Vec<ViewWarning>,
}

impl LeadersView {
    pub fn new(context: ViewContext, kind: LeaderKind) -> Self {
        Self {
            context,
            kind,
            applied_filters: Vec::new(),
            sort: None,
            rows: Vec::new(),
            empty_state: None,
            warnings: Vec::new(),
        }
    }

    pub fn skater_pace(repo: &StatsRepository, season: Season, season_type: SeasonType) -> Self {
        let has_window = repo.has_window(season, season_type);
        let mut rows: Vec<PlayerView<'_>> = repo.skaters(season, season_type).collect();
        rows.sort_by(|a, b| b.pace_sort_key().total_cmp(&a.pace_sort_key()));

        let mut view = Self::from_player_views(
            view_context(season, season_type, has_window),
            LeaderKind::Skaters,
            rows,
        );
        view.sort = Some(SortState {
            key: SortKey::from("pace_82"),
            label: "Pace 82".to_string(),
            direction: SortDirection::Desc,
        });

        if view.rows.is_empty() {
            view.empty_state = Some(EmptyState {
                kind: if has_window {
                    EmptyKind::NoRows
                } else {
                    EmptyKind::MissingSource
                },
                title: if has_window {
                    "No skaters".to_string()
                } else {
                    "Missing skater data".to_string()
                },
                detail: Some(if has_window {
                    "No skater rows are loaded for this season/type.".to_string()
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
        kind: LeaderKind,
        rows: impl IntoIterator<Item = PlayerView<'a>>,
    ) -> Self {
        let mut view = Self::new(context, kind);
        view.rows = rows
            .into_iter()
            .enumerate()
            .map(|(idx, player)| leader_row(idx as u32 + 1, &player))
            .collect();
        if view.rows.is_empty() {
            view.empty_state = Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No leaders".to_string(),
                detail: Some("No leader rows matched the current filters.".to_string()),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderKind {
    Skaters,
    Goalies,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaderRow {
    pub rank: u32,
    pub player_id: PlayerId,
    pub display_name: String,
    pub team: TeamAbbr,
    pub position: Position,
    pub primary: MetricCell,
    pub secondary: Vec<MetricCell>,
    pub tokens: Vec<SemanticToken>,
}

fn leader_row(rank: u32, player: &PlayerView<'_>) -> LeaderRow {
    LeaderRow {
        rank,
        player_id: player.id(),
        display_name: player.full_name().to_string(),
        team: player
            .team()
            .cloned()
            .unwrap_or_else(|| TeamAbbr("UNK".to_string())),
        position: player.position(),
        primary: metric_decimal(
            "pace_82",
            "Pace 82",
            player.pace_82(),
            MetricUnit::Per82,
            ValuePrecision::OneDecimal,
            Some(SemanticToken::DecisionHighlight),
        ),
        secondary: vec![
            MetricCell {
                key: StatKey::from("age"),
                label: "Age".to_string(),
                value: player
                    .identity
                    .bio
                    .birth_date
                    .as_deref()
                    .and_then(|d| d.get(..4))
                    .and_then(|y| y.parse::<u16>().ok())
                    .map(|y| 2026u16.saturating_sub(y) as i64)
                    .map(MetricValue::Integer)
                    .unwrap_or(MetricValue::Missing),
                unit: MetricUnit::Count,
                precision: ValuePrecision::Integer,
                token: None,
            },
            metric_int("gp", "GP", player.gp() as i64, MetricUnit::Games),
            metric_int("goals", "G", player.goals() as i64, MetricUnit::Goals),
            metric_int("assists", "A", player.assists() as i64, MetricUnit::Assists),
            metric_int("points", "PTS", player.points() as i64, MetricUnit::Points),
        ],
        tokens: if player.is_rankable() {
            vec![SemanticToken::SupportingEvidence]
        } else {
            vec![SemanticToken::SourcePartial]
        },
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
    token: Option<SemanticToken>,
) -> MetricCell {
    MetricCell {
        key: StatKey::from(key),
        label: label.to_string(),
        value: value
            .map(MetricValue::Decimal)
            .unwrap_or(MetricValue::Missing),
        unit,
        precision,
        token,
    }
}
