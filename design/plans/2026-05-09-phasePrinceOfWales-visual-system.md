# Phase Prince of Wales - ASPECT visual system

**Date**: 2026-05-09
**Status**: Active - Prince.3 TUI scan rhythm started
**Trophy**: Prince of Wales Trophy. Fit: conference champion polish before the final release push. This phase turns the working platform into something that feels composed, readable, and worth opening every day.
**External rubric**: DEGAS ASPECT v3.0 from `c:\src\degas\scoring\RUBRIC.md`
**Estimated**: 3-6 sub-phases

---

## Why

IceLines has strong data ambition and a lot of surface area, but the product
experience is not yet good enough:

- the TUI is useful but visually plain and not "ASCII hot";
- tables and panels often read as implementation output rather than designed
  hockey analysis;
- the web surface is improving, but not yet beautiful or confident;
- the CLI/TUI/web do not share one deliberate visual language;
- GLASS, broadcast, and CREST checks exist, but there is no phase that makes
  visual quality a first-class deliverable.

Prince of Wales applies the DEGAS ASPECT rubric to IceLines as a functional
information design system. The target school is a synthesis:

- **information architecture** for navigation and task flow,
- **statistical graphics** for ranking/comparison,
- **sports broadcast dashboard** for live context and scan rhythm,
- **schematic cartography** for TUI layout discipline and spatial memory.

Beauty is not decoration here. Beauty means disciplined hierarchy, confident
spacing, readable comparison, consistent visual grammar, and enough hockey
character that the app feels like IceLines rather than a generic table printer.

---

## ASPECT gates

| ASPECT | IceLines gate |
|---|---|
| Aim | Each surface names its primary user task: decide, compare, monitor, inspect, or export. Layout must serve that task first. |
| School | TUI, CLI, and web must declare the visual grammar they use. Mixed grammars are allowed only when the tension is named. |
| Precision | Every border, glyph, color, label, and column earns its place. No decorative noise that reduces scan speed. |
| Effect | The intended effect is "hockey command center": fast, composed, current, domain-specific. A generic admin dashboard fails. |
| Clarity | A user can identify active season/type, applied filters, top row meaning, and next action without reading docs. |
| Truth | Visual emphasis must match data truth: no color-only claims, no hidden stale data, no overconfident partial source display. |

---

## Platform contracts consumed

Prince of Wales consumes `design/specs/platform-contracts.md` this way:

- **Data context**: source/completeness/season/type state is always visible when
  it affects interpretation.
- **Query/filter intent**: visual chips and labels reflect the shared typed
  filter/sort state, not renderer-local strings.
- **ViewModel**: redesign work starts from ViewModels; renderers may style and
  arrange, but may not recompute hockey logic.
- **Surface parity**: visual design reconciles CLI/TUI/web differences without
  inventing feature differences.
- **Visual language**: Prince owns the semantic token vocabulary and ASPECT
  review for TUI/CLI/web.

---

## Role review gates

| Role | Gate |
|---|---|
| HART | Active `(season, season_type)` is visible on every designed surface and included in visual state tests. |
| KEEL | TUI, CLI, and web use the same semantic color/status vocabulary even when rendered differently. |
| TAPE | Source freshness and missing-source state are visible where they affect a view. |
| FORGE | Visual system tokens live in shared modules or docs; renderers do not grow copy-pasted color tables. |
| PACE | Any visual scoring, ranking, or sparkline claim names the metric and avoids false precision. |
| BENCH | Golden/screenshot tests protect the main TUI/web layouts after redesign. |
| EDGE | Empty, narrow, high-density, stale, no-data, bad-filter, and colorblind cases are reviewed explicitly. |
| WIRE | Web error/partial states use typed route errors and no silent fallback. |
| SCOUT | Hockey hierarchy is sensible: live games, standings/playoffs, team/player/goalie context read like hockey, not CRM. |
| GLASS | Accessibility, readability, color semantics, and 5-second scan tests pass before closeout. |
| broadcast | Web responsive behavior, HTMX fragments, browser affordances, and sticky URLs pass before closeout. |
| CREST | Aesthetic review passes: composition, product identity, screenshot quality, palette discipline, and intentional visual rhythm. |

---

## Sub-phase ordering

```text
Prince.1  Visual inventory and ASPECT baseline
Prince.2  Shared visual language and tokens
Prince.3  TUI redesign for scan rhythm
Prince.4  Web design pass and responsive polish
Prince.5  CLI table polish and export readability
Prince.6  Golden/screenshot tests and docs closeout
```

---

## Prince.1 - Visual inventory and ASPECT baseline

Create `design/specs/visual-system.md`.

For each major surface, record:

- primary audience and task;
- visual school/synthesis;
- current ASPECT score, rough but honest;
- top 3 friction points;
- target ASPECT score for phase exit;
- screenshots or terminal captures for baseline comparison.

Required surfaces:

- TUI Home/dashboard
- TUI Stats/Leaders
- TUI Team/Depth
- TUI Goalies
- TUI Schedule/Scores/Playoffs
- CLI `team`, `query leaders`, `goalies`, and future LP commands
- Web home/leaders/team/player/goalies/depth/schedule/playoffs/transactions

Acceptance:

- The plan stops saying "ugly" as a feeling and names exact failures:
  hierarchy, spacing, typography/glyphs, color, empty state, density, or flow.
- At least one DEGAS-informed review table exists for TUI and one for web.

Progress:

- 2026-05-12: Created `design/specs/visual-system.md` with the ASPECT baseline,
  visual grammar, semantic token vocabulary, surface-specific failure modes, and
  Prince exit gates.
- 2026-05-12: Reviewed the baseline with `.roles` and recorded findings in
  `design/notes/2026-05-12-prince-visual-system-roles-review.md`. Tightened
  the spec around evidence-backed ASPECT scores, token ownership, ASCII
  fallbacks, web/mobile artifacts, and edge-state capture requirements.
- 2026-05-12: Added a dedicated CREST aesthetic review protocol so visual
  polish has an explicit pass/fail artifact review separate from GLASS
  readability.

---

## Prince.2 - Shared visual language and tokens

Define:

- semantic colors for fit, status, source state, live/final/pre-game, playoff
  state, and warnings;
- TUI glyph set with ASCII fallback;
- table density rules for 80/120/160-column terminals;
- web CSS tokens matching GLASS color contract;
- state chips for season/type/source/filter/sort.

Rules:

- Color never carries meaning alone.
- Renderer-local color tables are a bug.
- Fancy glyphs require ASCII fallback.
- Web must remain no-SPA/no-build unless a later plan explicitly changes that.

Acceptance:

- One visual token source or documented mapping exists.
- TUI, CLI, and web use the same names for semantic states.

Progress:

- 2026-05-12: Added the first shared semantic token source in core:
  `SemanticToken::key()`, `SemanticToken::ascii_label()`, and
  `ALL_SEMANTIC_TOKENS`. This keeps ViewModels semantic while giving renderers
  stable names and ASCII-safe labels.
- 2026-05-12: Expanded `design/specs/visual-system.md` with the complete
  semantic token table, renderer mapping seed for TUI/CLI/web/markdown, and a
  drift inventory for the next cleanup passes.
- 2026-05-12: Reconciled web/site fit styling to the Prince token names:
  `fit-elite`, `fit-solid`, `fit-buried`, and `fit-stretch`. Removed the active
  `.fit-fringe` contract from CSS/tests and added mapping fences.
- 2026-05-12: Added a shared CLI renderer visual mapping for fit colors,
  TUI depth fit styling, markdown-safe fit labels, and scouting descriptions.
- 2026-05-12: Moved fantasy simulation warning rendering from inline hex styles
  to shared web state classes, with template/static-asset fences.
- 2026-05-12: Moved repeated web route fetch/error blocks for compare,
  playoffs, schedule, and scores to the shared `.state-error` class.
- 2026-05-12: Added shared web context/source/meta classes and migrated
  representative scores, schedule, playoffs, and poach state text.
- 2026-05-12: Added shared `.empty-state` styling and migrated representative
  no-data states on team, schedule, scores, playoffs, compare, fantasy, and
  poach routes.
- 2026-05-12: Prince.2 token foundation is in place for core semantic labels,
  CLI fit mapping, web fit/state/context/source/empty classes, and the first
  TUI style helper layer. Remaining token work should happen as part of
  surface-specific Prince.3-Prince.5 polish.

---

## Prince.3 - TUI redesign for scan rhythm

Make the TUI feel intentional:

- clear global header with season/type/source state;
- stronger screen titles and pane hierarchy;
- consistent key hint grammar;
- fewer arbitrary borders;
- deliberate use of line art/glyphs for hockey context;
- compact but readable cards for player/goalie/team summary;
- predictable empty and error states;
- 80-column graceful degradation.

Acceptance:

- GLASS 5-second test passes on Team/Depth, Goalies, and Scores/Schedule.
- CREST screenshot test passes on Team/Depth, Goalies, and Scores/Schedule.
- Snapshot goldens cover default and filtered states.
- No hidden keybinds introduced by the redesign.

Progress:

- 2026-05-12: Added shared TUI style helpers for panel borders, screen titles,
  table headers, quiet metadata, warnings/errors, and selected rows.
- 2026-05-12: Applied the helper layer to Fantasy Gaps, Fantasy Simulation, and
  Fantasy Poacher as the first representative TUI scan-rhythm pass.
- 2026-05-12: Applied the same helper layer to Team/Depth roster, Goalies
  leaderboard/detail, and Schedule/Scores week/team/matchup flows. Focused
  Team, Goalies, and Schedule TUI tests pass after the migration.
- 2026-05-12: Added Prince.3 80x24 and 120x32 TUI render-contract tests for
  Team, Goalies, Schedule, and Poach so representative screens keep identity
  and command context at both required evidence sizes.

---

## Prince.4 - Web design pass and responsive polish

Ted Lindsay has established route truth for the major web surfaces. Make the
web feel like a finished
local hockey dashboard:

- active context visible above the fold;
- nav and page titles reflect hockey tasks, not crate internals;
- tables/cards use restrained density, real hierarchy, and readable spacing;
- mobile layouts avoid horizontal chaos except where tables intentionally scroll;
- filter/sort chips are visible and bookmarkable;
- empty/error states provide recovery actions;
- page-to-page style is consistent.

Acceptance:

- Broadcast checklist passes for major routes.
- HTML tests assert active context and visible applied filters.
- Browser screenshots at desktop and mobile are reviewed against ASPECT.

---

## Prince.5 - CLI table polish and export readability

Bring CLI output up to the same standard:

- stable columns at common widths;
- right-aligned numeric columns;
- compact labels that do not wrap badly;
- source/staleness footers where relevant;
- JSON/CSV remain clean and scriptable;
- markdown export reads well in GitHub and docs.

Acceptance:

- L2 command snapshots cover representative 80-column output.
- JSON/CSV output is unaffected by decorative terminal styling.

---

## Prince.6 - Golden/screenshot tests and docs closeout

Update:

- `design/specs/visual-system.md`
- `COMMANDS.md`
- `README.md`
- `design/IceLines.md`
- `design/plans/INDEX.md`
- `CHANGELOG.md`

Verification:

```bash
cargo fmt --check
cargo test -p icelines-cli
cargo test -p icelines-web
```

Add screenshot/manual review commands once the app's local serve/TUI harness is
confirmed.

Acceptance:

- Visual improvements are protected by tests where practical.
- Known subjective tradeoffs are recorded, not left as taste arguments.
- The next release has a credible claim: IceLines is not merely powerful; it is
  pleasant and hockey-native to use.

---

## Out of scope

- New analytics.
- Public hosting, accounts, auth, TLS.
- Full SPA rewrite.
- Rebranding/logo package.
- Replacing ratatui or axum.
