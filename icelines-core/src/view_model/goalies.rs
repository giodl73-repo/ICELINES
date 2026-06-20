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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalieLeaderboardSort {
    SavePct,
    Gaa,
    Wins,
    Losses,
    Games,
    Saves,
    Shutouts,
}

impl GoalieLeaderboardSort {
    pub fn from_key(key: &str) -> Option<Self> {
        match key.to_ascii_lowercase().as_str() {
            "sv-pct" | "svpct" | "sv%" | "save_pct" | "save-pct" => Some(Self::SavePct),
            "gaa" | "goals-against-avg" => Some(Self::Gaa),
            "wins" | "w" => Some(Self::Wins),
            "losses" | "l" => Some(Self::Losses),
            "gp" | "games" => Some(Self::Games),
            "saves" => Some(Self::Saves),
            "so" | "shutouts" => Some(Self::Shutouts),
            _ => None,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::SavePct => "save_pct",
            Self::Gaa => "gaa",
            Self::Wins => "wins",
            Self::Losses => "losses",
            Self::Games => "gp",
            Self::Saves => "saves",
            Self::Shutouts => "shutouts",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::SavePct => "Save %",
            Self::Gaa => "GAA",
            Self::Wins => "Wins",
            Self::Losses => "Losses",
            Self::Games => "GP",
            Self::Saves => "Saves",
            Self::Shutouts => "SO",
        }
    }

    pub fn direction(self) -> SortDirection {
        match self {
            Self::Gaa => SortDirection::Asc,
            Self::SavePct
            | Self::Wins
            | Self::Losses
            | Self::Games
            | Self::Saves
            | Self::Shutouts => SortDirection::Desc,
        }
    }

    pub fn compare_player_views(
        self,
        a: &PlayerView<'_>,
        b: &PlayerView<'_>,
    ) -> std::cmp::Ordering {
        let ga = a.stats.goalie.as_ref();
        let gb = b.stats.goalie.as_ref();
        let primary = match self {
            Self::SavePct => gb
                .and_then(|g| g.save_pct)
                .unwrap_or(0.0)
                .total_cmp(&ga.and_then(|g| g.save_pct).unwrap_or(0.0)),
            Self::Gaa => ga
                .and_then(|g| g.goals_against_average)
                .unwrap_or(f32::INFINITY)
                .total_cmp(
                    &gb.and_then(|g| g.goals_against_average)
                        .unwrap_or(f32::INFINITY),
                ),
            Self::Wins => gb
                .map(|g| g.wins)
                .unwrap_or(0)
                .cmp(&ga.map(|g| g.wins).unwrap_or(0)),
            Self::Losses => gb
                .map(|g| g.losses)
                .unwrap_or(0)
                .cmp(&ga.map(|g| g.losses).unwrap_or(0)),
            Self::Games => b.gp().cmp(&a.gp()),
            Self::Saves => gb
                .map(|g| g.saves)
                .unwrap_or(0)
                .cmp(&ga.map(|g| g.saves).unwrap_or(0)),
            Self::Shutouts => gb
                .map(|g| g.shutouts)
                .unwrap_or(0)
                .cmp(&ga.map(|g| g.shutouts).unwrap_or(0)),
        };
        primary.then_with(|| a.identity.id.0.cmp(&b.identity.id.0))
    }
}

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
        rows.sort_by(|a, b| GoalieLeaderboardSort::SavePct.compare_player_views(a, b));

        let mut view = Self::new(view_context(season, season_type, has_window));
        view.sort = Some(SortState {
            key: SortKey::from("save_pct"),
            label: GoalieLeaderboardSort::SavePct.label().to_string(),
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
    let advanced = goalie.stats.goalie_advanced.as_ref();
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
            metric_decimal(
                "quality_start_pct",
                "QS%",
                advanced.and_then(|g| g.quality_starts_pct.map(|v| v as f64)),
                MetricUnit::Percentage,
                ValuePrecision::ThreeDecimals,
            ),
            metric_decimal(
                "shots_against_per_60",
                "SA/60",
                advanced.and_then(|g| g.shots_against_per_60.map(|v| v as f64)),
                MetricUnit::PerGame,
                ValuePrecision::OneDecimal,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::model::Position;
    use crate::season_stats::{GoalieAdvancedStats, GoalieSeasonStats};
    use crate::stats_repository::StatsRepository;

    fn goalie_repo() -> StatsRepository {
        let mut repo = StatsRepository::new();
        for (id, name, saves, save_pct) in [
            (2, "Later Id", 600, 0.910),
            (1, "Earlier Id", 600, 0.910),
            (3, "More Saves", 900, 0.900),
        ] {
            let normalized = crate::name::normalize_name(name);
            let identity = fixtures::identity(id).name(name, &normalized).build();
            let goalie_stats = GoalieSeasonStats {
                games_started: 20,
                wins: 10,
                losses: 8,
                ot_losses: Some(2),
                ties: None,
                shots_against: saves + 50,
                goals_against: 50,
                saves,
                save_pct: Some(save_pct),
                goals_against_average: Some(2.50),
                shutouts: 1,
                time_on_ice_sec: 20 * 3600,
            };
            let mut stats = fixtures::stats(id, 20242025, "WPG")
                .position(Position::Goalie)
                .goalie(goalie_stats)
                .goalie_advanced(GoalieAdvancedStats {
                    quality_starts: 12,
                    quality_starts_pct: Some(0.600),
                    regulation_wins: 8,
                    regulation_losses: 6,
                    complete_games: 18,
                    incomplete_games: 2,
                    complete_game_pct: Some(0.900),
                    shots_against_per_60: Some(31.5),
                })
                .build();
            stats.totals.gp = 20;
            repo.upsert_identity(identity).unwrap();
            repo.upsert_stats(stats).unwrap();
        }
        repo
    }

    #[test]
    fn goalie_leaderboard_sort_uses_uniform_id_tiebreak() {
        let repo = goalie_repo();
        let mut views: Vec<_> = repo
            .goalies(Season(20242025), SeasonType::Regular)
            .collect();

        views.sort_by(|a, b| GoalieLeaderboardSort::SavePct.compare_player_views(a, b));
        assert_eq!(views[0].identity.id.0, 1);
        assert_eq!(views[1].identity.id.0, 2);

        views.sort_by(|a, b| GoalieLeaderboardSort::Saves.compare_player_views(a, b));
        assert_eq!(views[0].identity.id.0, 3);
        assert_eq!(views[1].identity.id.0, 1);
        assert_eq!(views[2].identity.id.0, 2);
    }

    #[test]
    fn goalie_row_includes_advanced_workload_metrics() {
        let repo = goalie_repo();
        let view = GoaliesView::from_repository(&repo, Season(20242025), SeasonType::Regular);
        let row = &view.rows[0];

        assert_decimal_close(metric_decimal_value(row, "quality_start_pct"), 0.600);
        assert_decimal_close(metric_decimal_value(row, "shots_against_per_60"), 31.5);
    }

    fn metric_decimal_value(row: &GoalieRow, key: &str) -> Option<f64> {
        row.metrics.iter().find_map(|metric| {
            if metric.key.0 == key {
                match metric.value {
                    MetricValue::Decimal(value) => Some(value),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    fn assert_decimal_close(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("metric should carry decimal value");
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }
}
