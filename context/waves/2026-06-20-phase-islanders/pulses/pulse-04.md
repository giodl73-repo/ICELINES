# Phase Islanders Pulse 04 - Dashboard Selected Capture Proof/Fence

## Result

Passed with selected capture evidence. Dashboard workspace partial/browser proof
now has repeatable desktop/mobile screenshot artifacts, while full live-browser,
touch/focus, and exhaustive responsive coverage remain outside this pulse.

## Work completed

- Ran the existing capture harness:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\web-dashboard-capture.ps1
```

- The script built `target\release\icelines.exe`, launched
  `icelines --no-live serve --no-open --port 18988`, waited for `/dashboard`,
  and captured four headless Edge/Chrome screenshots under
  `dist\web-dashboard-captures\`.
- Captures produced:
  - `dashboard-leaders-desktop.png` at `1440x900`;
  - `dashboard-poach-desktop.png` at `1440x900`;
  - `dashboard-fantasy-mobile.png` at `390x844`;
  - `dashboard-team-season-mobile.png` at `390x844`.
- Updated `design/specs/surface-parity.md` to treat this as selected
  browser-render evidence rather than full visual QA.

## Validation

```powershell
powershell -ExecutionPolicy Bypass -File scripts\web-dashboard-capture.ps1
git diff --check
```

## Residual risk

The captures prove selected dashboard workspaces render nonblank at representative
desktop/mobile sizes. They do not claim exhaustive browser coverage, interactive
touch/focus behavior, every workspace, or every responsive overflow edge.

## Next pulse

Pulse 05 should roll up WP-009 cache-backed first-route evidence versus broader
workflow completion claims.
