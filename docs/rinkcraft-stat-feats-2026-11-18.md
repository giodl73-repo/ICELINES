# Statistical feats: experimental local integration

The shared Rust engine evaluates a versioned registry of 34 descriptive feats:
single statistics and conjunctions of two or three statistics. The complete
canonical inventory contains 108 statistics in eight categories. Rinkcraft's
research repository records another 33 candidate feats with explicit input gates;
those candidates are not implemented rules.

## Calculation and meaning

Reference groups are same-season forwards, defensemen and goalies, separately.
The authored pilot requires 20 skater games or 15 goalie games and 20 measured
peers for each metric. Midrank percentile is
`100 * (number below + 0.5 * number equal) / measured peers`.
Every condition must pass P60/P75/P90 for Bronze/Silver/Gold. Contextual risk
flags instead say Notable/Elevated/Marked. These thresholds and names are
experimental game-design choices, not validated capability standards.

General skater counts use count/GP; special-team output and goalie starts and
shutouts use explicitly labeled season counts. Save fraction uses saves/shots
faced. Missing values remain unavailable; measured zero is retained. Correlated
metrics do not count as independent evidence. Earned-feat counts are not levels
or player-quality scores.

## Integration

`rinkcraft_achievements.rs` owns evaluation and the canonical rule JSON. The NHL
parser validates complete report assemblies and joined actor/GP/season fields.
The WASM export and native profile builder use the same Rust evaluator. Native
canonical-row evaluation rejects mixed seasons and playoffs and preserves
missing optional data. Resident browser-v1 cohorts can lack realtime/penalty
inputs; unavailable rules stay unavailable. This change does not enlarge that
package or assert it contains a complete league reference.

Optional actor/year/type/group-scoped feat profiles populate Task Skills,
Playing Tendencies, Distinctive Features, Role Coverage and contextual flags in
Known Limitations. Compound patterns receive primary display priority. Remaining
earned, locked and unavailable choices remain browsable. Each feat exposes its
values, numerator/denominator, percentile, reference size, source links and hashes.
All-strength feats are labeled separately from the existing 5v5 Measured Record.

Generic totals do not populate Under Pressure or establish coaching assignments,
partner duties, actual power plays created, deliberate dumps, or giveaway-to-
retrieval sequences. Penalties drawn are distinct from non-offsetting PP creation;
PIM is distinct from penalty count. No 1–20 capability grade, class, simulation
modifier or causal interpretation was introduced.

## Captured pilot and verification

Public NHL 2024–25 regular-season summary, realtime, penalties and goalie-summary
reports were captured in 32 sorted pages: 920 skaters and 103 goalies. Early
limit-5000 probes were silently capped by the provider and are excluded from the
complete assembly. Per-page URLs and hashes are retained separately from the
assembled-input hash. Rule hashes identify canonical parsed definitions.

The local generated preview contains McDavid, Barkov, Fox, Makar and Shesterkin;
the evaluator produced 1,023 cohort profiles. An independent Python calculation
verified 45,698 conditions. Twelve adversarial WASM cases passed. Four focused
Rust engine tests, the native scope integration test and seven profile regression
tests passed. TypeScript compilation, 20 focused Node tests, native Askama check
and release WASM compilation passed. Five-profile UI checks covered 25 placements,
source/formula disclosures, missing-pressure safeguards, 30 unrated capabilities,
mobile layout and five single-page Letter exports. Theme checks covered all six
themes. No hosted or full native/browser-runtime acceptance is claimed.

Replay from the repository root (use retained inputs and compatible toolchain):

```text
cargo test -p icelines-core --lib rinkcraft_achievements --locked
cargo test -p icelines-core --test rinkcraft_feat_scope --locked
cargo check -p icelines-web --locked
cargo build -p icelines-wasm --target wasm32-unknown-unknown --release --locked
node tools/project_rinkcraft_achievements.mjs WASM_PACKAGE REPORTS_JSON RULES_JSON BASE_FIXTURES_JSON OUTPUT_DIRECTORY
node tools/prepare_rinkcraft_theme_studio.mjs OUTPUT_DIRECTORY/fixtures.json SITE_DIRECTORY
node icelines-browser/e2e/rinkcraft-achievements.mjs
```

The projector modifies only the optional achievements field and checks that prior
profile fields remain equal. The private dated artifact packet retains captures,
baseline, rules, WASM package, projections, numeric oracle, validation receipts and
the generated site. PUCK's authored scenario manifest records its exact paths,
hashes, commit and deterministic replay commands. Research definitions and the
readable active/candidate catalog live in Rinkcraft under `research/stat-feats/`
and `docs/research/stat-feat-catalog-01.md`.
