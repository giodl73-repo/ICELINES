# Mission

## Scope

Repo or feature: `icelines` — a single-binary NHL analytics, depth-chart,
fantasy, and live-game platform with three **active** rendering surfaces (CLI,
TUI, Web) over one shared engine, plus a fourth **deferred** surface — the
mkdocs static site (`icelines-site`), whose CLI entry was cut 2026-05-04 (see
Operating Context).

VTRACE adoption scope: **Repo baseline.** First VTRACE pass over IceLines. It
records the orienting intent for the platform as it actually exists today — the
**workbench** model (Phase Jack Adams), the Art Ross query engine, the ViewModel
layer, the six platform contracts, the `DataStore`/`StatsRepository`/bundle data
spine — and lays the traceability spine that connects that intent to code,
verification, and validation. This mission **supersedes the stale surface
framing** in `../IceLines.md` (written 2026-05-01, pre-workbench, "four
surfaces / 28 commands / 7 tabs / v1.0") and reorients around the realized
destination: a command-driven workbench whose main screen is normally a query.

## Mission Need

A serious NHL follower has no single tool that is both as **complete as the
major sites** (NHL.com, Hockey-Reference) and **customizable to the exact stats,
cohorts, and streaks they care about** — runnable offline, scriptable, and fast
to drive. League apps are shallow and read-only; reference sites are deep but
fixed and un-scriptable; spreadsheets are flexible but inert.

IceLines exists to be that tool: a **command-driven workbench** that matches the
big sites on coverage but lets the operator decide what to track. The main
workspace is normally a query (`LeadersView` and kin); an activity/catalog rail
selects the workspace; a scores ribbon rides the top as a live splash; and
left/right panes carry standing context (favorites, schedule, watchlist) with
sensible per-workspace defaults the operator can override and, at the state of
the art, **name, save, and restore**.

The core loop is **ask → see → reshape → act → repeat**, and one loop matters
enough to call out by name: the **stat-in-perspective post**. The operator spots
an interesting stat or streak, queries it *with historical context* ("is this
the longest such streak since …? where does it rank?"), and shares it. That
workflow — a precise historical query whose answer carries its own ranking and
context — is a first-class mission target, not a side feature. Here "perspective"
means **historical ranking, not era-adjusted**: IceLines ranks the streak
against history; it does not normalize for era/pace effects (a PACE
A-assumption limit), and it does not pretend to.

The need VTRACE serves is **not** to build the experience from scratch (most of
it exists). It is to (1) write this destination down as authoritative intent now
that the build has outrun the old app plan, (2) hold every surface (and every
shared-engine renderer) to the six platform contracts, and (3) make the trace
matrix expose — without shame —
where "as complete as the big sites, fully customizable, fully at parity" is
`partial`, `deferred`, or merely `verify` rather than `done`.

## Core Values

- **Competitive coverage, personal customization.** Match the major sites on
  what can be answered; beat them on letting the operator choose what to track,
  how to slice it, and how the workspace is laid out. "Competitive" is defined by
  a **named coverage checklist** (see Success Criteria) — it means *domain
  coverage* (the questions we can answer), **not numeric agreement** with
  third-party sources, which legitimately differ. Domains intentionally out of
  scope (NHL Edge skating speed; deployment-quality "value" beyond descriptive
  pace) are **conceded** in Non-Goals, not silently claimed.
- **Trustworthy data above all.** A wrong-but-confident number is the worst
  failure mode. Everything rests on the post-Hart canonical model —
  `(player_id, season, season_type)` read through `PlayerView` — so every result
  carries season/type, completeness, provenance (bundled → installed snapshot →
  live write-path), and freshness; missing data renders as typed `source_state`,
  never as silent zeroes, and an upstream **schema drift fails loud**
  (`deny_unknown_fields`) rather than silently misreading. (Platform Contract 1;
  Jennings "stabilization truth.")
- **State of the art on every surface, tuned to its user.** The **Web** dashboard
  is welcoming for a brand-new user; the **TUI** workbench is fast and dense for
  a power user (as fluid as k9s / lazygit / zellij); the **reports** are
  publication-grade for a stats blogger; **data management** makes boxscore- and
  shift-level tracking easy for anyone who wants depth.
- **Standalone and dependency-minimal.** IceLines builds and runs on its own
  with no cross-repo coupling: the portfolio integrations (FLETCH source
  orchestration, SLICE selectors) are **removed**, and the rendering surfaces
  (web, TUI) plus the network client are **feature-gated** so a lean,
  offline, CLI-only binary can be built with `--no-default-features`. The crate
  DAG holds — a lean build still compiles the pure `icelines-core` +
  `icelines-query`. Note (for `DESIGN.md`): `send-sync` is a **workspace-additive**
  feature, so enabling `web` flips `StatsRepository`'s `!Send` marker globally —
  a real feature interaction the gating design must own.

## Users

| User | Primary surface | Need | Success Signal |
|---|---|---|---|
| Power user / analyst | **TUI workbench** | Drive fast by command bar: precise filters (`pos=LW age<=25 AND g.last10g>=5`), time-travel 38 seasons, reshape panes, compare, read cross-league arcs | Reshapes the workbench and answers a non-trivial query in seconds without reaching for a mouse or a manual |
| Stats blogger / social poster | **Reports + query** | Find a stat or streak and query it **in historical perspective**, then export a clean, citable artifact to post | `query`/`streaks`/`records` answer a historical-streak question with ranking/context, and `report`/`export md`/`--json` produce a publication-grade, reproducible artifact |
| New / casual user | **Web dashboard** | Open `/dashboard`, browse leaders, scores, a player card, a bracket — no CLI, no setup | A first-time visitor navigates the workbench, finds a player and tonight's slate, and bookmarks a view via URL state |
| Fantasy manager | TUI + Web + CLI | Roster gaps, add/drop simulation, poach targets, daily/weekly matchup deltas — offline, repeatable | Runs `:poach …` / `fantasy simulate …`, reshapes to a fantasy layout, and acts in-tool |
| Data tracker | **Data management** | Install seasons and pull **boxscore- and (eventually) shift-level** detail without ceremony | `data` / `fetch` / `snapshot` make depth data easy to acquire, verify, and trust; the tool is honest about what shift data is and isn't available |
| Future agent / maintainer | n/a | Resume from intent; know crate ownership; know built vs verified vs validated | Reads `docs/vtrace/` + the trace matrix and states each capability's status against the surface-parity matrix |

## Operating Context

- **Delivery**: one statically-bundled Rust binary (`icelines`), ~56 MB, 38
  seasons embedded via `include_bytes!()`. Runs fully offline out of the box;
  fresh data is opt-in via `icelines fetch`.
- **Surfaces**: CLI (clap, 40+ commands), TUI (ratatui — default **workbench**
  MDI + `--classic` tabbed + `--standalone` single-screen), Web (axum HTML
  dashboard at `/dashboard` + versioned JSON APIs).
- **Fourth surface (deferred, not dropped)**: the mkdocs static site
  (`icelines-site`) still builds, but its CLI entry (`build`/`serve`/`deploy`)
  was cut 2026-05-04. Durable **exports** (`export md`, `--json`/`--csv`) remain
  first-class report artifacts. The surface-parity matrix tracks site/export
  rows; this mission scopes the static site as deferred and says so rather than
  pretending IceLines was only ever three surfaces.
- **Shared engine**: `icelines-core` (types, ViewModels, workbench catalog),
  `icelines-query` (Art Ross grammar/IR/executor), `icelines-fetch` (I/O, cache,
  bundle). `icelines-cli` is a thin UI layer; lower crates cannot import higher.
- **Dependency posture**: no cross-repo dependencies (the FLETCH/SLICE seams are
  being removed); crates.io dependencies are essential per surface and gated by
  `web` / `tui` / `net` Cargo features (default = all on). `icelines-core` stays
  dependency-light (no async runtime, no network, no I/O).
- **State**: `~/.icelines/` — `snapshots/`, `data/manifest/`, `config.toml`,
  `icelines.db` (SQLite: groups, favorites, fantasy, event stream), `headshots/`,
  `career_history.json`.
- **Data depth**: bios/stats/boxscores/play-by-play/transactions/career-history
  are reachable; **shift-level data is gated** — the Foster capability matrix
  enforces `shifts=off` (locked), so shift tracking is a stated target with a
  known constraint, not a shipped capability.
- **External dependency**: the public NHL API (`/v1/...`) for live/fresh data;
  optional MoneyPuck for advanced stats. Both are siloed and degrade to bundled
  data when unavailable.
- **Governing artifacts**: the six **platform contracts**
  (`../specs/platform-contracts.md`), the **surface-parity matrix**
  (`../specs/surface-parity.md`, the per-feature status source of truth), and the
  **ViewModel** contract (`../specs/viewmodels.md`).

## Constraints

- **Shared-engine parity** (Contract 4): any capability exposed through more than
  one renderer — CLI, TUI, Web, **and** the export/static-site artifacts that
  lower the same ViewModel (e.g. `export md` rendering `LeadersView`) — must
  produce the same answer. Parity is "every renderer of a ViewModel agrees," not
  "the three active surfaces agree." No doc may advertise a feature as shipped
  without a `done`/qualified-`partial` matrix row.
- **One typed query intent** (Contract 2): CLI args, TUI cmdbar, web params, and
  AI-fallback all validate through the deterministic `icelines-query`
  parser/planner; screen shortcuts must reduce to the same typed filter/sort.
- **One data model** (Contract 1): reads route `DataStore` (manifest → bundle →
  lazy fetch) → `StatsRepository` (session LRU, cap 80). `DataStore` stays
  stateless; caching lives only in `StatsRepository` (FORGE H3). Missing data is
  typed `source_state`, never silent zeroes.
- **ViewModels carry hockey semantics, not pixels** (Contract 3): renderers
  choose layout/styling but never recompute ranking, filtering, source state, or
  classification; ViewModels stay serializable for fixtures.
- **Reports are ViewModel-backed and reproducible** (Contract 5) for a fixed
  fixture and clock.
- **Shared semantic visual tokens** (Contract 6): color never carries meaning
  alone; TUI glyphs have ASCII fallback; web state is URL-bookmarkable.
- **Active context always visible** (GLASS): every surface shows the active
  `(season, season_type)`. Silent time-travel (`y` / `?season=`) is a defect —
  an ambiguous season makes every fit/stat on the screen untrustworthy.
- **Frozen interfaces**: JSON envelopes are `v1`, additive-only; the `--filter`
  grammar, CLI flags, and config keys are user-facing contracts.
- **Standalone build**: the repo must build with no cross-repo path/git
  dependencies, and a `--no-default-features` build (no `web`/`tui`/`net`) must
  produce a working offline CLI from bundled data.
- **Code rigor**: `cargo fmt --check`, `cargo clippy -- -D warnings`, and the
  L0/L1/L2 tiers stay green; new features carry L0 tests, new commands carry L2.
- **Identity**: commits are authored as the personal `giodl73-repo` identity,
  never the work account (repo `CLAUDE.md`).

## Non-Goals

- Not a cloud, multi-user, or hosted service; single-operator local tool, no
  auth, all state under `~/.icelines/`.
- Not real-time push; live data is poll-based (~30s on Scores).
- Not predictive/betting; pace projections are descriptive only.
- Not a declarative TOML dashboard generator — `dashboard-engine.md` and the
  proof/DASHBOARD-SPEC integration were **Cancelled** 2026-05-01; the workbench
  is the realized model.
- Does not bundle NHL Edge skating-speed stats (no public JSON endpoint).
- Does not model cap/contract value beyond expiry-year + expiry-type.
- Does not claim parity on every major-site feature: shot-location/heatmaps,
  Edge speed, and predictive "value over replacement" are **conceded out of
  scope** (SCOUT). Pace is descriptive; it does not normalize deployment quality
  or linemate effects, and the product does not pretend it does.
- VTRACE adoption documents and traces the architecture; it does not rewrite it
  and does not assert compliance — honest gaps are recorded, not papered over.

## Success Criteria

Criteria tagged **[target]** are **not met today** — they are tracked as gaps in
`REQUIREMENTS.md` / `TRACE.md`, not present-tense facts. Untagged criteria
describe current expected behavior to be confirmed by verification.

| Criterion | Validation Method | Evidence Pointer |
|---|---|---|
| Coverage matches a **named checklist** of domains — leaders, players, teams, goalies, schedule, scores, playoffs, transactions, career, fantasy — with conceded exclusions (Edge speed, shot-location, predictive value) listed, not implied | Analysis + demonstration | surface-parity matrix rows marked `done`; conceded exclusions in Non-Goals; gaps named as `partial`/`planned` |
| A historical streak/stat can be queried **in perspective** (the answer carries ranking/context) and exported for posting | Demonstration + test | Art Ross `.streak` / `EVER` / `seasons-with` atoms; `PlayerStreaksView` / `PlayerRecordsView`; `report` / `export md` / `--json` |
| Operator launches the workbench, swaps the main workspace to a query, and reshapes left/right panes via one command grammar on TUI and Web | Demonstration (CONOPS) | `WORKBENCH_CATALOG` / `WORKBENCH_PANE_BINDINGS` / `WORKBENCH_EXPERIENCES`; `:` cmdbar; `/dashboard?workspace=…&left=…&right=…` |
| **[target]** Custom layouts can be **named, saved, restored on launch, and shared** (state of the art: Zellij/tmux named layouts, VS Code workbench restore, Grafana saved dashboards) | Demonstration + inspection | Built today: `WORKBENCH_PANE_BINDINGS`, bound experiences, web URL state. **Target with gap**: durable named layouts persisted under `~/.icelines/` and round-tripped across TUI/Web — tracked in `REQUIREMENTS.md` / `TRACE.md` |
| The same filter query returns identical results on CLI, TUI, and Web | Test + analysis (parity) | shared `icelines-query` + `/api/v1/leaders` twin; fences like `l2_query_goalies_cli_and_web_row_identity_match` |
| Every result carries trustworthy context: season/type, completeness, provenance, freshness; missing data is typed state, never silent zeroes | Test + inspection | Contract 1; `ViewContext`/`source_state`; `LoadOutcome.missing` survives to render |
| Per-surface quality bars are met: Web friendly for a new user, TUI fast for a power user, reports publication-grade for a blogger, data mgmt easy for boxscore/shift tracking | Demonstration (per-persona scenario) | First Validation Scenarios (below); `VALIDATION.md` |
| **Offline by default**: the default workspace renders from bundled data with zero network calls | Test | bundled-data tests; no live calls in L1/L2 |
| **Interactive latency (target, to be measured)**: a query / player-card open returns in interactive time on bundled data | Demonstration + analysis | `StatsRepository` LRU cap 80; TUI poll cap is 100ms; the "~50ms career fan-out (UX.1)" figure is **claimed/unverified** until benchmarked — a target, not a fact |
| **Reproducible output**: a report/JSON is byte-stable for a fixed fixture and clock | Test | Contract 5; `MockClock`; envelope `v1` |
| **[target] Standalone, no cross-repo deps**: builds with no FLETCH/SLICE path/git dependency | Test + inspection | *Not met today* (FLETCH/SLICE still in tree). FLETCH/SLICE removal; `Cargo.toml` has no `path = "../..."` / cross-repo `git` deps |
| **[target] Lean build**: `--no-default-features --features cli` yields a working offline binary without web/TUI/network crates | Test | *Not met today* — `icelines-cli` hard-depends on `icelines-web` + `ratatui` + `axum`; needs `web`/`tui`/`net` Cargo features + Cargo.toml surgery; offline smoke test |
| Build, format, lint, and L0/L1/L2 suites pass | Test + static check | `cargo build/test`, `cargo clippy -- -D warnings`, `cargo fmt --check` — results in `VERIFICATION.md` / `REVIEW.md` |
| Each surface row in the parity matrix is honestly `done`/`partial`/`deferred`, never advertised beyond status | Inspection + review | `../specs/surface-parity.md` carried into `TRACE.md` / `REVIEW.md` |

> **Verifiability note (BENCH):** criteria validated by *demonstration* or
> *inspection* only — competitive coverage, per-surface quality bars,
> customization — are **not regression-testable**. `VERIFICATION.md` and
> `TRACE.md` convert what they can into automated fences (parity, offline,
> reproducibility, build/lint) and mark the rest explicitly as demo/inspection
> evidence, not as passing tests.

## First Validation Scenarios

These are the seed acceptance demos for `VALIDATION.md`, one per primary user:

1. **Power-user reshape (TUI).** Launch `icelines` (workbench). Swap the
   workspace to Stats, apply `pos=C age<=23 AND p.last20g>=20`, change the
   left/right panes, time-travel to a prior season, and read a player card —
   all by keyboard/cmdbar, in interactive time.
2. **Stat-in-perspective post (Reports).** From an observation, run a historical
   streak/record query that returns the streak **with its ranking/context** —
   correctly **skipping the 2004-05 lockout** (no season), honoring the October
   `CURRENT_SEASON` rollover, resolving name collisions (the Sebastian Aho
   problem) and streak continuity across a mid-season trade (`team_stints`), and
   **disclosing data completeness** over the range (only ~5 modern seasons carry
   full Tier-1 detail; the other 33 bundled seasons are skeleton) — so a "longest
   since 1987-88" claim is both right *and* honest — then `export md` / `--json`
   a clean, reproducible artifact suitable for a tweet or blog post.
3. **New-user browse (Web).** Open `/dashboard` cold. With no CLI knowledge,
   find league leaders, tonight's scores, a player card, and the playoff bracket;
   bookmark a workspace+panes view via URL.
4. **Cross-surface parity.** Run one non-trivial filter on CLI, `tui stats`, and
   `/leaders`; confirm identical rows and identical completeness/source warnings.
5. **Offline cold launch.** On a machine with no network, the default workspace
   and a historical-season query both succeed from bundled data.
6. **Data-depth acquisition.** A user installs a season and loads boxscore-level
   detail for favorites with simple commands; the tool is explicit about what
   shift-level data is and isn't available.

## Principal Risks to the Mission

| Risk | Why it threatens the mission | Where addressed |
|---|---|---|
| **Surface drift** — a feature ships on one surface and silently diverges on another | Breaks the "same answer everywhere" promise and the trust value | Contract 4 + surface-parity matrix; verified in `VERIFICATION.md` |
| **Data staleness / wrong-but-confident output** | Directly violates the trustworthy-data core value; worst failure mode for a stats poster | Contract 1 `source_state`/freshness; validated in `VALIDATION.md` |
| **TUI complexity** — the ~3,800-line `App` god-object (mid-refactor: Norris/Masterton) | Slows the power-user surface and raises regression risk | Tracked in `ARCHITECTURE.md` risks + `DESIGN.md` |
| **Customization gap** — no durable named layouts yet | The headline "customizable to me" promise is only partially met | `REQUIREMENTS.md` target + `TRACE.md`/`REVIEW.md` finding |
| **Shift-data expectation gap** — `shifts=off` is locked | "Easy shift-level tracking" could over-promise | Stated as target-with-constraint here + `INTERFACES.md` |
| **Web mutation parity deferred** — many admin/fantasy writes are CLI-only | New-user web surface can feel read-only for some workflows | surface-parity `partial`/`deferred` rows; `REVIEW.md` accepted risks |
| **Historical-perspective correctness** (EDGE) — the headline stat-in-perspective workflow crosses the 2004-05 lockout gap and the October season rollover | A wrong "longest streak since …" posted publicly is the worst failure mode for the social-poster persona | `design/PITFALLS.md`; First Validation Scenario 2; validated in `VALIDATION.md` |
| **Historical data-depth asymmetry** (EDGE) — only ~5 modern seasons carry full Tier-1 detail; 33 bundled seasons are skeleton (`MODERN_BUNDLED_SEASONS` vs `BUNDLED_SEASONS`) | A 38-season "perspective" claim may be computed over partial data yet read as authoritative | Perspective answers must disclose completeness (Contract 1); Scenario 2; `VALIDATION.md` |
| **Dependency elimination is a feature removal** — dropping FLETCH/SLICE deletes the `fetch sources\|partitions\|quivers` and SLICE-selector surfaces, not just deps | Could break dependents, tests, or docs if done blind | Specified in `REQUIREMENTS.md` (REQ-DEP) + `DESIGN.md` (what's removed, rollback) before implementation; verified by `cargo build/test` |

## Source Links

- Prior app plan (superseded surface framing): `../IceLines.md`.
- System architecture: `../ARCHITECTURE.md`; invariants: `../INVARIANTS.md`.
- Platform contracts: `../specs/platform-contracts.md`.
- Surface-parity matrix (status source of truth): `../specs/surface-parity.md`.
- ViewModel contract: `../specs/viewmodels.md`.
- Workbench: `../specs/phase-jack-adams-overview.md`,
  `../plans/2026-05-08-phaseJackAdams-mdi-dashboard.md`,
  `../plans/2026-05-12-phaseJackAdamsWeb-web-mdi.md`.
- Query engine + streak/EVER atoms: `../specs/phase-art-ross-overview.md`,
  `../specs/query-engine.md`.
- Streaks/records/events: `../specs/event-stream-payloads.md` (and the
  `records`/`streaks`/`awards` surfaces).
- Data spine + depth: `../specs/cache-model.md`, `../specs/data-bundles.md`,
  `../specs/data-sources.md`, `../specs/foster-data-architecture.md`,
  `../specs/snapshot-operations.md`.
- Spec index + status legend: `../specs/INDEX.md`.
- Repo context: `../../CLAUDE.md`, `../../SPEC.md`, `../../CODEBASE.md`,
  `../../COMMANDS.md`, `../../README.md`.
- VTRACE process: `repos/standards-protocols/vtrace/docs/framework/vtrace-process.md`.
- Companion VTRACE artifacts (this adoption): `CONOPS.md`, `REQUIREMENTS.md`,
  `ARCHITECTURE.md`, `INTERFACES.md`, `DESIGN.md`, `CODE_RIGOR.md`,
  `VERIFICATION.md`, `VALIDATION.md`, `TRACE.md`, `REVIEW.md`.
