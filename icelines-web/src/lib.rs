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
        // JSON twin — King.3.3.
        .route("/api/v1/player/:id", get(handlers::player::get_player_json))
        // Compare — UX.D. Side-by-side stats for two players.
        .route("/compare", get(handlers::compare::get_compare))
        // Goalie leaderboard — King.5.1 / .5.2.
        .route("/goalies", get(handlers::goalies::get_goalies))
        .route("/api/v1/goalies", get(handlers::goalies::get_goalies_json))
        // Team roster — King.4.1. /team/SEA, /team/EDM, etc.
        .route("/team/:abbrev", get(handlers::team::get_team))
        // JSON twin — King.4.2.
        .route("/api/v1/team/:abbrev", get(handlers::team::get_team_json))
        // Depth rankings — Phase Lady Byng follow-up. Cross-team
        // line-value rankings; mirror of TUI Depth tab.
        .route("/depth", get(handlers::depth::get_depth))
        // Docs — King.8.1. Rendered COMMANDS.md.
        .route("/docs", get(handlers::docs::get_docs))
        // Season-type flip — UX.E. Click writes WebState.config and
        // redirects back to the page the user came from (Referer).
        .route(
            "/season-type/:kind",
            get(handlers::season_type::set_season_type),
        )
        // Live NHL data — King.7.
        .route("/scores", get(handlers::scores::get_scores))
        .route("/schedule", get(handlers::schedule::get_schedule))
        .route("/playoffs", get(handlers::playoffs::get_playoffs))
        // Transactions feed — King.8.2.
        .route(
            "/transactions",
            get(handlers::transactions::get_transactions),
        )
        .route("/fantasy", get(cs::fantasy))
        // Sasq.7 — friendly 404 with a player-search input replaces
        // axum's bare default. Wired as router fallback so any
        // unmatched path lands here.
        .fallback(handlers::not_found::get_not_found)
        .with_state(state)
}

mod handlers {
    // QueryA — bio filter primitives moved to the icelines-query
    // crate so the CLI can share them. The web handlers reach in via
    // `super::extract_bio` / `super::BioConstraints`; the CLI's
    // `query --filter` will share these in a follow-up wiring.
    pub(crate) use icelines_query::{extract_bio, BioConstraints};

    /// Build the NHL CDN headshot URL for a player. UX.G2 — the
    /// `mugs/nhl/default/{id}.png` path serves silhouettes for many
    /// players; the seasonal `mugs/nhl/{season}/{team}/{id}.png` path
    /// serves real mug shots for current rosters. For multi-team rows
    /// pick the primary (first) team. For empty/sentinel teams fall
    /// through to `default` since we have nothing better.
    pub(crate) fn build_headshot_url(season: u32, team: &str, nhl_id: u32) -> String {
        let team = team.trim();
        // Validate against the NHL team-abbrev shape: 2-3 uppercase
        // ASCII letters only. Anything else (sentinels like "RET" or
        // "—", multi-team rows like "SEA/NYR", or lowercase/numeric
        // garbage from a malformed bundle) hits the silhouette
        // fallback rather than building a 404-prone URL.
        let valid_shape =
            (2..=3).contains(&team.len()) && team.chars().all(|c| c.is_ascii_uppercase());
        let valid_team = valid_shape && team != "RET";
        if !valid_team {
            return format!("https://assets.nhle.com/mugs/nhl/default/{nhl_id}.png");
        }
        format!("https://assets.nhle.com/mugs/nhl/{season}/{team}/{nhl_id}.png")
    }

    /// Same as `build_headshot_url` but takes a multi-team display
    /// string ("EDM" or "EDM/CGY") and uses the primary (first) team.
    pub(crate) fn build_headshot_url_for_display(
        season: u32,
        team_display: &str,
        nhl_id: u32,
    ) -> String {
        let primary = team_display.split('/').next().unwrap_or("").trim();
        build_headshot_url(season, primary, nhl_id)
    }

    #[cfg(test)]
    mod headshot_url_tests {
        use super::*;

        /// l0_headshot_seasonal_team_path
        /// — single-team rows must hit the seasonal CDN path that
        ///   serves real mug shots, not the silhouette fallback.
        #[test]
        fn l0_headshot_seasonal_team_path() {
            let url = build_headshot_url(20252026, "EDM", 8478402);
            assert_eq!(
                url,
                "https://assets.nhle.com/mugs/nhl/20252026/EDM/8478402.png"
            );
        }

        /// l0_headshot_falls_back_to_default_for_sentinel_team
        /// — RET / "—" / empty string land on the default silhouette.
        ///   Anything else where we'd otherwise build a broken URL
        ///   (numeric chars, lowercase, slashes) also falls back.
        #[test]
        fn l0_headshot_falls_back_to_default_for_sentinel_team() {
            for sentinel in ["", "—", "RET", "EDM/CGY", "edm", "abc123"] {
                let url = build_headshot_url(20252026, sentinel, 1);
                assert_eq!(
                    url, "https://assets.nhle.com/mugs/nhl/default/1.png",
                    "sentinel team {sentinel:?} should fall back to default"
                );
            }
        }

        /// l0_headshot_for_display_picks_primary_team_in_trade
        /// — a "SEA/NYR" mid-season trade row should key the URL by
        ///   the primary (first) team listed.
        #[test]
        fn l0_headshot_for_display_picks_primary_team_in_trade() {
            let url = build_headshot_url_for_display(20252026, "SEA/NYR", 8481789);
            assert_eq!(
                url,
                "https://assets.nhle.com/mugs/nhl/20252026/SEA/8481789.png"
            );
        }

        /// l0_headshot_for_display_passthrough_for_single_team
        /// — single-team display strings build the same URL as the
        ///   bare-abbrev variant.
        #[test]
        fn l0_headshot_for_display_passthrough_for_single_team() {
            assert_eq!(
                build_headshot_url_for_display(20252026, "EDM", 8478402),
                build_headshot_url(20252026, "EDM", 8478402),
            );
        }
    }

    /// Project a `PlayerView` into the `LeaderRow` shape that the
    /// /leaders, /team, and home-preview templates all consume. Centralized
    /// so the new realtime + special-teams columns (UX.C) get the same
    /// formatting everywhere.
    fn project_leader_row(
        v: &icelines_core::stats_repository::PlayerView,
    ) -> crate::templates::LeaderRow {
        project_leader_row_with_prior(v, None)
    }

    /// Same as `project_leader_row` but takes an optional
    /// `prior_points` (the player's points from the prior season,
    /// same season-type) so the row can carry a YoY delta. Used by
    /// /leaders for breakout/decline sorting; the team page and home
    /// preview pass None.
    fn project_leader_row_with_prior(
        v: &icelines_core::stats_repository::PlayerView,
        prior_points: Option<u32>,
    ) -> crate::templates::LeaderRow {
        let gp = v.gp();
        let points = v.points();
        let ppg_str = if gp > 0 {
            format!("{:.2}", points as f64 / gp as f64)
        } else {
            String::new()
        };
        let totals = &v.stats.totals;
        let plus_minus = v.plus_minus();
        let hits = v.hits();
        let blocks = v.blocked_shots();
        let shooting_pct = totals.shooting_pct;
        let faceoff_pct = totals.faceoff_win_pct;
        let opt_u = |o: Option<u32>| -> String {
            match o {
                Some(n) => n.to_string(),
                None => "—".to_owned(),
            }
        };
        let opt_pct = |o: Option<f32>| -> String {
            match o {
                Some(p) => {
                    if p.abs() <= 1.5 {
                        format!("{:.1}%", p * 100.0)
                    } else {
                        format!("{:.1}%", p)
                    }
                }
                None => "—".to_owned(),
            }
        };
        let team_display = v.team_display().to_owned();
        let primary_team = team_display
            .split('/')
            .next()
            .unwrap_or("")
            .trim()
            .to_owned();
        let headshot_url = build_headshot_url(v.season().0, &primary_team, v.id().0);
        let headshot_fallback_url =
            format!("https://assets.nhle.com/mugs/nhl/default/{}.png", v.id().0);

        // Sasq.5 — per-60 rates: stat × 3600 / total_toi_seconds.
        // total_toi_seconds = toi_per_game (sec) × gp. None when toi
        // data is missing so we don't emit "0.00" for half the league.
        let total_toi_secs: Option<u64> = totals
            .toi_per_game_sec
            .map(|tpg| u64::from(tpg) * u64::from(gp))
            .filter(|s| *s > 0);
        let per_60 =
            |stat: f64| -> Option<f64> { total_toi_secs.map(|toi| stat * 3600.0 / toi as f64) };
        let opt_p60 = |o: Option<f64>| -> String {
            match o {
                Some(v) => format!("{:.2}", v),
                None => "—".to_owned(),
            }
        };
        let goals_per_60 = per_60(v.goals() as f64);
        let assists_per_60 = per_60(v.assists() as f64);
        let points_per_60 = per_60(v.points() as f64);
        let hits_per_60 = hits.and_then(|h| per_60(h as f64));
        let blocks_per_60 = blocks.and_then(|b| per_60(b as f64));

        // Sasq.4 — point delta vs prior season. None when no prior
        // row exists. Pre-format the chip string + class so the
        // template doesn't have to branch.
        let points_delta = prior_points.map(|prev| v.points() as i32 - prev as i32);
        let (points_delta_str, points_delta_class) = match points_delta {
            Some(d) if d > 0 => (format!("{:+}", d), "delta-up".to_owned()),
            Some(d) if d < 0 => (format!("{:+}", d), "delta-down".to_owned()),
            Some(_) => ("0".to_owned(), "delta-flat".to_owned()),
            None => (String::new(), String::new()),
        };

        crate::templates::LeaderRow {
            nhl_id: v.id().0,
            name: v.full_name().to_owned(),
            position: v.position().abbreviation().to_owned(),
            team: team_display,
            gp,
            goals: v.goals(),
            assists: v.assists(),
            points,
            ppg_str,
            plus_minus_str: format!("{:+}", plus_minus),
            pim: totals.pim,
            shots: totals.shots,
            shooting_pct_str: opt_pct(shooting_pct),
            hits_str: opt_u(hits),
            blocks_str: opt_u(blocks),
            faceoff_pct_str: opt_pct(faceoff_pct),
            pp_points: totals.pp_points,
            plus_minus,
            shooting_pct,
            hits,
            blocks,
            faceoff_pct,
            headshot_url,
            headshot_fallback_url,
            points_per_60_str: opt_p60(points_per_60),
            goals_per_60_str: opt_p60(goals_per_60),
            assists_per_60_str: opt_p60(assists_per_60),
            hits_per_60_str: opt_p60(hits_per_60),
            blocks_per_60_str: opt_p60(blocks_per_60),
            points_per_60,
            goals_per_60,
            assists_per_60,
            hits_per_60,
            blocks_per_60,
            points_delta,
            points_delta_str,
            points_delta_class,
        }
    }

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

        // `scores`, `playoffs`, `transactions` stubs removed —
        // real handlers at `handlers::scores`, `handlers::playoffs`,
        // `handlers::transactions` (King.7.1, King.7.2, King.8.2).

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
                        let prev = prior_points.get(&v.id().0).copied();
                        super::project_leader_row_with_prior(&v, prev)
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

            // QueryA — pre-extract bio atoms from each filter's
            // top-level AND chain via the shared icelines-query crate.
            // Stat residue (stat-only pieces) is recombined for the
            // catalog parser. Filters containing OR/NOT bypass the
            // splitter and pass whole-cloth to the stat parser; users
            // who need OR with bio terms can fall back to the discrete
            // Bio Filters accordion.
            let (extracted_bio, stat_filters) = super::extract_bio(&raw_filters);

            let filter_expr_result = combine_filters(&stat_filters);
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
                    // QueryA — bio filters via shared icelines-query
                    // BioConstraints. No-op when nothing is set; when
                    // a constraint is set, players missing the bio
                    // field (e.g. no birth_date for an age filter) are
                    // excluded.
                    .filter(|v| bio.matches(v, season.0))
                    .map(|v| super::project_leader_row(&v))
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

    /// `/depth` — Phase Lady Byng follow-up. Cross-team line-value
    /// rankings (same data the TUI Depth tab consumes). Mirrors the
    /// goalies handler shape: load the active-season repo, compute
    /// `compute_team_strength_views`, project to template rows, render.
    pub mod depth {
        use crate::state::WebState;
        use crate::templates::{DepthRow, DepthTemplate};
        use askama::Template;
        use axum::extract::State;
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};
        use icelines_core::cross_team::{compute_team_strength_views, ScoringMode};
        use icelines_core::model::Season;

        pub async fn get_depth(State(state): State<WebState>) -> Response {
            let (season_str, season_type, active_label) = {
                let cfg = state.config.read().await;
                (
                    cfg.active_season.clone(),
                    super::leaders::parse_season_type(&cfg.active_season_type),
                    cfg.active_label.clone(),
                )
            };
            let season_u32: u32 = match season_str.parse() {
                Ok(n) => n,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Html(format!(
                            "<!doctype html><body><h1>500</h1><p>active season \
                             '{season_str}' is not a valid YYYYZZZZ id: {e}</p></body>"
                        )),
                    )
                        .into_response();
                }
            };
            let season = Season(season_u32);

            // Brief read of the repo. Project inside the lock scope so
            // PlayerView refs don't escape (same convention as
            // `/leaders` and `/goalies`).
            let rows: Vec<DepthRow> = {
                let repo = state.repo.read().await;
                let views: Vec<_> = repo.skaters(season, season_type).collect();
                let strength = compute_team_strength_views(&views, ScoringMode::Pace);
                let mut ranked: Vec<_> = strength.into_iter().collect();
                // Newest team rank first; tie-break alphabetical for
                // determinism.
                ranked.sort_by(|a, b| {
                    b.1.total
                        .partial_cmp(&a.1.total)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.0.cmp(&b.0))
                });
                ranked
                    .into_iter()
                    .map(|(team, s)| DepthRow {
                        team,
                        c_score: format!("{:.0}", s.c_score),
                        lw_score: format!("{:.0}", s.lw_score),
                        rw_score: format!("{:.0}", s.rw_score),
                        d_score: format!("{:.0}", s.d_score),
                        total: format!("{:.0}", s.total),
                        c_top: s.c_top,
                        lw_top: s.lw_top,
                        rw_top: s.rw_top,
                        d_top: s.d_top,
                    })
                    .collect()
            };

            let tmpl = DepthTemplate { active_label, rows };
            match tmpl.render() {
                Ok(html) => Html(html).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!(
                        "<!doctype html><body><h1>500</h1><p>{e}</p></body>"
                    )),
                )
                    .into_response(),
            }
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
                    .map(|v| super::project_leader_row(v))
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
                        let team_display = v.team_display().to_owned();
                        let headshot_url = super::build_headshot_url_for_display(
                            v.season().0,
                            &team_display,
                            v.id().0,
                        );
                        Some(GoalieRow {
                            nhl_id: v.id().0,
                            name: v.full_name().to_owned(),
                            team: team_display,
                            gp: v.gp(),
                            wins: g.wins,
                            losses: g.losses,
                            shutouts: g.shutouts,
                            save_pct_str,
                            gaa_str,
                            headshot_url,
                            headshot_fallback_url: format!(
                                "https://assets.nhle.com/mugs/nhl/default/{}.png",
                                v.id().0
                            ),
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

        // ── King.4.2 — JSON twin ──────────────────────────────────────

        #[derive(Debug, serde::Serialize)]
        pub struct TeamEnvelope {
            pub schema_version: u32,
            pub route: &'static str,
            pub data: TeamData,
            pub meta: TeamMeta,
        }

        #[derive(Debug, serde::Serialize)]
        pub struct TeamData {
            pub team_abbrev: String,
            pub skaters: Vec<TeamSkaterRow>,
            pub goalies: Vec<TeamGoalieRow>,
        }

        #[derive(Debug, serde::Serialize)]
        pub struct TeamSkaterRow {
            pub nhl_id: u32,
            pub name: String,
            pub position: String,
            pub games: u32,
            pub goals: u32,
            pub assists: u32,
            pub points: u32,
            pub points_per_game: Option<f64>,
        }

        #[derive(Debug, serde::Serialize)]
        pub struct TeamGoalieRow {
            pub nhl_id: u32,
            pub name: String,
            pub games: u32,
            pub wins: u32,
            pub losses: u32,
            pub shutouts: u32,
            pub save_pct: Option<f64>,
            pub goals_against_average: Option<f64>,
        }

        #[derive(Debug, serde::Serialize)]
        pub struct TeamMeta {
            pub team_abbrev: String,
            pub season: String,
            pub season_type: String,
            pub skater_count: usize,
            pub goalie_count: usize,
        }

        /// `GET /api/v1/team/:abbrev` — JSON twin of `/team/:abbrev`.
        pub async fn get_team_json(
            State(state): State<WebState>,
            Path(abbrev_raw): Path<String>,
        ) -> Response {
            let abbrev_upper = abbrev_raw.to_ascii_uppercase();
            let team = match TeamAbbr::parse(&abbrev_upper) {
                Ok(t) => t,
                Err(e) => {
                    return (
                        StatusCode::NOT_FOUND,
                        axum::Json(serde_json::json!({
                            "error": "unknown_team",
                            "message": format!(
                                "'{abbrev_upper}' is not a recognized NHL team abbrev: {e}"
                            ),
                            "team_abbrev": abbrev_upper,
                        })),
                    )
                        .into_response();
                }
            };

            let (season_str, season_type) = {
                let cfg = state.config.read().await;
                let st = match cfg.active_season_type.as_str() {
                    "playoff" | "playoffs" => SeasonType::Playoff,
                    _ => SeasonType::Regular,
                };
                (cfg.active_season.clone(), st)
            };
            let season_u32: u32 = match season_str.parse() {
                Ok(n) => n,
                Err(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        axum::Json(serde_json::json!({
                            "error": "bad_active_season",
                            "message": format!("Season '{season_str}' is not a valid YYYYZZZZ id"),
                        })),
                    )
                        .into_response();
                }
            };
            let season = Season(season_u32);

            let (skaters, goalies) = {
                let repo = state.repo.read().await;
                let roster = repo.team_roster(&team, season, season_type);

                let mut skaters: Vec<TeamSkaterRow> = roster
                    .iter()
                    .filter(|v| !v.is_goalie())
                    .map(|v| {
                        let gp = v.gp();
                        let points = v.points();
                        let ppg = if gp > 0 {
                            Some((points as f64) / (gp as f64))
                        } else {
                            None
                        };
                        TeamSkaterRow {
                            nhl_id: v.id().0,
                            name: v.full_name().to_owned(),
                            position: v.position().abbreviation().to_owned(),
                            games: gp,
                            goals: v.goals(),
                            assists: v.assists(),
                            points,
                            points_per_game: ppg,
                        }
                    })
                    .collect();
                skaters.sort_by(|a, b| {
                    b.points
                        .cmp(&a.points)
                        .then(b.goals.cmp(&a.goals))
                        .then(a.name.cmp(&b.name))
                });

                let mut goalies: Vec<TeamGoalieRow> = roster
                    .iter()
                    .filter(|v| v.is_goalie())
                    .filter_map(|v| {
                        let g = v.stats.goalie.as_ref()?;
                        Some(TeamGoalieRow {
                            nhl_id: v.id().0,
                            name: v.full_name().to_owned(),
                            games: v.gp(),
                            wins: g.wins,
                            losses: g.losses,
                            shutouts: g.shutouts,
                            save_pct: g.save_pct.map(f64::from),
                            goals_against_average: g.goals_against_average.map(f64::from),
                        })
                    })
                    .collect();
                goalies.sort_by(|a, b| b.wins.cmp(&a.wins).then(a.name.cmp(&b.name)));

                (skaters, goalies)
            };

            let envelope = TeamEnvelope {
                schema_version: 1,
                route: "team",
                meta: TeamMeta {
                    team_abbrev: team.0.to_string(),
                    season: season_str,
                    season_type: match season_type {
                        SeasonType::Regular => "regular".to_owned(),
                        SeasonType::Playoff => "playoff".to_owned(),
                    },
                    skater_count: skaters.len(),
                    goalie_count: goalies.len(),
                },
                data: TeamData {
                    team_abbrev: team.0.to_string(),
                    skaters,
                    goalies,
                },
            };
            axum::Json(envelope).into_response()
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
                        let team_display = v.team_display().to_owned();
                        let headshot_url = super::build_headshot_url_for_display(
                            v.season().0,
                            &team_display,
                            v.id().0,
                        );
                        Some(GoalieRow {
                            nhl_id: v.id().0,
                            name: v.full_name().to_owned(),
                            team: team_display,
                            gp: v.gp(),
                            wins: g.wins,
                            losses: g.losses,
                            shutouts: g.shutouts,
                            save_pct_str,
                            gaa_str,
                            headshot_url,
                            headshot_fallback_url: format!(
                                "https://assets.nhle.com/mugs/nhl/default/{}.png",
                                v.id().0
                            ),
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

                // UX.B — pull the expanded stat slice. Each
                // pre-formatted to a String so the template renders
                // without inline casts and Option<> shows "—".
                let opt_u = |o: Option<u32>| -> String {
                    match o {
                        Some(n) => n.to_string(),
                        None => "—".to_owned(),
                    }
                };
                let opt_pct = |o: Option<f32>| -> String {
                    match o {
                        Some(p) => {
                            // NHL APIs report shooting/faceoff% as
                            // 0.105 (10.5%) — surface as percentage
                            // with one decimal so users see "10.5".
                            if p.abs() <= 1.5 {
                                format!("{:.1}%", p * 100.0)
                            } else {
                                format!("{:.1}%", p)
                            }
                        }
                        None => "—".to_owned(),
                    }
                };
                let toi_mmss = |o: Option<u32>| -> String {
                    match o {
                        Some(secs) => {
                            let m = secs / 60;
                            let s = secs % 60;
                            format!("{m}:{s:02}")
                        }
                        None => "—".to_owned(),
                    }
                };

                let (
                    gp,
                    goals,
                    assists,
                    points,
                    position,
                    team,
                    team_link,
                    plus_minus_str,
                    pim_str,
                    shots_str,
                    shooting_pct_str,
                    hits_str,
                    blocks_str,
                    takeaways_str,
                    giveaways_str,
                    faceoff_pct_str,
                    pp_goals_str,
                    pp_points_str,
                    sh_goals_str,
                    gwg_str,
                    toi_per_game_str,
                ) = match view {
                    Some(v) => {
                        let totals = &v.stats.totals;
                        let team_display = v.team_display().to_owned();
                        // Only build a /team/ link when the display is
                        // a single uppercase abbrev (skip the "TBL/CGY"
                        // mid-season-trade format).
                        let team_link = if team_display.chars().all(|c| c.is_ascii_alphabetic())
                            && team_display.len() <= 3
                        {
                            team_display.clone()
                        } else {
                            String::new()
                        };
                        (
                            v.gp(),
                            v.goals(),
                            v.assists(),
                            v.points(),
                            v.position().abbreviation().to_owned(),
                            team_display,
                            team_link,
                            format!("{:+}", v.plus_minus()),
                            totals.pim.to_string(),
                            totals.shots.to_string(),
                            opt_pct(totals.shooting_pct),
                            opt_u(v.hits()),
                            opt_u(v.blocked_shots()),
                            opt_u(v.takeaways()),
                            opt_u(v.giveaways()),
                            opt_pct(totals.faceoff_win_pct),
                            totals.pp_goals.to_string(),
                            totals.pp_points.to_string(),
                            totals.sh_goals.to_string(),
                            totals.gwg.to_string(),
                            toi_mmss(totals.toi_per_game_sec),
                        )
                    }
                    None => (
                        0,
                        0,
                        0,
                        0,
                        "—".to_owned(),
                        "—".to_owned(),
                        String::new(),
                        "—".to_owned(),
                        "—".to_owned(),
                        "—".to_owned(),
                        "—".to_owned(),
                        "—".to_owned(),
                        "—".to_owned(),
                        "—".to_owned(),
                        "—".to_owned(),
                        "—".to_owned(),
                        "—".to_owned(),
                        "—".to_owned(),
                        "—".to_owned(),
                        "—".to_owned(),
                        "—".to_owned(),
                    ),
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
                //
                // UX.G — filter to the active season_type so the
                // career table matches what the global toggle says.
                // Mixing Regular + Playoff rows under a "Regular"
                // toggle was confusing.
                let mut career_rows: Vec<CareerRow> = match repo.career_all(pid) {
                    Some(iter) => iter
                        .filter(|s| s.season_type == season_type)
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
                            // Link only when the team is a single
                            // 2-3 char alpha abbrev — multi-team
                            // values like "SEA/NYR" or sentinels
                            // like "—"/"RET" don't get a /team/ URL.
                            let team_link = if last_team.chars().all(|c| c.is_ascii_alphabetic())
                                && (2..=3).contains(&last_team.len())
                            {
                                last_team.clone()
                            } else {
                                String::new()
                            };
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
                                team_link,
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

                // Sasq.3 — compute YoY delta against the prior season
                // of the SAME season-type (Regular vs Playoff).
                // career_rows is already filtered to active type and
                // sorted newest-first, so the prior season's row is
                // index 1 (index 0 is the active season we're showing).
                let prior_row = career_rows.get(1);
                let prior_season_label = prior_row
                    .map(|r| format!("vs {}", r.season))
                    .unwrap_or_default();

                fn delta_int(now: i64, prior: i64, prior_exists: bool) -> (String, String) {
                    if !prior_exists {
                        return (String::new(), String::new());
                    }
                    let d = now - prior;
                    let class = if d > 0 {
                        "delta-up"
                    } else if d < 0 {
                        "delta-down"
                    } else {
                        "delta-flat"
                    };
                    (format!("{:+}", d), class.to_owned())
                }

                let prior_exists = prior_row.is_some();
                let prior_gp = prior_row.map(|r| r.gp as i64).unwrap_or(0);
                let prior_goals = prior_row.map(|r| r.goals as i64).unwrap_or(0);
                let prior_assists = prior_row.map(|r| r.assists as i64).unwrap_or(0);
                let prior_points = prior_row.map(|r| r.points as i64).unwrap_or(0);
                let (gp_delta, gp_delta_class) = delta_int(gp as i64, prior_gp, prior_exists);
                let (goals_delta, goals_delta_class) =
                    delta_int(goals as i64, prior_goals, prior_exists);
                let (assists_delta, assists_delta_class) =
                    delta_int(assists as i64, prior_assists, prior_exists);
                let (points_delta, points_delta_class) =
                    delta_int(points as i64, prior_points, prior_exists);

                PlayerTemplate {
                    active_label: active_label.clone(),
                    nhl_id: id,
                    full_name: identity.full_name.clone(),
                    position,
                    team,
                    team_link: team_link.clone(),
                    // Prefer the seasonal team-keyed CDN path (real
                    // mug shot for current rosters); fall back to the
                    // legacy `default/{id}.png` (silhouette for many
                    // players) only when we don't have a team to key
                    // by.
                    headshot_url: if !team_link.is_empty() {
                        Some(super::build_headshot_url(season.0, &team_link, id))
                    } else {
                        identity.headshot_canonical_url.clone()
                    },
                    gp,
                    goals,
                    assists,
                    points,
                    ppg_str,
                    plus_minus_str,
                    pim_str,
                    shots_str,
                    shooting_pct_str,
                    hits_str,
                    blocks_str,
                    takeaways_str,
                    giveaways_str,
                    faceoff_pct_str,
                    pp_goals_str,
                    pp_points_str,
                    sh_goals_str,
                    gwg_str,
                    toi_per_game_str,
                    goals_delta,
                    goals_delta_class,
                    assists_delta,
                    assists_delta_class,
                    points_delta,
                    points_delta_class,
                    gp_delta,
                    gp_delta_class,
                    prior_season_label,
                    career_rows,
                    // UX.H — every active player + goalie name in
                    // the repo, sorted alphabetically. Renders as a
                    // <datalist> on the page so the Compare-with
                    // input gets native browser autocomplete with
                    // zero JS. Skips the player you're already
                    // viewing — comparing someone with themselves is
                    // never useful.
                    compare_suggestions: {
                        let mut pairs: Vec<(String, u32)> = repo
                            .iter_identities()
                            .filter(|i| i.id.0 != pid.0)
                            .map(|i| (i.full_name.clone(), i.id.0))
                            .collect();
                        pairs.sort_by(|a, b| a.0.cmp(&b.0));
                        pairs
                    },
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

        // ── King.3.3 — JSON twin ──────────────────────────────────────

        #[derive(Debug, serde::Serialize)]
        pub struct PlayerEnvelope {
            pub schema_version: u32,
            pub route: &'static str,
            pub data: PlayerData,
            pub meta: PlayerMeta,
        }

        #[derive(Debug, serde::Serialize)]
        pub struct PlayerData {
            pub nhl_id: u32,
            pub full_name: String,
            pub position: String,
            pub team: String,
            pub headshot_url: Option<String>,
            pub active_season_stats: PlayerActiveStats,
            pub career: Vec<PlayerCareerRow>,
        }

        #[derive(Debug, serde::Serialize)]
        pub struct PlayerActiveStats {
            pub season: String,
            pub season_type: String,
            pub games: u32,
            pub goals: u32,
            pub assists: u32,
            pub points: u32,
            pub points_per_game: Option<f64>,
        }

        #[derive(Debug, serde::Serialize)]
        pub struct PlayerCareerRow {
            pub season: String,
            pub season_type: String,
            pub team: String,
            pub games: u32,
            pub goals: u32,
            pub assists: u32,
            pub points: u32,
            pub points_per_game: Option<f64>,
        }

        #[derive(Debug, serde::Serialize)]
        pub struct PlayerMeta {
            pub season: String,
            pub season_type: String,
            pub career_rows: usize,
        }

        /// `GET /api/v1/player/:id` — JSON twin of `/player/:id`.
        ///
        /// Same load + projection path as the HTML handler. Errors for
        /// unknown id collapse into a 404 JSON body (axum default body
        /// is fine — clients should branch on status code).
        pub async fn get_player_json(
            State(state): State<WebState>,
            Path(id): Path<u32>,
        ) -> Response {
            let (season_str, season_type) = {
                let cfg = state.config.read().await;
                let st = match cfg.active_season_type.as_str() {
                    "playoff" | "playoffs" => SeasonType::Playoff,
                    _ => SeasonType::Regular,
                };
                (cfg.active_season.clone(), st)
            };
            let season_u32: u32 = match season_str.parse() {
                Ok(n) => n,
                Err(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        axum::Json(serde_json::json!({
                            "error": "bad_active_season",
                            "message": format!("Season '{season_str}' is not a valid YYYYZZZZ id"),
                        })),
                    )
                        .into_response();
                }
            };
            let season = Season(season_u32);
            let pid = PlayerId(id);

            // Mirror the HTML handler's lazy career fan-out.
            {
                let mut repo = state.repo.write().await;
                if let Err(e) =
                    icelines_fetch::stats_loader::load_player_career_into_repo(&mut repo, pid)
                {
                    eprintln!(
                        "warn: career fan-out for pid={id} failed: {e} — \
                         /api/v1/player/:id will return only seasons already loaded"
                    );
                }
            }

            let repo = state.repo.read().await;
            let identity = match repo.identity(pid) {
                Some(i) => i,
                None => {
                    return (
                        StatusCode::NOT_FOUND,
                        axum::Json(serde_json::json!({
                            "error": "player_not_found",
                            "message": format!(
                                "No player with NHL id {id} in the active repository."
                            ),
                            "nhl_id": id,
                        })),
                    )
                        .into_response();
                }
            };

            let (gp, goals, assists, points, position, team) =
                match repo.view(pid, season, season_type) {
                    Some(v) => (
                        v.gp(),
                        v.goals(),
                        v.assists(),
                        v.points(),
                        v.position().abbreviation().to_owned(),
                        v.team_display().to_owned(),
                    ),
                    None => (0, 0, 0, 0, String::new(), String::new()),
                };
            let ppg = if gp > 0 {
                Some((points as f64) / (gp as f64))
            } else {
                None
            };

            let mut career: Vec<PlayerCareerRow> = match repo.career_all(pid) {
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
                            .unwrap_or_default();
                        let ppg = if totals.gp > 0 {
                            Some((totals.points as f64) / (totals.gp as f64))
                        } else {
                            None
                        };
                        Some(PlayerCareerRow {
                            season: pretty_season(s.season),
                            season_type: match s.season_type {
                                SeasonType::Regular => "regular".to_owned(),
                                SeasonType::Playoff => "playoff".to_owned(),
                            },
                            team: last_team,
                            games: totals.gp,
                            goals: totals.goals,
                            assists: totals.assists,
                            points: totals.points,
                            points_per_game: ppg,
                        })
                    })
                    .collect(),
                None => Vec::new(),
            };
            career.sort_by(|a, b| {
                b.season
                    .cmp(&a.season)
                    .then(a.season_type.cmp(&b.season_type))
            });
            let career_rows_n = career.len();

            let envelope = PlayerEnvelope {
                schema_version: 1,
                route: "player",
                data: PlayerData {
                    nhl_id: id,
                    full_name: identity.full_name.clone(),
                    position,
                    team,
                    headshot_url: identity.headshot_canonical_url.clone(),
                    active_season_stats: PlayerActiveStats {
                        season: season_str.clone(),
                        season_type: match season_type {
                            SeasonType::Regular => "regular".to_owned(),
                            SeasonType::Playoff => "playoff".to_owned(),
                        },
                        games: gp,
                        goals,
                        assists,
                        points,
                        points_per_game: ppg,
                    },
                    career,
                },
                meta: PlayerMeta {
                    season: season_str,
                    season_type: match season_type {
                        SeasonType::Regular => "regular".to_owned(),
                        SeasonType::Playoff => "playoff".to_owned(),
                    },
                    career_rows: career_rows_n,
                },
            };
            axum::Json(envelope).into_response()
        }
    }

    /// `/compare` — UX.D. Side-by-side stat comparison of two players.
    pub mod compare {
        use crate::state::WebState;
        use crate::templates::{ComparePlayerCard, CompareTemplate};
        use askama::Template;
        use axum::extract::{Query, State};
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};
        use icelines_core::identity::PlayerId;
        use icelines_core::model::Season;
        use icelines_core::season_stats::SeasonType;
        use serde::Deserialize;

        #[derive(Debug, Deserialize, Default)]
        pub struct CompareQuery {
            /// Either a NHL id ("8478402") or a player name
            /// ("Connor McDavid"). UX.H — the player card's compare
            /// form posts a name selected from the autocomplete
            /// datalist; deep-linked URLs may still pass an id.
            #[serde(default)]
            pub a: Option<String>,
            #[serde(default)]
            pub b: Option<String>,
        }

        /// Resolve a `?a=` / `?b=` query value (id or name) into a
        /// numeric NHL id. Pure u32 short-circuits; otherwise the
        /// first repo identity whose `full_name` matches
        /// case-insensitively wins. Returns None when there's no
        /// match, so callers can render a friendly error instead of
        /// silently picking the wrong player.
        async fn resolve_id(state: &WebState, raw: &str) -> Option<u32> {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return None;
            }
            if let Ok(id) = trimmed.parse::<u32>() {
                return Some(id);
            }
            let needle = trimmed.to_ascii_lowercase();
            let repo = state.repo.read().await;
            for identity in repo.iter_identities() {
                if identity.full_name.to_ascii_lowercase() == needle {
                    return Some(identity.id.0);
                }
            }
            None
        }

        fn opt_u(o: Option<u32>) -> String {
            match o {
                Some(n) => n.to_string(),
                None => "—".to_owned(),
            }
        }
        fn opt_pct(o: Option<f32>) -> String {
            match o {
                Some(p) => {
                    if p.abs() <= 1.5 {
                        format!("{:.1}%", p * 100.0)
                    } else {
                        format!("{:.1}%", p)
                    }
                }
                None => "—".to_owned(),
            }
        }
        fn toi_mmss(o: Option<u32>) -> String {
            match o {
                Some(secs) => {
                    let m = secs / 60;
                    let s = secs % 60;
                    format!("{m}:{s:02}")
                }
                None => "—".to_owned(),
            }
        }

        async fn build_card(
            state: &WebState,
            id: u32,
            season: Season,
            season_type: SeasonType,
        ) -> Option<ComparePlayerCard> {
            // Lazy career fan-out so a freshly-opened player has full
            // career loaded — same pattern as the player handler.
            let pid = PlayerId(id);
            {
                let mut repo = state.repo.write().await;
                let _ = icelines_fetch::stats_loader::load_player_career_into_repo(&mut repo, pid);
            }
            let repo = state.repo.read().await;
            let identity = repo.identity(pid)?;
            let view = repo.view(pid, season, season_type);
            let (
                gp,
                goals,
                assists,
                points,
                position,
                team,
                team_link,
                plus_minus_str,
                pim_str,
                shots_str,
                shooting_pct_str,
                hits_str,
                blocks_str,
                takeaways_str,
                giveaways_str,
                faceoff_pct_str,
                pp_goals_str,
                pp_points_str,
                sh_goals_str,
                gwg_str,
                toi_per_game_str,
            ) = match view {
                Some(v) => {
                    let totals = &v.stats.totals;
                    let team_display = v.team_display().to_owned();
                    let team_link = if team_display.chars().all(|c| c.is_ascii_alphabetic())
                        && team_display.len() <= 3
                    {
                        team_display.clone()
                    } else {
                        String::new()
                    };
                    (
                        v.gp(),
                        v.goals(),
                        v.assists(),
                        v.points(),
                        v.position().abbreviation().to_owned(),
                        team_display,
                        team_link,
                        format!("{:+}", v.plus_minus()),
                        totals.pim.to_string(),
                        totals.shots.to_string(),
                        opt_pct(totals.shooting_pct),
                        opt_u(v.hits()),
                        opt_u(v.blocked_shots()),
                        opt_u(v.takeaways()),
                        opt_u(v.giveaways()),
                        opt_pct(totals.faceoff_win_pct),
                        totals.pp_goals.to_string(),
                        totals.pp_points.to_string(),
                        totals.sh_goals.to_string(),
                        totals.gwg.to_string(),
                        toi_mmss(totals.toi_per_game_sec),
                    )
                }
                None => (
                    0,
                    0,
                    0,
                    0,
                    "—".to_owned(),
                    "—".to_owned(),
                    String::new(),
                    "—".to_owned(),
                    "—".to_owned(),
                    "—".to_owned(),
                    "—".to_owned(),
                    "—".to_owned(),
                    "—".to_owned(),
                    "—".to_owned(),
                    "—".to_owned(),
                    "—".to_owned(),
                    "—".to_owned(),
                    "—".to_owned(),
                    "—".to_owned(),
                    "—".to_owned(),
                    "—".to_owned(),
                ),
            };
            let ppg_str = if gp > 0 {
                format!("{:.2}", points as f64 / gp as f64)
            } else {
                String::new()
            };
            Some(ComparePlayerCard {
                nhl_id: id,
                full_name: identity.full_name.clone(),
                position,
                team,
                team_link: team_link.clone(),
                headshot_url: if !team_link.is_empty() {
                    Some(super::build_headshot_url(season.0, &team_link, id))
                } else {
                    identity.headshot_canonical_url.clone()
                },
                gp,
                goals,
                assists,
                points,
                ppg_str,
                plus_minus_str,
                pim_str,
                shots_str,
                shooting_pct_str,
                hits_str,
                blocks_str,
                takeaways_str,
                giveaways_str,
                faceoff_pct_str,
                pp_goals_str,
                pp_points_str,
                sh_goals_str,
                gwg_str,
                toi_per_game_str,
            })
        }

        pub async fn get_compare(
            State(state): State<WebState>,
            Query(q): Query<CompareQuery>,
        ) -> Response {
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
                    let tmpl = CompareTemplate {
                        active_label,
                        a: None,
                        b: None,
                        error: Some(format!(
                            "Active season '{season_str}' is not a valid YYYYZZZZ id"
                        )),
                        winners: crate::templates::CompareWinners::default(),
                    };
                    return match tmpl.render() {
                        Ok(html) => Html(html).into_response(),
                        Err(e) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Html(format!("template render failed: {e}")),
                        )
                            .into_response(),
                    };
                }
            };
            let season = Season(season_u32);

            // Resolve "a" and "b" to numeric NHL ids. Each may be
            // either a u32 (deep-linked URL like
            // /compare?a=8478402&b=8477934) or a name typed into
            // the player-card autocomplete ("Connor McDavid"). A
            // raw value that doesn't parse as u32 AND isn't a known
            // repo identity name surfaces as `unresolved` so the
            // template can name what failed.
            let a_raw = q.a.as_deref().map(str::trim).filter(|s| !s.is_empty());
            let b_raw = q.b.as_deref().map(str::trim).filter(|s| !s.is_empty());
            let a_id = match a_raw {
                Some(raw) => resolve_id(&state, raw).await,
                None => None,
            };
            let b_id = match b_raw {
                Some(raw) => resolve_id(&state, raw).await,
                None => None,
            };

            let (a_card, a_missing) = match a_id {
                Some(id) => {
                    let card = build_card(&state, id, season, season_type).await;
                    let missing = card.is_none();
                    (card, missing.then_some(id))
                }
                None => (None, None),
            };
            let (b_card, b_missing) = match b_id {
                Some(id) => {
                    let card = build_card(&state, id, season, season_type).await;
                    let missing = card.is_none();
                    (card, missing.then_some(id))
                }
                None => (None, None),
            };

            // Distinguish "no input given" vs "input given but didn't
            // resolve to a known player". The latter is more useful to
            // surface with the typed text.
            let a_unresolved = a_raw.filter(|_| a_id.is_none());
            let b_unresolved = b_raw.filter(|_| b_id.is_none());

            let error = if a_raw.is_none() && b_raw.is_none() {
                None
            } else if a_raw.is_none() {
                Some("Missing first player (?a=).".to_owned())
            } else if b_raw.is_none() {
                Some("Missing second player (?b=).".to_owned())
            } else if let (Some(a_text), Some(b_text)) = (a_unresolved, b_unresolved) {
                Some(format!(
                    "Neither '{a_text}' nor '{b_text}' matches a player in the active repository."
                ))
            } else if let Some(text) = a_unresolved {
                Some(format!("No player matches '{text}'."))
            } else if let Some(text) = b_unresolved {
                Some(format!("No player matches '{text}'."))
            } else if let (Some(id_a), Some(id_b)) = (a_missing, b_missing) {
                Some(format!(
                    "Neither player {id_a} nor {id_b} is in the active repository."
                ))
            } else {
                a_missing
                    .or(b_missing)
                    .map(|id| format!("No player with NHL id {id}."))
            };

            // Sasq.8 — compute per-stat winner flags so the template
            // can bold whichever side has the better value. Most
            // stats are higher-is-better; PIM and giveaways are
            // flipped (lower-is-better in modern hockey
            // contexts — fewer minor penalties / fewer turnovers
            // are signals of cleaner play).
            let winners = match (&a_card, &b_card) {
                (Some(pa), Some(pb)) => build_compare_winners(pa, pb),
                _ => crate::templates::CompareWinners::default(),
            };

            let tmpl = CompareTemplate {
                active_label,
                a: a_card,
                b: b_card,
                error,
                winners,
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

        /// Compute per-stat winner flags for two ComparePlayerCards.
        /// Higher-is-better unless explicitly flipped (PIM, GV).
        /// Numeric stats (gp, goals, etc.) compare directly; string
        /// stats parse out a leading number where possible. Strings
        /// containing "—" or that fail to parse skip the comparison
        /// (both flags stay false → neither side bolded).
        fn build_compare_winners(
            pa: &ComparePlayerCard,
            pb: &ComparePlayerCard,
        ) -> crate::templates::CompareWinners {
            use crate::templates::CompareWinners;
            // Parse a stat string like "+12", "1.78", "10.5%", "20:20",
            // "—" into Option<f64>. The "20:20" TOI/G case is M:SS
            // which we convert to total seconds for comparison.
            fn parse_stat(s: &str) -> Option<f64> {
                let t = s.trim();
                if t.is_empty() || t == "—" {
                    return None;
                }
                if let Some((m, s)) = t.split_once(':') {
                    if let (Ok(mi), Ok(se)) = (m.parse::<u32>(), s.parse::<u32>()) {
                        return Some(f64::from(mi) * 60.0 + f64::from(se));
                    }
                }
                let stripped = t.trim_end_matches('%');
                stripped.parse::<f64>().ok()
            }
            // Compare two values with `higher_better` bias and write
            // the (a_wins, b_wins) booleans. Equality → both false.
            fn cmp_pair(a: f64, b: f64, higher_better: bool) -> (bool, bool) {
                use std::cmp::Ordering;
                let ord = a.partial_cmp(&b).unwrap_or(Ordering::Equal);
                if higher_better {
                    (ord == Ordering::Greater, ord == Ordering::Less)
                } else {
                    (ord == Ordering::Less, ord == Ordering::Greater)
                }
            }
            fn cmp_strs(sa: &str, sb: &str, higher_better: bool) -> (bool, bool) {
                match (parse_stat(sa), parse_stat(sb)) {
                    (Some(a), Some(b)) => cmp_pair(a, b, higher_better),
                    _ => (false, false),
                }
            }
            fn cmp_u32(a: u32, b: u32) -> (bool, bool) {
                cmp_pair(f64::from(a), f64::from(b), true)
            }
            let mut w = CompareWinners::default();
            (w.gp_a, w.gp_b) = cmp_u32(pa.gp, pb.gp);
            (w.goals_a, w.goals_b) = cmp_u32(pa.goals, pb.goals);
            (w.assists_a, w.assists_b) = cmp_u32(pa.assists, pb.assists);
            (w.points_a, w.points_b) = cmp_u32(pa.points, pb.points);
            (w.ppg_a, w.ppg_b) = cmp_strs(&pa.ppg_str, &pb.ppg_str, true);
            (w.plus_minus_a, w.plus_minus_b) =
                cmp_strs(&pa.plus_minus_str, &pb.plus_minus_str, true);
            (w.pim_a, w.pim_b) = cmp_strs(&pa.pim_str, &pb.pim_str, false); // lower better
            (w.shots_a, w.shots_b) = cmp_strs(&pa.shots_str, &pb.shots_str, true);
            (w.shooting_pct_a, w.shooting_pct_b) =
                cmp_strs(&pa.shooting_pct_str, &pb.shooting_pct_str, true);
            (w.hits_a, w.hits_b) = cmp_strs(&pa.hits_str, &pb.hits_str, true);
            (w.blocks_a, w.blocks_b) = cmp_strs(&pa.blocks_str, &pb.blocks_str, true);
            (w.takeaways_a, w.takeaways_b) = cmp_strs(&pa.takeaways_str, &pb.takeaways_str, true);
            (w.giveaways_a, w.giveaways_b) = cmp_strs(&pa.giveaways_str, &pb.giveaways_str, false); // lower better
            (w.faceoff_pct_a, w.faceoff_pct_b) =
                cmp_strs(&pa.faceoff_pct_str, &pb.faceoff_pct_str, true);
            (w.pp_goals_a, w.pp_goals_b) = cmp_strs(&pa.pp_goals_str, &pb.pp_goals_str, true);
            (w.pp_points_a, w.pp_points_b) = cmp_strs(&pa.pp_points_str, &pb.pp_points_str, true);
            (w.sh_goals_a, w.sh_goals_b) = cmp_strs(&pa.sh_goals_str, &pb.sh_goals_str, true);
            (w.gwg_a, w.gwg_b) = cmp_strs(&pa.gwg_str, &pb.gwg_str, true);
            (w.toi_per_game_a, w.toi_per_game_b) =
                cmp_strs(&pa.toi_per_game_str, &pb.toi_per_game_str, true);
            w
        }
    }

    pub mod home {
        use crate::state::WebState;
        use crate::templates::{GoalieRow, HomeTemplate, LeaderRow};
        use askama::Template;
        use axum::extract::State;
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};
        use icelines_core::model::Season;
        use icelines_core::season_stats::SeasonType;

        /// Goalie qualified-GP floors used in the home preview.
        /// Mirrors the constants in the goalies handler — kept local
        /// rather than re-exported because the values are identical
        /// today and a divergence would be a deliberate decision.
        const HOME_QUALIFIED_GP_REGULAR: u32 = 5;
        const HOME_QUALIFIED_GP_PLAYOFF: u32 = 1;
        const HOME_PREVIEW_N: usize = 3;

        /// `GET /` — askama-rendered home with top-3 skater + goalie
        /// previews. Reads the active (season, season_type) from
        /// `WebState.config`, then takes one read lock on the repo to
        /// project both slices. Empty-vec fallbacks (rather than
        /// erroring) so the home page stays useful even when the
        /// active season has no data loaded yet.
        pub async fn get_home(State(state): State<WebState>) -> Response {
            let (season_str, season_type, active_label) = {
                let cfg = state.config.read().await;
                let st = match cfg.active_season_type.as_str() {
                    "playoff" | "playoffs" => SeasonType::Playoff,
                    _ => SeasonType::Regular,
                };
                (cfg.active_season.clone(), st, cfg.active_label.clone())
            };

            let (top_skaters, top_goalies) = match season_str.parse::<u32>() {
                Ok(season_u32) => {
                    let season = Season(season_u32);
                    let goalie_floor = match season_type {
                        SeasonType::Regular => HOME_QUALIFIED_GP_REGULAR,
                        SeasonType::Playoff => HOME_QUALIFIED_GP_PLAYOFF,
                    };
                    let repo = state.repo.read().await;

                    let mut skaters: Vec<LeaderRow> = repo
                        .skaters(season, season_type)
                        .map(|v| super::project_leader_row(&v))
                        .collect();
                    skaters.sort_by(|a, b| {
                        b.points
                            .cmp(&a.points)
                            .then(b.goals.cmp(&a.goals))
                            .then(a.name.cmp(&b.name))
                    });
                    skaters.truncate(HOME_PREVIEW_N);

                    let mut goalies: Vec<GoalieRow> = repo
                        .goalies(season, season_type)
                        .filter(|v| v.gp() >= goalie_floor)
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
                            let team_display = v.team_display().to_owned();
                            let headshot_url = super::build_headshot_url_for_display(
                                v.season().0,
                                &team_display,
                                v.id().0,
                            );
                            Some(GoalieRow {
                                nhl_id: v.id().0,
                                name: v.full_name().to_owned(),
                                team: team_display,
                                gp: v.gp(),
                                wins: g.wins,
                                losses: g.losses,
                                shutouts: g.shutouts,
                                save_pct_str,
                                gaa_str,
                                headshot_url,
                                headshot_fallback_url: format!(
                                    "https://assets.nhle.com/mugs/nhl/default/{}.png",
                                    v.id().0
                                ),
                            })
                        })
                        .collect();
                    goalies.sort_by(|a, b| {
                        let ap = a.save_pct_str.parse::<f64>().unwrap_or(0.0);
                        let bp = b.save_pct_str.parse::<f64>().unwrap_or(0.0);
                        bp.partial_cmp(&ap)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then(b.wins.cmp(&a.wins))
                            .then(a.name.cmp(&b.name))
                    });
                    goalies.truncate(HOME_PREVIEW_N);

                    (skaters, goalies)
                }
                Err(_) => (Vec::new(), Vec::new()),
            };

            let tmpl = HomeTemplate {
                active_label,
                top_skaters,
                top_goalies,
            };
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

    /// `/transactions` — King.8.2. League moves feed for the active
    /// season. Uses `load_transactions_with_fallback` so the handler
    /// works against bundled snapshots, installed bundles, OR a
    /// fetched snapshot (priority: snapshot store → embedded →
    /// installed bundle).
    pub mod transactions {
        use crate::state::WebState;
        use crate::templates::{TransactionRow, TransactionsTemplate};
        use askama::Template;
        use axum::extract::{Query, State};
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};
        use icelines_core::transactions::{TransactionKind, TRANSACTIONS_EARLIEST_SEASON};
        use serde::Deserialize;

        /// Query params accepted by `/transactions`.
        #[derive(Debug, Deserialize, Default)]
        pub struct TransactionsQuery {
            /// Filter by kind: `trade`, `signing`, `recall`,
            /// `reassignment`, `waiver` (expands to all 3 waiver kinds),
            /// `ir`, `other`. Unknown → 400.
            #[serde(default)]
            pub kind: Option<String>,
            /// Filter by team abbreviation (case-insensitive).
            #[serde(default)]
            pub team: Option<String>,
        }

        fn pretty_season(s: &str) -> String {
            if s.len() == 8 {
                format!("{}-{}", &s[0..4], &s[6..8])
            } else {
                s.to_owned()
            }
        }

        /// Pretty-cased label per kind, for the chip column. Matches the
        /// CLI's display style ("Waiver claim" not "waiver_claim").
        fn pretty_kind(k: TransactionKind) -> &'static str {
            match k {
                TransactionKind::Trade => "Trade",
                TransactionKind::WaiverPlacement => "Waivers",
                TransactionKind::WaiverClear => "Waivers",
                TransactionKind::WaiverClaim => "Waiver claim",
                TransactionKind::Signing => "Signing",
                TransactionKind::Recall => "Recall",
                TransactionKind::Reassignment => "Reassignment",
                TransactionKind::InjuryReserve => "IR",
                TransactionKind::Other => "Other",
            }
        }

        pub async fn get_transactions(
            State(state): State<WebState>,
            Query(q): Query<TransactionsQuery>,
        ) -> Response {
            let (season_str, active_label) = {
                let cfg = state.config.read().await;
                (cfg.active_season.clone(), cfg.active_label.clone())
            };

            // Validate the kind filter early. Bad input → 400, not 500.
            let kind_filter: Option<Vec<TransactionKind>> = match q.kind.as_deref() {
                None | Some("") => None,
                Some(k) => match TransactionKind::parse_filter(k) {
                    Ok(v) => Some(v),
                    Err(msg) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Html(format!(
                                "<!doctype html><html><body>\
                                 <h1>Bad filter</h1><p>{msg}</p>\
                                 <p><a href=\"/transactions\">← back to transactions</a></p>\
                                 </body></html>",
                            )),
                        )
                            .into_response();
                    }
                },
            };
            let team_filter = q
                .team
                .as_deref()
                .map(|t| t.trim().to_ascii_uppercase())
                .filter(|t| !t.is_empty());
            let active_kind = q.kind.clone().unwrap_or_default();
            let active_team = team_filter.clone().unwrap_or_default();

            // Out-of-coverage check matches the CLI behavior.
            let out_of_coverage = season_str.as_str() < TRANSACTIONS_EARLIEST_SEASON;

            // Build the SnapshotStore for this request. Cheap — just a
            // PathBuf wrap. If `snapshots_root` is None (test setup),
            // fall back to the default (~/.icelines/snapshots).
            let snapshots_root = match state.snapshots_root.as_ref() {
                Some(p) => p.clone(),
                None => icelines_fetch::snapshot::SnapshotStore::default_root(),
            };
            let store = icelines_fetch::snapshot::SnapshotStore::new(snapshots_root);

            let envelope_result = if out_of_coverage {
                Err(())
            } else {
                icelines_fetch::bundled::load_transactions_with_fallback(&season_str, &store)
                    .map_err(|_| ())
            };

            let mut rows: Vec<TransactionRow> = match envelope_result {
                Ok(env) => env
                    .rows
                    .into_iter()
                    .filter(|t| match &kind_filter {
                        None => true,
                        Some(kinds) => kinds.contains(&t.kind),
                    })
                    .filter(|t| match &team_filter {
                        None => true,
                        Some(team) => t
                            .team
                            .as_ref()
                            .map(|a| a.as_str().eq_ignore_ascii_case(team))
                            .unwrap_or(false),
                    })
                    .map(|t| TransactionRow {
                        date: t.date,
                        team: t.team.map(|a| a.as_str().to_owned()).unwrap_or_default(),
                        kind_label: t.kind.label().to_owned(),
                        kind_pretty: pretty_kind(t.kind).to_owned(),
                        description: t.description,
                    })
                    .collect(),
                Err(()) => Vec::new(),
            };

            // Newest first. Date is YYYY-MM-DD so string sort works.
            rows.sort_by(|a, b| b.date.cmp(&a.date));
            // Cap to 1000 to keep the page render bounded.
            rows.truncate(1000);
            let total = rows.len();
            let empty_unfiltered =
                rows.is_empty() && kind_filter.is_none() && team_filter.is_none();

            let tmpl = TransactionsTemplate {
                active_label,
                season_pretty: pretty_season(&season_str),
                rows,
                total,
                empty_unfiltered,
                active_kind,
                active_team,
                out_of_coverage,
                earliest_season_pretty: pretty_season(TRANSACTIONS_EARLIEST_SEASON),
            };
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

    // ── King.7 — live NHL data ────────────────────────────────────────

    /// Build a fresh `NhlApiClient` per request. Cheap (just
    /// constructs the reqwest client) and avoids holding a long-lived
    /// HTTP client in `WebState` until we have a concrete reason to
    /// (cookie pool, custom retry, etc.). Lindsay L.1.5 retry policy
    /// fires inside the client.
    fn nhl_client() -> icelines_fetch::nhl_api::NhlApiClient {
        icelines_fetch::nhl_api::NhlApiClient::production()
    }

    /// `/scores` — King.7.1. Live NHL schedule for one game-week.
    pub mod scores {
        use crate::state::WebState;
        use crate::templates::{ScoreRow, ScoresDay, ScoresTemplate};
        use askama::Template;
        use axum::extract::{Query, State};
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};
        use chrono::{Datelike, Duration, NaiveDate, Utc, Weekday};
        use serde::Deserialize;

        #[derive(Debug, Deserialize, Default)]
        pub struct ScoresQuery {
            /// YYYY-MM-DD. The NHL API returns a 7-day window starting
            /// from this date. Default: today.
            #[serde(default)]
            pub date: Option<String>,
        }

        fn parse_date(s: &str) -> Option<NaiveDate> {
            NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
        }

        fn pretty_day(d: NaiveDate) -> String {
            let weekday = match d.weekday() {
                Weekday::Mon => "Mon",
                Weekday::Tue => "Tue",
                Weekday::Wed => "Wed",
                Weekday::Thu => "Thu",
                Weekday::Fri => "Fri",
                Weekday::Sat => "Sat",
                Weekday::Sun => "Sun",
            };
            let month = match d.month() {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => "?",
            };
            format!("{}, {} {}, {}", weekday, month, d.day(), d.year())
        }

        fn state_to_class_label(
            state: Option<&str>,
            last_period: Option<&str>,
        ) -> (String, String) {
            match state.unwrap_or("") {
                "FINAL" | "OFF" => {
                    let label = match last_period.unwrap_or("REG") {
                        "OT" => "FINAL/OT".to_owned(),
                        "SO" => "FINAL/SO".to_owned(),
                        _ => "FINAL".to_owned(),
                    };
                    ("final".to_owned(), label)
                }
                "LIVE" | "CRIT" => ("live".to_owned(), "LIVE".to_owned()),
                "PRE" => ("future".to_owned(), "Pre-game".to_owned()),
                "FUT" | "" => ("future".to_owned(), "Scheduled".to_owned()),
                other => ("future".to_owned(), other.to_owned()),
            }
        }

        /// Drop the date portion of an ISO-8601 timestamp and emit
        /// just `HH:MM UTC`. Inputs look like `2026-05-04T19:00:00Z`.
        fn pretty_time_utc(ts: &str) -> String {
            if let Some(t) = ts.split('T').nth(1) {
                let hhmm: String = t.chars().take(5).collect();
                if hhmm.len() == 5 {
                    return format!("{hhmm} UTC");
                }
            }
            String::new()
        }

        pub async fn get_scores(
            State(state): State<WebState>,
            Query(q): Query<ScoresQuery>,
        ) -> Response {
            let active_label = state.config.read().await.active_label.clone();

            let today = Utc::now().date_naive();
            let active_date = q.date.as_deref().and_then(parse_date).unwrap_or(today);
            let prev_date = active_date - Duration::days(7);
            let next_date = active_date + Duration::days(7);

            let client = super::nhl_client();
            let fetch_result = if q.date.is_some() {
                client
                    .fetch_schedule_for_date(&active_date.format("%Y-%m-%d").to_string())
                    .await
            } else {
                client.fetch_today_schedule().await
            };

            let (days, total_games, fetch_error) = match fetch_result {
                Ok(games) => {
                    use std::collections::BTreeMap;
                    let mut by_date: BTreeMap<String, Vec<ScoreRow>> = BTreeMap::new();
                    let total = games.len();
                    for g in games {
                        let (state_class, state_label) =
                            state_to_class_label(g.game_state.as_deref(), g.last_period.as_deref());
                        let series_context = if g.is_playoff() {
                            let series_game = g.series_game.unwrap_or_default();
                            let aw = g.away_wins.unwrap_or(0);
                            let hw = g.home_wins.unwrap_or(0);
                            let series_state = if aw > hw {
                                format!("{} leads {}-{}", g.away_abbrev, aw, hw)
                            } else if hw > aw {
                                format!("{} leads {}-{}", g.home_abbrev, hw, aw)
                            } else if aw == 0 {
                                "series begins".to_owned()
                            } else {
                                format!("tied {}-{}", aw, hw)
                            };
                            if series_game.is_empty() {
                                series_state
                            } else {
                                format!("{series_game} · {series_state}")
                            }
                        } else {
                            String::new()
                        };
                        let row = ScoreRow {
                            away_abbrev: g.away_abbrev,
                            away_name: g.away_name,
                            home_abbrev: g.home_abbrev,
                            home_name: g.home_name,
                            away_score_str: g.away_score.map(|s| s.to_string()).unwrap_or_default(),
                            home_score_str: g.home_score.map(|s| s.to_string()).unwrap_or_default(),
                            state_label,
                            state_class,
                            start_time_label: pretty_time_utc(&g.start_time_utc),
                            is_playoff: g.game_type == 3,
                            series_context,
                        };
                        by_date.entry(g.date).or_default().push(row);
                    }
                    let days: Vec<ScoresDay> = by_date
                        .into_iter()
                        .map(|(date, rows)| {
                            let date_pretty = parse_date(&date)
                                .map(pretty_day)
                                .unwrap_or_else(|| date.clone());
                            ScoresDay {
                                date,
                                date_pretty,
                                rows,
                            }
                        })
                        .collect();
                    (days, total, None)
                }
                Err(e) => (Vec::new(), 0, Some(e.to_string())),
            };

            let tmpl = ScoresTemplate {
                active_label,
                active_date: active_date.format("%Y-%m-%d").to_string(),
                prev_date: prev_date.format("%Y-%m-%d").to_string(),
                next_date: next_date.format("%Y-%m-%d").to_string(),
                today_date: today.format("%Y-%m-%d").to_string(),
                days,
                total_games,
                fetch_error,
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

    /// `/playoffs` — King.7.2. Bracket view, bundled fallback.
    pub mod playoffs {
        use crate::state::WebState;
        use crate::templates::{PlayoffsRoundView, PlayoffsSeriesView, PlayoffsTemplate};
        use askama::Template;
        use axum::extract::State;
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};

        fn pretty_season(s: &str) -> String {
            if s.len() == 8 {
                format!("{}-{}", &s[0..4], &s[6..8])
            } else {
                s.to_owned()
            }
        }

        /// Convert a `PlayoffBracket` (live or bundled-derived) into
        /// the template's view shape.
        fn project_bracket(b: icelines_fetch::nhl_api::PlayoffBracket) -> Vec<PlayoffsRoundView> {
            b.rounds
                .into_iter()
                .map(|r| {
                    let series = r
                        .series
                        .iter()
                        .map(|s| PlayoffsSeriesView {
                            top_abbrev: s.top_seed_abbrev.clone(),
                            top_name: s.top_seed_name.clone(),
                            top_wins: s.top_seed_wins,
                            bottom_abbrev: s.bottom_seed_abbrev.clone(),
                            bottom_name: s.bottom_seed_name.clone(),
                            bottom_wins: s.bottom_seed_wins,
                            summary: s.summary(),
                            is_complete: s.is_complete(),
                            conference: s.conference.clone().unwrap_or_default(),
                        })
                        .collect();
                    PlayoffsRoundView {
                        round_number: r.round_number,
                        label: r.label,
                        series,
                    }
                })
                .collect()
        }

        pub async fn get_playoffs(State(state): State<WebState>) -> Response {
            let (active_label, season_str) = {
                let cfg = state.config.read().await;
                (cfg.active_label.clone(), cfg.active_season.clone())
            };

            // 1. Try bundled (instant, historical seasons).
            let bundled =
                icelines_fetch::bundled::load_playoffs(&season_str).map(|b| b.to_bracket());

            let (rounds, source_label, fetch_error) = if let Some(bracket) = bundled {
                (
                    project_bracket(bracket),
                    "historical bundle".to_owned(),
                    None,
                )
            } else {
                // 2. Fall back to the live API. The playoff endpoint takes
                //    the second year of the season (2026 for 25-26).
                let year: u16 = season_str
                    .get(4..8)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                if year == 0 {
                    (
                        Vec::new(),
                        "—".to_owned(),
                        Some(format!(
                            "Cannot derive playoff year from season '{season_str}'"
                        )),
                    )
                } else {
                    let client = super::nhl_client();
                    match client.fetch_playoff_bracket(year).await {
                        Ok(b) => (
                            project_bracket(b),
                            format!("live · /v1/playoff-bracket/{year}"),
                            None,
                        ),
                        Err(e) => (Vec::new(), "—".to_owned(), Some(e.to_string())),
                    }
                }
            };

            let empty = rounds.iter().all(|r| r.series.is_empty());

            let tmpl = PlayoffsTemplate {
                active_label,
                season_pretty: pretty_season(&season_str),
                source_label,
                rounds,
                empty,
                fetch_error,
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

    /// `/schedule` — King.7.3. Team-season schedule view.
    pub mod schedule {
        use crate::state::WebState;
        use crate::templates::{ScheduleRow, ScheduleTemplate, TeamChip};
        use askama::Template;
        use axum::extract::{Query, State};
        use axum::http::StatusCode;
        use axum::response::{Html, IntoResponse, Response};
        use serde::Deserialize;

        #[derive(Debug, Deserialize, Default)]
        pub struct ScheduleQuery {
            #[serde(default)]
            pub team: Option<String>,
        }

        fn pretty_season(s: &str) -> String {
            if s.len() == 8 {
                format!("{}-{}", &s[0..4], &s[6..8])
            } else {
                s.to_owned()
            }
        }

        /// 32 active NHL franchises. Used to populate the team
        /// picker chip strip. Uppercase, alphabetical.
        const ALL_TEAM_ABBREVS: &[&str] = &[
            "ANA", "BOS", "BUF", "CAR", "CBJ", "CGY", "CHI", "COL", "DAL", "DET", "EDM", "FLA",
            "LAK", "MIN", "MTL", "NJD", "NSH", "NYI", "NYR", "OTT", "PHI", "PIT", "SEA", "SJS",
            "STL", "TBL", "TOR", "UTA", "VAN", "VGK", "WPG", "WSH",
        ];

        pub async fn get_schedule(
            State(state): State<WebState>,
            Query(q): Query<ScheduleQuery>,
        ) -> Response {
            let (active_label, season_str) = {
                let cfg = state.config.read().await;
                (cfg.active_label.clone(), cfg.active_season.clone())
            };

            let team_upper = q
                .team
                .as_deref()
                .map(|t| t.trim().to_ascii_uppercase())
                .filter(|t| !t.is_empty())
                .unwrap_or_default();

            let team_chips: Vec<TeamChip> = ALL_TEAM_ABBREVS
                .iter()
                .map(|a| TeamChip {
                    abbrev: (*a).to_owned(),
                    is_active: a.eq_ignore_ascii_case(&team_upper),
                })
                .collect();

            let (rows, total, fetch_error) = if team_upper.is_empty() {
                (Vec::new(), 0, None)
            } else {
                let client = super::nhl_client();
                match client
                    .fetch_team_season_schedule(&team_upper, &season_str)
                    .await
                {
                    Ok(games) => {
                        let mut rows: Vec<ScheduleRow> = games
                            .into_iter()
                            .map(|g| {
                                let is_home = g.home_abbrev.eq_ignore_ascii_case(&team_upper);
                                let opponent = if is_home {
                                    g.away_abbrev.clone()
                                } else {
                                    g.home_abbrev.clone()
                                };
                                let state_label = match g.game_state.as_deref() {
                                    Some("FINAL") | Some("OFF") => match g.last_period.as_deref() {
                                        Some("OT") => "FINAL/OT".to_owned(),
                                        Some("SO") => "FINAL/SO".to_owned(),
                                        _ => "FINAL".to_owned(),
                                    },
                                    Some("LIVE") | Some("CRIT") => "LIVE".to_owned(),
                                    Some("PRE") => "Pre-game".to_owned(),
                                    Some("FUT") | None => "Scheduled".to_owned(),
                                    Some(s) => s.to_owned(),
                                };
                                ScheduleRow {
                                    date: g.date,
                                    away_abbrev: g.away_abbrev.clone(),
                                    home_abbrev: g.home_abbrev.clone(),
                                    away_score_str: g
                                        .away_score
                                        .map(|s| s.to_string())
                                        .unwrap_or_default(),
                                    home_score_str: g
                                        .home_score
                                        .map(|s| s.to_string())
                                        .unwrap_or_default(),
                                    state_label,
                                    home_or_away: if is_home {
                                        "Home".to_owned()
                                    } else {
                                        "Away".to_owned()
                                    },
                                    opponent_abbrev: opponent,
                                    is_playoff: g.game_type == 3,
                                }
                            })
                            .collect();
                        rows.sort_by(|a, b| a.date.cmp(&b.date));
                        let total = rows.len();
                        (rows, total, None)
                    }
                    Err(e) => (Vec::new(), 0, Some(e.to_string())),
                }
            };

            let tmpl = ScheduleTemplate {
                active_label,
                season_pretty: pretty_season(&season_str),
                active_team: team_upper,
                team_chips,
                rows,
                total,
                fetch_error,
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

    /// `/season-type/:kind` — UX.E. Mutates `WebState.config.active_season_type`
    /// to "regular" or "playoff" and 303-redirects back to the page
    /// the user came from (Referer header), defaulting to `/`.
    pub mod season_type {
        use crate::config::WebConfig;
        use crate::state::WebState;
        use axum::extract::{Path, State};
        use axum::http::{header, HeaderMap, StatusCode};
        use axum::response::{IntoResponse, Response};

        pub async fn set_season_type(
            State(state): State<WebState>,
            Path(kind): Path<String>,
            headers: HeaderMap,
        ) -> Response {
            // Normalize: accept "playoff" / "playoffs" / "regular" /
            // anything-else as regular. Whitelist on the way in so a
            // malformed URL can't poison the config.
            let normalized = match kind.to_ascii_lowercase().as_str() {
                "playoff" | "playoffs" => "playoff",
                _ => "regular",
            };
            {
                let mut cfg = state.config.write().await;
                let new_cfg = WebConfig::new(cfg.active_season.clone(), normalized);
                *cfg = new_cfg;
            }
            // Bounce back to where the user clicked from. Empty/foreign
            // referers fall through to "/" so we never redirect off-site.
            let target = headers
                .get(header::REFERER)
                .and_then(|h| h.to_str().ok())
                .filter(|r| {
                    r.starts_with('/') || r.contains("://127.0.0.1") || r.contains("://localhost")
                })
                .map(|r| {
                    // Strip absolute prefix to keep relative for safety.
                    if let Some(idx) = r.find("://") {
                        let after = &r[idx + 3..];
                        if let Some(slash) = after.find('/') {
                            after[slash..].to_owned()
                        } else {
                            "/".to_owned()
                        }
                    } else {
                        r.to_owned()
                    }
                })
                .unwrap_or_else(|| "/".to_owned());
            (StatusCode::SEE_OTHER, [(header::LOCATION, target)]).into_response()
        }
    }

    /// `not_found` — Sasq.7. Friendly 404 page with a player search
    /// input, replacing axum's bare default. Wired as the router's
    /// `.fallback(...)`, so any unmatched path lands here with the
    /// requested URI surfaced for context.
    pub mod not_found {
        use crate::state::WebState;
        use crate::templates::NotFoundTemplate;
        use askama::Template;
        use axum::extract::State;
        use axum::http::{StatusCode, Uri};
        use axum::response::{Html, IntoResponse, Response};

        pub async fn get_not_found(State(state): State<WebState>, uri: Uri) -> Response {
            let active_label = state.config.read().await.active_label.clone();
            let compare_suggestions = {
                let repo = state.repo.read().await;
                let mut pairs: Vec<(String, u32)> = repo
                    .iter_identities()
                    .map(|i| (i.full_name.clone(), i.id.0))
                    .collect();
                pairs.sort_by(|a, b| a.0.cmp(&b.0));
                pairs
            };
            let tmpl = NotFoundTemplate {
                active_label,
                requested_path: uri.path().to_owned(),
                compare_suggestions,
            };
            match tmpl.render() {
                Ok(html) => (StatusCode::NOT_FOUND, Html(html)).into_response(),
                Err(_) => (
                    StatusCode::NOT_FOUND,
                    Html("<!doctype html><html><body><h1>404</h1></body></html>"),
                )
                    .into_response(),
            }
        }
    }
}
