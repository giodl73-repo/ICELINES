# Reading help for the compact profile

The profile's additive `evidence_guide` field is authored explanatory copy from
`icelines-core::view_model::rinkcraft_evidence_guide`, not player evidence,
an observed task rate, a capability battery or an estimator. It does not count
toward dossier sources or supply an assigned role, class or limitation.
Legacy JSON without the field deserializes to an empty list; clients omit the
guide when it is absent. The profile schema remains v1 with an additive field.

Native Askama and the browser renderer show one closed **How to read this
profile** disclosure after the compact sheet and before the full dossier.
The sheet's boxes and grades are untouched. The disclosure is hidden for print,
including when opened on screen. Unknown positions receive only common topics;
explicit goalie and skater positions receive their respective explanations.

Topics distinguish record/capability, skater delivery, reception, pre-outcome
choice, contextual direction/tempo, and goalie puck exchanges. These are a
reading aid for existing distinctions, not complete definitions of all six
attribute batteries. Player evidence retains its own source, window and context.
The Rinkcraft research source/decision record remains authoritative for the
provisional measurement design; provider pass labels never become scores here.

Focused checks:

```text
cargo test -p icelines-core --lib rinkcraft_profile --locked
cargo check -p icelines-web --locked
cargo build -p icelines-wasm --target wasm32-unknown-unknown --release --locked
node icelines-browser/node_modules/typescript/bin/tsc --project icelines-browser/tsconfig.json
node --test icelines-browser/tests/rinkcraft-evidence-guide.test.mjs icelines-browser/tests/rinkcraft-sheet.test.mjs icelines-browser/tests/rinkcraft-profile.test.mjs
node icelines-browser/e2e/rinkcraft-evidence-guide.mjs
```

The browser acceptance script requires explicit preprojected guide fixtures at
`ICELINES_GUIDE_FIXTURES` or the ignored local preview fixture path. It does not
fetch source data, manufacture evidence or start a native server. It checks
unchanged compact-front markup, skater/goalie topic separation, missing legacy
guide support via unit checks, mobile overflow, print hiding and profile wiring.
Native template compilation is not a full native runtime acceptance claim.

Local validation, 2026-11-18: seven focused Rust profile tests and 16 focused
Node checks pass. TypeScript compilation, native Askama check and release WASM
build pass. Five retained 2024–25 fixture projections acquire the guide from
rebuilt WASM with all previous fields unchanged. Fox/skater and Shesterkin/goalie
desktop, 390px mobile, closed disclosure, print hiding and full profile wiring
pass; independent PDF inspection finds one 612×792-point page per card and no
guide text. This is a generated preview, not a hosted/full native deployment.
