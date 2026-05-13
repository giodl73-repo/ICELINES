# IceLines Waves

Waves turn broad phase cleanup into auditable execution packets.

The model is adapted from `C:\src\craft-artifacts`: a wave owns the mission,
pulse plans split that mission into self-contained slices, review panels apply
the `.roles` lenses, and fork files become the single packet handed to agents.

## Layout

```text
design/waves/
  PHASES.md
  {date}-{verb}-the-{object}/
    WAVE.md
    plans/
      pulse-01.md
    forks/
      pulse-01.md
    panels/
      pulse-01-r1/
```

## Pulse Lifecycle

1. Open or select an active wave in `PHASES.md`.
2. Generate pulse plans in `plans/`.
3. Review pulse plans with `.roles` and write findings in `panels/`.
4. Materialize forks in `forks/`; each fork contains the full pulse and roles.
5. Dispatch agents with only the fork path.
6. Sync completed gates back into the pulse plan.
7. Close the wave with commits, evidence, and remaining debt.

## Backfill Rules

- A pulse must be small enough for one agent to complete independently.
- A pulse must list owned files, tests, gates, and stop conditions.
- A pulse must not require the agent to infer phase history from memory.
- A pulse that changes code must name focused tests and release impact.
- A pulse that only changes docs must still name the docs drift it removes.
