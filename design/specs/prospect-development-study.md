# Prospect Development Study

**Status:** Implemented foundation
**Schemas:** `prospect_development_study.v1`, `prospect_discovery_board.v1`,
`prospect_league_context.v1`, `prospect_league_discovery.v1`,
`prospect_goalie_development_study.v1`, `prospect_career_discovery.v1`,
`prospect_program_board.v2`, `prospect_program_sensitivity.v1`,
`prospect_program_history.v1`, `prospect_conversion_input.v2`,
`prospect_conversion_performance.v2`, `prospect_conversion_board.v2`

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
icelines icecast prospect-board \
  --study firkus-study.json \
  --study another-study.json \
  --json --out prospect-board.json
icelines icecast prospect-context \
  --snapshot ahl-2023-24.json \
  --snapshot ahl-2024-25.json \
  --snapshot ahl-2025-26.json \
  --league-crosswalk reviewed-league-2023-24.json \
  --league-crosswalk reviewed-league-2024-25.json \
  --league-crosswalk reviewed-league-2025-26.json \
  --affiliations ahl-affiliations-2025-26.json \
  --as-of 2026-09-15 --max-age 24 \
  --json --out prospect-context.json
icelines icecast prospect-league \
  --snapshot ahl-2024-25.json \
  --snapshot ahl-2025-26.json \
  --crosswalk reviewed-2024-cv.json \
  --crosswalk reviewed-2025-cv.json \
  --context examples/icecast-prospect-league-context.json \
  --json --out league-discovery.json
icelines fetch career \
  --camp-forecast league-training-camp.json
icelines icecast prospect-career-context \
  --camp-forecast league-training-camp.json \
  --rosters data/rosters.json \
  --bios data/seasons/20252026/bios.json \
  --candidate-overlay league-candidate-overlay.json \
  --career-history ~/.icelines/career_history.json \
  --json --out career-context.json
icelines icecast prospect-career \
  --context career-context.json \
  --career-history ~/.icelines/career_history.json \
  --json --out career-discovery.json
icelines icecast prospect-program \
  --league-discovery league-discovery.json \
  --career-discovery career-discovery.json \
  --prior-board prior-season-prospect-programs.json \
  --maximum-nhl-games 50 \
  --json --out prospect-programs.json
icelines icecast prospect-program-sensitivity \
  --league-discovery league-discovery.json \
  --career-discovery career-discovery.json \
  --thresholds 25,50,82 \
  --json --out prospect-program-sensitivity.json
icelines icecast prospect-program-history \
  --board prospect-programs-2024.json \
  --board prospect-programs-2025.json \
  --board prospect-programs-2026.json \
  --json --out prospect-program-history.json
icelines icecast prospect-conversion \
  --league-discovery frozen-2022-23-prospects.json \
  --career-history ~/.icelines/career_history.json \
  --baseline-season 20222023 --through-season 20252026 \
  --performance-out nhl-performance.json \
  --json --out prospect-conversion.json
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

## Discovery board

`ProspectDiscoveryBoardView` composes one or more validated study artifacts into
three independently ranked lanes:

- **Hidden Gems** requires supported upside plus underrecognition, or a study
  classification that explicitly identifies hidden or injury-obscured value;
- **Buyer Beware** requires overexposure or an explicit hype/cooling
  classification; and
- **Watch** retains aligned and uncertain cases without inventing a positive or
  negative conclusion.

Every row preserves its classification, market position, hidden-value score,
performance-attention gap, and complete set of active lenses. Hidden Gems rank
by hidden-value score. Buyer Beware ranks by the strongest supported risk or
negative attention-gap signal. Lane scores are not comparable across lanes.
The builder rejects malformed schemas and duplicate player IDs, so renderers do
not reconcile or silently overwrite studies.

## Reviewed league adapter

`ProspectLeagueDiscoveryView` is the mainline bridge from AHL data into the
study and board primitives. It accepts:

1. two or more official `ahl_roster_stats.v1` season snapshots;
2. reviewed `ahl_identity_crosswalk.v1` documents for the relevant season/team
   combinations; and
3. one `prospect_league_context.v1` document containing facts the feeds cannot
   safely infer.

The adapter joins provider-local AHL identities to canonical NHL IDs only
through rows whose status is `reviewed`. It aggregates joined skater totals by
season, attaches snapshot and identity provenance, builds the canonical studies,
and composes the board without reimplementing scoring. Context players that do
not have reviewed identity, joined skater facts, or two AHL seasons appear in a
typed exclusion list. An audited adapter result may contain a canonical empty
board when every supplied player is excluded, so absence remains inspectable.

The separate context file owns current organization, position, age, NHL games,
opportunity, availability, attention estimate/basis, and supporting evidence.
Those fields are deliberately not guessed from AHL production.

`prospect-context` can now create an `observed_draft` context for the whole AHL
from official season snapshots, reviewed league crosswalk envelopes, and a
dated affiliation catalog. It retains only skaters appearing in the latest
snapshot at or below the configured age ceiling with at least two joined AHL
seasons. Skaters and goalies use the same reviewed identity and dated
organization boundary. Provider `active` state resolves the current organization after an
in-season AHL trade; multiple active organizations remain an explicit
exclusion. Older players, one-season samples, missing affiliations, and
unresolved assignments are preserved in typed exclusions.

Historical calibration freezes the observation window instead of leaking
current affiliations or later performance into the cohort. For example, a
2021-22 + 2022-23 baseline supplies both official snapshots and both reviewed
identity envelopes, uses the dated 2022-23 affiliation catalog because it is
the latest season in that window, and sets an as-of date immediately after the
window. Earlier shared affiliations remain auditable historical provenance.
Names that cannot be joined through a reviewed canonical identity are coverage
gaps, not failed prospects; they remain absent from the scored cohort until
reviewed rather than receiving a zero outcome.

Historical identity proposals may use a later reviewed
`ahl_identity_league_crosswalk.v1` envelope as their canonical candidate
authority. Only its reviewed rows are extracted, duplicate appearances merge by
canonical NHL player ID, and conflicting canonical names or birth dates fail
closed. This lets replay reuse prior human review before official discovery is
reserved for the unresolved exception queue.

The generated artifact uses neutral placeholders for the facts the AHL adapter
cannot establish: NHL games remain zero, opportunity is `none`, availability is
`unknown`, and attention is 0.5. Its `observed_draft` authority fails validation
if those fields become non-neutral without first being promoted to authored
context. Consequently the draft is suitable for the attention-independent
program ranking, but Hidden Gems and Buyer Beware require separate sourced
enrichment. `prospect-league --crosswalk` accepts either individual reviewed
team crosswalks or reviewed league envelopes and flattens the latter without
weakening the reviewed-only join.

When that `observed_draft` context is consumed, all discovery-board lanes are
empty by contract. The production and trajectory studies remain available for
program ranking, but neutral attention placeholders cannot label a player a
Hidden Gem, Buyer Beware, or Watch recommendation. Supplying authored, sourced
attention context re-enables the normal board composition.

## Goalie development adapter

`ProspectGoalieDevelopmentStudyView` keeps goalie development native. Current
performance combines save percentage (70%) and inverse goals-against average
(30%), then applies latest-season workload confidence. Same-league trajectory
combines save-percentage change and GAA improvement only when both seasons clear
the goalie comparison workload. Opportunity and availability remain separate
context facts.

Goalie studies enter Pool, Development, Pipeline, positional balance, confidence,
and top-prospect rows. They do not enter the skater-oriented Hidden Gems or Buyer
Beware lanes because the observed draft has no sourced goalie attention context.
Team defense and shot quality are not yet isolated, so the adapter is explicitly
an AHL results-and-workload development signal rather than a complete goalie
talent model. Program development averages are workload-weighted; a two-game
sample lowers evidence coverage instead of being treated as poor development.

## Multi-league career adapter

`ProspectCareerDiscoveryView` bridges the cached official NHL player landing
history into the same skater, goalie, discovery-board, and program-board
primitives. It recognizes regular-season CHL and other classified junior rows,
NCAA/conference rows, and European professional rows. NHL, AHL, ECHL,
international tournaments, playoffs, and unclassified leagues are excluded.

Split-team stints are aggregated within a season and league, the official
landing URL is attached as evidence, and every missing or unusable player stays
visible through a typed exclusion. Raw trends still compare only within the
same league. An OHL-to-NCAA or SHL-to-North-America move is therefore
`insufficient`, not an invented rise or decline; no league-equivalency factor is
applied.

`fetch career --camp-forecast` resolves the distinct prospect IDs in a
`training_camp_league_forecast.v1` artifact instead of limiting acquisition to
current NHL rosters. The career cache retains official landing birth dates
beside career totals, allowing players absent from current roster and season-bio
files to remain addressable on later runs.

`prospect-career-context` converts that same camp pool into an
`observed_draft` context. Roster, bio, overlay, and cached landing identity facts
may supply birth dates; cached regular-season NHL stints supply NHL games
played. Camp probabilities and scores do not become development, opportunity,
availability, or attention evidence. Players with missing identity, excessive
age, or multiple camp organizations remain typed exclusions. Like the AHL
draft, its opportunity, availability, and attention fields must stay neutral
unless the artifact is deliberately promoted to authored context.

Window source assembly may run this chain directly with
`window-source-package --training-camp PATH --cache-prospect-program`. The
fetch-owned composer uses the configured official landing career cache (or an
explicit `--career-history` override), exact package cutoff, and the canonical
context/discovery/program builders. It does not consult repository-relative
rosters, bios, or overlays. A camp candidate enters the conservative draft pool
when marked as a prospect, rookie-eligible, or waiver-exempt, but exact age,
career evidence, NHL workload, and the normal program graduation boundary still
govern the resulting study. Waiver exemption is a compatibility lane for
authored camp inputs that predate the typed prospect flags; it does not add
player value or independently establish prospect status.
Candidate overlays additionally carry a source-bound relationship. Draft
rights, NHL contracts, and AHL assignments may enter career context;
development-camp participants, free-agent invitees, and unknown relationships
remain available to camp simulation but are removed before prospect ranking.
Legacy overlays retain their previous behavior through an explicitly named
compatibility state rather than an unlabeled inference.
`prospect-population-audit` projects the same fetch-owned overlay into a
UI-neutral authority workboard. It reports ranking-eligible, camp-only, legacy,
and unknown counts globally and by organization without resolving or mutating
canonical `PlayerIdentity`. Legacy and unknown relationships prevent
`fully_classified` authority even though legacy rows retain compatibility
behavior. Publication automation may use `--require-fully-classified` to fail
before writing output when either unresolved authority class remains.

`prospect-program --career-discovery` composes these studies with reviewed AHL
discovery. When both adapters contain the same player, reviewed AHL facts take
precedence and career discovery fills gaps, preventing duplicate program credit.
One-season AHL, CHL, NCAA, junior, or European-pro players remain eligible:
production uses their observed workload, trajectory is `insufficient`, and
workload confidence receives the disclosed 35% limited-history factor. This
keeps first-year professionals and newly drafted players visible without
pretending that a development trend has been observed.

## Prospect program board

`ProspectProgramBoardView` aggregates canonical prospect studies by
organization into three independent frozen ranks:

- **Pool / The Depth Chart** combines the top-three observed signal, quality
  depth, and positional balance;
- **Development / The Factory** combines same-league trajectory evidence with
  workload confidence and observed program breadth; and
- **Pipeline / The Pipeline** combines Pool, Development, documented
  readiness, and confidence.

Each organization also publishes an ordinal player ranking in `top_prospects`.
The default and CLI-selectable publication depth is ten players per team. The
publication depth is deliberately independent from `expected_depth`, so asking
for fewer or more displayed players cannot change Pool, Development, or
Pipeline scores. A team with fewer eligible supplied studies returns every
eligible player and remains visibly short of the requested depth; IceLines does
not fabricate or impute missing prospects. Player rows retain position,
observed signal, trajectory, documented opportunity, workload confidence, and
NHL games played so JSON, text, web, TUI, card, fantasy, and simulation clients
can render the same ranking without recreating scoring logic.
The board publishes complete and partial organization counts plus an exact
per-team shortfall. CLI publication may opt into `--require-complete-rankings`;
that gate fails before writing output when any organization lacks the requested
eligible depth. A successful all-32 publication therefore cannot silently mean
"top ten where available."

`prospect-program --source-package` additionally gates every supplied study
through the canonical current-control resolver. AHL affiliation, roster
presence, development-camp attendance, and free-agent invitations do not prove
NHL organization control. Unsupported rows and organization mismatches are
excluded, while the board publishes the source-package fingerprint, population
completion state, and retained/excluded study counts. Automation may combine
`--require-complete-population` and `--require-complete-rankings` to require both
a terminal source census and ten controlled, eligible studies per organization.

The observed player signal uses production, trajectory, and documented
opportunity components. It deliberately excludes hidden-value and attention-gap
scores because underrecognition is not prospect talent or ceiling. Missing
depth lowers depth and confidence instead of being imputed. The optional prior
board supplies rank and score deltas only; positive delta means improvement.
The board publishes `prior_as_of_season` plus a typed methodology record with
the scoring-method version, expected depth, and Pool/Development/Readiness/
Confidence weights. Comparison fails closed unless the prior season is earlier
and uses the same methodology, scope, source-league set, and NHL graduation
boundary. Older artifacts without provable methodology may still be rendered,
but cannot produce deltas. This prevents same-season, future, cross-scope, or
cross-method artifacts from being presented as year-over-year movement.

Version 2 applies a configurable reserve-system graduation boundary, defaulting
to 50 regular-season NHL games. Studies above the boundary remain in supplied
coverage and appear in each organization's typed `graduates` lane, but they do
not contribute to Pool, Development, Pipeline, positional balance, confidence,
or top-prospect rankings. The board publishes supplied, ranked, and graduated
counts plus the exact threshold. This is an IceLines population rule, not a
claim about NHL rookie eligibility. Prior-board deltas require the same
threshold so population changes cannot masquerade as program improvement.
Graduation applies only to NHL workload marked `observed`. Missing authority
remains ranked conservatively and is counted explicitly at board, organization,
and sensitivity-point levels; an adapter placeholder zero is never treated as
an observed zero.

`ProspectProgramSensitivityView` rebuilds the identical supplied studies across
two or more unique graduation thresholds. For each organization it freezes the
pipeline, pool, and development ranks, pipeline score, ranked count, and
graduated count, and unknown-workload count at every boundary, plus the
best/worst pipeline rank and numeric rank/score spans. It deliberately does not
label a threshold as correct or convert definition sensitivity into performance
uncertainty. The default CLI comparison uses 25, 50, and 82 NHL games; callers
may supply other boundaries. The sensitivity document carries the same typed
scoring methodology so its threshold-only comparison remains independently
auditable.

`ProspectProgramHistoryView` accepts two or more annual program boards and
requires unique seasons plus identical scope, source leagues, graduation
boundary, and typed scoring methodology. It recomputes adjacent-season deltas
from the board rows instead of trusting deltas embedded in those inputs, and
also reports each organization's first-to-latest movement. A team missing from
the immediately preceding board receives null adjacent deltas; the history does
not bridge that evidence gap. This is program-population movement, not a causal
claim about development or future NHL success.

## Prospect conversion efficiency

`ProspectConversionBoardView` compares a frozen, attention-free prospect signal
with NHL outcomes observed after a configurable minimum horizon. Arrival uses
NHL games and role uses NHL time on ice per game with separate forward,
defense, and goalie benchmarks. Performance is a canonical position-normalized
0..100 measure derived from the same official landing histories by default.
An authored `prospect_conversion_performance.v2` document may override that
authority, and `--performance-out` freezes either version for audit and reuse.

Skater performance combines points/82 (35%), goals/82 (20%), average TOI
(20%), power-play points/82 (15%), and shots/82 (10%), with separate forward
and defense targets. Goalie performance combines save percentage (50%),
inverse GAA (25%), start share (15%), and shutouts per game (10%). The raw
position score is multiplied by `GP/(GP+30)` for skaters or `GP/(GP+20)` for
goalies. The document publishes each raw metric, normalized component, weight,
raw-quality score, sample confidence, final score, horizon, and official player
landing URL. These are fixed modern-horizon quality benchmarks, not an
era-wide WAR model.

A complete official history with no post-baseline NHL games is an observed
zero. Missing official history, time on ice, or a required rate input fails the
adapter instead of becoming zero. This distinction permits full source
coverage without confusing a verified non-arrival with absent data.

`adapt_prospect_conversion_input` consumes the complete frozen skater and goalie
study cohorts rather than the top-five summaries on a program board. It applies
the program method's same production/trajectory/opportunity signal, copies the
baseline evidence, and totals only official regular-season NHL stints after the
baseline season through the declared outcome season. Missing career history or
TOI fails adaptation instead of becoming a zero-value outcome. Supplied player
performance must arrive as a complete v2 authority with an explicit method,
horizon, component evidence, and 0..100 score.

The application surface is `icelines icecast prospect-conversion`. It accepts
the same repeatable frozen league-discovery, career-discovery, and study inputs
as the program board, plus `--career-history`, `--baseline-season`, and
`--through-season`. An optional `--performance` document must use
`prospect_conversion_performance.v2`. When omitted, the command derives the
document from `--career-history`; `--performance-out` writes it separately.
Without sufficient performance coverage, the command still emits the complete
board and typed rank blockers but does not manufacture organization ranks.

Each player retains baseline season and confidence, outcome season, arrival,
role, performance, realized value, conversion delta, efficiency index,
established-player state, disposition, evidence URLs, and a typed comparison
class: `expected_hit`, `breakout`, `miss`, or `developing`. The labels compare
the frozen baseline and realized value against a disclosed 60-point threshold;
they are historical comparison buckets, not scouting verdicts. Organization rows
publish converted and established counts, retained and traded counts, aggregate
baseline and realized scores, baseline confidence, coverage, efficiency, an
optional rank, class counts, and typed rank blockers.
Trade or retention status does not add value until a separate sourced return
model exists.

Version 2 also retains the frozen production, trajectory, and opportunity
component scores and publishes cohort-level signal calibration beside overall
signal and workload confidence. Each row reports Pearson association with NHL
arrival, established-player status, and normalized role, plus tie-safe bottom-
and top-quartile arrival, establishment, and role results. A component with no
variation is explicitly non-informative and receives null correlations and no
quartile comparison. These are descriptive associations over the frozen cohort,
not causal claims or fitted probabilities. The performance score remains a
separate optional authority and is not inferred from arrival or role.

## Prospect arrival calibration

`prospect_arrival_calibration.v1` converts one current, attention-free prospect
signal into a historical NHL-arrival base rate without using NHL development
cohorts. It requires a frozen `prospect_conversion_board.v2` whose outcome
season strictly precedes the target forecast season. The target player must not
appear anywhere in that historical board and must have zero authoritative NHL
games. Players who already appeared in the NHL require established-role
forecasting; they must never be relabeled as arrival successes.

The default method selects the 50 nearest players from the same position group
(forward, defense, or goalie), requires at least 30 observations, and rejects a
mean signal distance above 15 points. The local empirical arrival rate is
shrunk toward the complete same-position board rate with 20 pseudo-observations.
Output retains candidate and neighbor counts, signal range and mean distance,
raw arrival and establishment counts, empirical rates, the shrunken probability,
source seasons, policy, and disclosures. Arrival means at least one NHL game
inside the frozen horizon; establishment remains a separate descriptive rate.

```text
icelines icecast prospect-arrival-calibrate \
  --input current-prospect.json \
  --conversion-board frozen-conversion.json \
  --json --out prospect-arrival.json

icelines icecast prospect-arrival-calibrate \
  --career-discovery current-career-discovery.json \
  --player-id 8485957 \
  --event-id nyr-smits-defense-hit \
  --forecast-season 20262027 \
  --conversion-archive frozen-conversion-archive.json \
  --input-out derived-arrival-input.json \
  --json --out prospect-arrival.json

icelines icecast prospect-arrival-league \
  --camp-forecast league-camp.json \
  --conversion-archive frozen-conversion-archive.json \
  --forecast-season 20262027 \
  --discovery-out league-career-discovery.json \
  --json --out league-arrival.json
```

`prospect_arrival_league_calibration.v1` requires the exact canonical NHL team
set. It retains one row for every organization, including zero-target and
zero-success teams, and reconciles target skaters as calibrated plus excluded.
Each skater uses one shared cohort and policy. Adapter, self-overlap, sample,
and comparable-distance failures remain attached to the player and team rather
than disappearing from league totals. Goalies remain outside this skater
cohort until a separately calibrated goalie-arrival authority exists.

The primitive never reconstructs an absent historical board. It accepts either
an authored current signal or derives one from a canonical skater career study;
goalies, missing IDs, and duplicate IDs fail closed. Historical proof runs
intended for future forecasting must retain their conversion board, frozen
adapted input, and performance authority. The preferred durable contract is
`prospect_conversion_archive.v1`: it stores all three, fingerprints every
member, replays the board from the input during validation, and is accepted
directly by both conversion replay and arrival calibration.

The default method requires a three-season horizon, treats 82 skater games or
40 goalie games as established, and caps the efficiency index while applying a
baseline denominator floor. A program also needs at least five players, 0.50
mean baseline confidence, and 0.80 outcome coverage to receive a rank. Programs
that miss any floor remain visible with the precise blockers. These are
transparent IceLines cohort rules, not a claim that the organization caused an
individual result.

The July 2026 2022-23-to-2025-26 proof covered all 247 frozen players and all 32
organizations from official landing histories. It produced complete
performance authority and made 19 organizations rankable; the remaining 13
were held out only by cohort-size or baseline-confidence floors. The Rangers'
five-player slice remained unranked for 0.44 baseline confidence, and Seattle's
three-player slice remained unranked for insufficient cohort size. Those rows
are useful side-by-side validation examples precisely because their rank
blockers remain visible.

The separately named August 2026 retrospective cohort is not presented as the
lost July proof. It begins with the frozen 2022-23 AHL snapshot, applies a
September 15, 2023 age-24 ceiling, and retains 543 players across all 32 NHL
organizations through 2025-26. Its fingerprinted archive is checked in as
`examples/prospect-conversion-archive-retrospective-2022-23-through-2025-26.json`.
The cohort includes 164 defensemen; Alberts Smits' derived 29.06 signal selects
50 neighbors with a 62% empirical arrival rate and a 45.7317% full-position
rate, producing a shrunken 57.3519% arrival probability. Arrival still means at
least one NHL game and must not be described as establishment or full-impact
breakout probability. Establishment is calibrated separately: the 22% neighbor
rate shrinks toward the 18.9024% complete-defenseman rate, producing 21.1150%
cumulatively over the three-season source horizon. The one-season 2026-27
projection is 7.6015% under the disclosed constant-hazard transform; the
three-season arrival rate similarly becomes 24.7280% for one season. The
11-of-31 established-given-arrival share is retained only as descriptive
context.

On the July 2026 all-organization proof, Seattle ranked first at all three
default boundaries; its score ranged only from 56.04 to 57.05. The Rangers
ranked 23rd at 25 GP, 11th at 50 GP, and 16th at 82 GP. Their strict-boundary
graduates were Brett Berard, Scott Morrow, Jaroslav Chmelar, and Brendan
Brisson. San Jose was the most definition-sensitive organization in that run,
spanning ranks 15–28 and scores 32.36–45.86. This distinction between stable
team evidence and changing relative rank is why the artifact publishes both
score and rank ranges.

An AHL-only board is `ahl_observed`; recognized career studies change the scope
to `multi_league_observed` and list the actual source leagues. The program
command accepts `prospect_league_discovery.v1` and
`prospect_career_discovery.v1` artifacts plus optional canonical studies. It is
not an all-system NHL ranking until supplied context and career cache cover the
complete organizational pools, including NHL-rostered prospects. This
limitation is part of the output contract, not renderer prose.

The production path has been exercised over three official AHL seasons and 32
NHL organizations. That result is a complete all-organization, eligible
multi-season AHL skater-and-goalie comparison, not a complete organizational
prospect-system ranking. The July 2026 proof retained 352 skaters and 31
goalies; every other candidate remained visible through the context exclusion
audit.

The July 2026 camp-to-career proof fetched 147 distinct prospect histories from
the 32-team camp artifact with no acquisition skips. The neutral context
retained all 147 players across 31 organizations (Florida's supplied camp pool
contained no prospect-flagged players). Career adaptation produced 138 skater
studies, six goalie studies, and three typed insufficient-history exclusions
across 15 observed source leagues. Composing those facts with the AHL board
produced a 32-organization `multi_league_observed` program board. These counts
prove adapter coverage for that dated input; they are not a claim that every
organization's complete reserve list was supplied.

The follow-up authority pass fetched official career history for all 383 players
in the reviewed AHL context with no acquisition skips. It reduced unknown NHL
workload from 323 combined studies to zero. Career-rate adaptation still
excluded Danila Klimovich, Bradly Nadeau, Stian Solberg, and Callum Tung for
insufficient eligible history, but those typed exclusions retain official NHL
GP so overlapping AHL studies do not lose valid workload authority. At the
default 50-GP boundary, all 467 studies remained auditable, 391 ranked, and 76
appeared as graduates. Seattle remained first; the Rangers ranked 11th at
45.65. Every 25-, 50-, and 82-GP sensitivity point reported zero unknown NHL
workloads.

## Guardrails

- At least one eligible season is required for a rankable production study.
- At least two comparable same-league seasons are required for a trajectory
  claim; one-season players remain `insufficient` with limited-history confidence.
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

Expand authored all-organization context and cached landing coverage, then add
role, chemistry, special-teams, and sustainability fact adapters. Those
adapters must emit typed evidence into the study rather than changing renderer
logic.
