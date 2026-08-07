# Player-Line Matchup Forecast — Implementation Plan

**Date:** 2026-08-01
**Status:** Active child workstream
**Parent:** Team Season Forecast / Game-Prediction Edge

## Objective

Turn dated player profiles, expected line combinations, pair/trio evidence,
opponent style, and manager execution into a leakage-safe matchup feature for
the existing game-probability model and all-32 season simulator.

## Delivery

1. **Typed foundation — implemented**
   - Add confidence-weighted multi-dimensional player profiles.
   - Separate shift-adjusted outcomes, deployment affinity, coarse co-appearance,
     and simulated fit.
   - Score complete forward lines and defense pairs against opponent style.
   - Report PP1-versus-PK1 suitability separately from 5-on-5.
   - Prove one method applies to all 32 teams.

2. **Game-edge bridge — implemented**
   - Seal `player_line_matchup_forecast.v1` with dated source fingerprints.
   - Attach the two bounded matchup values to the existing edge evidence package.
   - Preserve one game model and refuse mismatched identities or timestamps.

3. **Source adapters — in progress**
   - Derive player dimensions from strictly prior lineup scores, role evidence,
     official EV minutes, and exact shift counts — implemented.
   - Convert sealed shift-aligned xG outcomes relative to a declared baseline
     into chemistry evidence — implemented.
   - Parse published MoneyPuck pair/trio game files, recover stable NHL IDs,
     freeze games before the forecast, and aggregate score/venue-adjusted xG
     against per-game sealed baselines — implemented.
   - Require each pregame baseline to declare individual, opponent, and
     deployment components; disclose baseline coverage and excluded rows —
     implemented.
   - Discover units from a rights-declared local season-summary/line/skater/team
     package and automatically generate strictly pregame individual/opponent/
     zone-start baselines — implemented.
   - Do not automate bulk MoneyPuck network acquisition; the provider's live
     license gate asks bulk consumers to arrange an agreement — implemented
     refusal boundary.
   - Retain zone starts, score state, opponent quality, and role in the residual
     baseline instead of attributing them to chemistry.
   - Build projected/confirmed game-day lineups with captured-at authority.

4. **Lineup alternatives and manager behavior — comparison primitive implemented**
   - Run legal Blender alternatives through the same Matchup builder and rank
     them against a named frozen baseline — implemented.
   - Use home last change, hard-match confidence, line shares, fatigue, and coach
     adaptation without duplicating schedule effects — Bench adapter implemented.
   - Connect selected mid-season Bench lineups to subsequent game forecasts.

5. **Historical calibration — validation contract implemented**
   - Freeze every feature before the game.
   - Run player-only, pair, pair/trio, and manager/matchup ablations.
   - Require Brier/log-loss improvement plus leave-one-season and leave-one-team
     stability before registering a challenger.
   - Chronological five-stage metrics and stability are implemented; historical
     observation harvesting remains.

6. **Product surfaces**
   - Add The Matchup card with player faces, unit scores, evidence confidence,
     probability movement, and why-the-line-works explanations. **Initial
     UI-neutral/CLI slice implemented:** `line-matchup-card` projects one sealed
     forecast into the shared two-page card contract without recomputing hockey
     values. It preserves all 36 player joins, dressed-unit membership, evidence
     state, warnings, source fingerprint, methodology, and separate special-teams
     read. **Edge composition implemented:** the same command can accept a sealed
     prediction edge, require that edge to cite the exact matchup fingerprint,
     and display edge-owned win probability, matchup-factor movement, and
     evidence confidence without recomputing probability arithmetic. Headshot
     assets remain separate.
   - Add Web/TUI discovery while preserving the same UI-neutral document.
   - Pilot Rangers/Kraken, then run the identical all-32 pipeline.

## Current boundary

The foundation can build profiles from current IceLines authorities, consume
separately sealed shift-adjusted chemistry inputs or build them from published
MoneyPuck pair/trio game files plus complete frozen pregame baselines, feed the
game edge, and evaluate frozen five-stage ablations. It does not claim that
official shifts alone are outcome-adjusted, and it does not promote weights
until real chronological observations pass the validation contract. Automatic
baseline generation is implemented for caller-supplied, rights-declared source
packages; licensed cache import can be added without changing the model.
