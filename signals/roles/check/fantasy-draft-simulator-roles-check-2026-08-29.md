---
skill: roles-check
topic: fantasy-draft-simulator
date: 2026-08-29
roles_used:
  - hart
  - keel
  - tape
  - forge
  - pace
  - bench
  - edge
  - scout
  - glass
p1_count: 0
verdict: pass_with_advisories
---

# Fantasy draft simulator roles check

## Scope and selection

Reviewed PR 47's reusable snake-draft paths, exact NHL calendar replay,
schedule stress metrics, goalie-minimum proxy, post-draft injury replacements,
CLI/JSON output, documentation, and tests.

- HART: public model and season-axis shape.
- KEEL: core/CLI ownership and cross-output convergence.
- TAPE: player/team/schedule identity integrity.
- FORGE: Rust correctness, determinism, and lint cleanliness.
- PACE: algorithm bounds, heuristic transparency, and runtime.
- BENCH: L0/L2 evidence and regression coverage.
- EDGE: incomplete inputs and boundary behavior.
- SCOUT: hockey reasonableness, goalie interpretation, and youth context.
- GLASS: terminal readability and user-facing disclosures.

BROADCAST, CREST, and WIRE were not selected: this change does not add a web
surface, visual asset, network protocol, or externally hosted API endpoint.

## Findings by role

### HART

1. **PASS** — The core owns a single `FantasyDraftSimulationView`; text and
   JSON render the same paths rather than recomputing schedule logic.
2. **PASS** — Schedule dates, scoring season, positions, goalie opportunities,
   and replacement rows are explicit typed fields rather than overloaded
   player-score fields.
3. **P3** — Draft identities still use normalized player names. That matches
   the existing FantasyDb contract, but same-name active players remain a
   future migration case for canonical NHL-ID keys.

### KEEL

1. **PASS** — Snake selection, feasibility, calendar assignment, stress-week
   selection, and replacements remain pure `icelines-core` logic; the CLI only
   loads data and renders the view.
2. **PASS** — `draft-sim` consumes the same league rules and draft board used
   by the existing assistant instead of creating a parallel scoring system.
3. **P3** — Schedule loading intentionally degrades through `.ok()` and a
   warning. A later diagnostics pass should retain the underlying load error
   in machine output without making offline use fail closed.

### TAPE

1. **PASS** — Calendar membership is built from official regular-season games
   and keyed by current NHL team abbreviation; malformed dates are skipped
   deterministically.
2. **PASS** — Replacement candidates come only from the pool remaining after
   all simulated `league_size × rounds` selections.
3. **P3** — Market CSV joins are normalized-name joins. Ambiguous same-name
   imports should eventually carry NHL ID or a structured ambiguity result.

### FORGE

1. **RESOLVED P2** — Replacement selection reversed its final draft-value
   comparison under otherwise equal weekly results. The comparator now prefers
   the higher draft value and has a focused regression test.
2. **RESOLVED P2** — CI's `-D warnings` rejected `strategy_value` for eight
   arguments. Availability is now calculated at the call site, keeping the
   helper below the lint boundary without suppressing the lint.
3. **PASS** — Stable maps, explicit tie-breakers, saturating arithmetic, and
   finite validated input scores keep simulations deterministic and panic-free
   for validated inputs.

### PACE

1. **PASS** — Full-roster daily replay is exact for the league's legal active
   slots; per-pick exact scoring is deliberately bounded to eight proxy-ranked
   legal candidates.
2. **RESOLVED P3** — The eight-candidate bound was previously a magic literal
   and invisible to users. It is now a named constant and an emitted warning.
3. **P3** — Schedule-First coefficients and the 120/32 replacement screens are
   disclosed heuristics, not calibrated pick probabilities. Retain actual
   season outcomes before promoting them as predictive weights.

### BENCH

1. **RESOLVED P2** — The new command lacked the repository-required L2 proof.
   A subprocess test now verifies `draft-sim --help` and all schedule controls.
2. **RESOLVED P2** — A new L0 regression locks the corrected replacement
   draft-value tie-break behavior.
3. **PASS** — Existing focused tests cover 14-team snake turns, intervening
   market picks, exact collision avoidance, undrafted replacement isolation,
   and weekly goalie-opportunity risk.

### EDGE

1. **PASS** — League size, draft slot, rounds, current overall pick, goalie
   caps, quiet-slate threshold, and replacement count are range checked.
2. **PASS** — Empty schedules produce empty calendar/replacement sections plus
   explicit warnings instead of fabricated schedule values.
3. **P3** — Opening and closing partial NHL weeks can rank as stress weeks.
   Activity weighting reduces their impact, but a future option could exclude
   partial fantasy matchup weeks when league dates are known.

### SCOUT

1. **PASS WITH BOUNDARY** — Weekly goalie safety counts scheduled team-game
   opportunities, never claims confirmed starts, and repeats that limitation
   in output. In-season starter evidence remains required.
2. **PASS WITH BOUNDARY** — Completed-season production is a baseline; injury
   and role risk are explicitly not deducted. External projections can replace
   that baseline when supplied.
3. **P3** — Youth-Upside uses a transparent linear age bonus. Breakout curves,
   deployment, and prospect uncertainty remain scouting context rather than
   hidden certainty in this first reusable simulator.

### GLASS

1. **RESOLVED P2** — The `COMMANDS.md` draft examples left prose inside the
   PowerShell fence. The fence now closes immediately after the commands.
2. **PASS** — Text output separates picks, fallbacks, calendar totals, stress
   weeks, injury replacements, and goalie-risk flags into scan-friendly blocks.
3. **P3** — “Activity-weighted open slots” is precise but technical. A future
   compact glossary line would help first-time fantasy users interpret it.

## Severity summary

- P1: 0
- P2: 0 unresolved; 5 resolved during review
- P3: 7 open advisories; 1 resolved during review

Verdict: **PASS WITH ADVISORIES**. No correctness, CI, or release-blocking
finding remains in the reviewed scope.

## Recommended amendments

1. Migrate fantasy player identity from normalized names to NHL IDs while
   preserving a display-name/alias resolver at CSV and Yahoo boundaries.
2. Calibrate Schedule-First and goalie-risk assumptions against observed
   draft availability, starts, and usable lineup outcomes during 2026-27.
3. Add an optional fantasy matchup calendar so partial opening/closing weeks
   can be excluded from stress-week selection and playoff scoring can match the
   platform's exact week boundaries.

## Verification evidence

- `cargo test -p icelines-core replacement_ties_prefer_the_higher_draft_value`
- `cargo test -p icelines-cli --test system_tests l2_cmd_fantasy_draft_sim_help_exposes_schedule_controls`
- `cargo clippy -- -D warnings`
- `cargo fmt --all -- --check`
- `git diff --check`
