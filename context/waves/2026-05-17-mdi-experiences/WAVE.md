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
| 16 | Serve right pane schedule preview | done | Replaced generic right-pane schedule links with real schedule preview rows and collapsed links under Schedule views. |
| 17 | Serve unified navigation drawer | done | Merged separate Workspaces and Rooms controls into one collapsed Navigation drawer. |
| 18 | Serve clickable data preview | done | Made center workspace preview rows link to the full workspace so the data table itself is actionable. |
| 19 | Serve score chip deep links | done | Carried game ids into score rows so dashboard score chips and score-table cells link to game pages. |
| 20 | Serve schedule deep links | done | Carried game ids into schedule rows so right-pane schedule previews link to game pages. |
| 21 | Serve leader deep links | done | Linked leader preview rows to player cards instead of the generic Leaders page. |
| 22 | Serve row-specific preview links | done | Made center preview rows prefer row-specific hrefs before falling back to the workspace URL. |
| 23 | TUI MDI scaffolding cleanup | done | Removed stale placeholder scaffolding, refreshed MDI comments, and made pane-cycle tests follow the shared binding catalog. |
| 24 | TUI MDI Tab cleanup | done | Retired stale Tab no-op commentary and made the no-arg workbench mapper explicitly test-only. |
| 25 | TUI event scaffolding cleanup | done | Removed broad event-action dead-code suppression, added event-map tests, and refreshed the screen dispatch seam comment. |
| 26 | TUI broad-suite cleanup | done | Made the focused pane-cycle app test follow the shared right-pane binding catalog so the broad TUI suite passes. |
| 27 | TUI broad-suite hardening | done | Updated legacy MDI Tab tests to assert focus traversal and isolated command DB tests from process HOME races. |
| 28 | TUI release warning cleanup | done | Scoped test-only TUI helpers/imports so the release build stays warning-clean after cleanup. |
| 29 | TUI full CLI validation | done | Ran the full `icelines-cli` binary test target after cleanup and release hardening. |
| 30 | Serve wide dashboard layout | done | Let the browser dashboard break out of the global page width cap and prioritize the center workspace on wide screens. |
| 31 | Serve center card expansion | done | Changed the dashboard workspace partial from nested `main` to `section` so it no longer inherits the global page width cap. |
| 32 | Serve full leaders workspace | done | Embedded the full Leaders table/filter surface in the browser dashboard center workspace instead of the compact preview. |
| 33 | Serve workspace-local navigation | done | Made same-origin dashboard links and GET filters swap the center workspace while preserving side panes and command chrome. |
| 34 | Serve full player workspace | done | Embedded the full player card in dashboard player workspaces instead of the compact player summary preview. |
| 35 | Serve navigation QA hardening | done | Tightened workspace routing so unsupported app links fall back cleanly instead of no-oping or swapping to the wrong center. |
| 36 | Serve full team workspace | done | Embedded the full team roster in dashboard team workspaces instead of the compact team summary preview. |
| 37 | Serve full team season workspace | done | Embedded the full team season page in dashboard team-season workspaces instead of the compact season summary preview. |
| 38 | Serve full slate workspaces | done | Embedded the full Scores and Schedule pages in dashboard slate workspaces instead of compact previews. |
| 39 | Serve full goalie and depth workspaces | done | Embedded the full Goalies and Depth pages in dashboard stat workspaces instead of compact previews. |
| 40 | Serve pane-target navigation | done | Added modifier-click navigation so dashboard links can pin previews into left or right panes while preserving the center workspace. |
| 41 | Serve pane navigation state hardening | done | Preserved composed room and side-pane state when center swaps or pane pins rewrite dashboard URLs. |
| 42 | Serve composition pinned-pane hardening | done | Preserved pinned pane URLs when room and pane-control composition links navigate with full dashboard GETs. |
| 43 | Serve server-side pinned-pane links | done | Preserved pinned pane URLs in server-rendered room and pane-control hrefs for no-JS and open-in-new-tab flows. |
| 44 | Serve pinned pane actions | done | Added Open in center, Swap with center, and Clear pin actions to pinned pane headers. |

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
- The browser schedule side pane starts with real schedule rows before exposing
  secondary schedule links.
- Browser workspace and room navigation share one collapsed drawer instead of
  separate top chrome rows.
- Browser center preview rows link directly to their full workspace.
- Browser score previews link to specific game pages when a game id is
  available.
- Browser schedule previews link to specific game pages when a game id is
  available.
- Browser leader previews link to player cards when player ids are available.
- Browser center preview rows use row-specific hrefs when available.
- TUI MDI code no longer carries stale placeholder-pane scaffolding.
- TUI Tab handling comments and test-only helpers reflect the current MDI behavior.
- TUI event and screen dispatch scaffolding comments reflect current usage.
- The broad `icelines-cli` TUI test slice passes after catalog-order cleanup.
- TUI MDI Tab tests assert focus traversal rather than legacy no-op behavior.
- TUI command persistence tests isolate DB home state without process-wide HOME mutation.
- TUI cleanup compiles in release mode without cleanup-related warnings.
- The full `icelines-cli` binary test target passes after the TUI cleanup wave.
- Browser dashboard uses wide viewports for the workbench shell instead of centering inside the global content cap.
- Browser dashboard center workspace expands inside the workbench grid instead of behaving like a nested capped page.
- Browser dashboard Leaders workspace shows the full leaders table and controls rather than the generic preview table.
- Browser dashboard navigation keeps app-route clicks and GET filters inside the center workspace by default.
- Browser dashboard Player workspaces show the full player card content in the center workspace.
- Browser dashboard link interception only claims routes the workspace renderer can serve, with clean fallback for unsupported routes.
- Browser dashboard Team workspaces show the full roster content in the center workspace.
- Browser dashboard Team Season workspaces show the full season summary and schedule table in the center workspace.
- Browser dashboard Scores and Schedule workspaces show their full picker/table pages in the center workspace.
- Browser dashboard Goalies and Depth workspaces show their full stat table pages in the center workspace.
- Browser dashboard Ctrl-click pins link previews into the left pane, and Ctrl+Shift-click pins them into the right pane.
- Browser dashboard center swaps and pane pins preserve selected room, left pane, right pane, and pinned pane URL state.
- Browser dashboard room and pane-control navigation preserves pinned left/right workspace previews.
- Browser dashboard server-rendered room and pane-control links carry pinned left/right workspace URLs.
- Browser dashboard pinned pane headers expose Open in center, Swap with center, and Clear pin actions.
