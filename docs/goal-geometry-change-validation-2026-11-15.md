# Same-player goal geometry — 2026-11-15

Extends the native Analyze Goal feature with a shared Rust endpoint comparison.
The former nearest-distance subtraction could compare two different opponents.
The UI now identifies nearest-opponent changes and displays distance changes for
each matched player separately. No coverage, pass, possession, lane-opening or
fault classification is introduced.

## Contract

- Compare frame i with max(0, i-10), with no earlier comparison at frame zero.
- Preserve player identity; compute native Euclidean endpoint displacement and
  same-player before/after distance to the scorer.
- Missing scorer at an endpoint means unknown scorer movement/separation change.
- Report endpoint appearance/disappearance separately from matched identities;
  mark gaps in intervening observations, without interpreting substitutions.
- The optional plot overlay connects endpoints, not the actual skating path.
- Native HTML, portable HTML and JSON consume one Rust comparison object. No
  separate browser arithmetic or physical/time calibration is introduced.

## Producer-applied review

These are self-review lenses from `.roles/ROLE.md`, not independent reviewers.

| Lens | Finding |
|---|---|
| HART / KEEL | Additive, separately versioned geometry object; original descriptive audit retained. No player grades or game-state assignments changed. |
| TAPE / EDGE | Join by official player ID, retain missing scorer and observation gaps, distinguish changed nearest identity. |
| FORGE / BENCH | Rust owns calculations; synthetic fixtures exercise nearest switch, missing endpoints, observation gaps, bounded window and translation invariance. |
| PACE / SCOUT | Endpoint displacement is not speed or travelled distance; unknown active personnel and post-goal movement prohibit automatic duty conclusions. |
| GLASS | Same-player table and optional reference overlay use explicit units and limits; mobile table scrolls within its container. |

## Validation

- `cargo build -p icelines-cli --locked`: passed; native Askama and CLI compiled.
- `cargo test -p icelines-fetch --lib goal_analysis --locked`: 10 passed.
- `cargo test -p icelines-web --test goal_analysis_routes --locked`: 2 passed.
- `node --check icelines-browser/e2e/goal-analysis.mjs`: passed.
- `rustfmt --edition 2021 --check icelines-fetch/src/goal_analysis_motion.rs`: passed.
- Five cached goals re-exported through the final CLI with `--no-live`.
- `node icelines-browser/e2e/goal-analysis.mjs`, artifact override
  `target/goal-geometry-2026-11-15`: passed against native loopback8786,
  JSON and portable HTML. Checks include geometry equality, all frames, table
  row identity counts, overlay toggle, missing scorer, first frame, playback,
  desktop/mobile overflow, source expansion and zero page errors. Screenshots
  inspected at desktop and390px mobile; the table scrolls within its container.
- Independent acceptance arithmetic checked699 frames and8407 matched-player
  comparisons, including scorer-distance changes, presence sets and gap flags.
  All five original descriptive audits remain structurally identical.
- There are116 changed-nearest-identity comparisons across the overlapping
  ten-frame windows. These are not116 distinct hockey events, handoffs or errors.

Artifacts: ignored `target/goal-geometry-2026-11-15/`. Native preview:
`http://127.0.0.1:8786/goal-analysis?target=2025021167/996`.
Rust remains the product implementation; the independent arithmetic acceptance
script does not introduce a Python runtime dependency.

No network research, fresh NHL data acquisition, player-grade changes, simulation
calibration, main merge, push, hosted build or publication was performed.
