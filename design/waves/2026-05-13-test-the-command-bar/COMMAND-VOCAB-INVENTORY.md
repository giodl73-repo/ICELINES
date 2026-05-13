# Command vocabulary inventory

Pulse 03 compared the user-testing protocol commands against current TUI and
web command coverage.

## Gate results

- `cargo test -p icelines-cli --bin icelines l0_adams_parse` ran 41 matching
  parser tests and passed.
- `cargo test -p icelines-cli --bin icelines l0_adams_exec` ran 25 matching
  execution tests and passed.
- `cargo test -p icelines-web dashboard_command` ran 8 matching web command
  tests and passed.

## Protocol command classification

| Protocol command/task | Classification | Evidence |
|---|---|---|
| `:stats` / `:goalies` | Opens TUI workspace; web maps to internal route. | `l0_adams_parse_workspace_no_arg_verbs`, `l0_adams_exec_screen_swaps`, `l0_dashboard_command_navigation_examples_resolve_to_internal_routes` |
| `:poach rw cats=hits,blocks free top=12` | Opens/filters fantasy poach workspace. | `l0_adams_parse_poach_filters`, `l0_adams_exec_poach_kv_applies_filters` |
| `:simulate add=... drop=... weeks=3` | Applies fantasy simulation scenario. | `l0_adams_parse_fantasy_simulation_scenario`, `l0_adams_exec_fantasy_sim_kv_applies_scenario` |
| `/hide schedule` / `/show schedule` | Toggles panes; not a data mutation. | `l0_adams_parse_hide_show_panes`, `s047`-`s052`, `s061`-`s065` |
| `:team EDM` | Opens team depth workspace. | `l0_adams_parse_team_uppercases`, `l0_adams_exec_team_lands_on_team_screen` |
| `:team EDM season` | Opens team season-performance view. | `l0_adams_parse_team_season_distinct_variant`, `l0_adams_exec_team_season_lands_on_schedule_team` |
| `:career league=OHL season=20142015 top=8` | Flashes canonical CLI/web target; no dedicated TUI board. | `l0_adams_parse_career_cmdbar_handoff`, `l0_adams_exec_career_cmdbar_handoff_flashes_targets` |
| `:compare McDavid vs Crosby` | Head-to-head handoff flashes CLI/web targets. | `l0_adams_parse_compare_alias_vs`, `l0_adams_exec_compare_head_to_head_handoff_flashes_targets` |
| `/fav add <name>` | Safe favorite mutation path. | `l0_adams_parse_fav_add`, web command mutation tests |
| `:watch <name>` | Safe watch handoff; web mutation is POST-backed. | `l0_adams_parse_watch_cmdbar_handoff`, `l0_adams_exec_watch_cmdbar_handoff_flashes_targets`, `l0_dashboard_command_mutations_are_post_intents_not_get_routes` |
| Invalid command | Editable parse/flash recovery. | `l0_adams_parse_error_display_is_user_friendly`, `s081_garbage_then_correct_recovers`, `l1_dashboard_command_rejects_unknown_without_redirecting` |

## Testing implication

The command vocabulary is broad enough for user testing now. The highest-risk
human questions are not parser coverage; they are whether users discover the
grammar, understand handoff-only commands, and can predict focus/tab behavior.
