# WP-005 Pulse 07 - Chunked Snapshot Schema Drift Fixtures

## Scope

Selected chunked snapshot manifest schema compatibility and newer-schema refusal
boundary.

## Change

- Recorded existing L0 evidence that v1 chunked manifests promote into the v2
  in-memory representation, including pre-Hart.6 manifests without playoff
  fields.
- Recorded existing L0 evidence that v2 chunked manifests round-trip through the
  nested `reports` shape without leaking new report keys into legacy accessors.
- Recorded existing L0 evidence that v3/newer chunked manifests fail
  deserialization with a `RepoVersionUnknown`-shaped error instead of being
  accepted as trusted source state.

## Evidence

```powershell
cargo test -p icelines-fetch snapshot::tests::l0_lindsay_chunked_manifest --lib -- --nocapture
cargo fmt --check
cargo clippy -p icelines-fetch --lib --no-deps -- -D warnings
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
git diff --check
```

Result: passed 2026-05-31.

## Review

This closes the selected chunked snapshot manifest schema drift and newer-schema
refusal boundary. It does not close cache/refresh, upstream payload schema drift,
abbreviation drift, broader missing-source, or partial-fetch
resume/flag evidence.

## Status

`passed_with_risk`.
