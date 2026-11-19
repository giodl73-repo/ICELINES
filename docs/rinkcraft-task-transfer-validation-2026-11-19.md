# Task-record temporal transfer validation —2026-11-19

Generalize fixed task screen from a hardcoded season to a validated explicit season; same rules/gates except year. Add Rust adjacent-year annual-reference transfer with separate absence/group/exposure/source/reference/comparable statuses. No source repair, live award/card/class/attribute/simulator change.

Passed five screen and five transfer Rust tests.44new raw pages/112,516raw assembly fields/6,222rational ranks/42scenario groups/12overlaps and ten screen rejects. Exact old2024screen replay unchanged. Transfer oracle:21annual cases/14transitions/789common actor records/208prior winner audits/60parent hashes/five adversarial cases/exact replay. These are execution counts, not independent sampleN or scientific validation. No UI change/browser acceptance claimed.

```powershell
./tools/capture_rinkcraft_task_reports.ps1 -Season 20232024 -OutputDirectory NEW_2023_CAPTURE
./tools/capture_rinkcraft_task_reports.ps1 -Season 20252026 -OutputDirectory NEW_2025_CAPTURE
$env:CARGO_TARGET_DIR='F:/src/ICELINES/target'
cargo test -p icelines-sources --example rinkcraft_task_feat_screen --locked
cargo test -p icelines-sources --example rinkcraft_task_feat_transfer --locked
cargo run -p icelines-sources --example rinkcraft_task_feat_transfer --locked -- target/compact-sheet/task-transfer/input.json target/compact-sheet/task-transfer/results.json
py target/compact-sheet/task-transfer/verify-screen.py target/compact-sheet/task-transfer/screen-2023
py target/compact-sheet/task-transfer/verify-screen.py target/compact-sheet/task-transfer/screen-2025
py target/compact-sheet/task-transfer/verify.py
```

Retain original raw/assembled bytes for exact replay, not recaptured current vintage. New screen input files bind protocol/year-specific rules/four assemblies; normalized rules differ only by season. Transfer input binds all three screen/rules files and protocol. Annual same-season references are descriptive, not a frozen-norm forecast. Prior study exposure declared. Source/actor data in PUCK artifacts; neutral Rinkcraft decision0117/CLM-101/protocol/study/review. No native/WASM/hosted/prospective release.
