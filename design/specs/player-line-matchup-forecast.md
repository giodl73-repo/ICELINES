# Player-Line Matchup Forecast

**Status:** Active implementation contract
**Parent:** Team Game Prediction Edge
**Product language:** The Matchup

## Outcome

IceLines forecasts a game from the players expected to dress, the units they
are expected to form, the opponent those units face, and the manager's likely
deployment. The output is renderer-neutral and supplies the existing
`team_game_prediction_edge.v1` matchup feature; it does not create a second
game-probability model.

The capability must distinguish player quality, complementary role fit,
observed deployment, measured pair/trio performance, opponent suitability, and
manager execution. Shared ice alone is deployment affinity. It is never
presented or scored as causal chemistry.

## Dated player profiles

`player_forecast_profile.v1` contains stable player/team identity, evidence
cutoff, games, even-strength minutes, observed shifts, recency, component
coverage, and 0-100 dimensions for:

- scoring creation and finishing;
- passing and transition;
- forechecking and retrieval;
- defensive suppression;
- physical matchup play;
- discipline and puck security;
- faceoffs, power play, and penalty kill when available.

Confidence is the geometric mean of games, minutes, and shift-volume
reliability, multiplied by recency and component coverage. Each volume term
uses an explicit prior. The profile score shrinks toward the position-group
neutral value of 50. Eleven games therefore cannot carry the same authority as
a full season merely because the raw rate is high.

Unavailable dimensions remain absent. They are not converted to zero.

## Chemistry evidence

`line_chemistry_evidence.v1` identifies exactly two or three players and one
of four evidence kinds:

1. `shift_adjusted_outcome`: shift-aligned goals, shots, or expected-goal
   residual after the declared individual/opponent/deployment baseline;
2. `shift_deployment`: exact official shared-ice intervals without an outcome
   join;
3. `coarse_same_game`: same-game co-appearance;
4. `simulated_fit`: an explicit scenario assumption.

Only shift-adjusted outcome evidence may receive full chemistry authority.
Deployment-only evidence is reported as affinity and contributes zero causal
chemistry. Outcome residuals shrink by shared minutes, shared games, evidence
authority, and pair/trio complexity. Trio evidence has a larger prior than pair
evidence.

### MoneyPuck chronological adapter

`icelines-sources` normalizes MoneyPuck's published pair/trio game files into
stable NHL player IDs, game/date/team identity, 5-on-5 shared seconds, and
score-and-venue-adjusted xG for and against. `lineId` is accepted only when it
is exactly two or three concatenated seven-digit NHL player IDs and agrees with
the declared pairing/line type. Missing columns, ambiguous identities,
duplicate game/situation rows, and negative or non-finite measures fail closed.

`pregame_unit_xg_baseline.v1` is a separate authority for the expectation each
unit must beat. Every row is sealed before its game and must declare individual,
opponent, and deployment components. Same-day baselines are refused because the
MoneyPuck row has a game date but no puck-drop timestamp. Only 5-on-5 rows
strictly before the forecast date are aggregated. The output discloses baseline
coverage and exclusions, preserves distinct outcome and baseline seals, and
then enters the existing `shift_adjusted_outcome` reliability/shrinkage path.

MoneyPuck's published usage terms and credit requirement travel with the source
adapter disclosure. Raw unit xG is never labeled causal chemistry.

## Game matchup

`player_line_matchup_forecast.v1` requires two complete NHL lineups, dated
profiles for their skaters, opponent-style evidence, independently sourced
chemistry evidence, forecast/capture timestamps, and source fingerprints. It
reports:

- confidence-adjusted player profiles;
- each forward line and defense pair's offense, defense, style response,
  chemistry, deployment affinity, and evidence confidence;
- expected line-versus-line and pair support;
- home last-change and manager hard-match execution;
- PP-versus-PK unit suitability as a separate descriptive component;
- one bounded 5-on-5 matchup suitability value per team for the existing game
  edge;
- warnings, disclosures, and a canonical content fingerprint.

Special-teams suitability is not included in the 5-on-5 matchup feature because
the game edge owns a separate special-teams factor. Availability and injury
losses remain in the edge's lineup/availability features. The Matchup evaluates
only the submitted dressed lineup and must not count those losses again.

## Manager and lineup scenarios

The same builder accepts independently authored manager execution confidence
and projected forward-line shares. Home last change can improve matchup
execution; fatigue can reduce it. Alternative legal Blender lineups are built
as separate forecasts. Renderers may compare their probability impact only by
running each sealed matchup document through the same game-edge model.

Mid-season Blender changes remain owned by The Bench. A manager policy may
select a new lineup after a review window, after which subsequent games use the
matching lineup-specific forecast instead of a season-long opening lineup.

## Validation and promotion

Historical validation freezes profiles, lineup knowledge, opponent style,
chemistry windows, and manager assumptions before each game. Required ablations
are:

1. team strength only;
2. player profiles without chemistry;
3. profiles plus pair evidence;
4. profiles plus pair/trio evidence;
5. full matchup and manager execution.

A chemistry or matchup feature is eligible for a registered challenger only if
it improves chronological Brier score and log loss, is stable under
leave-one-season and leave-one-team tests, and its removal does not improve the
candidate. All 32 teams use one method and one prior set; team-specific weights
or handcrafted bonuses are prohibited.

## Refusals

Reject unknown teams, mismatched seasons, duplicate players, incomplete dressed
units, profiles or chemistry rows after the forecast boundary, non-canonical
source fingerprints, chemistry rows spanning teams, invalid samples, duplicate
pair/trio keys within one evidence kind, and a game whose home/away identities
do not match the supplied lineups. Missing evidence produces neutral shrinkage
or a no-read warning, not fabricated confidence.

## Delivery sequence

1. Core profile, chemistry, unit, and game-matchup contracts.
2. Shift/outcome adapter and exact source seals, including the chronological
   MoneyPuck line-game adapter and pregame baseline authority.
3. Bridge into `game_prediction_edge_evidence_package.v1`.
4. CLI JSON builder and UI-neutral comparison output.
5. Chronological observations, ablations, and challenger registration.
6. Rangers/Kraken pilot followed by the identical all-32 pipeline.
