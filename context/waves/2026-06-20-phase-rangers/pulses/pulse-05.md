# Pulse 05: Evidence-envelope bridge decision

## Goal

Decide whether the new Rangers Signals roster matrix should bridge into the
existing WP-009 analytics cache/evidence-card envelope.

## Result

Status: passed.

Decision: keep `signals-roster` outside analytics cache for now. The roster
matrix remains a direct `PlayerSignalsView` consumer with a `signals-roster.v1`
JSON envelope. WP-009's `AnalyticsCacheConsumerView` remains the canonical
envelope for cached decision surfaces, but Signals need a separate cache
promotion gate before they can become cache metrics.

## Evidence

| Evidence | Result |
|---|---|
| `icelines-core/src/analytics_cache.rs` inspection | WP-009 cache records validate metric keys, source state, invalidation, methodology, disclosures, non-claims, and supported consumer kinds. |
| `design/specs/icelines-signals.md` inspection | Signals remain outside `StatId`, `--filter`, analytics cache, and cross-team ranking surfaces. |
| `context/waves/2026-06-20-phase-rangers/EVIDENCE-BRIDGE.md` | Records the no-bridge decision and future bridge gate. |

## Non-claims

- No Signals cache metric family was added.
- No `StatId`, filter, leaderboard, or analytics-cache promotion was added.
- No Web/TUI/cache parity claim was added for `signals-roster`.

## Next pulse

Proceed to layout persistence hardening or lean CLI audit/fence. Do not start a
Signals cache bridge until the future bridge gate is accepted.
