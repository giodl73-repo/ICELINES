---
wave: call-the-changes
date_open: 2026-05-15
status: active
source: user request for final MDI stage after Sim the Spark closeout
---

# Call the Changes

## Mission

Finish IceLines MDI by making screen selection explicit and discoverable. The
TUI and web dashboard should let users choose the active screen from a catalog,
picker, or persistent navigation pane instead of relying on tab cycling or
command recall.

## Award Fit

This continues the Jack Adams / Masterton product arc: a coach's bench does not
cycle blindly through lines; it calls the right change at the right time. IceLines
already has MDI shells, command bars, standalone launchers, and panel-ready web
routes. This wave turns that foundation into a full MDI experience where every
main screen is visible, searchable, and directly selectable.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Screen catalog contract | Shared list of main screen entries, labels, aliases, help text, and surface route targets. | New analytics or data loading behavior. |
| TUI navigation | Replace default MDI tab strip/cycling with a screen picker/catalog and direct selection affordance. | Delete `--classic` before compatibility is reviewed. |
| Web navigation | Add a dashboard screen catalog/picker that opens workspace panels without requiring command syntax. | Convert the server-rendered dashboard into a SPA. |
| Docs and accessibility | Update keybinds, dashboard docs, and keyboard/ARIA expectations for direct screen selection. | Hide command bars; commands remain power-user shortcuts. |

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - MDI navigation inventory and contract | planned | `plans/pulse-01.md` |
| 02 - Shared screen catalog | planned | depends on Pulse 01 |
| 03 - TUI full-MDI picker | planned | depends on Pulse 02 |
| 04 - Web dashboard screen catalog | planned | depends on Pulse 02 |
| 05 - Docs, regression gates, and closeout | planned | depends on Pulses 03-04 |

## Role Notes

- **keel**: TUI and web must converge on the same screen catalog identity. The
  command grammar can remain a shortcut, but it must not be the only source of
  workspace truth.
- **glass**: default MDI should be glanceable. A user should see how to open
  Stats, Goalies, Scores, Schedule, Transactions, Playoffs, Favorites, Fantasy,
  and team/player drilldowns without memorizing verbs.
- **forge**: keep catalog types small and shared through existing crate
  boundaries. Do not introduce route-local clones or long-lived web state.
- **wire**: screen selection is navigation only. GET routes may load panels from
  existing cache/read paths, never trigger mutations or live fetch side effects.
- **bench**: add tests that prove tab-era regressions are intentional, catalog
  entries are complete, picker selection changes active workspace, and no command
  examples drift from catalog targets.

## Current Result

Wave opened after Sim the Spark closed and the release binary built. No code has
started. Pulse 01 should inventory the current TUI and web MDI shells, decide the
screen catalog shape, and split implementation into safe slices.

## Next

Execute Pulse 01: MDI navigation inventory and contract.
