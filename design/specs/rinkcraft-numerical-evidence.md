# Rinkcraft numerical evidence — experimental first pass

CHG-130, 2026-11-08. Owner directs numerical assessments to lead player evidence;
quotes remain optional explanatory context. No quote or class label enters the
calculation. Research meanings and player selection remain in Rinkcraft.

## Contract and surfaces

`icelines profile-numbers --season YYYYZZZZ --skaters FILE --goalies FILE
--player ID,ID,...` emits `icelines.rinkcraft-numerical.v1`. Selection limits output
after preserving the full source peer population. With no selection, all source
players are emitted. Core owns calculation, eligibility, missingness, league ranks
and reference fingerprints. Fetch owns bounded local CSV parsing; CLI owns
arguments/serialization only.

Native `/rinkcraft/numbers` and `/api/v1/rinkcraft/numbers` consume the explicit
local JSON path `ICELINES_RINKCRAFT_NUMBERS`. The numerical page links from the
existing atlas. Documentary classes remain unchanged. The CHG-131 extension below
adds prepared numerical evidence to character cards and the browser/WASM subset.

Native JSON input is capped at 2 MiB for this cohort pilot; unrestricted whole-
league CLI output can exceed that display budget and is still emitted normally.

## Source and arithmetic

Input: MoneyPuck regular-season summary CSVs under applicable source terms, with
credit in every display. Byte counts and SHA256 hashes travel with the output.
The adapter checks current headers, situation, season, position and duplicate
IDs. Blank cells stay unavailable; zeros stay zero. The command does no network
access or durable cache mutation. CSVs are limited to 20 MiB each.

Skaters use `5on5`; goalies use `all`. Time is seconds. Rates are count / seconds
* 3600. Metrics carry numerator, denominator, scale and formula. Shot quality is
individual xG / unblocked attempts, not shooting percentage. Goals minus xG is an
observed residual, not persistent finishing talent. Goalie GSAx is expected goals
against minus actual goals against; per-60 uses goalie ice time. On-ice xGF/xGA
rates describe team context. Model/event meanings remain source-specific.

Fifteen skater measures: primary/secondary assist rates; shot rate, individual xG
rate, modeled average shot quality and high-danger attempt rate; goal rate and
goal-minus-xG rate; hits/takeaways/giveaways per 60; penalties drawn/taken per 60;
on-ice xGF/xGA per 60. Five goalie measures: GSAx total/rate, observed save
fraction, expected threat faced per 60 and shot workload per 60.

## Peers and interpretation

Compare same-season regular-season Forwards, Defense and Goalies separately.
Exploratory eligibility: skaters >=300 five-on-five minutes and 10 GP; goalies
>=600 all-strength minutes and 5 GP; at least 20/10 measured peers. These are
sample/display filters, not reliability thresholds. Low-exposure targets keep raw
values but no rank. P = 100 * (strictly lower values + half tied values) / peers.
Higher P means a larger value, not always better performance. There is no hidden
inversion, overall score, five-band skill grade or role-membership probability.
Reference hashes include protocol, season, group, metric and sorted IDs/values.

One-season outputs remain descriptive: no uncertainty model, teammate/opponent
adjustment, persistence estimate, career average or provider-model vintage claim.
A primary assist is an outcome, not a census of playmaking attempts. On-ice
events include teammates. Counts retain recording and possession-demand limits.
Entries/exits, carry/pass/dump choice, direction, pressure, screens, assignment
compliance and goalie mechanics remain outside this input contract.

## Validation

Core checks cover full peers despite selected output, exposure, ties, zero/missing
values, negative goal residual, small cohorts, duplicate IDs and goalie time/shot
denominators. Fetch checks schema/season and blank cells. Real-data acceptance
checks all selected players, source hashes, recomposed components, rendered values/
ranks, disclosure and mobile layout. Research owns the fixed-season run and role
review. Software acceptance does not validate classes or persistent hockey skill.

## Integrated cards and comparisons (CHG-131)

Core projects a validated catalog to `icelines.rinkcraft-evidence-card.v1`:
six skater facets (primary assists, shots, individual xG, average shot quality,
hits and goals-minus-xG) or four goalie facets (GSAx rate, save fraction, threat
faced and shot workload). All 15/5 measurements remain available. Leading signals
are the highest two percentiles among primary assists, shots, individual xG and
hits; ties use that order. Goalies use GSAx rate alone. This editorial shortlist
can contain correlated measurements; it is neither an ability composite nor a
class classifier. Class hypotheses and documented methods remain separate.

Native profiles attach optional `numerical_evidence` only for regular season,
an included snapshot season, and an exact explicit display season if supplied.
Core applies a season ceiling. Numeric primary cards suppress mixed-window raw
season lines and documentary feature stat snippets. Missing documentary classes
do not suppress available numerical evidence. Legacy cards/history still work.

Native GET `/api/v1/rinkcraft/numbers/compare?left=ID&right=ID` returns a shared
core comparison; `/rinkcraft/numbers` provides controls and visual tracks. Reject
identical IDs, unknown players, different groups/situations and incompatible
metric units/reference hashes. Differences are left minus right at full precision;
fraction differences display percentage points, as do percentile differences.
Neither signifies a difference in overall ability or class fit.

WASM `numerical_evidence` and `numerical_comparison` validate the same bounded JSON
and call shared Rust. Corresponding worker operations are stateless; optional
prepared `rinkcraft-numbers.json` is copied and authenticated in the offline shell.
The browser adapter owns presentation only. Active regular-season package ceilings
prevent future evidence; the independently prepared snapshot displays its own
season/situation/peers. The JSON input budget remains 2 MiB. No CSV browser ingestion,
new measurement formula, automatic role transition or opportunity-adjusted model.
