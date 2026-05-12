# Markdown Export — Specification

**Version**: 1.1
**Date**: 2026-04-29
**Status**: Implemented (7 of 7 shapes shipped)

---

## Purpose

Generate **markdown data tables** suitable for `proof` /
`mdpath` / `dashboard-engine.md` integration. Produces structured
tables — one per dashboard component (player leaderboard, team depth
chart, fantasy standings, etc.) — that the proof renderer compiles
into ASCII for the TUI or HTML for the web site.

This is the **bridge** between IceLines' analytical output and
proof's declarative dashboard runtime. Today the TUI hand-crafts each
screen's render; with `export md`, every screen becomes a
`.dashboard.source.md` template that proof compiles.

---

## CLI

```
icelines export md  <SHAPE>  [--out PATH] [--filters …] [--season YYYYZZ]
```

Shapes (one of):

| Shape | Output table |
|-------|--------------|
| `leaders` | Top-N leaderboard (mirrors `query leaders`) |
| `team`    | Single team's lineup card |
| `depth`   | Cross-team line-value rankings |
| `fantasy` | Active league standings |
| `compare` | Two-player head-to-head |
| `series`  | One playoff series (game log + scorers) |
| `roster`  | All teams' rosters in one big table |

Filters mirror the corresponding query / TUI command:
- `leaders` accepts `--pos`, `--age-max`, `--top`, `--sort`, etc.
- `team` accepts `--team SEA`
- `compare` accepts `--p1 NAME --p2 NAME`

Output goes to `--out PATH` (file) or stdout if omitted. Default
path for the `proof` integration is `~/.icelines/reports/{shape}.md`.

---

## Output format

Every export produces one or more **GitHub-flavored markdown tables**
plus a YAML front-matter block that proof reads:

```markdown
---
type: leaderboard
title: Top 25 Centers (PPG)
generated_at: 2026-04-28T14:23Z
season: 20252026
filters:
  pos: C
  age-max: 23
  top: 25
proof:
  width: 80
  height: 30
---

| Rank | Player           | Team | Age | GP  | PPG  | Pts/82 |
|------|------------------|------|-----|-----|------|--------|
|  1   | Macklin Celebrini| SJS  | 19  | 65  | 1.412| 115.8  |
|  2   | Connor Bedard    | CHI  | 21  | 70  | 1.286|  99.0  |
| ...  |                  |      |     |     |      |        |
```

Front-matter keys:
- `type` — proof DASHBOARD-SPEC component type (`leaderboard`,
  `lineup-card`, `series-log`, etc.)
- `title` — human-readable
- `generated_at` — ISO-8601 UTC
- `season` — 8-digit season ID
- `filters` — verbatim of CLI args (lets proof regenerate)
- `proof` — render hints (width, height, color preferences)

---

## Per-shape table schemas

### `leaders`

```
| Rank | Player | Team | Pos | Age | GP | G | A | Pts | PPG | Pts/82 |
```

### `team`

Three sections (forwards, defense, goalies when available):

```
## Forwards

| Slot | LW | C | RW |
|------|-----|---|-----|
| Line 1 | A | B | C |
| Line 2 | D | E | F |

## Defense

| Slot | LD | RD |
| Pair 1 | A | B |
| ... |
```

### `compare`

```
| Stat | Player A | Player B | Diff |
|------|----------|----------|------|
| GP | 70 | 65 | +5 |
| Goals | 32 | 28 | +4 |
| ...
```

### `series`

```
## Game Log

| Game | Date | Result |
|------|------|--------|
| 1 | Apr 19 | FLA 4-2 TBL |
| ... |

## Leading Scorers

| Player | Team | G | A | Pts |
| Reinhart | FLA | 3 | 5 | 8 |
```

### `roster`

One big table — 32 teams × ~23 players each. Use `--top` to limit.

---

## Integration with proof

When proof's DASHBOARD-SPEC is implemented:

1. Each TUI screen becomes a `.dashboard.source.md` template that
   `include`s the relevant exported table.
2. `proof compile --width 100 --height 30 SCREEN.dashboard.source.md`
   produces a static ASCII layout the TUI renders.
3. The TUI no longer hand-crafts render code per screen — it loads
   the compiled output and prints.

This is the **dashboard-engine** vision (see `dashboard-engine.md`).
Today the TUI renders each screen with hand-written `Paragraph` /
`List` widgets; in v2, the rendered output is just bytes from
`proof compile`.

`export md` is the data-side prerequisite for that integration.
The proof side is tracked separately in the proof repo.

---

## Storage layout

```
~/.icelines/reports/
├── leaders-20252026.md
├── leaders-20252026-pos-c.md   ← filtered variants
├── team-EDM.md
├── depth.md
├── fantasy.md                  ← uses active league
├── series-A.md
└── ...
```

The directory is created lazily on first `export md` call. Files
are overwritten on rerun.

---

## Decisions (Open Questions resolved)

1. **GitHub-flavored markdown tables, not HTML or JSON**: Markdown
   tables work in both proof's ASCII renderer and any markdown
   viewer (GitHub, mkdocs, Obsidian). HTML would lose ASCII
   compatibility; JSON would lose human readability.

2. **Front-matter is YAML, not TOML**: Standard convention; mkdocs +
   pandoc both parse YAML front-matter natively.

3. **Generated-at timestamp is included**: Even though the rest of
   the site-generation pipeline avoids timestamps for determinism,
   exports are *snapshots in time* by definition — the timestamp is
   the point. Proof can ignore it for diff comparisons via a config
   flag.

4. **One table per file vs. multi-table files**: Single-table per
   file in v1. Cleaner mental model. Multi-table outputs (e.g. team
   page with forwards + D + roster) write multiple files.

5. **Proof DASHBOARD-SPEC dependency**: This spec describes the
   output format independent of proof's compile pipeline. Proof can
   land its DASHBOARD-SPEC in any order; this export format is
   stable.

6. **No per-player export shape in v1**: Use `scouting --format markdown`
   (see `scouting-reports.md`).

---

## Test coverage (when implemented)

L0:
- Each shape's table schema matches expected column order
- Front-matter is valid YAML
- Filter args round-trip into front-matter

L1:
- `export md leaders --pos C --top 5` against fixture data produces
  exactly five rows
- `export md team SEA` produces a 4-line + 3-pair lineup card

L2 (subprocess):
- `l2_cmd_export_md_leaders_exits_zero`
- `l2_cmd_export_md_team_writes_file`

---

## Status

**Implemented (partial)** as of 2026-04-29 (Phase 8d).

Seven shapes shipped:

| Shape | Status | Notes |
|-------|--------|-------|
| `leaders` | ✅ Implemented | sort=`pts-pace` only in v1; other metrics in 8d follow-up |
| `team` | ✅ Implemented | full skater roster (no L1/L2/L3 line-card grid yet) |
| `depth` | ✅ Implemented | Top 50 cross-team metrics, sorted by delta |
| `compare` | ✅ Implemented | GP/G/A/Pts/Pts82/PPG with diff |
| `roster` | ✅ Implemented | grouped by team alpha, then pace within team |
| `fantasy` | ✅ Implemented | fantasy poacher report rendered from `PoachReportView`; source omissions disclose missing league imports |
| `series` | ✅ Implemented | playoff game log rendered from `PlayoffsView` and the historical playoffs bundle |

13 L0 tests + 3 L2 subprocess tests. proof DASHBOARD-SPEC integration
is unblocked — the smoke test in `icelines-cli/tests/proof_lib_smoke.rs`
proves library link works; consuming the exported markdown into the
TUI is a separate phase.

---

## Future work (v2+)

| Item | Priority | Why deferred |
|------|----------|--------------|
| HTML output (`--format html`) | LOW | Markdown → HTML is a render-time transform |
| JSON output (`--format json`) | MED | Useful for non-proof consumers |
| `--diff <PREV>` to show change since previous export | LOW | Workflow nicety |
| `--watch` to regenerate on snapshot change | LOW | Most users batch |
| Per-player exports | MED | Use `scouting` for now |
| Embedded images / charts | LOW | Out of scope for markdown layer |
