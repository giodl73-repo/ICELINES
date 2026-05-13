---
name: icelines-pulse
description: "Execute one or more IceLines pulse plans from the active wave, preferably via forked agent packets."
tags: [icelines, pulse, execution, agents, gates]
---

# icelines-pulse

Use this skill to execute pulse plans in the active IceLines wave.

## Commands

```text
/icelines-pulse 02
/icelines-pulse 02 03 04
/icelines-pulse 02-05
/icelines-pulse all
```

## Active Wave

Read `design/waves/PHASES.md` and choose the first `active` row. Pulse plans
live in:

```text
design/waves/{active}/plans/pulse-{NN}.md
```

## Agent Dispatch Prompt

When using agents, dispatch the fork file, not a summary:

```text
Read this fork file completely and execute it:
design/waves/{active}/forks/pulse-{NN}.md
```

## Execution Contract

Each pulse must:

- read its pulse or fork file completely;
- load the governing `.roles`;
- implement only the owned files/surfaces;
- run every listed gate;
- check off completed gates in the fork or pulse file;
- report changed files, tests, and unchecked gates.

## IceLines Stop Conditions

- Stop if a pulse needs a ViewModel field that does not exist.
- Stop if a mutation would become GET-backed.
- Stop if a test requires network data without a fixture.
- Stop if changes would mix unrelated phase debt.
