# Evidence at a glance

The first block inside the closed profile dossier formats existing Rust-selected
measurements, historical assessment and optional finishing readout. No signal
ranking, repeated-evidence rule, interval or capability score is calculated here.
Browser TypeScript and native Askama/Rust view formatting share the same inputs.

- Measured signature follows the existing `EvidenceCard.leading` order and shows
  raw values, annual percentiles and measured-peer counts. High values are not
  necessarily better and do not determine a class.
- History support is shown only when history player ID, last supplied season
  and assessment last season match the selected numerical snapshot. Otherwise
  a mismatch/unavailable note replaces the summary. The detailed supplied history
  keeps its own explicit dates.
- Measurement gaps come from existing documentary feature `unmeasured_stats`,
  deduplicated in catalogue order. They are not inferred from outcomes or ranked
  numerically. An absent gap list is not complete capability coverage.
- Optional finishing preserves its status and conditional scope. Goalie
  non-applicability is not a zero score. No complete Shooting/1–20 grade follows.

Documentary evidence date and numerical season remain separate. The compact
front is unchanged; the brief is inside the closed dossier and omitted in print,
including when the dossier was opened before printing. No class, assignment,
simulation, data source, schema or mathematical model changes.

Validate the focused profile/sheet Node tests and native template compilation.
Use the existing frozen profile-window fixtures with
`ICELINES_REQUIRE_EVIDENCE_BRIEF=1` and a separate
`ICELINES_PROFILE_WINDOW_OUTPUT`. The acceptance harness checks all15 selections,
source-selected signal order, matched history, catalogue gaps, goalie status,
desktop/mobile geometry and10 single-page Letter exports with the dossier open.
It renders generated browser snapshots, not a full native/browser application.

No new WASM engine is needed: use the retained shared-Rust projections. Dated
named-player artifacts/provenance live in PUCK; interpretation in Rinkcraft.
