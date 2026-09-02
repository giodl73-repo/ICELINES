# UI-Neutral Card System — Specification

**Version**: 1.0
**Date**: 2026-07-22
**Status**: Implemented
**Owner domain**: cross-surface decision artifacts
**Parent contracts**: [`platform-contracts.md`](platform-contracts.md),
[`viewmodels.md`](viewmodels.md), and [`visual-system.md`](visual-system.md)
**Brand vocabulary**: [`brand-the-rink.md`](brand-the-rink.md)

## Purpose

IceLines needs one canonical way to package a hockey decision so CLI, TUI,
web, JSON, images, PDF, and third-party renderers can present the same meaning
without copying hockey logic.

The card system is a typed ViewModel boundary, not an image template and not a
generic dashboard engine. IceLines resolves identity, data authority, scores,
line assignments, recommendations, simulations, uncertainty, evidence, and
warnings. Renderers choose typography, spacing, responsive layout, terminal
density, and output format.

The first acceptance showcase is a two-page 2026-27 team card for the New York
Rangers and Seattle Kraken:

1. **The Depth Chart** — faces, names, one comparable IceLines score, and
   lineup position.
2. **The Insider** — baseline prognosis, internal ceiling, breakout path,
   isolated team impact, downside, and methodology.

The same system carries implemented fantasy roster, draft, morning, trade,
season-simulation, forecast-movement, and forecast-history cards.

## Architectural Rule

```text
authoritative IceLines inputs
            |
            v
domain builder in icelines-core
            |
            v
CardDocumentView (versioned JSON contract)
            |
    +-------+-------+--------+---------+
    |       |       |        |         |
   CLI     TUI     Web     Image      PDF/other
```

For identical inputs, every renderer consumes the identical serialized card
document. A renderer may omit a visual asset it cannot display, but it may not
change ordering, values, classifications, recommendations, evidence state, or
warnings.

If two renderers disagree on hockey meaning, the defect is in IceLines or the
contract. It is not fixed with renderer-local arithmetic.

The card system also must not create a parallel analytics store. Roster,
schedule, player, injury, transaction, fantasy, and simulation records enter
the same mainline IceLines data streams, retain their native source authority,
and join through stable league, season, team, game, and player identities. A
card builder reads those joined domain views. It does not copy fantasy or
simulation output into presentation-only records.

```text
official roster + schedule + stats + injuries + transactions
                           |
                           v
             shared identities and evidence graph
                    /                    \
          fantasy decisions        league simulation
                    \                    /
                     joined domain views
                           |
                           v
                   card document(s)
```

This integration is what makes side-by-side presentation reliable. Comparing
two teams, scenarios, players, or fantasy options selects aligned documents
from the same evidence graph and declares the comparison dimensions. It never
scrapes values back out of rendered cards.

## Scope and Non-Scope

### In scope

- a stable card envelope and typed semantic sections;
- domain-specific IceLines builders;
- deterministic JSON with schema and methodology versions;
- CLI, TUI, and web renderers over one document;
- source-stamped assets and explicit missing-asset behavior;
- semantic theme and state tokens;
- accessibility labels and plain-text fallbacks;
- reproducible scenario references and fingerprints;
- contract fixtures, renderer parity tests, and visual review artifacts;
- future fantasy and simulation card kinds.
- aligned side-by-side comparison sets over compatible card documents.

### Not in scope

- a free-form drag-and-drop dashboard language;
- renderer scripts that query NHL or fantasy sources directly;
- HTML, CSS, terminal widths, pixels, or font names inside core ViewModels;
- AI-generated player faces or invented official branding;
- renderer-specific score, ranking, or scenario calculations;
- autonomous mutations on NHL or fantasy platforms;
- treating a generated raster image as the durable source artifact.

## Contract Layers

The system has three explicit layers.

### 1. Domain inputs

Existing typed IceLines inputs remain authoritative: roster/depth views,
schedule and forecast views, fantasy league rules, scoring schemes, scenario
events, source states, and official asset records.

### 2. Domain card builders

Each card kind has one builder in `icelines-core`. Examples:

- `build_team_prognosis_card`;
- `build_season_simulation_card`;
- `build_forecast_movement_card`;
- `build_fantasy_roster_card`;
- `build_fantasy_draft_card`;
- `build_fantasy_morning_card`; and
- `build_fantasy_trade_card`.

Builders select and order evidence, calculate deltas, validate invariants, and
produce complete semantic sections. This work never occurs in a renderer.

### 3. UI-neutral document

All builders emit `CardDocumentView`. Its pages contain typed section variants
from a deliberately small visual grammar. Card kinds may restrict which
sections are legal and add typed domain data without weakening the envelope.

## Schema Family

Initial public schemas:

The checked shared-envelope artifact is
[`card_document.v1.schema.json`](../schemas/card_document.v1.schema.json).

| Schema | Purpose |
|---|---|
| `card_document.v1` | shared envelope, pages, sections, assets, context |
| `team_prognosis_card.v1` | depth chart plus team forecast/ceiling |
| `season_simulation_card.v1` | standings distribution and scenario paths |
| `forecast_movement_card.v1` | later-minus-earlier checkpoint movement with two sealed sources |
| `team_game_prediction_edge_card.v1` | one matchup's sealed vintage movement and factor attribution |
| `fantasy_roster_card.v1` | legal roster shape, lineup, gaps, schedule fit |
| `fantasy_draft_card.v1` | draft state, needs, best available, tier cliffs |
| `fantasy_morning_card.v1` | injuries, starts, locks, pickups, daily actions |
| `fantasy_trade_card.v1` | before/after rosters, value, needs, counters |
| `card_comparison_set.v1` | aligned documents plus declared comparison keys |

`card_document.v1` is not a promise that every renderer supports every future
section. Capability negotiation is explicit, and unsupported required sections
are an error rather than silently disappearing.

### Shared evidence joins

All domain builders consume a common identity and provenance boundary:

- `league_id`, `season_id`, and `season_type` establish the competition;
- stable `team_id`, `player_id`, and `game_id` fields resolve aliases;
- `roster_snapshot_id` and `evidence_at` bind membership to a point in time;
- `calendar_fingerprint` binds weekly fantasy and season simulations to the
  same schedule;
- `scoring_scheme_id` changes fantasy value without changing player identity;
- `scenario_id`, model version, seed, and trials bind simulated claims;
- `scenario_comparison_key` explicitly identifies comparable scenario families
  across team-specific scenario IDs, and renderers never infer it from
  prefixes; and
- every injury, waiver, pickup, and trade event retains effective and observed
  timestamps.

Fantasy and simulation may add derived views, but may not fork identity,
schedule, roster, injury, or transaction truth. Historical replay uses the
same joins with a historical evidence cutoff so future knowledge cannot leak
backward.

### Side-by-side comparison

`card_comparison_set.v1` is a thin UI-neutral wrapper around complete card
documents. It supplies:

- a stable comparison ID and ordered document IDs;
- compatibility keys such as schema, season, model, scoring scheme, evidence
  cutoff, and scenario family;
- declared aligned metric and section keys;
- typed incompatibility warnings; and
- a comparison fingerprint derived from the child document fingerprints.

Renderers may place the children beside one another, stack them, or offer a
page switcher. Deltas and rankings come from the domain comparison builder,
never from renderer subtraction. A Rangers/Kraken comparison therefore stays
meaningfully identical in web, TUI, image, PDF, and JSON output.

## Shared Envelope

Conceptual Rust shape:

```rust
pub struct CardDocumentView {
    pub schema: String,
    pub card_kind: CardKind,
    pub document_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub context: CardContextView,
    pub theme: CardThemeView,
    pub pages: Vec<CardPageView>,
    pub provenance: Vec<CardProvenanceView>,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}
```

### `CardContextView`

Required fields:

- season and season type when applicable;
- generated-at and evidence-as-of timestamps;
- source completeness and data-generation token;
- card-builder and methodology versions;
- parameter, scoring-scheme, scenario, and roster fingerprints when relevant;
- seed and trials for simulated results;
- locale and timezone used for date boundaries; and
- evidence labels: confirmed, reported, estimated, simulated, under review, or
  no read.

The evidence cutoff and generation time are different fields. Import time must
never be presented as the observation time of historical evidence.

### `CardThemeView`

Theme data expresses semantic roles, not renderer layout:

- `theme_key`, such as `team_nyr`;
- `primary`, `secondary`, `accent`, `surface`, and `text` color tokens;
- minimum contrast metadata for text roles;
- optional team abbreviation and wordmark text;
- ASCII-safe fallback identity; and
- provenance for any official visual asset.

Web and image renderers may use full color values. TUI maps roles to the nearest
supported palette. No meaning depends on color alone.

Team identity is core-owned. `nhl_team_card_theme` supplies the canonical
palette for every active NHL abbreviation, and all team-oriented card builders
consume that one primitive. Renderers must not maintain a card-specific team
palette or substitute a generic theme for a canonical team. Unknown identities
remain explicit and receive the neutral fallback.

### `CardPageView`

Required fields:

- stable page ID;
- literal purpose label;
- optional brand display label;
- order;
- accessible summary; and
- ordered sections.

Page IDs remain stable across renderers. Pagination is semantic: a narrow TUI
may switch pages while a wide web page may show them beside one another.

## Semantic Section Grammar

The initial grammar is intentionally bounded:

| Section | Meaning |
|---|---|
| `identity_header` | team, player, league, matchup, or scenario identity |
| `metric_strip` | ordered headline metrics with units and display policy |
| `lineup` | forwards, defense pairs, goalies, fantasy slots, or bench |
| `player_list` | ranked or grouped players with typed metrics |
| `scenario_bridge` | baseline-to-scenario or baseline-to-ceiling change |
| `probability_range` | distribution, percentile range, or outcome bands |
| `decision` | recommendation plus supporting evidence and alternatives |
| `timeline` | dated games, events, locks, waivers, injuries, or trades |
| `state_notice` | missing, partial, stale, unavailable, or blocked state |
| `methodology` | compact model/scoring explanation |
| `provenance` | source and freshness details |

Each variant has typed fields. There is no renderer-authored arbitrary HTML or
terminal markup in the document.

### Metrics

Every metric contains:

- stable key;
- literal label;
- raw numeric or typed value;
- canonical unit;
- precision and missing-value policy;
- preformatted accessible display text;
- optional comparison baseline and signed delta;
- semantic token; and
- evidence state.

Renderers may use the preformatted display or a locale-aware formatting helper
that obeys the supplied unit and precision. They may not choose a different
rounding policy.

### Assets

Player and team assets use `CardAssetView`:

- stable asset ID and subject identity;
- kind: headshot, team mark, or generated chart artifact;
- canonical URL or local content reference;
- source and observation time;
- integrity hash when locally persisted;
- accessible alternative text;
- state: available, missing, stale, blocked, or invalid; and
- fallback: initials, abbreviation, or none.

Player faces must come from official or explicitly accepted IceLines source
records. Missing faces use deterministic initials or silhouettes. Renderers
must never generate or substitute a person's likeness.

## Team Prognosis Card

### Inputs

- season calendar and team identity;
- authoritative or explicitly estimated roster;
- projected line/pair/goalie assignments and assignment evidence;
- `TeamDepthView`/team-ceiling player records;
- IceCast season forecast and scenario impacts;
- development calibration and named scenario events;
- isolated paired simulations for each highlighted event;
- official headshot records; and
- shared semantic tokens and team theme.

### Page 1 — `depth_chart`

Display label: **The Depth Chart**
Literal label: **Projected team lineup and IceLines player scores**

The page contains:

- team identity and one team score;
- four forward lines when the roster supports them;
- three defense pairs;
- starter and backup goalie roles;
- extras/unplaced players in an explicit secondary group;
- one official headshot, display name, and whole-number IceLines score per
  player; and
- an unobtrusive token identifying Page 2 breakout candidates.

No forecast odds, breakout probabilities, projected scoring totals, or prose
belong on this page. Its job is recognition and roster comprehension.

#### Player score authority

The displayed number is `IceLines Player Score / 100`, not NHL points, fantasy
points, or team-strength delta. It must:

- use a versioned multi-lens methodology;
- be position-aware;
- use the appropriate goalie methodology for goalies;
- carry coverage and source state;
- round once in core to a whole-number display value; and
- expose its component/model version in Page 2 methodology.

If a player lacks enough evidence for a responsible score, the score is null
and the displayed value is `NR`, never zero.

`icelines_player_score.v1` uses the existing team-ceiling lens reads and keeps
their raw values in the document. Forward normalization ceilings are 120
points/82, 60 goals/82, 450 fantasy points/82, and 120 upside units, weighted
40/20/25/15. Defense ceilings are 90, 30, 400, and 100, weighted 30/10/40/20.
Goalies use the existing position-specific 0-100 goalie-quality read directly.
Available component weights are renormalized, but a qualifying NHL sample and
the primary points/goalie read are mandatory. Core stores the one-decimal value
and whole-number display text alongside sample games, component coverage, and
evidence; renderers use the supplied display text unchanged.

#### Assignment authority

Every slot carries `actual`, `reported`, `estimated`, or `scenario` evidence.
The page title or context band states the overall lineup authority. A renderer
may visually de-emphasize the label but may not hide it.

`team_lineup_projection.v1` is the renderer-neutral Page 1 source: four typed
forward lines, three defense pairs, starter/backup goalies, extras, canonical
headshot references, deterministic initials, scores, eligibility, and shape
warnings. Current roster team controls membership; prior team is provenance
only. Explicit actual/reported/scenario assignments are placed first and all
remaining openings are filled deterministically as estimated assignments.

### Page 2 — `insider`

Display label: **The Insider**
Literal label: **Season prognosis, upside path, and risk analysis**

Required sections:

1. baseline projected points, percentile range, playoff probability, and Cup
   probability;
2. internal ceiling with the same four measures;
3. a baseline-to-ceiling bridge with explicit unit labels;
4. highlighted breakout candidates;
5. isolated paired impact for each candidate;
6. combined-event realization probabilities;
7. primary downside events and their isolated/combined impact;
8. methodology, trials, seed, scenario fingerprint, and as-of date; and
9. source warnings and non-claims.

#### Breakout rows

Every breakout row includes:

- player identity;
- current score and breakout score, or a labeled scouting prior;
- event occurrence probability and its provenance;
- raw team-strength delta with unit label;
- isolated expected standings-point delta;
- isolated playoff- and Cup-probability deltas;
- current role and breakout role label where supported; and
- evidence state.

`+3.47` alone is invalid presentation. It must be labeled `team strength` and
paired with the more intuitive isolated team outcome deltas.

#### Isolated impact authority

IceLines core computes each highlighted event with a paired, same-seed run:
the baseline disables every scenario event, and the conditional run forces
exactly one event while leaving schedule, seed, trials, and model parameters
unchanged. The document retains both the conditional absolute outcome and its
delta from baseline so consumers can verify reconciliation without rerunning
the simulation.

The natural scenario preserves authored occurrence probabilities and explicit
correlation keys. A separate forced ceiling includes positive-strength events
only. Renderers must consume the core-supplied raw sums and path labels; they
must not reconstruct correlations, event combinations, or rounded ceiling
labels.

#### The Path label

The combined upside label must distinguish:

- the sum of modeled team-strength events;
- the forced all-hit ceiling simulation;
- the probability distribution of naturally sampled event counts; and
- any explicit correlations.

Only core may round the raw sum into the display label. Under the current
nearest-whole-unit rule, `+15.4855` becomes `+15 Path`; a renderer may decorate
that text but cannot independently change it to `+16`. The document retains
the raw sum and labels the display as rounded team strength. It must not imply
the same number of standings points.

The all-hit ceiling is conditional, not a forecast. Its probability cannot be
created by multiplying marginal event probabilities in a renderer.

## Fantasy Card Kinds

Fantasy cards reuse the envelope and section grammar but use the active league
contract as authority.

### `fantasy_roster_card.v1`

- Page 1: 2 C, 2 LW, 2 RW, 3 D, UTIL, 2 G, four bench, 2 IR, and 2 IR+;
- Page 2: positional gaps, schedule collisions, complement classes, injuries,
  pickup budget, and best legal alternatives.

### `fantasy_draft_card.v1`

- current roster construction and open slots;
- taken-player and availability state;
- best available, safest fit, and highest-upside recommendation;
- multi-position flexibility, tier cliffs, schedule equivalence class, and
  playoff portfolio fit; and
- explicit recommendation deltas under the active scoring scheme.

### `fantasy_morning_card.v1`

- today's legal lineup;
- confirmed/estimated goalie starts;
- injury and IR/IR+ actions;
- remaining weekly pickup budget;
- waiver/free-agent availability timing; and
- prioritized actions with lock deadlines.

### `fantasy_trade_card.v1`

- both rosters before and after;
- scoring, lineup, matchup, schedule, and playoff deltas;
- needs helped and needs harmed;
- fairness range and counteroffers; and
- pending-offer freshness.

The implemented core projection accepts `FantasyTradeEvaluationView`, whose
contract is shared by the existing evaluator, finder, JSON, and card command.
The Trade Board preserves the recommendation, fairness gap, legality, and both
player packages. The Insider preserves each team's before/after value, value
and remaining-games deltas, roster capacity, and missing-slot changes.
`icelines fantasy trade-card` is read-only: it evaluates the requested package
through the existing trade path and seals the result without saving or
executing the offer.

Fantasy renderers never recalculate position legality, daily assignment,
waiver timing, pickup budget, score, or trade value.

### Implemented fantasy roster projection

`fantasy_roster_card.v1` is built by IceLines core from
`FantasyInjuryPlanView`, `FantasyDailyLineupView`, persisted assistant rules,
the weekly acquisition ledger, and `FantasyScheduleView`. It emits
`card_kind: fantasy_roster` inside the shared `card_document.v1` envelope.

The roster page preserves the configured 2 C / 2 LW / 2 RW / 3 D / UTIL / 2 G
active shape, four bench slots, two IR slots, and two IR+ slots. Rich bench
assignments retain stable player key, NHL team, multi-position eligibility,
projected value, game state, availability, and lock state; the legacy bench
name list remains wire-compatible. The context page carries scoring-scheme
identity, weekly limit/usage/remaining moves, same-day free-agent activation,
two-day waiver duration, usable starts, projected active value, and every
schedule equivalence class. Missing schedule authority is an explicit warning.

### Implemented fantasy draft projection

`fantasy_draft_card.v1` is built by IceLines core directly from
`FantasyDraftBoardView`. Page 1 preserves open starter slots, available/taken
counts, the next-pick recommendation and rationale, fallback, position leaders,
and ranked candidates. Candidate roles include NHL team, platform
multi-position eligibility, and the best open slot they fill.

Page 2 preserves the top pick's league-quality, starter-gap, scarcity,
flexibility, usable-start, quiet-slate, schedule-diversity, collision-cost,
playoff-fit, and risk components. Taken-player and eligibility-import match,
ambiguity, and unresolved counts remain visible. The CLI entry point is
`icelines fantasy draft-card`; it runs the existing draft-board pipeline before
sealing the card, so card renderers never rerank or refilter the player pool.

### Implemented fantasy morning projection

`fantasy_morning_card.v1` is built directly from
`FantasyMorningBriefingView`. Page 1 preserves the first recommended action,
ordered alternatives, conditional labels, and the complete legal daily lineup.
Page 2 preserves weekly acquisition usage and reserve, goalie refresh/safety/
lock timestamps, same-day starter evidence, ranked weekly moves, stale injury
evidence, warnings, methodology versions, and the source fingerprint.
`icelines fantasy morning-card` invokes the existing morning pipeline before
sealing the document; renderers do not reevaluate status, lineup legality,
goalie probability, or pickup value.

## Season Simulation Card

`season_simulation_card.v1` summarizes one IceCast/IceLab run:

- baseline and scenario standings distributions;
- best, median, and worst paths;
- scenario event journal and realization buckets;
- injuries, recoveries, trades, goalie events, and streaks;
- playoff/Cup deltas; and
- parameter, seed, trial, calendar, and replay authority.

The team prognosis card may project a focused subset from this view, but the
focused document retains fingerprints proving it came from the same league
simulation.

The implemented prospective fixture selects downside, middle, and upside
realization buckets without inventing new simulation output. The implemented
2024-25 rolling replay uses the same builder and adds confirmed actual W-L-OTL,
points, focused/league pick accuracy, Brier score, calibration error,
coin-flip skill, and best chronological Elo blend. Prospective metrics remain
simulated evidence; completed outcomes and calibration metrics are confirmed.

## Renderer Contracts

### CLI

- exports canonical JSON;
- offers compact text for inspection;
- uses stable 80-column fallbacks;
- never applies decorative output to JSON; and
- exits non-zero for unsupported required schema versions.

### Web

- serves the card document from a versioned JSON route;
- renders accessible HTML from the same in-memory document;
- supports page/depth selection in bookmarkable state;
- uses scenario IDs from a registry rather than arbitrary web file paths;
- preserves warnings above conclusions; and
- provides desktop and mobile layouts without changing content meaning.

### TUI

- renders Page 1 as a compact lineup board;
- toggles between The Depth Chart and The Insider;
- uses initials or omits images when terminal capability is insufficient;
- preserves order, metrics, evidence labels, and warnings;
- collapses density at 80/120/160 columns; and
- never mutates ViewModel rows during render.

### Image/PDF/reference renderer

- accepts only the JSON document plus renderer configuration;
- performs no NHL, fantasy, or IceCast queries;
- records document ID/schema and renderer version in artifact metadata;
- treats text accuracy as a validation gate; and
- keeps the JSON document as the durable source artifact.

The reference implementation is `scripts/render-card-document.ps1`. Its
default SVG mode is deterministic and uses initials supplied from document
labels. `-ResolveAssets` may dereference only HTTPS asset URLs already carried
by the document; it validates an image response, embeds the bytes, and falls
back to initials on any failure. It never constructs a player URL or infers
asset identity. SVG metadata, PDF sidecars, and `render-manifest.json` record
the renderer and document fingerprint; the manifest also records resolved and
fallback asset counts. Every source-derived rendered string is reparsed from
the SVG and compared exactly before the artifact is accepted.

## Public Entry Points

Implemented CLI entry points:

```powershell
icelines report team-card --team NYR --season 20262027 --scenario-id nyr-development-variance
icelines icecast season-card --input forecast.json --team SEA --team-name "Seattle Kraken"
icelines fantasy roster-card --date 2026-10-08 --json
icelines fantasy draft-card --taken-file taken.txt --json
icelines fantasy morning-card --date 2026-10-08 --json
icelines fantasy trade-card "Bouchard" --to-team Other --for-player "Werenski" --json
```

Implemented sealed showcase routes:

```text
GET /icecast/:season/:team/card?scenario=:scenario_id
GET /api/v1/cards/team-prognosis/:season/:team?scenario=:scenario_id
GET /fantasy/cards/roster/:team
GET /api/v1/cards/fantasy-roster/:team
GET /fantasy/cards/draft/:team
GET /api/v1/cards/fantasy-draft/:team
GET /fantasy/cards/morning/:team
GET /api/v1/cards/fantasy-morning/:team
GET /fantasy/cards/trade/:team
GET /api/v1/cards/fantasy-trade/:team
GET /icecast/:season/:team/simulation?page=scoreboard|insider
GET /api/v1/cards/season-simulation/:season/:team
GET /icecast/:season/:team/movement?page=shift|insider
GET /api/v1/cards/forecast-movement/:season/:team
GET /icecast/:season/:team/history?page=tape|insider
GET /api/v1/cards/forecast-history/:season/:team
```

The web provider is intentionally read-only and authority-gated to sealed NYR,
SEA, and Sample Multicategory showcase documents. It returns the complete core
document directly from JSON routes, uses the document fingerprint as its ETag,
and rejects unsupported dimensions explicitly. HTML page selection changes
projection only, never the document.

TUI:

```text
icelines tui team-card NYR
icelines tui team-card DEX
icelines tui team-card DRAFT
icelines tui team-card MORNING
icelines tui team-card TRADE
season-card NYR
replay-card NYR
movement-card NYR
history-card NYR
```

The implemented card experience is also available from the running TUI as
`:team-card NYR`, `:team-card SEA`, `:team-card DEX`, `:draft-card`,
`:morning-card`, `:trade-card`, `:season-card`, `:replay-card`, or
`:movement-card`, or `:history-card`. JSON schema names and core builders remain
canonical.

## Scenario Registry

Web/TUI card requests reference a stable scenario ID. A scenario registry row
contains:

- ID and display name;
- season and allowed team scope;
- scenario schema/version;
- content hash;
- evidence label;
- created/updated timestamps; and
- source path only for local administrative inspection.

CLI may import a local scenario file into the registry or explicitly run an
ephemeral file. Web routes never accept arbitrary filesystem paths.

## Validation and Invariants

All card documents validate before rendering:

1. document ID and fingerprints are deterministic for fixed inputs;
2. schema and card-kind versions are supported;
3. page and section IDs are unique and stable;
4. required sections for the card kind are present;
5. player/team identity uses stable IDs;
6. one player cannot occupy two active lineup slots;
7. lineup shape is legal or has an explicit warning/empty state;
8. scores are finite, bounded, and methodology-stamped;
9. missing metrics are null, never silent zero;
10. probabilities are finite and bounded from 0 to 1;
11. baseline/scenario deltas reconcile within tolerance;
12. isolated impacts come from paired identical-seed simulations;
13. combined-event probabilities reconcile to realization buckets;
14. asset source and fallback are explicit;
15. every semantic color has a non-color cue;
16. warnings and provenance survive JSON round-trip; and
17. renderers can reject unsupported required sections safely.

## Testing Strategy

### Core contract tests

- canonical NYR and SEA golden fixtures;
- fixed clock, seed, parameters, scenario, and source generation;
- duplicate identity and illegal lineup failures;
- missing headshot, missing score, stale roster, and partial forecast fixtures;
- exact score/delta/probability reconciliation;
- schema round-trip and unknown-version refusal;
- stable document fingerprint; and
- property tests for finite metrics and bounded probabilities.

### Renderer parity tests

- CLI text, TUI snapshot, web HTML, and web JSON consume one fixture object;
- no renderer imports scoring, simulation, ranking, or lineup builders;
- player ordering and primary metric displays match across surfaces;
- warnings and evidence labels appear in every applicable surface;
- unsupported-section behavior is explicit; and
- no-color/ASCII TUI and CLI remain meaningful.

### Visual and accessibility tests

- TUI snapshots at 80x24, 120x32, and 160x45;
- web screenshots at 1440x900 and 390x844;
- keyboard focus and screen-reader literal labels;
- contrast checks for supplied theme roles;
- long names, missing faces, `NR` scores, and partial lineups; and
- reference image text validation against the source JSON.

## Bug Policy

Card work is an IceLines truth audit.

When a card exposes a missing, contradictory, or malformed value:

1. reproduce it against the canonical builder input;
2. classify the defect as source, identity, domain logic, contract, or renderer;
3. fix the lowest authoritative IceLines layer;
4. add a regression test at that layer;
5. regenerate the ViewModel and all affected artifacts; and
6. never patch a hockey value only in CSS, templates, image prompts, or TUI
   render code.

## Acceptance Criteria

1. NYR and SEA card documents build from IceLines data with no manually entered
   roster, score, or probability in a renderer.
2. Page 1 contains legal projected lines, official/fallback faces, names, and
   one comparable score only.
3. Page 2 explains baseline, ceiling, the rounded +16 team-strength path,
   isolated player impacts, combined likelihood, and downside.
4. CLI JSON, web JSON, web HTML, and TUI consume the identical document.
5. Web and TUI perform no hockey calculations.
6. Missing or stale authority is visible and lowers claims rather than creating
   fallback facts.
7. The JSON schema is documented, fixture-tested, and usable by an independent
   renderer without repository access.
8. A fantasy roster card and a season simulation card prove that the shared
   grammar generalizes without collapsing their domain-specific rules.
9. All new bugs discovered during the showcase have regression tests.
10. Specs, plans, surface parity, commands, and architecture documents point to
    this contract without duplicating its field definitions.
