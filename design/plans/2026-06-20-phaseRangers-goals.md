# Phase Rangers - Post-Hurricane goals

> Phase Rangers is the post-Hurricane organization round: take the shipped
> analytics surfaces and make them easier to find, trust, persist, and run in
> a lean offline workflow. It does not reopen Hurricane's blocked source claims
> unless the missing evidence is added first.

**Created:** 2026-06-20
**Status:** Wrapped 2026-06-20

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
| 1 | **Rangers Goal 1 - Signals discovery lane** | Hurricane shipped Signals, but they remain mostly player-card/export driven. Users need a controlled path to find inspectable roster evidence. | Shipped in pulse 04: `icelines signals-roster --team NYR` and `--json` render a team-scoped matrix with methodology/non-claim copy, unavailable evidence states, and no zero-filled missing values; catalog/filter/leaderboard/cache promotion remains explicitly disallowed. |
| 2 | **Rangers Goal 2 - Evidence card envelope reuse** | WP-009 already has selected analytics cache/evidence-card consumers. Rangers should reuse that contract instead of creating a second evidence model. | Passed in pulse 05: `signals-roster` remains outside analytics cache because Signals have no accepted cache metric keys; future bridge work requires a separate Signals cache-promotion gate. |
| 3 | **Rangers Goal 3 - Workbench layout hardening** | WP-002 already shipped named layout persistence with accepted risk. Rangers should use or harden it, not rebuild it. | Passed in pulse 06: `scripts/rangers-layout-proof.ps1` saves, lists, shows, and deletes a temp-home `rangers-stats` layout while asserting stable pane IDs and preserve-active-context policy. |
| 4 | **Rangers Goal 4 - Lean offline CLI path** | REQ-DEP-001 and REQ-LEAN-001 remain target states. A lean CLI gives the repo a cleaner distributable story. | Passed in pulse 07 as a target-not-met audit: `scripts/rangers-lean-audit.ps1` verifies FLETCH/SLICE seams, FLETCH command surfaces, SLICE selector usage, and missing `cli` feature without claiming lean support. |
| 5 | **Rangers Goal 5 - Rangers team workflow proof** | The round needs one concrete user workflow instead of abstract platform cleanup. NYR can serve as a representative team path using existing bundled data. | Shipped in pulse 02: `scripts/rangers-workflow.ps1` runs team depth, leaders, goalie workload, player Signals, team export, and Signals export offline with disclosure assertions and no team-specific hardcoded claims. |

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
   Result: passed 2026-06-20; see `scripts/rangers-workflow.ps1`.
3. **Pulse 03 - Signals discovery design gate.** Decide the allowed discovery
   shape before implementation. Result: passed 2026-06-20; use a roster matrix,
   not a leaderboard.
4. **Pulse 04 - Signals roster matrix.** Add controlled team-scoped discovery
   after copy and evidence review. Result: passed 2026-06-20.
5. **Pulse 05 - Evidence-envelope bridge decision.** Decide whether
   `signals-roster` should bridge into WP-009 cache/evidence-card envelopes.
   Result: passed 2026-06-20; no cache bridge until a separate Signals
   cache-promotion gate.
6. **Pulse 06 - Layout persistence hardening proof.** Use existing WP-002 layout
   persistence in an isolated Rangers proof. Result: passed 2026-06-20.
7. **Pulse 07 - Lean CLI audit/fence.** Narrow or document feature/dependency
   boundaries and add a reproducible check command. Result: passed 2026-06-20
   as target-not-met audit; no lean support claimed.

This order started with proof and inventory, then promoted surfaces only after
the evidence contract was in place.

---

## Closeout

Phase Rangers is wrapped. All planned post-Hurricane goals have been dispositioned:

- The NYR workflow proof shipped through `scripts/rangers-workflow.ps1`.
- Signals discovery shipped as the gated, team-scoped `signals-roster` matrix.
- The evidence bridge decision keeps Signals roster discovery outside the
  analytics cache until a separate Signals cache-promotion gate exists.
- Layout persistence has an isolated temp-home save/list/show/delete proof.
- Lean CLI support remains target-not-met, with a reproducible audit rather than
  an unsupported standalone claim.

No additional Rangers implementation pulse remains. Follow-on work should start
a new phase or wave for Signals cache/catalog/filter/leaderboard promotion,
dependency surgery for lean CLI support, or live browser/interactive TUI proof.

---

## Validation expectations

- VTRACE docs check for planning and traceability edits.
- Focused Rust tests for any changed core, CLI, TUI, or web surface.
- No live network dependency in tests.
- `git diff --check` before committing.
- Child repo commit and push first; TRACKER records only the submodule pointer.
