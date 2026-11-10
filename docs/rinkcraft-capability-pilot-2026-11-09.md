# Experimental capability-component pilot

CHG-144 / WP-RINKCRAFT-20, 2026-11-09. Baseline c568df6.

Research scope is the owner-approved data audit and Skating/Shooting pilot.
Reusable prototype: `tools/rinkcraft_capability.py`, with pinned scientific
research dependencies. This is a local CLI experiment, not a production Rust/WASM
scorer. It reads authenticated annual source files and a saved tracking capture,
fails on source mutations/duplicates/invalid outcomes and emits JSON plus HTML.
No network, repository data acquisition, app mutation or headline score release.

Rinkcraft owns the authored protocol, selected contrasts, six source hashes and
config: `studies/2026-11-09-capability-pilot/config.json` and
`docs/research/capability-component-protocol-01.md` in the separate research repo.
The full source population sets a Gamma-Poisson conditional finishing prior.
2023–25 player totals fit the prior; 2025–26 goals are excluded from fitting.
Point forecasts condition on realized later xG and compare against league and
weak-pooling baselines with a fixed player bootstrap. Raw historical evidence,
latest-season-only and available three-season estimates remain labeled separately.
The candidate finishing display scale uses frozen prior-CDF knots, not raw outcome
ranks or preexisting 1–5 bands. A 10-xG display gate is authored, not validated.
No 20+ tier or headline aggregation weights are invented.

Checks: ten synthetic tests cover provenance, duplicates, temporal overlap,
held-out-goal independence, zero/new-player handling, shrinkage/interval width,
deterministic bootstrap, monotone bounded scale, unreleased headline/goalie
attributes and HTML escaping. These validate code behavior, not hockey constructs.

The comparative conditional check may pass while prospective validation is false:
modern provider snapshots have unverified point-in-time xG features. Saving hashes
does not establish historic model availability. Conditional posterior intervals
omit hyperparameter/provider uncertainty and context confounding. Player bootstrap
also leaves team/opponent dependence unresolved. No empirical taxonomy promotion.

Run with Python 3.12 and `tools/rinkcraft-capability-requirements.txt`:

```powershell
py -3.12 -m unittest discover -s tools/tests -p test_rinkcraft_capability.py -v
py -3.12 tools/rinkcraft_capability.py --config C:/src/Rinkcraft/studies/2026-11-09-capability-pilot/config.json --output C:/src/Rinkcraft/outputs/capability-pilot/2026-11-09-01
```

Generated results seal exact source/config/implementation hashes, source commit,
dirty state, environment, seed, bootstrap count and command. The report audits all
six skater and six goalie categories; only the narrow finishing component has an
experimental estimator. Existing forms, API contracts, source dossiers, numerical
vectors and browser/WASM build remain unchanged.
