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
    let intent = icelines_core::SeasonTypeMutationIntent::resolve(
        &kind,
        headers.get(header::REFERER).and_then(|h| h.to_str().ok()),
    );
    {
        let mut cfg = state.config.write().await;
        let new_cfg = WebConfig::new(cfg.active_season.clone(), &intent.active_season_type);
        *cfg = new_cfg;
    }
    (
        StatusCode::SEE_OTHER,
        [(header::LOCATION, intent.redirect_to)],
    )
        .into_response()
}
