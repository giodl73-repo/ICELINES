---
wave: backcheck-the-phases
pulse: 08
date: 2026-05-13
status: planned
depends_on: [pulse-01]
governing_roles:
  - scout
  - keel
  - bench
  - tape
  - wire
---

# Pulse 08 - Career and Docs Parity Backfill

## Mission

Close the remaining Calder/Ted Lindsay partials around cross-league career
cohorts and generated docs/spec-site verification. The career data path exists;
this pulse makes the remaining surface claims honest and useful without bundling
new career-history data.

## Deliverables

- Inventory current `query career`, `/career`, `/api/v1/career`, dashboard
  career workspace, and TUI career affordances.
- Add a dedicated TUI career/cohort board or explicitly document the canonical
  CLI/web handoff if a board is not justified.
- Verify generated docs/spec-site claims against `DocsView`, `COMMANDS.md`, and
  `surface-parity.md`.
- Update `design/specs/surface-parity.md`, `COMMANDS.md`, README snippets, and
  docs/spec references.

## Likely Files

- `icelines-core/src/view_model/career.rs`
- `icelines-cli/src/commands/query.rs`
- `icelines-cli/src/tui/command.rs`
- `icelines-cli/src/tui/screens/queries.rs`
- `icelines-web/src/handlers/career.rs`
- `icelines-web/src/handlers/docs.rs`
- `design/specs/surface-parity.md`
- `README.md`
- `COMMANDS.md`

## Gates

- [ ] `cargo test -p icelines-core career`
- [ ] `cargo test -p icelines-cli career`
- [ ] `cargo test -p icelines-web career`
- [ ] `cargo test -p icelines-web docs`
- [ ] `cargo fmt --check`
- [ ] If no TUI board is added, the parity matrix names the canonical handoff and the reason.

## Stop Conditions

- Stop if a TUI board would need career-history fields not exposed by
  `CareerView`.
- Stop if tests would require live NHL landing-endpoint data without fixtures.
- Stop if generated docs are treated as route truth instead of
  `surface-parity.md`.
