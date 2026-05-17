# Wave: MDI Experiences

## Goal

Finish the shared TUI workbench experience presets so the MDI activity rail can
compose the same named rooms as the browser dashboard without falling back to
unrendered side-pane placeholders.

## Pulse table

| Pulse | Title | Status | Outcome |
|------:|-------|--------|---------|
| 01 | TUI-bound workbench rooms | done | Bound Scoring, Team, Fantasy, and Admin room presets to TUI-safe pane compositions with compact side-pane summaries. |
| 02 | Activity rail room labels | done | Made bound MDI room presets visible in the TUI activity rail and centralized TUI experience lookup. |
| 03 | Active room field strip | done | Surfaced the active room's shared workbench fields in the MDI chrome. |

## Success criteria

- Tonight, Scoring, Team, Fantasy, and Admin room presets are visible to the TUI
  workbench adapter.
- TUI-safe side-pane bindings render either native content or a compact
  field/command summary.
- MDI activity-rail activation applies the preset panes together with the
  workspace.
- The activity rail advertises bound room presets before activation.
- Active room presets advertise their shared field scope in the dashboard
  chrome.
