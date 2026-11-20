# Rinkcraft evidence-readiness workbench

This local research CLI joins the versioned attribute contracts with a sealed
existing finishing-pilot export, its six historical MoneyPuck CSVs and one saved
NHL EDGE capture. It performs no downloads, model fitting or production updates.
The authored mappings, input hashes and acquisition sequence belong to Rinkcraft.
Python 3.12 standard library only.

```powershell
py -3.12 tools/rinkcraft_evidence_readiness.py --config C:/src/Rinkcraft/studies/2026-11-09-evidence-readiness/config.json --output target/compact-sheet/readiness
py -3.12 -m unittest discover -s tools/tests -p test_rinkcraft_evidence_readiness.py -v
node icelines-browser/e2e/rinkcraft-evidence-readiness.mjs
```

The browser check requires the existing loopback server on 8771. It starts no
server. Generated HTML/JSON/screenshots belong under ignored target. Change the
explicit local paths as needed; configs resolve data_root relative to themselves.

The workbench distinguishes experimental partial estimates, insufficient exposure,
historical raw measurement and uncollected task measurements. Adjacent outcomes
are separate: a field's presence does not mean it measures a capability. Missing
player-season rows remain missing; a real zero remains zero. Finishing estimates
appear only in their original target season. Historical speed/shot-speed captures
appear only for their recorded player and season. Goalie all-strength context
cannot silently become a five-on-five measurement.

Every headline is null and every facet validation flag false. Source hashes,
config, current implementation hash/commit and model lineage are exported. The
acquisition plan must cover the exact registry, but its priorities are authored
implementation choices, not measured scientific importance. Broad task definitions
still need model-specific sample plans, reliability criteria and validation.
Neither coverage counts nor the acquisition queue are a rating scale.
