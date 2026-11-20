# Fixed-model conditional finishing in Rust

Research-only arithmetic port in `icelines-core::view_model::rinkcraft_finishing`.
The offline batch example consumes a supplied frozen model and historical rows.
It does not fit a prior, choose position factors, bootstrap comparisons, rank
players or release capability grades. Optional conditional Gamma intervals use
pinned statrs with only its std feature; runtime remains offline. Existing Python
research remains a reference oracle;
it is not called by the Rust runtime.

```powershell
cargo test -p icelines-core --lib rinkcraft_finishing --locked
cargo run -p icelines-core --example rinkcraft_finishing_replay --locked -- REQUEST.json OUTPUT.json
```

The request contains `model` and a nonempty `cases` array. Each case has `target`,
`history` and `observed_goals`. Observed target goals are used only for evaluation;
the public `predict` function's Target type has no outcome field. Unknown fields,
goalie positions, duplicate targets, invalid chronology, duplicate/wrong-player
history and invalid numeric inputs are rejected. All cases must succeed before
the example writes its result. Output includes the exact request SHA256.

```json
{
  "model": {
    "calibration_season": 20232024,
    "shape": 4.0,
    "rate": 2.0,
    "forward_factor": 1.0,
    "defense_factor": 2.0
  },
  "cases": [{
    "target": {"player_id": 1, "season": 20252026, "position": "D", "xg": 3.0},
    "history": [{"player_id": 1, "season": 20242025, "position": "C", "goals": 2, "xg": 4.0}],
    "observed_goals": 3
  }]
}
```

This is a neutral arithmetic example, not calibrated hockey parameters.
The caller supplies an appropriately sourced frozen model; a chronology field
alone cannot prove how that model was trained. Qualification and cohort selection
are caller responsibilities, recorded in replay provenance.

## Equations and units

Let global prior be Gamma(shape a, **rate** b), historical goals G, historical
provider xG X, and target realized xG T. For position group g use fixed factor c_g;
adjust historical exposure H = sum(c_h * X_h), using each row's own position.

- Global baseline: (a/b) T.
- Global player: ((a+G)/(b+X)) T.
- Position baseline: c_g T.
- Position player: ((a+G)/(a+H)) c_g T.

Global multiplier posterior: Gamma(a+G, b+X). Position residual posterior:
Gamma(a+G, a+H), relative to the position-adjusted baseline. Multiplying this
residual by c_g gives Gamma(a+G, (a+H)/c_g), on the raw-provider-xG scale.
The output distinguishes these quantities; neither is a complete Shooting score.

For Gamma(s,r), mean = s/r and variance = s/r². Conditional predictive goal
variance = T * E[theta] + T² * Var(theta). This combines Poisson process variation
and conditional parameter uncertainty. It excludes prior/factor estimation,
provider-model error, context confounding and team/game dependence. Variance and
posterior parameters are not credible/predictive interval endpoints.

No history returns the corresponding baseline with uncertainty retained.
Zero target xG returns zero conditional goals; it does not resolve multiplier
uncertainty or prove low skill. Positive goals at zero historical xG are invalid.
Poisson deviance at positive goals and zero expectation is infinite, serialized
as null with an explicit root explanation, never epsilon-repaired or zero error.

## Replay scope

The retained position-finishing experiment supplies the frozen 2023–24 model.
Its 2024–25 evaluation uses only 2023–24 player history; 2025–26 uses both earlier
years. The replay preserves the original retrospective qualification and realized
target xG. It is not a preseason forecast or fresh prospective validation.

The first replay checked 664 + 678 player-season cases, four policies each,
posterior moments against SciPy, process/parameter predictive variances, aggregate
deviances, F/D totals and negative batches. Authored source inputs and methodological
decisions belong in Rinkcraft; dated named-player artifacts belong in PUCK.
Do not transplant legacy candidate 1–20 component scores into the compact sheet.
Prior fitting, bootstrap replay and UI/native/WASM integration require
separate explicit increments. No change to the existing research model's
calibration limitations or production policy follows from numerical equivalence.


## Optional equal-tailed multiplier intervals

Set top-level `interval_probability` to a finite value strictly between zero and
one, for example0.9. Requests omitting it retain the earlier point/moment behavior.
Interval mode emits schema `icelines.fixed-finishing-replay.v0.2` and separate
`interval_quantiles_computed` / `bootstrap_intervals_computed` flags. The legacy
combined metadata flag is retained; true means quantiles were computed, never
that bootstrap work also ran. Prediction values remain unchanged.

Each row's `credible_intervals` contains global multiplier, position residual
and raw-provider position multiplier intervals. Bounds are inverse-CDF values
at (1-p)/2 and1-(1-p)/2; they describe parameter uncertainty conditional on fixed
model inputs. No normal approximation or goal-count prediction interval is used.
The implementation brackets and bisects the small lower/upper tail at unit rate,
using statrs CDF/survival evaluation, then divides by the requested **rate**.
This avoids the generic inverse-CDF routine and upper-tail subtraction. SciPy
oracle uses1/rate as scale. See pinned [statrs Gamma API](https://docs.rs/statrs/0.19.1/statrs/distribution/struct.Gamma.html).

Quantiles must be finite, positive and ordered. Both lower CDF and upper survival
probabilities are checked against the requested tail with maximum absolute2e-12
or relative2e-9 numerical tolerance. Invalid, unrepresentable or failed numerical
results return an error; the batch runner writes no partial report. These are
numerical checks, not hockey calibration or evidence that credible intervals
have the stated frequentist coverage.

The neutral integration fixture records an independent SciPy grid across shapes,
rates and interval masses. It contains no NHL observations. Run:

```powershell
cargo test -p icelines-core --test rinkcraft_gamma_intervals --locked
```

The separate full-cohort replay checks conditional intervals against SciPy while
preserving all earlier point/deviance/moment fields. Main player cards remain
unchanged until a separately reviewed integration. No legacy1–20 component
scores are re-released through these interval fields.
