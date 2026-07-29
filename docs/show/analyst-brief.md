# Analyst brief — IceLines

**Timebox:** 15–25 minutes. **Goal:** leave able to run rank/team/query/fantasy
moves and know what not to overclaim.

## What you get

- Local NHL analytics without an account or hosted database.
- Depth charts, pace rankings, comps, scouting-style reports, fantasy schemes.
- CLI one-liners, interactive TUI workbench, and a public web/docs site.

## 10-minute path

1. Open [the public site](https://giodl73-repo.github.io/ICELINES/) or install a
   [release binary](https://github.com/giodl73-repo/ICELINES/releases/latest).
2. Run:

```text
icelines rank --top 10
icelines team EDM
icelines query leaders --pos C --age-max 23 --sort ppg --top 10
icelines tui scores
```

3. Fantasy-shaped tools (after minimal setup from the fantasy guide):

```text
icelines stathead
icelines fantasy gaps --category hits,blocks
icelines tui poach
```

4. Skim [The Rink brand](../../design/specs/brand-the-rink.md) so headings make
   sense; command names remain the stable API.

## How to read outputs

| Signal | Treat as… | Not as… |
|---|---|---|
| Pace / pts-per-82 style ranks | Season-rate view under stated filters | True talent or future guarantee |
| Depth chart lines | Modelled assignment helper | Coach’s locked lines |
| Fantasy gaps / poach | Category need under your scheme | Waiver wire certainty |
| IceCast trials / sealed showcases | Scenario stress + evidence route | Injury or start confirmation |
| Cap / window docs | Organization-health *model* work | Fully sealed GM product |

## Honest limits

- Bundled seasons cover an immediate offline start; **full 1987–present depth**
  needs `icelines data install` (see getting-started guide). Docs sometimes say
  “38 seasons bundled” in marketing voice — prefer VTRACE/data-status for truth.
- MoneyPuck is optional enrichment, not required for core NHL stats.
- Multi-surface parity is an architectural requirement with ongoing gates — if
  CLI and Web disagree, file it; don’t assume one surface is silently authoritative.

## Good first questions IceLines can answer

- Who leads U23 centers by PPG on current loaded data?
- How does Edmonton’s depth chart look under the built-in model?
- Which free-agent-shaped names help hits/blocks under my scheme?
- What does a morning-skate style card surface emphasize today?

## Bad first questions (wrong tool)

- Legal salary-cap compliance advice for a real club.
- Live in-game betting recommendations.
- “Is this player injured?” as medical fact from a scenario JSON.
