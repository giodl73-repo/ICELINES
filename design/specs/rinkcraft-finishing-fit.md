# Training-only finishing fit in Rust

This ports the numerical fitting/readout part of the earlier fixed component pilot.
It is an experimental conditional conversion model, not a full Shooting attribute,
causal talent estimate, current-season assessment or production rating.

## Preserved design

Use the three hash/length-pinned MoneyPuck annual skater CSVs for 2023–24 through
2025–26. Preserve the provider's `5on5` label, unique player-season rows, integer
counts and f64 xG/exposure. The parser rejects missing columns, duplicate actors,
wrong years/positions, invalid numbers and positive goals with zero xG. Annual
summaries do not independently establish every contextual condition of the full
attribute contract, including the registered opportunity/goalie-presence scope.
The existing general MoneyPuck f32 parser is unchanged; this separate parser
retains the precision and fields required by the numerical pilot.

Pool the two training years once per player. Prior eligibility is >=10 summed GP,
>=300 summed five-on-five minutes and positive xG. Fix mu=sum(G)/sum(X), estimate
Gamma shape a by bounded marginal likelihood, and set **rate** b=a/mu.
The bounds remain [0.05,100000], as in the authored config. Source files, GP/minute
floors, target year, interval probability and xG display gate are not retuned.

The Rust fitter evaluates a 129-point log-shape grid, brackets the lowest grid
region and applies bounded golden-section refinement. Likelihood sums use
compensation. This differs numerically from SciPy's bounded optimizer, so the port
checks shape/rate at relative 1e-4 and the objective at absolute 2e-6, with separate
checks on downstream predictions and intervals. Boundary fits are flagged. The
canonical training-input fingerprint is order-independent; outside-epoch rows and
duplicate player-seasons are rejected. Minimum exposure is an authored scope rule,
not evidence of reliability or sufficient construct coverage.

## Per-player readouts

The fitted prior and historical goals/xG produce a training-only forecast for the
2025–26 player, conditional on that player's **observed target xG**. No target
goals enter the fit or forecast. Compare league mean, weak pooling and empirical
Bayes using Poisson deviance and absolute error. Predictions and diagnostics are
reported separately for forwards and defensemen; no position tuning is introduced.

Retrospective target-season and named three-season pooled components use
Gamma(a+G, rate=b+X), with 90% equal-tailed intervals from the existing shared Rust
Gamma implementation. Pooled estimates assume a constant effect over that window,
not career or latest-current ability. Half/double prior-strength means are supplied
as deterministic sensitivity, not a probability interval. The >=10-xG display
gate is retained; lower-exposure estimates remain explicitly insufficient.
A real observed zero-exposure history is distinguished from absent history;
both have a prior-only multiplier when the history contributes no informative xG.

Intervals condition on fitted hyperparameters and provider xG. They exclude
prior-fit uncertainty, provider-model error, game/team dependence and confounding.
Neither parameter shrinkage nor narrow conditional intervals certifies a capability
construct. The original player bootstrap is not recomputed by this port, and the
conditional comparison/promotion gate is not re-certified. Provider source-vintage,
prospective and position-comparability limitations remain unresolved. No 1–20
number or headline score is emitted; all three other Shooting facets remain open.

## Verification and custody

Five focused Rust tests passed (three fitting/update, two source-parser tests).
The fresh SciPy/source oracle verified 1,844 training rows from 2,784 scoped source
rows, 787 training players, 678 target players, 2,034 forecasts and 4,068 interval
endpoints. Endpoints match SciPy at relative 1e-9 / absolute 1e-11; forecasts against
the independent fitted prior use relative 1e-5 / absolute 1e-7. Group totals, policy
metrics, status gates and sensitivity calculations are also checked.

The actual executable was run against a clearly synthetic +1 target-goal source
copy. The fitted prior and all 678 historical forecasts stayed exactly unchanged;
evaluation and the retrospective component changed. Synthetic bytes receive a test
URN, not the provider URL. Original source bytes remain untouched. Config-checksum
and overwrite guards reject bad requests. A full replay is byte-identical; later
header/empty-cohort guards leave the valid replay unchanged. Those guards prevent
an empty target from being represented as a completed evaluation.

The current legacy assessment copy hashes to 09857af0f5cf29dfab57faf9ab1ce40e1b1e6b50a9eb9d018fbb85495a2c95f9,
which differs from the older study README's recorded assessment hash. It is retained
as a comparison copy, not represented as the original sealed assessment. The
original config still matches 249d9c03a850c7b4a445887c5fea8c3d35198064d7f113938984e4ce2b9ae23d;
all three source CSVs match its pinned bytes/hashes. Fresh source/SciPy computation
provides the independent numerical oracle. The Rinkcraft research repo is read-only.

```text
cargo test -p icelines-core --lib rinkcraft_finishing_fit --locked
cargo test -p icelines-sources --lib moneypuck_finishing --locked
cargo run -p icelines-sources --example rinkcraft_finishing_fit --locked -- JOB_JSON NEW_DIRECTORY
py -3.12 tools/verify_rinkcraft_finishing_fit.py JOB_JSON OUTPUT_DIRECTORY LEGACY_ASSESSMENTS_JSON NEW_RECEIPT_JSON
py -3.12 tools/test_rinkcraft_finishing_fit_leakage.py JOB_JSON BASE_OUTPUT NATIVE_EXECUTABLE NEW_TEST_DIRECTORY
```

Production computation is Rust; Python/NumPy/SciPy serve only as verification and
synthetic-test orchestration. Dated readouts/findings live in PUCK; raw and generated
witnesses stay in the external artifact packet. Producer-applied evidence/methods
lenses are self-review, not an independent expert panel. No public cards, native
profile assembly, WASM exports, simulation parameters or hosted deployment change.

The retained evidence packet also passed a packet-local replay using copies of all
three raw skater CSVs. A derived config rebinds only paths and omits unused goalie
sources; the original config is preserved separately. Fitted prior, all 678 readouts,
policy evaluation and limits match exactly. Job/config/source-path provenance fields
change as declared. Synthetic target-goal copies use an explicit test URN and are
kept separate from the captured primary sources.
