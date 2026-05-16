# Fantasy Roster Shape Inventory

## Backlog source

`design/plans/INDEX.md` listed `Fantasy roster shape enforcement` as a Tier 3
future feature blocked only on per-scheme rules in TOML.

## Current state

| Area | Current behavior | Gap |
|---|---|---|
| FantasyDb leagues | `fl_leagues` stores `name`, `scheme`, `is_active`. | No roster-shape preset/config is persisted. |
| FantasyDb teams/rosters | `fl_teams` tracks teams/user team; `fl_roster` stores `(team_id, player_normalized)`. | No slot, position, active/bench/IR, max size, or goalie/skater constraint. |
| Manual add/drop | `fantasy team-add` resolves skater/goalie and blocks duplicate ownership across a league. | It can add a player that makes the roster illegal by size or position mix. |
| Yahoo import | Parser reads `Eligible Positions` as `position_hint`; importer dry-runs diagnostics and applies rows. | Position hints are not persisted or validated against a shape. |
| Scoring scheme | `icelines-core::scheme::Scheme` defines point weights and builtin names. | Scoring scheme is not a roster-shape rule set. |
| League ViewModel | `FantasyLeagueView` exposes team `player_count`. | No compliance rows, missing slots, or recovery commands. |
| Surfaces | CLI shows team count/roster score; dashboard/TUI hand off fantasy commands. | No `validate roster`/`shape set/show` surface yet. |

## Proposed contract

Add a pure core contract that can validate a team roster against a named shape:

- `RosterShape` / `RosterSlotRule` for counts by position group.
- Position groups should support skater positions (`C`, `LW`, `RW`, `D`), broad
  skater (`F`, `UTIL`), goalie (`G`), and bench/total caps.
- `RosterShapeValidationView` should summarize status per team with missing,
  overflow, unknown-player, and duplicate/ownership findings.
- Validation input should use canonical active-player positions when resolved;
  CSV/import position hints are fallback diagnostics only.

## Pulse map

1. **Pulse 01 - Roster shape inventory and pulse map**: open this wave, record
   current gaps, and split implementation pulses.
2. **Pulse 02 - Core roster shape contract**: add pure core types, validation
   logic, and L0 tests.
3. **Pulse 03 - FantasyDb shape persistence and import validation**: add safe
   migration/defaults and wire dry-run/apply import validation warnings.
4. **Pulse 04 - CLI, TUI, and dashboard validation surfaces**: expose
   command-copyable validate/show/set surfaces; keep browser mutation deferred.
5. **Pulse 05 - Docs, regression gates, and closeout**: update user docs,
   surface parity, backlog truth, and close wave after focused gates.

## Risks

- Position eligibility can be ambiguous. Do not make Yahoo hints authoritative
  over bundled/NHL player positions.
- Existing leagues should not become unusable after migration. Defaults should
  be explicit and command-visible.
- Shape validation must not silently change scoring or matchup results.

## Non-goals

- No remote Yahoo API sync.
- No automatic lineup optimizer.
- No browser GET mutation.
- No live network tests.
