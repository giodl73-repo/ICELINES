# R4 Review - forge

## Findings

### F-01 - WARN: Do not move I/O policy into `icelines-core`
File: `design/waves/2026-05-15-guard-the-operations/plans/pulse-02.md`
Finding: Config/report persistence and data-operation safety may tempt broad helpers in core.
Consequence: Putting filesystem, web, or network policy in `icelines-core` would violate the crate dependency contract.
Fix: Keep pure types/intents in core if needed; place filesystem and web handling in CLI/web/fetch layers that already own I/O.

### F-02 - NOTE: Prefer existing intent/result types
File: `design/waves/2026-05-15-guard-the-operations/OPERATIONS-PARITY-INVENTORY.md`
Finding: The inventory names `MutationResultView` and existing mutation intents as the path for writes.
Consequence: Reuse reduces stringly typed handlers and keeps error rendering consistent.
Fix: Extend typed intents only where necessary; avoid ad-hoc handler-local command parsing.
