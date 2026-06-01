# WP-005 Pulse 04 - Upstream Retry Failure Fixtures

## Scope

Selected fetch/upstream failure boundary: NHL API client retry behavior for
rate-limited and transient/unavailable upstream responses.

## Change

- Recorded existing httpmock retry evidence under `CHG-060` and
  `EVID-WP005-FETCH-RETRY-L1`.
- No implementation change was required; the existing `fetch_retry_l15` suite
  already exercises 429, 503, generic 5xx, non-retryable 4xx, retry budget, and
  backoff-cap behavior against `NhlApiClient` without live network calls.

## Evidence

```powershell
cargo test -p icelines-fetch --test fetch_retry_l15 -- --nocapture
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
git diff --check
```

Result: passed 2026-05-31.

## Review

The selected L1 evidence covers typed outcomes for `429` (`RateLimited`), `503`
(`ServiceUnavailable`), generic `500` (`Http` with status), fail-fast behavior
for non-retryable `401`/`404`, and bounded retry/backoff behavior. This closes
the 429/503 upstream-failure slice while leaving schema, integrity, cache/refresh,
CSV/column, abbreviation-drift, transcript, and resume evidence pending.

## Status

`passed_with_risk`.
