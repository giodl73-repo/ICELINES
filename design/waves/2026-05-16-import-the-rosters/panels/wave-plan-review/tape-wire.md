# TAPE/WIRE Review - Import the Rosters

## Findings

- TAPE: Yahoo CSV is optional fantasy context. It may populate local fantasy
  roster membership, but NHL API/bundled snapshots remain the authoritative
  player, team, photo, and stat sources.
- TAPE: normalized name matching must preserve diacritics for display and use
  team/position hints only as diagnostics or disambiguation aids.
- WIRE: CSV column drift is a first-class risk. The importer needs a documented
  header-alias table and clear missing-column errors instead of positional reads.
- WIRE: BOM/flexible-row behavior in `csv_loader.rs` is good precedent, but
  roster ownership import needs row-level diagnostics and dry-run parity.

## Required Pulse Constraints

- Do not read Yahoo stat columns into rankings, fantasy points, or projections.
- Validate required logical fields by name/alias and report missing headers.
- Surface duplicate ownership, missing fantasy team, unresolved player name, and
  ambiguous match as diagnostics.
- Do not add live Yahoo API access or network-backed tests.
