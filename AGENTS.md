# Working in IceLines

This is a local-first hobby project. Keep repository-defined GitHub Actions
manual-only; the owner does not want automatic builds on commits, PRs or tags.
Only add automatic triggers when the owner explicitly requests them.

Validate locally before committing. Choose checks appropriate to the changed
files; `scripts/test-slice.ps1 list` lists the focused Rust test slices. For broad
Rust changes use the local checks in CONTRIBUTING.md. Check workflow syntax and
trigger/job contracts for workflow-only edits. Do not run a full hosted matrix
as a substitute for local checks.

Publishing, release builds and rollback remain explicit manual operations.
