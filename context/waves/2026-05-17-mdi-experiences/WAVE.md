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
| 04 | Shared inspector pane cycling | done | Exposed the shared inspector pane catalog to TUI side-pane cycling with compact summaries. |
| 05 | Launch-time room presets | done | Applied bound MDI room presets when launching directly into a start workspace. |
| 06 | Activity rail selection sync | done | Kept the rail-selected workbench aligned with launch and command-driven workspace changes. |
| 07 | Activity rail viewport | done | Scrolled the rail viewport so selected lower room presets stay visible on short terminals. |
| 08 | Workbench chrome labels | done | Named Admin, Docs, and Groups in MDI chrome instead of falling back to generic Screen labels. |
| 09 | Hidden pane focus recovery | done | Moved focus back to the workspace when hiding a currently focused side pane. |
| 10 | Serve dashboard data-first shell | done | Made the browser dashboard open on compact data previews with collapsed workspace wiring and shorter top navigation. |
| 11 | Serve side panes data-first | done | Moved browser side-pane content ahead of pane selection/model controls so the columns read as data context first. |
| 12 | Serve command jump bar | done | Turned the browser command footer into a compact jump bar with examples hidden behind help. |
| 13 | Serve workspace catalog disclosure | done | Collapsed the browser workspace catalog behind a Workspaces disclosure so the default dashboard view starts with data. |
| 14 | Serve scores ribbon preview | done | Replaced the generic scores ribbon copy with real score summary chips from the Scores workspace. |
| 15 | Serve left pane leader fallback | done | Filled empty favorites/watchlist panes with a top-leaders preview so fresh dashboards still show data. |

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
- Side-pane cycling reaches the shared inspector pane catalog without falling
  back to dead placeholders.
- Launching directly into a bound workspace applies the same room preset before
  the first dashboard frame.
- Launch and command-bar workspace swaps keep the activity rail selection on the
  active workbench entry.
- Short-terminal rail rendering keeps the selected room preset visible.
- Admin, Docs, and Groups workbench destinations use named MDI chrome labels.
- Hiding a focused side pane moves focus to the visible workspace.
- The browser dashboard centers real workspace data before catalog or wiring
  affordances, with compact top navigation that stays to a single scannable row.
- Browser side panes expose favorites, watchlist, and schedule context before
  pane control affordances.
- The browser command surface stays available as a compact jump bar without
  adding another always-open menu row.
- The browser workspace catalog remains available but starts collapsed so top
  chrome no longer competes with the center data view.
- The browser scores ribbon shows real slate/game context instead of generic
  navigation copy.
- Empty browser favorites/watchlist panes fall back to top leaders instead of
  leaving the side column content-free.
