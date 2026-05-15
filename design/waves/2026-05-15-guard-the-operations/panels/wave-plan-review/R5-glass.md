# R5 Review - glass

## Findings

### F-01 - WARN: Controls must disclose whether they persist
File: `design/waves/2026-05-15-guard-the-operations/plans/pulse-02.md`
Finding: Runtime web config and persistent CLI/TUI config can look identical to users.
Consequence: Users may believe report toggles survive restart when they do not, or avoid useful controls because their scope is unclear.
Fix: Label runtime-only controls, persistent controls, and deferred controls directly in admin UI and docs.

### F-02 - WARN: Disabled or deferred operations need actionable copy
File: `design/waves/2026-05-15-guard-the-operations/plans/pulse-03.md`
Finding: Data install/remove may remain deferred.
Consequence: A missing button or generic "coming soon" label leaves users unsure whether to use CLI, web, or nothing.
Fix: If deferred, render the canonical CLI path and the reason: live/network install or destructive remove requires safer confirmation.
