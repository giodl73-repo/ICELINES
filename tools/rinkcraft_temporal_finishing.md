# Temporal finishing: pooling and change uncertainty

Offline exploratory research tool, Python 3.12 and the pinned dependencies in
`rinkcraft-capability-requirements.txt`. No network or production grading. Uses
already sealed annual MoneyPuck data through the existing parser/prior estimator.
Authored study inputs and findings belong to Rinkcraft.

```powershell
py -3.12 tools/rinkcraft_temporal_finishing.py --config C:/src/Rinkcraft/studies/2026-11-10-temporal-finishing/config.json --output target/compact-sheet/temporal-finishing
py -3.12 -m unittest discover -s tools/tests -p test_rinkcraft_temporal_finishing.py -v
node icelines-browser/e2e/rinkcraft-temporal-finishing.mjs
```

The browser test needs the report served by existing loopback 8771. It starts no
server. Generated JSON/HTML/screenshots stay under ignored target.

## Fixed reference and chronological comparisons

Fit one Gamma prior from the earliest season's full qualified positive-xG skater
cohort, not the showcase. Freeze it for all later comparisons. Given goals G,
exposure X and fixed Gamma(shape=a, rate=b), the conditional season effect has
Gamma(a+G, b+X). This reuses the existing count-model approximation; a multiplier
of one means conversion matching supplied xG. It is not causal mechanics or
complete Shooting. Empirical-Bayes hyperparameters are treated as fixed.

The middle-season replication compares the oldest-season posterior mean with the
frozen league mean. For the latest target, compare league, oldest season only,
most recent prior season only and both prior seasons pooled. All policies receive
the same realized target xG and never target goals. The pooled player estimate
is (a+G_old+G_recent)/(b+X_old+X_recent); an absent historical row contributes no
evidence and falls back toward the prior, not a recorded zero-goal season.

Qualification retains the existing source-config game/minute rules; target
eligibility and realized xG are known retrospectively. This is a conditional
model test, not a pre-season forecast or unseen confirmatory study. Report raw
counts, Poisson deviance, absolute errors, F/D totals and paired-player bootstrap
intervals. Negative left-minus-right deviance favors left. Player resampling does
not resolve game/team/time dependence or omitted model/source uncertainty.

## Change under a specified model

For independent fixed-prior season effects theta_old ~ Gamma(A,B) and
 theta_new ~ Gamma(C,D), the ratio theta_new/theta_old has a scaled beta-prime
law with shapes (C,A) and scale B/D. The implementation uses SciPy ppf/cdf/sf;
neutral tests compare it with independent Gamma simulation, symmetry and inverse
intervals. Conditional independence, constant within-season effects and fixed
hyperparameters are assumptions, not empirically established properties.

Report median ratio and a central configured interval, plus probabilities of a
relative increase/decrease under that model. With a 1.2 increase threshold,
increase is ratio>1.2 and a literal 20% decrease is ratio<0.8. Decrease is not
silently defined as the reciprocal 1/1.2. Missing or insufficient-exposure seasons
withhold the change comparison; actual zero goals remain actual zero goals.

No automatic changed/unchanged class is assigned. Interval-exclusion counts across
players are descriptive, not multiple-testing-controlled discoveries. Wide or
zero-crossing change uncertainty does not establish stability. Export all declared
prior-strength sensitivity factors, retaining the central prior and recording
any exploratory amendment in the authored config. Never select a flattering
prior using target outcomes. Goalie selectors explicitly mark this skater
component inapplicable; no goalie technique value is inferred.

Source/config/parser/tool hashes and exact code commit are exported. Unknown xG
model vintage, historical data revisions, omitted support/goalie effects and F/D
calibration limits remain visible. Software success or lower retrospective loss
does not release a Rinkcraft capability score. Framework/action versions and
old assessments remain unchanged.
