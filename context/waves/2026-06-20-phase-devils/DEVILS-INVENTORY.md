# Phase Devils Inventory

## Current dashboard visual QA posture

| Area | Current state | Devils disposition |
|---|---|---|
| Capture harness | `scripts/web-dashboard-capture.ps1` builds release CLI, starts `icelines --no-live serve`, waits for `/dashboard`, and uses installed Edge/Chrome headless screenshots. | Reuse this path unless a stronger local browser runner is already available. |
| Capture coverage | Pulse 02 expands the matrix to home/leaders/goalies/poach desktop, favorites/watchlist/schedule tablet, and fantasy/team-season/player mobile. | Treat this as representative workspace coverage, not exhaustive coverage of every dashboard workspace or every browser engine. |
| Artifact validation | The script currently checks file creation only. | Add automated checks for dimensions, nonblank pixels, and route/page readiness before promoting the claim. |
| Responsive proof | Islanders records selected desktop/mobile captures but not exhaustive overflow checks. | Treat responsive proof as partial until the matrix covers representative breakpoints and failure reporting. |
| Focus/touch proof | No live keyboard, focus order, touch, or pointer interaction proof is recorded. | Add focused checks only if feasible with local tooling; otherwise preserve a durable deferral. |
| Surface matrix wording | `design/specs/surface-parity.md` says selected capture evidence exists, but full visual QA remains future work. | Update only after Devils evidence exists; do not promote during planning. |

## Risks to avoid

- Treating screenshots as accessibility proof.
- Treating Edge/Chrome coverage as every browser engine.
- Treating selected workspaces as exhaustive dashboard coverage.
- Hiding mobile overflow by using cropped screenshots without automated checks.
- Letting browser proof mutate favorites, watch rules, admin, or cache state
  through GET navigation.

## Pulse map

1. Plan and inventory.
2. Capture matrix harness. Result: passed 2026-06-20 with a representative
   desktop/tablet/mobile matrix.
3. Artifact validation.
4. Responsive/focus decision.
5. Closeout and surface-matrix claim.
