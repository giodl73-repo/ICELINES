# IceLines Analyze Goal validation — 2026-11-15

First native product feature, not a PUCK-only pilot. Shared Rust audit/service in
`icelines-fetch`; native page/JSON twin and shared report surface in `icelines-web`;
`icelines analyze-goal` CLI and game/global navigation integration. The existing
example delegates to the shared audit without changing its output schema.

## Completed checks

- Locked CLI integration check and production binary build pass; Askama templates
  compile. Seven service tests, two native-route tests and one script-payload
  escaping test pass (10 focused checks).
- Service tests cover URL/ID resolution, strict official hosts, source/game/scorer/
  frame-count agreement, missing observations, keeper unknown versus recorded empty
  net, offline/no-live behavior, cached import, per-file hashes and concurrent capture.
- Four pilot goals import offline through the new CLI. URL and separate game/event
  flags work. One different historical goal, 2024020151/172, fetches live and yields
  140 frames; source/cache mode is recorded and no data is hardcoded to the pilot.
- The earlier complete four-goal/559-frame audit is byte-identical after extracting
  it into the reusable library. Per-goal results match its preserved events.
- Native page, JSON response and portable report agree for five goals across two
  games. Browser acceptance covers game picker, pasted URL form, all-frame access,
  playback, nearest-observation controls, empty-net context, missing-scorer state,
  desktop/mobile geometry and zero page errors. Screenshots inspected locally.
- Visual review identified playback-label contrast; the final scoped CSS fixes it.
  Existing UTF-8 text was restored after an editing issue; the final diff preserves
  existing text and limits changes to the feature and navigation.

## Reproduction

```powershell
cargo check -p icelines-cli --locked
cargo test -p icelines-fetch --lib goal_analysis --locked
cargo test -p icelines-web --test goal_analysis_routes --locked
cargo test -p icelines-web --lib goal_analysis --locked
cargo build -p icelines-cli --bin icelines --locked
icelines --no-setup analyze-goal https://www.nhl.com/ppt-replay/goal/2025021167/996 --output-dir ./goal-report
icelines --no-setup --no-live analyze-goal --game-id 2025021167 --event-id 996 --output-dir ./goal-report
```

Offline source capture uses the existing pilot's typed bundle plus official
landing/boxscore/play-by-play JSON. No raw NHL payload is committed. Native preview
is the actual owned-branch IceLines application on loopback8786 with `--no-open`;
unrelated older servers remain untouched. E2E:
`node icelines-browser/e2e/goal-analysis.mjs`, with optional
`ICELINES_GOAL_ANALYSIS_URL` and `ICELINES_GOAL_ANALYSIS_ARTIFACTS` overrides for the
running app and prepared CLI reports. Browser tests use headless system Edge.

## Boundaries

Native units/time scale, exact scoring frame, continuous active personnel,
possession/contact, sticks/height/vision and called duties remain unverified.
Native Euclidean distance is not physical stick reach. Nearest-observation runs
are not passes or possession. No automatic culprit, player class/grade, success
probability, complete-system diagnosis or full-game validation is claimed.

Stored captures are reused; refresh/version-history and video/shift alignment are
future work. JSON is a consumer boundary, not standalone WASM acceptance. No hosted
publication, main merge, release or automatic GitHub workflow was performed.
See `design/specs/goal-analysis.md`, COMMANDS and Rinkcraft decision0065/review.

## Final local acceptance

The final UI pass also opens source details at390px without horizontal overflow,
checks readable playback-label color, and clicks nearest-observation runs to reach
their actual first frames. Final screenshots inspected. Four negative CLI cases
(zero event, external host, missing paired event and uncached no-live target) reject
without generating a fabricated report. The native server remains on loopback8786.
