# Experimental faceoff profile validation —2026-11-19

Local Season-only source-bound task supplement: O-zone Draws, D-zone Draws, Faceoff Record. Shared Rust projection creates records/ranks/states/fixed-reference one-draw variants; per-pattern source/actor/year/group/type/capture metadata. Additive Node merge preserves all old values. TS validates scope/source-query bindings and displays; no ability or award arithmetic. Specialized records use existing three-slot cap; all other patterns/old feats remain browseable.

Passed four Rust tests,32 Node tests; independent24records/24rational ranks/24boundary variants/12earlier observations/eight source hashes/five old-payload preservation checks/seven rejected cases/exact replay. Future source files replaced with nonexistent paths leave patch bytes unchanged. Twelve Season patterns,30desktop/30mobile theme cases,11baseline comparisons (all Current/Career and goalie Season), five one-page612×792Letter exports, no page errors. All30abilities unrated. Expanded-source mobile overflow and count-line full-width geometry verified. Screenshots inspected after fixing grid-column wrapping.

```powershell
$env:CARGO_TARGET_DIR='F:/src/ICELINES/target'
cargo test -p icelines-sources --example rinkcraft_faceoff_profiles --locked
cargo run -p icelines-sources --example rinkcraft_faceoff_profiles --locked -- target/compact-sheet/positive-whistle/fixtures.json target/compact-sheet/faceoff-cards/input.json target/compact-sheet/faceoff-cards/patches.json
node tools/project_rinkcraft_faceoff_profiles.mjs target/compact-sheet/positive-whistle/fixtures.json target/compact-sheet/faceoff-cards/patches.json target/compact-sheet/faceoff-cards/fixtures.json
node icelines-browser/node_modules/typescript/bin/tsc --project icelines-browser/tsconfig.json
node tools/prepare_rinkcraft_theme_studio.mjs target/compact-sheet/faceoff-cards/fixtures.json target/compact-sheet/theme-studio
node tools/prepare_rinkcraft_theme_studio.mjs target/compact-sheet/positive-whistle/fixtures.json target/compact-sheet/faceoff-cards/baseline-site
node icelines-browser/e2e/rinkcraft-faceoff-cards.mjs
py target/compact-sheet/faceoff-cards/verify.py
```

Current/Career and goalie receive no borrowed faceoff data. These are experimental observed records, not role assignments, possession-after-draw, validated capability or simulator modifiers. Active34-rule registry/32candidate definitions unchanged. No hosted build, full native/WASM scope loader or scientific/independent review claimed. Rinkcraft decision0118, source174/177, CLM-102, contract and producer review retain neutral governance; actor fixtures/source packets remain private in PUCK. Deterministic; seed/trials not applicable.
