//! `WebConfig` — minimal config slice for `WebState`.
//!
//! The full `Config` lives in `icelines-cli` (consumer crate) and
//! cannot be imported from `icelines-web` without inverting the
//! crate dependency chain. King.1.x patch (broadcast review) advances
//! the active-(season, season_type) plumbing from King.6 → King.1.4 by
//! defining a tiny independent slice here that the askama base
//! template can render.
//!
//! Long-term plumbing options (deferred to King.6 — Reports overlay
//! sub-phase, where the cross-surface config-mutation contract lives):
//!
//! 1. **Move `Config` to `icelines-core`** — proper fix. Inverts the
//!    today's "config-is-cli-concern" assumption. Touches every
//!    `Config::load()` callsite (~15 in icelines-cli).
//! 2. **Pass a serializable snapshot** — `icelines-cli` builds a
//!    snapshot at boot, hands it to `icelines-web`. King.6's PATCH
//!    `/api/v1/reports` writes back through a callback. Smaller blast
//!    radius but two sources of truth.
//! 3. **Define `Config` trait in `icelines-core`, impl in `cli`** —
//!    most flexible, most boilerplate.
//!
//! For King.1.x patch we ship (2) in skeleton form: this `WebConfig`
//! struct holds the read-side; King.6 wires the write-side via a
//! callback when it lands the Reports overlay.

use serde::{Deserialize, Serialize};

/// Active-season slice the web layer needs at render time.
///
/// Today this is constructed by `commands::serve` (King.1.5) from the
/// CLI's `Config::load()` result. King.6 will extend the type with
/// the full `ReportToggleSet` so the Reports overlay can render.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebConfig {
    /// Active season in `YYYYZZZZ` form (e.g. `"20252026"`).
    pub active_season: String,
    /// Active season type (`"regular"` or `"playoff"`).
    pub active_season_type: String,
    /// Human-friendly label rendered in the active-(season, type)
    /// header on every page (`l1_html_each_route_has_active_season_header`
    /// fence). Pre-formatted by the constructor so handlers don't
    /// hold the `Config` lock for a string format.
    pub active_label: String,
}

impl Default for WebConfig {
    /// Default to the current season, regular. Used by tests and as a
    /// safe boot value before `commands::serve` overwrites with the
    /// real config-derived value.
    fn default() -> Self {
        let active_season = icelines_core::CURRENT_SEASON_STR.to_owned();
        let active_season_type = "regular".to_owned();
        Self {
            active_label: format_label(&active_season, &active_season_type),
            active_season,
            active_season_type,
        }
    }
}

impl WebConfig {
    pub fn new(active_season: impl Into<String>, active_season_type: impl Into<String>) -> Self {
        let active_season = active_season.into();
        let active_season_type = active_season_type.into();
        Self {
            active_label: format_label(&active_season, &active_season_type),
            active_season,
            active_season_type,
        }
    }
}

/// Format a `YYYYZZZZ` season + type into the header label used by
/// every page (e.g. `"2025-26 · Regular"`). Defensively returns the
/// raw string if the season doesn't parse.
fn format_label(season: &str, season_type: &str) -> String {
    if season.len() != 8 {
        return format!("{season} · {season_type}");
    }
    let yy_start = &season[2..4];
    let yy_end = &season[6..8];
    let pretty_type = match season_type {
        "regular" => "Regular",
        "playoff" => "Playoff",
        other => other,
    };
    format!("{yy_start}-{yy_end} · {pretty_type}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// l0_active_label_format
    /// — broadcast finding: every page's sticky header shows the
    ///   active (season, season_type). Lock the format here so the
    ///   askama template can rely on it.
    #[test]
    fn l0_active_label_format() {
        assert_eq!(
            WebConfig::new("20252026", "regular").active_label,
            "25-26 · Regular"
        );
        assert_eq!(
            WebConfig::new("20242025", "playoff").active_label,
            "24-25 · Playoff"
        );
    }

    /// l0_default_uses_current_season
    /// — Default boot value should track `CURRENT_SEASON_STR` so a
    ///   freshly-spawned WebState is never stuck on a stale season.
    #[test]
    fn l0_default_uses_current_season() {
        let cfg = WebConfig::default();
        assert_eq!(cfg.active_season, icelines_core::CURRENT_SEASON_STR);
        assert_eq!(cfg.active_season_type, "regular");
    }

    /// l0_malformed_season_falls_back_to_raw
    /// — defensive: if `commands::serve` somehow hands us a
    ///   non-`YYYYZZZZ` string, we don't panic; we render it raw.
    #[test]
    fn l0_malformed_season_falls_back_to_raw() {
        let cfg = WebConfig::new("not-a-season", "regular");
        assert!(cfg.active_label.contains("not-a-season"));
    }
}
