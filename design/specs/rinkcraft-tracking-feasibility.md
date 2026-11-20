# Public tracking source qualification

`icelines_sources::tracking_feasibility::audit` reads the exact public Big Data
Cup 2026 tracking CSV schema. This is source qualification for Rinkcraft's
measurement program, not an opportunity detector or a capability estimator.

The input must have the documented eleven headers in order. Frames are grouped
by literal Image Id. A frame must have a consistent period and whole-second game
clock, and cannot recur noncontiguously. Unsupported schemas/order are rejected;
the adapter does not silently repair or alias them.

The output retains source SHA256, row/frame counts, period/clock coverage,
adjacent-clock relations, image-counter gaps, raw and positioned player-count
histograms, puck-row and puck-position absence, blank jersey labels, observed
tracking-ID/jersey conflicts, coordinate missingness and literal Z categories.
The two missing-coordinate counters can overlap when a row has both missing and
invalid axes. Empty/invalid coordinates stay in the census; they never become
zero positions. An entity row is distinct from a usable XY observation.

Player keys combine the provider Team and Player Id. These are **source keys**,
not authenticated persistent athletes or NHL identifiers. Multiple jersey labels
on one key are reported without selecting a preferred identity. Blank jersey
labels remain blank. A repeated nonblank team/player key within a frame is a
diagnostic, not automatically a duplicate row to delete.

The suffix-gap histogram describes observed image-counter differences, not
elapsed time. Whole-second game clocks can repeat during ordinary subsecond
frames or stopped clocks. A high number of frames per game-clock second does
not authenticate FPS, camera sampling, missing video duration or synchronization
with event data. Never differentiate coordinates into speed, acceleration or
release latency using an assumed FPS without a separately validated timebase.

The documented rink rectangle is used for an out-of-bounds diagnostic. No rows
are removed: players may be off the playing surface. A count of twelve player
rows does not establish twelve usable positions, two goalies, correct manpower,
visibility of a feasible passing option or documented defensive duties. Z values
are source categories, not a validated model of stick/body/puck motion.

These qualifications matter across all six attributes:

| Attribute | Information that coordinates alone do not establish |
| --- | --- |
| Skating | Calibrated timebase, qualifying effort, task geometry, correct exit state and persistent identity |
| Handling | Playability, purposeful control, identified challenge, intentional release and receiver attribution |
| Passing | Intended target, actual playable arrival, timing window, deception and a complete attempt denominator |
| Shooting | Intended aim, quality-preserving release, calibrated puck velocity and a non-circular shot baseline |
| Reading | Information available before the outcome, feasible options, choice quality and timing costs |
| Defending | Tested threat, legal attributable intervention, support, permitted movement and replacement cover |

## Native use

```text
cargo test -p icelines-sources --lib tracking_feasibility --locked
cargo run -p icelines-sources --example rinkcraft_tracking_feasibility --locked -- INPUT_CSV NEW_REPORT_JSON
py -3.12 tools/verify_rinkcraft_tracking_feasibility.py INPUT_CSV REPORT_JSON NEW_RECEIPT_JSON
```

The CLI bounds one input to 64 MiB and uses create-new output semantics. The pure
library does not open URLs or files. The independent Python oracle performs a
separate grouped CSV census and checks every reported measurement field, source
hash and output hash. Synthetic tests cover clock/identity/visibility diagnostics,
missing and invalid coordinates, schema drift and noncontiguous frames. They
validate the audit, not the provider's calibration or Rinkcraft constructs.

Retain public dataset terms and publisher hashes with dated external research
outputs. Raw source files are not repository fixtures or automatically permitted
hosted assets. The anonymized release is a methods-development source until
competition, identity, population and transfer are independently established;
its measurements cannot be attached to named NHL profiles.

There is no automatic path from this audit to a 1–20 attribute or simulator bonus.
The complete-battery release gates and typed task intake remain unchanged.
