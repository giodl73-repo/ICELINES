# Prospect Development Study

**Status:** Implemented foundation
**Schema:** `prospect_development_study.v1`

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

Build league-wide studies from reviewed NHL/AHL identity joins, consecutive AHL
season facts, transactions/injuries, official organization reporting, and a
separately disclosed attention feed. That adapter remains distinct from this
deterministic core contract.
