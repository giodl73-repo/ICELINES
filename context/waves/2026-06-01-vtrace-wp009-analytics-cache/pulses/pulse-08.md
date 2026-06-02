# WP-009 Pulse 08 - Player evidence-card cache surface

Date: 2026-06-01

## Scope

Add the first player evidence-card Web surface for the Major Analytics Cache
without promoting a full player research, deployment, or roster-decision
workflow. The slice mounts active-context routes:

- `/player/evidence-card`
- `/api/v1/player/evidence-card`

The routes derive the default cache key from Web active config as
`player_evidence_card:<season>:<season_type>` and default the selected player
metric set to `expected_goals_share`. Operators can still pass `cache_key` or
`metrics` for explicit cache inspection, but the normal player evidence-card
route no longer requires the generic named-cache report query contract.

## Changes

- Added player evidence-card HTML and JSON handlers that reuse
  `AnalyticsCacheStore`, `analytics_cache_consumer_envelope`, and
  `AnalyticsCacheConsumerView`.
- Passed `AnalyticsCacheConsumerKind::PlayerEvidenceCard` through the Web
  consumer path so ready JSON and HTML render the player-specific ViewModel
  title and consumer identity.
- Preserved explicit unavailable behavior for missing active player evidence-card
  cache records, including no cache-directory creation on read.
- Mounted `/player/evidence-card` and `/api/v1/player/evidence-card`.
- Added focused L2 route evidence for active-cache defaults, missing-cache
  unavailable state, ready HTML rendering, ready JSON rendering, non-claim copy,
  and route inventory/surface-parity coverage.
- Updated WP-009 VTRACE evidence rows and surface parity to distinguish this
  player evidence-card route from the generic named-cache report, coach route,
  and opponent-scout route.

## Evidence

| Level | Evidence | Result |
| --- | --- | --- |
| L2 | `cargo test -p icelines-web --test l2_analytics_cache_report --quiet` | passed 2026-06-01 |
| L1 | `cargo test -p icelines-web --test ted_lindsay_route_inventory --quiet` | passed 2026-06-01 |
| L1 | `cargo clippy -p icelines-web --lib --tests -- -D warnings` | passed 2026-06-01 |
| L0 | `cargo fmt --check` | passed 2026-06-01 |
| VTRACE | `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only` | passed 2026-06-01 |
| Hygiene | `git diff --check` | passed 2026-06-01 |

## Residual risk

- This is the first player evidence-card cache-backed route, not a finished
  player profile, deployment recommendation, or roster-decision workflow.
- Only `expected_goals_share` is the default metric for the active player card
  route; broader player metric families and player discovery remain future
  product work.
- Line explorer, goalie readiness, practice, postgame, and agent surfaces still
  require their own copy review and evidence before cache-backed claims.
