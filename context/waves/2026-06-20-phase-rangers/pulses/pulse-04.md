# Pulse 04: Signals roster matrix

## Goal

Implement the Phase Rangers Signals discovery lane accepted by pulse 03: a
team-scoped roster matrix that helps users inspect player Signals evidence
without promoting Signals into leaderboards, `StatId`, filters, or cache.

## Scope

Added the additive CLI command:

```powershell
icelines signals-roster --team NYR
icelines signals-roster --team NYR --json
```

The command:

- reuses `PlayerSignalsView` for every skater on the requested team;
- sorts rows alphabetically by player name, not by a Signal value;
- renders one matrix cell per current Signal;
- renders missing evidence as `unavailable`, never `0.00`;
- includes disclosure and non-promotion copy in both text and JSON;
- emits a `signals-roster.v1` JSON envelope for scripting.

## Non-claims

- No stable `StatId` rows were added.
- No `query leaders --sort` or `--filter` integration was added.
- No public cross-team Signal leaderboard was added.
- No analytics-cache publication was added.
- No Web/TUI parity claim was added.
- No prediction, betting, injury, deployment, player-quality grade, or coaching
  recommendation copy was added.

## Validation

| Command | Result |
|---|---|
| `cargo fmt --check` | passed |
| `cargo test -p icelines-cli --bin icelines signals` | passed |
| `cargo test -p icelines-cli --test signals_system` | passed |
| `powershell -ExecutionPolicy Bypass -File scripts\rangers-workflow.ps1` | passed |
| `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only` | passed |
| `git diff --check` | passed |

## Result

Status: passed.

Rangers now has a controlled Signals discovery lane for team workflows. Future
work can decide whether to bridge this into an existing evidence-card/cache
consumer, but that requires a separate gate.
