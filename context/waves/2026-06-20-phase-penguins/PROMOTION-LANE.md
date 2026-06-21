# Penguins Promotion Lane Selection

## Decision

Phase Penguins selects **coach dashboard** as the candidate workflow lane for
the next gates.

This is not a promotion yet. It means the next pulses should audit coach
dashboard product copy and workflow evidence before any surface-matrix claim is
strengthened.

## Why Coach Dashboard

Coach dashboard has the strongest first-route posture among the WP-009 family:

- default active-context cache key: `coach_dashboard:<season>:<type>`;
- HTML and JSON routes exist;
- missing cache renders explicit unavailable state;
- missing reads do not create cache storage;
- ready records render through `AnalyticsCacheConsumerView`;
- existing L2 tests cover non-claim copy.

It is also the least specific high-authority family compared with line
deployment, goalie readiness, practice prescriptions, postgame blame, or agent
action. Those families carry stronger product risk and should remain bounded
unless separate evidence is added.

## Families Kept Bounded For Now

- Named cache report remains generic prepared-cache inspection.
- Opponent scout remains first-route scout report evidence, not a game-plan
  workflow.
- Player evidence card remains first-route player evidence evidence, not a full
  research/deployment/transaction workflow.
- Line combinations remain first-route evidence, not deployment advice or
  line-chemistry causality.
- Goalie readiness remains first-route evidence, not injury certainty or
  start/sit authority.
- Practice focus remains first-route evidence, not mandatory drill plans.
- Postgame review/adjustments remain first-route evidence, not causal blame or
  automatic correction authority.
- Agent evidence remains first-route evidence, not autonomous action.

## Next Gate

Pulse 03 should audit the coach dashboard copy and decide whether the current
route copy is enough for a bounded workflow claim or whether it needs durable
deferral wording.
