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
- the 0..100 discovery score and classification; and
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

## Next data step

Build league-wide studies from reviewed NHL/AHL identity joins, consecutive AHL
season facts, transactions/injuries, official organization reporting, and a
separately disclosed attention feed. That adapter remains distinct from this
deterministic core contract.
