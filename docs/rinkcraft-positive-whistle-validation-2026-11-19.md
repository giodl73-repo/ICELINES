# Signed whistle records and positive-net trial —2026-11-19

Local profile wording/count correction plus separate retrospective proposal trial. Shared Rust supplies signed drawn/taken/net; frontend shows Relative Whistle Record, missing stays unavailable. All old ranks/award states preserved; no new live feat rule, capability or simulator effect. Season source-count gap filled only after exact original report checksum/actor/GP/group match. Career and goalies receive no annual whistle pattern.

Trial adds strict drawn>taken to existing frozen P60 AND, preserving all original references/exposure cohorts.127forward/32defense awards;22 defender exclusions. Two retained defense awards gain one-event sign-boundary sensitivity. Post-hoc-derived proposal, not independent validation or actual PP creation.

Passed: three core/three trial Rust tests, scoped producer cargo check and four fresh producer Season matches,29 Node tests. Independent715 source counts/2,847 event variants/2,860 reference decisions, eight signed profiles, five prior-payload preservation checks,30 parent hashes, seven rejected malformed/binding cases and exact trial replay. Ten Season/Current UI cases,60desktop/60mobile themes, five Career marker regressions, ten single-page Letter612×792 exports, no page errors. McDavid Season screenshot visually inspected (23drawn12taken+11net). No full native/WASM app build or hosted validation claimed.

```powershell
$env:CARGO_TARGET_DIR='F:/src/ICELINES/target'
cargo test -p icelines-core --lib rinkcraft_whistle --locked
cargo test -p icelines-sources --example rinkcraft_whistle_trial --locked
cargo run -p icelines-sources --example rinkcraft_whistle_trial --locked -- target/compact-sheet/positive-whistle/trial-input.json target/compact-sheet/positive-whistle/trial-results.json
cargo run -p icelines-sources --example rinkcraft_whistle_profiles --locked -- target/compact-sheet/career-milestones/fixtures.json F:/src/PUCK-artifacts/research/2026-11-18-feat-longitudinal/baseline/reports.json target/compact-sheet/positive-whistle/profile-patches.json
node tools/project_rinkcraft_whistle_profiles.mjs target/compact-sheet/career-milestones/fixtures.json target/compact-sheet/positive-whistle/profile-patches.json target/compact-sheet/positive-whistle/fixtures.json
node icelines-browser/node_modules/typescript/bin/tsc --project icelines-browser/tsconfig.json
node tools/prepare_rinkcraft_theme_studio.mjs target/compact-sheet/positive-whistle/fixtures.json target/compact-sheet/theme-studio
node icelines-browser/e2e/rinkcraft-whistle-record.mjs
py target/compact-sheet/positive-whistle/verify.py
```

Private inputs, source/actor output and portable replay in PUCK artifacts; neutral protocol, CLM-099, decision0115 and producer review in Rinkcraft. Seed/trials not applicable; deterministic. Preserve older packets unchanged.
