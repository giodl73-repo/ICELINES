# Event-level conditional finishing experiment

Research-only, offline tool: `rinkcraft_event_finishing.py`. Python 3.12 standard
library; no network, production schema writes or ability scores. This is a
separate exploratory protocol from the annual Gamma-Poisson pilot.

```powershell
py -3.12 tools/rinkcraft_event_finishing.py --config C:/src/Rinkcraft/studies/2026-11-09-event-finishing/config.json --output target/compact-sheet/event-validation
py -3.12 -m unittest discover -s tools/tests -p test_rinkcraft_event_finishing.py -v
node icelines-browser/e2e/rinkcraft-event-finishing.mjs
```

The browser check requires the generated report on the existing loopback 8771
server. With input=null, the CLI emits awaiting_observed_event_data and no metrics;
that state is not a successful experiment. Explicit synthetic inputs always emit
synthetic_demonstration and cannot establish empirical validity.

## Provider-neutral JSONL input

One JSON object per unblocked attempt; preserve saves and misses, not goals only.
These are our field names, not inferred names from an unseen provider dictionary.
Adapters must document definitions, completeness, omissions, corrections and rights.

| Field | Contract |
|---|---|
| game_id, event_id | Positive integer IDs; pair unique; every game has one date |
| game_date | Canonical YYYY-MM-DD; all events inside declared windows |
| shooter_id, goalie_id | Positive integer actor IDs; unknown actors rejected |
| position_group | F or D; consistent for each player in the experiment |
| competition, season, season_type | NHL; explicit integer season; regular |
| situation, both_goalies_present | 5v5; literal true, not integer 1 or a truthy string |
| attempt_scope, outcome | unblocked; goal, saved or missed; blocked attempts require another protocol |
| baseline_id | Exactly the configured immutable model version |
| baseline_probability | Finite numeric probability strictly between zero and one |

Observed input manifest entries require path, sha256, source_url, accessed,
adapter_version, rights and coverage_note. Baseline metadata requires id, sha256,
trained_through, available_from, feature_columns, audit_locator and immutable=true.
Training must end before the first experiment date, and availability must precede
or equal that date. Only the explicit pre-shot feature allowlist is accepted.

These are admission checks on supplied metadata. A hash/date string does not prove
that a model actually obeyed them; inspect its artifact, training lineage and
source corrections. The runner cannot independently verify a hidden vendor model,
complete missing events or establish historical real-time availability. No vendor
adapter is provided or assumed validated.

## Statistical specification

Let p_i be the externally frozen pre-shot probability and y_i the goal indicator.
The league shift delta maximizes the training Bernoulli log likelihood with a
N(0, league_prior_sd^2) penalty. Conditional on that fitted delta, each player's
beta_j maximizes the same likelihood for that player's training events with a
N(0, player_prior_sd^2) penalty:

    logit(q_i) = logit(p_i) + delta + beta_j

This is a sequential penalized MAP construction, not a jointly fitted hierarchical
posterior. All players share the fixed declared prior strength; unseen players
use beta=0. No role/class terms or goalie-technique interpretation. Priors and
sensitivity alternatives are authored exploratory choices, not optimized truth.

Use the first two chronological blocks to fit the final training model; the
third remains held out. Compare baseline, league-only and player-MAP predictions
using mean log loss, Brier score, expected goals, fixed probability-bin counts
and separate F/D observed/predicted totals. Stability pairs use a common league
offset learned only in the first block, and separate player effects in each block.
The minimum attempt count for those pairs is a display/design threshold, not a
universal sample-size guarantee. Log-loss evaluation clips probabilities to
[1e-15, 1-1e-15] only as a floating-point safeguard.

Paired resampling selects complete held-out games, retaining every selected game
and both sides' attempts. Report the player-MAP minus league-only log-loss interval;
negative favors player MAP. It is conditional on fitted training and the supplied
baseline, not a player-ability interval or complete model/source uncertainty.
With fewer than two held-out games, no clustered interval is issued. Repeated
players and temporal dependence can require additional sensitivity analysis.

Declared train/heldout separation prevents direct target-outcome fitting; it does
not remove hidden model leakage, omitted pre-shot context, goalie/team confounding,
selection or revised-source bias. No automatic empirical promotion occurs even
if an observed run improves a metric. No 1–20 capability score is produced.
