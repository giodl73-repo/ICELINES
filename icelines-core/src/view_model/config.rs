use serde::{Deserialize, Serialize};

use crate::season_stats::SeasonType;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeasonTypeMutationIntent {
    pub active_season_type: String,
    pub redirect_to: String,
}

impl SeasonTypeMutationIntent {
    pub fn resolve(kind: &str, referer: Option<&str>) -> Self {
        let active_season_type = SeasonType::parse_lossy(kind).label().to_string();
        let redirect_to = safe_redirect_from_referer(referer).unwrap_or_else(|| "/".to_string());

        Self {
            active_season_type,
            redirect_to,
        }
    }
}

fn safe_redirect_from_referer(referer: Option<&str>) -> Option<String> {
    let referer = referer?;
    if is_safe_relative_path(referer) {
        return Some(referer.to_string());
    }

    let (_, after_scheme) = referer.split_once("://")?;
    let (host, path) = match after_scheme.split_once('/') {
        Some((host, path)) => (host, format!("/{path}")),
        None => (after_scheme, "/".to_string()),
    };
    if is_local_host(host) && is_safe_relative_path(&path) {
        Some(path)
    } else {
        None
    }
}

fn is_local_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let host_name = if let Some(value) = host.strip_prefix('[') {
        value.split_once(']').map(|(addr, _)| addr).unwrap_or("")
    } else {
        host.split(':').next().unwrap_or("")
    };
    matches!(host_name, "127.0.0.1" | "localhost" | "::1")
}

fn is_safe_relative_path(path: &str) -> bool {
    path.starts_with('/') && !path.starts_with("//")
}
