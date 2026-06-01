# WP-005 Pulse 09 - FLETCH Cache/Refresh Fallback Fixtures

## Scope

Selected FLETCH generic HTTP cache/refresh boundary.

## Change

- Added a verified-cache fallback for non-forced generic FLETCH HTTP fetches
  when the upstream source is unavailable after a prior verified cache fill.
- Kept forced refresh strict: `force=true` does not hide an unavailable source
  behind previously cached bytes.
- Added an L0 httpmock/tempdir fixture that populates a cache object, proves the
  non-forced unavailable-source path returns the cached bytes, and proves forced
  refresh still errors.

## Evidence

```powershell
cargo test -p icelines-fetch fletch::tests::fetch_generic_http_bytes_uses_cached_object_when_source_unavailable --lib -- --nocapture
cargo fmt --check
cargo clippy -p icelines-fetch --lib --no-deps -- -D warnings
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
git diff --check
```

Result: passed 2026-05-31.

## Review

This closes the selected generic FLETCH HTTP cache/refresh boundary for verified
cache fallback and forced-refresh refusal. It does not close upstream payload
schema drift, broader missing-source, abbreviation drift, or partial-fetch
resume/flag evidence.

## Status

`passed_with_risk`.
