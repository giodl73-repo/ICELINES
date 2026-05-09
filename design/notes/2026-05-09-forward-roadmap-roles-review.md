# Forward roadmap roles review

**Date**: 2026-05-09
**Scope**: Jennings, Campbell, Selke, Messier, Lester Patrick, Ted Lindsay,
Prince of Wales, Jim Gregory, and `design/specs/platform-contracts.md`.

---

## Verdict

The roadmap is architecturally sound after the Campbell insertion. The highest
risk is no longer "wrong order"; it is overconfidence:

- treating a ViewModel as a formatter DTO instead of a contract boundary;
- treating `PoachScore` as truth instead of a scored recommendation with
  evidence and confidence;
- letting reports become a second data path;
- allowing web/CLI/TUI to drift after Campbell.

The fixes below were folded into the plans.

---

## Role findings

| Role | Finding | Resolution |
|---|---|---|
| HART | Campbell must make `(season, season_type)` unavoidable in ViewModel context and cache identity. | Already gated in Campbell; retained as blocking. |
| KEEL | Reports were missing from the platform-contract list, which would let markdown/HTML/JSON reports drift from CLI/TUI/web. | Added a report-generation contract to `platform-contracts.md`; Selke reports must render from `PoachReportView`. |
| TAPE | Selke could confuse missing line/PP/ownership data with negative evidence. | Added `DeploymentSignal` and `AvailabilityState`; `Unknown` cannot subtract from score by itself. |
| FORGE | `PoachScore` can become an ad hoc score bag. | Selke requires typed components, units, clamps, weights, and explanation rows. |
| PACE | Poacher weights and confidence need explicit measured/estimated/deferred status. | Selke.1 requires component status and confidence labels. |
| BENCH | ViewModel and report reproducibility need fixture-level tests. | Campbell requires contract fixtures; platform report contract requires fixed fixture/clock reproducibility. |
| EDGE | Magic-score overconfidence is the main product failure. | Selke requires explanation lists, unavailable-component disclosure, confidence tags, and closeout pitfalls. |
| WIRE | Optional ownership/import/live sources must not make default tests flaky. | Selke keeps optional imports graceful and no live dependency in default tests. |
| SCOUT | "Player poacher" only works if deployment context is hockey-sensible. | Selke preserves line/PP/TOI/role as separate evidence, not just raw points. |
| GLASS/Broadcast | A single score is not enough for action. | Selke board/report must show "why this player" and risk/confidence at a glance. |

---

## Remaining watch items

1. Campbell should create `design/specs/viewmodels.md` before implementing broad
   migrations.
2. Ted Lindsay should create `design/specs/surface-parity.md` before changing
   route behavior.
3. Selke should create `design/specs/fantasy-poacher.md` before code; this is
   where weights, confidence, and missing-data behavior become concrete.
4. Prince of Wales should create `design/specs/visual-system.md` before visual
   redesign.
5. Jennings should refresh `PITFALLS.md` and `INVARIANTS.md` so new Campbell and
   Selke contracts have institutional memory.

---

## Final order

```text
Jennings
Campbell
Selke / Messier
Lester Patrick
Ted Lindsay
Prince of Wales
Jim Gregory
```

Selke can start after Campbell on CLI/report/ViewModel work. Full TUI/web
polish waits for Messier/Ted Lindsay/Prince of Wales as appropriate.
