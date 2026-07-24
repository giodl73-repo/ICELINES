# Line Combination Simulation

**Status:** Implemented (foundation)

## Product contract

IceLines can compare legal lineup configurations, translate each configuration
into an explicit team-strength assumption, and run a season in which a coach may
change configurations after a review window. The UI-neutral outputs are intended
for JSON, web, TUI, cards, and later historical replay.

The product language is **The Cut** for training-camp roster selection,
**The Blender** for lineup comparison, and **The Bench** for the adaptive
coaching policy. Opponent-specific deployment and team/era decision traits are
owned by [`management-behavior-simulation.md`](management-behavior-simulation.md).

## Questions answered

- Which invitees make the opening roster, and who is on the bubble?
- What is the probability that a prospect breaks camp with the NHL team?
- Which players and units are projected to fit best together?
- Which complete lineup is strongest under the selected scoring mechanism?
- How does each lineup change expected wins, points, playoffs, and Cup odds?
- How often does a result-aware coach change the lineup during a simulated season?
- Which configuration finishes the season most often?

## Preseason roster-selection contract — The Cut

Line optimization must not assume the opening roster is already known. Each
season simulation begins with a camp pool containing NHL incumbents, AHL
players, drafted prospects, professional tryouts, and injured/non-roster
players known as of the scenario date. Each seeded camp trial samples player
readiness and camp performance, then selects 12 forwards, 6 defensemen, and 2
goalies before The Blender assigns units.

Selection is constrained rather than a raw top-score sort. The model retains:

- translated prior performance by league and age, with uncertainty increasing
  when NHL-equivalent evidence is weak;
- GP confidence, development range, health, conditioning, and camp performance;
- center/wing and left/right flexibility under the selected manager policy;
- contract status, waiver eligibility/risk, slide rules, and roster limits as
  explicit decision costs rather than hidden overrides;
- incumbency as a configurable coach/management preference, not ground truth;
- role coverage, including enough centers, penalty killers, power-play options,
  physical/checking players, and injury replacements.

`training_camp_forecast.v1` reports each invitee's make probability, cut
probability, most common role, bubble rank, players most often displaced, and
the most common 12F/6D/2G roster. Every season trial must carry its selected
roster into The Blender and The Bench instead of applying one fixed lineup to
all trials. Cup/playoff odds therefore marginalize over roster uncertainty.

`training_camp_lineup_set.v1` is the implemented UI-neutral Cut-to-Blender
bridge. It retains the probability and stable roster IDs of each requested
top camp branch and embeds a complete `team_lineup_projection.v1`. Dual-eligible
forwards may be reassigned from wing to center before off-wing completion so a
center-capable camp roster does not become an incomplete lineup. The document
reports `retained_probability`; consumers must not renormalize omitted camp
outcomes without an explicit policy.

Each embedded lineup also carries two estimated power-play units and two
estimated penalty-kill units. PP units require one dressed defenseman as an
explicit quarterback plus four dressed forwards; PK units require two dressed
defensemen and two dressed forwards. The estimator ranks official prior-season
PP or shorthanded TOI per game after GP-confidence shrinkage
(`games / (games + 20)`). A recorded zero is preserved as evidence but is not
treated as evidence of a special-teams role. Units never pull from extras or
invent deployment for prospects without data. Missing candidates produce
explicit warnings rather than silent fallback assignments.

`training_camp_blender_set.v1` independently scores every retained lineup and
uses the modal camp branch as the cross-roster strength reference. Its
`opening_roster_policy` is directly consumable by `team_season_scenario.v1`.
IceCast samples exactly one choice per team at the beginning of each season
trial and carries its strength delta through the regular season and playoffs.
When the lineup set omits long-tail outcomes, their probability becomes an
explicit `camp-roster-residual` at modal strength with no fabricated roster IDs.
The forecast reports configured and sampled probabilities for auditability.
Detailed lineup and Blender artifacts remain bounded by
`--max-lineup-branches`, while the compact season policy independently uses
`--season-max-roster-branches` (default 3,000). The Rangers fixture produces
2,669 distinct camp rosters, all of which are scored into the season scenario;
therefore its current residual is zero. Season sampling precomputes cumulative
probabilities and uses binary search so thousands of choices do not add a
linear scan to every team in every trial.

For the Rangers, Cole Beaudoin must enter the camp pool rather than appearing
only as a November call-up event. His output should answer how frequently he
beats an incumbent for an opening-night center/wing role, whom he displaces,
and how that roster branch changes the preferred lines and season forecast.

Historical replay has two modes: `as_known`, using only evidence available at
that camp date, and `retrospective`, using the recorded camp outcome for model
calibration. Retrospective knowledge must never leak into an as-known forecast.

## Evidence boundary

Lineup forecasts must distinguish three things:

1. `confirmed_deployment`: an authoritative lineup assignment supplied by a
   roster or lineup source.
2. `observed_pair`: a measured shared-deployment input. Same-game co-appearance
   is not shift-level chemistry and must be labeled as coarse evidence.
3. `simulated_fit`: an IceLines assumption derived from player scores, position
   eligibility, role balance, and optional pair evidence.

The current foundation supports player scores, simulated-fit inputs, and exact
official historical shift overlap through `--shift-season`. Shared ice is
reported as deployment affinity and can inform candidate fit, but it is kept
separate from positive/negative teammate multipliers. True shift-level
with/without-you and expected-goal evidence remains source-gated.

## UI-neutral lineup document

`line_combination_forecast.v1` contains:

- team, season, method, and evidence disclosures;
- a baseline configuration and ranked candidate configurations;
- complete forward lines and defense pairs represented by stable player IDs;
- talent, role-balance, and pair-evidence components;
- GP confidence (`games / (games + 20)`) used to shrink player scores toward
  the current position-group prior before lineup placement;
- a bounded strength delta relative to the baseline;
- warnings when scores, positions, or observed pair evidence are incomplete.
- player leaderboards for best overall, prior deployment anchors, positive multipliers, and negative
  multipliers. Best-overall order regresses raw score toward neutral 50 by
  `games / (games + 20)`; multiplier/drag rows require labeled pair evidence
  and retain observation counts, helped/hurt partner counts, and authority.

Candidate generation is bounded and deterministic. It compares the submitted
baseline with legal swaps rather than searching every permutation. A caller may
also submit curated coach configurations. Manager policy may permit LW/RW
off-wing experiments and natural centers filling otherwise-empty wing slots;
those candidates remain legal but receive a role-fit penalty rather than being
treated as natural-position assignments. Flexible completion never silently
moves a wing into a vacant center slot.

## Adaptive season policy

An optional `adaptive_lineup_policy` belongs to `team_season_scenario.v1`:

- one team;
- an ordered list of lineup choices and their strength deltas;
- a review interval measured in that team's games;
- a minimum standings-points percentage;
- a maximum number of changes.

Each Monte Carlo trial begins with the first choice. At the end of each review
window, The Bench keeps the current combination when the points percentage meets
the threshold; otherwise it advances to the next choice until the change limit
or candidate list is exhausted. The chosen delta affects subsequent games.

`team_season_forecast.v1` reports per-policy switch probability, average changes,
average games by lineup, and the probability of finishing on each lineup. These
are simulated decisions, not a prediction of a named coach's actual choices.

## Validation rules

- Teams must be NHL abbreviations present in the forecast schedule.
- Review intervals are 2-20 team games.
- Thresholds are finite values from 0 through 1.
- Policies contain 1-12 uniquely identified choices.
- Strength deltas are finite and bounded to -20 through +20 on the existing
  IceCast 0-100 team-strength scale.
- Change limits cannot exceed the available transitions.
- Same seed plus same inputs produces byte-stable policy summaries.

## Deferred work

- automatic NHL/AHL/prospect pool assembly and per-trial opening-roster
  propagation into The Blender and IceCast (the seeded core document and
  explicit JSON/CLI input path are implemented);
- shift-aligned on-ice event and expected-goal ingestion;
- learned trio/pair interactions with rolling-origin calibration;
- injury-driven emergency combinations and scratched-player legality;
- special-teams alternatives, five-forward PP support, and zone-start matching
  (deployment-first PP1/PP2 and PK1/PK2 estimation is implemented);
- game-by-game individual point distributions by assigned unit;
- dedicated Blender web/TUI renderers and a Rangers scenario command.
