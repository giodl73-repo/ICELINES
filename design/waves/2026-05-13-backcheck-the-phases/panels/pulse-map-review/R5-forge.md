# R5 Review - forge

## Findings

### F-01 - WARN: Pulse 06 likely files risk scope creep into scoring/model changes
File: `design/waves/2026-05-13-backcheck-the-phases/plans/pulse-06.md`
Finding: Pulse 06 is a TUI/watch UX backfill, but the likely files include core poach ViewModel, web dashboard, and web poach handlers. That is acceptable for inventory or adapter wiring, but risky if it changes `PoachScore` semantics.
Consequence: A UX pulse could accidentally alter the fantasy recommendation model and invalidate Selke scoring fixtures.
Fix: Treat `icelines-core/src/view_model/poach.rs` as read/adapter-only unless a stop-condition follow-up ViewModel pulse is created. Any scoring change must be its own Selke model pulse with known-value tests.

### F-02 - NOTE: Fork packets should be materialized before dispatch
File: `design/waves/2026-05-13-backcheck-the-phases/WAVE.md`
Finding: The wave's mission says agents receive fork files rather than vague instructions. Pulse 03-08 plans exist, but fork packets have not been materialized yet.
Consequence: Running agents directly against plan files risks missing governing roles, execution contract, and recent panel findings.
Fix: Create `forks/pulse-03.md` through `forks/pulse-08.md` before implementation dispatch, and include this panel directory in each fork's context.
