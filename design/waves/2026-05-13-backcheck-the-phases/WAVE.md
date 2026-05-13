---
wave: backcheck-the-phases
date_open: 2026-05-13
status: active
source: previous trophy phases and current Jack Adams Web continuation
---

# Backcheck the Phases

## Mission

Convert the accumulated phase work into executable backfill pulses. IceLines has
strong architecture, ViewModels, tests, docs, and plans, but several waves of
work were driven conversationally. This wave makes the remaining cleanup
auditable: every residual gap gets a pulse, every pulse gets role review, and
agents receive fork files rather than vague instructions.

## Inputs

| Input | Source |
|---|---|
| Phase roadmap | `design/phases.md` |
| Phase plans | `design/plans/` |
| Surface parity | `design/specs/surface-parity.md` |
| Architecture | `design/ARCHITECTURE.md` |
| Invariants | `design/INVARIANTS.md` |
| Pitfalls | `design/PITFALLS.md` |
| Role lenses | `.roles/*.md` |
| Current product focus | Jack Adams Web, Presidents Trophy team season, fantasy and reporting surfaces |

## Opening Rule

No more "go" slices without a durable pulse. A backfill task is ready only when
it has an owner surface, explicit files or discovery scope, tests, gates, and a
role panel.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Phase inventory | Identify residual gaps across completed trophy phases. | Re-litigate already closed architecture decisions. |
| Pulse generation | Produce small, agent-executable pulse plans. | Bundle several unrelated cleanups into one giant task. |
| Fork dispatch | Materialize complete context files for agents. | Ask agents to infer context from chat history. |
| Role review | Apply `.roles` to plans and outputs. | Turn review into vague prose without actionable gates. |

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Backfill inventory and pulse map | done | `plans/pulse-01.md`; `BACKFILL-INVENTORY.md` |
| 02 - Jack Adams Web dashboard continuity | done | `plans/pulse-02.md`; `forks/pulse-02.md`; this commit |
| 03 - Presidents Trophy team season report/export parity | done | `plans/pulse-03.md` |
| 04 - Visual and CREST regression captures | planned | `plans/pulse-04.md` |
| 05 - Scenario harness classification | done | `plans/pulse-05.md`; `SCENARIO-INVENTORY.md` |
| 06 - Selke watch-rule TUI editor and UX | done | `plans/pulse-06.md` |
| 07 - Web admin operations parity | done | `plans/pulse-07.md`; `ADMIN-OPERATIONS-INVENTORY.md` |
| 08 - Career/docs parity backfill | planned | `plans/pulse-08.md` |

## Closeout Target

This wave closes when:

- every known previous-phase residual item is either assigned to a pulse,
  explicitly deferred, or deleted as obsolete;
- at least one forked pulse has been executed by an agent from a materialized
  fork packet;
- `design/waves/PHASES.md` and this `WAVE.md` reflect true status;
- focused tests and docs prove the backfill model is usable.
