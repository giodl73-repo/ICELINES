# Pulse 04 - Player/team records screens

## Goal

Make records visible where users naturally look for them: player cards and team
screens.

## Deliverables

- Player Records screen or player-card section.
- Team Records screen or team-season subsection.
- TUI command-bar handoffs for records commands.
- Web route/API parity if the ViewModels are stable.

## Inputs

- `PlayerRecordsView` / `TeamRecordsView` from Pulse 03.
- `icelines records player/team` CLI behavior and output columns.

## Gates

- TUI render tests for records entry points.
- Web route/API tests if web parity is included.

## Result

Done. Added player/team records visibility in three places:

- Web HTML/API routes: `/records/player/:id`, `/records/team/:abbrev`,
  `/api/v1/records/player/:id`, and `/api/v1/records/team/:abbrev`.
- Player/team web pages link directly to records pages.
- TUI player card and team-season screens show records entry points, and the
  command bar parses `records player <name>` / `records team <ABBR>` handoffs.

All records rows still come from `PlayerRecordsView` / `TeamRecordsView` over
persisted boxscore goal inputs. Goalie/fight records remain deferred until
play-by-play participants are validated.
