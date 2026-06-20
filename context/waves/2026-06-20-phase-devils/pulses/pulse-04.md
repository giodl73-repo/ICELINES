# Phase Devils Pulse 04 - Responsive/Focus Decision

## Result

Passed with a bounded proof decision. Phase Devils keeps the current claim to
representative responsive capture evidence plus artifact validation, and keeps
keyboard focus order, pointer/touch behavior, and screen-reader behavior
deferred until a real browser automation gate exists.

## Decision

The current local harness uses installed Edge/Chrome headless screenshots and
PowerShell artifact checks. That is enough to prove:

- representative desktop, tablet, and mobile dashboard routes load the dashboard
  shell;
- expected screenshot artifacts are created;
- PNG dimensions match the requested viewport;
- sampled pixels are not blank.

It is not enough to prove:

- keyboard traversal order;
- focus restoration after pane toggles or command submissions;
- pointer/touch gestures;
- screen-reader output;
- every browser engine;
- exhaustive responsive overflow behavior across every workspace.

## Existing supporting evidence

- `scripts/web-dashboard-capture.ps1` covers the representative capture matrix.
- `icelines-web/tests/l1_router.rs` includes route-level dashboard shell,
  accessibility-token, URL allowlist, fragment, command redirect, and workspace
  embedding tests.
- `icelines-web/static/style.css` includes focus-visible and mobile scrolling
  affordance styles.

## Validation

```powershell
cargo test -p icelines-web --test l1_router l1_dashboard_shell_renders_no_js_regions
git diff --check
```

## Residual risk

Focus, touch, and screen-reader behavior remain manual or future automation
claims. Devils closeout should update `design/specs/surface-parity.md` to say
representative browser capture and artifact validation exist, while interaction
and accessibility breadth remain outside the claim.

## Next pulse

Pulse 05 should close Devils with the exact surface-matrix wording and no active
Devils pulse remaining.
