# Browser WASM pulse 42 — first remote failures diagnosed

Date: 2026-10-04. Previous turn made progress by creating draft PR #74.
Goal remains active. Remote browser run 37213291939 is terminal failure;
repository CI run 37213291912 had a completed failed audit job at inspection.

Browser CI installed `wasm32-unknown-unknown` for Rust 1.95.0, but the repository
`rust-toolchain.toml` selects `stable`. The build entry point did not enforce its
browser pin, causing missing `core` during WASM compilation. The entry point now
sets `RUSTUP_TOOLCHAIN` from browser configuration for both captured commands and
all build/test child processes. Native repository toolchain policy is unchanged.

The audit found RUSTSEC-2026-0285 in rustls 0.23.39, which also exists in the base
master lockfile. The constrained update changes only rustls to the minimum
patched 0.23.45 and its required rustls-webpki to 0.103.15. No exception was added.
The primary advisory is https://rustsec.org/advisories/RUSTSEC-2026-0285.
Local `cargo audit` exited 0, with the same three allowed warning advisories.
`cargo test --locked -p icelines-fetch --lib` passed all 585 tests.

The isolated complete pinned checked build is running as terminal session 25474,
log `target/browser-pulse-42-pinned-check.txt`. Its WASM build and distribution
verification have passed; native parity/remaining suites are not yet inferred
successful. Next turn must revalidate that handle rather than restart it.
The two-file changes remain uncommitted until verification passes. Original
checkout contains the build-entry fix; its lockfile has not yet been replaced.
Next action: finish verification, reconcile the original lockfile safely, and
commit/push these CI fixes to the existing draft PR before inspecting fresh CI.

Release remains unaccepted. No Pages setting, merge or deployment occurred.
