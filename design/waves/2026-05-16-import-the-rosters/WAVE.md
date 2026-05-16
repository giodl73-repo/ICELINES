---
wave: import-the-rosters
date_open: 2026-05-16
status: active
source: Tier 3 backlog - Yahoo league CSV roster import
---

# Import the Rosters

## Mission

Add a safe local Yahoo roster CSV import path for fantasy leagues: parse a
user-supplied export, preview what will change, and write FantasyDb league/team
roster rows without treating Yahoo stats as authoritative data.

## Award Fit

This is a Selke / Jim Gregory utility wave: it removes tedious manual fantasy
setup while preserving IceLines' local-first, typed-contract, no-silent-import
discipline.

## Scope

| Track | Target | Non-goal |
|---|---|---|
| Import contract | Define a shared import preview/result ViewModel and row statuses. | Add Yahoo API auth or private remote sync. |
| CSV parsing | Extend the existing Yahoo CSV boundary to roster membership, header aliases, BOM handling, and explicit diagnostics. | Read Yahoo stat columns into rankings or fantasy scores. |
| Persistence | Bulk upsert FantasyDb leagues, teams, user-team marker, and normalized roster memberships. | Replace manual league/team commands or rewrite fantasy schemas broadly. |
| Surfaces | Add a CLI import command plus TUI/web-dashboard handoffs after the shared contract exists. | Upload files through a GET route or mutate web state without POST. |
| Closeout | Document file format, dry-run behavior, warnings, and gates. | Import official Yahoo schedules or keeper/salary settings. |

## Operating Rules

- Yahoo CSV is optional fantasy context. NHL API/bundled snapshots remain the
  player and stat source of truth.
- Import rows must normalize names with `normalize_name()` before persistence.
- Missing/ambiguous player identity, duplicate roster ownership, unknown fantasy
  teams, and header drift must surface as explicit row diagnostics.
- Dry-run must perform the same parse/validation path as apply mode without
  mutating FantasyDb.
- Web/dashboard GET navigation remains read-only; any future browser import must
  be POST-backed.
- Do not add live-network tests or Yahoo API dependencies.

## Pulse Status

| Pulse | Status | Evidence |
|---|---|---|
| 01 - CSV import inventory and pulse map | complete | `FANTASY-CSV-IMPORT-INVENTORY.md`; `plans/pulse-01.md`; `panels/wave-plan-review/` |
| 02 - Shared import contract | complete | `icelines-core/src/view_model/fantasy_import.rs`; `plans/pulse-02.md` |
| 03 - CSV parser and FantasyDb importer | complete | `icelines-fetch/src/fantasy_import.rs`; `icelines-fetch/src/lib.rs`; `plans/pulse-03.md` |
| 04 - CLI, TUI, and dashboard import surfaces | planned | depends on Pulse 03 |
| 05 - Docs, regression gates, and closeout | planned | depends on Pulses 02-04 |

## Role Notes

- **tape**: Yahoo CSV cannot become player/stat ground truth. Use it only for
  fantasy roster membership and eligibility metadata; preserve NHL identity
  checks and normalized-name diagnostics.
- **wire**: validate headers by name/alias, strip BOM, reject unsupported schema
  with actionable errors, and never index silently into assumed columns.
- **forge**: core owns pure import ViewModel/result types; fetch owns CSV and
  SQLite work; CLI/web/TUI are adapters.
- **bench**: fixture tests must cover dry-run no mutation, apply mutation,
  missing columns, duplicate ownership, diacritics, and unresolved rows without
  live Yahoo data.
- **glass**: import output must be readable as a summary first: league, teams,
  rostered players, skipped/error rows, and next recovery command.

## Current Result

Pulse 03 added the fetch-layer Yahoo roster CSV import path. The parser supports
BOM stripping, flexible rows, header aliases, row-level diagnostics, optional
known-player validation, duplicate ownership detection, dry-run no-mutation
preview, and apply-mode FantasyDb league/team/roster convergence through the
shared `FantasyImportView` contract.

## Next

Execute Pulse 04: CLI, TUI, and dashboard import surfaces.
