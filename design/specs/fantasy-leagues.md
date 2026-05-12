# Fantasy Leagues — Specification

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Implemented

---

## Purpose

Manage one or more **fantasy hockey leagues** locally: create leagues,
add teams, set rosters, evaluate trades, and serve a web dashboard. The
fantasy engine combines canonical NHL stats loaded into
`icelines_core::stats_repository::StatsRepository` with a user-selected scoring scheme
(Yahoo standard, ESPN, or custom; see `fantasy-scheme.md`) to produce
team standings.

This spec covers data model, CLI commands, scoring algorithm, trade
evaluation, and the HTTP server. It does **not** cover scheme TOML
authoring (see `fantasy-scheme.md`) or scheme CLI ops (see
`scheme-customization.md`).

---

## Data model

Three tables are added by migrations 003–005 to the shared SQLite
database at `~/.icelines/icelines.db`. They sit alongside the
`groups`, `group_members`, and `saved_queries` tables from `group-management.md`.

### `fl_leagues`

```sql
CREATE TABLE fl_leagues (
    id         TEXT PRIMARY KEY,         -- UUID v4
    name       TEXT UNIQUE NOT NULL,     -- user-chosen, case-sensitive
    scheme     TEXT NOT NULL DEFAULT 'yahoo-standard',
    is_active  INTEGER NOT NULL DEFAULT 0, -- 0 or 1; exactly one league = 1 at any time
    created_at TEXT NOT NULL
);
```

The `is_active` flag is the **active league** — used by every CLI
command that doesn't pass `--league` and by the HTTP server's default
view. `league-use` flips active to a new league atomically (set old
league's `is_active=0` then new league's `is_active=1` in one tx).

### `fl_teams`

```sql
CREATE TABLE fl_teams (
    id         TEXT PRIMARY KEY,
    league_id  TEXT NOT NULL,
    name       TEXT NOT NULL,
    owner      TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    FOREIGN KEY(league_id) REFERENCES fl_leagues(id) ON DELETE CASCADE,
    UNIQUE(league_id, name)
);
```

Team name is unique within a league but the same name can recur across
leagues. Owner is free-form text (display only).

### `fl_roster`

```sql
CREATE TABLE fl_roster (
    team_id           TEXT NOT NULL,
    player_normalized TEXT NOT NULL,    -- normalize_name(full_name)
    added_at          TEXT NOT NULL,
    PRIMARY KEY(team_id, player_normalized),
    FOREIGN KEY(team_id) REFERENCES fl_teams(id) ON DELETE CASCADE
);
```

Same normalization rule as `group_members` so a player on multiple
fantasy teams keys consistently.

`PRAGMA foreign_keys = ON` enables cascade deletes; `PRAGMA journal_mode
= WAL` enables concurrent reads while the HTTP server runs.

---

## CLI commands

### League management

```
icelines fantasy league-create <NAME> [--scheme yahoo-standard]
icelines fantasy league-list
icelines fantasy league-use    <NAME>      # flip active flag
icelines fantasy league-switch <NAME>      # alias for league-use
icelines fantasy league-delete <NAME>      # cascade-deletes teams + rosters
```

Newly created league becomes active automatically if no active league
exists. Otherwise active stays where it was; user runs `league-use`
to switch.

### Team management

```
icelines fantasy team-create <NAME> [--owner OWNER] [--league LEAGUE]
icelines fantasy team-list   [--league LEAGUE]
icelines fantasy team-show   <NAME> [--league LEAGUE]   # roster + total fantasy pts
icelines fantasy team-add    <TEAM> <PLAYER> [--league LEAGUE]
icelines fantasy team-drop   <TEAM> <PLAYER> [--league LEAGUE]
```

`--league` overrides the active league for one command. Player
references use partial-name matching against the loaded player set.

### Standings + trades + server

```
icelines fantasy standings [--league LEAGUE] [--scheme SCHEME]
icelines fantasy trade <P1> --to-team <T> --for-player <P2> [--execute] [--league LEAGUE]
icelines fantasy serve [--port 8080] [--league LEAGUE]
```

`standings` recomputes from current snapshot data; `--scheme` overrides
the league's scheme for what-if analysis.
`trade` defaults to **dry-run** (shows projected pts before/after); pass
`--execute` to actually mutate rosters in a single transaction.

---

## Scoring algorithm

Fantasy points = `Σ stat_value * scheme_weight` over a team's roster,
applied per-player then summed. The per-player computation lives in
`icelines-core::scheme`:

```rust
pub fn compute_fantasy_score(
    stats:   &SkaterStats,
    weights: &SkaterWeights,
    gp:      u32,
) -> Option<FantasyScore>
```

Returns `None` when `gp < MIN_GP_SCHEME` (insufficient sample). The
returned `FantasyScore` carries both the raw total and a per-category
breakdown so the dashboard can show the contribution of each weight.
The team-total walker iterates the roster and sums each `Some(score)`,
treating `None` as zero.

Standings sort descending by total; ties broken alphabetically by team
name. See `fantasy-scheme.md` for the full weight types and `SkaterWeights`
shape.

**Pace adjustment** (default in standings): per-player score is projected through
the shared fantasy scoring/ViewModel helpers from the loaded stats window, not
from a renderer-local `Player::pace_score` field. This avoids penalizing
late-acquired players or rewarding players with extra games. Pass `--scheme raw`
(planned) for un-normalized totals.

**Players not in the loaded snapshot** contribute 0 to the team total
and render as `(not in current data)`. They are kept on the roster
record (no auto-drop) so the user can decide.

---

## Trade evaluation

`fantasy trade <P1> --to-team <T> --for-player <P2>`:

1. Resolve P1 and P2 to canonical players.
2. Find each player's current team in the active league.
3. If `--execute` is **not** set: compute and print before/after totals
   for both teams; show the points delta and a verdict
   (`UPGRADE`/`DOWNGRADE`/`even` — same threshold as `icelines trade`).
4. If `--execute` is set: in a single SQLite transaction, swap the two
   roster rows. Print confirmation.

Edge cases:
- P1 and P2 on the same team → error: trade requires two teams.
- Either player not on any team in this league → error.
- `--to-team` doesn't match P1's destination logic (P2 must be on
  `--to-team`) → error with clarifying message.

---

## Local fantasy HTTP server (legacy axum)

```
icelines fantasy serve [--port 8080] [--league NAME]
```

Spawns the local fantasy-only axum server bound to `127.0.0.1:<port>`. These
routes are the legacy standalone fantasy-server mutation/read contract. The main
dashboard parity surface now lives under `/fantasy`, `/api/v1/fantasy/gaps`, and
`/api/v1/fantasy/simulate`; see `surface-parity.md`.

Routes:

| Method + Path | Returns | Notes |
|---|---|---|
| `GET  /` | HTML dashboard | Server-rendered standings, click-through to teams |
| `GET  /api/standings` | JSON `[{ team, owner, total_pts }]` | Sorted desc |
| `GET  /api/teams` | JSON `[{ id, name, owner }]` | All teams in active league |
| `GET  /api/team/:name/roster` | JSON `[{ player, position, team_nhl, fpts }]` | Per-player breakdown |
| `POST /api/team/:name/add` body `{player}` | `{ added: true|false, error? }` | Server-side player resolution |
| `POST /api/team/:name/drop` body `{player}` | `{ dropped: true|false }` | |
| `POST /api/trade` body `{p1, p2, to_team}` | `{ delta, verdict, executed: false }` | Dry-run only via API in v1 |

The server reads from the same `~/.icelines/icelines.db` as the CLI.
WAL mode lets the CLI read/write while the server is running.
The server is **not** a long-running daemon — it stops when the
spawning process exits.

Main-dashboard write policy: `/fantasy` and `/api/v1/fantasy/*` are
ViewModel-backed read/product surfaces for roster gaps, standings simulation,
and add/drop/drop-only scenario projection. League/team roster mutation remains
in the CLI and this legacy local fantasy server until a dedicated web write-flow
phase defines CSRF, confirmation, validation, and `MutationResultView` contracts
for those routes.

**Security**: localhost-only, no auth, no TLS. Designed for single-user
dashboards. Do not expose the port to the network without a reverse proxy.

---

## Active-league semantics

`is_active=1` is enforced by application code (a UNIQUE-style trigger
is not used, to keep the schema portable). Every `league-use` flips
the flag in a transaction. Concurrent processes (e.g. CLI + server)
that both try to switch will see one update; the loser sees the new
state on next read (WAL mode).

If somehow zero rows have `is_active=1` (corruption / aborted txn),
commands that depend on the active league error with:
> `No active league. Run \`icelines fantasy league-use <NAME>\`.`

If two rows have `is_active=1`, the database file is considered corrupt
and the user is told to run `--repair-active` (planned; not in v1).

---

## Decisions (Open Questions resolved)

1. **Storage location**: Same SQLite as groups/queries
   (`~/.icelines/icelines.db`). One DB file per user; no per-project
   isolation in v1.

2. **Roster size**: No enforcement in v1. Yahoo standard is 14F + 4D
   + 2G + 2 IR + 1 BN; the spec leaves this to the user. A v2
   `--roster-shape` per scheme would enforce.

3. **Schedule integration**: Standings use season-to-date pace, not
   weekly matchup scoring. Head-to-head week-by-week is deferred to
   v2 (requires a `fl_matchups` table and a schedule walker).

4. **Goalies**: Goalie scoring is **not in v1**. Leagues with goalies
   must use a custom scheme that assigns 0 to all skater categories
   for goalie players (or accept zeros). Full goalie support requires
   `data-sources.md` tier-2 fetches we don't bundle yet.

5. **Multi-user / cloud sync**: Out of scope. The HTTP server is a
   single-user local dashboard, not a multi-tenant web app.

6. **Draft tools**: Not in v1. `icelines query leaders --sort
   pts-pace` covers draft prep; a dedicated `fantasy draft` mode is
   backlog.

7. **Yahoo / ESPN import**: CSV import via `icelines scheme fromcsv`
   handles scheme detection (see `scheme-customization.md`); roster
   import from a Yahoo league export is **not** implemented in v1.

---

## Test coverage

L1 (in-memory SQLite) in `icelines-cli/src/fantasy_db.rs::tests`:
- `l1_fantasy_create_league`
- `l1_fantasy_create_and_list_teams`
- `l1_fantasy_add_drop_player`
- `l1_fantasy_active_league` — flag invariants on switch
- `l1_fantasy_player_already_taken`
- `l1_fantasy_duplicate_league_name_errors`
- `l1_fantasy_delete_league_cascades_to_teams_and_rosters`
- `l1_fantasy_delete_team_cascades_roster`
- `l1_fantasy_duplicate_player_on_same_team_is_noop`

L2 (subprocess) in `tests/system_tests.rs`:
- `l2_cmd_fantasy_help_exits_zero`
- `l2_cmd_fantasy_league_create_exits_zero`
- `l2_cmd_fantasy_league_list_exits_zero`
- `l2_cmd_fantasy_league_create_then_list_shows_league`
- `l2_cmd_fantasy_standings_exits_zero`
- `l2_cmd_fantasy_serve_help_exits_zero`

HTTP-route tests via in-process `axum::Router` (TODO — see Future work).

---

## Future work (v2+)

| Item | Priority | Why deferred |
|------|----------|--------------|
| Goalie scoring | HIGH | Needs data-sources tier 2 wins/SV%/SO; not bundled |
| Head-to-head matchups (`fl_matchups`) | MED | Common but requires schedule walker; standings = pace works for now |
| Roster shape enforcement (per scheme) | MED | Yahoo / ESPN have different rules; punt to scheme TOML |
| Yahoo league CSV import (rosters, not just scheme) | MED | Useful but not blocking |
| Auth for HTTP server | LOW | Single-user assumption; reverse-proxy if needed |
| Daily delta scoring (`yesterday's stats`) | LOW | Backlog; see plans/INDEX.md |
| Draft mode (`fantasy draft`) | LOW | `query leaders` covers most prep |
