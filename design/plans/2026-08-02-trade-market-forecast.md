# Trade Market Forecast — Implementation Plan

1. Add a pure core draft-pick calibration contract with mature-cohort and
   temporal leakage gates, monotone fitting, slot distributions, time discount,
   and uncertainty.
2. Add team preference, player asset, package utility, mutual-benefit, and pick
   balancing primitives. Keep availability evidence and transaction feasibility
   separate from hockey fit.
3. Build a fetch adapter that joins official draft identity to bundled career
   outcomes and seals a reusable curve artifact. **Complete:** the initial
   all-player GP curve uses terminal 2005-2018 draft ledgers, bundled skater
   bios, official goalie bios, and explicit zero outcomes.
4. Add sourced contract, clause, pick-ownership, and availability overlays.
   Missing authority must remain unknown rather than modeled as clear.
   **Core assembly complete, including retained-salary cap math and slot gates;
   provider population remains in progress.**
5. Add `icecast trade-market` as a thin adapter producing one UI-neutral view.
   **Complete:** direct evaluation and raw authority-backed assembly share the
   same view; `trade-market-assemble` accepts a sealed pick curve. Evaluation
   rejects mixed player/pick value bases and exposes next-season standings
   impact as a separate axis.
6. Generate candidate packages from depth needs and surplus, then apply cap,
   roster, clause, and mutual-utility gates before ranking.
   **In progress:** a sourced NYR/NSH retained-salary assembly now exercises the
   complete path; automated candidate generation and paired lineup effects are
   still pending.
7. Feed each surviving proposal into paired season simulations so points,
   playoffs, Cup odds, lineup, and prospect-pipeline costs share one scenario.
   **In progress:** core and CLI now attach same-seed paired season results for
   both clubs and disclose residual error versus isolated impact. Core and CLI
   also optimize explicit multi-position buyer/seller lineup inputs and report
   trade removals separately from competition displacement. **Complete:** a
   rich change now feeds those exact dressed assignments into the generic
   lineup primitive, preserving incumbent score/role evidence while rebuilding
   all even-strength lines, goalies, PP1/PP2, and PK1/PK2. The CLI can explicitly
   select the top retained training-camp branch when it is the richer baseline.
   Chemistry, matchup deployment, and pipeline cost remain.
   **NYR checkpoint:** the existing lineup projection plus the disclosed
   O'Reilly score-60 scenario dresses him at C and displaces Joe Veleno; this is
   a lineup-score result, not a standings-points conversion. **Paired-season
   checkpoint:** archived season artifacts can now be rehydrated with explicit
   baseline-v1 parameters. Same-schedule, same-seed 10,000-trial runs attach
   full NYR/NSH O'Reilly and NYR/PIT Rust deltas to the evaluated packages and
   to their actionable board rows.
   **Candidate-board checkpoint:** O'Reilly, Rust, Boeser, DeBrusk, and Vatrano
   now run through one baseline and one full-lineup rebuild. Hockey and
   actionable ranks are independent. Authority-complete retained O'Reilly and
   Rust/Veleno-plus-pick packages currently receive actionable ranks; protected
   Vancouver targets and Vatrano have complete failed-package evaluations and
   remain blocked specifically on destination authority. Availability and
   executable feasibility are now separate board fields.
8. Backtest historical deadlines and calibrate completion probabilities before
   promoting likelihood language beyond scenario ranking. **Initial core
   primitive complete:** `trade_completion_calibration.v1` validates reviewed,
   point-in-time labeled proposal cohorts and reports Brier score, log loss,
   equal-width reliability bins, and expected calibration error. It rejects
   future evidence and refuses to manufacture failed-proposal labels from absent
   transactions. CLI ingestion and a reviewed historical proposal corpus remain.
9. Discover complementary buyer/seller fits and generate bounded negotiation
   ladders. **Initial core and CLI complete:** `trade_scout.v1` ranks supplied
   targets without inventing availability, excludes protected assets, publishes
   opening/fair/maximum packages and a walk-away boundary, and leaves generated
   packages blocked pending execution authority. A Seattle control board
   protects Catton, Wright, and injury-obscured Firkus; destination evidence
   removes Larkin despite his hockey fit. League-wide source ingestion and
   learned player values remain.
10. Normalize all organizations into automatic discovery. **Core and CLI
    boundary complete:** `trade_scout_league.v1` derives targets from buyer
    needs plus seller surplus/availability, derives buyer picks/prospects and
    surplus roster assets, preserves protection, and publishes explicit
    complete/partial coverage. The Seattle control inventory proves 4/32
    partial coverage and derives Rust/O'Reilly while rejecting below-threshold
    Boeser. Mainline roster/prospect adaptation is completed in step 11; live
    contract, availability, and pick-ownership provider population remains.
11. Populate normalized inventory from mainline IceLines sources. **Core and CLI
    complete:** `trade_scout_population.v1` reuses the all-team training-camp
    forecast as roster/prospect authority, applies an explicit score/value/role
    translation policy, and accepts separate dated availability, protection,
    and pre-valued pick overlays. `trade-scout-populate` writes the reusable
    population document and can immediately write the evaluated league board.
    Live availability and pick-ownership provider population remain source work;
    execution authority remains downstream in Trade Desk.
12. Populate current future-pick capital. **Initial source/core/CLI path
    complete:** a reviewed provenance-required CSV distinguishes unconditional,
    conditional, and encumbered rights; `trade-pick-populate` values only
    unconditional picks through the sealed curve; and `trade-scout-populate
    --pick-assets` merges them into the buyer inventory. The Seattle control
    now carries seven valued 2027 rights, protects its own first, and leaves the
    lower-of-Columbus/Winnipeg second-round condition unresolved. **Slot-model
    checkpoint:** season simulations now retain every team's trial-level league
    rank distribution, and `trade-pick-populate --season-forecast` values each
    right from its original team's pre-lottery standings-order proxy. Seattle's
    own 2027 first now projects earlier and more valuable than the Tampa Bay
    first it owns. **Draft-order checkpoint:** the simulator now emits separate
    Round 1 and Rounds 2-7 slot distributions. The first applies two weighted
    lottery draws and the ten-place limit; both incorporate the actual simulated
    playoff stage, division-winner ordering, finalist, and champion under an
    explicit season-scoped ruleset. Full league provider acquisition remains.
13. Audit all-team pick chain of title. **Coverage primitive and CLI complete:**
    `trade_draft_pick_ownership_coverage.v1` measures a target year against all
    224 original-team/round coordinates, rejects duplicate claims, and keeps
    coordinate completeness separate from offer readiness. The current reviewed
    Seattle evidence covers 8/224 coordinates (seven unconditional, one
    conditional), so the league board correctly remains incomplete. Automated
    PuckPedia public-page acquisition is blocked by its browser challenge;
    licensed API access or reviewed snapshots are the next provider input, not
    a reason to assume native ownership for the remaining 216 coordinates.
14. Value structured conditional rights without making them executable.
    **Initial primitive complete:** `earlier_of` and `later_of` contracts require
    two explicit original teams and never parse prose into logic. Candidate
    round distributions produce an independence estimate and aligned/opposed
    dependence sensitivity. Seattle's later-of Columbus/Winnipeg 2027 second is
    now visible as indicative value but remains unresolved and offer-ineligible.
    **Protection-chain checkpoint:** ordered multi-year legs now expose reach,
    conveyance, conditional leg value, future-year discount, and blended value;
    terminal conveyance is mandatory. Future years use a declared current-team-
    strength persistence proxy. **Compound-condition checkpoint:** any chain leg
    can now apply an explicit two-team `earlier_of`/`later_of` selector before
    protection. Central and dependence-sensitivity values remain separate, all
    probability must still reach an unprotected terminal leg, and the right
    remains offer-ineligible. A synthetic CLI fixture proves selected first-year
    conveyance plus protected deferral conserves total probability end to end;
    it is intentionally separate from reviewed ownership evidence.
