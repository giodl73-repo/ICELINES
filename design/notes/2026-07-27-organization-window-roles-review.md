# The Window — role review

**Date:** 2026-07-27
**Reviewed:**

- `design/specs/organization-window.md`
- `design/plans/2026-07-27-organization-window.md`

**Role source:** `.roles/ROLE.md` plus HART, KEEL, TAPE, FORGE, PACE,
BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and broadcast.

## Verdict

**GO WITH CHANGES — all review findings are incorporated in the draft.**

The 2026-07-28 consolidation review below supersedes this planning verdict for
the current implementation state while retaining the original findings as the
design record.

The architecture is sound after revision: typed profile observations feed a
sealed declarative Frame, hierarchical core scoring produces one complete
league board, and all surfaces consume the same UI-neutral documents. Weight
customization does not mutate hockey logic; method changes create new versions.

The initial review found 5 blockers, 5 warnings, and 2 notes. All 12 items were
applied before this verdict.

## Findings and applied fixes

| Role | Severity | Triggering draft text | Finding | Applied fix |
|---|---|---|---|---|
| HART | BLOCKER | “organization / season / season_type / as_of / horizon” | A free-form team abbreviation would not fully identify a historical or relocated organization, and the axes had to survive every cache and artifact. | Added typed season-aware organization identity/version, join validation, and the complete cache key. Composite state stays outside `StatsRepository`. |
| KEEL | WARNING | “older saved boards remain readable or fail” | Cross-version behavior was an outcome but not a contract. | Added exact major-version support, additive-field defaults, refusal, immutable migration, and method-version rules. |
| TAPE | BLOCKER | “organizations[32]” | A hardcoded 32-row schema is wrong for pre-expansion historical replay and can conceal an incomplete source cohort. | Replaced it with expected season-canonical organizations; current NHL runs require 32, historical runs require that season's complete catalog. Added profile cohort gates. |
| FORGE | BLOCKER | “deterministic canonical fingerprints” | Floating-point, map-order, locale, and negative-zero behavior were underspecified and could produce cross-platform fingerprint drift. | Specified canonical serialization, finite-number validation, normalized negative zero, stable ordering, and cross-platform golden vectors. |
| PACE | BLOCKER | “contribution_cap” | Runtime clipping or ambiguous per-profile caps would make the formula non-reviewable. Multiple horizons also risked one universal weight set. | Replaced clipping with manifest-time signal-family weight caps and made each Frame own one primary decision horizon. Added explicit degenerate-cohort behavior. |
| BENCH | WARNING | “Required tests include…” | The test list needed source assembly, schema compatibility, platform fingerprints, and current-vs-historical team-count fences in addition to pure scoring tests. | Expanded W1/W2/W8/W9 gates and retained L0/L1/L2, parity, no-network, historical replay, and package verification. |
| EDGE | BLOCKER | “weights sum to 1 within a small tolerance” | NaN, infinity, all-zero budgets, no eligible teams, zero-variance cohorts, and incomplete team sets could escape ordinary happy-path validation. | Added fail-closed numeric validation, explicit zero-variance neutral handling, minimum cohorts, incomplete-board rank withholding, and degenerate-budget rejection. |
| WIRE | WARNING | “unsupported schema or method version: hard error” | The boundary needed a compatibility/migration matrix and source-artifact validation before scoring. | Added saved-document compatibility, immutable migrations, upstream schema/version checks, and provider dependency declarations. |
| SCOUT | WARNING | “Development system — open jobs and recalls” | A weak NHL roster can create opportunity that looks like excellent development; prospect strength, conversion, and open roster spots must remain distinct. | Kept prospect strength, conversion, NHL opportunity, deployment, and current roster quality in separate signal families and explicitly barred rewarding weakness merely for open jobs. |
| GLASS | NOTE | “32-row board … dozens of profiles” | Showing every profile at once would bury the decision and make multi-horizon state ambiguous. | Board stays compact; detail drills into panes/lines/evidence, selected horizon is explicit, and narrow terminal/browser behavior is gated. |
| CREST | NOTE | “overall score” | A giant gauge would make an opaque master number the product and push evidence below the fold. | Explicitly rejected the giant-gauge treatment in favor of a hockey-native board and pane hierarchy with screenshot review. |
| broadcast | WARNING | “--manifest file” plus Web routes | A local file selection cannot become a stable bookmarkable Web GET state; immutable artifacts also need suitable HTTP caching. | Web exposes registered Frame IDs/fingerprints, keeps all context in the URL, adds semantic HTMX/no-JS behavior, and plans ETag/conditional GET for fingerprinted JSON. |

## Reconciled tensions

### PACE versus SCOUT

The scorer owns explicit quantitative rules; hockey context cannot override a
measured value after the fact. Context may remain evidence-only or enter through
a separately calibrated, versioned profile. This preserves both statistical
auditability and hockey meaning.

### GLASS/CREST versus PACE

The compact board may summarize, but the sealed document retains raw values,
weights, confidence, coverage, and methodology. Detail and The Insider expose
the full explanation without forcing every number into the first screen.

### KEEL versus CREST/broadcast

All surfaces consume one board and one card projection. TUI, CLI, Web, and
reports may compose that material differently for their medium, but none may
change scoring, focus before sealing the league cohort, or lose active context.

### Extensibility versus compatibility

Users alter weights through validated Frames. Developers add typed providers
and descriptors. Existing profile methods and sealed boards remain immutable;
new formulas receive new method versions and historical comparison requires an
explicit bridge.

## Implementation review gates

The roles must review again at four boundaries:

1. W0 inventory and readiness classification — HART/KEEL/TAPE/SCOUT/PACE.
2. W1-W2 contracts and scorer — HART/FORGE/PACE/BENCH/EDGE/WIRE.
3. W3-W7 real all-league board and calibration — all core roles.
4. W8-W9 surfaces and release — KEEL/BENCH/GLASS/CREST/broadcast plus final
   all-role closeout.

Review complete. 0 unresolved blockers, 0 unresolved warnings, 0 unresolved
notes.

## Implementation checkpoint review

The roles reviewed the implemented baseline against the plan after the first
contracts, scorer, adapters, history/scenario/calibration primitives, and
CLI/TUI/Web/card projections landed.

**Verdict: GO FOR CONTINUED IMPLEMENTATION; NOT YET A FULL RELEASE.**

| Role | Checkpoint finding | Plan disposition |
|---|---|---|
| HART | The board identity and axes are sealed, but bridge/rebase remains a first-class missing contract rather than a renderer concern. | Keep W5 open and require an immutable bridge artifact. |
| KEEL | Shared board/card values now reach CLI, TUI, Web, and JSON; report output and full-board interaction are not yet converged. | Keep W8 partial and add parity evidence before release. |
| TAPE | The saved all-32 artifact is honest but source-incomplete; absence must never be inferred as zero or average. | Keep it labeled evaluation-only and rank-withheld. |
| FORGE | Core owns formulas and renderers project typed documents, preserving crate boundaries. | Retain pure-core/no-I/O and thin-surface gates. |
| PACE | Deterministic scoring is testable; predictive language is still unsupported without rolling-origin evidence. | Keep W7 open and label the Frame descriptive/heuristic until calibrated. |
| BENCH | Focused core, TUI, and Web tests cover the present slice; report, compatibility matrix, full parity, and package gates remain. | Add them to W8-W9 closeout rather than treating compile success as release evidence. |
| EDGE | Required missingness and incompatible comparisons fail closed; alterations now need explicit lanes so saved artifacts cannot drift silently. | Added the extension and alteration protocol to the plan. |
| WIRE | Schemas and HTTP routes exist, but compatibility and migration require multi-version fixtures. | Keep W9 open with refusal and additive-compatibility cases. |
| SCOUT | The first Frame keeps prospect strength, conversion, opportunity, and NHL quality distinct; future Lines still require hockey reasonableness checks. | Require SCOUT on profile additions, formula changes, and Frame reweights. |
| GLASS | Focused cards have a usable hierarchy, but the 32-team board and partial/blocked recovery states need surface review. | Keep board/drilldown, 80-column, and accessibility gates in W8. |
| CREST | The product avoids a giant master gauge; screenshot-level polish across board, detail, and report is still unproven. | Require final screenshot review without moving logic into surfaces. |
| broadcast | Stable Frame URLs and ETags are present; bookmarkable as-of/view state, narrow layout, keyboard flow, and no-JS recovery need live review. | Keep browser acceptance work in W8-W9. |

### Reconciled decision

The implementation may continue from the existing baseline without redesigning
the core contracts. The next critical path is W5 bridge/history, W6 typed
scenario attribution, and W7 rolling-origin calibration. W8 can mature in
parallel against sealed fixtures, but no surface may promote an evaluation
artifact into a complete or predictive claim. W9 closes compatibility,
extension, parity, and release evidence.

Checkpoint review complete: 0 unresolved architecture blockers; 4 intentional
release blockers remain tracked in W5, W6, W7, and W8-W9.

### W9 trust-boundary checkpoint

The hardening pass now replays every loaded board through the canonical scorer
before CLI, card, Web, TUI, comparison, or calibration code may trust it. A
valid checksum is insufficient: manifest identity, cohort shape, numeric
bounds, raw observations, normalized values, aggregates, classifications,
drivers, blockers, and rank state must reconcile. BENCH/WIRE compatibility
fixtures also prove older movement and scenario documents deserialize when the
new bridge and attribution fields are absent.

Affected production targets pass strict Clippy, all nine Window schemas pass
Draft 2020-12 meta-validation, the saved board and both cards validate, and the
25-test core Window slice plus focused CLI/TUI/Web tests pass. The extension
fixture registers a new method, adds it to a custom Frame, and scores all 32
organizations without scorer or renderer changes. This closes the
implementation trust-boundary blocker, but does not close the four product
evidence blockers above: real checkpoint history, real scenario distributions,
real rolling origins/holdout evidence, and automated full cross-surface golden
parity remain.

The subsequent W9b pass added a fixed-hash Windows/Linux/macOS CI matrix,
offline Window release smoke, verified Windows archive/checksum/manifest,
dependency audit, and an optimized performance baseline. Live Edge review at
1440px, 900px, and 390px exposed and fixed missing board skip/caption/focus
semantics plus narrow fingerprint/card overflow. Five regenerated screenshots
then passed dimension/nonblank checks and manual hierarchy/containment review.
The subsequent keyboard run verified first-Tab skip links and Enter transfer to
the main focus target on both board and card routes. Reduced-motion emulation
reported no spinner animation and 0.01ms fallback durations; computed layout
reported no page overflow at 390px. The remaining visual gate is automated full
cross-surface golden parity, not first browser or interaction evidence. PR #23
then observed the canonical fixed hashes and replay test passing on all three
operating systems; that closes the platform-fingerprint observation, while the
full PR workflow passed 22 of 22 checks. The real-history, package-matrix, and
full-parity blockers remain explicit.

### W6 real-scenario checkpoint

The later W6 closeout added a real 32-team 2026-27 baseline from sealed
team-season, lineup, and training-camp documents, plus a sourced NYR
development/downturn scenario and paired same-seed isolated event evidence.
The combined 1,000-trial Window distribution also propagates a mean-centered
SEA camp outcome through the full scorer. TAPE/EDGE review found and fixed a
partial-pane panic, added explicit inactive outcomes, enforced registry
`scenario_support`, and separated authority fingerprints from numeric-estimate
fingerprints. PACE keeps the result descriptive: missing panes and ranks remain
withheld, percentile boundaries can produce zero aggregate movement from a
real raw change, and the shock assumptions are not labeled calibrated. This
closes the real-scenario evidence blocker without changing the W3-W4 production
coverage, W7 future-holdout, package-matrix, or cross-surface parity gates.

### W8-W9 parity and package checkpoint

The automated golden now proves exact sealed board/card equality across CLI
JSON, TUI bundles, Web JSON, and the card API, then checks the renderer contract
for score, rank-withheld state, confidence, and coverage in both TUI and focused
HTML. The PR workflow also packages and verifies Linux x86_64, Windows x86_64,
macOS ARM64, and macOS x86_64 archives using the tagged-release shape. W8 is
complete; W9 still requires observing all four new package jobs green before
release closeout.

PR #23 run `30351304523` subsequently passed all 26 checks. Linux x86_64,
Windows x86_64, macOS ARM64, and macOS x86_64 each built, archived,
checksummed, inspected, and uploaded successfully. The existing release smoke,
coverage, quality, test, and three-OS fingerprint jobs were green in the same
run. This closes W9's package observation and release-evidence gate; it does
not alter the explicit incomplete-source or future-holdout product status.

### W5 real-checkpoint checkpoint

TAPE/PACE/HART reviewed the existing sealed Jan. 31, Feb. 28, and Mar. 31,
2025 IceCast history as a point-in-time authority. The new core adapter projects
it into three comparable all-32-team Window boards under a dedicated
`icecast_forecast_history.v1` manifest containing only
`nhl.expected_points@icecast_expected_points.v1`. This closes the real
within-season checkpoint gap without inventing observations for the other
panes. Personnel attribution remains `null` and W5 remains partial until a
dated typed personnel authority supports that causal decomposition.

HART/EDGE then reviewed the attribution trust boundary. The resulting v1 input
requires dated events, a scenario board, and typed authorities whose identity,
source fingerprint, and organization scope agree. Core replays the supplied
movement from both boards and recomputes the typed scenario before setting any
`personnel_delta`; tampered movement, out-of-window events, unmatched
authorities, duplicate events, and unsupported sources fail closed. PACE keeps
the value labeled a counterfactual estimate and assigns all unexplained change
to the residual. A real paired historical scenario remains the W5 evidence
gate.

### W5 paired-personnel closeout

HART/TAPE required the estimate basis to be explicit rather than inferred from
optional boards. The final contract supports an earlier scenario or a later
counterfactual and requires exactly the board matching the selected basis. It
also replays the supplied movement, binds every dated event to a typed
authority, and retains profile-level effects in the attributed movement.

PACE/EDGE reviewed the real Jan. 31 -> Feb. 28, 2025 paired rolling replay.
The counterfactual retains personnel evidence through Jan. 31 and omits only
later evidence while preserving the later game checkpoint, 1,000 trials, and
seed `20242025`. The interval contains 219 dated events. Eleven organizations
show a nonzero raw expected-points effect, but none crosses a cohort-percentile
boundary, so all aggregate personnel score deltas are zero. A compact typed
summary preserves the raw effects and explicitly refuses causal or calibrated
language.

BENCH/WIRE required tests for both estimate bases, movement tampering,
out-of-window evidence, source/scope matching, schemas, and CLI parsing. They
also required numeric-tolerant semantic replay after JSON serialization while
keeping metadata exact; this fixes false refusal from floating-point round
trips without weakening deliberate tamper detection.

This closes W5. It does not promote the narrow NHL-strength history to a
complete multi-pane production board, and it does not satisfy W7's future
untouched holdout. Those remain the two product gates for the broader Window.

### W3-W4 source-package checkpoint

FORGE/KEEL rejected filesystem paths as a core contract. The implementation
instead seals the owned upstream documents into
`organization_window_source_package.v1`; CLI path resolution is a thin edge,
while canonical sorting, nested authority validation, fingerprint replay, and
package-to-board adaptation remain in core.

TAPE/WIRE require the package axes to be `regular`, `nhl_32.v1`, and one season
and cutoff. Duplicate teams, mismatched nested schemas or axes, and fingerprint
tampering fail closed. BENCH added package replay, schema, tamper, CLI parse,
and incomplete-board refusal tests.

TAPE also rejected board `league_coverage` as an acquisition-completeness
proxy: an all-32 cohort can still have only a subset of profile authorities.
The separate source-coverage document counts observations and score-eligible
values for every configured profile, lists exact missing organizations, and
reports both required-profile completion and rank-eligible organization count.

PACE keeps partial packages valid for evaluation because honest missingness is
a supported product state. Production publication is different:
`--require-ranked` refuses to write unless every organization has an eligible
rank under `balanced.v1`. W3-W4 remain open until a real all-league package
passes that gate; the package contract prevents acquisition work from changing
the scorer or surface contracts.

FORGE/TAPE then reviewed all-league assembly. Reopening 32 roster/statistics
caches per team was rejected; the edge now loads configuration, snapshot store,
and statistics authority once and core receives the resulting typed lineup
documents. Schedule fatigue likewise derives in core from the sealed
`team_game_forecast.v1` game rows rather than from CLI calculations. Partial
forecasts create only represented-team profiles, duplicate games or teams fail
closed, and the source audit remains the authority on whether league coverage
is genuinely complete.

FORGE/KEEL applied the same rule to affiliate composition. The CLI may resolve
files and caches, but it does not rebuild NHL/AHL lines or recall ladders.
Sealed `ahl_affiliate_projection.v1` documents are paired with matching NHL
lineups in core and passed through The System's existing validation. TAPE keeps
unmatched teams missing and rejects explicit and derived organization lineups
for the same club as competing authority. The available full 2025-26 AHL
snapshot is acquisition evidence, not permission to copy provider-local IDs or
invent 2026-27 assignments; reviewed league crosswalk and projection facts
remain the production gate.

### Frozen strength and special-teams acquisition checkpoint

TAPE found that a sealed preseason `team_season_forecast.v1` carries stable
home/away strength on every game even though `opening_strengths` is reserved
for dated replay. Core now derives `nhl.team_strength` only when those game
features are undated and stable for the team; any dated or rolling evidence
suppresses the fallback. EDGE also found that partial-league season simulation
could reach playoff indexing and panic. The simulator now fails explicitly
unless all 32 canonical NHL teams and eight teams per division are present.

FORGE then traced empty special-teams units past the lineup builder to an
unfinished Lindsay boundary: IceLines fetched official
`skater/timeonice`, but `load_into_repo` never merged it into
`SeasonStats.time_on_ice`. The production loader now uses the existing Tier-1
file and season fence, joins rows by canonical NHL player ID, and preserves
missing-versus-zero semantics. BENCH added a repository-path regression proving
both the populated and absent cases.

The same real 32-team replay moved from 8/16 to 10/16 complete required
profiles. Power-play depth and penalty-kill depth each have 32 observations
and 32 values. The audit still reports 0/32 rank-eligible organizations, so
PACE keeps the board evaluation-only while six required profiles remain
incomplete. No AHL identity, prospect, or future-holdout evidence was inferred
to improve that result.

## 2026-07-28 consolidated plan and extensibility review

**Reviewed:** the current implementation, the 13/16 real source audit, the
prospect-cohort correction, and the consolidated delivery tracks in the plan.

**Verdict: GO FOR THE CONSOLIDATED PLAN. KEEP PRODUCTION RANKING AND PREDICTIVE
CLAIMS GATED.**

The role panel agrees that the architecture is already extensible in the right
places. The remaining work is primarily authority assembly and evidence, not a
new scoring engine or renderer rewrite. The plan now separates source
completeness, predictive evidence, and extension maturity so success in one
cannot silently promote another.

| Role | Finding | Plan response |
|---|---|---|
| HART | Prospect, affiliate, goalie, season, and cutoff identities must remain typed axes; an automatic convenience path must produce the same canonical artifacts as the explicit path. | S1 reuses typed builders and S2 requires season-aware affiliation/assignment facts. New methods or schema semantics version rather than mutate existing boards. |
| KEEL | A new convenience command is acceptable only if core/fetch/CLI and all renderers still converge on one package and board. Fantasy and simulation must consume Window outputs rather than create sibling formulas. | Fetch owns source composition, core owns hockey logic, CLI only orchestrates, and the extension backlog explicitly routes new consumers through sealed artifacts. |
| TAPE | The 13/16 audit is honest. BOS/UTA goalie gaps and all-league AHL identity gaps cannot be repaired with stale rosters, guessed affiliations, or averages. | S2 requires reviewed crosswalks; S3 either introduces a typed, newly versioned translation authority or preserves missingness. Proxy filling is forbidden. |
| FORGE | The next code slice should be a small library composition API with thin CLI wiring, explicit conflicts, typed errors, and no repository-relative runtime dependency. | S1 defaults through configured cache paths, permits an explicit override, and composes existing builders instead of duplicating them in `main.rs`. |
| PACE | Production-ranked, calibrated, and custom are different statistical claims. A 16/16 board is not thereby predictive, and a new goalie scale cannot inherit v1 semantics. | The three delivery tracks and product labels are explicit. S3 requires a method-version decision; the future holdout is an independent calendar gate. |
| BENCH | Parity between the explicit and automatic prospect paths is more valuable than another happy-path snapshot. Crosswalk and goalie decisions need adversarial fixtures. | S1 acceptance requires artifact-semantic parity, order invariance, offline execution, exclusions, and FLA coverage. S2 covers shared affiliates, trades, loans, relocation, ambiguity, and missing teams. |
| EDGE | Likely failures are duplicate authorities, camp/artifact conflicts, missing birth dates, one-goalie rosters, provider-ID collisions, shared affiliates, and a future result accidentally used during tuning. | The plan calls for explicit option conflicts, typed exclusions, ambiguity failures, conservative goalie missingness, and a pre-frozen untouched holdout protocol. |
| WIRE | Cache defaults and overrides must be part of a stable CLI contract; nested artifacts need schema/fingerprint validation before assembly. Source replacement may require a new method version even when JSON still parses. | S1 seals source fingerprints and keeps explicit overrides. The extension protocol requires source dependency/version declarations and compatibility fixtures. |
| SCOUT | Rookie eligibility is a legitimate candidate-pool signal, but it is not proof of readiness. AHL recalls and goalie projections must preserve hockey roles and uncertainty. | Candidate selection remains separate from exact age/workload filters and prospect scoring. S2/S3 keep assignment, readiness, and translated projection evidence distinct. |
| GLASS | Users need one legible readiness state, not internal workstream jargon or a falsely precise rank. Missing evidence should tell them what can be done next. | Surfaces continue to show rank status, coverage, blockers, and evidence. The four product labels provide compact, consistent language across CLI/TUI/Web/cards. |
| CREST | Extensibility should not turn The Window into a wall of profiles or generic dashboard controls. Official Frames need editorial restraint. | New Lines enter the registry first, not `balanced.v1`; admission requires a named decision benefit and ablation/sensitivity evidence. Pane hierarchy stays primary. |
| broadcast | Saved Web views require stable Frame/artifact identity and bookmarkable context. A local cache path or uploaded manifest cannot become hidden browser state. | Cache composition remains an acquisition/CLI concern. Web consumes registered, fingerprinted boards and keeps season/as-of/Frame state in the URL. |

### Reconciled decisions

1. **Automate composition, not judgment.** S1 may automate deterministic joins
   over reviewed inputs. S2 identity resolution and S3 method selection retain
   explicit human review where the source cannot establish the fact.
2. **Do not make 16/16 the only useful state.** Evaluation artifacts remain
   first-class and inspectable, while `--require-ranked` protects official
   ranking publication.
3. **Do not make 16/16 a predictive claim.** Calibration is target-specific and
   remains open until a genuinely later untouched holdout is scored.
4. **Prefer additive extension.** New Lines and Frames reuse the registry and
   scorer; formula/source-semantic changes get new method versions; saved boards
   remain immutable; unlike boards need a reviewed bridge.
5. **Keep official Frames curated.** Computability is necessary but not
   sufficient. Each addition needs source authority, leakage safety, a signal
   family budget, and evidence that it improves a named decision.

### Review gates for the next implementation slice

- HART/KEEL: automatic prospect composition produces the canonical existing
  program/package shapes and introduces no second business-logic path.
- TAPE/WIRE: configured career cache, explicit override, captured cutoff,
  nested schemas, and fingerprints are inspectable and validated.
- PACE/SCOUT: `prospect || rookie_eligible` only broadens the candidate pool;
  exact age, workload, readiness, confidence, and exclusions still govern the
  resulting evidence.
- BENCH/EDGE: FLA's 24-year-old rookie-only case, missing identity, duplicate
  authority, option conflict, order invariance, offline run, and explicit-vs-
  automatic parity are regression-tested.
- GLASS/CREST/broadcast: no surface redesign is needed for S1; the existing
  rank-withheld/coverage language must remain accurate when the package is
  rebuilt.

### S1 cache-native prospect closeout

HART/KEEL verified that the new fetch-owned composition result retains the
typed context, discovery, and program stages while the CLI only selects a cache
path and inserts the resulting canonical program board. No renderer or source
package adapter gained prospect scoring logic.

TAPE/SCOUT accepted the candidate-pool correction from `prospect` alone to
`prospect || rookie_eligible` because exact dated age, official career history,
NHL workload, study eligibility, and the separate 50-game program graduation
boundary still govern inclusion and scoring. Missing birth dates remain typed
context exclusions.

PACE/BENCH/EDGE compared the real cache-native and explicit July 28 paths. Both
produce 32 programs from 162 studies: 94 ranked and 68 graduated. All 96 values
across 32 organizations and the three prospect Lines match on raw value,
normalized score, league rank, and sample size. The full package remains 13/16
required profiles and 0/32 rank eligible. This proves composition parity
without weakening the source gate.

WIRE/FORGE verified the configured cache default, explicit path override,
empty-cache diagnostic, option conflicts, order invariance, focused tests,
447-test fetch suite, and strict production lint. The different source
fingerprint is intentional when the explicit path retains extra overlay
citations; it does not change the Window values.

### S2 all-league identity checkpoint

HART/WIRE verified that the reviewed 2025-26 AHL envelope retains its official
snapshot identity and team-season occurrences while canonical NHL identities
remain a separate reviewed join. Across 1,425 appearances, 1,410 are mapped and
reviewed, 15 are explicitly rejected as mappings, and none remain pending.
Resolved coverage is 100.00%; canonical identity coverage is 98.95%.

TAPE/SCOUT accepted the new league rejection lane because it closes only an
unsupported NHL mapping. It never deletes the AHL person or turns an unmatched,
ambiguous, or collision-scale proposal into a player-quality judgment. Repeated
provider IDs are handled across every pending club occurrence, which preserves
trade history rather than selecting a convenient team row.

KEEL/PACE keep S2 open at the assignment boundary. The fully reviewed prior
snapshot establishes identity evidence, not a 2026-27 affiliate assignment,
projected role, readiness score, waiver fact, or professional-game total. The
existing core contract explicitly forbids equating a camp cut with successful
AHL assignment, so organization and recall depth remain missing until that
season-aware authority is assembled.

HART/KEEL then reviewed the forecast-native league rollover boundary. The
sealed league camp forecast retains all fields rollover actually consumes, and
the adapter produces the same team artifact as the legacy explicit-input path.
Separate dated 2025-26 and 2026-27 affiliation catalogs prevent NYI's
Bridgeport-to-Hamilton move from relabeling historical evidence.

The real 32-team run built every team without a source-binding failure but
found zero projection-ready pools. TAPE/PACE accepted this as a stronger result
than proxy completion: 1,174 prior-only organization statuses, 15 rejected
identity mappings, 144 waiver gates, and aggregate shortages of 357F/171D/59G
are now explicit work queues. The next S2 slice must reduce those queues with
sourced status and professional-game authorities; it may not reinterpret them
as assignments.

FORGE/BENCH then verified forecast-native parity for organization-status draft
and application, plus an exact league envelope over all 1,425 appearances. The
real envelope builds 32/32 children, reports 1,174 required decisions and 15
identity blockers, and has zero source-binding failures. League application is
atomic and reuses each canonical child validator.

EDGE identified an over-coupled failure mode: one explicitly rejected identity
blocked otherwise valid status decisions for the entire affiliate. The gates
are now independent. A mapping rejection still prevents projection readiness
and canonical player joins, while complete evidence-backed decisions for other
players can be preserved in the config.

HART/KEEL reviewed the professional-game boundary next. The league crosswalk is
now a first-class career-cache target, and a versioned policy—not Rust code—owns
exact league inclusion. The first all-league run acquired 1,323/1,323 canonical
histories, then completed 585 totals while withholding 738 whose observed pro
league abbreviations still need treatment. SCOUT/EDGE required the output to
say `within_game_threshold`, not `development_player`: age qualification and
the European CHL-eligibility exemption can change final rule status and must
not be hidden inside a raw total.

WIRE/EDGE required a policy-authority state as well. The provisional exact-
league mapping resolves 1,323/1,323 game totals and separately exposes 8,780
youth-exempt European games across 181 players, but deliberately certifies zero
final classifications. Only a `final` policy can populate that field; publishing
the new-season rule book is an authority change, not a code edit.

FORGE/SCOUT then caught the downstream compatibility issue: the existing core
optimizer classified from 260 games alone and could mislabel an automatically
age-qualified player. The player contract now carries the final reviewed rule
qualification beside its raw total. New composition must provide it; legacy
documents retain the count fallback so saved artifacts do not break.

KEEL/WIRE accepted the official-snapshot application bridge because it is
narrow: only final-policy game and qualification facts are filled, conflicting
authored values fail, and all scoring/assignment/waiver fields remain owned by
their original sources. HART identified that preseason camp-only candidates
need an NHL-ID-keyed league composer because they do not yet have provider-
scoped AHL IDs.

### S2 league facts-workboard review

HART/KEEL accepted the NHL-ID-keyed preseason composer as the correct boundary
for camp-only candidates: provider-local AHL identity remains unchanged, while
the league artifact composes evidence against canonical NHL identity and feeds
the existing affiliate primitive only after its blockers are resolved.

TAPE/WIRE verified that the board distinguishes available raw professional-
game totals from provisional rule authority. It cannot turn the 2025-26 rule
interpretation into a final 2026-27 development classification, and it does not
reinterpret waiver exposure as clearance or a camp cut as assignment.

SCOUT/PACE required exact player eligibility to survive the camp seal. The
rebuild preserves position lists for all 933 camp players, including 26 with
multiple positions. Old forecasts without primary eligibility now fail closed;
the initial 255 missing exact-position blockers were prior-only evidence gaps,
not forward-group guesses.

FORGE/BENCH/EDGE reviewed the composition tests and real 32-team run. Matching
season/schema/cohort checks, duplicate-team/player refusal, provisional/final
rule behavior, goalie treatment, and CLI parsing pass. The resulting 1,371
candidate work queue has zero false facts-ready rows and exposes every remaining
assignment/prospect/recall/status/waiver/score/game/rule authority separately.

GLASS/CREST/broadcast accepted the concise text view because it reports
readiness and blockers rather than simulating certainty. JSON remains the
UI-neutral source for future TUI/Web queues; neither surface may recompute or
silently clear a blocker.

**Verdict:** pass for the workboard slice. S2 remains open for a sourced,
fingerprint-bound facts overlay/application and the resulting all-32 affiliate
and organization-lineup projections.

### S2 facts overlay/application review

HART/KEEL accepted the overlay as an application layer rather than a second
roster model. Organization status remains owned by rollover review;
professional games and final rule qualification remain ledger-owned; only
position, score, prospect, recall, assignment, and waiver facts can be supplied.

TAPE/WIRE required exact workboard binding plus evidence on every finalized
row. Reviewer identity, RFC3339 time, absolute HTTP(S) URLs, notes, duplicate
refusal, and deterministic overlay/result fingerprints are enforced before a
state change.

PACE/SCOUT accepted explicit false values: a player can be reviewed as not a
prospect or not assigned without either fact being confused with missingness.
`not_assigned` removes the player from the candidate pool but makes no departed,
other-league, waiver, or quality claim.

FORGE/BENCH/EDGE verified stale/draft refusal, conflict checks, partial blocker
clearing, readiness bounds, exact position coherence, and an all-32 binary
smoke. The one-row smoke reduces only the expected league queues and is labeled
synthetic rather than retained as source evidence.

GLASS/CREST/broadcast retain one user story: generate a review draft, source
only known facts, apply it, and inspect the blocker delta. Future TUI/Web
surfaces consume the application document and may not infer omitted values.

**Verdict:** pass for overlay/application. S2 remains open at facts acquisition
and facts-ready application-to-projection lowering.

### S2 projection-input lowering review

HART/KEEL verified that lowering targets the existing
`AhlAffiliateProjectionInput` and calls the canonical affiliate projection
builder as its acceptance gate. There is no second lineup optimizer.

TAPE/WIRE accepted provenance retention from team rollover sources, per-player
review evidence, rule source, review timestamps, application fingerprint, and
result-workboard fingerprint. Provisional policy authority is rejected even
when every raw career total is available.

SCOUT/PACE required both hockey-shape and rule feasibility, not merely empty
blocker vectors. The positive fixture dresses 12F/6D/2G with 12 development
skaters; a missing second goalie remains a team failure.

FORGE/BENCH/EDGE verified complete application replay, threshold/rule checks,
deterministic ordering, named partial failures, CLI parsing, and a real all-32
provisional-authority refusal. No production input is emitted from that smoke.

GLASS/CREST/broadcast accepted the league envelope because `teams_requested`,
`teams_built`, and every failure stay visible. Downstream surfaces may focus a
team only after consuming the complete envelope state.

**Verdict:** pass for lowering. S2 remains open for real source review and
32/32 affiliate plus organization-lineup artifacts.

### S2 official-position cache review

HART/KEEL/TAPE accepted official NHL landing `position` as additive identity
metadata in the existing career cache. The ledger carries it through the same
source fingerprint; the workboard consults it only when rollover position is
missing.

SCOUT/WIRE required narrow semantics: landing primary position can resolve a
generic AHL `F`, but it cannot create dual eligibility, assignment, prospect
status, recall readiness, or projected value. Camp-authored exact eligibility
continues to win when present.

BENCH/EDGE/FORGE verified old-cache compatibility, store round trip, fallback
behavior, 1,323/1,323 refreshed positions with zero acquisition skips, and the
real blocker delta from 255 to zero. The 1,174 projected-score gaps remain
untouched rather than borrowing a position-based proxy.

**Verdict:** pass. Exact-position acquisition is closed for the current
all-32 workboard; S2 remains open on score, status, assignment, prospect,
recall, waiver, and final-rule authority.

### S2 AHL player-value review

HART/KEEL accepted the split authority: core owns one reusable typed estimate,
while fetch composes official AHL rows, reviewed canonical identity, and the
existing preseason workboard. The application is not a second optimizer and
can clear only `projected_score`.

TAPE/WIRE required the exact prior-season snapshot, reviewed all-league
crosswalk, explicit policy/method versions, source URLs, and deterministic
fingerprints. Multi-team stints aggregate by canonical NHL ID. Rejected or
unmapped identities do not receive a score.

PACE/SCOUT accepted separate position-specific skater priors and shot-based
goalie confidence, provided the output remains labeled evaluation-only and not
an NHL equivalency or calibrated projection. Missing observations and the one
real position conflict remain blocked rather than receiving defaults.

BENCH/EDGE/FORGE verified short-sample shrinkage, goalie workload behavior,
invalid policy/totals, rejected identity, JSON round-trip stability, saved-v1
workboard compatibility, CLI parsing, and the real application. That real run
scored 1,221 players and filled 1,076 of 1,174 missing values. During the run,
two serialization defects were found and fixed: additive null fields no longer
invalidate old workboard fingerprints, and value-ledger fingerprints now use
an explicit length-delimited canonical contract rather than whole-JSON bytes.

GLASS/CREST/broadcast accepted a concise evaluation label and blocker delta;
surfaces consume the sealed application and may not recompute the formula.

**Verdict:** pass for the player-value slice. Historical calibration is a
separate promotion gate, and S2 remains open for the other sourced facts and
the 98 honest score gaps.

### S2 operational prospect-status review

HART/KEEL separated player population from organization placement. Prospect
status is keyed by canonical NHL player ID and cutoff; organization status and
affiliate assignment remain dated team facts. This allows one classification
to serve Window, camp, NHL/AHL lineups, simulation, and fantasy consumers
without creating a second roster model.

TAPE/WIRE accepted exact birth date and NHL regular-season workload from the
configured official career cache, bound to the input workboard and a versioned
policy. Missing or invalid evidence remains unavailable; no repository-relative
bio overlay or name matching is used. Diff review also caught and closed an
acquisition-boundary bug: raw or nested workboards now pass their complete seal
validator before they can select player IDs for official career fetches.

PACE/SCOUT required the label `organizational prospect`: the 24-year/50-game
boundary defines an IceLines reserve-system population, not NHL rookie
eligibility, waiver exemption, contract status, assignment, or player quality.
One observed graduation axis is decisive, while a positive classification
requires both axes.

BENCH/EDGE/FORGE verified exact age boundaries, partial-evidence semantics,
future-date refusal, stale-ledger refusal, narrow blocker clearing, resealing,
and the duplicate-organization case. The real cohort exposed 81 canonical
players across multiple organization appearances; the regression fixture now
proves they are classified once, applied everywhere, and remain unassigned.

GLASS/CREST/broadcast keep the operational label, method, evidence, and blocker
delta in the UI-neutral artifact. Renderers may focus or summarize it but may
not reinterpret the classification or imply an assignment.

**Verdict:** pass for operational prospect status. The real run classifies
1,282/1,282 canonical candidates and applies to 1,371/1,371 appearances, leaving
zero prospect-status blockers. Recall readiness is the next distinct modeled
authority; it must not reuse camp-make probability or prospect status as a
synonym.

### S2 recall-readiness review

HART/KEEL accepted a separate readiness policy, estimate, ledger, and narrow
workboard application. Readiness remains player-global evidence; organization
assignment, waiver clearance, and recall order remain separate scenario facts.

TAPE/WIRE required exact workboard, career-cache, and camp-forecast binding.
The real saved artifact exposed an in-memory float fingerprint mismatch; the
producer now canonicalizes through its supported JSON wire representation
before sealing. Raw and nested workboards retain their complete seal gate.

PACE/SCOUT approved the 0.50 value / 0.30 NHL-workload / 0.20 camp-proximity
evaluation formula only with a 0.70 minimum coverage gate, explicit component
values, and separate confidence. It is labeled an index rather than a recall
or NHL-success probability. Value is normalized within forward, defense, and
goalie cohorts.

PACE/TAPE also rejected correlated double counting. A separately modeled
prior-AHL value wins over a camp value. When camp supplies the fallback value,
camp proximity is omitted from the same estimate. Cross-organization rollover
rows reuse the canonical choice without deciding assignment.

BENCH/EDGE/FORGE added known-value, missing-component, coverage refusal, tied
percentile, JSON round-trip, stale-ledger, narrow application, and
cross-organization precedence tests. Diff review found and fixed the older
Window adapter's mixed-unit fallback from missing 0..1 readiness to 0..100
projected score.

GLASS/CREST/broadcast retain the index, confidence, coverage, method, evidence,
and missing reason in the UI-neutral documents. Renderers may not relabel the
index as a probability or recompute it.

**Verdict:** pass for evaluation recall readiness. The real run estimates
1,185/1,282 canonical candidates, applies to 1,273/1,371 appearances, and leaves
97 canonical players covering 98 appearances explicitly blocked. Historical
calibration remains a separate promotion gate.

### S2 organization-status evidence review

HART/KEEL required organization status to remain keyed by player,
organization, target season, and evidence time. Unlike prospect and readiness
facts, it is not player-global: the same current-team observation can establish
departure from one prior organization and retention by another.

TAPE/WIRE accepted official NHL landing `currentTeamAbbrev` only as positive,
dated evidence. Missing current team, inactive status, or camp absence cannot
prove a departure or another league. The cache retains per-player observation
time and direct landing URL instead of relying only on the global refresh time.

EDGE/SCOUT separated organization retention from target NHL/AHL assignment,
contract rights, waivers, lineup selection, and recall order. The evidence
application prefills the exact league review but deliberately leaves it draft
and cannot emit `other_league`.

BENCH/FORGE added same-team, different-team, missing-team, identity mismatch,
freshness, stale-review, exact-key, CLI parser, and all-32 execution checks.
The first stale-executable run produced zero resolutions and therefore proved
the no-read path fails closed; the rebuilt reader then completed 1,282/1,282
verified July 25-26 landing acquisitions with zero skips.

**Verdict:** pass for narrow official-current-team authority. It resolves 549
of 1,174 prior-only appearances (425 retained, 124 departed) and leaves all 625
players without a current NHL team unresolved for contract/league research.

### S2 official AHL transaction-source review

HART/TAPE retained AHL provider player and team IDs in the source document and
required the full provider team catalog beside them. Canonical NHL joins remain
owned by the reviewed identity envelope; transaction ingestion cannot bypass
that boundary.

WIRE/FORGE required exact pagination reconciliation and real cache acquisition
times. The first live implementation repeatedly read and rewrote the shared
manifest once per page; the corrected path batches page cachelines with bounded
concurrency and reads the verified manifest once. Every declared result must
appear in exactly one retained source page before sealing.

EDGE/SCOUT prohibited interpreting a raw `ADD`, `DEL`, or missing row inside the
source parser. Descriptions, same-day moves, cutoffs, and destination state
belong to a separate versioned ledger. In particular, a completed prior season
cannot establish the target-season opening assignment.

BENCH validated provider IDs, positions, event types, malformed identity,
team-catalog coverage, page/result reconciliation, CLI parsing, and live
completed/target-season runs. The completed replay contains 4,011 events over
21 pages (2,259 additions, 1,752 deletions). The official 2026-27 catalog has
32 teams and zero events, which correctly resolves no assignment decisions.

**Verdict:** pass for official transaction acquisition; assignment authority
remains open pending the cutoff-aware event-state ledger and new target-season
source events.

### S2 cutoff-aware AHL transaction-state review

HART/KEEL kept the state ledger separate from source acquisition and preseason
facts. The schema carries source season, cutoff, method, source/identity/
affiliation fingerprints, result fingerprint, typed counts, and player rows;
renderers consume it without reproducing event rules.

TAPE/WIRE required provider IDs to remain local until the reviewed crosswalk
join and provider teams to join only through the target dated affiliation
catalog. A prior reviewed identity envelope may identify a target-season
provider player, but prior transactions cannot establish target assignment.

SCOUT/EDGE set conservative hockey semantics: one latest ADD destination may
assign; a simultaneous DEL from a different club may express the move; DEL
without ADD expresses removal from the observed AHL state. Same-team ADD/DEL,
multiple ADD clubs, and unknown kinds cannot be ordered reliably and stay
ambiguous. Transaction absence remains no-read.

BENCH/FORGE added destination, same-team ambiguity, cutoff exclusion, empty
target feed, tamper/rebinding, CLI parse, historical replay, and target replay
checks. The completed season produces 695 assigned, 403 removed, and 63
ambiguous states from 4,011 events; 1,149 of 1,161 states have reviewed NHL
identity. The target season produces zero states from zero events.

GLASS/CREST/broadcast retain method, cutoff, counts, source fingerprint, raw
descriptions, and ambiguity in the UI-neutral document. They may summarize but
cannot reinterpret same-day order or call the output waiver/contract evidence.

The application review then confirmed exact-organization semantics: assigned
state is true only for its destination organization and false for other
candidate appearances; removed state is false; ambiguity and provider-only
state do not write. Each written row retains method, cutoff, provider identity,
source URL, state, and ledger fingerprint. Existing facts must agree.

**Verdict:** pass for cutoff-aware transaction state and narrow workboard
application. The real empty 2026-27 ledger applied zero positive and zero
negative facts and left all 1,371 assignment blockers intact; actual
target-season transactions remain the evidence gate.

### S2 waiver-clearance authority review

TAPE/WIRE audited both transaction sources before approving a new primitive.
The completed ESPN season has 109 placements and 10 claims but zero explicit
clearance events; the AHL feed has no waiver descriptions. Absence cannot be
used as the next-day result. PuckPedia publishes explicit waiver history and
the 10-game/30-day re-waiver rule, while its production API is private, so the
boundary is a reviewed import rather than brittle Cloudflare scraping.

HART/KEEL separated eligibility, placement, clearance, claim, assignment, and
organization. The exact queue is bound to one workboard and cutoff. A finalized
partial review carries result dates, source URLs, reviewer/timestamp, typed
counts, and a canonical fingerprint.

EDGE/SCOUT required a claim to remain assignment-blocking and prohibited the
waiver module from selecting the claiming organization's affiliate. Clearance
removes only `WaiverClearance`; existing assignment and every other fact lane
remain unchanged.

BENCH/FORGE cover non-applicable drafts, explicit clearance, claims, future
dates, missing sources, tampering, and CLI parsing. The real target draft
contains 144 pending rows and zero fabricated resolutions.

GLASS/CREST/broadcast retain pending counts and row-level source/reviewer
provenance. Renderers may call the queue a waiver gate but may not call
placement or eligibility “clearance.”

**Verdict:** pass for camp-time waiver review/application. The July queue is
correctly 0/144 resolved; real clearances remain calendar-gated evidence.

## 2026-07-28 profile-lifecycle and composite-claim review

**Reviewed:** the consolidated implementation plan's add/change lanes, the
implemented v1 registry and saved-board contracts, and the intended use of
multiple-dozen profiles across official and custom Frames.

**Verdict: GO WITH ONE AMENDMENT.** The existing provider/manifest split is the
right extension seam. Add an explicit registry lifecycle in a later atomic
schema slice, and keep organization health, competitive success, and window
timing as separate claims.

| Role | Finding | Applied plan/spec response |
|---|---|---|
| HART | Readiness, per-observation status, and lifecycle are different axes. Reusing one enum would permit invalid states and ambiguous cache identity. | Lifecycle is orthogonal and requires a new registry revision; no v1 wire shape is silently widened. |
| KEEL | Deprecation must behave the same in core, CLI, Web, TUI, cards, history, and replay. A renderer cannot substitute the replacement. | Sealed artifacts retain the exact old method; official-Frame migration creates a new manifest and all surfaces consume it through the shared board. |
| TAPE | A source regression may require immediate demotion without pretending the hockey method ceased to exist. | Source readiness can demote independently; rank eligibility is withheld while historical observations remain attributable. |
| FORGE | The current Rust enum should not be stretched until schema and compatibility semantics are ready. | Lifecycle is a planned atomic slice with typed metadata, validator changes, fixtures, and explicit unsupported-version refusal. |
| PACE | A health percentile, an outcome probability, and a window label answer different statistical questions. | The plan now defines three composite products with separate evidence and validation language. |
| BENCH | Add/change testing is incomplete without old-reader, pinned-Frame, deprecated-method, retired-method, and no-silent-substitution cases. | The lifecycle slice requires compatibility and sealed replay fixtures plus an official-Frame migration audit. |
| EDGE | The dangerous cases are replacement cycles, missing replacements, retirement while an official Frame still references the method, and accidental aliases. | Supersession is an explicit immutable edge; validator/review must reject cycles and unresolved official-Frame references. |
| WIRE | A lifecycle field added to v1 would be ignored by permissive older readers and create split behavior. | The amendment uses a new registry version and explicit reader support/refusal behavior. |
| SCOUT | More profiles do not automatically make a better hockey judgment; official Frames need a named decision and horizon. | New Lines stay opt-in until they improve a named decision and pass authority, family-cap, ablation, and hockey-reasonableness review. |
| GLASS | Three products need compact labels; exposing dozens of switches as the default interface would bury the decision. | Pane-first views and curated Frames remain primary; detailed Lines stay drilldown/custom-authoring material. |
| CREST | The Window should feel like a point of view, not a profile marketplace. | Official Frames remain editorially constrained; extensibility lives in contracts and custom Frames rather than dashboard clutter. |
| broadcast | Saved browser views must pin exact Frame and board identity across lifecycle changes. | URL state continues to carry season/as-of/Frame identity; no browser-local replacement or hidden migration is allowed. |

### Fixed-point decisions

1. Keep `organization_window_registry.v1` unchanged during current source
   completion; lifecycle ships later as one compatibility-reviewed registry
   revision.
2. Never rewrite or alias a sealed observation, Frame, board, history, scenario,
   or card because a profile was superseded.
3. Require every official Frame to name one decision and primary horizon.
4. Treat health, forecast success, and window timing as related consumers of
   shared profiles, not synonymous outputs.
5. Preserve the existing narrow extension path: typed provider -> descriptor ->
   registry -> manifest opt-in -> sealed board -> shared renderers.

**Fixed point:** pass. No new role is warranted; the existing roles cover the
defect classes. Current S2 source-completion work may continue without waiting
for the lifecycle schema slice.

### S2 development-rule effective-season review

TAPE/WIRE found that URL provenance alone did not prevent a prior-season
exception clause from being relabeled `final`. HART/KEEL required the base
composition rule, age exception, and European-youth exception to remain
separate authority axes.

BENCH/EDGE/FORGE required explicit refusal when any final clause is absent,
future-dated, or effective for a season other than the target. Application
rechecks those seasons instead of trusting the authority enum alone.

SCOUT/GLASS/CREST accepted provisional use of the 2025-26 exception clauses
for planning only. The official 2026 announcement confirms the 260-game
composition rule and unrestricted six veteran slots, but the public rules page
still exposes only the 2025-26 book; surfaces must keep final classifications
withheld.

**Verdict:** pass for the v2 calendar gate. The 2026-27 exception authority is
still source-gated and cannot be closed by changing a label.

### S2 paired cross-league value review

HART/KEEL required method and application to remain separate, bind the exact
workboard and career-history axes, and run before recall readiness. The value
ledger cannot imply assignment, organization status, waiver state, or roster
authority.

TAPE/WIRE accepted only official landing career rows and sealed fingerprints.
No external conversion table or unreviewed scrape enters the authority chain,
and a saved JSON ledger must replay identically.

PACE/SCOUT approved paired same-season or next-season AHL calibration with
separate skater and goalie forms. Minimum pairs, unique players, aggregate and
per-pair workload, source recency and sample, and RMSE-derived sample/fit
confidence are explicit. This is an evaluation index input, not a promotion or
NHL-success probability.

BENCH/EDGE/FORGE cover invalid policies, weak calibrations, short player
workload, fingerprint tampering, direct-score preservation, duplicate
organization appearances, canonical JSON round trips, and the real all-32
chain.

GLASS/CREST/broadcast require the UI-neutral document to label the result as an
evaluation, expose calibration and player provenance, and retain unavailable
rows. Renderers may not describe the method as a universal NHLe model.

**Verdict:** pass for paired cross-league value and narrow application. The
real run supported 14 calibrations, estimated 78/97 unique candidates, and
applied 79/98 appearances. Rebuilt readiness covered 1,263/1,282 candidates
and 1,352/1,371 appearances. The remaining 19 are deliberately unavailable;
historical outcome calibration remains a separate future authority.

### S3 AHL-to-NHL goalie-value review

HART/KEEL separated missing player value from missing lineup assignment. The
new ledger estimates only goalie quality; it cannot add a goalie to a roster,
select a backup, or resolve organization depth.

TAPE/WIRE retained official landing career rows, a dated store, exact player
IDs, same/next-season pair rules, and a sealed fingerprint. The live roster
refresh demonstrated resumable 429 handling: 24/32 teams were cached, the
partial snapshot was refused, and the second pass sealed all 32.

PACE/SCOUT approved a shot-weighted additive save-percentage translation only
with minimum pair/player/shot gates, RMSE fit confidence, sample confidence,
confidence-discounted workload, and an explicit NHL prior. The result is an
evaluation score, not NHL equivalency, roster probability, or direct reuse of
the AHL value scale.

BENCH/EDGE/FORGE cover known delta, weak cohort, short candidate workload,
prior shrinkage, bounded confidence, unavailable candidates, canonical sealing,
and tamper rejection. Observed NHL quality retains precedence; the lineup
labels fallback values `estimated` and names the method.

GLASS/CREST/broadcast accepted the existing compact score with an estimated
evidence label and method warning. Rich calibration detail stays in the
UI-neutral ledger rather than being recomputed by renderers.

**Verdict:** pass for missing-value translation. The live all-32 source audit
now has `nhl.goalie_quality` at 32/32 and goalie dependency at 31/32. UTA is
resolved through an estimated Stauber score; BOS remains correctly blocked
because the official roster has no backup. A dated organization-pool/camp
assignment authority is the next separate slice.

### S3 camp goalie-assignment review

HART/KEEL required camp assignment and goalie value to remain independent.
The modal confirmed-pool branch can fill only an empty slot and cannot evict an
existing goalie; the paired-career ledger supplies the inserted player's score.

TAPE/WIRE required the existing full package fingerprint to validate before
refresh and the complete package to reseal afterward. That replay exposed two
additive-field compatibility defects: empty `eligible_positions` and absent
`development_rule_qualified` were being serialized into old v1 documents.
Omitting only empty/absent additive values restores the historical fingerprint
without accepting a mismatched seal.

PACE/SCOUT accepted Michael DiPietro only as the camp's scenario backup. His
65.4 value comes from 16 confidence-adjusted paired-career games, not his
one-game NHL rate or the camp's own score. Swayman's confirmed 70 remains
unchanged.

BENCH/EDGE/FORGE verified missing value refusal, exact skater preservation,
explicit off-natural-side assignments, full two-goalie completion, parser
coverage, old-package validation, refreshed-package sealing, and all-32 audit.
The preservation test found and fixed the prior inability to replay a manager's
off-natural-side forward choice; `FlexibleForward` records that deployment
without falsifying natural eligibility.

GLASS/CREST/broadcast retain compact score/evidence output and a method warning;
renderers do not join the camp and value authorities themselves.

**Verdict:** pass. Both goalie profiles are 32/32 and the refreshed full package
is now 14/16 required profiles complete. Organization depth and recall depth
are the only required source gaps.

### S2 league-to-Window composition review

HART/KEEL required the reviewed league projection-input artifact to remain the
authority boundary. The package command builds canonical affiliate projections
from that artifact and core continues to compose the NHL/AHL System and both
Window profiles.

TAPE/WIRE required exact schema and season matching, a non-empty complete
cohort, matching input counts, and zero named failures. Loose per-team inputs
conflict with the league artifact instead of creating precedence ambiguity.

FORGE/BENCH/EDGE approved one narrow loader seam, parser and conflict coverage,
and fail-closed validation. The feature does not invent assignments or turn a
preseason candidate into an AHL roster fact.

PACE/SCOUT retained the existing organization-depth and recall-depth methods;
this change supplies their reviewed source shape but does not reweight or
reinterpret them. GLASS/CREST/broadcast require no surface fork because the
sealed package and downstream UI-neutral views are unchanged.

**Verdict:** pass for extensible composition. The remaining 14/16 state is a
dated source-authority gate, not missing Window plumbing. A complete reviewed
league artifact will flow to 16/16 through the existing adapters.
