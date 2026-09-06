---
skill: roles-check
topic: weekly-review-calibration
date: 2026-09-05
roles_used: [hart, keel, tape, forge, pace, bench, edge, wire, scout, glass, crest, broadcast]
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Weekly Review and Calibration — `.roles` Review

## Artifact

- Type: implementation plan
- Reviewed: `design/plans/2026-09-05-weekly-review-calibration.md`
- Domains: fantasy decision modeling, SQLite audit history, quantitative
  assessment, privacy, CLI, TUI, Web, accessibility, and testing

## Role selection

All installed roles are relevant. HART, KEEL, FORGE, and WIRE cover the model,
shared service, Rust, and schema boundaries. TAPE and SCOUT cover whether the
observed fantasy facts retain their hockey meaning. PACE governs error and
calibration claims. BENCH and EDGE cover proof and adversarial histories. GLASS,
CREST, and broadcast cover the three user-facing review projections.

## Findings

### HART

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| H1 | “Latest valid leaf” is ambiguous when execution, points, matchup, and reserve observations have independent correction histories. | P2 | Typed outcome contract | Reduce one effective leaf per typed observation lane, then compose the review. |
| H2 | The selected alternative is part of frozen identity, but decoder behavior for unsupported projection schemas needs a type-level result. | P2 | Review contract | Use a version-dispatched decoder returning `Decoded`, `Unsupported`, or `Invalid`; never partially guess. |
| H3 | Calibration groups do not distinguish no-move, skater-only, goalie-only, and mixed sequences. | P3 | Calibration summary | Derive a frozen decision lane and include it in comparability identity. |

### KEEL

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| K1 | TUI integration is named but its state-loading seam and refresh behavior are not explicit. | P2 | Architecture / TUI | Add one fetch-owned read service used by CLI, TUI, and Web; TUI loads on entry/refresh and does not cache a second projection. |
| K2 | The JSON contract changes from a raw array, which can break scripts despite a new schema. | P2 | Compatibility | Add a documented compatibility transition or explicit legacy flag; do not silently change default machine output. |
| K3 | Delivery commits can strand consumers between persistence and new view types. | P3 | Delivery slices | Keep each slice build-green and add the new service before switching adapters. |

### TAPE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| T1 | Manager-supplied fantasy points are assertions, not independently verified facts. | P2 | Source and trust | Serialize source kind, observed time, completeness, and optional evidence reference on every observation. |
| T2 | “Actual usable starts” could be mistaken for scheduled games. | P2 | Typed outcome contract | Define usable starts as league-legal active-slot assignments; manual values must be labeled asserted until derivation exists. |
| T3 | A matchup result without the final scored totals loses useful verification context. | P3 | Typed outcome contract | Allow optional user/opponent final totals and retain their source. |

### FORGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| F1 | String values for execution, source, and assessments permit invalid states. | P2 | Typed outcome/review contracts | Use enums with serde names and typed validation errors in core. |
| F2 | Persistence currently accepts arbitrary kind plus arbitrary JSON. | P2 | Persistence and service | Validate the v1 envelope before insert and keep arbitrary legacy rows on a separate opaque path. |
| F3 | The plan correctly rejects N+1 reads but needs an explicit batched repository method. | P3 | Performance | Add `list_decision_outcomes_for_decisions` with a bounded input/query. |

### PACE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| P1 | The ±1 point aligned band is a product threshold, not statistical calibration. | P2 | Review contract | Name it `display_alignment_tolerance`, serialize it, and forbid confidence-language around it. |
| P2 | Five observations are too few for automatic retuning. | P2 | Calibration summary | Keep five as descriptive-display readiness only and hard-block automatic retuning in v1. |
| P3 | Metric formulas and treatment of missing rows need exact definitions. | P3 | Calibration summary | Specify denominators and formulas; omit metrics when comparable `n=0`. |

### BENCH

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| B1 | Correction reduction needs branch, cycle, missing-parent, and cross-decision fences. | P2 | Verification matrix | Add explicit L0/L1 tests for every malformed graph and deterministic leaf selection. |
| B2 | Hand-calculated metric fixtures are required to catch sign inversions. | P2 | Verification matrix | Document exact expected bias, MAE, and RMSE arithmetic in tests. |
| B3 | Privacy parity needs negative assertions, not just positive rendering tests. | P3 | Parity | Assert rationale/notes are absent from default CLI JSON, TUI buffer, HTML, and API bytes. |

### EDGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| E1 | Two corrections can branch from one parent, leaving no unique effective row. | P2 | Typed outcome contract | Reject a second child for the same observation lane and corrected parent. |
| E2 | An observation can arrive before the frozen week ends or before the decision was evaluated. | P2 | Identity and time | Permit provisional execution facts but gate final value/matchup observations by time and label early rows incomplete. |
| E3 | Week filtering can be wrong around league timezone boundaries. | P3 | Identity and time | Filter on frozen `week_start`, never derive it from `observed_at` or UTC week. |

### WIRE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| W1 | The outcome envelope needs its schema field inside the stored JSON. | P2 | Typed outcome contract | Require `schema: fantasy_decision_outcome.v1` and validate exact supported versions. |
| W2 | Idempotency material is underdefined and could change with timestamps or private notes. | P2 | Typed outcome contract | Fingerprint normalized public material plus decision/lane; exclude insertion time and private note. |
| W3 | Unsupported legacy payloads should not make the entire endpoint a 500. | P3 | Degradation | Return typed per-item warnings and continue rendering supported items. |

### SCOUT

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| S1 | A matchup win is team-level and cannot establish that one pickup was good. | P2 | Review contract | Keep matchup result separate and prohibit causal language. |
| S2 | Goalie streams and skater quiet-night pickups have materially different opportunity semantics. | P2 | Calibration summary | Derive and stratify a decision lane; never pool goalie-only and skater-only error. |
| S3 | Active points alone can miss the value of protecting a goalie minimum or acquisition reserve. | P3 | Review contract | Surface constraint satisfaction and reserve observations independently from points. |

### GLASS

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| G1 | A season review can become an unreadable wall of metrics. | P2 | CLI/TUI | Lead with three assessment labels and one recovery/action; place arithmetic in detail. |
| G2 | Unknown and insufficient are first-class states, not blank cells or zeros. | P2 | Review contract | Use explicit text markers on every surface; never color alone. |
| G3 | The TUI plan does not state how many review items fit at 80 columns. | P3 | TUI | Show a compact latest-item summary at 80 columns and expand metrics at 120. |

### CREST

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| C1 | The plan names content but not a visual hierarchy. | P2 | TUI/Web | Compose context → assessment trio → evidence → next action, with restrained status styling. |
| C2 | Three similarly weighted labels can feel like dashboard clutter. | P3 | Review contract | Make process the primary label and result/projection supporting evidence. |
| C3 | Empty and partial review states need designed copy. | P3 | Degradation | Give each state a short explanation and exact recovery command rather than generic panels. |

### broadcast

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BR1 | The proposed Web URL includes week but omits the season filter promised by the contract. | P2 | Web | Support sticky `league`, `week`, and `season`; reject conflicting filters with a typed 400. |
| BR2 | Private changing data needs response-level cache discipline on success and error paths. | P2 | Web | Apply `Cache-Control: no-store` to HTML/JSON success, empty, and error responses. |
| BR3 | Semantic HTML alone does not guarantee narrow-screen usability. | P3 | Web | Use summary articles and horizontally contained detail tables with visible focus and no-JS links. |

## Synthesis

Roles reviewed: 12
P1 blockers: 0 | P2 issues: 24 | P3 notes: 12

**Verdict: APPROVED-WITH-CONDITIONS**

**Top finding:** Independent observation lanes need independent, unambiguous,
append-only correction chains.

**Cross-role consensus:** HART, TAPE, EDGE, WIRE, and BENCH agree that source,
time, completeness, idempotency, and correction semantics must be frozen before
code. PACE, SCOUT, GLASS, and CREST agree that the product must show separate
process/result/projection axes without causal or confidence overclaiming.

## Amendments required

1. Replace the single effective-outcome concept with typed observation lanes,
   normalized material fingerprints, and linear per-lane correction chains.
2. Make methodology explicit: serialize the display tolerance, stratify goalie,
   skater, mixed, and no-move decisions, define metric denominators, and keep
   automatic retuning blocked.
3. Preserve compatibility and privacy across one shared service: add a legacy
   JSON transition, sticky Web filters/no-store behavior, compact TUI hierarchy,
   and negative privacy parity tests.

## Implementation disposition

Rechecked after implementation on 2026-09-05. All three required amendments
are satisfied. In particular, provisional value observations are excluded from
calibration, final value/matchup observations are time-gated against the frozen
league week, legacy database rows remain nullable/opaque, and public Web/TUI
paths request no private fields. Verdict remains **APPROVED-WITH-CONDITIONS**,
with the conditions now discharged and no P1 blocker found.
