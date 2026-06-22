use crate::state::WebState;
use crate::templates::{GoalieRow, GoaliesTemplate};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    Completeness, GoaliesView, MetricValue, SortDirection, SortKey, SortState, SourceKind,
    ViewContext, ViewWindow,
};
use serde::Deserialize;

/// Spec's rate-stat floor for goalie save-pct: 5+ GP qualifies
/// for ranking. Without this, a goalie who plays one perfect
/// period tops the leaderboard at 1.000 SV%.
const QUALIFIED_GP_REGULAR: u32 = 5;
const QUALIFIED_GP_PLAYOFF: u32 = 1;

#[derive(Debug, Deserialize, Default)]
pub struct GoaliesQuery {
    /// Sort key: `save_pct` (default), `wins`, `gaa`, `gp`,
    /// `shutouts`. Aliases `sv-pct` and `sv%` accepted.
    #[serde(default)]
    pub sort: Option<String>,
    /// Top-N rows. Default 20, clamped 1..=200.
    #[serde(default)]
    pub top: Option<usize>,
    /// Minimum games-played floor. Defaults to the route's qualified threshold.
    #[serde(default, alias = "min_gp", alias = "gp-min", alias = "min-gp")]
    pub gp_min: Option<u32>,
    /// Skip the gp_min floor (e.g. show all goalies, not
    /// just those with 5+ GP). Spec'd flag.
    #[serde(default)]
    pub include_below_threshold: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalieSort {
    SavePct,
    Wins,
    Losses,
    Games,
    Saves,
    Shutouts,
    GaaAsc, // GAA: lower is better, sort ascending
}

impl GoalieSort {
    pub fn from_query(s: Option<&str>) -> Self {
        match s.unwrap_or("").to_ascii_lowercase().as_str() {
            "wins" | "w" => Self::Wins,
            "losses" | "l" => Self::Losses,
            "gp" | "games" => Self::Games,
            "saves" => Self::Saves,
            "shutouts" | "so" => Self::Shutouts,
            "gaa" | "goals-against-avg" => Self::GaaAsc,
            _ => Self::SavePct,
        }
    }
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            Self::SavePct => "Save %",
            Self::Wins => "Wins",
            Self::Losses => "Losses",
            Self::Games => "Games",
            Self::Saves => "Saves",
            Self::Shutouts => "Shutouts",
            Self::GaaAsc => "GAA",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::SavePct => "save_pct",
            Self::Wins => "wins",
            Self::Losses => "losses",
            Self::Games => "gp",
            Self::Saves => "saves",
            Self::Shutouts => "shutouts",
            Self::GaaAsc => "gaa",
        }
    }

    pub fn direction(self) -> SortDirection {
        self.leaderboard_sort().direction()
    }

    pub fn leaderboard_sort(self) -> icelines_core::GoalieLeaderboardSort {
        match self {
            Self::SavePct => icelines_core::GoalieLeaderboardSort::SavePct,
            Self::Wins => icelines_core::GoalieLeaderboardSort::Wins,
            Self::Losses => icelines_core::GoalieLeaderboardSort::Losses,
            Self::Games => icelines_core::GoalieLeaderboardSort::Games,
            Self::Saves => icelines_core::GoalieLeaderboardSort::Saves,
            Self::Shutouts => icelines_core::GoalieLeaderboardSort::Shutouts,
            Self::GaaAsc => icelines_core::GoalieLeaderboardSort::Gaa,
        }
    }
}

/// Shared data path so HTML + JSON can't drift.
struct GoalieResult {
    rows: Vec<GoalieRow>,
    total: usize,
    sort: GoalieSort,
    qualified_threshold: u32,
    include_below_threshold: bool,
    active_label: String,
    active_season: String,
    active_season_type: SeasonType,
    top_n: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GoalieXgaSourceGate {
    source: &'static str,
    source_kind: SourceKind,
    state: Completeness,
    coverage_state: &'static str,
    basis: &'static str,
    blocked_metrics: Vec<&'static str>,
    required_evidence: Vec<&'static str>,
    limitations: Vec<&'static str>,
    label: String,
}

async fn build_goalie_result(state: &WebState, q: &GoaliesQuery) -> Result<GoalieResult, Response> {
    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        let st = SeasonType::parse_lossy(&cfg.active_season_type);
        (cfg.active_season.clone(), st, cfg.active_label.clone())
    };
    let season_u32: u32 = season_str
        .parse()
        .map_err(|_| error_500(format!("active season '{season_str}' is not a YYYYZZZZ id")))?;
    let season = Season(season_u32);
    let qualified_threshold = match season_type {
        SeasonType::Regular => QUALIFIED_GP_REGULAR,
        SeasonType::Playoff => QUALIFIED_GP_PLAYOFF,
    };
    let include_below_threshold = q.include_below_threshold.unwrap_or(false);
    let requested_floor = q.gp_min.unwrap_or(qualified_threshold);
    let effective_floor = if include_below_threshold {
        0
    } else {
        requested_floor
    };
    let sort = GoalieSort::from_query(q.sort.as_deref());
    let top_n = q.top.unwrap_or(20).clamp(1, 200);

    let (rows, total) = {
        let repo = state.repo.read().await;
        let mut all: Vec<_> = repo
            .goalies(season, season_type)
            .filter(|v| v.gp() >= effective_floor)
            .collect();
        all.sort_by(|a, b| compare_goalie_views(a, b, sort));
        let total = all.len();
        all.truncate(top_n);

        let mut view = GoaliesView::from_player_views(
            ViewContext::new(ViewWindow::new(season, season_type)),
            all,
        );
        view.sort = Some(SortState {
            key: SortKey::from(sort.key()),
            label: sort.label().to_string(),
            direction: sort.direction(),
        });

        let rows = view
            .rows
            .iter()
            .map(|row| goalie_template_row_from_view(row, season))
            .collect();
        (rows, total)
    };

    Ok(GoalieResult {
        rows,
        total,
        sort,
        qualified_threshold: effective_floor,
        include_below_threshold,
        active_label,
        active_season: season_str,
        active_season_type: season_type,
        top_n,
    })
}

fn compare_goalie_views(
    a: &icelines_core::stats_repository::PlayerView<'_>,
    b: &icelines_core::stats_repository::PlayerView<'_>,
    sort: GoalieSort,
) -> std::cmp::Ordering {
    sort.leaderboard_sort().compare_player_views(a, b)
}

fn goalie_template_row_from_view(row: &icelines_core::GoalieRow, season: Season) -> GoalieRow {
    let team = row.team.0.clone();
    GoalieRow {
        nhl_id: row.player_id.0,
        name: row.display_name.clone(),
        team: team.clone(),
        gp: goalie_metric_u32(row, "gp"),
        wins: goalie_metric_u32(row, "wins"),
        losses: goalie_metric_u32(row, "losses"),
        saves: goalie_metric_u32(row, "saves"),
        shutouts: goalie_metric_u32(row, "shutouts"),
        save_pct_str: goalie_metric_f64(row, "save_pct")
            .map(|v| format!("{v:.3}"))
            .unwrap_or_else(|| "—".to_owned()),
        gaa_str: goalie_metric_f64(row, "gaa")
            .map(|v| format!("{v:.2}"))
            .unwrap_or_else(|| "—".to_owned()),
        quality_start_pct: goalie_metric_f64(row, "quality_start_pct"),
        quality_start_pct_str: goalie_metric_f64(row, "quality_start_pct")
            .map(|v| format!("{:.1}%", v * 100.0))
            .unwrap_or_else(|| "-".to_owned()),
        shots_against_per_60: goalie_metric_f64(row, "shots_against_per_60"),
        shots_against_per_60_str: goalie_metric_f64(row, "shots_against_per_60")
            .map(|v| format!("{v:.1}"))
            .unwrap_or_else(|| "-".to_owned()),
        headshot_url: super::shared::build_headshot_url_for_display(
            season.0,
            &team,
            row.player_id.0,
        ),
        headshot_fallback_url: format!(
            "https://assets.nhle.com/mugs/nhl/default/{}.png",
            row.player_id.0
        ),
    }
}

fn goalie_metric_u32(row: &icelines_core::GoalieRow, key: &str) -> u32 {
    row.metrics
        .iter()
        .find_map(|metric| {
            if metric.key.0 == key {
                match metric.value {
                    MetricValue::Integer(value) => u32::try_from(value).ok(),
                    _ => None,
                }
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn goalie_metric_f64(row: &icelines_core::GoalieRow, key: &str) -> Option<f64> {
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

pub async fn get_goalies(State(state): State<WebState>, Query(q): Query<GoaliesQuery>) -> Response {
    let tmpl = match build_goalies_template(&state, &q).await {
        Ok(tmpl) => tmpl,
        Err(resp) => return resp,
    };
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => error_500(format!("template render failed: {e}")),
    }
}

pub async fn build_goalies_template(
    state: &WebState,
    q: &GoaliesQuery,
) -> Result<GoaliesTemplate, Response> {
    let r = build_goalie_result(state, q).await?;
    let _ = r.include_below_threshold;
    let goalie_sv_pct_svg = render_goalie_sv_pct_svg(&r.rows);
    Ok(GoaliesTemplate {
        active_label: r.active_label,
        rows: r.rows,
        goalie_xga_source_label: goalie_xga_source_gate().label,
        goalie_sv_pct_svg,
        total: r.total,
        qualified_threshold: r.qualified_threshold,
    })
}

fn render_goalie_sv_pct_svg(rows: &[GoalieRow]) -> Option<String> {
    let mut values: Vec<(&GoalieRow, f64)> = rows
        .iter()
        .filter_map(|row| {
            row.save_pct_str
                .parse::<f64>()
                .ok()
                .map(|value| (row, value))
        })
        .filter(|(_, value)| value.is_finite() && *value > 0.0)
        .collect();
    if values.is_empty() {
        return None;
    }
    values.sort_by(|(_, a), (_, b)| b.total_cmp(a));
    values.truncate(10);
    let max = values
        .iter()
        .map(|(_, value)| *value)
        .fold(0.0_f64, f64::max);
    if max <= 0.0 {
        return None;
    }

    let mut bars = String::new();
    for (idx, (row, save_pct)) in values.iter().enumerate() {
        let y = 46 + idx * 24;
        let width = ((*save_pct / max) * 420.0).max(2.0);
        let name = escape_svg_text(&row.name);
        let value = format!("{save_pct:.3}");
        bars.push_str(&format!(
            r##"  <text x="24" y="{label_y}" fill="#334155" font-size="11">{name}</text>
  <rect x="168" y="{bar_y}" width="{width:.1}" height="14" rx="3" fill="#0f766e"/>
  <text x="{value_x:.1}" y="{label_y}" fill="#0f172a" font-size="11">{value}</text>
"##,
            label_y = y + 11,
            bar_y = y,
            value_x = 176.0 + width,
        ));
    }
    let height = 72 + values.len() * 24;

    Some(format!(
        r##"<svg class="goalie-sv-pct-svg" viewBox="0 0 640 {height}" role="img" aria-labelledby="goalie-sv-title goalie-sv-desc">
  <title id="goalie-sv-title">Goalie SV% chart</title>
  <desc id="goalie-sv-desc">Returned goalies by current-window save percentage.</desc>
  <rect x="0" y="0" width="640" height="{height}" rx="8" fill="#f8fafc"/>
  <text x="24" y="26" fill="#334155" font-size="13">Goalie SV%</text>
  <line x1="168" y1="36" x2="588" y2="36" stroke="#cbd5e1"/>
{bars}</svg>"##
    ))
}

fn escape_svg_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// ── King.5.2 — JSON envelope ─────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct GoalieJsonRow {
    pub nhl_id: u32,
    pub name: String,
    pub team: String,
    pub games: u32,
    pub wins: u32,
    pub losses: u32,
    pub saves: u32,
    pub shutouts: u32,
    pub save_pct: Option<f64>,
    pub goals_against_average: Option<f64>,
    pub quality_start_pct: Option<f64>,
    pub shots_against_per_60: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct GoaliesMeta {
    pub season: String,
    pub season_type: String,
    pub sort: String,
    pub qualified_gp_min: u32,
    pub include_below_threshold: bool,
    pub total: usize,
    pub returned: usize,
    pub top: usize,
    pub goalie_xga_source: GoalieXgaSourceGate,
}

pub async fn get_goalies_json(
    State(state): State<WebState>,
    Query(q): Query<GoaliesQuery>,
) -> Response {
    let r = match build_goalie_result(&state, &q).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let returned = r.rows.len();
    let data: Vec<GoalieJsonRow> = r
        .rows
        .iter()
        .map(|row| GoalieJsonRow {
            nhl_id: row.nhl_id,
            name: row.name.clone(),
            team: row.team.clone(),
            games: row.gp,
            wins: row.wins,
            losses: row.losses,
            saves: row.saves,
            shutouts: row.shutouts,
            save_pct: row.save_pct_str.parse().ok(),
            goals_against_average: row.gaa_str.parse().ok(),
            quality_start_pct: row.quality_start_pct,
            shots_against_per_60: row.shots_against_per_60,
        })
        .collect();

    let meta = GoaliesMeta {
        season: r.active_season,
        season_type: r.active_season_type.label().to_owned(),
        sort: match r.sort {
            GoalieSort::SavePct => "save_pct".to_owned(),
            GoalieSort::Wins => "wins".to_owned(),
            GoalieSort::Losses => "losses".to_owned(),
            GoalieSort::Games => "gp".to_owned(),
            GoalieSort::Saves => "saves".to_owned(),
            GoalieSort::Shutouts => "shutouts".to_owned(),
            GoalieSort::GaaAsc => "gaa".to_owned(),
        },
        qualified_gp_min: r.qualified_threshold,
        include_below_threshold: r.include_below_threshold,
        total: r.total,
        returned,
        top: r.top_n,
        goalie_xga_source: goalie_xga_source_gate(),
    };
    let _ = r.active_label;
    crate::api::json_data_meta("goalies", data, meta)
}

fn goalie_xga_source_gate() -> GoalieXgaSourceGate {
    GoalieXgaSourceGate {
        source: "verified goalie xGA source",
        source_kind: SourceKind::Unknown,
        state: Completeness::Unavailable,
        coverage_state: "blocked",
        basis: "GSAx requires goalie-level expected goals against with schema fixtures, source-state metadata, freshness labels, and explicit non-claims",
        blocked_metrics: vec![
            "goalie_xg_against",
            "goalie_xg_against_per_60",
            "goals_saved_above_expected",
            "gsax_60",
        ],
        required_evidence: vec![
            "pinned_goalie_xga_schema_fixture",
            "goalie_identity_join_fixture",
            "freshness_and_source_state_contract",
            "missing_source_non_claim_copy",
        ],
        limitations: vec![
            "quality_start_pct_is_not_gsax",
            "shots_against_per_60_is_not_xga",
            "skater_on_ice_xga_is_not_goalie_xga",
        ],
        label: "GSAx blocked: verified goalie xGA source is not loaded or promoted yet".to_string(),
    }
}

fn error_500(msg: String) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Html(format!(
            "<!doctype html><html><body><h1>500</h1><p>{msg}</p></body></html>"
        )),
    )
        .into_response()
}
