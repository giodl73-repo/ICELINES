# Phase Devils Pulse 03 - Artifact Validation

## Result

Passed with automated artifact validation in the dashboard capture harness.

## Work completed

- Updated `scripts/web-dashboard-capture.ps1` so every matrix entry checks route
  readiness before screenshot capture.
- Route readiness now requires HTTP success and the dashboard shell marker
  `class="jaw-shell"`.
- Each generated PNG is validated for:
  - expected viewport dimensions;
  - sampled nonblank pixel diversity;
  - file existence.
- Preserved the existing headless Edge/Chrome and `icelines --no-live serve`
  proof path.

## Validation

```powershell
powershell -ExecutionPolicy Bypass -File scripts\web-dashboard-capture.ps1
git diff --check
```

## Residual risk

Artifact validation proves the generated screenshots are present, correctly
sized, and not blank for representative dashboard routes. It does not prove
semantic visual correctness, full responsive overflow behavior, keyboard focus
order, touch behavior, screen-reader accessibility, or every browser engine.

## Next pulse

Pulse 04 should decide whether to add focused keyboard/focus/mobile checks in
this phase or keep them explicitly deferred.
