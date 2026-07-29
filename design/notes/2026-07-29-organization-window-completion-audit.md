# The Window — completion audit

**Date:** 2026-07-29

**Scope:** `WP-011`, `REQ-WINDOW-001..009`, the Window plan definition of done,
and the Organization Window release checklist

**Decision:** engineering-complete for evaluation; production release blocked
on two external evidence gates

## Authoritative state

- Implementation head `1b4ff230` passed all 26 jobs in PR #23 run
  `30488170619`; the PR has no review threads and remains draft.
- Focused core/fetch/CLI Window suites pass 56/5/22 tests. The full CLI/TUI
  binary partition passes 1,396 tests under the CI stack setting.
- `examples/organization-window-source-audit-partial-2026-07-28.json` reports
  14/16 complete required profiles, 0/32 rank-eligible organizations, and
  `production_ranked: false`.
- The two incomplete required methods are
  `development.organization_depth@organization_lineup_depth.v1` and
  `development.recall_depth@organization_recall_depth.v1`, both missing for all
  32 organizations.
- `examples/window-history/future-holdout-2025-26-to-2026-27-registration.json`
  seals a complete ranked 32-team feature board with no outcomes field. Its
  fingerprint is
  `17bab9aa568a7c3a5f788736c11671165a582da565641d1e24ed1fcfa95a68d1`
  and its outcome eligibility date is 2027-04-11.

## Definition-of-done audit

| Requirement | Evidence | Decision |
|---|---|---|
| Exact profile-readiness inventory | `EVID-WINDOW-INVENTORY`; 37-profile fixture reports 17 ready / 13 evaluation / 4 context-only / 3 blocked. | proved |
| Deterministic explainable official all-league Frame | Sealed `balanced.v1` all-32 evaluation board, canonical scorer replay, three-platform fingerprint matrix. | proved for evaluation; production rank held |
| Rank and delta gates | Incomplete required profiles withhold rank; movement/history reject unmatched identities, phases, methods, manifests, and cohorts. | proved |
| Point-in-time historical replay and honest calibration | Four observed-history origins, leakage audits, baselines, ablations, trial-noise state, and an explicitly inconclusive retrospective holdout. | proved; predictive promotion held |
| Scenario sensitivity uses sealed authorities | Typed trade, injury, development, camp, lineup, and team-season authority adapters with direct/cohort/unchanged attribution. | proved |
| Users can alter weights without changing hockey logic | Fingerprinted custom Frames validate registered methods, gates, weights, and family caps while reusing observations and scorer. | proved |
| Developers can add a profile without changing scorer/renderers | Registered-profile extension fixture builds and renders a new all-32 custom Frame through the shared contracts. | proved |
| Lifecycle evolution preserves sealed artifacts | Lifecycle amendment covers deprecation, demotion, supersession, retirement, exact-manifest holds, authoring refusal, and unchanged replay. | proved |
| Health, forecast, and timing claims stay separate | Separate contracts, labels, calibration states, and renderer copy prohibit treating a health percentile as Cup probability or inferred timing. | proved |
| CLI/TUI/Web/API/JSON/report/card parity | Shared loaded-board validator, exact-document and semantic golden parity, route tests, browser review, and UI-neutral cards. | proved |
| Correct current/historical cohorts | Current board contains all 32 canonical teams; historical origins use complete season-canonical leagues and preserve franchise continuity. | proved |
| Specs, plan, commands, parity, VTRACE, and release docs match the build | Final lifecycle/holdout close review, current test counts, current CI run, README discovery section, and two explicit release gates. | proved after this closeout change |

## Open product gates

1. Acquire and review real target-season affiliate organization, assignment,
   waiver, final development-rule, score/readiness, and roster facts; lower the
   completed league artifact through the existing pipeline; require 16/16
   profiles and 32/32 ranked organizations without proxies.
2. After 2027-04-11, score the preregistered 2026-27 holdout exactly once with
   final standings and retain the result whether it passes, fails, or is
   inconclusive.

Neither gate has a remaining scorer, adapter, schema, renderer, CLI, or test
harness dependency. Source silence, prior-season assignment, camp membership,
or known outcomes cannot be substituted for the required evidence.

## Resume triggers

- Run `affiliate-facts-status --require-ready` when target-season AHL
  transactions, waiver results, assignments, and the final public rule
  authority are available; then use the existing league projection-input and
  package-refresh commands.
- Run `window-holdout-score` only on or after 2027-04-11 with the registered
  target standings snapshot. The command will reject an early, mismatched, or
  contaminated result.
