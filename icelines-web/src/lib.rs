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
pub fn router(state: WebState) -> Router {
    Router::new()
        .route("/", get(handlers::home::get_home))
        .route("/static/:asset", get(static_assets::serve_static))
        .with_state(state)
}

mod handlers {
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
