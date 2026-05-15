# Operations Parity Inventory

Pulse 01 reviewed the post-Compose partials in `surface-parity.md` and prior
Backcheck inventories. This wave owns only operational/product-UX gaps where a
user could otherwise misunderstand persistence, mutation safety, or available
editing affordances.

## Residual Map

| Residual | Source | Decision | Pulse | Notes |
|---|---|---|---|---|
| Persistent web config/report toggles | `surface-parity.md` config/report row; Backcheck admin inventory | pulse | 02 | Web admin currently mutates runtime web season/type keys. Persistent report-toggle UI remains planned and must share CLI/TUI config contracts. |
| Web data install/remove safety | `surface-parity.md` data row; Backcheck admin inventory | done | 03 | Install remains deferred because it is live/network release work; remove remains deferred because it is destructive filesystem mutation. `/admin` now renders explicit deferral copy and install/remove routes remain unmounted. |
| Admin game-cache load evidence | `surface-parity.md` admin route rows | done | 03 | Game-cache routes are labeled as POST-backed cache warmers, not release data install/remove. Invalid requests are rejected before network work. |
| Snapshot activate/delete safety | `surface-parity.md` snapshot row | watch | 03 | Already POST-backed with active-snapshot guard. Re-audit only if Pulse 03 touches admin forms. |
| Watch-rule arbitrary team/deployment editing | `surface-parity.md` watch rules row | pulse | 04 | Player-rule create/toggle/delete exists; richer rule dimensions are deferred. Pulse must stop if needed fields are not present in `WatchRulesView` or mutation intents. |
| Favorites/groups management | `surface-parity.md` favorites/groups row | pulse | 05 | Add/remove favorites exists. Group-management parity needs a concrete UX decision around group selection/create/remove without changing identity semantics casually. |
| Career TUI cohort board | `surface-parity.md` career row; `CAREER-DOCS-INVENTORY.md` | done | n/a | Handoff-only is deliberate: TUI points to canonical CLI/web cohort surfaces because local career-history data is unbundled. |
| Fantasy main-dashboard mutations | `surface-parity.md` fantasy row | defer | n/a | CLI and legacy fantasy server remain write surfaces. Not part of operations hardening unless a later fantasy wave reopens it. |
| TUI poach/weekly report viewer | `surface-parity.md` poach report row | defer | n/a | Cmdbar handoff is explicit. No separate viewer in this wave. |
| Dashboard metadata-only pane stubs | Compose closeout | done | n/a | Web/TUI now show explicit unavailable stubs rather than mismatched bodies. |

## Pulse Map

| Pulse | Owner surfaces | Owned files / discovery scope | Gates |
|---|---|---|---|
| 02 - Persistent config/report toggle contract | Web admin, config contract docs | `icelines-web/src/handlers/admin.rs`; admin templates/styles/tests; config/report helpers in CLI/core/fetch as needed; `surface-parity.md` | `cargo fmt --check`; `cargo test -p icelines-web --quiet`; `cargo clippy -p icelines-web --no-deps -- -D warnings`; proof on touched docs |
| 03 - Admin data operation safety | Web admin, admin JSON, docs | `icelines-web/src/handlers/admin.rs`; admin route tests; Backcheck admin inventory; `COMMANDS.md` if behavior changes | `cargo fmt --check`; `cargo test -p icelines-web --quiet`; `cargo clippy -p icelines-web --no-deps -- -D warnings`; proof on wave docs |
| 04 - Watch-rule editor parity | TUI cmdbar/watchlist, web watchlist, shared mutation intents | `icelines-core` watch view/intents if needed; `icelines-cli/src/tui/command.rs`; `icelines-web/src/handlers/poach.rs`; tests | `cargo fmt --check`; `cargo test -p icelines-core --quiet`; `cargo test -p icelines-cli --quiet`; `cargo test -p icelines-web --quiet`; focused clippy for touched crates |
| 05 - Favorites/groups parity | Web favorites/dashboard side pane, TUI favorites affordance, group DB | `icelines-core` favorites views/intents if needed; `icelines-cli` group/favorites surfaces; `icelines-web/src/handlers/favorites.rs`; dashboard side panes | `cargo fmt --check`; focused core/cli/web tests; clippy for touched crates |
| 06 - Docs, regression gates, and closeout | Docs and wave records | `README.md`; `COMMANDS.md`; `design/specs/surface-parity.md`; wave docs | full closeout proof; focused crate gates from Pulses 02-05; release smoke or release build as appropriate |

## Stop Conditions

- Stop if a pulse needs a ViewModel or mutation-intent field that does not exist
  and adding it would turn the pulse into a broad schema redesign.
- Stop if a web control would mutate state via GET.
- Stop if data install/remove tests require live network or real user files.
- Stop if a favorites/groups change requires a database migration not already
  scoped in the pulse plan.
