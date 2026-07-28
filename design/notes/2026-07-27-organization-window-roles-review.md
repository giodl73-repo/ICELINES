# The Window — role review

**Date:** 2026-07-27
**Reviewed:**

- `design/specs/organization-window.md`
- `design/plans/2026-07-27-organization-window.md`

**Role source:** `.roles/ROLE.md` plus HART, KEEL, TAPE, FORGE, PACE,
BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and broadcast.

## Verdict

**GO WITH CHANGES — all review findings are incorporated in the draft.**

The architecture is sound after revision: typed profile observations feed a
sealed declarative Frame, hierarchical core scoring produces one complete
league board, and all surfaces consume the same UI-neutral documents. Weight
customization does not mutate hockey logic; method changes create new versions.

The initial review found 5 blockers, 5 warnings, and 2 notes. All 12 items were
applied before this verdict.

## Findings and applied fixes

| Role | Severity | Triggering draft text | Finding | Applied fix |
|---|---|---|---|---|
| HART | BLOCKER | “organization / season / season_type / as_of / horizon” | A free-form team abbreviation would not fully identify a historical or relocated organization, and the axes had to survive every cache and artifact. | Added typed season-aware organization identity/version, join validation, and the complete cache key. Composite state stays outside `StatsRepository`. |
| KEEL | WARNING | “older saved boards remain readable or fail” | Cross-version behavior was an outcome but not a contract. | Added exact major-version support, additive-field defaults, refusal, immutable migration, and method-version rules. |
| TAPE | BLOCKER | “organizations[32]” | A hardcoded 32-row schema is wrong for pre-expansion historical replay and can conceal an incomplete source cohort. | Replaced it with expected season-canonical organizations; current NHL runs require 32, historical runs require that season's complete catalog. Added profile cohort gates. |
| FORGE | BLOCKER | “deterministic canonical fingerprints” | Floating-point, map-order, locale, and negative-zero behavior were underspecified and could produce cross-platform fingerprint drift. | Specified canonical serialization, finite-number validation, normalized negative zero, stable ordering, and cross-platform golden vectors. |
| PACE | BLOCKER | “contribution_cap” | Runtime clipping or ambiguous per-profile caps would make the formula non-reviewable. Multiple horizons also risked one universal weight set. | Replaced clipping with manifest-time signal-family weight caps and made each Frame own one primary decision horizon. Added explicit degenerate-cohort behavior. |
| BENCH | WARNING | “Required tests include…” | The test list needed source assembly, schema compatibility, platform fingerprints, and current-vs-historical team-count fences in addition to pure scoring tests. | Expanded W1/W2/W8/W9 gates and retained L0/L1/L2, parity, no-network, historical replay, and package verification. |
| EDGE | BLOCKER | “weights sum to 1 within a small tolerance” | NaN, infinity, all-zero budgets, no eligible teams, zero-variance cohorts, and incomplete team sets could escape ordinary happy-path validation. | Added fail-closed numeric validation, explicit zero-variance neutral handling, minimum cohorts, incomplete-board rank withholding, and degenerate-budget rejection. |
| WIRE | WARNING | “unsupported schema or method version: hard error” | The boundary needed a compatibility/migration matrix and source-artifact validation before scoring. | Added saved-document compatibility, immutable migrations, upstream schema/version checks, and provider dependency declarations. |
| SCOUT | WARNING | “Development system — open jobs and recalls” | A weak NHL roster can create opportunity that looks like excellent development; prospect strength, conversion, and open roster spots must remain distinct. | Kept prospect strength, conversion, NHL opportunity, deployment, and current roster quality in separate signal families and explicitly barred rewarding weakness merely for open jobs. |
| GLASS | NOTE | “32-row board … dozens of profiles” | Showing every profile at once would bury the decision and make multi-horizon state ambiguous. | Board stays compact; detail drills into panes/lines/evidence, selected horizon is explicit, and narrow terminal/browser behavior is gated. |
| CREST | NOTE | “overall score” | A giant gauge would make an opaque master number the product and push evidence below the fold. | Explicitly rejected the giant-gauge treatment in favor of a hockey-native board and pane hierarchy with screenshot review. |
| broadcast | WARNING | “--manifest file” plus Web routes | A local file selection cannot become a stable bookmarkable Web GET state; immutable artifacts also need suitable HTTP caching. | Web exposes registered Frame IDs/fingerprints, keeps all context in the URL, adds semantic HTMX/no-JS behavior, and plans ETag/conditional GET for fingerprinted JSON. |

## Reconciled tensions

### PACE versus SCOUT

The scorer owns explicit quantitative rules; hockey context cannot override a
measured value after the fact. Context may remain evidence-only or enter through
a separately calibrated, versioned profile. This preserves both statistical
auditability and hockey meaning.

### GLASS/CREST versus PACE

The compact board may summarize, but the sealed document retains raw values,
weights, confidence, coverage, and methodology. Detail and The Insider expose
the full explanation without forcing every number into the first screen.

### KEEL versus CREST/broadcast

All surfaces consume one board and one card projection. TUI, CLI, Web, and
reports may compose that material differently for their medium, but none may
change scoring, focus before sealing the league cohort, or lose active context.

### Extensibility versus compatibility

Users alter weights through validated Frames. Developers add typed providers
and descriptors. Existing profile methods and sealed boards remain immutable;
new formulas receive new method versions and historical comparison requires an
explicit bridge.

## Implementation review gates

The roles must review again at four boundaries:

1. W0 inventory and readiness classification — HART/KEEL/TAPE/SCOUT/PACE.
2. W1-W2 contracts and scorer — HART/FORGE/PACE/BENCH/EDGE/WIRE.
3. W3-W7 real all-league board and calibration — all core roles.
4. W8-W9 surfaces and release — KEEL/BENCH/GLASS/CREST/broadcast plus final
   all-role closeout.

Review complete. 0 unresolved blockers, 0 unresolved warnings, 0 unresolved
notes.

## Implementation checkpoint review

The roles reviewed the implemented baseline against the plan after the first
contracts, scorer, adapters, history/scenario/calibration primitives, and
CLI/TUI/Web/card projections landed.

**Verdict: GO FOR CONTINUED IMPLEMENTATION; NOT YET A FULL RELEASE.**

| Role | Checkpoint finding | Plan disposition |
|---|---|---|
| HART | The board identity and axes are sealed, but bridge/rebase remains a first-class missing contract rather than a renderer concern. | Keep W5 open and require an immutable bridge artifact. |
| KEEL | Shared board/card values now reach CLI, TUI, Web, and JSON; report output and full-board interaction are not yet converged. | Keep W8 partial and add parity evidence before release. |
| TAPE | The saved all-32 artifact is honest but source-incomplete; absence must never be inferred as zero or average. | Keep it labeled evaluation-only and rank-withheld. |
| FORGE | Core owns formulas and renderers project typed documents, preserving crate boundaries. | Retain pure-core/no-I/O and thin-surface gates. |
| PACE | Deterministic scoring is testable; predictive language is still unsupported without rolling-origin evidence. | Keep W7 open and label the Frame descriptive/heuristic until calibrated. |
| BENCH | Focused core, TUI, and Web tests cover the present slice; report, compatibility matrix, full parity, and package gates remain. | Add them to W8-W9 closeout rather than treating compile success as release evidence. |
| EDGE | Required missingness and incompatible comparisons fail closed; alterations now need explicit lanes so saved artifacts cannot drift silently. | Added the extension and alteration protocol to the plan. |
| WIRE | Schemas and HTTP routes exist, but compatibility and migration require multi-version fixtures. | Keep W9 open with refusal and additive-compatibility cases. |
| SCOUT | The first Frame keeps prospect strength, conversion, opportunity, and NHL quality distinct; future Lines still require hockey reasonableness checks. | Require SCOUT on profile additions, formula changes, and Frame reweights. |
| GLASS | Focused cards have a usable hierarchy, but the 32-team board and partial/blocked recovery states need surface review. | Keep board/drilldown, 80-column, and accessibility gates in W8. |
| CREST | The product avoids a giant master gauge; screenshot-level polish across board, detail, and report is still unproven. | Require final screenshot review without moving logic into surfaces. |
| broadcast | Stable Frame URLs and ETags are present; bookmarkable as-of/view state, narrow layout, keyboard flow, and no-JS recovery need live review. | Keep browser acceptance work in W8-W9. |

### Reconciled decision

The implementation may continue from the existing baseline without redesigning
the core contracts. The next critical path is W5 bridge/history, W6 typed
scenario attribution, and W7 rolling-origin calibration. W8 can mature in
parallel against sealed fixtures, but no surface may promote an evaluation
artifact into a complete or predictive claim. W9 closes compatibility,
extension, parity, and release evidence.

Checkpoint review complete: 0 unresolved architecture blockers; 4 intentional
release blockers remain tracked in W5, W6, W7, and W8-W9.

### W9 trust-boundary checkpoint

The hardening pass now replays every loaded board through the canonical scorer
before CLI, card, Web, TUI, comparison, or calibration code may trust it. A
valid checksum is insufficient: manifest identity, cohort shape, numeric
bounds, raw observations, normalized values, aggregates, classifications,
drivers, blockers, and rank state must reconcile. BENCH/WIRE compatibility
fixtures also prove older movement and scenario documents deserialize when the
new bridge and attribution fields are absent.

Affected production targets pass strict Clippy, all nine Window schemas pass
Draft 2020-12 meta-validation, the saved board and both cards validate, and the
25-test core Window slice plus focused CLI/TUI/Web tests pass. The extension
fixture registers a new method, adds it to a custom Frame, and scores all 32
organizations without scorer or renderer changes. This closes the
implementation trust-boundary blocker, but does not close the four product
evidence blockers above: real checkpoint history, real scenario distributions,
real rolling origins/holdout evidence, and automated full cross-surface golden
parity remain.

The subsequent W9b pass added a fixed-hash Windows/Linux/macOS CI matrix,
offline Window release smoke, verified Windows archive/checksum/manifest,
dependency audit, and an optimized performance baseline. Live Edge review at
1440px, 900px, and 390px exposed and fixed missing board skip/caption/focus
semantics plus narrow fingerprint/card overflow. Five regenerated screenshots
then passed dimension/nonblank checks and manual hierarchy/containment review.
The subsequent keyboard run verified first-Tab skip links and Enter transfer to
the main focus target on both board and card routes. Reduced-motion emulation
reported no spinner animation and 0.01ms fallback durations; computed layout
reported no page overflow at 390px. The remaining visual gate is automated full
cross-surface golden parity, not first browser or interaction evidence. PR #23
then observed the canonical fixed hashes and replay test passing on all three
operating systems; that closes the platform-fingerprint observation, while the
real-history, package-matrix, and full-parity blockers remain explicit.
