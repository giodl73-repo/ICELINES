use chrono::{Datelike, NaiveDate};
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
        Self::from_player_views_with_primary(context, kind, rows, default_primary_metric)
    }

    pub fn from_player_views_with_primary<'a>(
        context: ViewContext,
        kind: LeaderKind,
        rows: impl IntoIterator<Item = PlayerView<'a>>,
        primary: impl Fn(&PlayerView<'a>) -> MetricCell,
    ) -> Self {
        let season = context.window.season;
        let mut view = Self::new(context, kind);
        view.rows = rows
            .into_iter()
            .enumerate()
            .map(|(idx, player)| leader_row(season, idx as u32 + 1, &player, primary(&player)))
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

fn leader_row(
    season: Season,
    rank: u32,
    player: &PlayerView<'_>,
    primary: MetricCell,
) -> LeaderRow {
    LeaderRow {
        rank,
        player_id: player.id(),
        display_name: player.full_name().to_string(),
        team: player
            .team()
            .cloned()
            .unwrap_or_else(|| TeamAbbr("UNK".to_string())),
        position: player.position(),
        primary,
        secondary: vec![
            MetricCell {
                key: StatKey::from("age"),
                label: "Age".to_string(),
                value: player_age_for_season(player, season)
                    .map(|age| MetricValue::Integer(age as i64))
                    .unwrap_or(MetricValue::Missing),
                unit: MetricUnit::Count,
                precision: ValuePrecision::Integer,
                token: None,
            },
            metric_int("gp", "GP", player.gp() as i64, MetricUnit::Games),
            metric_int("goals", "G", player.goals() as i64, MetricUnit::Goals),
            metric_int("assists", "A", player.assists() as i64, MetricUnit::Assists),
            metric_int("points", "PTS", player.points() as i64, MetricUnit::Points),
            metric_decimal(
                "ppg",
                "PPG",
                player.pace_82().map(|pace| pace / 82.0),
                MetricUnit::PerGame,
                ValuePrecision::ThreeDecimals,
                None,
            ),
            metric_decimal(
                "pts_per_82",
                "Pts/82",
                player.pace_82(),
                MetricUnit::Per82,
                ValuePrecision::OneDecimal,
                None,
            ),
            metric_decimal(
                "goals_per_82",
                "G/82",
                player.goals_per_82(),
                MetricUnit::Per82,
                ValuePrecision::OneDecimal,
                None,
            ),
        ],
        tokens: if player.is_rankable() {
            vec![SemanticToken::SupportingEvidence]
        } else {
            vec![SemanticToken::SourcePartial]
        },
    }
}

fn player_age_for_season(player: &PlayerView<'_>, season: Season) -> Option<u8> {
    let birth_date = player.identity.bio.birth_date.as_deref()?;
    let birth_date = NaiveDate::parse_from_str(birth_date, "%Y-%m-%d").ok()?;
    let as_of = NaiveDate::from_ymd_opt(season.end_year() as i32, 1, 31)?;
    let mut age = as_of.year() - birth_date.year();
    if (as_of.month(), as_of.day()) < (birth_date.month(), birth_date.day()) {
        age -= 1;
    }
    u8::try_from(age).ok()
}

fn default_primary_metric(player: &PlayerView<'_>) -> MetricCell {
    metric_decimal(
        "pace_82",
        "Pace 82",
        player.pace_82(),
        MetricUnit::Per82,
        ValuePrecision::OneDecimal,
        Some(SemanticToken::DecisionHighlight),
    )
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
