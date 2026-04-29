# Scouting Reports — Specification

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Implemented

---

## Purpose

Produce a single-player **scouting report** suitable for trade-deadline
research, draft-prep workups, or fantasy-roster decisions. The report
combines bio, current-season stats, career trajectory, peer comparison,
linemates, depth-chart slot, cross-team value, and a final fit summary
in one command.

```
icelines scouting "Cale Makar"
icelines scouting "Bouchard" --format markdown
icelines scouting "Celebrini" --format json
```

---

## CLI

```
icelines scouting <PLAYER>  [--format terminal|markdown|json]
```

Default format: `terminal`. The format flag is a lowercase string
strictly matched against the three values; unknown formats error out
with `unknown format '{X}' — valid: terminal, markdown, json`.

Player resolution uses partial-name matching against the loaded player
set (same logic as `query player`, `group add`, etc.). An ambiguous
match errors with the candidates listed.

---

## Report sections

The report has **eight sections**, in fixed order. Markdown uses `## N.
Title`; terminal uses uppercase headings without the `##`.

### 1. Bio

```
## 1. Bio
  Team:         EDM (Edmonton Oilers)
  Position:     Center
  Age:          26
  Nationality:  CAN
  Draft:        2015 · Round 1 · Pick #1
  Handedness:   L
```

Fields: team abbrev + full name, position enum (`Center`, `LeftWing`,
etc.), age in years (computed from `date_of_birth`), three-letter
nationality code, draft year/round/overall (or `—` if undrafted),
shoots/catches (`L`/`R`).

### 2. Current Season

```
## 2. Current Season
  GP:           42
  G:            18  →  35/82
  A:            41  →  80/82
  PPG:          1.405 pts/gp
  G/gp:         0.429
  Proj/82g:     115.2
```

Pace projections come from `Player::pace_score` (see
`projection-engine.md`). If `gp < MIN_GP` (currently 10), this section
shows `< {MIN_GP} games played — not enough data` and skips numeric
output.

### 3. Career Trajectory

```
## 3. Career Trajectory
  Season    Team   GP   G    A    PPG
  21-22     EDM    80  44   79   1.538
  22-23     EDM    82  64   89   1.866   ← peak
  23-24     EDM    76  32   100  1.737
  24-25     EDM    79  26   74   1.266
  ...
  Career PPG: 1.523  Peak: 22-23 (1.866)
```

Season abbreviations are `YY-YY` (e.g. `21-22`). Peak season is
flagged inline. Career PPG is the GP-weighted average across all
seasons in the snapshot/bundle.

### 4. Peer Group Rank

```
## 4. Peer Group Rank (same draft year ± 1, same position)
  1.  Connor McDavid   EDM   1.405
  2.  Nathan MacKinnon COL   1.402
  3.  Cale Makar       COL   1.244     ← target
  ...
  Cohort: 23 C/D drafted 2014–2016
```

The cohort is `draft_year ± 1` and same position. If the player has
no `draft_year` (undrafted), section shows `Cohort unavailable —
undrafted player`.

### 5. Linemates

```
## 5. Linemates (current team — top forwards / D)
  Connor McDavid     C    1.405
  Leon Draisaitl     C    1.366
  Zach Hyman         LW   0.930
  ...
```

Top 5 same-team players excluding the target. Used for context — who
is the player playing with?

### 6. Depth Chart Position

```
## 6. Depth Chart Position
  Line/Pair: 1st pair (LD)
  Team rank: 2nd of 7 D
  Cross-team rank: 3rd LD league-wide
```

Driven by `DepthChartBuilder` (see `depth-chart.md`) and
`compute_cross_team_metrics`.

### 7. Cross-Team Value

```
## 7. Cross-Team Value
  Average line/pair across all 32 teams: 4th LD
  This player is on a 1st pair → would play 4th LD on average team
  Fit class: Stretch (overplaying his level)
```

Fit class is one of `Elite`, `Solid`, `Buried`, `Stretch` (see
`depth-chart.md` §2.2 for thresholds).

### 8. Fit Interpretation

```
## 8. Fit Interpretation
  Projected pts/82: 109.4 (regressed)
  Confidence: ±12.3 (medium — 42 GP, age curve flat)
  Recommendation: Hold / acquire — pace + cohort rank both top-quartile
```

Projection is the `regressed` mode from `projection-engine.md`.
Recommendation is a one-line synthesis driven by:
- Pace (above/below position median?)
- Peer rank (top quartile / median / bottom?)
- Fit class (does the team utilize them well?)
- Age (rising / peak / declining?)

---

## Output formats

### `terminal` (default)

ANSI-colored, fixed-width text via `comfy_table` and `owo_colors`.
Section headers in cyan bold; numeric callouts (peak, target row) in
yellow. Fit class colors match the TUI:
- Green (Elite), Yellow (Solid), Blue (Buried), Red (Stretch).

Width is 80 columns. Longer player names truncate at 22 chars.

### `markdown`

Pure markdown — same content as terminal, just with `# Heading 1` /
`## Heading 2`, `*italics*`, and pipe-table syntax. No ANSI codes.
Suitable for committing into reports/, pasting into a wiki, or feeding
into proof DASHBOARD-SPEC (see `dashboard-engine.md`).

### `json`

Machine-readable — one JSON object with eight keys mirroring the
section names plus a top-level `player` object. Pretty-printed:

```json
{
  "player": {
    "name": "Cale Makar",
    "nhl_id": 8480069,
    "team": "COL",
    "position": "Defense"
  },
  "bio": { "age": 26, "nationality": "CAN", ... },
  "current_season": { "gp": 42, "goals": 18, ... },
  "career": [ {...}, {...} ],
  "peer_group": [ {...} ],
  "linemates": [ {...} ],
  "depth_chart": { "pair": "1st", "rank": 2 },
  "cross_team": { "avg_slot": "4th LD", "fit": "Stretch" },
  "fit": { "projection_82": 109.4, "confidence": 12.3, "recommendation": "..." }
}
```

Field naming uses snake_case for keys.

---

## Decisions (Open Questions resolved)

1. **Section order is fixed**: Reports are grep-able and diffable
   when section headers are stable. No `--sections` flag in v1.

2. **Eight sections, not nine**: Goalie / save% data is omitted.
   Goalies require tier-2 data not bundled. Skater-only.

3. **No PDF / HTML output in v1**: Markdown can be converted by any
   external tool (pandoc, mkdocs, etc.). Adding native PDF requires
   a renderer dep we don't have.

4. **`comfy_table` for terminal alignment**: Already a workspace dep
   (used by `query leaders`); reusing avoids new deps.

5. **JSON shape is stable**: Field names are documented above; future
   versions add keys without renaming existing ones. Consumers can
   grep `"player.name"` reliably.

6. **`--format json` does not include the section text**: It includes
   the structured data; the human-readable narrative is rebuildable
   from the data. Consumers who want pre-rendered text run
   `--format markdown`.

7. **Peer cohort = draft year ±1**: Wider cohorts dilute the signal;
   narrower (exact year) starves data. Tested empirically; ±1 is the
   sweet spot.

8. **Confidence band**: Computed from GP-played + age-curve slope, not
   bootstrap. A v2 could use `--bootstrap N` for proper CIs.

---

## Test coverage

The command is implemented but **dedicated tests are not yet
written** (the report renders correctly under manual smoke testing
against bundled data). The following coverage is recommended:

L0 (unit, in `commands/scouting.rs`):
- `format_terminal_includes_all_eight_sections` — render output, grep `## 1.`–`## 8.`
- `format_markdown_uses_h2_headings`
- `format_json_has_section_keys`
- `unknown_format_errors_with_valid_options_listed`
- `low_gp_skips_current_season_numerics`

L1 (integration):
- Full report round-trip against fixture players (McDavid-like,
  Beniers-like, Makar-like in `mock_nhl_api.rs`) with expected output
  snapshots.

L2 (subprocess) in `tests/system_tests.rs`:
- `l2_cmd_scouting_terminal_exits_zero`
- `l2_cmd_scouting_json_parses` — pipes output to a JSON validator
- `l2_cmd_scouting_unknown_format_exits_nonzero`

Adding these is tracked in `design/plans/INDEX.md` backlog under
"scouting test coverage".

---

## Future work (v2+)

| Item | Priority | Why deferred |
|------|----------|--------------|
| Goalie scouting | HIGH (when goalie data lands) | Needs tier-2 fetches |
| `--bootstrap N` confidence bands | MED | Empirical age-slope is fine for v1 |
| `--save FILE` to write to a file | LOW | Use shell `>` redirection |
| HTML / PDF export | LOW | Pandoc covers it |
| Diff between two snapshots (`--vs SNAPSHOT`) | MED | Useful for trade-eve analysis |
| Cohort tunables (`--cohort-size N`) | LOW | Default ±1 works |
| Per-game splits (5v5 / PP / PK) | depends | Needs strength-state data not bundled |
