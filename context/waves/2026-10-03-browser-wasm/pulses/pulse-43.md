# Browser WASM pulse 43 — CI fixes locally accepted

Date: 2026-10-04. Previous turn made progress by diagnosing and correcting
remote failures. Goal remains active.

Terminal 25474 completed with exit 0. The complete isolated checked build with
the explicit Rust 1.95.0 pin and constrained TLS update passed distributed-WASM
verification, 270 full-row native/WASM comparisons, affected Rust suites,
18 Python tests and all 128 browser tests. Full log is
`target/browser-pulse-42-pinned-check.txt` in the managed worktree. The separate
585-test native fetch library run and local audit also passed (pulse 42).

The original lockfile was reconciled only after its normalized contents matched
the pre-update committed lockfile exactly. No unrelated native edits were copied.
The change updates only rustls 0.23.39 to 0.23.45 and required rustls-webpki
0.103.13 to 0.103.15; the browser build entry point now enforces its configured
toolchain for every child command. This resolves the diagnosed failures locally;
fresh remote CI remains required. The first remote matrix still had only the
audit failure among completed repository jobs at the final inspection, alongside
the separate failed browser build. In-progress jobs are not inferred successful.

Next action is the scoped CI-fix commit/push to draft PR #74, followed by fresh
remote validation and retained artifact inspection. This evidence is a pre-commit
record; actual follow-up commit and run identities must be read after execution.
No merge, Pages setting change or deployment is authorized by these checks.
Device/resource, full state composition and deployed live/offline/update/rollback
acceptance remain open. Relay ownership remains deferred until origin testing.
