use crate::state::WebState;
use crate::templates::{LeaderRow, LeadersTemplate};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::identity::PlayerId;
use icelines_core::model::{Position, Season};
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    LeaderKind, LeadersView, MetricCell, MetricUnit, MetricValue, SemanticToken, SortDirection,
    SortKey as VmSortKey, SortState, StatKey, TeamAbbr, ValuePrecision, ViewContext, ViewWindow,
};
use serde::Deserialize;

/// Query params accepted by `/leaders`. King.2.2 added
/// `sort`/`pos`/`top`; King.2.3 adds `filter` (repeatable).
///
/// `filter` uses a custom Vec-preserving extractor (see
/// `parse_filters_from_query` below) because the default
/// `Query<HashMap>` collapses repeated `?filter=` keys into
/// one — silent data loss per the spec's wire-contract
/// review.
#[derive(Debug, Deserialize, Default)]
pub struct LeadersQuery {
    /// Sort key: `points` (default), `goals`, `assists`, `gp`,
    /// `ppg`. Aliases `g`/`a`/`p` accepted.
    #[serde(default)]
    pub sort: Option<String>,
    /// Position filter: `C`, `LW`, `RW`, `D`, `F` (forwards),
    /// `G` (goalies — empty for now since /leaders is skaters
    /// only; King.5 has /goalies). Case-insensitive.
    #[serde(default)]
    pub pos: Option<String>,
    /// Top-N rows to render. Default 20, clamped 1..=500.
    #[serde(default)]
    pub top: Option<usize>,

    // Sasq.9 — bio filters. All optional. Empty = no constraint.
    // The dashes-vs-underscores split on `?age-min=` happens
    // because serde_urlencoded normalizes `-` → `_` for field
    // matching when we use serde(rename) below.
    #[serde(default, rename = "age-min")]
    pub age_min: Option<u32>,
    #[serde(default, rename = "age-max")]
    pub age_max: Option<u32>,
    #[serde(default, rename = "draft-min")]
    pub draft_year_min: Option<u16>,
    #[serde(default, rename = "draft-max")]
    pub draft_year_max: Option<u16>,
    #[serde(default, rename = "height-min")]
    pub height_min: Option<u32>, // inches
    #[serde(default, rename = "height-max")]
    pub height_max: Option<u32>,
    #[serde(default, rename = "weight-min")]
    pub weight_min: Option<u32>, // pounds
    #[serde(default, rename = "weight-max")]
    pub weight_max: Option<u32>,
    /// Three-letter ISO country code, e.g. "CAN", "USA", "SWE".
    /// Case-insensitive. Matched against bio.birth_country.
    #[serde(default)]
    pub country: Option<String>,
    /// "L" or "R". Case-insensitive. Matched against
    /// bio.shoots_catches.
    #[serde(default)]
    pub shoots: Option<String>,
}

/// Sort key parsed from the `?sort=` param. Stable PascalCase
/// for use in template (`{% if active_sort == "Points" %}`).
///
/// UX.C — added every column the table renders so each header
/// is a sortable link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Points,
    Goals,
    Assists,
    Games,
    PointsPerGame,
    PlusMinus,
    Pim,
    Shots,
    ShootingPct,
    Hits,
    Blocks,
    FaceoffPct,
    PowerPlayPoints,
    // Sasq.5 — per-60 rates.
    PointsPer60,
    GoalsPer60,
    AssistsPer60,
    HitsPer60,
    BlocksPer60,
    // Sasq.4 — YoY point delta surfaces.
    Breakout,
    Decline,
}

impl SortKey {
    pub fn from_query(s: Option<&str>) -> Self {
        match s.unwrap_or("").to_ascii_lowercase().as_str() {
            "g" | "goals" => Self::Goals,
            "a" | "assists" => Self::Assists,
            "gp" | "games" => Self::Games,
            "ppg" | "points-per-game" => Self::PointsPerGame,
            "+/-" | "plus-minus" | "plusminus" => Self::PlusMinus,
            "pim" => Self::Pim,
            "sog" | "shots" => Self::Shots,
            "sh%" | "shooting-pct" | "shootingpct" => Self::ShootingPct,
            "hits" => Self::Hits,
            "blk" | "blocks" | "blocked-shots" => Self::Blocks,
            "fow%" | "faceoff" | "faceoff-win-pct" => Self::FaceoffPct,
            "ppp" | "pp-points" | "power-play-points" => Self::PowerPlayPoints,
            "p/60" | "points-per-60" | "p60" => Self::PointsPer60,
            "g/60" | "goals-per-60" | "g60" => Self::GoalsPer60,
            "a/60" | "assists-per-60" | "a60" => Self::AssistsPer60,
            "hits/60" | "h/60" | "hits-per-60" => Self::HitsPer60,
            "blocks/60" | "blk/60" | "blocks-per-60" => Self::BlocksPer60,
            "breakout" | "yoy-up" | "yoy" => Self::Breakout,
            "decline" | "yoy-down" => Self::Decline,
            // p / pts / points / "" / unknown → Points (default)
            _ => Self::Points,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Points => "Points",
            Self::Goals => "Goals",
            Self::Assists => "Assists",
            Self::Games => "Games",
            Self::PointsPerGame => "Points/Game",
            Self::PlusMinus => "+/-",
            Self::Pim => "PIM",
            Self::Shots => "Shots",
            Self::ShootingPct => "SH%",
            Self::Hits => "Hits",
            Self::Blocks => "Blocks",
            Self::FaceoffPct => "FOW%",
            Self::PowerPlayPoints => "PP P",
            Self::PointsPer60 => "P/60",
            Self::GoalsPer60 => "G/60",
            Self::AssistsPer60 => "A/60",
            Self::HitsPer60 => "Hits/60",
            Self::BlocksPer60 => "Blocks/60",
            Self::Breakout => "YoY ▲",
            Self::Decline => "YoY ▼",
        }
    }

    /// Stable URL token for column-header links.
    pub fn url_token(self) -> &'static str {
        match self {
            Self::Points => "points",
            Self::Goals => "goals",
            Self::Assists => "assists",
            Self::Games => "gp",
            Self::PointsPerGame => "ppg",
            Self::PlusMinus => "plus-minus",
            Self::Pim => "pim",
            Self::Shots => "shots",
            Self::ShootingPct => "shooting-pct",
            Self::Hits => "hits",
            Self::Blocks => "blocks",
            Self::FaceoffPct => "faceoff",
            Self::PowerPlayPoints => "ppp",
            Self::PointsPer60 => "p60",
            Self::GoalsPer60 => "g60",
            Self::AssistsPer60 => "a60",
            Self::HitsPer60 => "hits60",
            Self::BlocksPer60 => "blocks60",
            Self::Breakout => "breakout",
            Self::Decline => "decline",
        }
    }

    /// All sort keys, in display order. The leaderboard column
    /// header strip iterates this so adding a variant lights
    /// up a new column without further wiring.
    pub const ALL: &'static [SortKey] = &[
        Self::Games,
        Self::Goals,
        Self::Assists,
        Self::Points,
        Self::PointsPerGame,
        Self::PlusMinus,
        Self::Pim,
        Self::Shots,
        Self::ShootingPct,
        Self::Hits,
        Self::Blocks,
        Self::FaceoffPct,
        Self::PowerPlayPoints,
        Self::PointsPer60,
        Self::GoalsPer60,
        Self::AssistsPer60,
        Self::HitsPer60,
        Self::BlocksPer60,
        Self::Breakout,
        Self::Decline,
    ];
}

/// JSON envelope returned by `/api/v1/leaders`. Per spec
/// "URL & API contract → Response envelope":
///     { schema_version, route, data: [...rows], meta: {...} }
///
/// `data` rows use snake_case keys (spec WIRE-1 contract for
/// non-stat keys: `nhl_id`, `team_abbrev`, ...). The HTML
/// surface and the JSON surface share the same upstream
/// projection (`build_leader_rows`) so KEEL-B1 round-trip is
/// straightforward.
#[derive(Debug, serde::Serialize)]
pub struct LeaderJsonRow {
    pub name: String,
    pub position: String,
    pub team: String,
    pub games: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub points_per_game: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct LeadersMeta {
    pub season: String,
    pub season_type: String,
    pub sort: String,
    pub position_filter: Option<String>,
    pub active_filters: Vec<String>,
    pub total: usize,
    pub returned: usize,
    pub top: usize,
}

/// Shared data-path: resolves query params, applies filters,
/// sorts, returns rows + total. Both the HTML and JSON
/// handlers call this so they can't drift.
struct LeaderResult {
    rows: Vec<LeaderRow>,
    total: usize,
    sort_key: SortKey,
    pos_active_upper: String,
    top_n: usize,
    raw_filters: Vec<String>,
    active_label: String,
    active_season_id: Season,
    active_season: String,
    active_season_type: SeasonType,
}

fn leaders_view_from_template_rows(
    rows: &[LeaderRow],
    sort_key: SortKey,
    season: Season,
    season_type: SeasonType,
) -> LeadersView {
    let mut view = LeadersView::new(
        ViewContext::new(ViewWindow::new(season, season_type)),
        LeaderKind::Skaters,
    );
    view.sort = Some(SortState {
        key: VmSortKey::from(sort_key.url_token()),
        label: sort_key.label().to_owned(),
        direction: SortDirection::Desc,
    });
    view.rows = rows
        .iter()
        .enumerate()
        .map(|(idx, row)| icelines_core::LeaderRow {
            rank: idx as u32 + 1,
            player_id: PlayerId(row.nhl_id),
            display_name: row.name.clone(),
            team: TeamAbbr(row.team.clone()),
            position: position_from_template_row(row.position.as_str()),
            primary: leader_primary_metric(row, sort_key),
            secondary: vec![
                web_metric_int("gp", "GP", row.gp as i64, MetricUnit::Games),
                web_metric_int("goals", "G", row.goals as i64, MetricUnit::Goals),
                web_metric_int("assists", "A", row.assists as i64, MetricUnit::Assists),
                web_metric_int("points", "P", row.points as i64, MetricUnit::Points),
            ],
            tokens: vec![SemanticToken::SupportingEvidence],
        })
        .collect();
    view
}

fn position_from_template_row(position: &str) -> Position {
    match position {
        "C" => Position::Center,
        "LW" => Position::LeftWing,
        "RW" => Position::RightWing,
        "D" => Position::Defense,
        "G" => Position::Goalie,
        _ => Position::Defense,
    }
}

fn web_metric_int(key: &str, label: &str, value: i64, unit: MetricUnit) -> MetricCell {
    MetricCell {
        key: StatKey::from(key),
        label: label.to_owned(),
        value: MetricValue::Integer(value),
        unit,
        precision: ValuePrecision::Integer,
        token: None,
    }
}

fn web_metric_text(key: &str, label: &str, value: String) -> MetricCell {
    MetricCell {
        key: StatKey::from(key),
        label: label.to_owned(),
        value: MetricValue::Text(value),
        unit: MetricUnit::None,
        precision: ValuePrecision::Raw,
        token: Some(SemanticToken::DecisionHighlight),
    }
}

fn leader_primary_metric(row: &LeaderRow, sort_key: SortKey) -> MetricCell {
    match sort_key {
        SortKey::Points => {
            web_metric_int("points", "Points", row.points as i64, MetricUnit::Points)
        }
        SortKey::Goals => web_metric_int("goals", "Goals", row.goals as i64, MetricUnit::Goals),
        SortKey::Assists => web_metric_int(
            "assists",
            "Assists",
            row.assists as i64,
            MetricUnit::Assists,
        ),
        SortKey::Games => web_metric_int("gp", "Games", row.gp as i64, MetricUnit::Games),
        SortKey::PointsPerGame => {
            web_metric_text("points_per_game", "Points/Game", row.ppg_str.clone())
        }
        SortKey::PlusMinus => web_metric_text("plus_minus", "+/-", row.plus_minus_str.clone()),
        SortKey::Pim => web_metric_int("pim", "PIM", row.pim as i64, MetricUnit::Count),
        SortKey::Shots => web_metric_int("shots", "Shots", row.shots as i64, MetricUnit::Count),
        SortKey::ShootingPct => {
            web_metric_text("shooting_pct", "SH%", row.shooting_pct_str.clone())
        }
        SortKey::Hits => web_metric_text("hits", "Hits", row.hits_str.clone()),
        SortKey::Blocks => web_metric_text("blocks", "Blocks", row.blocks_str.clone()),
        SortKey::FaceoffPct => web_metric_text("faceoff_pct", "FOW%", row.faceoff_pct_str.clone()),
        SortKey::PowerPlayPoints => web_metric_int(
            "pp_points",
            "PP P",
            row.pp_points as i64,
            MetricUnit::Points,
        ),
        SortKey::PointsPer60 => {
            web_metric_text("points_per_60", "P/60", row.points_per_60_str.clone())
        }
        SortKey::GoalsPer60 => {
            web_metric_text("goals_per_60", "G/60", row.goals_per_60_str.clone())
        }
        SortKey::AssistsPer60 => {
            web_metric_text("assists_per_60", "A/60", row.assists_per_60_str.clone())
        }
        SortKey::HitsPer60 => {
            web_metric_text("hits_per_60", "Hits/60", row.hits_per_60_str.clone())
        }
        SortKey::BlocksPer60 => {
            web_metric_text("blocks_per_60", "Blocks/60", row.blocks_per_60_str.clone())
        }
        SortKey::Breakout | SortKey::Decline => web_metric_text(
            "points_delta",
            sort_key.label(),
            row.points_delta_str.clone(),
        ),
    }
}

fn leader_secondary_i64(row: &icelines_core::LeaderRow, key: &str) -> Option<i64> {
    row.secondary
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Integer(value) => Some(value),
            _ => None,
        })
}

fn leader_json_rows_from_view(view: &LeadersView) -> Vec<LeaderJsonRow> {
    view.rows
        .iter()
        .map(|row| {
            let games = leader_secondary_i64(row, "gp").unwrap_or_default() as u32;
            let points = leader_secondary_i64(row, "points").unwrap_or_default() as u32;
            LeaderJsonRow {
                name: row.display_name.clone(),
                position: row.position.abbreviation().to_owned(),
                team: row.team.0.clone(),
                games,
                goals: leader_secondary_i64(row, "goals").unwrap_or_default() as u32,
                assists: leader_secondary_i64(row, "assists").unwrap_or_default() as u32,
                points,
                points_per_game: if games > 0 {
                    Some(points as f64 / games as f64)
                } else {
                    None
                },
            }
        })
        .collect()
}

fn leader_template_rows_from_view(view: &LeadersView, rows: Vec<LeaderRow>) -> Vec<LeaderRow> {
    rows.into_iter()
        .zip(view.rows.iter())
        .map(|(mut row, view_row)| {
            row.nhl_id = view_row.player_id.0;
            row.name = view_row.display_name.clone();
            row.position = view_row.position.abbreviation().to_owned();
            row.team = view_row.team.0.clone();
            row.gp = leader_secondary_i64(view_row, "gp").unwrap_or_default() as u32;
            row.goals = leader_secondary_i64(view_row, "goals").unwrap_or_default() as u32;
            row.assists = leader_secondary_i64(view_row, "assists").unwrap_or_default() as u32;
            row.points = leader_secondary_i64(view_row, "points").unwrap_or_default() as u32;
            row.ppg_str = if row.gp > 0 {
                format!("{:.2}", row.points as f64 / row.gp as f64)
            } else {
                String::new()
            };
            row
        })
        .collect()
}

async fn build_leader_result(
    state: &WebState,
    q: &LeadersQuery,
    raw_query: &str,
) -> Result<LeaderResult, Response> {
    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            parse_season_type(&cfg.active_season_type),
            cfg.active_label.clone(),
        )
    };
    let season_u32: u32 = season_str.parse().map_err(|e| {
        error_page(format!(
            "active season '{season_str}' is not a valid YYYYZZZZ id: {e}"
        ))
    })?;
    let season = Season(season_u32);

    let raw_filters = parse_filters_from_query(raw_query);

    // Wave 19 — partition new-grammar filters from legacy
    // residue, mirroring the /leaders HTML route fix.
    let (new_plans, legacy_residue, helpful_errs) = partition_new_pipeline_filters(&raw_filters);
    if !helpful_errs.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Html(format!(
                "<!doctype html><html><body><h1>Bad filter</h1><pre>{}</pre></body></html>",
                helpful_errs
                    .join("\n")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;"),
            )),
        )
            .into_response());
    }

    let filter_expr = combine_filters(&legacy_residue).map_err(|e| {
        let hint = e
            .hint()
            .unwrap_or("see `icelines docs` for the filter grammar");
        (
            StatusCode::BAD_REQUEST,
            Html(format!(
                "<!doctype html><html><body>\
                         <h1>Bad filter</h1><p>{e}</p>\
                         <p style=\"color:#b71c1c\"><strong>Hint:</strong> {hint}</p>\
                         <p><a href=\"/leaders\">← back to leaders</a></p>\
                         </body></html>",
            )),
        )
            .into_response()
    })?;

    let sort_key = SortKey::from_query(q.sort.as_deref());
    let pos_filter = q.pos.as_deref().and_then(parse_position_filter);
    let top_n = q.top.unwrap_or(20).clamp(1, 500);
    let pos_active_upper = q
        .pos
        .as_deref()
        .map(str::to_ascii_uppercase)
        .unwrap_or_default();

    let (rows, total) = {
        let repo = state.repo.read().await;

        // Sasq.4 — build a pid→prior_points map by reading
        // the prior season same-type once. The lazy career
        // fan-out (UX.1) ensures historical seasons are
        // loaded into the repo when player cards have been
        // visited; otherwise this map is empty and breakout
        // sort silently degrades to 0-everywhere.
        let prior_season = icelines_core::model::Season(
            season.0.saturating_sub(10001), // YYYYZZZZ → (Y-1)(Z-1)
        );
        let prior_points: std::collections::HashMap<u32, u32> = repo
            .skaters(prior_season, season_type)
            .map(|v| (v.id().0, v.points()))
            .collect();

        // Wave 22 — hoist provider/clock construction out of
        // the per-player .filter() closure. Was rebuilt N
        // times (one per skater), each doing redundant Path
        // resolution + DataStore::open work.
        let new_plans_provider = if new_plans.is_empty() {
            None
        } else {
            Some(icelines_fetch::query_provider::IcelinesProvider::new(
                std::env::var_os("HOME")
                    .or_else(|| std::env::var_os("USERPROFILE"))
                    .map(std::path::PathBuf::from)
                    .unwrap_or_default()
                    .join(".icelines")
                    .join("data"),
            ))
        };
        let new_plans_clock = icelines_core::freshness::SystemClock;

        let mut all: Vec<LeaderRow> = repo
            .skaters(season, season_type)
            .filter(|v| match pos_filter {
                None => true,
                Some(PosFilter::Exact(p)) => v.position() == p,
                Some(PosFilter::Forwards) => matches!(
                    v.position(),
                    Position::Center | Position::LeftWing | Position::RightWing
                ),
            })
            .filter(|v| match &filter_expr {
                None => true,
                Some(expr) => expr.matches(v),
            })
            .filter(|v| {
                // Wave 19 — apply new-pipeline plans
                // (Phase Art Ross grammar — `<`, `IN`,
                // `BETWEEN`, `LIKE`, sliding/career/league).
                if new_plans.is_empty() {
                    return true;
                }
                let ctx = icelines_query::EvalCtx::from_clock(
                    new_plans_provider.as_ref().unwrap(),
                    icelines_query::StrictMode::Off,
                    false,
                    &new_plans_clock,
                    season.0,
                );
                new_plans.iter().all(|plan| plan.root.matches(v, &ctx))
            })
            .map(|v| {
                let prev = prior_points.get(&v.id().0).copied();
                super::shared::project_leader_row_with_prior(&v, prev)
            })
            .collect();
        let total = all.len();

        all.sort_by(|a, b| {
            let primary = match sort_key {
                SortKey::Points => b.points.cmp(&a.points),
                SortKey::Goals => b.goals.cmp(&a.goals),
                SortKey::Assists => b.assists.cmp(&a.assists),
                SortKey::Games => b.gp.cmp(&a.gp),
                SortKey::PointsPerGame => {
                    let ap = if a.gp > 0 {
                        a.points as f64 / a.gp as f64
                    } else {
                        0.0
                    };
                    let bp = if b.gp > 0 {
                        b.points as f64 / b.gp as f64
                    } else {
                        0.0
                    };
                    bp.partial_cmp(&ap).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::PlusMinus => b.plus_minus.cmp(&a.plus_minus),
                SortKey::Pim => b.pim.cmp(&a.pim),
                SortKey::Shots => b.shots.cmp(&a.shots),
                SortKey::ShootingPct => {
                    let av = a.shooting_pct.unwrap_or(f32::NEG_INFINITY);
                    let bv = b.shooting_pct.unwrap_or(f32::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::Hits => b.hits.unwrap_or(0).cmp(&a.hits.unwrap_or(0)),
                SortKey::Blocks => b.blocks.unwrap_or(0).cmp(&a.blocks.unwrap_or(0)),
                SortKey::FaceoffPct => {
                    let av = a.faceoff_pct.unwrap_or(f32::NEG_INFINITY);
                    let bv = b.faceoff_pct.unwrap_or(f32::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::PowerPlayPoints => b.pp_points.cmp(&a.pp_points),
                SortKey::PointsPer60 => {
                    let av = a.points_per_60.unwrap_or(f64::NEG_INFINITY);
                    let bv = b.points_per_60.unwrap_or(f64::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::GoalsPer60 => {
                    let av = a.goals_per_60.unwrap_or(f64::NEG_INFINITY);
                    let bv = b.goals_per_60.unwrap_or(f64::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::AssistsPer60 => {
                    let av = a.assists_per_60.unwrap_or(f64::NEG_INFINITY);
                    let bv = b.assists_per_60.unwrap_or(f64::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::HitsPer60 => {
                    let av = a.hits_per_60.unwrap_or(f64::NEG_INFINITY);
                    let bv = b.hits_per_60.unwrap_or(f64::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::BlocksPer60 => {
                    let av = a.blocks_per_60.unwrap_or(f64::NEG_INFINITY);
                    let bv = b.blocks_per_60.unwrap_or(f64::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::Breakout => {
                    let av = a.points_delta.unwrap_or(i32::MIN);
                    let bv = b.points_delta.unwrap_or(i32::MIN);
                    bv.cmp(&av)
                }
                SortKey::Decline => {
                    let av = a.points_delta.unwrap_or(i32::MAX);
                    let bv = b.points_delta.unwrap_or(i32::MAX);
                    av.cmp(&bv)
                }
            };
            primary
                .then(b.points.cmp(&a.points))
                .then(a.name.cmp(&b.name))
        });
        all.truncate(top_n);
        (all, total)
    };

    Ok(LeaderResult {
        rows,
        total,
        sort_key,
        pos_active_upper,
        top_n,
        raw_filters,
        active_label,
        active_season_id: season,
        active_season: season_str,
        active_season_type: season_type,
    })
}

/// `GET /api/v1/leaders` — JSON twin of `/leaders`.
pub async fn get_leaders_json(
    State(state): State<WebState>,
    Query(q): Query<LeadersQuery>,
    uri: axum::http::Uri,
) -> Response {
    let raw_query = uri.query().unwrap_or("");
    let result = match build_leader_result(&state, &q, raw_query).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    let leaders_view = leaders_view_from_template_rows(
        &result.rows,
        result.sort_key,
        result.active_season_id,
        result.active_season_type,
    );
    let returned = leaders_view.rows.len();
    let data = leader_json_rows_from_view(&leaders_view);

    let meta = LeadersMeta {
        season: result.active_season,
        season_type: match result.active_season_type {
            SeasonType::Regular => "regular".to_owned(),
            SeasonType::Playoff => "playoff".to_owned(),
        },
        sort: result.sort_key.url_token().to_owned(),
        position_filter: if result.pos_active_upper.is_empty() {
            None
        } else {
            Some(result.pos_active_upper)
        },
        active_filters: result.raw_filters,
        total: result.total,
        returned,
        top: result.top_n,
    };

    // Suppress unused warning on active_label — the JSON
    // surface doesn't render it (the meta has season +
    // season_type which clients can format themselves).
    let _ = result.active_label;
    let _ = uri;

    crate::api::json_data_meta("leaders", data, meta)
}

pub async fn get_leaders(
    State(state): State<WebState>,
    Query(q): Query<LeadersQuery>,
    uri: axum::http::Uri,
) -> Response {
    // Extract repeated `?filter=` from the raw query string.
    // The default `Query<HashMap>` collapses repeats; the
    // typed `Query<LeadersQuery>` above only captures
    // sort/pos/top because Option<String> overwrites on
    // re-parse. For filter, we need ALL occurrences ANDed.
    let raw_filters = parse_filters_from_query(uri.query().unwrap_or(""));

    // QueryA — pre-extract bio atoms from each filter's
    // top-level AND chain via the shared icelines-query crate.
    // Stat residue (stat-only pieces) is recombined for the
    // catalog parser. Filters containing OR/NOT bypass the
    // splitter and pass whole-cloth to the stat parser; users
    // who need OR with bio terms can fall back to the discrete
    // Bio Filters accordion.
    let (extracted_bio, stat_filters) = super::extract_bio(&raw_filters);

    // Wave 17 — partition stat filters into new-pipeline
    // plans (handle the full Phase Art Ross grammar:
    // `<` `>` `!=` `IN` `BETWEEN` `LIKE` + sliding/career/
    // league atoms) vs legacy-residue (the leftover that
    // the legacy parser still handles). Helpful errors
    // surface as 400 BadFilter directly.
    let (new_plans, legacy_residue, helpful_errs) = partition_new_pipeline_filters(&stat_filters);
    if !helpful_errs.is_empty() {
        let body = format!(
            "<!doctype html><html><body>\
                     <h1>Bad filter</h1><pre>{}</pre>\
                     <p><a href=\"/leaders\">← back to leaders</a></p>\
                     </body></html>",
            helpful_errs
                .join("\n")
                .replace('<', "&lt;")
                .replace('>', "&gt;"),
        );
        return (StatusCode::BAD_REQUEST, Html(body)).into_response();
    }

    let filter_expr_result = combine_filters(&legacy_residue);
    // Resolve active (season, season_type) from config.
    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            parse_season_type(&cfg.active_season_type),
            cfg.active_label.clone(),
        )
    };
    let season_u32: u32 = match season_str.parse() {
        Ok(n) => n,
        Err(e) => {
            return error_page(format!(
                "active season '{season_str}' is not a valid YYYYZZZZ id: {e}"
            ));
        }
    };
    let season = Season(season_u32);

    // Resolve query params into typed values. Invalid
    // `?pos=` is treated as no filter (per spec: don't error
    // for things the user might be exploring); invalid sort
    // falls through to the default (Points).
    let sort_key = SortKey::from_query(q.sort.as_deref());
    let pos_filter = q.pos.as_deref().and_then(parse_position_filter);
    let top_n = q.top.unwrap_or(20).clamp(1, 500);

    // If filter parsing failed, render a 400 page with the
    // hint surfaced from the parser. (Per spec: BadFilter is
    // a 400, not a 500 — the user typed something invalid;
    // it's not an internal bug.)
    let filter_expr = match filter_expr_result {
        Ok(opt) => opt,
        Err(e) => {
            let hint = e
                .hint()
                .unwrap_or("see `icelines docs` for the filter grammar");
            return (
                StatusCode::BAD_REQUEST,
                Html(format!(
                    "<!doctype html><html><body>\
                             <h1>Bad filter</h1>\
                             <p>{e}</p>\
                             <p style=\"color:#b71c1c\"><strong>Hint:</strong> {hint}</p>\
                             <p><a href=\"/leaders\">← back to leaders</a></p>\
                             </body></html>",
                )),
            )
                .into_response();
        }
    };

    // QueryA — discrete query params seed the BioConstraints,
    // grammar atoms then merge in (tightening min/max bounds,
    // overwriting country/shoots). Country/shoots are
    // uppercased and trimmed; empty strings are dropped.
    let mut bio = super::BioConstraints {
        age_min: q.age_min,
        age_max: q.age_max,
        draft_min: q.draft_year_min,
        draft_max: q.draft_year_max,
        height_min: q.height_min,
        height_max: q.height_max,
        weight_min: q.weight_min,
        weight_max: q.weight_max,
        country: q
            .country
            .as_deref()
            .map(|s| s.trim().to_ascii_uppercase())
            .filter(|s| !s.is_empty()),
        shoots: q
            .shoots
            .as_deref()
            .map(|s| s.trim().to_ascii_uppercase())
            .filter(|s| !s.is_empty()),
    };
    for atom in &extracted_bio {
        bio.merge(atom);
    }
    // Snapshot for templating — BioConstraints fields are
    // primitives so the form re-render below reads them back.
    let bio_age_min = bio.age_min;
    let bio_age_max = bio.age_max;
    let bio_draft_min = bio.draft_min;
    let bio_draft_max = bio.draft_max;
    let bio_height_min = bio.height_min;
    let bio_height_max = bio.height_max;
    let bio_weight_min = bio.weight_min;
    let bio_weight_max = bio.weight_max;
    let bio_country = bio.country.clone();
    let bio_shoots = bio.shoots.clone();

    // Brief read of the repo. Project each PlayerView into a
    // LeaderRow inside the lock scope (per spec: views must
    // not escape the lock; we copy out scalar fields).
    let (rows, total) = {
        let repo = state.repo.read().await;

        // Wave 22 — hoist provider/clock construction out of
        // the per-player .filter() closure (same fix pattern
        // as build_leader_result).
        let new_plans_provider = if new_plans.is_empty() {
            None
        } else {
            Some(icelines_fetch::query_provider::IcelinesProvider::new(
                std::env::var_os("HOME")
                    .or_else(|| std::env::var_os("USERPROFILE"))
                    .map(std::path::PathBuf::from)
                    .unwrap_or_default()
                    .join(".icelines")
                    .join("data"),
            ))
        };
        let new_plans_clock = icelines_core::freshness::SystemClock;

        let mut all: Vec<LeaderRow> = repo
            .skaters(season, season_type)
            .filter(|v| match pos_filter {
                None => true,
                Some(PosFilter::Exact(p)) => v.position() == p,
                Some(PosFilter::Forwards) => matches!(
                    v.position(),
                    Position::Center | Position::LeftWing | Position::RightWing
                ),
            })
            .filter(|v| match &filter_expr {
                None => true,
                Some(expr) => expr.matches(v),
            })
            // Wave 17 — new-pipeline plans (every filter
            // shape from Phase Art Ross: `<`/`>`/`!=`/IN/
            // BETWEEN/LIKE plus sliding-window/career/league
            // atoms). Each plan must hold for the player
            // to be included. Provider falls back to
            // empty when boxscore/career data isn't local
            // (fail-closed default).
            .filter(|v| {
                if new_plans.is_empty() {
                    return true;
                }
                let ctx = icelines_query::EvalCtx::from_clock(
                    new_plans_provider.as_ref().unwrap(),
                    icelines_query::StrictMode::Off,
                    false,
                    &new_plans_clock,
                    season.0,
                );
                new_plans.iter().all(|plan| plan.root.matches(v, &ctx))
            })
            // QueryA — bio filters via shared icelines-query
            // BioConstraints. No-op when nothing is set; when
            // a constraint is set, players missing the bio
            // field (e.g. no birth_date for an age filter) are
            // excluded.
            .filter(|v| bio.matches(v, season.0))
            .map(|v| super::shared::project_leader_row(&v))
            .collect();
        let total = all.len();

        // Sort by chosen key descending. Secondary: goals
        // desc, then name asc — deterministic tie-break.
        all.sort_by(|a, b| {
            let primary = match sort_key {
                SortKey::Points => b.points.cmp(&a.points),
                SortKey::Goals => b.goals.cmp(&a.goals),
                SortKey::Assists => b.assists.cmp(&a.assists),
                SortKey::Games => b.gp.cmp(&a.gp),
                SortKey::PointsPerGame => {
                    let a_ppg = if a.gp > 0 {
                        a.points as f64 / a.gp as f64
                    } else {
                        0.0
                    };
                    let b_ppg = if b.gp > 0 {
                        b.points as f64 / b.gp as f64
                    } else {
                        0.0
                    };
                    b_ppg
                        .partial_cmp(&a_ppg)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::PlusMinus => b.plus_minus.cmp(&a.plus_minus),
                SortKey::Pim => b.pim.cmp(&a.pim),
                SortKey::Shots => b.shots.cmp(&a.shots),
                SortKey::ShootingPct => {
                    let av = a.shooting_pct.unwrap_or(f32::NEG_INFINITY);
                    let bv = b.shooting_pct.unwrap_or(f32::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::Hits => b.hits.unwrap_or(0).cmp(&a.hits.unwrap_or(0)),
                SortKey::Blocks => b.blocks.unwrap_or(0).cmp(&a.blocks.unwrap_or(0)),
                SortKey::FaceoffPct => {
                    let av = a.faceoff_pct.unwrap_or(f32::NEG_INFINITY);
                    let bv = b.faceoff_pct.unwrap_or(f32::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::PowerPlayPoints => b.pp_points.cmp(&a.pp_points),
                SortKey::PointsPer60 => {
                    let av = a.points_per_60.unwrap_or(f64::NEG_INFINITY);
                    let bv = b.points_per_60.unwrap_or(f64::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::GoalsPer60 => {
                    let av = a.goals_per_60.unwrap_or(f64::NEG_INFINITY);
                    let bv = b.goals_per_60.unwrap_or(f64::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::AssistsPer60 => {
                    let av = a.assists_per_60.unwrap_or(f64::NEG_INFINITY);
                    let bv = b.assists_per_60.unwrap_or(f64::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::HitsPer60 => {
                    let av = a.hits_per_60.unwrap_or(f64::NEG_INFINITY);
                    let bv = b.hits_per_60.unwrap_or(f64::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::BlocksPer60 => {
                    let av = a.blocks_per_60.unwrap_or(f64::NEG_INFINITY);
                    let bv = b.blocks_per_60.unwrap_or(f64::NEG_INFINITY);
                    bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
                }
                SortKey::Breakout => {
                    let av = a.points_delta.unwrap_or(i32::MIN);
                    let bv = b.points_delta.unwrap_or(i32::MIN);
                    bv.cmp(&av)
                }
                SortKey::Decline => {
                    let av = a.points_delta.unwrap_or(i32::MAX);
                    let bv = b.points_delta.unwrap_or(i32::MAX);
                    av.cmp(&bv)
                }
            };
            primary
                .then(b.points.cmp(&a.points))
                .then(a.name.cmp(&b.name))
        });
        all.truncate(top_n);
        (all, total)
    };
    let leaders_view = leaders_view_from_template_rows(&rows, sort_key, season, season_type);
    let rows = leader_template_rows_from_view(&leaders_view, rows);

    let active_sort_token = sort_key.url_token().to_owned();
    let active_pos = q
        .pos
        .as_deref()
        .map(str::to_ascii_uppercase)
        .unwrap_or_default();

    // Pre-compute the position chips + column headers so the
    // askama template doesn't need to compare String to &str.
    let pos_chips = ["", "C", "LW", "RW", "F", "D"]
        .iter()
        .map(|p| crate::templates::PosChip {
            label: if p.is_empty() {
                "All".to_owned()
            } else {
                (*p).to_owned()
            },
            value: (*p).to_owned(),
            is_active: *p == active_pos.as_str(),
        })
        .collect();

    // UX.C — every SortKey variant lights up a header.
    // Display order matches `SortKey::ALL`. Adding a new
    // sort key automatically adds a column header.
    let col_headers = SortKey::ALL
        .iter()
        .map(|k| crate::templates::ColHeader {
            url_token: k.url_token().to_owned(),
            label: match k {
                SortKey::Games => "GP".to_owned(),
                SortKey::Goals => "G".to_owned(),
                SortKey::Assists => "A".to_owned(),
                SortKey::Points => "P".to_owned(),
                SortKey::PointsPerGame => "P/GP".to_owned(),
                _ => k.label().to_owned(),
            },
            is_active: k.url_token() == active_sort_token.as_str(),
        })
        .collect();

    // Sasq.9 — bio filter values back into the template so
    // the form re-renders with the user's current selection.
    let opt_str = |o: Option<&dyn std::fmt::Display>| -> String {
        o.map(|v| v.to_string()).unwrap_or_default()
    };
    let bio_age_min_str = bio_age_min.as_ref().map(u32::to_string).unwrap_or_default();
    let bio_age_max_str = bio_age_max.as_ref().map(u32::to_string).unwrap_or_default();
    let bio_draft_min_str = bio_draft_min
        .as_ref()
        .map(u16::to_string)
        .unwrap_or_default();
    let bio_draft_max_str = bio_draft_max
        .as_ref()
        .map(u16::to_string)
        .unwrap_or_default();
    let bio_height_min_str = bio_height_min
        .as_ref()
        .map(u32::to_string)
        .unwrap_or_default();
    let bio_height_max_str = bio_height_max
        .as_ref()
        .map(u32::to_string)
        .unwrap_or_default();
    let bio_weight_min_str = bio_weight_min
        .as_ref()
        .map(u32::to_string)
        .unwrap_or_default();
    let bio_weight_max_str = bio_weight_max
        .as_ref()
        .map(u32::to_string)
        .unwrap_or_default();
    let bio_country_str = bio_country.clone().unwrap_or_default();
    let bio_shoots_str = bio_shoots.clone().unwrap_or_default();
    let bio_active = bio_age_min.is_some()
        || bio_age_max.is_some()
        || bio_draft_min.is_some()
        || bio_draft_max.is_some()
        || bio_height_min.is_some()
        || bio_height_max.is_some()
        || bio_weight_min.is_some()
        || bio_weight_max.is_some()
        || bio_country.is_some()
        || bio_shoots.is_some();
    let _ = opt_str;

    // Build &-prefixed URL suffix so chip/column-header links
    // preserve bio narrowing across nav. urlencoding-light:
    // values are numeric or short ASCII so we just push raw.
    let mut bio_query_suffix = String::new();
    if let Some(v) = bio_age_min {
        bio_query_suffix.push_str(&format!("&age-min={v}"));
    }
    if let Some(v) = bio_age_max {
        bio_query_suffix.push_str(&format!("&age-max={v}"));
    }
    if let Some(v) = bio_draft_min {
        bio_query_suffix.push_str(&format!("&draft-min={v}"));
    }
    if let Some(v) = bio_draft_max {
        bio_query_suffix.push_str(&format!("&draft-max={v}"));
    }
    if let Some(v) = bio_height_min {
        bio_query_suffix.push_str(&format!("&height-min={v}"));
    }
    if let Some(v) = bio_height_max {
        bio_query_suffix.push_str(&format!("&height-max={v}"));
    }
    if let Some(v) = bio_weight_min {
        bio_query_suffix.push_str(&format!("&weight-min={v}"));
    }
    if let Some(v) = bio_weight_max {
        bio_query_suffix.push_str(&format!("&weight-max={v}"));
    }
    if let Some(v) = &bio_country {
        bio_query_suffix.push_str(&format!("&country={v}"));
    }
    if let Some(v) = &bio_shoots {
        bio_query_suffix.push_str(&format!("&shoots={v}"));
    }

    let tmpl = LeadersTemplate {
        active_label,
        rows,
        total,
        active_sort_label: sort_key.label().to_owned(),
        active_sort: active_sort_token,
        active_pos,
        active_top: top_n,
        pos_chips,
        col_headers,
        active_filters: raw_filters,
        bio_age_min_str,
        bio_age_max_str,
        bio_draft_min_str,
        bio_draft_max_str,
        bio_height_min_str,
        bio_height_max_str,
        bio_weight_min_str,
        bio_weight_max_str,
        bio_country: bio_country_str,
        bio_shoots: bio_shoots_str,
        bio_active,
        bio_query_suffix,
    };
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => error_page(format!("template render failed: {e}")),
    }
}

pub fn parse_season_type(s: &str) -> SeasonType {
    match s {
        "playoff" | "playoffs" => SeasonType::Playoff,
        _ => SeasonType::Regular,
    }
}

/// Pull every `filter=...` occurrence out of a raw query
/// string, in order. URL-decodes each value. Empty-string
/// values (`?filter=`) are dropped (per spec).
///
/// We do this by hand instead of using `serde_urlencoded` /
/// `serde_qs` because axum's stock `Query<T>` extractor
/// silently collapses repeated keys when T deserializes as
/// `Option<String>` — the spec's wire-review flagged this as
/// a silent-data-loss bug.
pub fn parse_filters_from_query(qs: &str) -> Vec<String> {
    qs.split('&')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            if k != "filter" {
                return None;
            }
            let decoded = urldecode(v);
            if decoded.is_empty() {
                None
            } else {
                Some(decoded)
            }
        })
        .collect()
}

/// Tiny URL-decoder for the filter parameter. Handles `%XX`
/// escapes and `+` → space (form-encoding convention). We
/// don't pull `percent-encoding` as a workspace dep just for
/// this — the filter character set is small and bounded.
fn urldecode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

/// Combine multiple `?filter=` strings into one `FilterExpr`.
/// Each is parsed independently; results are ANDed at the
/// top level (spec rule: repeated keys = AND, mirroring the
/// CLI's repeated `--filter` semantics).
pub fn combine_filters(
    raw: &[String],
) -> Result<
    Option<icelines_core::stats_catalog::FilterExpr>,
    icelines_core::stats_catalog::FilterParseError,
> {
    use icelines_core::stats_catalog::{parse_filter_expr, FilterExpr};
    let mut combined: Option<FilterExpr> = None;
    for raw_str in raw {
        let parsed = parse_filter_expr(raw_str)?;
        combined = Some(match combined {
            None => parsed,
            Some(existing) => FilterExpr::And(Box::new(existing), Box::new(parsed)),
        });
    }
    Ok(combined)
}

/// Wave 17 fix — partition raw filter strings into the
/// new-pipeline plans (handled by `parse_query`) vs
/// legacy-residue (handled by `combine_filters` →
/// `parse_filter_expr`). Mirrors the CLI's dispatch.
///
/// Returns `(new_plans, legacy_residue, helpful_errors)`:
///   - `new_plans`: parsed via the new pipeline; eval via
///     `Constraint::matches`.
///   - `legacy_residue`: filter strings the new parser
///     rejected for non-helpful reasons; pass to legacy.
///   - `helpful_errors`: parse errors with helpful
///     diagnostics (IncompatiblePredicate / EmptySet /
///     FeatureNotYet / UnknownWindowUnit / ZeroWindowSize
///     / WindowSizeOutOfRange) — surface these instead
///     of falling through to the legacy parser which
///     would give a worse "no op" error.
pub fn partition_new_pipeline_filters(
    raw: &[String],
) -> (Vec<icelines_query::QueryPlan>, Vec<String>, Vec<String>) {
    let mut plans: Vec<icelines_query::QueryPlan> = Vec::new();
    let mut legacy: Vec<String> = Vec::new();
    let mut helpful: Vec<String> = Vec::new();
    for raw_str in raw {
        match icelines_query::parse_query(icelines_query::FilterInput::Cli(raw_str.clone())) {
            Ok(plan) => plans.push(plan),
            Err(es) => {
                let prefer_new = es.iter().any(|e| {
                    matches!(
                        e,
                        icelines_query::ParseError::IncompatiblePredicate { .. }
                            | icelines_query::ParseError::EmptySet { .. }
                            | icelines_query::ParseError::FeatureNotYet { .. }
                            | icelines_query::ParseError::UnknownWindowUnit { .. }
                            | icelines_query::ParseError::ZeroWindowSize { .. }
                            | icelines_query::ParseError::WindowSizeOutOfRange { .. }
                    )
                });
                if prefer_new {
                    for e in es {
                        helpful.push(format!("--filter {raw_str:?}: {e}"));
                    }
                } else {
                    legacy.push(raw_str.clone());
                }
            }
        }
    }
    (plans, legacy, helpful)
}

/// What `?pos=X` means after parsing.
enum PosFilter {
    /// Single-position filter (C / LW / RW / D / G).
    Exact(Position),
    /// `?pos=F` — forwards = C ∪ LW ∪ RW.
    Forwards,
}

fn parse_position_filter(s: &str) -> Option<PosFilter> {
    match s.to_ascii_uppercase().as_str() {
        "C" => Some(PosFilter::Exact(Position::Center)),
        "LW" => Some(PosFilter::Exact(Position::LeftWing)),
        "RW" => Some(PosFilter::Exact(Position::RightWing)),
        "D" => Some(PosFilter::Exact(Position::Defense)),
        "G" => Some(PosFilter::Exact(Position::Goalie)),
        "F" | "FORWARD" | "FORWARDS" => Some(PosFilter::Forwards),
        _ => None,
    }
}

fn error_page(msg: String) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Html(format!(
            "<!doctype html><html><body><h1>500</h1><p>{msg}</p></body></html>"
        )),
    )
        .into_response()
}
