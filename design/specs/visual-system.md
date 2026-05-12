# IceLines visual system

**Status**: Draft - Prince of Wales baseline
**Owner phase**: Prince of Wales - ASPECT visual system
**Rubric**: DEGAS ASPECT v3.0 from `c:\src\degas\scoring\RUBRIC.md`

This spec turns visual quality into a testable product contract. It does not
replace the ViewModel contracts; it describes how CLI, TUI, web, and markdown
surfaces should render those contracts with a shared hockey-native visual
language.

---

## Design Aim

IceLines is a local NHL analysis command center. The primary effect is fast,
composed, current, domain-specific decision support.

The product must feel like:

- a hockey operations room for player, team, goalie, schedule, and fantasy
  decisions;
- a sports broadcast dashboard for live context and scan rhythm;
- a statistical workbook for ranking, filtering, and comparison;
- a terminal-native tool when rendered in the TUI or CLI.

The product must not feel like:

- a generic admin dashboard;
- raw implementation tables dumped to the screen;
- a marketing landing page;
- a decorative sports skin that hides source truth or weakens scan speed.

---

## ASPECT Baseline

Scores are rough current-state review scores. They are not release claims.

| Surface | Aim | School | Precision | Effect | Clarity | Truth | Total | Target |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| TUI home/dashboard | 9 | 8 | 7 | 7 | 8 | 15 | 54 | 75 |
| TUI stats/leaders | 10 | 9 | 8 | 8 | 10 | 16 | 61 | 78 |
| TUI team/depth | 10 | 8 | 7 | 8 | 9 | 16 | 58 | 78 |
| TUI goalies | 10 | 9 | 8 | 8 | 10 | 16 | 61 | 78 |
| TUI scores/schedule/playoffs | 10 | 8 | 7 | 8 | 9 | 16 | 58 | 78 |
| CLI tables | 11 | 10 | 9 | 8 | 11 | 17 | 66 | 78 |
| Web home | 10 | 9 | 8 | 8 | 10 | 16 | 61 | 80 |
| Web leaders/player/team/goalies/depth | 10 | 9 | 8 | 8 | 10 | 16 | 61 | 80 |
| Web schedule/playoffs/transactions | 10 | 9 | 8 | 8 | 10 | 16 | 61 | 80 |
| Markdown exports/reports | 11 | 10 | 9 | 9 | 11 | 17 | 67 | 78 |

Current pattern:

- Truth is comparatively strong because ViewModels now carry stable identity,
  source context, metrics, and warnings.
- Clarity is acceptable on many table surfaces, but context and next action are
  inconsistent.
- Effect and School are the weak points: the product is useful, but not yet
  visually hockey-native or composed.
- Precision is uneven: tables carry value, but borders, spacing, glyphs,
  labels, and emphasis are not governed by one visual grammar.

---

## Visual Grammar

IceLines uses a declared synthesis:

| School | Use |
|---|---|
| Information architecture | Navigation, task flow, active context, recovery actions |
| Statistical graphics | Rankings, player comparison, fit, score components, schedule density |
| Sports broadcast dashboard | Live games, standings/playoffs, team/player identity, scan rhythm |
| Schematic cartography | TUI panes, dense grids, spatial memory, keyboard workflows |

Rules:

- Dense is acceptable when hierarchy is clear.
- Hockey identity should come from actual team/player/game context, not generic
  decoration.
- Visual emphasis follows ViewModel semantics and source truth.
- Color never carries meaning alone.
- Every renderer needs an ASCII-safe path.
- Web stays no-build/no-SPA unless a later plan explicitly changes that.

---

## Shared Semantic Tokens

These are product-level visual meanings. Implementations may map them to
terminal styles, CSS classes, or plain-text labels.

| Token | Meaning | Required non-color cue |
|---|---|---|
| `primary_action` | next recommended action | verb label or leading command |
| `decision_highlight` | row or metric driving the decision | rank/label/strong weight |
| `fit_elite` | strong positive role/fit signal | star or `ELITE` label |
| `fit_solid` | acceptable role/fit signal | `SOLID` label |
| `fit_buried` | underused positive player | up arrow or `UNDERUSED` label |
| `fit_stretch` | overextended negative fit | down arrow or `OVEREXTENDED` label |
| `game_pre` | scheduled game | `PRE` label and start time |
| `game_live` | in-progress game | `LIVE` label and period/clock |
| `game_final` | completed game | `FINAL` label |
| `source_complete` | complete source for the view | `complete` source chip |
| `source_partial` | partial/degraded source | `partial` source chip |
| `source_stale` | stale cached source | timestamp/age chip |
| `warning` | recoverable issue | `warning` label |
| `error` | blocking issue | `error` label and recovery |

Renderer mappings must keep these names or document exact aliases.

---

## TUI Baseline

Audience: terminal-first users comparing and monitoring hockey data under
moderate time pressure.

Primary tasks:

- monitor games, schedules, and playoff state;
- compare leaders, team depth, goalies, poach candidates, and fantasy gaps;
- inspect one player/team and navigate without remembering all commands.

Top failures:

- **Hierarchy**: screens often read as equally weighted panes and tables.
- **Chrome pressure**: key hints and filters can crowd the main data at narrow
  widths.
- **Hockey identity**: team/player/game context is present, but visual rhythm is
  generic terminal UI rather than hockey command center.
- **State discoverability**: active season/type/source/filter/sort is not
  consistently obvious in five seconds.
- **Empty/error states**: some are technically correct but visually weak.

Prince targets:

- one global header grammar for season/type/source/context;
- one footer/keyhint grammar with priority and overflow behavior;
- one pane/title hierarchy for primary, secondary, and supporting content;
- explicit 80/120/160-column density rules;
- snapshot tests for representative default and filtered states.

---

## Web Baseline

Audience: local browser users inspecting players, teams, schedules, fantasy
boards, and reports.

Primary tasks:

- scan league/team/player state quickly;
- compare rows without terminal constraints;
- use bookmarkable routes for repeated workflows;
- read reports and poacher/fantasy outputs.

Top failures:

- **Composition**: pages work, but many still feel like route templates rather
  than one designed product.
- **Responsive density**: tables and cards need clearer mobile and wide-screen
  behavior.
- **Visual token drift**: CSS classes exist for some fit/status states, but the
  whole route set does not share a documented token vocabulary.
- **Context placement**: active season/type and source state are present in many
  routes, but the above-the-fold treatment is not yet a consistent visual
  contract.
- **Error/empty affordances**: route truth improved, but recovery actions need
  stronger visual treatment.

Prince targets:

- shared web tokens for source, game, fit, warning, and action states;
- consistent header/context band across major routes;
- table/card density rules for desktop and mobile;
- route screenshots reviewed against ASPECT;
- HTML tests for active context and applied filters where practical.

---

## CLI Baseline

Audience: scriptable and terminal-first users who need fast answers and stable
JSON/CSV.

Primary tasks:

- get one answer quickly;
- export or pipe machine-readable output;
- inspect team/player/goalie/fantasy summaries without opening the TUI.

Top failures:

- **Table rhythm**: output is useful but not consistently designed for 80-column
  reading.
- **Numeric alignment**: not every table has the same treatment for ranks,
  player/team labels, and metrics.
- **Context footers**: source/season/staleness is not always visible when it
  affects interpretation.

Prince targets:

- stable 80-column text snapshots for representative commands;
- shared table rules for rank/name/team/metric columns;
- source/context footer convention;
- no decorative styling in JSON/CSV.

---

## Markdown And Reports Baseline

Audience: users saving durable reports or reading generated docs.

Primary tasks:

- preserve analytical context;
- remain readable on GitHub/MkDocs;
- avoid renderer-specific claims.

Top failures:

- **Report hierarchy**: generated sections are correct but can read as exported
  tables rather than finished reports.
- **State context**: source/season context should be visible before conclusions.
- **Cross-surface visual drift**: markdown uses its own incidental emphasis.

Prince targets:

- report section hierarchy tied to `ReportView` section refs;
- standard source/context preamble;
- stable table column rules for GitHub/MkDocs reading.

---

## Implementation Constraints

- ViewModels own hockey meaning. Renderers own layout and styling.
- Renderer styling must not recompute rank, fit, score, eligibility, or source
  truth.
- Any color/status table added to TUI, CLI, or web must map back to this spec or
  a shared core semantic token.
- Use ASCII fallbacks for all glyphs.
- Do not make a broad redesign without before/after screenshots or text
  captures.
- Avoid one-note palettes. Hockey identity should be contextual, not a wash of
  one color.

---

## Prince Exit Gates

| Gate | Evidence |
|---|---|
| TUI scan rhythm | snapshots for Team/Depth, Goalies, Scores/Schedule at representative widths |
| Web responsive polish | desktop and mobile screenshots for home, leaders, team/player, fantasy/poach |
| CLI readability | 80-column snapshots for leaders, team, goalies, fantasy/poach |
| Token consistency | documented mapping from semantic token to TUI/CSS/text style |
| Truth preservation | active season/type/source state visible where interpretation depends on it |
| Accessibility | no color-only status; meaningful labels for fit, live/final, source, warning, error |

