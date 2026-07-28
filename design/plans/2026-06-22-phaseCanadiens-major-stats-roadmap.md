# Phase Canadiens - Major stats systems roadmap

**Date**: 2026-06-22
**Status**: Active
**Specification**: [`../specs/query-engine.md`](../specs/query-engine.md),
[`../specs/data-sources.md`](../specs/data-sources.md), and
[`../specs/surface-parity.md`](../specs/surface-parity.md)
**Archive when**: every ordered workstream is complete, explicitly deferred,
or transferred to a named successor roadmap

## Goal

Turn the current gold-plated local stats workbench into a credible competitor
against major public hockey stats systems. The order below prioritizes user
value, data authority, and visible product trust.

## Ordered Work

| Order | Phase | Goal | Done when |
|---|---|---|---|
| 1 | Strength-state splits | Add 5v5 / PP / PK context across leaders, player/team pages, exports, and JSON. | Strength-state metrics have a verified source/join contract, `StatId` keys where appropriate, CLI/TUI/Web/API parity, and missing-source copy. |
| 2 | Advanced source authority | Expand verified advanced metrics beyond current MoneyPuck xG: goalie xGA/GSAx candidates, high-danger context, and richer xG/xGA evidence. | Each promoted source has schema fixtures, source-state metadata, freshness labels, and explicit non-claims. |
| 3 | Signals promotion | Move accepted Signals metrics into bounded cache/catalog/filter/leaderboard surfaces. | Cache metric keys, invalidation rules, `StatId` semantics, bounded ranking copy, and CLI/Web/TUI tests exist. |
| 4 | Historical shift-data policy | Decide whether and how historical shifts become a supported local capability. | A source/bundle/fetch policy exists, `sync.capabilities.shifts` is intentionally changed or reaffirmed, and strength-state/deployment joins have fixtures. |
| 5 | Browser QA and accessibility proof | Upgrade dashboard proof from representative captures to broad interaction confidence. | Keyboard order, pointer/touch behavior, screen-reader labels, cross-browser smoke, and responsive overflow sweeps are automated or explicitly documented. |
| 6 | Production packaging | Make ICELINES easy to install, update, demo, and trust outside a developer checkout. | Release artifacts, one-command setup/update, seeded demo profile, public API docs, and freshness diagnostics are tested. |
| 7 | Editorial/stathead workflows | Add high-value exploratory workflows that major stats users expect. | Saved query packs, era/historical leaderboards, comparison collections, playoff-run narratives, and shareable reports exist behind stable commands/routes. |
| 8 | Data freshness and authority | Make cache/source truth visible and operationally reliable. | Source timestamps, refresh status, stale warnings, automated freshness checks, and provenance are visible across major surfaces. |

## Execution Rules

- Do not promote a metric without a source-state, freshness, and non-claim
  contract.
- Do not add leaderboard/ranking semantics until tie-breaks, sample-size gates,
  unavailable values, and source omissions are tested.
- Keep CLI/TUI/Web/API parity as the default for promoted stats; explicitly
  document any surface that remains a handoff or partial.
- Keep browser mutations POST-backed and keep GET reads side-effect-free.
- Prefer focused phases that ship one durable contract at a time over broad
  speculative rewrites.

## First Phase Candidate

Start with the strength-state split foundation. It unlocks the most visible
"major stats system" gap and gives later advanced-source work a cleaner place to
land.

Minimum first-phase scope:

- inventory current play-by-play, game-cache, and shift-source capabilities
- define strength-state key names and source-state copy
- add one bounded read surface before broadening the catalog
- prove no read route performs live fetch or local cache creation
- record explicit blockers for historical shifts and unavailable source windows

## Validation Expectations

Each implementation phase should name focused gates before it starts. The
default closeout should include:

```powershell
cargo fmt --check
cargo test --workspace --no-fail-fast
git diff --check
```

Large phases may split validation into documented slices when the full workspace
gate is too slow for inner-loop work, but final promotion should run the broad
gate or record the exact reason it could not.
