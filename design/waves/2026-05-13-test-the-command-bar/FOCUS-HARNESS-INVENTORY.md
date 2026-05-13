# Focus harness inventory

Pulse 02 verified the current command-bar focus model against the existing MDI
persona harness.

## Gate result

`cargo test -p icelines-cli --bin icelines persona_jack_adams` ran 100 matching
tests and passed.

## Focus behavior already fenced

| Behavior | Representative tests |
|---|---|
| `:` focuses the command bar | `s066_colon_focuses_bar` |
| `/` focuses with slash prefilled | `s067_slash_focuses_bar_with_slash_prefilled` |
| `Esc` clears and defocuses | `s068_escape_clears_bar_and_defocuses` |
| Backspace defocuses at empty | `s069_backspace_pops_then_defocuses_at_empty` |
| Per-screen keys are blocked while bar is focused | `s070_cmdbar_per_screen_keybind_blocked_while_focused` |
| Per-screen keys resume after defocus | `s071_per_screen_keybind_works_when_bar_unfocused` |
| Help overlay can open and dismiss | `s041_slash_help_opens_overlay`, `s072_help_overlay_dismisses_on_any_key` |
| Help does not break resumed typing | `s073_help_then_resume_typing` |
| MDI tab behavior is intentional | `s074_tab_in_mdi_is_noop` |
| SDI tab behavior still cycles | `s075_tab_in_sdi_cycles` |
| Pane toggles work during command focus | `s065_pane_toggle_during_cmdbar_focus` |
| Error recovery remains editable | `s081_garbage_then_correct_recovers`, `s082_parse_error_input_editable` |
| Long commands and resize bursts do not panic | `s097_resize_burst_no_panic`, `s098_long_command_line_no_panic` |

## Testing implication

The automated harness is strong enough to protect focus behavior while human
sessions test discoverability. If participants cannot discover `:`, `/`, `Esc`,
or pane toggles, treat that as a help/affordance problem rather than an unfenced
state-machine problem.
