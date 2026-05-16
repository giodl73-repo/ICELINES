# FORGE Review - Import the Rosters

## Findings

- The shared import summary belongs in `icelines-core` as a pure ViewModel/result
  contract.
- CSV parsing, filesystem reads, and FantasyDb writes belong in `icelines-fetch`
  or CLI-adjacent adapters; core must remain I/O-free.
- The existing FantasyDb APIs are single-row operations. A bulk import should
  converge state idempotently and avoid broad schema churn unless a typed need is
  proven.

## Required Pulse Constraints

- Keep CSV parsing and SQLite errors explicit; no broad catch/success fallbacks.
- Use `normalize_name()` at the mutation/import boundary.
- Keep CLI/web/TUI as thin adapters over the shared import contract.
- Preserve existing manual fantasy commands and table semantics.
