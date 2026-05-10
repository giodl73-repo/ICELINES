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
        .filter(|r| r.starts_with('/') || r.contains("://127.0.0.1") || r.contains("://localhost"))
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
