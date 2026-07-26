# Prospect Development Study

**Status:** Implemented foundation
**Schemas:** `prospect_development_study.v1`, `prospect_discovery_board.v1`,
`prospect_league_context.v1`, `prospect_league_discovery.v1`,
`prospect_program_board.v1`

## Purpose

Identify prospects whose current development signal is stronger than their
public visibility. The study keeps performance, workload confidence,
opportunity, injury/availability, and attention separate so an interrupted NHL
debut does not masquerade as development failure.

```bash
icelines icecast prospect-study \
  --input examples/icecast-jagger-firkus-prospect-study.json
icelines icecast prospect-study \
  --input examples/icecast-jagger-firkus-prospect-study.json \
  --json --out firkus-study.json
icelines icecast prospect-board \
  --study firkus-study.json \
  --study another-study.json \
  --json --out prospect-board.json
icelines icecast prospect-context \
  --snapshot ahl-2023-24.json \
  --snapshot ahl-2024-25.json \
  --snapshot ahl-2025-26.json \
  --league-crosswalk reviewed-league-2023-24.json \
  --league-crosswalk reviewed-league-2024-25.json \
  --league-crosswalk reviewed-league-2025-26.json \
  --affiliations ahl-affiliations-2025-26.json \
  --as-of 2026-09-15 --max-age 24 \
  --json --out prospect-context.json
icelines icecast prospect-league \
  --snapshot ahl-2024-25.json \
  --snapshot ahl-2025-26.json \
  --crosswalk reviewed-2024-cv.json \
  --crosswalk reviewed-2025-cv.json \
  --context examples/icecast-prospect-league-context.json \
  --json --out league-discovery.json
icelines icecast prospect-program \
  --league-discovery league-discovery.json \
  --json --out prospect-programs.json
```

## Contract

The input supplies consecutive season totals, documented NHL opportunity,
availability state, an explicit 0..1 attention estimate with its basis, and
source URLs. The core primitive owns:

- points-per-game and same-league year-over-year changes;
- workload confidence;
- rising, stable, cooling, or insufficient trajectory;
- transparent production, trajectory, opportunity, and attention-gap lenses;
- the 0..100 discovery score, market position, independent discovery lenses,
  and summary classification; and
- disclosures explaining what the score can and cannot claim.

The CLI, TUI, web, fantasy, simulation, and cards may render or consume the
same view without recomputing those semantics.

## Discovery board

`ProspectDiscoveryBoardView` composes one or more validated study artifacts into
three independently ranked lanes:

- **Hidden Gems** requires supported upside plus underrecognition, or a study
  classification that explicitly identifies hidden or injury-obscured value;
- **Buyer Beware** requires overexposure or an explicit hype/cooling
  classification; and
- **Watch** retains aligned and uncertain cases without inventing a positive or
  negative conclusion.

Every row preserves its classification, market position, hidden-value score,
performance-attention gap, and complete set of active lenses. Hidden Gems rank
by hidden-value score. Buyer Beware ranks by the strongest supported risk or
negative attention-gap signal. Lane scores are not comparable across lanes.
The builder rejects malformed schemas and duplicate player IDs, so renderers do
not reconcile or silently overwrite studies.

## Reviewed league adapter

`ProspectLeagueDiscoveryView` is the mainline bridge from AHL data into the
study and board primitives. It accepts:

1. two or more official `ahl_roster_stats.v1` season snapshots;
2. reviewed `ahl_identity_crosswalk.v1` documents for the relevant season/team
   combinations; and
3. one `prospect_league_context.v1` document containing facts the feeds cannot
   safely infer.

The adapter joins provider-local AHL identities to canonical NHL IDs only
through rows whose status is `reviewed`. It aggregates joined skater totals by
season, attaches snapshot and identity provenance, builds the canonical studies,
and composes the board without reimplementing scoring. Context players that do
not have reviewed identity, joined skater facts, or two AHL seasons appear in a
typed exclusion list. If no eligible study remains, the command fails instead
of returning a zero-shaped board.

The separate context file owns current organization, position, age, NHL games,
opportunity, availability, attention estimate/basis, and supporting evidence.
Those fields are deliberately not guessed from AHL production.

`prospect-context` can now create an `observed_draft` context for the whole AHL
from official season snapshots, reviewed league crosswalk envelopes, and a
dated affiliation catalog. It retains only skaters appearing in the latest
snapshot at or below the configured age ceiling with at least two joined AHL
seasons. Provider `active` state resolves the current organization after an
in-season AHL trade; multiple active organizations remain an explicit
exclusion. Goalies, older players, one-season samples, missing affiliations,
and unresolved assignments are preserved in typed exclusions.

The generated artifact uses neutral placeholders for the facts the AHL adapter
cannot establish: NHL games remain zero, opportunity is `none`, availability is
`unknown`, and attention is 0.5. Its `observed_draft` authority fails validation
if those fields become non-neutral without first being promoted to authored
context. Consequently the draft is suitable for the attention-independent
program ranking, but Hidden Gems and Buyer Beware require separate sourced
enrichment. `prospect-league --crosswalk` accepts either individual reviewed
team crosswalks or reviewed league envelopes and flattens the latter without
weakening the reviewed-only join.

## Prospect program board

`ProspectProgramBoardView` aggregates canonical prospect studies by
organization into three independent frozen ranks:

- **Pool / The Depth Chart** combines the top-three observed signal, quality
  depth, and positional balance;
- **Development / The Factory** combines same-league trajectory evidence with
  workload confidence and observed program breadth; and
- **Pipeline / The Pipeline** combines Pool, Development, documented
  readiness, and confidence.

The observed player signal uses production, trajectory, and documented
opportunity components. It deliberately excludes hidden-value and attention-gap
scores because underrecognition is not prospect talent or ceiling. Missing
depth lowers depth and confidence instead of being imputed. The optional prior
board supplies rank and score deltas only; positive delta means improvement.

The initial board scope is explicitly `ahl_observed`. It accepts one or more
`prospect_league_discovery.v1` artifacts plus optional canonical studies from
future adapters. It is not an all-system NHL ranking until goalie, CHL, NCAA,
European, junior, and NHL-rostered prospect adapters provide equivalent typed
facts. This limitation is part of the output contract, not renderer prose.

The production path has been exercised over three official AHL seasons and 32
NHL organizations. That result is a complete all-organization AHL-skater
comparison, not a complete organizational prospect-system ranking.

## Guardrails

- At least two seasons are required.
- Both same-league seasons must meet the configured comparison workload; a
  two-game injury season cannot manufacture a recovery decline.
- Raw scoring changes are computed only against another season in the same
  league. A WHL-to-AHL move therefore cannot be labeled a decline from raw P/GP.
- Attention is an explicit sourced or analyst-estimated input, never inferred
  silently from performance.
- Injury is a separate availability state. It explains interrupted opportunity
  but adds no score.
- `injury_obscured_riser` requires a rising same-league trajectory, low authored
  attention, documented planned debut, and injury-interrupted availability.
- `injury_recovery_watch` keeps a productive return from long-term injury
  visible when the injured comparison season is too small to prove a trend.
- The score is a discovery signal, not an NHL-equivalency or roster forecast.

## Two-sided discovery lenses

The study does not force every player into a single story. It emits every
supported active lens with an upside, risk, or context direction:

| Lens | Direction | Question |
|---|---|---|
| `production_riser` | upside | Is same-league scoring improving with credible workload? |
| `injury_obscured` | context | Did injury interrupt documented opportunity? |
| `recovery_unproven` | context | Is the return promising while the injury season is too small to compare? |
| `opportunity_backed` | upside | Did the organization document recall or debut intent? |
| `attention_lag` | upside | Is evidence stronger than the authored attention estimate? |
| `attention_ahead_of_evidence` | risk | Is attention stronger than performance and opportunity evidence? |
| `workload_uncertain` | risk | Is the comparable sample below the confidence gate? |
| `cooling_signal` | risk | Did same-league scoring decline beyond the configured threshold? |

These lenses support hidden-gem classes such as `injury_obscured_riser` and
`injury_recovery_watch`, plus skeptical classes such as
`small_sample_hype_risk`, `hype_ahead_of_evidence`, and
`overexposed_cooling`.

## Planned additional viewpoints

The next adapters can add lenses only when their required facts exist:

- **depth-chart blocked** — NHL-ready evidence with no role vacancy;
- **role-obscured scorer** — strong rate production without top-six or PP time;
- **special-teams unlock** — credible PP/PK role change preceding raw totals;
- **chemistry driver/passenger risk** — shift evidence showing who creates or
  depends on teammate lift;
- **bad-team suppressed** — individual process holding up under weak team
  context;
- **shooting-percentage mirage** — goals rising without repeatable shot volume;
- **power-play dependency** — headline production overly concentrated on PP;
- **draft-pedigree bias** — attention remains high while pro evidence lags;
- **post-hype sleeper** — prior attention collapsed before underlying play; and
- **age/overage inflation** — junior dominance discounted for age and league
  context.

None of these are inferred from names or prose alone. Each requires its own
typed facts, confidence, evidence, and disclosure before activation.

## Next data step

Complete reviewed crosswalk coverage for all affiliate-season documents, then
add role, chemistry, special-teams, and sustainability fact adapters. Those
adapters must emit typed evidence into the study rather than changing renderer
logic.
