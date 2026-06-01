# Wave: VTRACE WP-002 Layout Persistence

## Goal

Execute `WP-002` as a controlled VTRACE implementation wave: durable named
workbench layout persistence through shared schema, CLI management, TUI restore,
Web bookmark restore, and evidence updates.

## Pulse table

| Pulse | Title | Status | Outcome |
|------:|-------|--------|---------|
| 01 | Named layout schema and restore surfaces | closed_with_risk | L0 and L2 evidence pass; L1 workspace clippy remains blocked by unrelated existing lint debt; WP-002 close review accepted the risk. |

## Success criteria

- `WP-002` stays linked to `REQ-WB-003`, `IF-LAYOUT-001`, `VAL-010`,
  `EVID-VAL-010`, and `EVID-CR-008`.
- Shared layout state is versioned and validated before restore.
- TUI and Web use the shared layout record instead of renderer-local semantic
  state.
- Local-state, browser URL state, trace, verification, validation, and review
  evidence are updated before any unrestricted `REQ-WB-003` pass claim.
- TRACKER submodule pointer updates remain separate from ICELINES child-repo
  implementation commits.
