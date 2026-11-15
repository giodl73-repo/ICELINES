# Observed goal-sequence audit validation — 2026-11-15

A Rust example consumes the existing typed `icelines_fetch::goal_visualizer`
bundle and reuses its frame parser. No new network importer or gameplay logic.
It joins game-specific official roster/event metadata, preserves every native
frame and unknown units, orders events by period/clock, and exposes observation
counts/appearances, scorer presence and explicitly non-possession geometry.

Locked build passes:

```powershell
cargo build -p icelines-fetch --example goal_sequence_audit --locked
goal_sequence_audit BUNDLE LANDING BOXSCORE PLAY_BY_PLAY OUTPUT
```

Live bounded source inspection: existing icelines 0.26.0 fetcher discovers/fetches
all four available goal clips in documentation game 2025021167. Its binary hash is
recorded but its exact build commit is unknown. Inspected adapter bytes match both
checkouts; do not treat that as proof of the executable's build revision.

Four clips/559 frames validate through the existing Rust parser. A separate Python
audit recomputes identities, timestamps/deltas, native geometry, scorer appearances,
observation counts and official modifiers against source files. Native physical/
time units, exact scoring frame, active-ice membership and assignments stay unknown.
No distances become capability scores or authored grid positions.

Private research artifacts: PUCK `scenarios/nyr/2025-26/goal-sequence-pilot/`.
Full-frame inspector passes four-goal selection, last-frame reachability, playback,
plotted observations and zero page errors in headless system Edge. Plot inspected
locally. No hosted/public publication or full-app/WASM acceptance is claimed.
Generated payloads/plots remain on F: through ignored output junction; source/binary
hashes, deterministic replay and commands are retained by the dated run manifest.

This is descriptive source-access feasibility, not validated tactical causality,
real-player ratings, goalie fault or success/failure rates. Rinkcraft decision 0064
and its producer-applied review preserve evidence, sampling and ownership limits.
