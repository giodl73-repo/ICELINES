# Trade Market Forecast — Specification

**Version:** 0.1  
**Date:** 2026-08-02  
**Status:** In progress  
**Owner domain:** IceCast personnel and organization strategy

## Purpose

Rank plausible NHL trades that solve identifiable depth-chart needs and can
create positive utility for both organizations. The model must distinguish a
sourced availability signal from a speculative hockey fit and must never
describe a generated proposal as a reported negotiation.

## Decision model

Each team supplies a dated outlook, competitive timeline, cap flexibility,
position/role needs, roster surplus, and ownership of tradeable assets. Each
player supplies present impact, future/control value, cap surplus, fit, and an
availability probability with provenance. No-trade clauses, retained salary,
contract expiry, protection, roster limits, and cap compliance are gates rather
than narrative footnotes.

`transaction_ready` requires confirmed cap and roster compliance, complete
contract/clause authority for every player, and confirmed ownership for every
included pick. Unknown is not equivalent to clear: incomplete authority keeps
hockey utility visible but forces transaction feasibility to zero and prevents
a mutual-benefit recommendation.

Package assembly accepts raw player IDs and probabilistic pick assets. Core
joins each asset to dated execution authority, confirms the sending club,
values picks from the sealed curve, and derives both teams' post-trade cap and
active-roster states. Cap totals, roster counts, contract control, clause
review, and pick ownership require direct source URLs and observation dates;
missing provenance produces `unknown`, never an inferred clearance.
Player movements may declare retained cap hit. Core applies the transferred
portion to both clubs, limits a club's retained amount to 50% of that contract,
and checks the sending club's sourced remaining retention slots. Missing slot
authority blocks transaction readiness. Club cap utility uses the derived
package-level cap-space change in millions, so retention can benefit the buyer
and cost the seller without inventing one universal asset-level cap value.

A proposal is ranked on four separate outputs:

1. buyer utility delta;
2. seller utility delta;
3. market-value balance and uncertainty; and
4. transaction feasibility.

Market fairness uses one explicit control-value currency. Every player and pick
must carry the same outcome measure, horizon, and reproducible method; mixed
currencies block evaluation. Immediate on-ice quality is reported separately as
expected next-season standings points gained or lost. It can influence a club's
timeline utility, but it is never silently added to expected NHL games, cap
surplus, or another unlike unit.

Mutual benefit does not mean identical valuation. A contender can rationally
prefer current wins while a rebuilding club prefers delayed value and contract
control. A proposal that fails either team's utility floor is not recommended.

## Draft-pick value

Draft picks are probabilistic assets, not round labels. A pick contains a draft
year and a probability distribution over overall selection slots. Protection
and deferral are represented as explicit branches in that distribution.

IceLines learns the base curve from mature historical draft cohorts using a
fixed post-draft outcome horizon. Candidate outcome measures include NHL games,
time on ice, point share, IceLines player value, and entry-level-contract
surplus. The published curve must be monotone: an earlier pick cannot have
lower expected value solely because of historical noise. It retains sample
counts and uncertainty by slot.

The calibration denominator is every non-forfeited selection from complete,
terminal official draft ledgers. Skater-only or NHL-appearance-only datasets
are invalid because they omit goalies and zero-game selections. Regular-season
appearances join to selections by official draft year and overall pick, never
by display name. Exact duplicate source rows may be collapsed with a published
count; conflicting rows block publication.

Future selections are discounted for time and their slot distribution is
conditioned on the transferring team's season forecast. Draft-class strength
may modify a curve only through separately sourced, dated evidence.

A pick is labeled `rounding` only when it contributes less than 40% of the
receiving package's expected value. Otherwise it is a principal asset. This
prevents a lottery first from being disguised as a throw-in.

## Availability evidence

Availability is a typed evidence state:

- `reported_request`: a reputable report says the player requested a trade;
- `club_shopping`: the club or a reputable reporter says the club is taking calls;
- `contract_pressure`: expiry, arbitration, cap, or clause evidence creates pressure;
- `depth_surplus`: IceLines infers a surplus from the roster, without claiming talks;
- `speculative_fit`: no availability evidence; scenario exploration only; and
- `unavailable`: clause, club statement, or roster strategy blocks the proposal.

Every non-speculative state requires source URL and observation time. Player
preferences and trade protection are constraints. A trade request does not
waive a no-movement clause or imply acceptance of every destination.

## UI-neutral output

The core view includes team needs, proposed assets in each direction, each
team's utility delta, expected pick values and ranges, cap/roster gates,
availability evidence, likelihood, season-forecast deltas, and disclosures.
CLI, TUI, web, cards, and simulations consume this document without recomputing
trade logic.

A package may attach paired `TeamSeasonForecastView` results only when baseline
and scenario share the same season, schedule, trial count, seed, and team set.
The attachment reports buyer and seller changes in average points, playoff
probability, and Cup probability, plus the buyer residual versus the earlier
isolated points assumption. Paired simulation supersedes that isolated estimate
for season interpretation; it does not rewrite the historical package inputs.
An archived `team_season_forecast.v1` retains the exact game rows needed for a
paired replay. Rehydration must require explicit forecast parameters because
the v1 archive does not retain that envelope; it must disclose those parameters
and must never silently regenerate game probabilities or the schedule.

Lineup impact is a separate UI-neutral attachment. Core optimizes the dressed
roster across configurable C/LW/RW/D/G capacity using confidence-weighted
player scores, natural multi-position eligibility, and explicitly permitted
alternate positions with a declared penalty. The result distinguishes players
explicitly removed by the package from incumbents displaced only by lineup
competition. Chemistry, special teams, opponent matchups, waivers, and final
manager deployment remain downstream inputs rather than hidden optimizer
assumptions.

When full incoming-player evidence is supplied, the optimizer's exact dressed
assignments feed the generic team-lineup rebuild. The rebuild preserves every
incumbent score component and PP/PK role score, releases prior line placement,
and emits four forward lines, three defense pairs, two goalies, PP1/PP2, and
PK1/PK2 in the same UI-neutral result. Players outside the optimized dressed
set become extras; the renderer cannot independently redress a player reported
as displaced. Missing special-teams evidence remains a warning, never an
invented unit. A training-camp lineup set may supply the baseline only through
an explicitly disclosed retained branch.

Multi-candidate screens publish `trade_lineup_board.v1`. Hockey rank orders the
modeled dressed-lineup delta regardless of rumor status. Actionable rank is a
separate nullable field: it exists only for candidates whose complete package
has passed the transaction gate, and ranks positive lineup impact multiplied by
the supplied feasibility probability. A popular or familiar player who fails to
win a dressed spot remains visible with zero impact rather than receiving a
reputation bonus.

An actionable row must embed its `TradePackageEvaluationView`. Core rejects a
claimed ready flag when package evidence is absent or when buyer, feasibility,
or transaction readiness differs from the attached evaluation. This prevents a
hand-authored candidate board from promoting a rumor by toggling a boolean.
Availability probability is retained separately from executable feasibility.
The former describes the candidate-market hypothesis; the latter becomes zero
when any cap, roster, retention, contract/clause, or pick gate fails. Renderers
must display both rather than describing a protected player's nonzero rumor
signal as a nonzero chance of an executable transaction.

Existing `TeamLineupProjectionView` documents adapt directly into the optimizer;
scored extras join the competition pool, while unscored extras are omitted with
an explicit disclosure rather than assigned zero. Projection-change inputs must
carry any authored incoming-player score or alternate-position assumptions.

## Automatic Trade Scout and negotiation ladder

`trade_scout.v1` discovers buyer-specific fits from supplied league assets and
team preferences. Discovery score is buyer hockey utility multiplied by the
asset's sourced or explicitly speculative availability. A destination veto or
`unavailable` state removes the target from discovery even when its pure hockey
fit is high. The Scout does not crawl rumors or invent an availability signal.

Buyer assets carry an independent `protected` policy. Protected assets remain
visible in the audit and are excluded from every enumerated package. The
negotiator searches bounded combinations of the remaining assets and publishes
an opening offer, closest fair midpoint, maximum acceptable package, and numeric
walk-away value. Exact threshold comparisons use a scale-aware numeric tolerance.

Generated ladders are pre-authority strategy documents. Their embedded package
evaluations must remain transaction-blocked until the Trade Desk attaches cap,
roster, retention, contract/clause, and pick-ownership authority. Sourced
availability states require both a valid URL and observation time; a URL alone
is not dated evidence.

`trade_scout_league.v1` is the normalization boundary for league discovery.
Each organization supplies one preference profile and a globally unique asset
inventory classified as NHL player, prospect, or draft pick. NHL-player targets
are derived only when their role matches a declared buyer need and the seller
supplies either a configurable surplus score or stronger dated availability.
Explicit destination vetoes remain excluded. Buyer offer inventory is derived
from picks, prospects, and sufficiently surplus NHL players; protection policy
is preserved for downstream package enumeration.

League output publishes supplied versus expected organization coverage and an
`inventory_complete` flag. Partial inventory must be explicitly authorized and
must render as partial; it cannot silently claim a 32-team search. Normalization
does not itself create player values, surplus evidence, or contract authority.

`trade_scout_population.v1` connects that normalization boundary to IceLines'
existing `training_camp_league_forecast.v1` authority. Every usable camp player
is translated exactly once, preserving canonical player ID, team, position,
prospect status, projected score, GP confidence, and the camp's roster outcome
probabilities. It must not fetch or maintain a parallel roster list.

The translation from camp score to control value, current/future value share,
season impact, and top-six/top-four/starter role is a declared policy. These are
screening assumptions, not learned market prices. Incumbent cut/scratch risk and
non-incumbent selection loss produce only a `depth_surplus` hypothesis unless a
separate overlay supplies stronger evidence. Reported-request, club-shopping,
or contract-pressure overlays require a source URL and observation time.

Protection policy and already-valued, ownership-scoped draft-pick assets remain
explicit inputs. Missing camp forecasts remain named coverage gaps, and the
populator automatically marks the downstream league inventory partial. Contract,
clause, retention, cap, roster, and pick-execution authority still belong to the
Trade Desk transaction gate; population cannot make a generated offer executable.

## Future draft-pick ownership

`trade_scout_draft_pick_population.v1` is the reviewed bridge between a current
future-pick ledger and Trade Scout assets. Every ownership row carries a stable
asset ID, current owner, original team, draft year, round, status, source URL,
and observation time. The supported states are `confirmed_unconditional`,
`conditional`, and `encumbered`. Conditional and encumbered rights require a
human-readable condition and remain unresolved; they cannot enter an offer.

Confirmed-unconditional rights are valued through the sealed mature-cohort pick
curve. When `team_season_forecast.v1` supplies trial-level league-rank
probabilities, the original team's distribution is reversed into a pre-lottery
standings-order proxy. Ownership and slot outlook therefore remain independent:
a Tampa Bay pick owned by Seattle follows Tampa Bay's forecast. This proxy does
not model the lottery or playoff-based draft ordering. Older forecasts without
the distribution and absent teams fall back explicitly to a uniform distribution
across the round's 32 nominal slots. Slots map by within-round percentile when
the sealed curve reflects a smaller historical league, preserving all
probability mass instead of truncating late rounds. The output retains the curve
disclosure while adopting the supplied common player/pick value basis, so a
caller must explicitly name the bridge that makes package arithmetic valid.

There is no assumed seven-picks-per-team fallback. A reviewed consolidated
provider page or transaction source must support every imported right. Pick
protection is a separate buyer policy, and the Trade Desk must still reconfirm
ownership before labeling any generated package executable.

League coverage uses `trade_draft_pick_ownership_coverage.v1`. For one draft
year, the expected inventory is 32 original teams times seven rounds: 224
coordinates. Coverage is indexed by original team and round, not by the number
of assets currently listed under an owner. This catches transferred-away picks
and prevents a team page with seven visible assets from proving league-wide
chain of title.

The coverage view publishes coordinate completeness separately from offer-ready
completeness. A sourced conditional or encumbered coordinate closes a source
gap but remains unavailable to the package generator. A missing coordinate is
unknown ownership; it cannot default to the original team. Two asset claims for
the same original-team/year/round coordinate are an ingestion error requiring
review.

Provider acquisition is intentionally outside core. A licensed provider API or
reviewed saved snapshot may populate the CSV adapter. Automated public-page
access that returns an access challenge is a source failure, never an empty
team ledger. The current PuckPedia public pages require browser execution in
this environment, and its documented data API requires separate access.

## Historical calibration and validation

Backtests freeze evidence at the proposal date. Trade completion is the market
label; later player performance is used only for retrospective value scoring.
Validation reports calibration for completion probability, error in package
value, and realized utility for both clubs. Trades from the evaluation window
cannot train the pick curve or availability model.

## Initial Rangers question

The initial view searches for a top-six forward while protecting the Rangers'
highest-upside young core. It compares reported-request, plausible seller, and
purely speculative candidates. Dylan Larkin is a useful constraint test: a
reported request raises availability, but destination control can still make a
Rangers proposal infeasible.
