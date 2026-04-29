# Group Management — Specification

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Implemented

---

## Purpose

Persistent, named **player watchlists** scoped to one user. Groups let
fantasy managers, scouts, and analysts curate a list of players to track
across queries, projections, schedule lookups, and the TUI.

A group is a name + a description + a set of players (referenced by
canonical normalized name). Membership is stored in SQLite at
`~/.icelines/icelines.db`, shared with the saved-queries store and the
fantasy database.

---

## CLI commands

```
icelines group create   <NAME>   [--desc TEXT]    create an empty group
icelines group add      <GROUP>  <PLAYER>         add a player by name fragment
icelines group remove   <GROUP>  <PLAYER>         remove a player
icelines group list                                show all groups + member counts
icelines group show     <NAME>                     list members with current pace stats
icelines group delete   <NAME>                     delete the group + cascade members
```

Player references use **partial-name matching**: `add Watchlist McDavid`
resolves to the only `mcdavid` substring match in the loaded player set.
Ambiguous matches (multiple `*smith*`) error with the candidates listed.

The default `Favorites` group is auto-seeded on first DB open so the
TUI's `f` (instant-add) key has somewhere to put players from session
zero.

---

## TUI integration

| Key | Where | Action |
|-----|-------|--------|
| `g` | Any player-list screen | Open the group picker overlay; select a group with `↑↓` + Enter; `Esc` cancels |
| `f` | Any player-list screen | Add the highlighted player to `Favorites` instantly (no picker) |
| `Enter` | Groups tab list | Open the group's member detail screen |
| `Enter` | Group detail | Open the highlighted member's player profile |
| `Esc` | Group picker / detail | Back / cancel |

The group picker appears on: player profile, team roster, search results,
projections, queries, group detail, and comps. It does **not** appear on
home, depth, scores, schedule, or playoffs (those screens don't have a
single-player cursor).

---

## Database schema

### `groups`

```sql
CREATE TABLE groups (
    name        TEXT PRIMARY KEY,        -- case-sensitive, user-chosen
    description TEXT NOT NULL DEFAULT '', -- shown in `group list` and TUI
    created_at  TEXT NOT NULL              -- ISO-8601 UTC
);
```

### `group_members`

```sql
CREATE TABLE group_members (
    group_name        TEXT NOT NULL,
    player_normalized TEXT NOT NULL,    -- normalize_name(full_name)
    added_at          TEXT NOT NULL,
    PRIMARY KEY (group_name, player_normalized),
    FOREIGN KEY (group_name) REFERENCES groups(name) ON DELETE CASCADE
);
```

`PRAGMA foreign_keys = ON` is set on every connection so cascade delete
works. `PRAGMA journal_mode = WAL` is set for concurrent-write safety
(the TUI may write while a CLI process reads).

---

## Player reference: normalized names

Members are keyed by `normalize_name(full_name)` from `icelines-core::name`:
- Lowercase, NFKD-decomposed, diacritics stripped, whitespace collapsed
- Example: `"Connor McDavid"` → `"connor_mcdavid"`

This is **stable across seasons** so a group survives a player retiring,
changing teams, or dropping out of the bundled dataset. Members that
don't match any player in the currently loaded season render with
`(not in current data)` in the TUI; CLI `group show` skips them with a
warning.

---

## Operations contract

Each public method on `GroupDb` (`icelines-cli/src/db.rs`) has these
guarantees:

| Method | Returns | Errors when |
|--------|---------|-------------|
| `create_group(name, desc)` | `Ok(())` | name already exists |
| `delete_group(name)` | `Ok(true)` if existed, `Ok(false)` otherwise | I/O / corrupt DB only |
| `add_member(group, norm)` | `Ok(true)` if added, `Ok(false)` on duplicate | group does not exist |
| `remove_member(group, norm)` | `Ok(())` (no-op if not a member) | group does not exist |
| `list_groups()` | `Vec<GroupRow>` (name, description, member_count) | I/O only |
| `list_members(group)` | `Vec<String>` of normalized names, sorted by `added_at` | group does not exist |

The "duplicate add returns false" pattern lets the TUI distinguish "added"
from "was already in this group" without failing the operation.

---

## Auto-seeded `Favorites` group

On every `GroupDb::open()`:

```sql
INSERT OR IGNORE INTO groups (name, description, created_at)
VALUES ('Favorites', 'My favorite players', datetime('now'));
```

This is **idempotent**: it never overwrites an existing description, never
clears members, and runs after migrations. Users can `delete Favorites`
explicitly; it will be re-seeded the next time the DB is opened. This is
a deliberate trade-off so the `f` instant-add key always has a target.

---

## Decisions (Open Questions resolved)

1. **Storage location**: Single shared SQLite at `~/.icelines/icelines.db`
   alongside saved queries and fantasy data. Per-user, not per-project.
   No XDG path support in v1; respects `$HOME` (Unix) or `$USERPROFILE`
   (Windows).

2. **Player keying**: Normalized name, not `nhl_id`. Reason: groups
   often span seasons where IDs may not be in the bundled dataset; name
   normalization is stable. Trade-off: a player who legally changes
   their name will appear as two separate members until manually fixed.

3. **Description field**: Optional, plain text, no length limit enforced
   in v1. Markdown is **not** rendered — the TUI shows the first 28
   chars truncated.

4. **Sharing / export**: Not in v1. Future: `icelines group export
   <name>` → JSON, `import` → load JSON. Tracked in backlog.

5. **Multi-user / cloud sync**: Out of scope. Groups are local-only.

---

## Test coverage

L1 tests live in `icelines-cli/src/db.rs` `#[cfg(test)] mod tests`:

- `l1_db_create_and_list_group` — round-trip
- `l1_db_add_remove_member` — membership ops + ordering
- `l1_db_delete_group_cascades_members` — FK cascade verified
- `l1_db_duplicate_member_is_noop` — duplicate add returns false, no error

L2 (subprocess) coverage in `system_tests.rs`:
- `l2_cmd_group_list_exits_zero` — basic exit-code assertion

The DB is opened via `GroupDb::open_in_memory()` in tests so they don't
touch the user's real database.

---

## Future work (v2+)

| Item | Why deferred |
|------|--------------|
| `group export <name>` → JSON, `group import` ← JSON | Not blocking; users currently rebuild groups from CSV |
| Multi-user / sync | No backend; out of scope |
| Tag-based grouping (one player in many tags) | Use cases unclear; current name-keyed model covers the watchlist case |
| Color labels on groups | Cosmetic; punted until requested |
| `group rename <old> <new>` | Implementable as DELETE + re-INSERT today |
