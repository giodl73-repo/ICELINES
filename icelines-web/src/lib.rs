//! `icelines-web` — Phase King Clancy web dashboard.
//!
//! Crate skeleton from King.1.1 of the King Clancy phase plan
//! (`design/plans/2026-05-04-phaseKingClancy-1-skeleton.md`). Spec at
//! `design/specs/web-dashboard.md`.
//!
//! ## Surface
//!
//! ```ignore
//! use icelines_web::{router, WebState};
//!
//! let state = WebState::new(/* config, repo, ... */);
//! let app   = router(state);
//! axum::serve(listener, app).await?;
//! ```
//!
//! Today (King.1.1) the router exposes `/` only — a placeholder home
//! page proving the binary boots, the static-assets pipeline works, and
//! the `WebState` plumbing is wired through axum's router state.
//!
//! Subsequent sub-phases:
//! - King.1.2 — concurrency-model decision (`StatsRepository` send-conversion)
//! - King.1.3 — vendored static assets (`/static/htmx.min.js`, `/static/style.css`)
//! - King.1.4 — `templates/{base,home}.html` with active-season header
//! - King.1.5+ — `Commands::Serve` wiring + browser auto-open + LAN guard
//!
//! Each handler returns either an HTML response or a JSON envelope per
//! the URL & API contract section of the spec. Errors flow through
//! [`WebError`] which implements [`axum::response::IntoResponse`] so the
//! handler bodies stay free of HTTP boilerplate.

pub mod config;
pub mod error;
pub mod state;
pub mod static_assets;
pub mod templates;

use axum::{routing::get, Router};

pub use config::WebConfig;
pub use error::{Wants, WebError};
pub use state::WebState;

/// Build the axum router for the IceLines web dashboard.
///
/// Routes mounted today:
/// - `GET /` — placeholder home (King.1.1)
/// - `GET /static/:asset` — vendored CSS / HTMX / logo (King.1.3)
///
/// Later sub-phases attach real surfaces (`/leaders`, `/player/:id`,
/// `/api/v1/*`, ...).
///
/// Note: `/site/*` mkdocs mount + the corresponding `RouterConfig` /
/// `router_with` constructor were removed 2026-05-04 alongside the
/// mkdocs-frontend cut. If a future sub-phase wants optional disk
/// directory mounts, reintroduce a `RouterConfig` then.
pub fn router(state: WebState) -> Router {
    use handlers::coming_soon as cs;
    Router::new()
        .route("/", get(handlers::home::get_home))
        .route("/static/:asset", get(static_assets::serve_static))
        // Real handlers — replace coming-soon stubs as each lands.
        .route("/leaders", get(handlers::leaders::get_leaders))
        // JSON API — King.2.4. /api/v1/leaders is the JSON twin of
        // /leaders. Same query params; envelope shape per spec.
        .route("/api/v1/leaders", get(handlers::leaders::get_leaders_json))
        // Player card — King.3.1. Name links on /leaders point here.
        .route("/player/:id", get(handlers::player::get_player))
        // Goalie leaderboard — King.5.1 / .5.2.
        .route("/goalies", get(handlers::goalies::get_goalies))
        .route("/api/v1/goalies", get(handlers::goalies::get_goalies_json))
        // Team roster — King.4.1. /team/SEA, /team/EDM, etc.
        .route("/team/:abbrev", get(handlers::team::get_team))
        // Docs — King.8.1. Rendered COMMANDS.md.
        .route("/docs", get(handlers::docs::get_docs))
        // Coming-soon stubs for the rest of the section nav links.
        // Each lands on a real page (with the active-season header)
        // so clicks don't fail with a bare 404. Replaced by real
        // handlers in their respective sub-phases.
        .route("/scores", get(cs::scores))
        .route("/playoffs", get(cs::playoffs))
        .route("/transactions", get(cs::transactions))
        .route("/fantasy", get(cs::fantasy))
        .with_state(state)
}

mod handlers {
    /// Coming-soon stub handlers for routes whose real implementation
    /// hasn't shipped yet. Each fn renders the same template with a
    /// title + King.X label + one-sentence description.
    pub mod coming_soon {
        use crate::state::WebState;
        use crate::templates::ComingSoonTemplate;
        use askama::Template;
        use axum::extract::State;
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};

        async fn render(state: WebState, title: &str, king: &str, desc: &str) -> Response {
            let active_label = state.config.read().await.active_label.clone();
            let tmpl = ComingSoonTemplate {
                title: title.to_owned(),
                king_phase: king.to_owned(),
                description: desc.to_owned(),
                active_label,
            };
            match tmpl.render() {
                Ok(html) => Html(html).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("template error: {e}")),
                )
                    .into_response(),
            }
        }

        // `leaders` and `goalies` stubs removed (real handlers at
        // `handlers::leaders` and `handlers::goalies`).

        pub async fn scores(State(s): State<WebState>) -> Response {
            render(
                s,
                "Scores",
                "King.7",
                "Tonight's NHL games + a date picker for any past or future date. \
                 Live data from the NHL API.",
            )
            .await
        }

        pub async fn playoffs(State(s): State<WebState>) -> Response {
            render(
                s,
                "Playoffs",
                "King.7",
                "Current playoff bracket (or a historical season's). Click a series \
                 for the per-game log; click a game for the full boxscore.",
            )
            .await
        }

        pub async fn transactions(State(s): State<WebState>) -> Response {
            render(
                s,
                "Transactions",
                "King.8",
                "League-wide transactions feed: trades, signings, recalls, IR, waivers. \
                 Filterable by team / player / date / kind. Same data as `icelines transactions`.",
            )
            .await
        }

        pub async fn fantasy(State(s): State<WebState>) -> Response {
            render(
                s,
                "Fantasy",
                "King.9",
                "Fantasy league dashboard — standings, team rosters, scheme manager. \
                 Folds in the existing `icelines fantasy serve` axum routes under one root.",
            )
            .await
        }

        // `docs` stub removed in King.8.1 — real handler at
        // `handlers::docs::get_docs` renders COMMANDS.md as HTML.
    }

    /// `/leaders` — King.2.1 minimum viable real-data leaderboard.
    ///
    /// Reads the active season + season type out of `WebState.config`,
    /// iterates `repo.skaters(...)`, sorts by points descending, takes
    /// top 20, projects each into a `LeaderRow`, renders the template.
    ///
    /// What's NOT here yet (lands in King.2.2/.2.3):
    /// - filter form (?filter=g>=50)
    /// - sort picker (?sort=ppg, ?sort=hits)
    /// - pagination (?limit/?offset)
    /// - JSON twin at /api/v1/leaders
    /// - moka response cache
    pub mod leaders {
        use crate::state::WebState;
        use crate::templates::{LeaderRow, LeadersTemplate};
        use askama::Template;
        use axum::extract::{Query, State};
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};
        use icelines_core::model::{Position, Season};
        use icelines_core::season_stats::SeasonType;
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
        }

        /// Sort key parsed from the `?sort=` param. Stable PascalCase
        /// for use in template (`{% if active_sort == "Points" %}`).
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum SortKey {
            Points,
            Goals,
            Assists,
            Games,
            PointsPerGame,
        }

        impl SortKey {
            pub fn from_query(s: Option<&str>) -> Self {
                match s.unwrap_or("").to_ascii_lowercase().as_str() {
                    "g" | "goals" => Self::Goals,
                    "a" | "assists" => Self::Assists,
                    "gp" | "games" => Self::Games,
                    "ppg" | "points-per-game" => Self::PointsPerGame,
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
                }
            }
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
        pub struct LeadersEnvelope {
            pub schema_version: u32,
            pub route: &'static str,
            pub data: Vec<LeaderJsonRow>,
            pub meta: LeadersMeta,
        }

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
            active_season: String,
            active_season_type: SeasonType,
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
            let filter_expr = combine_filters(&raw_filters).map_err(|e| {
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
                    .map(|v| {
                        let gp = v.gp();
                        let points = v.points();
                        let ppg_str = if gp > 0 {
                            format!("{:.2}", points as f64 / gp as f64)
                        } else {
                            String::new()
                        };
                        LeaderRow {
                            nhl_id: v.id().0,
                            name: v.full_name().to_owned(),
                            position: v.position().abbreviation().to_owned(),
                            team: v.team_display().to_owned(),
                            gp,
                            goals: v.goals(),
                            assists: v.assists(),
                            points,
                            ppg_str,
                        }
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
                    };
                    primary
                        .then(b.goals.cmp(&a.goals))
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

            let returned = result.rows.len();
            let data: Vec<LeaderJsonRow> = result
                .rows
                .iter()
                .map(|r| LeaderJsonRow {
                    name: r.name.clone(),
                    position: r.position.clone(),
                    team: r.team.clone(),
                    games: r.gp,
                    goals: r.goals,
                    assists: r.assists,
                    points: r.points,
                    points_per_game: if r.gp > 0 {
                        Some(r.points as f64 / r.gp as f64)
                    } else {
                        None
                    },
                })
                .collect();

            let envelope = LeadersEnvelope {
                schema_version: 1,
                route: "leaders",
                data,
                meta: LeadersMeta {
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
                },
            };

            // Suppress unused warning on active_label — the JSON
            // surface doesn't render it (the meta has season +
            // season_type which clients can format themselves).
            let _ = result.active_label;
            let _ = uri;

            axum::Json(envelope).into_response()
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
            let filter_expr_result = combine_filters(&raw_filters);
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

            // Brief read of the repo. Project each PlayerView into a
            // LeaderRow inside the lock scope (per spec: views must
            // not escape the lock; we copy out scalar fields).
            let (rows, total) = {
                let repo = state.repo.read().await;
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
                    .map(|v| {
                        let gp = v.gp();
                        let points = v.points();
                        let ppg_str = if gp > 0 {
                            format!("{:.2}", points as f64 / gp as f64)
                        } else {
                            String::new()
                        };
                        LeaderRow {
                            nhl_id: v.id().0,
                            name: v.full_name().to_owned(),
                            position: v.position().abbreviation().to_owned(),
                            team: v.team_display().to_owned(),
                            gp,
                            goals: v.goals(),
                            assists: v.assists(),
                            points,
                            ppg_str,
                        }
                    })
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
                    };
                    primary
                        .then(b.goals.cmp(&a.goals))
                        .then(a.name.cmp(&b.name))
                });
                all.truncate(top_n);
                (all, total)
            };

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

            let col_headers = [
                ("gp", "GP"),
                ("goals", "G"),
                ("assists", "A"),
                ("points", "P"),
                ("ppg", "P/GP"),
            ]
            .iter()
            .map(|(token, label)| crate::templates::ColHeader {
                url_token: (*token).to_owned(),
                label: (*label).to_owned(),
                is_active: *token == active_sort_token.as_str(),
            })
            .collect();

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
            };
            match tmpl.render() {
                Ok(html) => Html(html).into_response(),
                Err(e) => error_page(format!("template render failed: {e}")),
            }
        }

        fn parse_season_type(s: &str) -> SeasonType {
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
    }

    /// `/docs` — King.8.1. Renders COMMANDS.md as HTML.
    pub mod docs {
        use crate::state::WebState;
        use crate::templates::DocsTemplate;
        use askama::Template;
        use axum::extract::State;
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};
        use std::sync::OnceLock;

        /// COMMANDS.md is embedded at compile time. Same source the
        /// CLI's `icelines docs` subcommand reads — no drift.
        const COMMANDS_MD: &str = include_str!("../../COMMANDS.md");

        /// Pre-rendered HTML cached for the lifetime of the process.
        /// COMMANDS.md changes only at compile time (it's a baked-in
        /// asset), so rendering once at first request and caching
        /// forever is correct.
        static RENDERED: OnceLock<String> = OnceLock::new();

        fn rendered() -> &'static str {
            RENDERED.get_or_init(|| {
                use pulldown_cmark::{html, Options, Parser};
                let mut opts = Options::empty();
                opts.insert(Options::ENABLE_TABLES);
                opts.insert(Options::ENABLE_STRIKETHROUGH);
                opts.insert(Options::ENABLE_FOOTNOTES);
                let parser = Parser::new_ext(COMMANDS_MD, opts);
                let mut out = String::with_capacity(COMMANDS_MD.len() * 2);
                html::push_html(&mut out, parser);
                out
            })
        }

        pub async fn get_docs(State(state): State<WebState>) -> Response {
            let active_label = state.config.read().await.active_label.clone();
            let tmpl = DocsTemplate {
                active_label,
                rendered_html: rendered().to_owned(),
            };
            match tmpl.render() {
                Ok(html) => Html(html).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("template render failed: {e}")),
                )
                    .into_response(),
            }
        }
    }

    /// `/team/:abbrev` — King.4.1 roster page.
    pub mod team {
        use crate::state::WebState;
        use crate::templates::{GoalieRow, LeaderRow, TeamTemplate};
        use askama::Template;
        use axum::extract::{Path, State};
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};
        use icelines_core::model::{Season, TeamAbbr};
        use icelines_core::season_stats::SeasonType;

        pub async fn get_team(
            State(state): State<WebState>,
            Path(abbrev_raw): Path<String>,
        ) -> Response {
            let abbrev_upper = abbrev_raw.to_ascii_uppercase();
            let team = match TeamAbbr::parse(&abbrev_upper) {
                Ok(t) => t,
                Err(e) => {
                    return (
                        StatusCode::NOT_FOUND,
                        Html(format!(
                            "<!doctype html><html><body><h1>Unknown team</h1>\
                             <p>'{abbrev_upper}' is not a recognized NHL team abbrev: {e}</p>\
                             <p><a href=\"/leaders\">← back to leaders</a></p>\
                             </body></html>"
                        )),
                    )
                        .into_response();
                }
            };

            let (season_str, season_type, active_label) = {
                let cfg = state.config.read().await;
                let st = match cfg.active_season_type.as_str() {
                    "playoff" | "playoffs" => SeasonType::Playoff,
                    _ => SeasonType::Regular,
                };
                (cfg.active_season.clone(), st, cfg.active_label.clone())
            };
            let season_u32: u32 = match season_str.parse() {
                Ok(n) => n,
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Html(format!(
                            "<!doctype html><html><body><h1>500</h1>\
                             <p>Active season '{season_str}' is not a YYYYZZZZ id</p>\
                             </body></html>"
                        )),
                    )
                        .into_response();
                }
            };
            let season = Season(season_u32);

            let (skaters, goalies) = {
                let repo = state.repo.read().await;
                let roster = repo.team_roster(&team, season, season_type);

                let mut skaters: Vec<LeaderRow> = roster
                    .iter()
                    .filter(|v| !v.is_goalie())
                    .map(|v| {
                        let gp = v.gp();
                        let points = v.points();
                        let ppg_str = if gp > 0 {
                            format!("{:.2}", points as f64 / gp as f64)
                        } else {
                            String::new()
                        };
                        LeaderRow {
                            nhl_id: v.id().0,
                            name: v.full_name().to_owned(),
                            position: v.position().abbreviation().to_owned(),
                            team: v.team_display().to_owned(),
                            gp,
                            goals: v.goals(),
                            assists: v.assists(),
                            points,
                            ppg_str,
                        }
                    })
                    .collect();
                skaters.sort_by(|a, b| {
                    b.points
                        .cmp(&a.points)
                        .then(b.goals.cmp(&a.goals))
                        .then(a.name.cmp(&b.name))
                });

                let mut goalies: Vec<GoalieRow> = roster
                    .iter()
                    .filter(|v| v.is_goalie())
                    .filter_map(|v| {
                        let g = v.stats.goalie.as_ref()?;
                        let save_pct_str = match g.save_pct {
                            Some(p) => format!("{:.3}", p),
                            None => "—".to_owned(),
                        };
                        let gaa_str = match g.goals_against_average {
                            Some(a) => format!("{:.2}", a),
                            None => "—".to_owned(),
                        };
                        Some(GoalieRow {
                            nhl_id: v.id().0,
                            name: v.full_name().to_owned(),
                            team: v.team_display().to_owned(),
                            gp: v.gp(),
                            wins: g.wins,
                            losses: g.losses,
                            shutouts: g.shutouts,
                            save_pct_str,
                            gaa_str,
                        })
                    })
                    .collect();
                goalies.sort_by(|a, b| b.wins.cmp(&a.wins).then(a.name.cmp(&b.name)));

                (skaters, goalies)
            };

            let tmpl = TeamTemplate {
                active_label,
                team_abbrev: team.0.to_string(),
                skaters,
                goalies,
            };
            match tmpl.render() {
                Ok(html) => Html(html).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("template render failed: {e}")),
                )
                    .into_response(),
            }
        }
    }

    /// `/goalies` — King.5.1 + King.5.2 goalie leaderboard.
    pub mod goalies {
        use crate::state::WebState;
        use crate::templates::{GoalieRow, GoaliesTemplate};
        use askama::Template;
        use axum::extract::{Query, State};
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};
        use icelines_core::model::Season;
        use icelines_core::season_stats::SeasonType;
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
            Shutouts,
            GaaAsc, // GAA: lower is better, sort ascending
        }

        impl GoalieSort {
            pub fn from_query(s: Option<&str>) -> Self {
                match s.unwrap_or("").to_ascii_lowercase().as_str() {
                    "wins" | "w" => Self::Wins,
                    "losses" | "l" => Self::Losses,
                    "gp" | "games" => Self::Games,
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
                    Self::Shutouts => "Shutouts",
                    Self::GaaAsc => "GAA",
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

        async fn build_goalie_result(
            state: &WebState,
            q: &GoaliesQuery,
        ) -> Result<GoalieResult, Response> {
            let (season_str, season_type, active_label) = {
                let cfg = state.config.read().await;
                let st = match cfg.active_season_type.as_str() {
                    "playoff" | "playoffs" => SeasonType::Playoff,
                    _ => SeasonType::Regular,
                };
                (cfg.active_season.clone(), st, cfg.active_label.clone())
            };
            let season_u32: u32 = season_str.parse().map_err(|_| {
                error_500(format!("active season '{season_str}' is not a YYYYZZZZ id"))
            })?;
            let season = Season(season_u32);
            let qualified_threshold = match season_type {
                SeasonType::Regular => QUALIFIED_GP_REGULAR,
                SeasonType::Playoff => QUALIFIED_GP_PLAYOFF,
            };
            let include_below_threshold = q.include_below_threshold.unwrap_or(false);
            let effective_floor = if include_below_threshold {
                0
            } else {
                qualified_threshold
            };
            let sort = GoalieSort::from_query(q.sort.as_deref());
            let top_n = q.top.unwrap_or(20).clamp(1, 200);

            let (rows, total) = {
                let repo = state.repo.read().await;
                let mut all: Vec<GoalieRow> = repo
                    .goalies(season, season_type)
                    .filter(|v| v.gp() >= effective_floor)
                    .filter_map(|v| {
                        let g = v.stats.goalie.as_ref()?;
                        let save_pct_str = match g.save_pct {
                            Some(p) => format!("{:.3}", p),
                            None => "—".to_owned(),
                        };
                        let gaa_str = match g.goals_against_average {
                            Some(a) => format!("{:.2}", a),
                            None => "—".to_owned(),
                        };
                        Some(GoalieRow {
                            nhl_id: v.id().0,
                            name: v.full_name().to_owned(),
                            team: v.team_display().to_owned(),
                            gp: v.gp(),
                            wins: g.wins,
                            losses: g.losses,
                            shutouts: g.shutouts,
                            save_pct_str,
                            gaa_str,
                        })
                    })
                    .collect();
                let total = all.len();

                all.sort_by(|a, b| {
                    let primary = match sort {
                        GoalieSort::SavePct => {
                            let ap = a.save_pct_str.parse::<f64>().unwrap_or(0.0);
                            let bp = b.save_pct_str.parse::<f64>().unwrap_or(0.0);
                            bp.partial_cmp(&ap).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        GoalieSort::Wins => b.wins.cmp(&a.wins),
                        GoalieSort::Losses => b.losses.cmp(&a.losses),
                        GoalieSort::Games => b.gp.cmp(&a.gp),
                        GoalieSort::Shutouts => b.shutouts.cmp(&a.shutouts),
                        GoalieSort::GaaAsc => {
                            // Lower GAA is better; sort ascending.
                            // Treat "—" as worst.
                            let av = a.gaa_str.parse::<f64>().unwrap_or(f64::INFINITY);
                            let bv = b.gaa_str.parse::<f64>().unwrap_or(f64::INFINITY);
                            av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
                        }
                    };
                    primary.then(b.wins.cmp(&a.wins)).then(a.name.cmp(&b.name))
                });
                all.truncate(top_n);
                (all, total)
            };

            Ok(GoalieResult {
                rows,
                total,
                sort,
                qualified_threshold,
                include_below_threshold,
                active_label,
                active_season: season_str,
                active_season_type: season_type,
                top_n,
            })
        }

        pub async fn get_goalies(
            State(state): State<WebState>,
            Query(q): Query<GoaliesQuery>,
        ) -> Response {
            let r = match build_goalie_result(&state, &q).await {
                Ok(v) => v,
                Err(resp) => return resp,
            };
            let _ = r.include_below_threshold;
            let tmpl = GoaliesTemplate {
                active_label: r.active_label,
                rows: r.rows,
                total: r.total,
                qualified_threshold: r.qualified_threshold,
            };
            match tmpl.render() {
                Ok(html) => Html(html).into_response(),
                Err(e) => error_500(format!("template render failed: {e}")),
            }
        }

        // ── King.5.2 — JSON envelope ─────────────────────────────────

        #[derive(Debug, serde::Serialize)]
        pub struct GoaliesEnvelope {
            pub schema_version: u32,
            pub route: &'static str,
            pub data: Vec<GoalieJsonRow>,
            pub meta: GoaliesMeta,
        }

        #[derive(Debug, serde::Serialize)]
        pub struct GoalieJsonRow {
            pub nhl_id: u32,
            pub name: String,
            pub team: String,
            pub games: u32,
            pub wins: u32,
            pub losses: u32,
            pub shutouts: u32,
            pub save_pct: Option<f64>,
            pub goals_against_average: Option<f64>,
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
                    shutouts: row.shutouts,
                    save_pct: row.save_pct_str.parse().ok(),
                    goals_against_average: row.gaa_str.parse().ok(),
                })
                .collect();

            let envelope = GoaliesEnvelope {
                schema_version: 1,
                route: "goalies",
                data,
                meta: GoaliesMeta {
                    season: r.active_season,
                    season_type: match r.active_season_type {
                        SeasonType::Regular => "regular".to_owned(),
                        SeasonType::Playoff => "playoff".to_owned(),
                    },
                    sort: match r.sort {
                        GoalieSort::SavePct => "save_pct".to_owned(),
                        GoalieSort::Wins => "wins".to_owned(),
                        GoalieSort::Losses => "losses".to_owned(),
                        GoalieSort::Games => "gp".to_owned(),
                        GoalieSort::Shutouts => "shutouts".to_owned(),
                        GoalieSort::GaaAsc => "gaa".to_owned(),
                    },
                    qualified_gp_min: r.qualified_threshold,
                    include_below_threshold: r.include_below_threshold,
                    total: r.total,
                    returned,
                    top: r.top_n,
                },
            };
            let _ = r.active_label;
            axum::Json(envelope).into_response()
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
    }

    /// `/player/:id` — King.3.1 + King.3.2. Player card + career table.
    pub mod player {
        use crate::state::WebState;
        use crate::templates::{CareerRow, PlayerTemplate};
        use askama::Template;
        use axum::extract::{Path, State};
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};
        use icelines_core::identity::PlayerId;
        use icelines_core::model::Season;
        use icelines_core::season_stats::SeasonType;

        /// Format a YYYYZZZZ season as "YYYY-YY" (e.g. 20242025 → "2024-25").
        fn pretty_season(s: Season) -> String {
            let raw = s.0;
            if raw < 10_000_000 {
                return raw.to_string();
            }
            let yyyy_start = raw / 10_000;
            let yy_end = raw % 100;
            format!("{:04}-{:02}", yyyy_start, yy_end)
        }

        pub async fn get_player(State(state): State<WebState>, Path(id): Path<u32>) -> Response {
            let (season_str, season_type, active_label) = {
                let cfg = state.config.read().await;
                let st = match cfg.active_season_type.as_str() {
                    "playoff" | "playoffs" => SeasonType::Playoff,
                    _ => SeasonType::Regular,
                };
                (cfg.active_season.clone(), st, cfg.active_label.clone())
            };
            let season_u32: u32 = match season_str.parse() {
                Ok(n) => n,
                Err(_) => {
                    return not_found_page(format!(
                        "Season '{season_str}' is not a valid YYYYZZZZ id"
                    ));
                }
            };
            let season = Season(season_u32);
            let pid = PlayerId(id);

            // King.3.2 — lazy career fan-out (UX.1 pattern). Brief
            // write lock loads all 38 bundled seasons for this pid
            // into the repo. Idempotent — re-opening the same player
            // is a ~5ms no-op aside from the bundle scans.
            // Per spec: subsequent reads are concurrent (RwLock).
            {
                let mut repo = state.repo.write().await;
                if let Err(e) =
                    icelines_fetch::stats_loader::load_player_career_into_repo(&mut repo, pid)
                {
                    eprintln!(
                        "warn: career fan-out for pid={id} failed: {e} — \
                         player card will show only seasons already loaded"
                    );
                }
            }

            let projection = {
                let repo = state.repo.read().await;
                let identity = match repo.identity(pid) {
                    Some(i) => i,
                    None => {
                        return not_found_page(format!(
                            "No player with NHL id {id} in the active repository. \
                             They may not have a row in the {season_str} season — \
                             try editing `~/.icelines/config.toml` to switch seasons."
                        ));
                    }
                };
                // Try the active season's view; fall back to None if
                // the player has no row that season (e.g. injured all
                // year, traded mid-season, retired).
                let view = repo.view(pid, season, season_type);
                let (gp, goals, assists, points, position, team) = match view {
                    Some(v) => (
                        v.gp(),
                        v.goals(),
                        v.assists(),
                        v.points(),
                        v.position().abbreviation().to_owned(),
                        v.team_display().to_owned(),
                    ),
                    None => (0, 0, 0, 0, "—".to_owned(), "—".to_owned()),
                };
                let ppg_str = if gp > 0 {
                    format!("{:.2}", points as f64 / gp as f64)
                } else {
                    String::new()
                };

                // King.3.2 — collect every (season, type) row this
                // player has stats for. Newest first. Skips empty
                // (gp=0) rows so a player who was rostered but never
                // played a regular-season game in a given (year,type)
                // doesn't add noise.
                let mut career_rows: Vec<CareerRow> = match repo.career_all(pid) {
                    Some(iter) => iter
                        .filter_map(|s| {
                            let totals = &s.totals;
                            if totals.gp == 0 {
                                return None;
                            }
                            let last_team = s
                                .team_stints
                                .last()
                                .map(|st| st.team.0.as_str().to_owned())
                                .unwrap_or_else(|| "—".to_owned());
                            let ppg_str = if totals.gp > 0 {
                                format!("{:.2}", totals.points as f64 / totals.gp as f64)
                            } else {
                                String::new()
                            };
                            Some(CareerRow {
                                season: pretty_season(s.season),
                                season_type: match s.season_type {
                                    SeasonType::Regular => "Regular".to_owned(),
                                    SeasonType::Playoff => "Playoff".to_owned(),
                                },
                                team: last_team,
                                gp: totals.gp,
                                goals: totals.goals,
                                assists: totals.assists,
                                points: totals.points,
                                ppg_str,
                            })
                        })
                        .collect(),
                    None => Vec::new(),
                };
                // Newest season first; within a season, regular before playoff.
                career_rows.sort_by(|a, b| {
                    b.season
                        .cmp(&a.season)
                        .then(a.season_type.cmp(&b.season_type))
                });

                PlayerTemplate {
                    active_label: active_label.clone(),
                    nhl_id: id,
                    full_name: identity.full_name.clone(),
                    position,
                    team,
                    headshot_url: identity.headshot_canonical_url.clone(),
                    gp,
                    goals,
                    assists,
                    points,
                    ppg_str,
                    career_rows,
                }
            };

            match projection.render() {
                Ok(html) => Html(html).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("template render failed: {e}")),
                )
                    .into_response(),
            }
        }

        fn not_found_page(msg: String) -> Response {
            (
                StatusCode::NOT_FOUND,
                Html(format!(
                    "<!doctype html><html><body>\
                     <h1>Player not found</h1>\
                     <p>{msg}</p>\
                     <p><a href=\"/leaders\">← back to leaders</a></p>\
                     </body></html>"
                )),
            )
                .into_response()
        }
    }

    pub mod home {
        use crate::state::WebState;
        use crate::templates::HomeTemplate;
        use askama::Template;
        use axum::extract::State;
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};

        /// `GET /` — askama-rendered home. Reads the active-season
        /// label from `WebState.config` (RwLock'd, brief read).
        /// Render failure is treated as `Internal` (template bugs are
        /// programmer errors; users see the standard error page).
        pub async fn get_home(State(state): State<WebState>) -> Response {
            let active_label = {
                // Brief read of the config RwLock. The clone is one
                // string allocation per request — cheap relative to
                // template render. Holding the guard across `.render()`
                // is also fine (askama is sync) but the explicit
                // clone-then-drop pattern matches the spec's "no lock
                // held across .await" rule for handlers that do reach
                // for an async dependency.
                let cfg = state.config.read().await;
                cfg.active_label.clone()
            };

            let tmpl = HomeTemplate { active_label };
            match tmpl.render() {
                Ok(html) => Html(html).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!(
                        "<!doctype html><html><body><h1>500</h1>\
                         <p>template render failed: {e}</p></body></html>"
                    )),
                )
                    .into_response(),
            }
        }
    }
}
