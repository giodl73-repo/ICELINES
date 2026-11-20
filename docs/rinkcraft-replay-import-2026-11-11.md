# Rust-owned possession replay files

Rust now constructs and verifies `rinkcraft.possession-replay.v1` exports. Existing
v1 files remain accepted. Core APIs live in `rinkcraft_possession.rs`:
`export_replay_from_json`, `import_replay`, and `import_replay_from_json`.
They validate bounded rules/requests and the fictional hero, replay legal actions,
then compare the entire saved session against the reconstruction. No validation
algorithm is duplicated in the UI. Verification is internal consistency under
embedded authored rules, not a signature or NHL assessment.

Native routes: POST `/api/v1/rinkcraft/possession/export` with the existing
`{character_rules,rules,request}` body; POST `/api/v1/rinkcraft/possession/import`
with `{replay}`. WASM methods delegate to the same Rust functions. Imports are
bounded at 256 KiB; nested limits and the sixteen-opportunity bound still apply.

The UI uses the verified scene's hero/rules and labels it as a saved replay. It
never replaces the character sheet above. Continuing, restarting and exporting
retain the imported context. **Use current character** loads the sheet's hero
and installed operator rules. Errors preserve the last valid scene. Self-contained
imports work without installed possession rules and in the loaded browser offline.

Rust CLI, with a nonzero error status on rejected files:

```powershell
cargo run -p icelines-core --example rinkcraft_replay_verify --locked -- path/to/replay.json
```

Validation: eight possession tests (including three new replay tests), eight
creation and six arithmetic regressions; 26 focused Node tests; actual native/WASM
export/import parity for ten templates; legacy file acceptance; modified-session
rejection; different saved character rules/goalie/scene rules; continuation and
reset; scene/card preservation; oversized files; offline browser import; import
without installed scene rules; desktop/390-pixel screenshots; existing possession
and creator E2E regressions; CLI valid/invalid files; and browser-dist verification.

Replay using existing owned native/browser servers and supplied external assets:

```powershell
node icelines-browser/e2e/rinkcraft-replay-import.mjs
node icelines-browser/e2e/rinkcraft-possession.mjs
node icelines-browser/e2e/rinkcraft-game-studio.mjs
```

Artifacts are ignored under `target/replay-import/`. Rinkcraft retains the study,
governance and source/build lineage. No gameplay-rule version, NHL profile or
publication workflow changed. Source commits remain local.
