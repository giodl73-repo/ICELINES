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
use std::path::PathBuf;

pub use config::WebConfig;
pub use error::{Wants, WebError};
pub use state::WebState;

/// Optional configuration when constructing the router.
#[derive(Debug, Clone, Default)]
pub struct RouterConfig {
    /// Directory containing the built mkdocs site. When set AND the
    /// directory exists, it's mounted at `/site/*`. When the directory
    /// is missing, `/site` returns a friendly "run `icelines site
    /// build` first" page instead of a generic 404.
    pub site_dir: Option<PathBuf>,
}

/// Build the axum router for the IceLines web dashboard.
///
/// Routes mounted today:
/// - `GET /` — placeholder home (King.1.1)
/// - `GET /static/:asset` — vendored CSS / HTMX / logo (King.1.3)
/// - `GET /site/*` — mounted mkdocs static site (King.1.5b, optional)
///
/// Later sub-phases attach real surfaces (`/leaders`, `/player/:id`,
/// `/api/v1/*`, ...).
pub fn router(state: WebState) -> Router {
    router_with(state, RouterConfig::default())
}

/// Same as [`router`] but takes a [`RouterConfig`] for optional
/// extras like the mkdocs `/site/*` mount. Used by the `icelines
/// serve` driver when `--site-dir PATH` is supplied (or its default
/// `../fantasy-site` exists on disk).
pub fn router_with(state: WebState, cfg: RouterConfig) -> Router {
    let mut app = Router::new()
        .route("/", get(handlers::home::get_home))
        .route("/static/:asset", get(static_assets::serve_static));

    // Mount mkdocs site at /site/* if a build directory was provided.
    if let Some(site_dir) = cfg.site_dir.as_ref() {
        if site_dir.is_dir() {
            // ServeDir serves files from disk. fallback_index ensures
            // mkdocs-style pretty URLs (`/site/teams/SEA/`) resolve to
            // the matching `index.html` on disk.
            let svc = tower_http::services::ServeDir::new(site_dir)
                .append_index_html_on_directories(true);
            app = app.nest_service("/site", svc);
        } else {
            // Directory configured but missing — surface a helpful
            // page instead of a silent 404. Capture the path for the
            // closure.
            let path_str = site_dir.display().to_string();
            app = app.route(
                "/site",
                get(move || {
                    let p = path_str.clone();
                    async move { axum::response::Html(missing_site_html(&p)) }
                }),
            );
        }
    }

    app.with_state(state)
}

/// Friendly error page when `/site/*` is requested but the configured
/// site_dir doesn't exist on disk yet.
fn missing_site_html(path: &str) -> String {
    format!(
        "<!doctype html>\n\
         <html lang=\"en\"><head>\
         <meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>Site not built — IceLines</title>\
         <link rel=\"stylesheet\" href=\"/static/style.css\">\
         </head><body><main id=\"main\">\
         <h1>Site not built yet</h1>\
         <p>The mkdocs documentation site is mounted under \
         <code>/site/</code>, but the build directory is missing:</p>\
         <p><code>{}</code></p>\
         <h2>To build it</h2>\
         <pre>icelines site build</pre>\
         <p>Then refresh this page. The build is idempotent and \
         takes ~10–20 s.</p>\
         <p><a href=\"/\">← back to dashboard</a></p>\
         </main></body></html>\n",
        html_escape(path)
    )
}

fn html_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '<' => "&lt;".to_owned(),
            '>' => "&gt;".to_owned(),
            '&' => "&amp;".to_owned(),
            '"' => "&quot;".to_owned(),
            '\'' => "&#39;".to_owned(),
            other => other.to_string(),
        })
        .collect()
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
