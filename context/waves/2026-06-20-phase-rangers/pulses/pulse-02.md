# Pulse 02: NYR workflow proof

## Goal

Prove a concrete Rangers workflow across existing offline ICELINES surfaces
without adding new analytics semantics or team-specific hardcoded claims.

## Scope

Added `scripts/rangers-workflow.ps1`, a repeatable PowerShell proof that runs
the CLI offline with `--no-setup --no-live` and validates these surfaces:

- `icelines team NYR --no-color`
- `icelines query leaders --team NYR --top 5`
- `icelines query goalies --team NYR --top 5`
- `icelines signals "Mika Zibanejad"`
- `icelines export md team --team NYR --out -`
- `icelines export md signals --player "Mika Zibanejad" --out -`

The script asserts source/completeness text, goalie workload columns, Signals
non-claim copy, unavailable-state disclosure, and Markdown report disclosures.

## Non-claims

- No MoneyPuck deployment columns were added.
- No GSAx or high-danger save percentage claim was added.
- No team confidence band was synthesized.
- No Signals `StatId`, filter, leaderboard, or analytics-cache promotion was
  added.
- NYR is used as a representative bundled-data workflow, not as a hardcoded
  product path.

## Validation

| Command | Result |
|---|---|
| `powershell -ExecutionPolicy Bypass -File scripts/rangers-workflow.ps1` | passed |
| `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only` | passed |
| `git diff --check` | passed |

## Result

Status: passed.

The Rangers round now has a concrete offline proof tying the post-Hurricane
surfaces together. The next pulse can choose either the Signals discovery design
gate or a focused evidence-envelope bridge, using this workflow as the product
thread.
