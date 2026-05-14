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

#![deny(unsafe_code)]

pub mod api;
pub mod config;
pub mod dashboard_command;
pub mod error;
pub mod state;
pub mod static_assets;
pub mod templates;

use axum::{
    routing::{get, post},
    Router,
};

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
    Router::new()
        .route("/", get(handlers::home::get_home))
        .route("/dashboard", get(handlers::dashboard::get_dashboard))
        .route(
            "/dashboard/command",
            post(handlers::dashboard::post_dashboard_command),
        )
        .route("/static/:asset", get(static_assets::serve_static))
        // Real handlers — replace coming-soon stubs as each lands.
        .route("/leaders", get(handlers::leaders::get_leaders))
        // JSON API — King.2.4. /api/v1/leaders is the JSON twin of
        // /leaders. Same query params; envelope shape per spec.
        .route("/api/v1/leaders", get(handlers::leaders::get_leaders_json))
        // Player card — King.3.1. Name links on /leaders point here.
        .route("/player/:id", get(handlers::player::get_player))
        .route(
            "/player/:id/awards",
            get(handlers::awards::get_player_awards),
        )
        // JSON twin — King.3.3.
        .route("/api/v1/player/:id", get(handlers::player::get_player_json))
        .route(
            "/api/v1/player/:id/awards",
            get(handlers::awards::get_player_awards_json),
        )
        .route("/scouting/:id", get(handlers::scouting::get_scouting))
        .route(
            "/api/v1/scouting/:id",
            get(handlers::scouting::get_scouting_json),
        )
        // Compare — UX.D. Side-by-side stats for two players.
        .route("/compare", get(handlers::compare::get_compare))
        .route("/api/v1/compare", get(handlers::compare::get_compare_json))
        // Goalie leaderboard — King.5.1 / .5.2.
        .route("/goalies", get(handlers::goalies::get_goalies))
        .route("/api/v1/goalies", get(handlers::goalies::get_goalies_json))
        // Team roster — King.4.1. /team/SEA, /team/EDM, etc.
        .route("/team/:abbrev", get(handlers::team::get_team))
        .route("/team/:abbrev/season", get(handlers::team::get_team_season))
        .route(
            "/records/player/:id",
            get(handlers::records::get_player_records),
        )
        .route(
            "/records/team/:abbrev",
            get(handlers::records::get_team_records),
        )
        // JSON twin — King.4.2.
        .route("/api/v1/team/:abbrev", get(handlers::team::get_team_json))
        .route(
            "/api/v1/team/:abbrev/season",
            get(handlers::team::get_team_season_json),
        )
        .route(
            "/api/v1/records/player/:id",
            get(handlers::records::get_player_records_json),
        )
        .route(
            "/api/v1/records/team/:abbrev",
            get(handlers::records::get_team_records_json),
        )
        // Depth rankings — Phase Lady Byng follow-up. Cross-team
        // line-value rankings; mirror of TUI Depth tab.
        .route("/depth", get(handlers::depth::get_depth))
        // T3 (post-LP test gap): JSON twin for /depth so external
        // scripts don't have to scrape the HTML table.
        .route("/api/v1/depth", get(handlers::depth::get_depth_json))
        // Phase Selke.6 — fantasy poacher board. HTML and JSON share
        // the core PoachBoardView contract.
        .route("/poach", get(handlers::poach::get_poach))
        .route("/reports/poach", get(handlers::poach::get_poach_report))
        .route("/reports/weekly", get(handlers::poach::get_weekly_report))
        .route("/api/v1/poach", get(handlers::poach::get_poach_json))
        .route(
            "/api/v1/watch-rules",
            get(handlers::poach::get_watch_rules_json),
        )
        .route(
            "/api/v1/watch-rules/set-enabled",
            post(handlers::poach::post_watch_rule_enabled_json),
        )
        .route(
            "/watch-rules/set-enabled",
            post(handlers::poach::post_watch_rule_enabled_form),
        )
        .route(
            "/watch-rules/create",
            post(handlers::poach::post_watch_rule_create_form),
        )
        .route(
            "/watch-rules/delete",
            post(handlers::poach::post_watch_rule_delete_form),
        )
        // Phase Calder.4 — cross-league cohort leaderboard.
        // /career?league=OHL&season=20142015&sort=points
        .route("/career", get(handlers::career::get_career))
        .route("/api/v1/career", get(handlers::career::get_career_json))
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
        .route("/api/v1/scores", get(handlers::scores::get_scores_json))
        .route("/schedule", get(handlers::schedule::get_schedule))
        .route(
            "/api/v1/schedule",
            get(handlers::schedule::get_schedule_json),
        )
        .route("/playoffs", get(handlers::playoffs::get_playoffs))
        .route(
            "/api/v1/playoffs",
            get(handlers::playoffs::get_playoffs_json),
        )
        // Phase Foster.2 — favorites dashboard
        .route("/favorites", get(handlers::favorites::get_favorites))
        .route(
            "/api/v1/favorites",
            get(handlers::favorites::get_favorites_json),
        )
        .route("/watchlist", get(handlers::favorites::get_watchlist))
        .route(
            "/api/v1/watchlist",
            get(handlers::favorites::get_watchlist_json),
        )
        // Phase Conn Smythe C.3 — per-game live detail
        .route("/game/:id", get(handlers::game::get_game))
        .route("/api/v1/game/:id", get(handlers::game::get_game_json))
        // Foster +18 — POST mutators (kept as POST so they can't be
        // CSRF'd via image tags / link prefetch).
        .route("/favorites/add", post(handlers::favorites::post_add))
        .route("/favorites/remove", post(handlers::favorites::post_remove))
        .route(
            "/api/v1/favorites/add",
            post(handlers::favorites::post_add_json),
        )
        .route(
            "/api/v1/favorites/remove",
            post(handlers::favorites::post_remove_json),
        )
        // Transactions feed — King.8.2.
        .route(
            "/transactions",
            get(handlers::transactions::get_transactions),
        )
        .route(
            "/api/v1/transactions",
            get(handlers::transactions::get_transactions_json),
        )
        .route("/fantasy", get(handlers::fantasy::get_fantasy))
        .route(
            "/api/v1/fantasy/gaps",
            get(handlers::fantasy::get_fantasy_gaps_json),
        )
        .route(
            "/api/v1/fantasy/simulate",
            get(handlers::fantasy::get_fantasy_simulation_json),
        )
        .route("/admin", get(handlers::admin::get_admin))
        .route(
            "/api/v1/admin/data-status",
            get(handlers::admin::get_data_status_json),
        )
        .route(
            "/api/v1/admin/snapshots",
            get(handlers::admin::get_snapshots_json),
        )
        .route(
            "/api/v1/admin/config",
            get(handlers::admin::get_config_json),
        )
        .route(
            "/api/v1/admin/config/set",
            post(handlers::admin::post_config_set_json),
        )
        .route(
            "/api/v1/admin/config/reset",
            post(handlers::admin::post_config_reset_json),
        )
        .route(
            "/admin/config/set",
            post(handlers::admin::post_config_set_form),
        )
        .route(
            "/admin/config/reset",
            post(handlers::admin::post_config_reset_form),
        )
        .route(
            "/api/v1/admin/snapshots/activate",
            post(handlers::admin::post_snapshot_activate_json),
        )
        .route(
            "/admin/snapshots/activate",
            post(handlers::admin::post_snapshot_activate_form),
        )
        .route(
            "/api/v1/admin/snapshots/delete",
            post(handlers::admin::post_snapshot_delete_json),
        )
        .route(
            "/admin/snapshots/delete",
            post(handlers::admin::post_snapshot_delete_form),
        )
        .route(
            "/api/v1/admin/data/verify",
            post(handlers::admin::post_data_verify_json),
        )
        .route(
            "/admin/data/verify",
            post(handlers::admin::post_data_verify_form),
        )
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
    pub mod shared;

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
    pub mod leaders;

    /// `/depth` — Phase Lady Byng follow-up. Cross-team line-value
    /// rankings (same data the TUI Depth tab consumes). Mirrors the
    /// goalies handler shape: load the active-season repo, compute
    /// `compute_team_strength_views`, project to template rows, render.
    pub mod depth;

    pub mod admin;
    pub mod fantasy;
    /// Phase Selke.6 — fantasy poacher web board.
    pub mod poach;

    /// Phase Calder.4 — cross-league cohort leaderboard.
    pub mod career;

    /// `/docs` — King.8.1. Renders COMMANDS.md as HTML.
    pub mod docs;

    /// `/team/:abbrev` — King.4.1 roster page.
    pub mod team;

    /// `/goalies` — King.5.1 + King.5.2 goalie leaderboard.
    pub mod goalies;

    pub mod awards;
    /// `/player/:id` — King.3.1 + King.3.2. Player card + career table.
    pub mod player;
    pub mod records;
    pub mod scouting;

    /// `/compare` — UX.D. Side-by-side stat comparison of two players.
    pub mod compare;

    pub mod dashboard;
    pub mod home;

    /// `/transactions` — King.8.2. League moves feed for the active
    /// season. Uses `load_transactions_with_fallback` so the handler
    /// works against bundled snapshots, installed bundles, OR a
    /// fetched snapshot (priority: snapshot store → embedded →
    /// installed bundle).
    pub mod transactions;

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
    pub mod scores;

    /// `/playoffs` — King.7.2. Bracket view, bundled fallback.
    pub mod playoffs;

    /// `/schedule` — King.7.3. Team-season schedule view.
    pub mod schedule;

    /// Phase Foster.2 — `/favorites` HTML route.
    pub mod favorites;
    pub mod favorites_data;

    /// Phase Conn Smythe C.3 — `/game/:id` per-game live detail.
    pub mod game;

    /// `/season-type/:kind` — UX.E. Mutates `WebState.config.active_season_type`
    /// to "regular" or "playoff" and 303-redirects back to the page
    /// the user came from (Referer header), defaulting to `/`.
    pub mod season_type;

    /// `not_found` — Sasq.7. Friendly 404 page with a player search
    /// input, replacing axum's bare default. Wired as the router's
    /// `.fallback(...)`, so any unmatched path lands here with the
    /// requested URI surfaced for context.
    pub mod not_found;
}
