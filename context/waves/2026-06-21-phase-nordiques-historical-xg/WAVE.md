# Phase Nordiques Historical xG

## Mission

Clear the MoneyPuck historical xG backlog item by extending the existing
MoneyPuck silo to fetch a bounded regular-season history window and read
MoneyPuck rows by requested season.

## Scope

- Add a public `fetch money-puck --seasons N` option.
- Preserve the default one-season `fetch money-puck` behavior.
- Prefer season-aware MoneyPuck snapshot reads for regular-season loads.
- Keep MoneyPuck optional, schema-checked, and regular-season only.

## Pulses

| Pulse | Status | Evidence |
|---|---|---|
| 01 | passed | Plan and inventory created. |
| 02 | passed | CLI fetch window implemented and tested. |
| 03 | passed | Historical snapshot read path implemented and tested. |
| 04 | passed | Docs, backlog, and closeout updated. |

## Closeout

Phase Nordiques is complete. `fetch money-puck --seasons N` now fetches the
selected season plus prior regular seasons, and historical repo loads prefer the
sealed MoneyPuck snapshot matching the requested season.
