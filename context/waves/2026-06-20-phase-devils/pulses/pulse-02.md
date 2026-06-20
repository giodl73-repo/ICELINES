# Phase Devils Pulse 02 - Capture Matrix Harness

## Result

Passed with a representative dashboard capture matrix. The capture harness now
records desktop, tablet, and mobile screenshots across multiple dashboard
workspace families while keeping artifact validation for the next pulse.

## Work completed

- Expanded `scripts/web-dashboard-capture.ps1` from four captures to ten:
  - desktop `1440x900`: home dashboard, leaders, goalies, poach;
  - tablet `900x1100`: favorites, watchlist, schedule;
  - mobile `390x844`: fantasy, team-season, player.
- Kept the existing offline proof path:

```powershell
icelines --no-live serve --no-open --port <port>
```

- Preserved the current script contract: build the release CLI unless
  `-SkipBuild` is passed, start the local server, wait for `/dashboard`, run
  installed Edge/Chrome headless, and fail when a screenshot file is not
  created.

## Validation

```powershell
powershell -ExecutionPolicy Bypass -File scripts\web-dashboard-capture.ps1
git diff --check
```

## Residual risk

This pulse expands representative coverage, but it still checks only screenshot
file creation. It does not validate dimensions, nonblank pixels, route readiness
inside each workspace, focus order, touch behavior, accessibility, or every
dashboard workspace.

## Next pulse

Pulse 03 should add automated artifact validation for expected file names,
dimensions, nonblank pixels, and a minimal page-readiness signal.
