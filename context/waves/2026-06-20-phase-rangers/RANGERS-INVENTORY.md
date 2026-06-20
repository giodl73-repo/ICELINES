# Phase Rangers inventory

Date: 2026-06-20

## Purpose

Confirm the post-Hurricane starting point before implementation. This inventory
keeps Phase Rangers from reopening shipped work or promoting blocked claims.

## Current surface posture

| Area | Current evidence | Rangers posture |
|---|---|---|
| Signals | WP-010 pulses 01-05 ship core descriptors, `PlayerSignalsView`, CLI text/JSON, TUI player-card block, Web player HTML/JSON, and Markdown export. Surface parity row records no zero-fill and non-claim copy. | Reuse the existing ViewModel. Add discovery only after copy/evidence review; do not promote to `StatId`, filters, cache, or leaderboards by default. |
| Evidence cards / cache | WP-009 has `AnalyticsCacheConsumerView`, named-cache report, coach dashboard, opponent scout, player evidence-card, line explorer, goalie readiness, practice focus, postgame, adjustment, and agent evidence Web/API route evidence. | Reuse the cache/evidence envelope where it already fits. Rangers should not define a second evidence-card model; it should bridge selected Signals or NYR workflow output to the existing envelope only if the source semantics are compatible. |
| Workbench layouts | WP-002 closed with risk: shared named layout schema/store, CLI management commands, TUI restore hook, and Web dashboard `layout=<name>` restore exist. | Treat layout persistence as existing capability with residual risk, not a blank build. Rangers can harden docs, workflow use, or risk closure, but should not rebuild the schema. |
| Lean offline CLI | WP-007 is target-not-met and dispositioned: FLETCH path dependency, direct/transitive SLICE git dependencies, affected commands, and missing `cli` feature are known blockers. | Keep as an audit/fence goal. Do not perform broad Cargo surgery inside early Rangers pulses. |
| NYR workflow | Team docs already use `NYR` examples, and bundled data supports team/player/goalie/report paths. No single Rangers workflow proof ties the post-Hurricane surfaces together. | Best first implementation pulse. Build a repeatable script or doc-backed command transcript that proves the shipped surfaces work together without team-specific hardcoding. |

## Blockers kept from Hurricane

- MoneyPuck deployment catalog expansion needs pinned upstream schema evidence.
- Goalie GSAx/high-danger save percentage needs a verified goalie xGA or danger
  source.
- Team outlook confidence needs a team-level ViewModel/source contract.
- Signals cache/catalog/filter/leaderboard promotion needs product-copy and
  evidence review.
- Broader interactive TUI/browser proof needs explicit capture evidence.

## Recommended implementation order after inventory

1. **NYR workflow proof.** Add a repeatable Rangers workflow that exercises
   existing surfaces and reveals any product gaps without changing analytics
   semantics. Status: passed in pulse 02.
2. **Signals discovery design gate.** Decide whether discovery is a report,
   route, command, or cache consumer. Require product-copy and evidence review
   before implementation. Status: passed in pulse 03; use a roster discovery
   matrix, not a leaderboard or `StatId` promotion.
3. **Evidence-envelope bridge.** If the NYR workflow needs a shared evidence
   card, reuse `AnalyticsCacheConsumerView` or explicitly document why Signals
   need a different envelope. Status: passed in pulse 05; keep `signals-roster`
   outside analytics cache until a separate Signals cache-promotion gate.
4. **Layout persistence hardening.** Use existing WP-002 layout save/restore in
   the workflow or close one residual risk with focused tests/docs.
5. **Lean CLI audit/fence.** Refresh WP-007 blocker evidence and add a
   reproducible no-claim check command only after the workflow proof lands.

## Pulse 02 candidate

Create a `scripts/rangers-workflow.ps1` or documentation-backed command transcript
that runs offline against bundled data:

- `icelines team NYR`
- `icelines query leaders --team NYR`
- `icelines signals "<NYR player>"`
- `icelines query goalies --team NYR`
- one Markdown export or report path with source/completeness disclosure

The pulse should assert disclosure text, unavailable-state handling, and no
team-specific hardcoded claims.
