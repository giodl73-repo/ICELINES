# Campbell specs role review

**Date**: 2026-05-09
**Reviewed specs**:

- `design/specs/platform-contracts.md`
- `design/specs/viewmodels.md`
- `design/specs/surface-parity.md`

**Role source**:

- `.roles/ROLE.md` for role ordering, tensions, and tiebreakers.
- `.roles/hart.md`, `.roles/keel.md`, `.roles/tape.md`,
  `.roles/forge.md`, `.roles/pace.md`, `.roles/bench.md`,
  `.roles/edge.md`, `.roles/wire.md`, `.roles/scout.md`,
  `.roles/glass.md`, `.roles/crest.md`, and `.roles/broadcast.md`.

## Verdict

The Campbell direction is sound. The review found no reason to change the
architecture: `StatsRepository + typed intent -> ViewModel -> renderer` remains
the right uniformity layer for CLI, TUI, web, JSON, reports, and static output.

The main fixes were contract hardening before implementation starts.

## Role findings applied

| Role | Finding | Applied fix |
|---|---|---|
| HART | Cache identity needed more than season/type once filters and windows enter. | Contracts now require query/filter/sort/window signature and source/data generation where available. |
| KEEL | Static site/export was not first-class in the parity contract. | Surface parity now names static/export artifacts and adds a dedicated coverage table. |
| TAPE | Source provenance and stale/missing states needed to survive as typed state. | `SourceState` now includes provenance, stale reason, source kind, and mapping from repository missing state. |
| FORGE | Display labels were fine, but canonical keys should not become strings forever. | ViewModel spec now calls out typed filter/sort/stat keys where enums or IDs exist. |
| PACE | Numeric rendering policy was underspecified. | Added `MetricCell` with unit, precision, value, and token policy. |
| BENCH | Fixture expectations needed sharper failure modes. | Tests now require serialization survival, duplicate/empty/stale cases, and renderer no-recompute checks. |
| EDGE | Empty and warning states needed recovery affordances. | Added `RecoveryAction` to empty states and warnings. |
| WIRE | JSON projections needed to preserve contract fields. | JSON renderer responsibility now includes schema version and no-loss projection rules. |
| SCOUT | Depth claims needed actual-vs-estimated deployment evidence. | Team depth rows now preserve actual/estimated/unknown evidence and distinguish current vs historical depth. |
| GLASS | Semantic tokens cannot rely on color alone. | Visual contract now requires text/glyph/badge/label equivalents across surfaces. |
| CREST | Aesthetic polish needs semantic material without leaking renderer styling into core. | ViewModel tokens stay semantic, giving Prince enough structure for intentional composition later. |
| broadcast | Web pages and HTMX fragments can silently lose context if they render as route-local views. | Surface parity now requires active context, applied state, accessible labels, and bookmarkable state checks for web HTML/partials. |

## Implementation implications

- Campbell.2 should build the shared types first, even if the first ViewModels
  are thin.
- Campbell.3 should avoid renderer-owned rounding and classification from day
  one.
- Ted Lindsay should treat static/export parity as a surface, not a docs
  afterthought.
- Jim Gregory should add release gates for fixture parity once Campbell.4 lands.
