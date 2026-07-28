# Management Behavior Simulation

**Status:** Implemented (behavior and game-plan foundation)

## Product contract

IceLines separates player capability from decision-maker behavior. The same
players can produce different roster, lineup, matchup, and transaction outcomes
under different team/era profiles without changing the players' underlying
ratings.

`team_decision_profile.v1` has two independent sections:

- the general manager controls rookie opportunity, veteran preference,
  waiver-asset protection, trade aggression, and deadline buying bias;
- the manager controls matchup intensity, tactical adaptability, lineup
  patience, position flexibility, physical-fourth-line preference, four-line
  usage, and fatigue rotation.

Every signed trait is bounded from -1 through +1, carries evidence games and an
evidence label, and is shrunk by `games / (games + 20)`. Team or named-person
profiles must be calibrated from dated evidence; unsupported reputation cannot
be emitted as confirmed behavior.

## The Cut boundary

GM opportunity, veteran, and waiver traits may change training-camp selection.
Rookie opportunity applies only to an explicit `rookie_eligible` input; a
non-incumbent veteran or PTO is not silently classified as a rookie.
The output exposes the resulting `management_behavior_delta` for every player.
Manager deployment traits cannot change who makes the roster.

This distinction prevents a coach's preference for a checking line from being
misrepresented as a front-office prospect evaluation.

## The Bench game plan

`bench_game_plan.v1` is opponent- and date-specific. It consumes one complete
lineup, one team/era decision profile, opponent tactical style, player role
evidence, and schedule load for both teams. It emits:

- a tactical response to north-south rush, east-west possession,
  dump-and-chase, heavy-cycle, counterattack, or balanced opponents;
- one matchup forward line and shutdown defense pair, optionally targeting a
  named opponent line;
- primary and secondary scoring lines;
- a checking/energy line;
- projected five-on-five line shares;
- hard-match confidence and a signed relative-fatigue edge;
- warnings and evidence disclosures.

Matchup suitability changes by opponent. Rush defense weights transition and
gap defense; dump-and-chase weights retrieval proxies, physical play, and exits;
heavy-cycle defense weights low-zone defense and strength; possession defense
weights disruption and transition. These are disclosed estimates until
shift-aligned event evidence supports learned weights.

## Schedule interaction

Schedule load is bilateral. IceLines distinguishes playing on a back-to-back
from catching the opponent on one, and also considers third-in-four load and
travel. Home last change raises hard-match confidence. Fatigue can cause a
four-line manager to increase checking-line share or a star-heavy manager to
shorten the bench.

Season reporting should aggregate:

- own back-to-backs and third-in-four games;
- opponents caught on back-to-backs and third-in-four games;
- rested-versus-tired and tired-versus-rested games;
- travel-adjusted rest advantage;
- expected and simulated points attributable to the schedule imbalance.

## Evidence and learning

Historical calibration should use dated opening rosters, prior NHL experience,
waiver decisions, line/shift deployment, home/away matchup usage, scratches,
call-ups, trades, deadline standings, and compressed-schedule TOI shares.
Calibration must be rolling-origin: a 2026-27 as-known simulation may use only
behavior observed before its cutoff.

The system learns team/era behavior, not permanent personality. Coaching or GM
changes close one profile interval and begin another.

### Historical calibration contract

`team_behavior_calibration.v1` supports an explicit one-, two-, or three-season
lookback. Only unique completed seasons before the target season are accepted.
Recency weights are 1.00 for the newest season, 0.65 for the prior season, and
0.40 for the third. Trait opportunity counts provide the separate confidence
shrinkage used by `BehaviorTraitView`.

Source adapters should emit `TeamBehaviorSeasonFactsInput`, whose fields contain
auditable team and league success/opportunity counts. The shared builder turns
the team-versus-league rate difference into a bounded signed observation. This
supports facts such as rookie opening-roster decisions, veteran retention,
waiver protection, transaction activity, deadline buying, hard matches,
opponent-specific changes, lineup continuity, off-position deployment,
physical-fourth-line use, balanced four-line games, and fatigue rotation.

The calibrated document reports every trait's contributing seasons, signed raw
value, confidence-adjusted value, opportunities, and evidence label. Derived
traits are estimates even when their underlying counts are confirmed. Missing
signals remain neutral and `no_read`; simulated evidence is rejected to prevent
the simulator from training on its own output.

### Leadership research lane

Observed team behavior and leadership research are separate evidence lanes.
`team_behavior_research.v1` records dated GM/head-coach tenures and
citation-backed profile markers. Every marker names the person, role, trait,
publication date, HTTPS source, source title, an IceLines-authored paraphrase,
direction, and editorial confidence. Source quotations are not stored.

```text
icelines icecast behavior-research \
  --rankings examples/team-behavior-rankings-2026-27.json \
  --research path/to/team-research.json \
  --out path/to/enriched-profile.json
```

The target date resolves the active GM and coach. A predecessor's markers are
retired automatically after a personnel change. A current leader may carry
dated evidence from prior teams, with an eight-year recency decay. Research is
capped at 25% when observed behavior exists. Where quantitative evidence is
`no_read`, accepted research may establish a lower-confidence `reported` read.
It never becomes `confirmed` and never represents a quality or character grade.

Web-search collection should prefer primary appointment/termination sources
for tenures and directly attributable reporting for style markers. Each refresh
rechecks active leadership before rankings are rebuilt. Controversies are only
included when they materially support a defined behavior trait, remain framed
as attributed history, and are not transferred to a successor.

### League rankings

`team_behavior_ranking.v1` ranks same-target-season profiles on all twelve
confidence-adjusted behavior traits. It also emits separate equal-weight GM and
manager indices, team coverage percentages, ranks, and percentiles. Raw
calibrated values and evidence opportunities remain beside the effective value.

No-read teams are retained in the document with no rank or percentile. A team
cannot appear league-average merely because its historical adapter is missing.
The composite indices describe decision style, not front-office quality,
coaching quality, standings value, or forecast strength.

## Deferred work

- automatic player role-score construction from shift-aligned defensive,
  transition, retrieval, forecheck, hit, and penalty evidence;
- transaction, waiver, opening-roster, and shift adapters for the seven traits
  that remain `no_read` in the first all-team bundle;
- all-32-team leadership-tenure and citation-marker research manifests;
- opponent style classification by rolling team play evidence;
- game-plan strength deltas inside the season Monte Carlo loop;
- in-game score-state changes, shortened benches, and matchup escape behavior;
- goalie rotation behavior under back-to-backs.

## Rangers sensitivity checkpoint

Using the same 10,000 camp trials and seed, with every player input held fixed,
Cole Beaudoin's behavior-neutral make probability is 78.57%. An isolated
rookie-opportunity trait of +0.5 raises it to 80.54%; -0.5 lowers it to 76.38%.
This 4.16-point end-to-end span is a sensitivity test, not a calibrated claim
about the Rangers. The published Rangers fixture remains neutral until its
historical count facts are harvested, and Beaudoin is explicitly marked
`rookie_eligible`.
