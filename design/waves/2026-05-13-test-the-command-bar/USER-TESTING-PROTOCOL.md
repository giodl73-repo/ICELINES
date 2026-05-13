# Command-bar user-testing protocol

## Goal

Determine whether a first-time IceLines user can navigate the MDI dashboard and
discover useful subcommands without maintainer coaching.

## Participant profile

- Knows hockey or fantasy hockey basics.
- Has not worked on the IceLines codebase.
- Is comfortable typing terminal commands but does not know the TUI grammar.

## Setup

1. Build or use the current `icelines` binary.
2. Start deterministic mode with `icelines --no-live tui`.
3. Tell the participant only: "Use the dashboard to answer the tasks. You can
   use keyboard navigation, `?`, `:`, or `/` if you discover them."
4. Record: command typed, screen reached, hesitation points, wrong turns, and
   whether the participant needed a hint.

## Success scale

| Rating | Meaning |
|---|---|
| Pass | User completes task without a hint and can explain what happened. |
| Pass with friction | User completes task but hesitates, tries dead-end commands, or misreads a handoff. |
| Fail | User cannot complete task without a direct command or maintainer explanation. |

## Core tasks

| Task | Expected route |
|---|---|
| Find skater leaders, then change to goalies. | Tab navigation or `:stats` then `:goalies`. |
| Open the fantasy poach board and narrow to RW category helpers. | `:poach rw cats=hits,blocks free top=12`. |
| Run an add/drop fantasy simulation. | `:simulate add=Connor_McDavid drop=Bench_Forward weeks=3`. |
| Hide and restore the schedule pane. | `/hide schedule`, `/show schedule`, or visible pane controls. |
| Open a team depth chart, then the same team's season-performance view. | `:team EDM`, `:team EDM season`. |
| Get the command target for OHL career leaders. | `:career league=OHL season=20142015 top=8`. |
| Compare two players head-to-head. | `:compare McDavid vs Crosby`; user should understand the CLI/web handoff. |
| Add a player to watch/favorites without using a GET mutation. | `/fav add <name>` or `:watch <name>` handoff. |
| Recover from an invalid command. | Error/flash should identify a correction path. |

## Observations to classify

| Class | Definition |
|---|---|
| Parser gap | A reasonable command fails even though it maps to an existing feature. |
| Discoverability gap | Command works but user never finds it or the help hint. |
| Feedback gap | Command runs but result/target is unclear. |
| Focus gap | User cannot predict what `Tab`, `Shift+Tab`, `Esc`, `:`, or `/` will do. |
| Handoff gap | User does not understand why a command flashes a CLI/web target instead of opening a TUI board. |
| Documentation gap | `COMMANDS.md`, help overlay, or examples disagree with behavior. |

## Stop conditions

- Do not add new live-network requirements to usability tests.
- Do not turn mutation commands into GET-backed links.
- Do not invent surface-local scoring or projections while fixing command flow.
- If the user asks for a capability that needs a missing ViewModel, record the
  gap and create a follow-up pulse instead of faking the value.
