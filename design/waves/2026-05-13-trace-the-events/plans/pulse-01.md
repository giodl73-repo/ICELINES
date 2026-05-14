# Pulse 01 - Event Participant Source Inventory

## Goal

Validate whether NHL play-by-play data can support the remaining individual
records metrics without inference:

- player goalies scored against
- team goalies beaten
- player fight opponents
- team fight opponent pairs

## Governing lenses

- **tape**: verify the source row carries the actual participant ids.
- **wire**: record optional and drifting fields explicitly before adding a typed
  parser.
- **forge**: keep this pulse to inventory and pulse map; no renderer-local
  computation.
- **edge**: call out empty-net goals, missing drawn-by penalty ids, and
  reciprocal fight rows before implementation.
- **glass**: preserve the existing `records` family as the user-visible home.

## Work completed

1. Inspected the existing fetch layer and confirmed IceLines currently has a
   boxscore parser but no play-by-play parser/store path.
2. Probed `GET /v1/gamecenter/{game_id}/play-by-play` for a regular-season game
   with goals and penalties.
3. Verified goal rows expose `scoringPlayerId` and `goalieInNetId` when a goalie
   is present.
4. Verified an empty-net/no-goalie goal row can omit `goalieInNetId`.
5. Verified fighting majors appear as explicit penalty participant rows with
   reciprocal `committedByPlayerId` and `drawnByPlayerId` values.
6. Wrote `EVENT-DATA-INVENTORY.md` and opened this wave's remaining pulse map.

## Gates

- `proof check design\waves\2026-05-13-trace-the-events design\waves\PHASES.md --errors-only`

## Result

The source is viable for the remaining records family, but implementation must
start with a cached play-by-play fetch/store path. Goalie-beaten counts should
skip goal rows without `goalieInNetId`, and fight-opponent counts should dedupe
reciprocal fighting-major rows by a normalized player-pair key.
