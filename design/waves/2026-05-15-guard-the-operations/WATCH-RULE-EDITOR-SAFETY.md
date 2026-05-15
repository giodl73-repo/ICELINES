# Watch-Rule Editor Safety

Pulse 04 inspected the shared watch-rule model before widening any editor
surface. The safe editable contract is currently narrow:

| Surface | Supported mutation | Boundary |
|---|---|---|
| CLI | `watch player ... --save`, `watch enable`, `watch disable`, alert history | TUI/CLI commands call `WatchRuleMutationIntent` for create/toggle and preserve stored events. |
| TUI cmdbar | `watch player <name> when=<trigger>`, `watch enable|disable <id>` | TUI-command-backed only; team/deployment edit attempts return a deferred message. |
| Web watchlist | player-rule create form, enable/disable form, delete form | POST-backed only; dashboard command preserves `when=` for player rules, can toggle existing persisted rules, and rejects team/deployment edits. |
| Web JSON | list rules and enable/disable persisted rules | `/api/v1/watch-rules/set-enabled` accepts only rule id plus enabled flag. |

## Deferral

Arbitrary team/deployment editing remains deferred. `WatchRuleTrigger` can
represent some team/deployment concepts, but `WatchRuleMutationIntent` only
describes create/toggle/delete by rule id and does not carry validated team,
deployment evidence, or rule-dimension fields. The existing CLI deployment
command is therefore a preview/save shortcut, not a general editor contract for
TUI or web.

The web dashboard and TUI command bar now fence this explicitly so unsupported
phrases like `watch deployment TOR` do not silently become player rules.
