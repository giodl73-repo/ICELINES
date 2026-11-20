# Fixed attacking-goal reference —2026-11-15

The official NHL visualizer's rink SVG spans2400 by1020 units, with midpoint
1200/510 and goal-line bands at130–132 and2268–2270. Its replay component
translates the published player/puck x/y directly into that rink space. NHL
Rule1.2 specifies200 by85 feet; Rule1.5 places goals11 feet from the ends.
These establish a regulation-derived12-native-unit-per-foot reference. This is
not an independent survey of tracking accuracy. Source snapshots, URLs and
hashes are retained in the external study; Rinkcraft records source custody.

Shared Rust `goal_geometry::assess` determines the attacking goal from scoring
team and recorded home defending side. Unknown orientation stays unavailable.
Goal-line reference points are132/510 or2268/510, fixed independently of goalie
position. Per-frame output includes distance, signed depth/lateral offset,
bearing, distance/lateral change and sampled centreline-side change. Behind-net
positions remain explicit; bearing at the reference point is undefined.

Raw audits and units remain unchanged. The additive
`icelines.goal-relative-geometry.v0.1` describes its derived reference. Bearing
is not a release-aligned shot angle, physical reach, goal detection or xG.
Contact/goal alignment, height, vision, continuous personnel and coaching duties
remain separately unverified. Current coordinates are not rescaled or repaired.

Native HTML/API/CLI export and study review use the same Rust function. The
native viewer draws the fixed attacking-goal reference and a puck-distance line;
the latter is not a reconstructed shot path. Study cards show distance/bearing
at candidate interval endpoints, not asserted contact or release locations.

## Executed validation

- `cargo test -p icelines-fetch --lib goal_ --locked`:23 passed, including four
  geometry tests and a raw-audit-preservation integration test.
- `cargo test -p icelines-fetch --example goal_study_report --locked`:7 passed.
- Locked native CLI and study-report builds passed; Askama templates compiled.
- Five stored goals exported with `--no-live` to the ignored
  `target/goal-reference-2026-11-15/` acceptance directory.
- Native/JSON/portable browser acceptance passed all five goals, fixed-target
  markers and geometry equality, playback, disclosures, mobile and zero errors.
-100-case study packet acceptance passed contributor/anchor/goal-relative fields,
  official/native links, filters, mobile and zero errors. Screenshots inspected.
- Independent arithmetic checked all11,508 frames across96 goals:51 left-target,
  45 right-target. Prior contributor/context values and original goal-report
  hashes unchanged. Endpoint orientation diagnostic agrees for96/96; this does
  not establish the goal/release frame or tracking accuracy.

The worked sequence can move farther from the fixed goal before its forward
delivery. That is an observed geometry pattern, not an estimate of the value of
backward passes. One same-game pair also swaps a cohort player between scorer
and primary-assister roles; another case credits a goalie as secondary assister.
These motivate broader action/context profiles, not player ability grades.

No simulator probability, player class, main merge, push, release or hosted build
changed. Native preview remains loopback8786; research packet loopback8787.
