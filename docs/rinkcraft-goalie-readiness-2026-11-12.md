# Contextual goalie readiness

Three authored readiness challenges attach to verified unit 0.4 snapshots:
Positioning versus receiver Reading before release, Movement versus Passing
during on-target flight with hypothetical lateral context explicitly declared,
and Recovery versus Skating after failed rebound containment. Outputs are angle
set/exposed, across/late and reset/unreset. Ties remain unresolved.

These are readiness assessments, not a new unit rule version or a universal
save score. They preserve the complete source replay and puck state and do not
change subsequent unit rolls, legal actions, goals, saves or possession. A reset
goalie can still face an uncontrolled rebound. Connecting readiness to future
decisions needs a separate versioned gameplay contract. No arbitrary bonus or
automatic goal is fabricated. Lateral context is authored, not derived from
abstract scene geometry. The challenge seed starts its own d20 stream, not the
next unit-scene pair. All figures are fictional game values, not NHL assessments.

The Rust module `rinkcraft_goalie_readiness` owns frozen 0.1 rules, phase/context
validation, d20/half-offset arithmetic, exact counts, outcomes and hashes. Input,
report and assessment have v1 schemas. Reports embed the verified source replay
and recompute both source and result on import. Reports are bounded at 640 KiB;
unit studio commands remain bounded at 768 KiB. Unsupported versions/contexts,
unknown fields and forged results are rejected.

The existing native/WASM studio envelope adds assess/import-readiness commands.
Optional response fields are omitted from older unit-version responses. Rust
projects available tasks. The shared UI exposes a closed readiness section,
independent seed, context declaration, six goalie attributes, numerical trace,
report download/import and unchanged source state. Importing a report explicitly
loads its source unit. Invalid reports preserve valid state; changing the source
clears the displayed report. An identical unit refresh preserves it.

CLI:

```powershell
cargo run -p icelines-core --example rinkcraft_goalie_readiness --locked -- INPUT.json
cargo run -p icelines-core --example rinkcraft_goalie_readiness --locked -- --verify REPORT.json
```

93 focused Rust tests pass. New checks cover frozen 0.4 equality, all three
opportunities, same-source-event/dice low/high contrasts, unused-capability
controls, unresolved ties, phase/version/context rejection, exact import,
source/result forgery, size bounds and studio command preservation. Seeds 0
give each selected contrast; source seed 286 retains 4/5/7 action prefixes.
26 Node tests pass. The Rinkcraft authored packet is
`C:/src/Rinkcraft/game/playtests/2026-11-12-goalie-readiness/`; full artifacts stay
under ignored `target/goalie-readiness`. The UI acceptance script is
`icelines-browser/e2e/rinkcraft-goalie-readiness.mjs`, requiring prepared six-case
CLI projections and matching local native 8772/browser 8773 builds.

The frozen 0.4 fixture comes from prior b719cf3, WASM
`fca5476af6ce20853bd8ed5c057fa59462706ac69caa0deb5588cdda11b50cd5`,
copied before edits. Old unit 0.1–0.4 semantics and default operator setup remain
unchanged. No hosted build, push, merge, NHL calibration, game balance or human/
independent playtest is claimed. Rinkcraft decision 0036 records the design.

All six CLI reports verify and match native/WASM. Actual native and offline
browser checks cover available tasks, source equality, unchanged subsequent
save, explicit lateral context, report download/import, forged-report rejection,
clearing a stale result, unchanged primary character sheet and 390-pixel layout.
Independent pypdf checks confirm two one-page 612 by 792-point Letter exports.
Eight prior UI suites pass, including shot/save/rebound and ten-template creator
parity. Visual review moved the detailed definition into a disclosure and hid
the lateral declaration outside its task; a scoped rule keeps checkboxes compact.
These are presentation changes, not rule changes.
