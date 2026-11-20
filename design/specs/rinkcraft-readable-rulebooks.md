# Readable attribute valuation and feat rules

Owner request: explain the six major attributes and each feat's formula/category
in a readable character-sheet style. Preserve the compact profile. This change
implements the existing candidate attribute recipe and exposes qualification
rules; it does not release new player capability grades.

## Attribute model

The source is Rinkcraft's authored `research/attributes/registry.v1.json`, draft
0.1.0, SHA-256 `53dee7e65caa2efaa67c1268c4d63d33c246aeca0e563e08153a5b47dd3a7805`.
Its operational definitions are not an empirically calibrated NHL rating scale.
The compact implementation export is `rinkcraft_capability_rules.json`.

| Skater attribute | Required facets | Candidate weights |
| --- | --- | --- |
| Skating | Speed, acceleration, agility, transitions | 25% each |
| Handling | Reception, control, protection, pressure execution | 25% each |
| Passing | Delivery, range, timing, deception | 25% each |
| Shooting | Finishing, accuracy, release, velocity | 25% each |
| Reading | Recognition, anticipation, decision timing | One third each |
| Defending | Gap control, coverage, denial, recovery execution | 25% each |

Goalies retain separate Tracking, Positioning, Movement, Rebounds, Recovery and
Puck Play batteries, each with three required facets. Their benchmarks are not
interchangeable with skater scores. The shared math does not establish comparable
measurement or context support across positions.

The Rust core candidate helper requires already context-adjusted, calibrated
component quantiles and a supplied complete headline reference. It applies:

1. `z_j = inverse_normal(clamp(q_j, .005, .995))`.
2. `H = sum(weight_j * z_j)` using all required facets.
3. `Q = (below(H) + .5 * tied(H)) / reference_size`.
4. Linear interpolation of frozen anchors, half-up rounding, clamp to 1–20.

Anchors include P25→8, P50→10, P70→12, P90→16, P97→18, P99.5→20.
No D&D modifier, player level, class bonus, overall six-attribute sum or simulator
bonus is assigned. Equal domain weights are an authored normative candidate.

The helper rejects missing/duplicate facets, nonfinite/out-of-range quantiles,
unknown attributes and invalid references. Its minimum 20-reference input is a
mechanical math guard, not proof of adequate calibration. It cannot verify a
caller's reference context, completeness, source custody or component models.
Every result is `unvalidated_candidate`; it carries `proposed_display_score`,
not an authorized player grade. It supplies no claimed uncertainty interval.

Actual release still requires the registry's source, coverage, reliability,
context support, full-cohort calibration, joint uncertainty, held-out validation,
construct validation, weight sensitivity and independent methods review records.
Missing facets are never zero-filled, median-filled or reweighted away.
Existing annual percentile bands and feat ranks cannot populate this battery.

Clicking an attribute opens a readable rulebook: plain-language facets, weights,
measurement details, proposed anchors, exact formula and current evidence gaps.
The main player's score remains Unrated. The root rulebook JSON is included in
the public preview, theme-studio projection and browser build/offline shell.

## Feat rulebooks

The existing Rust feat outputs are unchanged. Opening a feat shows a compact
prerequisite/result preview; **Open feat rulebook** shows its category, AND rule,
actual values/percentiles, numerator/denominator arithmetic, tier ladder, scope,
exposure and reference details. Sources/limits remain in a separate disclosure.

All conditions use the same tier threshold: Bronze P60, Silver P75, Gold P90.
Caution bands are Notable/Elevated/Marked at those thresholds, describing event
volume rather than validated harm severity. Unknown future tiers are not guessed.
Rounded values never recompute eligibility. Experimental patterns keep their
single configured threshold and source-specific opportunity gates.

For example, the frozen Killorn 2024–25 Goal Output record is 19/82 = .231707...
goals per appearance, P61.4035 among 456 eligible forwards: Bronze. This places
the feat in Task Skills. It does not define a Shooting capability score.

## Validation and review boundary

Four focused Rust tests cover all 12 batteries, required components, invalid
inputs, frozen anchors, a 1,001-point monotonic sweep and an independently known
normal-quantile/empirical-reference case. Thirty-two Node tests passed, including
arithmetic, missing/zero values, unknown tiers, categories and rounded-boundary
non-awards. Browser acceptance exercises all 12 rulebooks, feat arithmetic and
category, caution bands, six mobile themes and single-page print with an expanded
feat. The public explorer's 20 player/season cases also passed.

Producer-applied lenses: methods (candidate versus validated score and uncertainty),
taxonomy (activity/output versus capability), defense/goalie (distinct full
batteries), evidence custody (registry fingerprint), and UX (short card, readable
rulebook). This is self-review, not independent expert review.

The existing 3,941 public player-season data files and cohort catalog are retained
unchanged; this is a rulebook/presentation update. Native/WASM live capability
assessment and real-player calibration are not claimed.

Published as public preview version 2 at
`https://rinkcraft-nhl-preview.ivoryorchid34.chatgpt.site`.
Static-source commit: `bb3258edbc141391acd39d6e5cb83017568e40b3`.
Saved version: `appgprj_6aff7eac4c1081919202e36a7730dc2a~appgver_ab4ba9c31ef481918895ba8212ed0d3d`.
Deployment: `appgdep_6aff8b7567088191b7127f44f284ae4e`, succeeded.
The full 12-rulebook browser check also passed on the deployed HTTPS origin.
File-by-file SHA-256 comparison against the version-1 deployment archive confirms
all 3,941 profile files and the cohort catalog exactly unchanged.
