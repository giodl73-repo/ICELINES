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
        use axum::extract::State;
        use axum::response::Html;

        /// `GET /` — placeholder home. King.1.4 swaps this for an
        /// askama-rendered template with the active-season header.
        ///
        /// King.1.x patch (broadcast review): the placeholder carries
        /// `<meta viewport>`, `<main>` landmark, and a skip-to-content
        /// link so the askama base template can be modeled on it
        /// without re-introducing those omissions. The viewport tag
        /// is non-negotiable for any page that might land on a tablet
        /// or phone via LAN-mode (`--bind 0.0.0.0`, King.1.6).
        pub async fn get_home(State(_state): State<WebState>) -> Html<&'static str> {
            Html(
                "<!doctype html>\n\
                 <html lang=\"en\">\
                 <head>\
                 <meta charset=\"utf-8\">\
                 <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
                 <title>IceLines</title>\
                 </head>\
                 <body>\
                 <a href=\"#main\" class=\"skip-link\">Skip to content</a>\
                 <main id=\"main\">\
                 <h1>IceLines</h1>\
                 <p>web dashboard skeleton — King.1.x.\
                 The real home page lands in King.1.4.</p>\
                 </main>\
                 </body></html>\n",
            )
        }
    }
}
