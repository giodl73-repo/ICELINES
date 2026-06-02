# WP-009 Pulse 14 - Agent evidence summary cache surface

Date: 2026-06-02

## Scope

Add the first narrow agent-evidence Web surface for the Major Analytics Cache
without promoting autonomous agent action, recommendation execution, prediction,
or complete-world claims. The slice mounts active-context routes:

- `/agents/evidence`
- `/api/v1/agents/evidence`

The routes derive the default cache key from Web active config as
`agent_evidence:<season>:<season_type>` and default the selected metric set to
`expected_goals_share`. Operators can still pass `cache_key` or `metrics` for
explicit cache inspection, but the normal agent evidence route does not require
the generic named-cache report query contract.

## Changes

- Added agent evidence summary HTML and JSON handlers that reuse
  `AnalyticsCacheStore`, `analytics_cache_consumer_envelope`, and
  `AnalyticsCacheConsumerView`.
- Used `AnalyticsCacheConsumerKind::AgentEvidence` so the route stays a
  read-only evidence surface rather than an action-taking agent workflow.
- Preserved explicit unavailable behavior for missing active agent-evidence
  cache records, including no cache-directory creation on read.
- Mounted `/agents/evidence` and `/api/v1/agents/evidence`.
- Added focused L2 route evidence for active-cache defaults, missing-cache
  unavailable state, ready HTML rendering, ready JSON rendering, non-claim copy,
  and route inventory/surface-parity coverage.
- Updated WP-009 VTRACE evidence rows and surface parity to distinguish this
  agent evidence route from broader unfinished agent workflows.

## Evidence

| Level | Evidence | Result |
| --- | --- | --- |
| L2 | `cargo test -p icelines-web --test l2_analytics_cache_report --quiet` | passed |
| L1 | `cargo test -p icelines-web --test ted_lindsay_route_inventory --quiet` | passed |
| L1 | `cargo clippy -p icelines-web --lib --tests -- -D warnings` | passed |
| L0 | `cargo fmt --check` | passed |
| VTRACE | `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only` | passed |
| Hygiene | `git diff --check` | passed |

## Residual risk

- This is a read-only cache evidence summary, not an agent action, autonomous
  coaching workflow, recommendation executor, or complete-world decision system.
- Only `expected_goals_share` is the default metric for the active agent evidence
  route; broader agent evidence metric families remain future product work.
- Agent workflows still require their own copy review and evidence before
  cache-backed claims.
