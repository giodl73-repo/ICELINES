# Site Generation — Specification

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Implemented

---

## Purpose

Generate a static **mkdocs site** from the current player snapshot:
one page per team with a depth-chart lineup card, a team-rankings
landing page, and per-player references. The generated markdown is
versioned in `docs/` and rendered to HTML by mkdocs (Material theme).

The site is the canonical "share this with non-CLI users" surface.
The fantasy HTTP server (`fantasy serve`, see `fantasy-leagues.md`)
is dynamic and live; the static site is a snapshot baked at build
time.

---

## CLI commands

```
icelines build                  # render markdown into docs/
icelines serve [--port 8000]    # build + launch mkdocs dev server
icelines deploy [--remote origin]  # build + mkdocs gh-deploy
```

All three are thin wrappers over `icelines-site::SiteBuilder` plus
subprocess calls to the `mkdocs` binary (Python). `mkdocs` is
**not** a Cargo dependency — users install it separately:

```
pip install mkdocs-material
```

If `mkdocs` is missing, `serve` and `deploy` error with an explicit
hint to install it.

---

## Crate boundary

`icelines-site` is a small crate that exposes:

```rust
pub struct SiteConfig {
    pub docs_dir:     PathBuf,    // usually "docs/"
    pub mkdocs_yml:   PathBuf,    // usually "mkdocs.yml"
    pub snapshot_dir: PathBuf,    // ~/.icelines/snapshots/
    pub season:       u32,        // 20252026
}

pub struct SiteBuilder { /* ... */ }
impl SiteBuilder {
    pub fn new(config: SiteConfig) -> Self;
    pub fn build(&self) -> Result<Vec<String>, SiteError>;  // returns generated paths
}
```

`SiteBuilder::build()` reads snapshots into the stats repository, sorts player
views by pace, builds generated team pages via `TeamDepthView`, computes
cross-team metrics via `compute_cross_team_metrics`, and writes markdown files.
It also rewrites `mkdocs.yml`'s nav to reflect the current pace-ranked team
order (see `nav.rs`).

---

## Generated layout

```
docs/
├── index.md                ← Landing page: 32 teams ranked by aggregate pace
├── teams/
│   ├── EDM.md              ← One page per team
│   ├── COL.md
│   └── ...
├── guides/                 ← Hand-authored, compiled from src/guides/
│   └── 06-tui.md
└── TUTORIAL.md             ← Hand-authored
```

Generated files (`index.md`, `teams/*.md`) are **regenerated every build**
and should not be hand-edited. Hand-authored docs (`guides/`,
`TUTORIAL.md`) are preserved.

---

## Team page format

Each `teams/{ABBR}.md` follows this structure:

```markdown
# Edmonton Oilers

Pace rank: **1**

<div class="team-header">
  <img src="https://assets.nhle.com/logos/nhl/svg/EDM_light.svg">
  <div>
    <h2>Lineup card</h2>
    <p>Pts/82 totals based on current snapshot</p>
  </div>
</div>

## Forwards

| LW | C | RW |
|----|---|----|
| <span class="fit-elite">Player A · 1.45 → 119</span> | ... | ... |
| ... three more lines ... |

## Defense

| LD | RD |
|----|----|
| ... three pairs ... |

## Goaltenders

Goalie rows are available from the bundled goalie repository when the selected
season has goalie data.

## All skaters (ranked)

| Rank | Player | Pos | GP | G | A | Pts/82 | Pace fit |
|------|--------|-----|----|----|----|--------|---------|
| ... full team table ... |
```

**Player cell formatting** (`html.rs::player_cell`):
- consumes `DepthPlayerSlot` metrics from `TeamDepthView`
- `<span class="fit-elite">` for top 25% of cross-team peers
- `fit-solid` for next 25%
- `fit-buried` for second-quartile
- `fit-stretch` for bottom quartile
- Bare `<span>` (no class) when no cross-team metric exists

The four classes are styled by `docs/assets/fantasy.css` (green /
yellow / blue / red, matching the TUI fit indicators).

---

## Index page format

`docs/index.md`:

```markdown
# IceLines — NHL Depth Chart Tracker

The 32 NHL teams, ranked by aggregate skater pace (pts/82).

| # | Team | Aggregate Pace | Top 3 |
|---|------|----------------|-------|
| 1 | Edmonton Oilers (EDM) | 234.5 | McDavid · Draisaitl · Bouchard |
| 2 | ... |
```

The header bar (search, theme toggle, etc.) comes from
`mkdocs.yml`'s `theme: material` config.

---

## mkdocs.yml integration

The site uses **mkdocs Material**. Key config:

```yaml
site_name: IceLines
site_description: NHL depth charts, pace-adjusted rankings, ...
docs_dir: docs
site_dir: ../fantasy-site

theme:
  name: material
  features: [navigation.instant, navigation.tabs, content.code.copy, ...]

markdown_extensions:
  - tables
  - attr_list
  - admonition
  - pymdownx.superfences
```

The `nav:` section is **rewritten** by `SiteBuilder::build()` —
specifically by `nav::update_nav(yml_path, ranked_teams)`. The
rewrite reorders the Teams section to match the current pace
ranking. The Guides / Tutorial sections (if present) are preserved
verbatim.

`update_nav` parses YAML manually (no `serde_yaml` dep) to avoid
disturbing comments or unusual key ordering. It's idempotent — running
`build` twice in a row produces byte-identical output.

---

## CSS classes

`docs/assets/fantasy.css` defines:

| Class | Color | Meaning |
|-------|-------|---------|
| `.fit-elite` | green | Top quartile vs cross-team peers at same line slot |
| `.fit-solid` | yellow | Above-median |
| `.fit-buried` | blue | Below-median (talent on a stacked team) |
| `.fit-stretch` | red | Bottom quartile (overplaying their level) |
| `.team-header` | flex container | Logo + intro at top of team page |
| `.bar-fill` | inline div with width % | Bar chart fill in player cells |

Adding a new fit tier requires updating both the `WebFitClass` enum
in `icelines-core::cross_team` and `fantasy.css`.

---

## Build / serve / deploy lifecycle

### build

`icelines build`:
1. Resolve `mkdocs.yml` directory upward from CWD (or error).
2. `SiteBuilder::new(config).build()` — generates markdown.
3. Print summary: `Generated 33 files in docs/ (+ updated mkdocs.yml).`

No mkdocs invocation; output is just markdown.

### serve

`icelines serve [--port 8000]`:
1. Run `build` (always, not optional).
2. `Command::new("mkdocs").args(["serve", "--dev-addr", "127.0.0.1:8000"])`.
3. Inherit stdout/stderr — user sees the live-reload server logs.
4. Block until the user kills the process (Ctrl+C).

Errors:
- `mkdocs` not found → "install with `pip install mkdocs-material`"
- mkdocs exits non-zero → propagated

### deploy

`icelines deploy [--remote origin]`:
1. Run `build`.
2. `Command::new("mkdocs").args(["gh-deploy", "--remote-name", "origin", "--force"])`.
3. mkdocs handles the actual git push to the `gh-pages` branch.

`--force` is hardcoded — site deploys are routinely full overwrites,
not history-preserving. Concern about losing custom commits on
`gh-pages` is out of scope (the branch is regenerated, not authored).

---

## Generation determinism

For a given snapshot input, the generated markdown is **byte-for-byte
deterministic**:
- Player order is sorted by pace, ties broken by `nhl_id`.
- HashMaps are walked via `BTreeMap` or sorted Vec keys before render.
- Floats are formatted with fixed precision (`{:.2}` or `{:.0}`).
- Time-of-day-dependent text (e.g. "Generated at 14:23 UTC") is **not**
  embedded.

This makes the site git-diffable: a no-op rebuild produces zero diff.

---

## Decisions (Open Questions resolved)

1. **No Tera (or any) template engine in v1**: The markdown is
   simple enough that string concatenation in `builder.rs` is
   readable and debuggable. Tera was prototyped, decided
   over-engineered. `dashboard-engine.md` (declarative dashboards)
   is a separate, deferred concern.

2. **mkdocs as Python dep, not a Rust port**: Material theme is
   excellent and well-maintained. Reimplementing it in Rust is a
   six-month project for marginal value.

3. **Static-only site**: Dynamic features (search, sorting) come from
   the Material theme's client-side JS. No server-side rendering.
   The fantasy dashboard is the dynamic surface — see
   `fantasy-leagues.md`.

4. **`site_dir: ../fantasy-site`**: The output directory is one level
   up from the project so the GitHub Pages action can publish from a
   sibling repo without confusing the source repo's git history.
   This is unusual; consider documenting in CLAUDE.md.

5. **Per-player pages**: **Not** in v1. The team page covers most
   needs. Per-player pages would explode the site to ~700 files
   (32 teams × 23-player roster) without much marginal value.

6. **Search**: Provided by mkdocs' built-in `search` plugin.
   Index is generated by mkdocs at build time. No custom configuration.

---

## Test coverage

L1 (in `icelines-site/tests/`):
- `builder_generates_32_team_pages` — count + non-empty assertion
- `builder_idempotent` — two builds produce identical output
- `nav_rewrite_preserves_guides` — `update_nav` round-trip with guides
- `player_cell_renders_fit_classes` — class selection per metric

L2 (subprocess) in `system_tests.rs`:
- `l2_cmd_build_exits_zero` — `icelines build` from a tempdir with
  fixture snapshot
- `l2_cmd_serve_help_exits_zero` — `serve --help` parses

Live mkdocs invocation is **not** tested in CI (would require Python
in CI runners). A manual smoke test runs before each release.

---

## Future work (v2+)

| Item | Priority | Why deferred |
|------|----------|--------------|
| Tera templates / `dashboard-engine.md` integration | MED | Spec exists, blocked on proof DASHBOARD-SPEC |
| Per-player pages | LOW | Site explosion without clear value |
| Goalie section | depends | Needs goalie data in snapshots |
| Custom search ranking | LOW | mkdocs default is acceptable |
| Dark/light theme toggle | DONE | Material theme handles it |
| Cross-season comparison pages | LOW | `query compare` covers it via CLI |
| Per-page generation timestamp | LOW | Conflicts with determinism goal |
