# Source-scoped workload on Rinkcraft cards

Owner requests continued completion of the five contrast cards using public data.
This increment fills measured workload, not tactical duties or capability grades.

## Contract

`RinkcraftStatProfile.deployment_evidence` is optional and omitted when absent.
The shared Rust builder accepts one player/season/type with positive appearances
and exactly one skater or goalie payload. Skater EV+PP+SH must equal total TOI;
goalie starts cannot exceed appearances. Explicit zero is retained. A zero total
has no share denominator. Native canonical profiles build from normalized optional
TOI/goalie payloads; absent or incoherent payloads withhold workload.

Each row retains integer seconds, GP-derived unrounded seconds per appearance,
own-TOI share, and Rust display strings. Rounding happens after division. EV is
not exclusively 5v5. OT overlaps strength categories; it is not added to the sum.
Goalie TOI divides by appearances, including relief appearances, not starts.

The stateless `BrowserEngine.deployment_report` uses the NHL source parser:
bounded complete filtered payload, unique selected actor, exact echoed season,
report kind, declared game type, required nonnegative integer fields and total
reconciliation. The source URL must identify the official HTTPS report and its
season/game-type filters. SHA256 binds the exact supplied bytes. Source URL and
manifest declare game type; the response does not independently echo it. This
validation does not authenticate an arbitrary caller's claimed network capture.

The browser v1 package schema remains unchanged: skater TOI reports are still
not carried by that package. The explicit captured-report projection supplements
the local preview without replacing resident packages or their missing-report
disclosures. Native normalized report availability follows the existing loader.

## Rendering

Role Coverage shows measured PP/SH time per GP; goalie coverage points to its own
workload. Deployment shows compact TOI/appearance values. A disclosure preserves
seconds, minutes, GP, own-TOI shares, notes, source URL and raw capture hash. Native
catalog routes without raw capture metadata say the exact snapshot is unaudited.

Browser and native withhold a usage object if its actor, season, season type or
goalie/skater group differs from the displayed context. Unit/station remain
undocumented; no PP1/PP2, pair, F1/F2/F3, matchup, responsibility, effectiveness
or capability is inferred. Existing 5v5 numerical evidence remains unchanged.

## Inputs and outputs

- Authored public-source request manifest:
  `C:/src/PUCK/scenarios/league/2024-25/profile-contrasts/deployment-input.json`.
- Complete raw captures, four skaters/one goalie:
  `F:/src/PUCK-artifacts/research/2026-11-18-deployment-usage/raw`.
- Baseline: `target/compact-sheet/card-completion/fixtures.json`.
- Generated: `target/compact-sheet/deployment/fixtures.json` and receipts.
- Rinkcraft sources SRC-159–SRC-162; descriptive workload claim CLM-091.
  `docs/research/profile-workload-01.md` records methods and source limits.
  The immutable updated completion worklist retains all 110 required facets.

## Completed local verification

- Three core workload tests, three canonical-profile integration cases, seven
  existing profile regression tests, two source-parser tests with multiple
  rejection branches; native Askama compilation and release WASM build.
- TypeScript compilation and 19 focused Node tests, including actor/window/type/
  group gating, zero SH exposure and separation from capabilities.
- Actual WASM: five source-validated profiles and 13 rejection cases. All prior
  profile fields remain equal. Stateless projection leaves resident revisions
  unchanged.
- Independent raw JSON arithmetic oracle: 17 rows, GP, totals, shares, rounding
  and goalie starts. Native profile integration includes rounded-summary-rate
  disagreement, overlapping OT, relief appearances and missing/incoherent TOI.
- UI: five cards, 17 rows, five source/hash disclosures, mobile overflow and
  30 unrated attributes. All five PDFs independently verified by pypdf as one-page
  612×792pt Letter files containing workload/context labels.
- Existing theme acceptance: 48 desktop/24 mobile cases, six fictional theme
  print exports; factual-card acceptance: ten source-linked facts, five mobile
  cards and five actual-profile print exports. McDavid screenshot inspected.
- New Rust files pass rustfmt; diff whitespace checks pass. Rinkcraft foundation
  checks structural integrity, not empirical validity.

## Replay

Use the pinned Rust/TypeScript/wasm-bindgen tools. On this machine set TEMP/TMP
to the owned worktree's `target/windows-tmp` and CARGO_TARGET_DIR to
`F:/src/ICELINES/target`.

```powershell
cargo test -p icelines-core --lib rinkcraft_deployment --locked
cargo test -p icelines-core --test rinkcraft_deployment_profile --locked
cargo test -p icelines-core --lib rinkcraft_profile --locked
cargo test -p icelines-sources --lib nhl::deployment --locked
cargo check -p icelines-web --locked
cargo build -p icelines-wasm --target wasm32-unknown-unknown --release --locked
node icelines-browser/node_modules/typescript/bin/tsc --project icelines-browser/tsconfig.json
node --test icelines-browser/tests/rinkcraft-sheet.test.mjs icelines-browser/tests/rinkcraft-profile.test.mjs icelines-browser/tests/rinkcraft-evidence-guide.test.mjs
```

Run pinned wasm-bindgen 0.2.118 against the built WASM with `--target web` and
`--out-dir target/compact-sheet/inputs/pkg-deployment`, then:

```powershell
node tools/project_rinkcraft_deployment.mjs target/compact-sheet/inputs/pkg-deployment C:/src/PUCK/scenarios/league/2024-25/profile-contrasts/deployment-input.json target/compact-sheet/card-completion/fixtures.json target/compact-sheet/deployment
node tools/prepare_rinkcraft_theme_studio.mjs target/compact-sheet/deployment/fixtures.json target/compact-sheet/theme-studio
node target/compact-sheet/deployment/verify-projection.mjs
node icelines-browser/e2e/rinkcraft-deployment.mjs
node icelines-browser/e2e/rinkcraft-theme-studio.mjs
node icelines-browser/e2e/rinkcraft-card-facts.mjs
```

E2E uses the prepared loopback server 8771; it does not start one. Override with
ICELINES_THEME_URL. Private packet and PUCK provenance retain the exact source
commit, source URLs, input/output hashes and commands. No random seed/trials
apply to these deterministic calculations.

Generated local preview only; no full native/browser application deployment,
hosted build, push or merge. No new task observations, full-card completion,
rating calibration, or simulation parameter change is claimed. Next: dated
unit/station/duty dossiers with cover and matchup context.
