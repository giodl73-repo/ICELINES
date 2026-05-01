# IceLines Phases — Trophy Naming

Each phase of the project is named after an NHL trophy that captures its
character. The matchup is intentional: the Vezina is for goalies, the
Norris is for defensemen who shore up the structure, the Hart is for
the foundational MVP-tier work that touches everything. The names are
load-bearing — they show up in commits, tags, and plan filenames — so
adding a new phase means picking a trophy that fits.

## Active mapping

| Phase | Trophy | What it covered | Rationale |
|---|---|---|---|
| 1 — Foundation | **Calder** | Initial Rust CLI, fetch pipeline, snapshot store | Rookie of the Year — the project's first draft. |
| 2 — Site | **Lady Byng** | mkdocs static site, ranked-team index, lineup cards | Sportsmanship + skill — the polished, restrained, public-facing output. |
| 3 — Projections | **Art Ross** | TUI projections leaderboard, pace_82 sorting | Most points — the scoring title, the headline leaderboard. |
| 4 — History polish | **King Clancy** | Career history, historical seasons, data hygiene | Leadership + community — long-term data quality work that benefits every later phase. |
| 5 — Query engine | **Bill Masterton** | `query leaders/player/compare`, --seasons N, percentiles | Perseverance — the hard, unglamorous filter/sort/score plumbing. |
| 6 — Export & dashboards | **Mark Messier** | `export md`, dashboard panel, scout cards | Leadership Award — multi-surface vision, leading data into other tools. |
| 7 — TUI v2 redesign | **Jack Adams** | 7-tab layout, admin overlay, season time-travel | Coach of the Year — strategic restructuring of the whole system. |
| 8 — Spec delta + chunks | **Norris** | Spec catch-up, chunked snapshots, data integrity | Best defenseman — defensive, structural, plays both ends, foundational. |
| G — Goalies | **Vezina** | Goalie type, repository, fantasy goalie scoring, 5 bundled seasons | The goalie trophy. Obvious. |
| T — Transactions | **Selke** | Trades/waivers/signings/IR feed, ESPN source, TUI tab | Best defensive forward — two-way work spanning data ingestion + UI + 5 seasons. |
| E — Edge speed | **Maurice Richard** | Skating speed leaderboard (blocked: no public API) | Most goals — pure scoring ability — but the data isn't accessible. Parked. |
| Hart — Normalization | **Hart** *(in progress)* | Full data model normalization — `PlayerIdentity` + `SeasonStats` keyed by (player_id, season, type) | League MVP — most valuable single piece of work; touches every consumer. Subsumes Phase Presidents (season-type). |

## Future / parked

| Phase | Trophy | Status | What it would cover |
|---|---|---|---|
| Presidents — Season type | **Presidents'** | **Folded into Hart** | Regular vs playoff scoping. Naturally falls out of the (player_id, season, type) primary key after Hart. |
| Cup — Live game tracking | **Conn Smythe** | Future | Real-time playoff game tracking, series momentum, Cup-run player narratives. |
| (TBD) — Scoring scheme builder | **Frank Selke** | Future | Defensive-leaning fantasy scheme (penalty kills, shorthanded points, blocks-heavy) — a complement to the offense-tuned Yahoo/ESPN defaults. |

## Naming conventions

- **Plan filenames**: `YYYY-MM-DD-phase{Trophy}-{slug}.md` — e.g.
  `2026-04-30-phaseHart-normalization.md`. Trophy is the canonical
  identifier; the slug is for filesystem readability.
- **Tag names**: continue using semver (`v0.12.0`), but reference the
  phase trophy in the tag annotation message.
- **Commit subjects**: `Phase {Trophy}: short description` — e.g.
  `Phase Vezina: goalie repository + 5 bundled seasons`. (Existing
  commits used letter codes like `G.4`; future commits switch to
  trophy names.)
- **Memory entries**: refer to phases by trophy when documenting
  decisions for future sessions.

## Why bother

Three reasons:
1. **Mnemonic**: "Vezina was the goalie phase" sticks; "G.4 was
   the team-roster goalie strip" doesn't.
2. **Tone**: the project is hockey-domain; the names should be too.
3. **Scope discipline**: picking a trophy forces an honest read on
   what the phase IS. If you can't name it after a trophy, the scope
   is probably tangled.

## Picking a trophy

Match the trophy's character to the phase's character:
- **Foundational, structural, both-ends work** → Norris.
- **MVP impact, touches everything** → Hart.
- **Pure scoring / leaderboard** → Art Ross / Maurice Richard.
- **Polish, surface, public-facing** → Lady Byng.
- **Strategic restructuring** → Jack Adams.
- **Two-way concern, multi-surface** → Selke.
- **Long-term, unglamorous, infrastructure** → Bill Masterton / King Clancy.
- **Goalie or playoff-specific work** → Vezina / Conn Smythe.

If two trophies fit, pick the one with the more specific mandate — the
trophy should narrow the scope, not widen it.
