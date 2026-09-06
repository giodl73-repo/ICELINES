---
skill: roles-check
topic: external-player-links
date: 2026-09-06
roles_used: 7
p1_count: 0
verdict: APPROVED
---

# External player links role review

## Artifact identification

- Type: reusable IceLines web navigation and consumer-report integration.
- Artifacts: `GET /player/by-name/:name?team=ABBR`, its route tests and surface-parity contract, plus PUCK's private morning-report links.
- Signals: canonical player identity, Unicode normalization, duplicate-name recovery, browser cold start, accessibility, and cross-repository ownership.

## Role selection

- KEEL: verifies that PUCK links into IceLines rather than duplicating hockey history.
- TAPE: verifies canonical name-to-NHL-ID identity flow.
- FORGE: reviews the Rust handler and response paths.
- BENCH: requires route, normalization, ambiguity, and recovery tests.
- EDGE: challenges missing names, accents, duplicate names, stale teams, and unavailable servers.
- GLASS: reviews link discoverability, visual hierarchy, and focus treatment.
- broadcast: reviews bookmarkability, cold-browser behavior, recovery pages, and local-server lifecycle.

## Review

### KEEL

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Copying IceLines career history into PUCK would create a divergent second analytics surface. | P2 | Ownership | Keep PUCK as the decision-report renderer and link to IceLines' `PlayerCardView` page. Resolved. |
| 2 | A Yahoo player ID cannot be treated as the canonical NHL identity. | P2 | Identity bridge | Resolve the canonical NHL ID inside IceLines from the retained player name. Resolved. |
| 3 | The new mounted route needs a surface-parity declaration and route-inventory fence. | P2 | Platform contract | Document `GET /player/by-name/:name` and add it to the route inventory. Resolved. |

### TAPE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Accent differences such as Slafkovsky/Slafkovský can break literal joins. | P2 | Name resolution | Use IceLines' canonical Unicode-normalized name axis and test the unaccented request. Resolved. |
| 2 | Duplicate normalized names must not silently resolve to the first bundled row. | P2 | Name resolution | Require a team discriminator or render explicit choices. Resolved. |
| 3 | Consumer team abbreviations differ from canonical NHL abbreviations for LA, NJ, SJ, and TB. | P3 | Team hint | Normalize those known aliases only for duplicate-name disambiguation. Resolved. |

### FORGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | This adapter needs no repository mutation or async loader work. | P3 | Handler boundary | Keep it as a thin read-only resolver and redirect. Resolved. |
| 2 | Manually rendered recovery HTML can admit injection through the path value. | P2 | Error rendering | Escape the requested name and candidate labels before interpolation. Resolved. |
| 3 | A new dependency would be unnecessary for this small route. | P3 | Dependency surface | Reuse existing normalization, Axum extractors, and response types. Resolved. |

### BENCH

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | A happy-path redirect alone would not prove identity safety. | P2 | L1 tests | Test unique resolution, missing-diacritic resolution, ambiguity, team disambiguation, and unknown recovery. Resolved. |
| 2 | Route documentation can drift from the Axum router. | P2 | Inventory test | Extend `ted_lindsay_route_inventory`. Resolved. |
| 3 | The consumer report needs executable evidence, not only string construction tests. | P3 | Integration | Regenerate the report, start the local server, and verify Nick Suzuki redirects to `/player/8480018` and renders the career page. Resolved. |

### EDGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Sebastian Aho demonstrates that normalized names are not unique. | P2 | Ambiguity | Return 300 with player choices unless the team hint selects exactly one candidate. Resolved. |
| 2 | A player may have no team hint or a stale team after a trade. | P3 | Team hint | Let one exact name resolve regardless of team; use team only to break duplicates. Resolved. |
| 3 | An unknown player or unavailable portrait must not make the report unusable. | P3 | Degradation | Keep portrait fallback independent and provide a player-not-found page with a leaders link. Resolved. |

### GLASS

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Clickable cards need an explicit affordance beyond cursor behavior. | P3 | Lineup board | Add explanatory copy and hover/focus elevation with a gold outline. Resolved. |
| 2 | Repeating portraits in data tables would dilute the large lineup board. | P3 | Report hierarchy | Keep portraits in the formation and make table names textual links. Resolved. |
| 3 | Motion and focus styling must remain accessible. | P2 | CSS | Add `:focus-visible` treatment and disable card transitions under `prefers-reduced-motion`. Resolved. |

### broadcast

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | A localhost link fails when the user opens the report cold and IceLines is not running. | P2 | Cold start | On a visible morning run, probe and start `icelines serve --no-open` on the configured loopback port. Resolved. |
| 2 | External report links must be bookmarkable and safe in a new tab. | P2 | Browser navigation | Use encoded path/query state with `target=_blank` and `rel=noopener`. Resolved. |
| 3 | Unknown and ambiguous lookups require usable HTML, not opaque errors. | P2 | Recovery | Render semantic, viewport-aware pages with skip links and direct recovery choices. Resolved. |

## Synthesis

Roles reviewed: 7  
P1 blockers: 0 | P2 issues: 13 | P3 notes: 8

Verdict: APPROVED

Top finding: canonical NHL identity must be resolved by IceLines and duplicate names must never be guessed.

Cross-role consensus: KEEL, TAPE, BENCH, and EDGE agree that the report should retain names and team context while IceLines owns canonical ID resolution and history rendering. GLASS and broadcast agree that the entire visible card should be a keyboard-accessible link with a cold-start path.

## Amendments applied

1. Added an exact normalized, team-aware, read-only route that redirects to the canonical `/player/:id` card and explicitly recovers from unknown or ambiguous names.
2. Added L1 coverage for unique, accented, duplicate, team-disambiguated, and unknown player requests, plus the route-inventory and surface-parity records.
3. Linked PUCK's formation cards and player-name tables to IceLines, added accessible interaction styling, and made visible report runs start the loopback dashboard when necessary.
