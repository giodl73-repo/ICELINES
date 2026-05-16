# FORGE/WIRE Review - Shape the Rosters

## Findings

- Roster shape belongs in pure core types; FantasyDb should persist selected
  rules, not compute slot legality itself.
- Existing leagues need a migration/default that is visible and reversible.
- Yahoo CSV position hints are external data. They may enrich diagnostics, but
  canonical eligibility should come from the loaded player pool when available.

- forge + wire
