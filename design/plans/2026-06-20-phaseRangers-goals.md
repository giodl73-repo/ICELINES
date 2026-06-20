# Phase Rangers - Post-Hurricane goals

> Phase Rangers is the post-Hurricane organization round: take the shipped
> analytics surfaces and make them easier to find, trust, persist, and run in
> a lean offline workflow. It does not reopen Hurricane's blocked source claims
> unless the missing evidence is added first.

**Created:** 2026-06-20
**Status:** Active - pulse 01 inventory passed

---

## Frame

Phase Hurricane shipped the high-visibility analytics push: Signals on real
surfaces, MoneyPuck on-ice fields, player confidence ranges, goalie workload
fields, season-depth honesty, and compact charts. Phase Rangers should now
improve the product's operating shape around those wins:

- make shipped analytics discoverable without overstating them;
- preserve the same evidence and disclosure across surfaces;
- let power users keep a workbench shape;
- reduce release and dependency friction for offline CLI use.

The phase stays descriptive and evidence-first. No prediction, betting, injury,
deployment-quality, or autonomous coaching claims are allowed without a later
requirement and validation record.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Rangers Goal 1 - Signals discovery lane** | Hurricane shipped Signals, but they remain mostly player-card/export driven. Users need a controlled path to find and compare them. | A small Signals discovery surface exists with methodology/non-claim copy, unavailable evidence states, and tests proving no zero-filled missing values. Any catalog/filter/leaderboard promotion is explicitly gated by product-copy and evidence review. |
| 2 | **Rangers Goal 2 - Evidence card envelope reuse** | WP-009 already has selected analytics cache/evidence-card consumers. Rangers should reuse that contract instead of creating a second evidence model. | One Rangers slice either consumes the existing `AnalyticsCacheConsumerView` path or records why Signals/NYR workflow evidence should remain outside the cache envelope. |
| 3 | **Rangers Goal 3 - Workbench layout hardening** | WP-002 already shipped named layout persistence with accepted risk. Rangers should use or harden it, not rebuild it. | A Rangers workflow uses existing layout persistence or closes one residual WP-002 risk with focused evidence while keeping stable workbench pane IDs and context fields. |
| 4 | **Rangers Goal 4 - Lean offline CLI path** | REQ-DEP-001 and REQ-LEAN-001 remain target states. A lean CLI gives the repo a cleaner distributable story. | Cargo feature boundaries are inspected and narrowed in one safe slice, with a documented command for an offline CLI check. Any remaining FLETCH/SLICE dependency or feature blocker is recorded precisely. |
| 5 | **Rangers Goal 5 - Rangers team workflow proof** | The round needs one concrete user workflow instead of abstract platform cleanup. NYR can serve as a representative team path using existing bundled data. | A scripted or documented NYR workflow runs through team page, roster/depth, player Signals, goalie workload, and export/report output with source/completeness disclosures visible. No team-specific hardcoded claims are added. |

---

## Non-goals

- Do not add MoneyPuck deployment columns without pinned schema evidence.
- Do not add GSAx or high-danger save percentage without a verified goalie xGA
  or danger source.
- Do not synthesize team confidence bands by summing or averaging player bands.
- Do not promote Signals into broad `StatId`, filters, cache, or leaderboards
  without the Rangers Goal 1 review gate.
- Do not make the web or TUI claim live-browser proof unless the phase captures
  browser/TUI evidence explicitly.

---

## Recommended pulse order

1. **Pulse 01 - Plan and inventory.** Confirm the exact surfaces and current
   blockers for Signals discovery, evidence cards, layouts, lean CLI, and the
   NYR workflow. Result: passed 2026-06-20; see
   `context/waves/2026-06-20-phase-rangers/RANGERS-INVENTORY.md`.
2. **Pulse 02 - NYR workflow proof.** Build the smallest repeatable script or
   docs-backed workflow that exercises existing surfaces and reveals gaps.
3. **Pulse 03 - Evidence card contract.** Define one shared envelope and wire a
   low-risk consumer.
4. **Pulse 04 - Signals discovery lane.** Add controlled discovery after copy
   and evidence review.
5. **Pulse 05 - Layout persistence slice.** Land a versioned named-layout
   storage contract and one restore path.
6. **Pulse 06 - Lean CLI audit/fence.** Narrow or document feature/dependency
   boundaries and add a reproducible check command.

This order starts with proof and inventory, then promotes surfaces only after the
evidence contract is in place.

---

## Validation expectations

- VTRACE docs check for planning and traceability edits.
- Focused Rust tests for any changed core, CLI, TUI, or web surface.
- No live network dependency in tests.
- `git diff --check` before committing.
- Child repo commit and push first; TRACKER records only the submodule pointer.
