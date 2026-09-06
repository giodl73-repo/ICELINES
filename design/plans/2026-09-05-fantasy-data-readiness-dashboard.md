# Fantasy Data-Readiness Dashboard

**Date:** 2026-09-05
**Status:** Implemented
**Roadmap:** Fantasy War Room, Wave 23

## Outcome

Add one read-only `fantasy_readiness.v1` contract that tells a manager whether
IceLines has enough current evidence to support each fantasy workflow. Every
non-ready check names an exact local recovery command. CLI, TUI, and Web render
the same core projection and never fetch or mutate while inspecting readiness.

## Contract

The core contract owns:

- workflow IDs: `draft`, `today`, `matchup`, `week_plan`, `goalie`, `trade`,
  and `decision_review`;
- check requirement: `required` or `optional`;
- state: `ready`, `provisional`, or `blocked`;
- reason code, human detail, source observation/fetch timestamps, and recovery;
- per-workflow and whole-dashboard state, stable ordering, counts, and a
  material fingerprint.

State aggregation is deliberately conservative:

1. any blocked required check blocks its workflow;
2. any other non-ready check makes the workflow provisional;
3. otherwise the workflow is ready;
4. the dashboard is blocked if any selected workflow is blocked, provisional
   if any is provisional, and ready only when all are ready.

## Evidence assembly

`icelines-fetch` reuses the immutable `fantasy_today.v2` assembly and its
existing schedule, rules/roster, player-rate, current-roster, status, matchup,
and source-authority rows. It derives only:

- acquisition-budget presence from the computed weekly budget;
- goalie-evidence freshness from the shared goalie summary;
- decision-journal availability from read-only decision rows.

If a root prerequisite prevents daily assembly, the dashboard still returns a
typed blocked view rather than failing without recovery. It must not create a
database, SQLite sidecar, cache, snapshot, or network request.

## Surfaces

```text
icelines fantasy readiness
icelines fantasy readiness --workflow matchup
icelines fantasy readiness --json
```

- CLI shows a compact workflow summary followed by non-ready checks.
- Fantasy Today TUI shows the selected dashboard state and top recovery.
- `/fantasy/readiness` and `/api/v1/fantasy/readiness` support sticky
  `league`, `team`, `workflow`, and `stats_season` query parameters.
- Web responses are private/local, read-only, and `Cache-Control: no-store`.

## Compatibility and privacy

- No existing `fantasy_today.v1/v2` field changes.
- No migration is required.
- No manager rationale or private decision-outcome notes enter this contract.
- Unknown or absent optional evidence is provisional, never numeric zero.

## Verification

- L0: aggregation, duplicate fences, recovery requirements, fingerprints.
- L1: complete, provisional, and root-blocked assembly without writes.
- L2: CLI parsing/text/JSON, TUI recovery rendering, Web sticky filters,
  semantic empty states, and no-store behavior.
- Regression: formatting, Clippy, workspace tests, source inventory, and
  `git diff --check`.

Validation completed on 2026-09-05:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace -- --test-threads=1`
- `cargo test -p icelines-fetch --test source_module_inventory`
- `py C:/src/tracker/repos/standards-protocols/roles/tools/check_roles.py .`
- `git diff --check`

## Role-review amendments

- HART/KEEL: existing Today evidence remains authoritative; readiness is a
  projection, not a second persistence model.
- TAPE/WIRE: source timestamps remain optional and typed; absence never becomes
  freshness zero.
- FORGE: the builder is pure core logic and the assembler opens only immutable
  local stores.
- PACE: no readiness score or invented probability is introduced.
- BENCH/EDGE: duplicate checks, missing recovery, root failure, and read-only
  filesystem behavior receive explicit tests.
- SCOUT: data readiness is kept separate from hockey-value quality.
- GLASS/CREST/broadcast: state is encoded by labels as well as color; recovery
  is visible above detail; narrow HTML remains semantic and scroll-safe.
