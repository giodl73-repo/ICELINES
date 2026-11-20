# Explicit units in the native and browser studios

Open `/rinkcraft/game` on native or the browser creator and choose **Arrange
explicit units**. The new panel uses unit rules 0.2; the separate **Try a
possession** panel is visibly marked legacy 0.1 and retains its timeline/branches.

The unit workbench displays twelve named members, roster addresses, classes,
task bindings, selected-hero capabilities, control/cover, exact dice/counts and
history. Hero selection is inspection, not a task reassignment. Setup edits stay
pending until **Start new unit scene**; rolls are disabled meanwhile and download
still saves the live recorded source. Invalid edits/imports preserve that source.

**Place sheet character & restart** copies the current fictional sheet build into
a compatible home member and starts a new scene with duties unchanged. Rust
projects compatible positions and validates rules/placement. The unit controller
persists across sheet builds, allowing sequential squad construction without
silently replacing recorded characters. Current sheet and scene hero remain
explicitly distinct. Imports use embedded rules; missing operator setup or failed
reload does not prevent importing or erase a valid unit. Legacy conversion is an
explicit new copy, not an in-place migration.

Shared Rust `rinkcraft_unit_studio` owns create/import/reset/hero selection/
placement/step/auto/freeze commands and layout eligibility. It shares blueprint
construction with the existing Rust CLI. Native POST `/api/v1/rinkcraft/unit-scene`
and WASM `unit_studio` execute the same bounded command envelope. JS/TS handles
controls, rendering, files and transport only. No game arithmetic/rules changed.

Operator configuration on native:

```text
ICELINES_RINKCRAFT_UNIT_BLUEPRINT=C:/src/Rinkcraft/game/studio/lineup-0.2.json
ICELINES_RINKCRAFT_UNIT_RULES=C:/src/Rinkcraft/game/rulesets/possession-units-0.2.json
```

Browser optional copies are `public/rinkcraft-unit-blueprint.json` and
`public/rinkcraft-unit-rules.json`, alongside character rules. Caller-supplied rule
bytes are embedded/verified; browser transport does not follow arbitrary blueprint
file references. The new JS asset and present setup files are in the verified
offline shell. The distribution allowlist adds exactly those three names.

Executed validation:

- 75 Rinkcraft Rust tests and 26 focused Node tests pass; TypeScript/JS checks pass.
- Ten-template native/WASM command parity, prior 0.2 replay equality and shared
  blueprint CLI output/verification pass. Unit/rules schemas and legacy saves stay intact.
- Actual native/browser UI passes hero inspection, persistent units across sheet
  changes, compatible placement, same-dice Defending 1 versus 20 outcomes, all
  duty bindings, pending/live export, rejected edits/imports, oversized files,
  missing setup/failed reload, explicit legacy copy and card preservation.
- Browser offline play/creation/placement/reload, keyboard focus, 390-pixel layout,
  three-theme text/hero-highlight colors and inspected screenshots pass.
- Independent pypdf inspection verifies two one-page 612×792-point Letter
  character exports, with identity/attributes retained and workbench excluded.
- Legacy timeline/import/comparison/possession and creator E2E regressions pass.
- Dist verifies 49 shell assets and 75 supplied packages with actual WASM queries;
  app plus one package is about 806 KiB gzip, below the existing 10 MiB budget.

Corrections during validation: focus was explicitly established before the
keyboard selection test; theme selection uses its accessible combobox role.
Visual review isolated the light panel from global dark table variables and
replaced attacker-relative outcome headings with resulting phases. A native
rebuild corrected an embedded-asset/source mismatch after final UI edits. Cold
browser failures were traced to refused ES-module connections from the local
Python preview's installed accept queue of five. The retained quiet loopback
helper uses queue 128; five fresh starts then passed without module failures.
These are preview/test setup corrections, not new hockey rules.

Replay using matching built servers on 8772/8773:

```powershell
node icelines-browser/e2e/rinkcraft-unit-studio.mjs
node scripts/verify-browser-dist.mjs
```

Ignored artifacts: `target/unit-studio/`. Its retained `serve-browser.py` serves
only the owned dist directory on loopback with queue 128. No hosted validation,
push, merge or publication. No new shots, saves, fatigue, NHL abilities or balance
claim. Rinkcraft decision 0033 governs the interpretation and execution packet.
