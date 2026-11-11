# Rinkcraft profile studio

The NHL character front retains the accepted compact section mapping and keeps
raw statistical records separate from capability fields. Seven documentary
classes have consistent crests/colors across sheets and the atlas. A structured
atlas introduces each player by class, specialty and summary before opening.
Historical evidence cutoffs remain visible. Unassessed capabilities remain unrated.

A separate fictional character creator uses shared Rust in native APIs and browser
WASM. It accepts operator-provided `rinkcraft.game-rules.v1` rules and exports
`icelines.game-character.v1`. The source rules live in Rinkcraft, not this repo.
The authored playtest supports standard arrays, seeded 4d6-drop-lowest rolls and
manual 1–20 scores. Six goalie capabilities are separate from six skater ones.
No scores, class bonuses or generated statistics are written into NHL profiles.
Player inspiration is provenance, never a rating input. F1/F2/F3 remain assignments.

Import rebuilds under the versioned rules/hash and rejects mismatched results.
Changing a score uses manual creation. Exported task skills/methods are repertoire
labels; they are not validated event probabilities or an implemented game engine.
Under-pressure and response numbers explicitly reference capabilities, without
inventing additional composite scores. Rules balance and action resolution remain
future work.

## Local operation

Set `ICELINES_RINKCRAFT_GAME_RULES` to an explicit authored rules JSON file for
native `/rinkcraft/game`. Documentary catalog/history configuration remains
separate. Native creation/import routes are stateless, accept bounded JSON, and
return the standard data/meta envelope. Game errors do not mutate NHL evidence.

For browser builds, place optional operator assets in `icelines-browser/public/`:
`rinkcraft-game-rules.json`, `rinkcraft-characters.json`, and existing numerical
catalog/history assets. The build copies and fingerprints supplied assets in the
offline shell; it never fetches or invents them. Missing rules show a retry control.
Native NHL dossiers remain readable without JavaScript; character creation needs it.

## Validation and replay

From the repository root, with the installed pinned dependencies:

```powershell
$env:CARGO_TARGET_DIR='F:/src/ICELINES/target'
cargo test -p icelines-core rinkcraft_game --lib --locked
cargo build -p icelines-cli --bin icelines --locked
node icelines-browser/build.mjs
node --test icelines-browser/tests/rinkcraft-profile.test.mjs icelines-browser/tests/rinkcraft-sheet.test.mjs icelines-browser/tests/offline.test.mjs icelines-browser/tests/metadata-build.test.mjs
node scripts/verify-browser-dist.mjs
node icelines-browser/e2e/rinkcraft-game-studio.mjs
node icelines-browser/e2e/rinkcraft-profile-studio.mjs
```

E2E scripts require an existing owned native server on 8772 and full browser build
on 8773. They do not start or stop servers. The showcase script requires the
external 26-player Rinkcraft catalog and retained public 2024–25/2025–26 packages.
Barkov uses his explicitly available older package; no appearance is fabricated.
Results, exports, PDFs and screenshots go to ignored `target/profile-studio/`.

Completed checks before the final rebuild: eight Rust tests; 26 focused Node
tests; exact native/WASM parity for ten game templates; creation/seed/manual/
export/import/tamper/mobile/print and missing-rules recovery; 26 native and browser
NHL sheets across seven classes; F/D/G inspired builds; real NHL JSON unchanged;
six compact-sheet fallback/Letter cases; full browser-distribution verification.
The existing 25-player app audit additionally passed exact native/WASM numerical
cards and displayed values, no-JS keyboard/native comparisons, offline/mobile
browser comparison and all three appearance themes.
The study in Rinkcraft owns the final requirement audit and source/build lineage.
No hosted CI, release, merge or push is implied.
