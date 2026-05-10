# Ted Lindsay Closeout Tasklist

**Date**: 2026-05-10
**Purpose**: turn the remaining web-parity work into ordered implementation slices.

## Current Position

The web surface now has a route inventory gate, handler modules, shared
ViewModel paths for the major read routes, and consistent shared envelopes for
the most important JSON success/error paths. The remaining work is mostly
verification, documented exceptions, and tightening route behavior where the web
surface still differs from CLI/TUI.

## Order Of Work

1. **Keep CI green while slicing** - active
   - Treat `cargo fmt --check`, `cargo clippy -p icelines-web --all-targets -- -D warnings`,
     and `cargo test -p icelines-web` as the local web gate.
   - Keep each web parity slice separately committed so failing CI can identify
     the regression quickly.

2. **Finish JSON contract inventory** - complete for mounted web APIs
   - Confirm every `/api/v1/*` route is one of:
     - shared `{schema_version, route, data, meta}` success envelope;
     - shared `{schema_version, route, data, meta, error}` error envelope;
     - documented raw ViewModel exception.
   - Current documented exceptions: `/api/v1/poach`, `/api/v1/watch-rules`,
     `/api/v1/favorites`, and `/api/v1/watchlist`.
   - Completed slices normalized transactions, scores, schedule, playoffs,
     compare, game, leaders bad-filter errors, and goalies envelope coverage.

3. **Lock leaders web parity** - next implementation slice
   - Keep `/leaders` and `/api/v1/leaders` aligned for repeated filters,
     bad filters, sort, top, position, country, age, and compound filters.
   - Add any missing tests to the existing persona wave files instead of
     creating another broad web test binary.

4. **Verify CLI/TUI adapter alignment for partial rows**
   - Leaders: TUI stats execution/selection remains the named gap; CLI
     JSON/CSV and TUI result rendering now serialize/render from `LeadersView`.
     See `design/notes/2026-05-10-leaders-adapter-alignment.md` for the
     row-contract migration boundary.
   - Goalies: CLI, TUI, web HTML, and web JSON already build `GoaliesView`;
     CLI JSON/CSV row identity is now fenced by
     `l2_cmd_query_goalies_json_csv_row_identity_match`; web JSON now accepts
     the CLI-parity `saves` sort and explicit `gp_min` floor; TUI/web parity
     checks include `l0_goalies_view_from_tui_state_carries_saves_sort_and_metric`;
     CLI/TUI/web row ordering now uses shared `GoalieLeaderboardSort` with
     player-id tie-break; fixture coverage includes
     `goalie_leaderboard_sort_uses_uniform_id_tiebreak` and
     `l2_query_goalies_cli_and_web_row_identity_match`.
   - Team depth: web JSON row identity is now fenced against
     `TeamDepthView` by `l1_team_json_rows_match_team_depth_view`; CLI/TUI
     adapter alignment remains before marking the row `done`.
   - Cross-team depth: web JSON row identity is now fenced against
     `DepthLeagueView` by `l1_depth_json_rows_match_depth_league_view`;
     CLI/TUI adapter alignment remains before marking the row `done`.
   - Player card: web JSON row identity is now fenced against
     `PlayerCardView` by `l1_player_json_rows_match_player_card_view`;
     CLI/TUI adapter alignment remains before marking the row `done`.
   - Compare: web JSON card identity is now fenced against `CompareView` by
     `l1_compare_json_cards_match_compare_view`; CLI/TUI adapter alignment
     remains before marking the row `done`.
   - Career: web JSON row identity is now fenced against `CareerView` by
     `l1_api_career_rows_match_career_view`; CLI rows now project through
     `CareerView` and are fenced by `l0_cli_career_rows_project_from_career_view`;
     richer TUI affordance remains before marking the row `done`.

5. **Resolve live-route envelope policy** - complete for Ted Lindsay
   - Scores, schedule, playoffs, and game carry live-fetch failures in
     `meta.source_error` on successful API responses.
   - Transactions and compare now use strict data/meta success envelopes.

6. **Close high-value route claims** - complete for deferred web routes
   - Confirm `/fantasy` remains intentionally deferred and is not advertised as
     shipped elsewhere.
   - Confirm docs/search/admin snapshot routes are either implemented, planned,
     or absent from user-facing claims.
   - Current explicit deferred/not-mounted claims cover fantasy fold-in,
     scouting web routes, report toggles, data admin, and snapshots.

7. **Hand off visual quality**
   - Ted Lindsay should leave web route truth settled.
   - Prince of Wales owns beauty: layout, responsive density, visual tokens,
     screenshot checks, and DEGAS rubric alignment.

## Exit Criteria

- `design/specs/surface-parity.md` has no vague route claims.
- Every mounted route has a status, owner, and test reference or explicit
  exception.
- API envelopes either conform to the shared helper or are recorded as raw
  contract exceptions.
- `cargo test -p icelines-web` passes locally and CI confirms the pushed head.
