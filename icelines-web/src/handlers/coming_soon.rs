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
