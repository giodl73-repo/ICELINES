# Position-conditioned finishing comparison

Research-only offline CLI, using already sealed annual MoneyPuck summaries.
Python 3.12 and `rinkcraft-capability-requirements.txt`; no downloads or ability
grades. New model namespace; earlier reports/assessments remain unchanged.

```powershell
py -3.12 tools/rinkcraft_position_finishing.py --config C:/src/Rinkcraft/studies/2026-11-10-position-finishing/config.json --output target/compact-sheet/position-finishing
py -3.12 -m unittest discover -s tools/tests -p test_rinkcraft_position_finishing.py -v
node icelines-browser/e2e/rinkcraft-position-finishing.mjs
```

Browser check requires the report served on loopback 8771. It starts no server.
Generated output stays under ignored target. Research config/results belong in
Rinkcraft. All source inputs and config/shared code hashes are exported.

Fit the global Gamma prior from the earliest qualified positive-exposure cohort,
and position factor c_g = sum(goals_g)/sum(xG_g) separately for F and D in that
same cohort. Require positive calibration support for both. Freeze factors and
prior for both later evaluation years; no later outcomes set them.

Compare four models on identical qualified rows and realized target xG X:

- global baseline: mu*X;
- global player: (a+G_history)/(b+X_history)*X;
- position baseline: c_g*X;
- position player: (a+G_history)/(a+sum(c_h*X_h))*c_g*X.

The position-player residual prior is Gamma(a,a), mean one, retaining the global
shape a as an authored variance-matching choice, not a refitted optimal prior.
Each historical row uses its own recorded position factor; the target uses its
own. This transfer across position groups is an explicit model assumption and
switches are flagged. Missing history supplies no evidence; the player prediction
then equals its appropriate baseline. No within-position rank or score conversion.

2024–25 evaluation uses only 2023–24 player history; 2025–26 uses both earlier
seasons. All factors stay frozen at 2023–24 even for the latter. Report four-policy
Poisson deviance, F/D totals, group deviance and relative calibration errors.
Compare position/global baselines, position/global player models, and position
player/simple position baseline with paired-player bootstrap intervals. Negative
left-minus-right favors left. Training-factor/prior uncertainty and game/team
clustering are omitted; full qualified cohort and model inputs remain conditional
on observed xG and retrospective qualification. F/D totals can improve while
individual error fails to improve. No empirical capability validation follows.

This experiment is explicitly motivated by already observed subgroup residuals.
Later years are not an unseen confirmatory test, even though software prevents
later outcomes from fitting the earlier model. Source vintage, shot/task mix,
selection, opponent goalie/support and cross-position comparability remain open.
No recommendation to replace production xG or award a player grade is made.
