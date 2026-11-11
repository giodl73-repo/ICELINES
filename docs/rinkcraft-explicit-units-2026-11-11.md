# Explicit units and defensive hero — local validation

`rinkcraft_units` adds separately versioned unit inputs, roster IDs, scene addresses
and task assignments. Each side must have F1/F2/F3 forward-position characters,
D1/D2 defense-position characters and G goalie, with twelve unique IDs/addresses.
Task assignments remain separate from roster positions/classes. Every character
is verified under embedded character-creation rules. Carrier and receiver differ;
high cover is a distinct supporting character; skater duties reject goalies.

Selecting a home hero identifies an existing member without moving that character
into a task. D2 can oppose the counter with Defending while F1 carries/recovers.
Bindings can make a defender carry, a forward cover or a different away skater
pressure/intercept. These are declared abstract tasks, not proof of movement or
deployment feasibility. Forward/defense/goalie addresses are this scene's legend.

The contest/state kernel was extracted from the legacy scene and shared through
internal bindings. Legacy v1 retains its fixed bindings, metadata and prose.
Three neutral v1 outputs were captured from the prior 3e3d3c3 WASM before extraction
and committed as portable regression fixtures. New unit rules are separately
versioned `0.2.0-playtest`; the explicit-unit replay root is distinct from v1.
Unit/duty hashes canonicalize member ordering and bind full character content.
They are not authorship signatures. Legacy freezing verifies and prints a new
copy; it does not overwrite the original or implement a durable lineage archive.

```powershell
cargo run -p icelines-core --example rinkcraft_unit_possession --locked -- BLUEPRINT.json
```

The read-only CLI also accepts `--verify`, `--input`, `--step FILE ACTION`,
`--auto` and `--freeze-legacy`. Blueprint paths resolve relative to the file;
Rust builds all twelve members. Replay/import/continuation share core validation.
Inputs and outputs have explicit 256/512-KiB limits. No network or file write is
performed by the CLI; stdout can be saved as a new file.

Executed checks:

- All **72 Rinkcraft Rust tests pass**, including six new tests for frozen legacy
  equality, explicit conversion, order-independent units, content/duty hashes,
  selected-defender effect, alternate task bindings, roster/hero/duty rejection,
  inconsistent replay rejection, bounds and legal auto-continuation.
- Authored seed-26 neutral probe: all capabilities 12 except D2 Defending 1/12/20;
  first two events and counter dice 19/18 stay identical. Attack total 20 opposes
  defense totals 15/21/25. Outcomes are advanced/stopped/stopped. Stops create a
  loose puck, not automatic home control. Exact stopping counts are 120/229/295
  of 400 ordered d20 pairs, respectively. These are fictional game counts.
- D2 Shooting-only negative control changes unit identity hash but no event;
  hero selection does not reassign a task. Rebinding home/away duties changes
  actual actors/receiver/cover. Twelve IDs and complete characters stay explicit.
- CLI verify/input/step/auto agree; terminal continuation and inconsistent replay
  fail without changing source files. Seven malformed roster/duty blueprints
  return nonzero with no stdout.
- Eight legacy cases preserve complete events and final control/state through
  freezing: three portable neutral snapshots, three retained complete/recovery/
  counter examples and independent prior-WASM defense/goalie heroes.

Two local helper mistakes were corrected without a model change: a seed search
attempted recovery before checking legality, and a JSON comparison removed its
reference before finishing comparisons. Final probes pass. Selected seed 26 is
an illustration, not a population sample. All stats/arithmetic remain Rust;
ignored helpers author/retain inputs and compare/format JSON.

Artifacts stay ignored under `target/explicit-units/`; Rinkcraft owns decision
0032, authored rules/blueprint/protocol, compact results and source/binary hashes.
No receiving, shots/saves, clock/fatigue, real-player ratings or balance claim.
This checkpoint is Rust CLI functionality; native/WASM studio remains at its
separately identified 3e3d3c3 build with its legacy-format controls.
