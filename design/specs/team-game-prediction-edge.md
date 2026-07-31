# Team Game Prediction Edge

**Status:** active implementation contract
**Parent:** Team Season Forecast
**Owner:** IceCast

## Outcome

Improve held-out NHL game probabilities by retaining chronological Elo as the
stable prior and adding only dated evidence that Elo cannot observe: the
players available to dress, the expected or confirmed starting goalie,
score-adjusted expected-goal form, special-teams matchup quality, and bounded
opponent-style suitability. The same enhanced game probabilities must feed the
existing 32-team season simulator.

The feature is successful only when it improves out-of-sample probability
quality. A more detailed explanation without a held-out scoring gain is useful
analysis, but is not promoted as prediction edge.

## Forecast vintages

Every prediction declares one of three non-interchangeable vintages:

| Vintage | Evidence boundary | Intended use |
|---|---|---|
| `preseason` | Before the game date; roster branches may be probabilistic | Schedule and season-distribution forecast |
| `game_morning` | Captured on the game date; reported lineups and goalie states remain uncertain | Morning board and daily refresh |
| `pregame_confirmed` | Captured on the game date near lock; confirmed scratches, dressed lineup, and starter may be used | Final pregame probability |

No vintage may read a source captured after its `forecast_at`, a final result,
or a later vintage. Historical replay must reconstruct the same boundary.

## Evidence contract

For each scheduled team-game side, IceLines may carry:

- roster strength, on the existing comparable 0-100 scale;
- availability strength for the players expected to dress, 0-100;
- starting-goalie quality, 0-100, with `confirmed`, `reported`, `modeled`, or
  `unavailable` evidence state;
- trailing starting-goalie GSAx form and workload readiness, with NHL player
  identity, appearance count, and the same dated evidence state;
- score-adjusted expected-goal share over a named trailing window, 0-1;
- opponent-adjusted expected-goal share, computed from each selected game's xG
  share relative to that opponent's strictly-prior trailing xG form;
- special-teams quality, 0-100, derived from separate PP and PK evidence;
- bounded opponent-matchup suitability, -1 to +1.

Every optional value carries a sample count or evidence state where applicable.
Unavailable values remain absent. They are not zero-filled. Small-sample form
and special-teams values shrink toward neutral before entering the model.
Evidence-package floating-point values are normalized to nine decimal places
before sealing. This keeps standard `serde_json` file round trips stable without
changing the workspace serializer or invalidating unrelated canonical artifacts;
non-canonical package values are refused.

## Model

The candidate model starts from a convex IceLines/Elo probability blend, moves
to log-odds, and applies signed home-minus-away feature contributions:

```text
logit(p_home) = intercept
              + baseline_logit_weight * logit(p_blend)
              + roster_weight * roster_difference
              + availability_weight * availability_difference
              + goalie_weight * goalie_difference
              + goalie_form_weight * shrunk_goalie_form_difference
              + goalie_workload_weight * goalie_workload_difference
              + xg_weight * shrunk_xg_difference
              + opponent_adjusted_xg_weight * shrunk_opponent_adjusted_xg_difference
              + special_teams_weight * shrunk_special_teams_difference
              + matchup_weight * matchup_difference

logit(p_calibrated) = calibration_intercept
                    + calibration_slope * logit(p_home)
```

Calibration is selected only from predictions made in earlier outer holdouts;
the first holdout uses identity calibration. This prevents the current or a
future season from setting its own probability temperature.

The initially checked model is explicitly `evaluation` authority. Production
weights must come from the rolling-origin trainer and preserve their training
seasons, explicitly versioned feature set, regularization, calibration, and
fingerprint. A prospective registration seals that feature-set identifier;
adding a feature requires a new registration before the untouched season.

## Forecast document

`team_game_prediction_edge.v1` owns:

- source `team_game_forecast.v1` identity;
- vintage and exclusive evidence timestamp;
- frozen model parameters and authority;
- base, Elo-blended, and enhanced home-win probabilities;
- one row per feature with raw difference, effective shrunken difference,
  log-odds contribution, probability delta, and evidence coverage;
- an active-feature, weight-adjusted evidence confidence score and an
  evidence-stability range produced by a fixed half-unit perturbation of
  uncertain evidence. This range is sensitivity analysis, not a statistical
  confidence interval, and excludes model-selection and outcome uncertainty;
- the enhanced `TeamGameForecastView` consumed by season simulation;
- explicit missing-evidence warnings and a content fingerprint.

The overlay must preserve schedule, teams, final labels, and all non-probability
forecast metadata. It may update probabilities, expected standings points,
favorite, confidence, grading fields, and appended explanation factors only.

`team_game_prediction_edge_card.v1` compares one game across independently
sealed vintages only when the source forecast and complete model are identical.
It projects the focused-team probability, opening-to-latest movement, latest
factor contributions, coverage, authority, and source fingerprints through the
shared `card_document.v1` grammar. Away-team probabilities and contributions
are inverted in core, never by a renderer.
An optional closing-market benchmark may be projected only when it carries its
own capture time and source fingerprint. It appears in a separately labeled
benchmark section and provenance row; it is never added to factors, training
observations, or forecast probabilities.

## Training and promotion

Training uses dated feature vectors and final outcomes, with final outcomes
joined only after prediction features are frozen. Seasons are validated through
rolling origins: every holdout is scored using parameters fit only on earlier
seasons.

The promotion gate requires all of the following:

1. at least five graded holdout seasons;
2. pooled Brier improvement of at least 0.001 versus chronological Elo;
3. pooled binary log-loss improvement versus Elo;
4. Brier improvement in at least four holdout seasons;
5. expected calibration error no worse than Elo;
6. roster and goalie ablations do not improve the candidate;
7. no feature survives solely through one season or one team;
8. exact model and input fingerprints replay on Windows, Linux, and macOS,
   including lossless JSON round trips for floating-point features.
9. a sealed prospective registration predates every forecast in the final
   untouched holdout and binds the exact training configuration.
10. every newly introduced candidate feature clears its own configured Brier
    gain threshold versus an otherwise identical ablated model.

Failure keeps the model evaluation-only and names the failed checks. It does
not prevent users from inspecting challenger forecasts.

## Source boundaries

- Official NHL schedule, roster, boxscore, play-by-play, and transaction data
  remain the preferred dated authorities.
- MoneyPuck-derived xG is optional and must retain its season, capture, schema,
  and coverage disclosures.
- Opponent adjustment seals the selected team rows and every opponent-prior
  fingerprint. An opponent game on or after the selected game date is
  ineligible.
- Historical reconstruction separates `evidence_cutoff_at` from
  `retrieved_at`: later retrieval is permitted, but only rows strictly before
  the forecast boundary may contribute. Live captures require both timestamps
  to match.
- Goalie starter observations reuse the shared confirmed/reported/modeled/
  unavailable semantics; a fantasy recommendation is not starter authority.
- Retrospective official boxscores may reconstruct pregame identities only from
  team, dressed-player ID, explicit starter flag, NHL game date, and start
  time. Score, decision, save, shot, goal, and TOI fields are forbidden from
  the identity projection and fingerprint.
- Window organization health may inform preseason roster-depth uncertainty but
  is never itself a game probability or fitted outcome label.
- Closing betting odds may be used only as a benchmark unless a separately
  approved model explicitly includes them as features.

## Required surfaces

1. Core builder and validator.
2. Fetch-owned dated input package and cache adapter.
3. `icelines icecast edge` JSON/text command.
4. `icelines icecast edge-train` rolling-origin validation command.
5. `icecast season-simulate` bridge for baseline or edge-enhanced forecasts,
   without probability recomputation.
6. `icecast edge-card` UI-neutral vintage comparison projection.

## Refusals

Reject duplicate game/vintage rows, unknown games or teams, evidence after the
forecast boundary, morning/pregame evidence for a different game date,
non-finite or out-of-range values, unsupported model authority, missing source
fingerprints, result-bearing training rows without a frozen feature timestamp,
and promotion claims that do not pass every gate.
