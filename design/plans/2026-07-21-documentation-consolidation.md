# Documentation Consolidation — Plan

**Date**: 2026-07-21
**Status**: Active
**Specification**: None — process-only documentation migration
**Archive when**: the archive migration, reading-order rewrite, and zero-drift
documentation gate are complete
**Scope**: IceLines `design/` documentation only
**Trigger**: 57 spec files and 156 plan files at the 2026-07-21 audit

## Current audit checkpoint

The reproducible audit in `scripts/documentation-audit.ps1` now reports 216
documents: 57 specs, 157 plans, and two indexes. After the card-system closeout,
the canonical active set is four plans; the completed card plan remains in the
roadmap ledger but not the active table. Two older plans classify as
superseded, and no local design links are broken. It also identifies 146 files
with legacy header or index issues for the archive migration; those known
issues are why strict zero-drift mode is a later-wave gate rather than a claim
of current cleanliness.

## Outcome

Reduce the active IceLines reading surface to a small, truthful canonical set
without deleting historical design evidence. Closed micro-plans move to an
indexed archive, active workstreams gain clear parents, duplicate status text
is eliminated, and automated checks prevent renewed sprawl.

This is a documentation migration, not a rewrite of product history. Git
history, closeout evidence, and phase names remain discoverable.

## Problems to Solve

1. The plans directory mixes current roadmaps, implemented orchestrators,
   superseded drafts, review summaries, and dozens of closed micro-gates.
2. Several old files still say Draft even though their index row says
   Implemented.
3. Product truth is repeated across `IceLines.md`, `ARCHITECTURE.md`, specs,
   plans, notes, `README.md`, `COMMANDS.md`, and surface matrices.
4. Recent active fantasy and IceCast work has grown inside large chronological
   plans that now mix design, execution log, and measured results.
5. A new feature can add a plan without naming a parent, specification,
   superseded document, or archive destination.
6. Indexes are manually curated and can drift from file headers.

## Canonical Documentation Model

### Level 1 — Product orientation

| Document | Owns |
|---|---|
| `design/IceLines.md` | product mission, users, surfaces, current portfolio |
| `design/ARCHITECTURE.md` | crate boundaries, data flow, invariants |
| `design/specs/INDEX.md` | specification registry and status |
| `design/plans/INDEX.md` | active roadmap and archive entry point |

These documents summarize and link. They do not duplicate complete feature
schemas or implementation journals.

### Level 2 — Durable specifications

One specification owns the stable contract for a product/domain capability.
Specs describe what must be true, not chronological implementation progress.

Required header:

```text
Version:
Date:
Status: Draft | Accepted | Implemented | Implemented (partial) | Deferred | Retired
Owner domain:
Parent contracts:
Supersedes: optional
```

### Level 3 — Active implementation plans

Plans sequence work against one or more specs. At most eight plans may appear
in the active-roadmap table without an explicit portfolio review.

Required header:

```text
Date:
Status: Planned | Active | Blocked | Complete | Superseded
Specification:
Parent plan: optional
Supersedes: optional
Archive when: completion condition
```

Active plans contain milestones, gates, dependencies, risks, and current
status. Detailed daily execution logs move to `design/notes/` or
`context/waves/`.

### Level 4 — Historical archive

Closed, superseded, and review-only plan files move under:

```text
design/archive/plans/YYYY-MM/
design/archive/reviews/YYYY-MM/
```

`design/archive/INDEX.md` retains title, original path, final status, date,
replacement/canonical document, and final commit when known.

Moving files must preserve Git history through `git mv`. Links are rewritten
and checked before the migration commits. No historical plan is deleted merely
to reduce count.

## Source-of-Truth Rules

| Information | Canonical owner |
|---|---|
| Commands and flags | `COMMANDS.md` plus Clap tests |
| Product mission/surfaces | `design/IceLines.md` |
| Crate/data architecture | `design/ARCHITECTURE.md` |
| Stable feature behavior/schema | feature spec |
| Work order/status | active plan and plan index |
| Surface parity | `design/specs/surface-parity.md` |
| Visual tokens/review | `design/specs/visual-system.md` |
| Measured run/report result | dated note or report artifact |
| Historical implementation narrative | archived plan or `context/waves/` |

Other documents link to the owner and summarize only what their audience needs.

## Consolidation Waves

### Wave A — Machine-readable inventory

- Enumerate every spec, plan, note, and archive candidate.
- Parse title, date, status, parent/spec links, and inbound links.
- Flag missing/contradictory headers, duplicate titles, broken links, and files
  not represented in an index.
- Produce `design/documentation-inventory.json` as a generated audit artifact
  or a script output, not a hand-maintained truth source.

Exit: every one of the current 213 spec/plan files has a classification.

### Wave B — Active-set decision

- Select the current durable specs.
- Limit the plan index active table to true current workstreams.
- Mark completed-but-open-looking documents for archive.
- Mark superseded plans with their replacement before moving them.
- Resolve the overlapping fantasy assistant/war-room responsibilities.
- Keep Team Season Forecast and UI-Neutral Cards as distinct active plans but
  link their boundary explicitly.

Proposed active plan set after review:

1. UI-Neutral Card System;
2. Team Season Forecast;
3. Fantasy War Room (absorbing the older assistant execution plan);
4. Phase Canadiens forward stats roadmap; and
5. Documentation Consolidation.

Additional plans require a named reason and parent.

### Wave C — Specs consolidation

- Keep platform contracts, ViewModels, visual system, and surface parity as
  cross-cutting parents.
- Keep one durable spec per major feature family.
- Merge status-only or delta-only spec fragments into their parent where doing
  so does not erase a distinct contract.
- Retire cancelled specs to the archive with a visible non-claim.
- Add `ui-neutral-card-system.md` and link rather than copy its schema into
  forecast/fantasy/web/TUI specs.
- Refresh all spec headers and the spec health table.

Exit: every active spec has a unique purpose and parent relationship.

### Wave D — Plans archive migration

- Move closed micro-gates and review summaries in month-sized batches.
- Start with the large 2026-06-20/21 closed route-wording and promotion gates.
- Preserve a manifest and rewrite inbound links in the same batch.
- Run link checks and `git diff --check` after every batch.
- Keep large historical orchestrators archived, not summarized into active
  plans.

Suggested batches:

1. June 20 promotion/cleanup gates;
2. June 21 route-wording/documentation gates;
3. superseded April/May drafts and review summaries;
4. completed phase orchestrators no longer needed in active navigation; and
5. remaining one-off closed plans.

Exit: `design/plans/` contains active plans plus a small number of canonical
implemented milestone plans; history lives under `design/archive/`.

### Wave E — Extract execution journals

- Remove long dated status narratives from active plans when they obscure the
  remaining work.
- Move exact run results, bug diaries, calibration measurements, and historical
  recovery logs to dated notes or `context/waves/`.
- Replace removed sections with concise current status and links.
- Keep acceptance criteria and unresolved gates in the active plan.

Priority candidates:

- `2026-07-19-team-season-forecast.md`;
- `2026-07-18-fantasy-war-room-roadmap.md`; and
- any future card-system implementation journal.

### Wave F — Automated documentation gates

Add a lightweight repository script/test that checks:

- every active spec/plan has the required header;
- every indexed link resolves;
- every active file appears exactly once in its index;
- archived files do not appear in the active table;
- statuses use the canonical vocabulary;
- no plan is active without a linked spec or documented exception;
- no spec embeds a second copy of a public JSON field list owned elsewhere;
- active-plan count is within policy; and
- generated inventory is current when the gate is invoked.

The gate reports drift first. It becomes blocking only after the initial
migration reaches zero known violations.

### Wave G — Final reading-order rewrite

Provide four short paths:

- new contributor;
- feature implementer;
- renderer/surface implementer; and
- historical/research reader.

The default reader should reach current product truth without entering the
archive.

## Migration Safety

- Use `git mv`; never copy/delete historical files manually.
- Rewrite links mechanically and review the diff.
- Do not combine code changes with large archive moves.
- Do not update the TRACKER submodule pointer until child changes are committed
  and explicitly approved for a portfolio pulse.
- Preserve unrelated dirty worktree changes.
- Split archive batches so failures are reversible without reset/checkout.
- Record old-to-new paths in the archive manifest.

## Acceptance Criteria

1. Every spec and plan is classified as active, canonical implemented,
   archived, superseded, or retired.
2. Active plan navigation contains no more than eight entries.
3. Every active plan names its specification and completion/archive condition.
4. Every durable spec has one unique contract purpose.
5. Closed June micro-gates are absent from the active plans directory and
   discoverable through the archive index.
6. Current product, architecture, command, parity, and visual truth each have
   one named owner.
7. Team forecast, fantasy, and card plans link instead of copying contract
   definitions.
8. All repository-local documentation links resolve after each migration
   batch.
9. The documentation audit gate reports no unknown active files or status
   contradictions.
10. Historical evidence remains accessible and Git history is preserved.

## Non-Goals

- deleting historical design work;
- rewriting every old document to modern terminology;
- moving product documentation into TRACKER;
- changing code behavior during archive batches; or
- making generated inventory the only human-readable index.

## Recommended Execution Order

Run Waves A-B before card-system implementation begins in earnest. Complete
Wave C while the card spec is still fresh. Perform Wave D as separate
documentation-only commits between feature slices. Waves E-G close after the
active feature plans have stable remaining-work sections.
