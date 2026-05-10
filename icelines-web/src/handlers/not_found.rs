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
