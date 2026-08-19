# IceLines documentation archive manifest

**Date**: 2026-07-22
**Status**: Active manifest

This index is the destination manifest for historical specs, plans, and review
notes moved by the documentation-consolidation plan. The canonical current
reading paths remain [`../IceLines.md`](../IceLines.md),
[`../ARCHITECTURE.md`](../ARCHITECTURE.md),
[`../specs/INDEX.md`](../specs/INDEX.md), and
[`../plans/INDEX.md`](../plans/INDEX.md).

No historical file is deleted. Every archive batch must use `git mv`, preserve
history, update inbound links in the same batch, and record the original path,
new path, final status, canonical replacement, and commit when known.

## Migration ledger

| Batch | State | Scope | Manifest |
|---|---|---|---|
| 2026-06-20 promotion and cleanup gates | Archived | Closed analytics promotion and surface cleanup micro-plans | [`plans/2026-06/INDEX.md`](plans/2026-06/INDEX.md) |
| 2026-06-21 route wording and documentation gates | Pending separate docs-only change | Closed route truth/wording micro-plans | Not moved |
| Superseded April/May drafts and reviews | Pending separate docs-only change | Superseded plans and review summaries | Not moved |
| Completed phase orchestrators | Pending portfolio review | Large completed plans no longer needed for active navigation | Not moved |

The initial card-system closeout intentionally creates this manifest without
moving files because the consolidation safety contract forbids combining a
large archive migration with implementation changes.
