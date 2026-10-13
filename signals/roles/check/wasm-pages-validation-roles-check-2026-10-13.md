---
skill: roles-check
topic: wasm-pages-validation
date: 2026-10-13
roles_used: [bench, wire, keel, forge, edge]
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Pages preservation review

Artifact: explicit hidden-file inclusion in publication and rollback uploads,
and the existing composition test extended with a public .well-known file.
All 13 installed role Markdown files were inventoried previously; the selected
verification lenses were read in this review session. BENCH covers evidence,
WIRE external publication, KEEL scope, FORGE build integrity and EDGE omissions.
This is one agent applying five lenses, not independent release approval.

| Role / ID | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| BENCH B1 | Composition tests now prove a hidden public directory survives | P3 verified | test_browser_pages.py | Keep the .well-known fixture and repository-metadata exclusion |
| BENCH B2 | Local staging cannot prove the uploaded archive contains every file | P3 condition | Pages upload | Inspect the actual deployment artifact before publication |
| BENCH B3 | Ten composition/network-boundary tests pass | P3 verified | Local validation | Require fresh CI before merging |
| WIRE W1 | Pages upload defaults exclude hidden paths, including .nojekyll | P2 amended | Both workflows | Explicitly include hidden files in the composed public site |
| WIRE W2 | The input changes archive membership, not NHL request behavior | P3 verified | Workflow scope | Do not claim relay or upstream coverage |
| WIRE W3 | Hidden files must already belong to the reviewed public site | P3 condition | Staging boundary | Verify source membership; repository metadata remains excluded |
| KEEL K1 | Forward publication and rollback need the same preservation rule | P2 amended | Both workflows | Apply the input to both upload steps |
| KEEL K2 | Workbench replacement remains limited to its subdirectory | P3 verified | stage-browser-pages.py | Preserve current documentation hashes during rehearsal |
| KEEL K3 | No engine, schema, catalog or persistence behavior changes | P3 verified | Diff scope | Keep memory and phone acceptance separate |
| FORGE F1 | The workflow still uses pinned action commits | P3 verified | Upload steps | Preserve pinning |
| FORGE F2 | No Rust target or dependency changes are introduced | P3 verified | Diff scope | Rely on existing build checks without new application tests |
| FORGE F3 | This fix does not itself deploy anything | P3 verified | Dispatch conditions | Use the existing master-only publication controls |
| EDGE E1 | A visible-file-only fixture missed hidden public content | P2 amended | Composition test | Exercise a hidden directory and exact bytes |
| EDGE E2 | Both old v4 and v5 upload defaults excluded hidden files | P3 constraint | Cause | Do not describe this as a newly introduced v5 regression |
| EDGE E3 | Physical Android and memory acceptance remain unmeasured for this patch | P3 condition | Release scope | Do not close the full release gates on workflow CI alone |

## Synthesis

Five lenses; zero unresolved P1 issues, three P2 issues amended, twelve P3
findings/conditions. **APPROVED-WITH-CONDITIONS**: required CI must pass, and
actual archive membership plus deployed documentation hashes must be checked.
WIRE and KEEL agree that publication and rollback must preserve the same files.

Amendments: explicitly include hidden files in both uploads; add a hidden public
fixture to composition verification; verify the deployed archive and documentation
without treating this workflow patch as phone or memory acceptance.
