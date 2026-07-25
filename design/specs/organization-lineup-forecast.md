# Organization Lineup Forecast — The System

**Status:** Implemented (foundation)

The System joins an NHL lineup projection to its current AHL affiliate projection
without moving lineup logic into a UI. Its output is the versioned,
UI-neutral `organization_lineup_forecast.v1` document consumed by CLI, web,
TUI, cards, fantasy, and simulation surfaces.

## Contract

An `organization_lineup_forecast.v1` contains:

- four NHL and four AHL forward lines;
- three NHL and three AHL defense pairs;
- two NHL and two AHL goaltenders;
- NHL extras and AHL players outside the dressed lineup;
- forward, defense, and goalie recall ladders and a first-recall plan;
- the NHL special-teams projection; and
- both source documents, disclosures, evidence labels, development-rule
  status, and explicit AHL roster-pool authority.

The builder accepts one JSON object with existing UI-neutral documents:

```json
{
  "nhl_lineup": { "schema": "team_lineup_projection.v1" },
  "ahl_affiliate": { "schema": "ahl_affiliate_projection.v1" }
}
```

The abbreviated objects above illustrate the envelope only; both embedded
documents must be complete.

## Validation

The builder fails closed unless the parent team and season match, the current
official affiliation matches, the AHL development rule is satisfied, and both
levels contain complete 12F/6D/2G dressed lineups. Player identities must be
unique within each level. A player dressed in the NHL cannot also be assigned
to the AHL.

Recall candidates are grouped as forward, defense, and goalie. Explicit
recall-readiness evidence ranks first, then projected player score. A leading
candidate is not an automatic transaction: waivers, cap, contract consent,
emergency-recall rules, and current injury status remain transaction-time gates.

## Surfaces

```powershell
icelines icecast organization --input organization.json
icelines icecast organization --input organization.json --json --out the-system.json
```

Text output is a compact inspection surface. JSON is authoritative and is the
integration point for web, TUI, cards, fantasy roster decisions, and season
simulation.

## Evidence boundary

The initial contract preserves NHL PP1/PP2 and PK1/PK2 from the NHL lineup.
AHL special teams remain explicitly unavailable until affiliate role evidence
is supplied. Real NYR/Hartford and SEA/Coachella forecasts also require a
reviewed NHL/AHL identity crosswalk and a complete organizational candidate
pool; the engine must not invent player IDs or treat an empty current roster
feed as a complete camp roster.

The System may render a sourced preseason projection before the official AHL
roster publishes, but its top-level `ahl_pool_authority` and renderer label must
remain `preseason_projection`, never `official_snapshot`.

## Next increments

1. Populate reviewed Hartford and Coachella Valley identity artifacts when
   official 2026-27 roster coverage is complete, then add cross-league career games.
2. Add evidence-labeled AHL PP/PK role inputs and units.
3. Simulate injury/recall/demotion cascades through both levels.
4. Feed organization branches into The Blender, The Bench, and IceCast season
   trials so lineup changes affect both NHL results and affiliate development.
