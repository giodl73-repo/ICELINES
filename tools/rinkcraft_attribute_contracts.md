# Rinkcraft attribute-contract research tools

`rinkcraft_attribute_contracts.py` validates an authored definition registry and
renders readable Markdown/HTML. Python 3.12 standard library only. The registry
belongs to the separate Rinkcraft research repo. This tool does not ingest player
statistics or write production profile grades.

```powershell
py -3.12 tools/rinkcraft_attribute_contracts.py --registry C:/src/Rinkcraft/research/attributes/registry.v1.json
py -3.12 tools/rinkcraft_attribute_contracts.py --registry C:/src/Rinkcraft/research/attributes/registry.v1.json --output target/compact-sheet/attributes
py -3.12 tools/rinkcraft_attribute_contracts.py --registry C:/src/Rinkcraft/research/attributes/registry.v1.json --output C:/src/Rinkcraft/docs/attributes/reference --markdown-only
py -3.12 -m unittest discover -s tools/tests -p test_rinkcraft_attribute_contracts.py -v
node icelines-browser/e2e/rinkcraft-attribute-contracts.mjs
```

The last check requires the generated preview already served on loopback 8771.
It starts no server. Generated reports and screenshots stay under ignored target.
Paths above describe the local research arrangement; no checkout mutation other
than the explicitly selected output directory occurs.

Pure helpers implement the candidate balanced-facet index, empirical midrank
reference and frozen ordinal mapping. They require already-calibrated facet
quantiles and a complete reference; they do not estimate or validate either.
`publication_state` checks audit-record presence and mechanical display gates.
The caller remains responsible for substantiating real review, reliability,
calibration and source evidence. Synthetic passing records in tests establish
software behavior only. They never authorize a real player's grade.

Validation is structural and checks the current draft's fixed facet weights and
unvalidated status. It cannot prove construct validity or adjudicate hockey
scenarios. Rinkcraft records the separate producer-applied semantic audit.
Existing conditional-finishing pilot inputs, labels and implementation remain
unchanged; no production native/WASM pipeline change is claimed.
