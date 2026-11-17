# Fixed-model conditional finishing in Rust

Research-only arithmetic port in `icelines-core::view_model::rinkcraft_finishing`.
The offline batch example consumes a supplied frozen model and historical rows.
It does not fit a prior, choose position factors, compute interval quantiles,
bootstrap comparisons, rank players or release capability grades. No network or
new dependency is required. Existing Python research remains a reference oracle;
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
Prior fitting, quantile/bootstrap replay and UI/native/WASM integration require
separate explicit increments. No change to the existing research model's
calibration limitations or production policy follows from numerical equivalence.
