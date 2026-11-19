# Feat round-two validation — 2026-11-19

Offline Rust example only: no live feat registry, UI, capability, class, native/WASM or simulator change. Three frozen candidates, development2023–24/2024–25, retrospective evaluation2025–26. Prior exposure remains declared. Fixed-reference one-event stress and actor/row/single-year reference comparisons, not confidence intervals or calibrated talent. A post-hoc signed-whistle audit is separately labeled.

Six Rust tests pass. Independent rational oracle:20,432 condition ranks,3,931 count variants,7,638 reference observations,2,514 cross-round baseline condition matches; six input hashes and117 parent packet files verified. Five malformed/hash/order/holdout/seed cases reject. Exact replay byte-identical. No UI change, so no browser regression claimed.

```powershell
$env:CARGO_TARGET_DIR='F:/src/ICELINES/target'
cargo test -p icelines-sources --example rinkcraft_feat_robustness --locked
cargo run -p icelines-sources --example rinkcraft_feat_robustness --locked -- target/compact-sheet/feat-round-two/input.json target/compact-sheet/feat-round-two/results.json
py target/compact-sheet/feat-round-two/verify.py
```

Inputs reference sealed public NHL source assemblies, not live queries. Full actor-level run and parent lineage belong in PUCK artifacts. Rinkcraft decision0114, CLM-098, frozen protocol, neutral aggregate study and producer-applied review explain findings, limitations and proposed next-trial wording/gates. No automatic rule promotion, source refresh or independent expert validation.
