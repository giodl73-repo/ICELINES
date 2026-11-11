# Conditional support/cover comparison

Shared Rust `rinkcraft_plan_compare` verifies a v1 possession replay, holds its
hero/rules/seed/action path fixed, and reconstructs held-cover and committed-support
plans. Each stops before its first unavailable recorded action. No adaptive path
or missing roll is invented. Existing session/replay fields and gameplay stay intact.

The versioned comparison sidecar carries source fingerprint, both last-valid
sessions, blocked-step information and paired event rows. It distinguishes initial
cover differences from first arithmetic differences and first resolution/phase/
control/loose-state differences. Unpaired rows have null comparison flags. The
sidecar is not an ordinary replay-import root; the original replay is preserved.

Adapters: native POST `/api/v1/rinkcraft/possession/compare`, WASM
`compare_possession_plans`, worker `comparePossessionPlans`, and Rust CLI:

```powershell
cargo run -p icelines-core --example rinkcraft_replay_compare --locked -- replay.json
```

The UI compares the full live path even while inspecting a prefix, renders Rust
values/flags, downloads the sidecar separately, and clears comparison results on
source/setup changes. Character sheets and normal replay export stay unchanged.

Core verification: all 61 `rinkcraft_` library tests pass, including four new
comparison tests for illegal-path stopping, paired dice/counter divergence,
zero-bonus setup-versus-outcome separation, empty history and invalid source files.
TypeScript and JavaScript syntax checks pass. The rebuilt native mobile UI and
Rust CLI reproduce seed 92's first arithmetic difference at step 1 and first
outcome difference at step 3.

Integration verification completed: ten-template native/WASM parity, held and
committed source orientation, seed-92 identical dice and counter difference,
seed-43 illegal continuation stopping, invalid source rejection, sidecar download
parity, full-live comparison during prefix review, unchanged replay/card, context
change clearing, offline browser use and 390-pixel layout. All 26 focused Node
tests and existing timeline/import/possession/creator E2E pass. Desktop/mobile
screenshots were inspected. Independent pypdf inspection confirms two single-page
612×792-point Letter exports with identity/attributes present and comparison absent.
Rust CLI/native output parity and nonzero invalid-input exit pass.

Replay commands after building matching native/WASM assets:

```powershell
node icelines-browser/e2e/rinkcraft-plan-compare.mjs
node icelines-browser/e2e/rinkcraft-replay-timeline.mjs
node icelines-browser/e2e/rinkcraft-replay-import.mjs
node icelines-browser/e2e/rinkcraft-possession.mjs
node icelines-browser/e2e/rinkcraft-game-studio.mjs
node scripts/verify-browser-dist.mjs
```

Local artifacts belong under ignored `target/plan-comparison/`; Rinkcraft governs
interpretation and retains compact results/provenance under decision 0030.
Selected-path results do not estimate policy value, game balance or NHL effects.
