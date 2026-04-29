# Scheme Customization — Specification

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Implemented
**Extends**: `fantasy-scheme.md` (TOML format, `compute_fantasy_score`)

---

## Purpose

Manage **fantasy scoring schemes** from the CLI: list installed
schemes, inspect their weights, and bootstrap a new scheme from a
Yahoo Fantasy Hockey CSV export. The scheme TOML format itself is
specified in `fantasy-scheme.md`; this spec covers the CLI surface
and the scheme registry.

```
icelines scheme list                              # show all schemes
icelines scheme show yahoo-standard               # weights table
icelines scheme fromcsv ./yahoo-export.csv        # detect + write scheme TOML
```

---

## Scheme registry

A "scheme" is a TOML file mapping stat names to weights. Two
sources:

| Source | Path | Notes |
|--------|------|-------|
| **Built-in** | `icelines-core/data/schemes/*.toml` (compiled in via `include_str!`) | `yahoo-standard`, `yahoo-pp`, `espn-standard` |
| **User** | `~/.icelines/schemes/*.toml` | One file per scheme; filename = scheme name |

Resolution order: **user schemes shadow built-ins** with the same
name. A user file at `~/.icelines/schemes/yahoo-standard.toml`
overrides the built-in. Use `scheme show yahoo-standard --source` to
disambiguate (planned, not in v1).

`Scheme::load(name)` (in `icelines-core::scheme`):
1. Check `~/.icelines/schemes/{name}.toml`. If present, parse and return.
2. Check the built-in registry. If present, parse and return.
3. Error: `unknown scheme '{name}'. Try \`icelines scheme list\`.`

---

## CLI commands

### `scheme list`

```
$ icelines scheme list
yahoo-standard      (built-in)   17 categories
yahoo-pp            (built-in)   12 categories
espn-standard       (built-in)   15 categories
my-league           (user)       21 categories
```

Columns: scheme name, source (built-in / user), category count
(non-zero weights). Sorted alphabetically. User schemes that shadow a
built-in show only as `(user)` with a note in the footer:

```
* my-league shadows the built-in yahoo-standard
```

### `scheme show <NAME>`

```
$ icelines scheme show yahoo-standard
Scheme: yahoo-standard (built-in)

Stat              Weight   Category
goals             3.0      offense
assists           2.0      offense
shots             0.6      offense
hits              0.5      defense
blocked_shots     0.4      defense
plus_minus        0.5      defense
power_play_goals  1.0      special
short_handed_goals 1.5     special
...
```

Columns: stat name (snake_case, matching `Player` fields), weight
(float, can be negative), category (free-form grouping for display).
Output is sorted by category then stat name.

### `scheme fromcsv <PATH> [--name NAME]`

Read a Yahoo Fantasy Hockey CSV export, **detect** which stat columns
are present, and write a starter `~/.icelines/schemes/{name}.toml`.

```
$ icelines scheme fromcsv ./yahoo-2026.csv --name my-league
Detected 17 scoreable stat columns:
  G, A, PTS, +/-, PIM, PPG, PPP, SHG, SOG, FW, HIT, BLK, ...
Wrote ~/.icelines/schemes/my-league.toml with default Yahoo-standard weights.
Edit the file to customize weights, then run:
  icelines fantasy league-create "My League" --scheme my-league
```

If `--name` is omitted, the scheme name is derived from the CSV
basename (`yahoo-2026.csv` → `yahoo-2026`). If a file with that name
already exists in `~/.icelines/schemes/`, the command errors and
suggests `--force` (planned, not in v1).

**Detection rules**: each column header is mapped to a canonical stat
name via a fixed alias table:

| CSV header | Canonical stat |
|---|---|
| `G` / `Goals` | `goals` |
| `A` / `Asts` | `assists` |
| `PTS` / `Points` | `points` |
| `+/-` / `+/-` | `plus_minus` |
| `PIM` | `pim` |
| `PPG` | `power_play_goals` |
| `PPP` | `power_play_points` |
| `SHG` | `short_handed_goals` |
| `SHP` | `short_handed_points` |
| `GWG` | `game_winning_goals` |
| `OTG` | `ot_goals` |
| `SOG` / `Shots` | `shots` |
| `S%` | `shooting_pct` |
| `FW` / `FOW` | `faceoffs_won` |
| `HIT` | `hits` |
| `BLK` | `blocked_shots` |
| `GVA` | `giveaways` |
| `TKA` | `takeaways` |

Unknown headers are listed at the end with a warning so the user can
extend the alias table.

---

## TOML schema (recap from `fantasy-scheme.md`)

```toml
name = "my-league"
description = "Custom 14-cat league"

[weights]
goals               = 3.0
assists             = 2.0
shots               = 0.6
hits                = 0.5
blocked_shots       = 0.4
plus_minus          = 0.5
power_play_points   = 1.0
short_handed_goals  = 1.5
faceoffs_won        = 0.05
# Stats with weight 0 (or absent) contribute 0 to fantasy score.
```

Stats not listed default to weight 0. Negative weights are valid
(e.g. `pim = -0.2` for leagues that penalize penalties).

`compute_fantasy_score(player, scheme)` iterates the scheme's
`weights` map, multiplies each weight by the corresponding `Player`
field, sums, returns `f64`.

---

## Storage layout

```
~/.icelines/
├── schemes/
│   ├── my-league.toml
│   ├── friends-league.toml
│   └── deep-league-2026.toml
├── icelines.db
└── ...
```

Built-in schemes live in the binary at `icelines-core/data/schemes/`
and are loaded via `include_str!`. They cannot be edited in place; to
override, copy to `~/.icelines/schemes/` and edit there.

---

## Decisions (Open Questions resolved)

1. **User vs built-in precedence**: User wins. Reason: matches
   common dotfile conventions (user overrides system).

2. **`fromcsv` writes a *template* with default weights, not the
   actual Yahoo weights**: Yahoo's CSV doesn't include weights — only
   columns. The user must adjust weights manually. The template
   uses `yahoo-standard` weights as a starting point so most leagues
   need only minor tweaks.

3. **CSV detection is fixed-table, not heuristic**: Rules above are
   hardcoded in `commands/scheme.rs`. New columns require a code
   change. Trade-off: predictable + diffable vs flexible. For v1,
   predictability wins.

4. **No `scheme create` command**: Use `fromcsv` with a sample CSV,
   or hand-edit a TOML in `~/.icelines/schemes/`. A `scheme create
   --interactive` is backlog (low priority — TOML is simple).

5. **No `scheme delete` command**: Use `rm
   ~/.icelines/schemes/{name}.toml`. The shell handles it. Adds
   value only if we add validation (e.g. "this scheme is in use by 3
   leagues"); deferred.

6. **No `scheme validate`**: TOML parsing already errors on malformed
   files. A separate validate command would be redundant. Possible
   future: detect typos in stat names (`goals` vs `goal`).

7. **No version field in scheme TOML**: Schemas are append-only —
   adding new keys doesn't break existing schemes. Renaming or
   removing a key would, but isn't planned.

---

## Test coverage

L0 / L1:
- `Scheme::load_builtin` returns each built-in scheme without I/O
- `Scheme::load_user` reads from a tempdir with `HOME` overridden
- `Scheme::load` shadow precedence: user beats built-in
- `csv_detect_yahoo_export` against a fixture CSV
- `compute_fantasy_score` with negative weight (`pim = -0.2`)

L2 (subprocess):
- `l2_cmd_scheme_list_includes_yahoo_standard`
- `l2_cmd_scheme_show_unknown_exits_nonzero`
- `l2_cmd_scheme_fromcsv_writes_toml` — uses tempdir + verifies file content

---

## Future work (v2+)

| Item | Priority | Why deferred |
|------|----------|--------------|
| `scheme show --source` to disambiguate user vs built-in | LOW | Easy add when needed |
| `scheme create --interactive` | LOW | TOML hand-edit is simple |
| `scheme validate` (typo detection) | LOW | Most errors caught by parser |
| `scheme export <name>` → portable JSON | LOW | TOML is already portable |
| Scheme aliasing (`--alias yahoo-std=yahoo-standard`) | LOW | Tab-complete is fine |
| ESPN / Sleeper / Fantrax CSV detection | MED | Yahoo covers ~70% of users |
| Goalie-aware schemes | depends | Needs goalie data |
