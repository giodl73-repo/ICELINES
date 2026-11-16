# Goalie-marker reference study —2026-11-15

Shared Rust `goalie_geometry::assess` adds a separately versioned observation
model, first consumed by the study-report adapter. Native goal analysis/API and
the existing CLI export are unchanged by this increment. Standalone WASM and
universal goalie-profile integration are not claimed.

Match the official opposing goalie ID to published frame markers; never choose
the nearest available goalie or substitute the scorer's own keeper. Reuse the
previous fixed goal/derived scale. Missing/unknown identity, role metadata,
orientation, scale or coordinates remain unavailable. Explicit empty-net goals
are inapplicable; an undefined puck-to-goal ray is separate. Missing observations
break displacement comparisons.

Measurements include signed marker depth from the goal line, perpendicular
marker offset from the puck–net-centre reference, projection fraction/between-
endpoints flag, puck-to-marker distance and separate marker displacement/offset
changes. The reference line changes with puck movement. A stationary goalie can
have a changing offset; zero offset behind the net is not interposition.

This does not establish facing, body/gear coverage, skate position, vision,
readiness, optimal positioning, save technique or response latency. The point
reference is not the actual shot target, and contact/release/goal alignment
remains unknown. Frame presence does not establish active ice membership. No
camera-codebook categories, capability scores or simulation probabilities change.

## Executed acceptance

- Seven focused `cargo test -p icelines-fetch --lib goalie_geometry --locked`
  tests pass: mirrored directions, aligned point, stationary-goalie/moving-puck,
  missing continuity, empty/unknown/undefined states, behind-net projection and
  unknown versus conflicting metadata.
- Locked study-report build passes. All96 captured cases regenerated from the
  unchanged original goal reports. No fresh NHL data fetched.
-93 recorded-goalie cases supply11,149 available frames. Three explicit empty-net
  cases remain inapplicable. Point-to-projected-point distance checks independently
  match Rust's cross-product results, depth, projection, distance and displacement.
- Prior contribution chains, fixed-goal geometry, selections and source report
  hashes remain unchanged; the earlier packet is retained privately.
- Browser packet acceptance passes player/missing-case filters, source/native
  links, credited-order anchors, added goalie values, mobile and zero page errors.
  Updated card screenshot inspected.

Exact source revision, commands/evaluator hashes, independent checker and bounded
case findings are retained in PUCK `scenarios/league/2023-24/goal-mechanisms/`.
This is descriptive study evidence from successful goals, not a goalie ranking
or a calibrated model of which positioning choices improve save probability.
