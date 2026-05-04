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
        // Coming-soon stubs for the section nav links on home.html.
        // Each mounts a real page (with the active-season header)
        // so clicks don't fail with a bare 404. Real handlers ship
        // in King.2+ and replace these mounts one by one.
        .route("/leaders", get(cs::leaders))
        .route("/goalies", get(cs::goalies))
        .route("/scores", get(cs::scores))
        .route("/playoffs", get(cs::playoffs))
        .route("/transactions", get(cs::transactions))
        .route("/fantasy", get(cs::fantasy))
        .route("/docs", get(cs::docs))
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

        pub async fn leaders(State(s): State<WebState>) -> Response {
            render(
                s,
                "Leaderboards",
                "King.2",
                "Top-N skater leaderboards by any of 30+ sort metrics, with the full \
                 boolean filter grammar (g>=50 AND hits>=200, etc.). Same data the CLI's \
                 `query leaders` exposes — just rendered as an HTML table with a filter form.",
            )
            .await
        }

        pub async fn goalies(State(s): State<WebState>) -> Response {
            render(
                s,
                "Goalies",
                "King.5",
                "Goalie leaderboard with save-percentage, GAA, quality starts, and the \
                 advanced report toggles. Mirrors `icelines query goalies`.",
            )
            .await
        }

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

        pub async fn docs(State(s): State<WebState>) -> Response {
            render(
                s,
                "Docs",
                "King.8",
                "The full COMMANDS.md command reference, rendered as HTML. \
                 Until then, run `icelines docs` from the terminal for the same content.",
            )
            .await
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
