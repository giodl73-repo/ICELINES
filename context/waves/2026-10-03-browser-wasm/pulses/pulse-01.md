# Browser WASM foundation pulse 01

Date: 2026-10-03
Work package: WP-BW-01; change: CHG-119
Parent: REQ-BROWSER-001; IF-BROWSER-001/IF-FETCH-001/IF-BUILD-001.

## Scope

Portable icelines-data normalization extracted without rewriting formulas.
Native stats_loader delegates to shared functions; icelines-wasm accepts validated
season-package bytes and runs the shared parser/evaluator on owned repository data.
No UI/local-storage/live/deployed completion claimed.

## Evidence

Passed:

- `cargo check -p icelines-wasm --target wasm32-unknown-unknown --offline --target-dir target/browser --locked`.
- `cargo test -p icelines-data -p icelines-wasm --offline`: 2 normalization + 5 engine tests.
- `cargo test -p icelines-fetch --test stats_loader --offline --locked`: 42 existing native loader integration tests.
- `cargo test -p icelines-fetch --test browser_native_parity --offline`: 2 real-bundle regular/playoff native-browser parity tests.
- `cargo clippy -p icelines-data -p icelines-wasm --all-targets --offline -- -D warnings`.
- `cargo clippy -p icelines-fetch --lib --offline -- -D warnings`.
- Affected-source rustfmt checks and scoped `git diff --check`.
- `python scripts/build-browser-catalog.py`: 75 packages generated from current repository data.
- `python -m unittest discover -s scripts/tests -p test_browser_catalog.py`: 7 archive/schema tests.
- `cargo run -p icelines-wasm --example verify_catalog --offline -- icelines-browser/public/catalog.json`: all 75 package hashes and shared engine loads verified.
- `cargo tree -p icelines-wasm --target wasm32-unknown-unknown --offline --prefix none`: runtime graph excludes native fetch/FLETCH/Tokio/reqwest/SQLite; retains pinned portable slice-core.

Not yet proven: browser bindings runtime, worker generation protocol, application UI,
IndexedDB transactions, live deployed-origin reachability/relay disposition,
release archive download conversion, offline shell, and Pages preview/publication.
The verifier used repository season files; published release archive contents are
not inferred equivalent. Python's legacy null seasonId behavior now matches the
native compatibility rule; explicit wrong-season rows are rejected.

Actual WASM binary build passed: `cargo build -p icelines-wasm --target
wasm32-unknown-unknown --offline --target-dir target/browser`.
Artifact: `target/browser/wasm32-unknown-unknown/debug/icelines_wasm.wasm`.
This debug artifact is not the release download-size measurement.

## Gate

In progress. Full plan remains active; source/browser/storage/Pages gates pending.

## Continuation

Optimized WASM build is running in exec session **46628**:
`cargo build -p icelines-wasm --target wasm32-unknown-unknown --profile
browser-release --offline --target-dir target/browser`.
Re-poll this handle before starting another optimized build. The browser-only
profile uses size optimization, thin LTO and stripped debug information.
Next: pin/install matching wasm-bindgen CLI into workspace tool output, generate
bindings, then implement worker protocol and browser shell/library/live adapters.
The previous planning turn and this foundation turn both made authoritative
progress. No external blocker has been established; full goal remains active.
