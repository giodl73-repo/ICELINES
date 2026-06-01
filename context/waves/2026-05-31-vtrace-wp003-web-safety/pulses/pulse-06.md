# Pulse 06: Browser Shell and Recovery Navigation

## Scope

`WP-003` selected Web/browser safety slice for the shared HTML shell,
no-JS affordance, viewport metadata, dashboard URL-addressable workspace copy,
and unknown-route recovery navigation.

The observed gap was documentation/evidence rather than a broad route mutation:
the shell already carried viewport metadata and a skip link, and the unknown-route
template already offered recovery links/search, but the no-JS contract was not
explicit in the rendered shell or fenced by route evidence.

## Change

- The shared base template now renders a `<noscript>` notice that states
  JavaScript is optional and directs users to links, forms, and full-page
  workspace URLs.
- The stylesheet includes a visible, responsive no-JS notice treatment.
- Route tests now assert `/dashboard` exposes viewport metadata, skip-link,
  no-JS shell copy, global navigation, and server-rendered URL-addressable
  workspace copy.
- Route tests now assert an unknown route returns a 404 with explicit recovery
  copy, compare search, and navigation links.

## Evidence

| Level | Evidence | Result |
|---|---|---|
| L0 | `cargo test -p icelines-web --test l1_router l1_html_shell_exposes_no_js_viewport_and_recovery_navigation -- --nocapture` | passed 2026-05-31 |
| L0 | `cargo test -p icelines-web --test l1_router l1_unknown_route_returns_404 -- --nocapture` | passed 2026-05-31 |
| L1 | `cargo fmt --check` | passed 2026-05-31 |
| L1 | `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings` | passed 2026-05-31 |

## Decision

`closed_with_risk` for this selected browser shell and recovery boundary.

The pulse proves the shared shell exposes no-JS/viewport/skip-link affordances
and unknown-route recovery in route-level HTML. It does not close full browser
launch, host/bind, URL-before-open, touch/focus interaction, or broader JSON-twin
inspection for `VAL-003`.
