# Command-bar session findings

## Session type

Dry-run. No live participant was available inside this CLI session, so the
protocol was exercised against the current automated command-bar and dashboard
coverage instead of a moderated human session.

## Dry-run blocker

True user testing still needs at least one external participant running
`icelines --no-live tui` with `USER-TESTING-PROTOCOL.md`. This artifact does
not claim that a first-time user succeeded; it records that the current product
is ready for that session and identifies what the automated harness already
protects.

## Task outcomes

| Task | Dry-run outcome | Classification |
|---|---|---|
| Find skater leaders, then change to goalies. | Covered by command workspace and persona swaps. | No parser/focus gap found. |
| Open fantasy poach board and narrow to RW category helpers. | Parser and exec tests cover poach filters. | No parser gap found; human discoverability still untested. |
| Run an add/drop fantasy simulation. | Parser and exec tests cover simulation scenario application. | No parser gap found; fixture names may need tester-friendly examples. |
| Hide and restore schedule pane. | Persona tests cover slash commands and Ctrl pane toggles. | No focus gap found; discoverability still untested. |
| Open team depth, then team season-performance. | Parser and exec tests cover `team EDM` and `team EDM season`. | No parser gap found. |
| Get OHL career leader target. | Parser and exec tests cover career handoff flash. | Handoff remains the main human-risk class. |
| Compare two players head-to-head. | Parser and exec tests cover `compare A vs B` handoff. | Handoff remains the main human-risk class. |
| Add player to watch/favorites safely. | TUI parser plus web command tests cover safe mutation intent / POST boundary. | No WIRE gap found. |
| Recover from invalid command. | Persona and parser tests cover editable parse errors and recovery. | No feedback-state gap found; wording should be judged by participants. |

## Automated evidence

- `cargo test -p icelines-cli --bin icelines persona_jack_adams` passed with
  100 matching MDI persona/focus scenarios.
- `cargo test -p icelines-cli --bin icelines l0_adams` passed with parser and
  execution coverage for command-bar grammar and outcomes.
- `cargo test -p icelines-web dashboard_command` passed with 8 matching web
  dashboard command tests.

## Findings

| Class | Finding | Disposition |
|---|---|---|
| Parser gap | No protocol command failed in the existing parser/exec harness. | Fixed/covered by existing tests. |
| Discoverability gap | Post-dry-run user feedback called out confusing command-mode focus and weak hints for repeated command use. | Fixed in follow-up: sticky empty prompt now says the next command can be typed and `Tab`/`Esc` leave. |
| Feedback gap | Successful command execution could leave an empty focused prompt, hiding the fact that the app was ready for the next command. | Fixed in follow-up: successful commands now render a visible command-row handoff. |
| Focus gap | Pressing `:` again after a successful command could be interpreted as literal `:` input because sticky focus was still active. | Fixed in follow-up: `:` at an empty focused prompt is a harmless re-entry no-op. |
| Handoff gap | Career and compare intentionally flash CLI/web targets instead of opening richer TUI boards. | Defer judgment to participant comprehension; do not change before observation. |
| Documentation gap | `COMMANDS.md` documents the command-bar vocabulary, web contract, and sticky command-mode exit keys. | Covered for dry-run; validate with participant. |

## Recommendation

Run one live moderated session next with special attention to whether users
notice sticky command mode, understand that `Tab`/`Esc` leaves it, and can chain
workspace commands without maintainer coaching. Keep career/compare handoff
judgment for that session; the follow-up fix only addresses the repeated-command
focus and feedback friction reported after the dry-run.
