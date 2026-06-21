# Phase Capitals - Signals cache promotion gate

> Phase Capitals decides whether Signals can move from inspected player/roster
> surfaces into durable shared cache, catalog, filter, or leaderboard surfaces.

**Created:** 2026-06-20
**Status:** Active - pulse 03 catalog/filter/leaderboard gate passed

---

## Frame

Phase Hurricane shipped Signals to real user surfaces without promoting them
into `StatId`, filters, leaderboards, or analytics cache. Phase Rangers then
added a team-scoped Signals roster matrix and explicitly kept it outside the
WP-009 analytics cache envelope until a later cache-promotion gate.

Phase Capitals is that gate. It should capitalize Signals only where the
methodology, source-state, invalidation, unavailable-state, and product-copy
contracts are strong enough. If the contract is not strong enough, the phase
should record durable non-promotion wording instead of forcing Signals into a
shared surface prematurely.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Capitals Goal 1 - Signals promotion inventory** | Existing Signals work spans CLI, TUI, Web, Markdown, and roster discovery, but prior phases intentionally blocked cache/catalog/filter/leaderboard promotion. | A wave inventory lists current Signals surfaces, existing methodology/copy fences, and the exact blockers inherited from Hurricane/Rangers. |
| 2 | **Capitals Goal 2 - Cache metric eligibility decision** | WP-009 cache records require stable metric keys, source-state, invalidation, methodology, disclosures, and consumer semantics. | The phase either defines accepted Signals cache metric keys and fixtures or records why Signals remain direct `PlayerSignalsView` consumers. |
| 3 | **Capitals Goal 3 - Catalog/filter/leaderboard decision** | `StatId`, `--filter`, and leaderboards imply stable comparable stats, which may overstate scorer-biased descriptive Signals. | The phase either promotes a deliberately bounded subset with tests or keeps Signals outside `StatId`, filters, and public cross-team ranking. |
| 4 | **Capitals Goal 4 - Product-copy and unavailable-state gate** | Missing Signal evidence must never read as zero-value truth, prediction, deployment guidance, betting edge, injury signal, or player-quality grade. | Tests or docs prove unavailable states and non-claim copy survive any promoted surface. |
| 5 | **Capitals Goal 5 - Surface matrix closeout** | The active surface matrix should distinguish promoted Signals contracts from durable deferrals. | `design/specs/surface-parity.md` records the final Capitals decision without reopening Hurricane/Rangers blocked source claims. |

---

## Non-goals

- Do not add MoneyPuck deployment columns without pinned upstream schema
  evidence.
- Do not add goalie GSAx or high-danger save percentage without a verified
  goalie xGA/danger source.
- Do not add team confidence bands without a team-level ViewModel/source
  contract.
- Do not turn Signals into prediction, betting, injury, deployment, player-grade,
  or autonomous coaching recommendations.
- Do not promote every Signal everywhere by default. Promotion must be explicit,
  scoped, and tested.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Record current Signals surfaces, prior
   non-promotions, and promotion questions.
2. **Pulse 02 - Cache eligibility gate.** Result: Signals are not eligible for
   WP-009 analytics cache publication yet. They remain uncached
   `PlayerSignalsView` projections until accepted Signal cache metric keys,
   source-state, invalidation, and methodology versioning exist.
3. **Pulse 03 - Catalog/filter/leaderboard gate.** Result: Signals are not
   eligible for `StatId`, `--filter`, or public leaderboard promotion yet.
   `signals-roster` remains a team-scoped inspection matrix, not a ranking
   surface.
4. **Pulse 04 - Promotion or durable deferral implementation.** Add the accepted
   contract and focused tests, or record the durable no-promotion fence.
5. **Pulse 05 - Closeout.** Update the wave, plan, and surface matrix.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Cache or catalog changes require focused Rust tests for metric keys,
  unavailable states, and non-claim copy.
- Route or CLI changes stay offline and fixture-backed.
- Child repo commit and push first; TRACKER records only the submodule pointer.
