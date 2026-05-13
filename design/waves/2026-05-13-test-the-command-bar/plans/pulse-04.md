---
wave: test-the-command-bar
pulse: 04
date: 2026-05-13
status: done
depends_on: [pulse-02, pulse-03]
governing_roles:
  - glass
  - crest
  - bench
  - scout
---

# Pulse 04 - Moderated Session Findings and Fixes

## Mission

Run or dry-run the protocol, classify the observed failures, and apply only the
smallest fixes needed to make IceLines usable without maintainer coaching.

## Deliverables

- Session findings artifact under this wave directory.
- Fixes or explicit deferrals for parser, feedback, focus, handoff, and docs
  gaps.
- Updated tests for every behavior change.

## Gates

- [x] Session findings artifact names participant/task outcomes or a dry-run
      blocker.
- [x] `cargo test -p icelines-cli --bin icelines persona_jack_adams`
- [x] `cargo test -p icelines-cli --bin icelines l0_adams`
- [x] `cargo test -p icelines-web dashboard_command`
- [x] `cargo fmt --check`

## Stop Conditions

- Stop if tester feedback asks for a product capability outside command-bar,
  tabbing, or subcommand usability; create a separate wave instead.
- Stop if a fix would hide source truth or make a mutation look like navigation.
