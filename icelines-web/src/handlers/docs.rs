use crate::state::WebState;
use crate::templates::DocsTemplate;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::Season;
use icelines_core::{DocsView, ViewContext, ViewWindow};
use std::sync::OnceLock;

/// COMMANDS.md is embedded at compile time. Same source the
/// CLI's `icelines docs` subcommand reads — no drift.
const COMMANDS_MD: &str = include_str!("../../../COMMANDS.md");

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
    let (active_label, context) = {
        let cfg = state.config.read().await;
        let season = cfg
            .active_season
            .parse::<u32>()
            .map(Season)
            .unwrap_or(Season(0));
        (
            cfg.active_label.clone(),
            ViewContext::new(ViewWindow::new(
                season,
                super::leaders::parse_season_type(&cfg.active_season_type),
            )),
        )
    };
    let view = DocsView::rendered(
        context,
        "COMMANDS.md",
        "IceLines Commands",
        COMMANDS_MD,
        rendered(),
    );
    let tmpl = DocsTemplate {
        active_label,
        rendered_html: view.rendered_html,
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
