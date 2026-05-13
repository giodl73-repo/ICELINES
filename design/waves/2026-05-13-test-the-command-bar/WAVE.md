---
wave: test-the-command-bar
date_open: 2026-05-13
status: closed
source: user-testing request for command bar, tabbing, and subcommand usability
---

# Test the Command Bar

## Mission

Find out whether a user who did not build IceLines can operate the default MDI
dashboard through the command bar, tabbing, and documented subcommands. The
wave should turn observed confusion into small, test-backed improvements rather
than broad redesign guesses.

## Inputs

| Input | Source |
|---|---|
| Command-bar reference | `COMMANDS.md` |
| TUI command parser | `icelines-cli/src/tui/command.rs` |
| MDI dashboard persona harness | `icelines-cli/src/tui/persona_jack_adams.rs` |
| TUI screen/chrome renderers | `icelines-cli/src/tui/screens/` |
| Web dashboard command contract | `icelines-web/src/handlers/dashboard.rs`; `icelines-web/tests/l1_router.rs` |
| Visual system | `design/specs/visual-system.md` |

## Scope

| Track | Target | Non-goal |
|---|---|---|
| User testing | Create a reproducible protocol for command-bar and tabbing sessions. | Replace user sessions with only automated tests. |
| Discoverability | Identify commands that are documented but not obvious in the UI. | Change command grammar before observing failures. |
| Keyboard flow | Validate `Tab`, `Shift+Tab`, `:`, `/`, `Esc`, and command submission behavior. | Redesign the whole MDI shell. |
| Subcommand handoffs | Check whether users understand when a command opens a TUI workspace vs flashes a CLI/web target. | Add unsafe or GET-backed mutations. |

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - Usability protocol and command inventory | done | `plans/pulse-01.md`; `USER-TESTING-PROTOCOL.md` |
| 02 - Tabbing and focus regression harness | done | `plans/pulse-02.md`; `FOCUS-HARNESS-INVENTORY.md` |
| 03 - Command vocabulary and subcommand discoverability | done | `plans/pulse-03.md`; `COMMAND-VOCAB-INVENTORY.md` |
| 04 - Moderated session findings and fixes | done | `plans/pulse-04.md`; `SESSION-FINDINGS.md` |

## Closeout Target

This wave closes when:

- the protocol has been run or dry-run against the current dashboard;
- command-bar/tabbing issues are classified as fixed, documented, or deferred;
- any behavior changes have focused parser/render/persona tests;
- `COMMANDS.md` and help affordances match the commands users are expected to try.

## Closeout Result

Closed with a protocol dry-run because no live participant was available inside
the CLI session. The current command bar has strong automated coverage:

- 100 MDI persona/focus scenarios.
- 41 parser tests and 25 execution tests under `l0_adams`.
- 8 web dashboard command tests.

The next true usability step is an external moderated session using
`USER-TESTING-PROTOCOL.md`; any observed failures should open a new follow-up
wave or reopen this one with concrete findings.
