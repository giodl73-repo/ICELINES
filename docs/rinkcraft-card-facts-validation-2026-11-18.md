# Sourced background and completion audit — 2026-11-18

Owner: fill the five existing actual contrast cards, using public sources.
This increment fills ten historical facts, not all card sections or capabilities.
Research authority and source records stay in Rinkcraft; named-player replay
outputs/provenance stay in PUCK. No framework, scoring or simulator changes.

## Inputs and scope

- Rinkcraft `assessments/2026-11-18-card-facts-pilot.json`, derived from the
  immutable `2026-11-18-repertoire-pilot.json`.
- Baseline `target/compact-sheet/real-repertoire/fixtures.json`: five retained
  2024–25 profiles with documentary selector explicitly as-of 2026–27.
- Rinkcraft `assessments/2026-11-18-card-completion-worklist.json`: field plan
  and 110 required facets copied from the registry, with its SHA256. Thirty
  headline attributes remain unrated. Partial measurements are not discarded.
- Sources SRC-152–SRC-158, claims CLM-086–CLM-090: original official historical
  records, brief facts and locators only. Makar's debut marked postseason;
  Barkov PDF publication precision retained as year-only.

## Contract and validation

See [factual identity contract](../design/specs/rinkcraft-card-facts.md).
Rust enforces source/date selection, duplicate fields, required context and
expiry. Browser and native templates present the same selected facts.

Completed local checks:

- Six focused `rinkcraft_character` Rust tests: legacy selection/fingerprints,
  required support/context, duplicate fields, date boundaries and expiry.
- TypeScript compilation and 18 focused Node tests, including factual source
  disclosure, missing legacy fields and separation from six unrated abilities.
- `cargo check -p icelines-web --locked`: native Askama compilation.
- Release WASM projection: five profiles, ten source-date checks, four negative
  cases and an expiry case. Existing numerics, identity, methods, tendencies and
  twelve repertoire entries are unchanged.
- Theme acceptance: 48 desktop/24 mobile cases across three fictional specimens
  and five actual profiles; six theme print exports. Themes preserve text.
- Factual UI acceptance: ten source disclosures/links, five mobile cards,
  thirty unrated attributes, zero page errors and five actual-profile PDFs.
- `pypdf` independently verifies all five actual exports as one-page 612×792pt
  Letter files with factual, measured-record and unrated labels. Fox desktop
  screenshot inspected locally.
- Rinkcraft foundation: 427 Markdown files, 1591 local links, 158 sources,
  92 claims, ten producer-applied review lenses. Structural, not hockey validity.
- `rustfmt --check` and repository diff whitespace checks.

## Replay

Use pinned repository tools. Set `CARGO_TARGET_DIR=F:/src/ICELINES/target` and
TEMP/TMP to the owned worktree's `target/windows-tmp` on this machine.

```powershell
cargo test -p icelines-core --lib rinkcraft_character --locked
cargo check -p icelines-web --locked
cargo build -p icelines-wasm --target wasm32-unknown-unknown --release --locked
node icelines-browser/node_modules/typescript/bin/tsc --project icelines-browser/tsconfig.json
node --test icelines-browser/tests/rinkcraft-sheet.test.mjs icelines-browser/tests/rinkcraft-profile.test.mjs icelines-browser/tests/rinkcraft-evidence-guide.test.mjs
```

Generate `target/compact-sheet/inputs/pkg-card-facts` using the pinned
wasm-bindgen 0.2.118 and built `icelines_wasm.wasm` with `--target web`.

```powershell
node tools/project_rinkcraft_repertoires.mjs target/compact-sheet/inputs/pkg-card-facts C:/src/Rinkcraft/assessments/2026-11-18-card-facts-pilot.json target/compact-sheet/real-repertoire/fixtures.json target/compact-sheet/card-completion
node tools/prepare_rinkcraft_theme_studio.mjs target/compact-sheet/card-completion/fixtures.json target/compact-sheet/theme-studio
node icelines-browser/e2e/rinkcraft-theme-studio.mjs
node icelines-browser/e2e/rinkcraft-card-facts.mjs
```

E2E requires the prepared loopback server on 8771; it does not start one. Both
scripts accept `ICELINES_THEME_URL`. Private checks/fixtures/receipts retained in
`target/compact-sheet/card-completion`; source commit and file hashes retained
in the dated PUCK provenance packet.

## Limits

Generated local preview, not rebuilt or deployed full native/browser applications.
No hosted checks, push or merge. Public-source acquisition only; no vendor
contact, licensed feed, private account or purchase. No new task observations or
ability calibration. 2024–25 overlaps the proposed calibration epoch; a separate
retrospective design would be required to rate those cards independently. Next:
public 2024–25 game-state deployment, followed by original duty/pressure accounts
and required task opportunity evidence.
