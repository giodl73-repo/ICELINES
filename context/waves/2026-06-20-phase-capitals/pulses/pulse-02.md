# Phase Capitals Pulse 02 - Cache Eligibility Gate

**Date:** 2026-06-20
**Result:** Passed as not eligible yet

## Decision

Signals remain uncached. Do not bridge `PlayerSignalsView` or `signals-roster`
into WP-009 analytics cache during this pulse.

## Evidence

| Evidence | Result |
|---|---|
| `icelines-core/src/analytics_cache.rs` | Cache records require supported metric keys, record and metric source-state, invalidation keys, methodology version, disclosures, non-claims, and supported consumers. |
| `icelines-core/src/view_model/signals.rs` | `PlayerSignalsView` carries Signal rows, methodology, unavailable values, disclosures, and non-claims, but not cache invalidation or accepted cache metric keys. |
| `design/specs/icelines-signals.md` | The promotion rule requires cache-envelope methodology before any cache metric family is added. |
| `context/waves/2026-06-20-phase-capitals/CACHE-ELIGIBILITY.md` | Records the durable no-cache decision and prerequisites for a future bridge. |

## Validation

- `git diff --check`

## Residual Risk

Signals still have no cache-backed consumer. That is intentional until a future
implementation pulse defines and tests cache metric keys, source-state,
invalidation, methodology versioning, and consumer semantics.

## Next Pulse

Pulse 03 decides whether any Signal can enter `StatId`, `--filter`, or public
leaderboard surfaces, or whether those promotions also remain deferred.
