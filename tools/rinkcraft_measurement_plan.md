# Authored measurement-plan companion

`rinkcraft_measurement_plan::build` validates three explicitly supplied UTF8 JSON
inputs: the draft attribute registry, authored task queue and dated player task
priorities. The exporter is local/offline:

```text
rinkcraft_measurement_plan REGISTRY_JSON QUEUE_JSON PRIORITIES_JSON OUTPUT_JSON
```

Metadata paths inside those documents are never opened. Each input is bounded
to1MiB. Raw-byte SHA256 links bind priorities to queue and queue to registry;
attribute/component ownership, unique task/player IDs, authored selection order,
nonempty denominators/boundaries/missingness, the nine-outcome vocabulary and
unreleased/unfitted status are checked. Unknown task IDs or queue/priority fields, stale fingerprints,
duplicate player tasks and grade/fitting promotion fail without fallback.
The existing PUCK authored-priorities schema is an explicit input adapter; no
named-player defaults or assumptions live in reusable model code.

The output copies the authored tasks and preserves skater/goalie attribute
branches. It does not observe opportunities, estimate a probability, choose
research priorities, verify a player's roster position, assess performance,
establish coding reliability or release a1–20 score. Fingerprints/reference IDs
establish consistency, not semantic or empirical validation of the contracts.
The ten selected tasks do not constitute the full capability battery.

The browser component formats this validated export as a research companion,
separate from the evidence profile. Each task exposes its opportunity,
denominator, candidate measure, current evidence gap and expandable boundaries,
outcomes, censoring and remaining collection rules. Unknown player IDs show no
plan rather than a fallback. Authored plan dates and prospective status remain
visible, including when linked from a historical numerical profile.

## Checks and local preview

```powershell
cargo test -p icelines-core --test rinkcraft_measurement_plan --locked
cargo build -p icelines-core --example rinkcraft_measurement_plan --locked
node icelines-browser/node_modules/typescript/bin/tsc --project icelines-browser/tsconfig.json
$env:ICELINES_MEASUREMENT_PLAN='PATH/TO/RUST-EXPORTED-plans.json'
node icelines-browser/e2e/rinkcraft-measurement-plan.mjs
```

`ICELINES_MEASUREMENT_PLAN_OUTPUT` overrides the companion output directory.
The existing profile-window renderer accepts a root-relative
`ICELINES_MEASUREMENT_PLAN_URL` such as `/measurement-plan/` to attach exact-player
links. Use a new profile output directory; keep old snapshots intact. The companion
supports `?player=ID`; unknown queries hide all player plans until a listed player
is selected. Dated player inputs/artifacts belong in PUCK; research rules and
authored registry/queue remain in Rinkcraft. Neutral validator and rendering code
belong here. No live-app asset loader or native/WASM runtime integration is claimed.

Current acceptance uses retained named-player plans plus frozen Rust profile
projections. No new observations, goal-animation detector, rating, model change,
hosted build, workflow trigger or simulation parameter is part of this increment.


## Unfilled observation worksheet drafts

`worksheet(registry,queue,priorities,player_id)` first runs the same bound-plan
validation, then preserves all declared required-record fields and full parent
observation/evidence policies. It requires definition version, essential record
identifiers and consistent parent outcome/accounting rules. Unknown players and
missing contracts fail without fallback. Native offline export:

```text
rinkcraft_observation_draft REGISTRY_JSON QUEUE_JSON PRIORITIES_JSON PLAYER_ID OUTPUT_JSON
```

Each selected task has a **template**, not an observation. Only player/task/
attribute/component/definition identifiers are prefilled. All other record
fields and all proposed protocol fields are JSON null. `observations` is an
empty fixed-length array in the output type. This API does not accept, import,
validate or score completed observations. Editing a downloaded draft does not
make it valid evidence. Sampling/task envelopes, source rights, coding rules,
review and reliability still need a separate registered protocol.

The companion accepts `ICELINES_MEASUREMENT_WORKSHEETS` pointing to a directory
of native-exported `PLAYER_ID.json` files. Set
`ICELINES_MEASUREMENT_PROFILE_URL` to its linked profile preview path. Downloads
are exposed only for matching player/source bindings and an empty observation
list; mismatched or populated drafts show no link. The renderer is read-only,
with no capture, upload, import, reconciliation or scoring workflow. The harness
checks all source fields/task contracts, download paths, null results, stale/
wrong-player/nonblank rejection, and desktop/mobile layout.

These are research drafts, not approved sampling protocols. No real player
observations, inferred failures, success rates, grades or simulation changes.
