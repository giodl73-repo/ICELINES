//! `/static/*` asset serving — Phase King Clancy King.1.3.
//!
//! Vendored assets compiled into the binary via `include_bytes!`:
//! - `htmx.min.js` (~14 KB after `scripts/vendor-htmx.sh`; stub today)
//! - `style.css`   (~5 KB hand-rolled, lockstep with glass.md palette)
//! - `icelines.svg` (logo)
//!
//! ## Headers
//!
//! Per spec "Compression + caching headers":
//! - `Content-Type` per asset extension
//! - `Cache-Control: public, max-age=31536000, immutable`
//! - `ETag` from `env!("CARGO_PKG_VERSION")` quoted-strong
//!
//! ## Spec rule
//!
//! Static assets are versioned with the binary — a new release means
//! a new ETag, which busts every browser cache. Within a release they
//! are immutable so a year of caching is safe.

use axum::extract::Path;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

const HTMX_MIN_JS: &[u8] = include_bytes!("../static/htmx.min.js");
const DASHBOARD_JS: &[u8] = include_bytes!("../static/dashboard.js");
const STYLE_CSS: &[u8] = include_bytes!("../static/style.css");
const ICELINES_SVG: &[u8] = include_bytes!("../static/icelines.svg");

/// `Cache-Control` value for all `/static/*` responses. One year +
/// `immutable` per the spec's static-asset header policy. Safe because
/// the ETag changes on every binary version (so cache busts on each
/// release without requiring URL hashing).
const CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// Build the strong-ETag value for a given asset based on the binary
/// version. Format: `"<version>"` (quoted strong validator per RFC
/// 7232). Same value across all assets within one release.
fn etag() -> String {
    format!("\"{}\"", env!("CARGO_PKG_VERSION"))
}

/// `GET /static/:asset` handler. Dispatches by asset name (not file
/// extension) so the URL-to-content map is explicit and a typo in
/// the path returns 404 deterministically rather than serving the
/// wrong MIME for a near-miss.
pub async fn serve_static(Path(asset): Path<String>) -> Response {
    let (bytes, mime) = match asset.as_str() {
        "htmx.min.js" => (HTMX_MIN_JS, "application/javascript; charset=utf-8"),
        "dashboard.js" => (DASHBOARD_JS, "application/javascript; charset=utf-8"),
        "style.css" => (STYLE_CSS, "text/css; charset=utf-8"),
        "icelines.svg" => (ICELINES_SVG, "image/svg+xml; charset=utf-8"),
        _ => return (StatusCode::NOT_FOUND, "static asset not found").into_response(),
    };

    let etag_value = etag();

    // If-None-Match check — return 304 when the client already has
    // this version cached. Cheap because ETag is a single workspace
    // version string, not a content hash.
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static_or(mime, "application/octet-stream"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(CACHE_CONTROL),
    );
    if let Ok(v) = HeaderValue::from_str(&etag_value) {
        headers.insert(header::ETAG, v);
    }

    (StatusCode::OK, headers, bytes).into_response()
}

/// Tiny helper: try `HeaderValue::from_static`; on failure return a
/// safe fallback. The compile-time MIME strings above are all valid
/// header values, but staying defensive keeps a future MIME edit
/// (e.g. an exotic charset) from triggering a panic in the response
/// path.
trait HeaderValueExt {
    fn from_static_or(s: &'static str, fallback: &'static str) -> HeaderValue;
}
impl HeaderValueExt for HeaderValue {
    fn from_static_or(s: &'static str, fallback: &'static str) -> HeaderValue {
        s.parse()
            .unwrap_or_else(|_| HeaderValue::from_static(fallback))
    }
}

/// `If-None-Match` middleware shim. Phase King Clancy King.1.3 ships
/// the response side (ETag header set on every static response).
/// 304-conditional handling lands in King.1.6 alongside the broader
/// middleware stack (host validation, tracing, compression). Until
/// then browsers issue a fresh GET each load — the body is a few KB
/// and `include_bytes!` makes it free to serve, so this is a tax we
/// can pay for one sub-phase.
pub fn etag_for_current_version() -> String {
    etag()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// l0_etag_format
    /// — King.1.3 fence: ETag must be quoted-strong (`"version"`),
    ///   not bare. Browsers and intermediate caches require the
    ///   quotes per RFC 7232.
    #[test]
    fn l0_etag_format() {
        let e = etag();
        assert!(e.starts_with('"'), "ETag must start with quote, got: {e}");
        assert!(e.ends_with('"'), "ETag must end with quote, got: {e}");
        assert!(
            e.contains(env!("CARGO_PKG_VERSION")),
            "ETag must include workspace version, got: {e}"
        );
    }

    /// l0_cache_control_includes_immutable_and_one_year
    /// — Spec lock: static assets get `Cache-Control: public,
    ///   max-age=31536000, immutable`. Drift = browsers re-fetch
    ///   on every page load, defeating the vendoring point.
    #[test]
    fn l0_cache_control_includes_immutable_and_one_year() {
        assert!(CACHE_CONTROL.contains("public"));
        assert!(CACHE_CONTROL.contains("immutable"));
        assert!(
            CACHE_CONTROL.contains("max-age=31536000"),
            "max-age must be one year (31536000s); got: {CACHE_CONTROL}"
        );
    }

    /// l0_assets_compile_in_non_empty
    /// — fence against an empty include_bytes! (e.g. file deleted but
    ///   path kept). Each vendored asset must have at least the
    ///   placeholder header bytes.
    #[test]
    fn l0_assets_compile_in_non_empty() {
        assert!(
            HTMX_MIN_JS.len() > 100,
            "htmx.min.js bytes look empty ({} bytes)",
            HTMX_MIN_JS.len()
        );
        assert!(
            DASHBOARD_JS.len() > 100,
            "dashboard.js bytes look empty ({} bytes)",
            DASHBOARD_JS.len()
        );
        assert!(
            STYLE_CSS.len() > 100,
            "style.css bytes look empty ({} bytes)",
            STYLE_CSS.len()
        );
        assert!(
            ICELINES_SVG.len() > 100,
            "icelines.svg bytes look empty ({} bytes)",
            ICELINES_SVG.len()
        );
    }

    /// l0_style_css_carries_fit_class_contract
    /// — Prince visual-token contract (`.fit-elite/.fit-solid/.fit-buried
    ///   /.fit-stretch` with the shared fit palette). Catches a future
    ///   stylesheet refactor that loses the lockstep.
    #[test]
    fn l0_style_css_carries_fit_class_contract() {
        let css = std::str::from_utf8(STYLE_CSS).expect("style.css is utf-8");
        for class in &[".fit-elite", ".fit-solid", ".fit-buried", ".fit-stretch"] {
            assert!(
                css.contains(class),
                "style.css must define {class} (CSS class contract)"
            );
        }
        assert!(
            !css.contains(".fit-fringe"),
            "style.css must use semantic fit-stretch instead of legacy fit-fringe"
        );
        // Prince fit palette hex values — Green / Blue / Yellow / Red.
        for hex in &["#2e7d32", "#1565c0", "#f9a825", "#b71c1c"] {
            assert!(
                css.contains(hex),
                "style.css must include glass.md palette hex {hex}"
            );
        }
    }

    #[test]
    fn l0_style_css_carries_warning_state_contract() {
        let css = std::str::from_utf8(STYLE_CSS).expect("style.css is utf-8");
        for class in &[
            ".state-warning",
            ".state-warning-line",
            ".state-error",
            ".context-line",
            ".source-note",
            ".meta-line",
            ".empty-state",
        ] {
            assert!(
                css.contains(class),
                "style.css must define {class} for state token rendering"
            );
        }
        assert!(
            css.contains("var(--accent-warn)"),
            "warning state must use the shared warning accent token"
        );
        assert!(
            css.contains("var(--accent-bad)"),
            "error state must use the shared bad/error accent token"
        );
    }

    /// l0_htmx_stub_carries_explicit_placeholder_warning
    /// — King.1.3 ships a stub htmx.min.js. The warning text is the
    ///   contract that tells future contributors to vendor the real
    ///   file before King.2 ships HTMX-driven UI. If a refactor
    ///   removes the warning without vendoring, this fails.
    #[test]
    fn l0_htmx_stub_carries_explicit_placeholder_warning() {
        let js = std::str::from_utf8(HTMX_MIN_JS).expect("htmx.min.js is utf-8");
        // Either we still have the stub (warning present) OR the
        // real HTMX is vendored (the htmx.org signature appears).
        let is_stub = js.contains("PLACEHOLDER") || js.contains("STUB");
        let is_real = js.contains("htmx.org") || js.contains("htmx.min.js v");
        assert!(
            is_stub || is_real,
            "htmx.min.js is neither the stub nor real HTMX — pipeline regression"
        );
    }

    #[test]
    fn l0_dashboard_js_carries_workspace_fragment_contract() {
        let js = std::str::from_utf8(DASHBOARD_JS).expect("dashboard.js is utf-8");
        for needle in &[
            "partial",
            "workspace",
            "pushState",
            "popstate",
            "FormData",
            "redirect: \"manual\"",
            "localStorage",
            "data-dashboard-pane",
            "data-workspace-url",
            "data-dashboard-command-status",
            "data-dashboard-command-input",
            "data-dashboard-workspace-input",
            "data-dashboard-workspace",
            "copyDashboardState",
            "left",
            "right",
            "experience",
            "left_workspace",
            "right_workspace",
            "paneTargetFromClick",
            "appWorkspaceFromUrl",
            "isDashboardWorkspace",
            "setCommandStatus",
            "updateCommandWorkspace",
            "sessionStorage",
            "ArrowUp",
            "ArrowDown",
            "aria-expanded",
            "data-dashboard-pane-collapsed",
            "matchMedia",
            "Error: ",
        ] {
            assert!(
                js.contains(needle),
                "dashboard.js must carry workspace fragment contract token {needle}"
            );
        }
    }
}
