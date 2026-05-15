# IceLines Wave Index

| Wave | Status | Mission | Active Evidence |
|---|---|---|---|
| [Sim the Spark](2026-05-15-sim-the-spark/WAVE.md) | active | Add descriptive Rocket Richard scoring pace and outlook contracts without betting or proprietary projection claims. | `SPARK-INVENTORY.md`; `PlayerScoringPaceView`; `plans/pulse-01.md`; `plans/pulse-02.md`; `plans/pulse-03.md` |
| [Measure the Finish](2026-05-14-measure-the-finish/WAVE.md) | closed | Deepen Rocket Richard into player scoring trends, streak leaderboards, and owned shot-quality proxy contracts. | `FINISH-INVENTORY.md`; `plans/pulse-01.md`; `plans/pulse-02.md`; `plans/pulse-03.md`; `plans/pulse-04.md`; `plans/pulse-05.md` |
| [Aim the Rocket](2026-05-14-aim-the-rocket/WAVE.md) | closed | Open Phase Rocket Richard by proving official scoring-event data contracts before building shot maps, scoring reports, tonight intel, or projections. | `SCORING-DATA-INVENTORY.md`; `plans/pulse-01.md`; `plans/pulse-02.md`; `plans/pulse-03.md`; `plans/pulse-04.md`; `plans/pulse-05.md`; `plans/pulse-06.md` |
| [Audit the Stack](2026-05-14-audit-the-stack/WAVE.md) | closed | Run a whole-codebase bug-detection and architecture review focused on cross-surface mismatches, stale state, and missing regression coverage. | `panels/whole-codebase-bug-pass/`; `plans/pulse-01.md`; `plans/pulse-02.md`; `plans/pulse-03.md`; `plans/pulse-04.md`; `plans/pulse-05.md` |
| [Profile the Player](2026-05-14-profile-the-player/WAVE.md) | closed | Define the complete player screen system, including records, streaks, awards, career arc, comparisons, and fantasy context. | `PLAYER-SCREEN-MAP.md`; `plans/pulse-01.md`; `plans/pulse-02.md`; `plans/pulse-03.md`; `plans/pulse-04.md`; `plans/pulse-05.md` |
| [Trace the Events](2026-05-13-trace-the-events/WAVE.md) | closed | Validate and ingest NHL play-by-play participants so richer individual records can count goalies beaten and fight opponents without aggregate inference. | `EVENT-DATA-INVENTORY.md`; `icelines fetch play-by-play`; event-backed `icelines records` metrics; metric-aware records web/API routes |
| [Align the Reports](2026-05-13-align-the-reports/WAVE.md) | closed | Make CLI query/report/export surfaces discoverable and prepare symmetric player/team records as a report and screen family. | `REPORT-SURFACE-INVENTORY.md`; `RECORDS-DATA-INVENTORY.md`; `icelines report list`; `icelines records`; `/records/player/:id`; `/records/team/:abbrev` |
| [Test the Command Bar](2026-05-13-test-the-command-bar/WAVE.md) | closed | Validate whether real users can navigate IceLines through the MDI command bar, tabbing, and subcommand handoffs without maintainer coaching. | `USER-TESTING-PROTOCOL.md`; `SESSION-FINDINGS.md`; sticky command-mode follow-up; command-bar/tui persona tests |
| [Backcheck the Phases](2026-05-13-backcheck-the-phases/WAVE.md) | closed | Backfill previous trophy phases into executable pulse packets so agents can clear residual gaps without relying on chat memory. | `BACKFILL-INVENTORY.md`; `VISUAL-CAPTURE-INVENTORY.md`; pulse plans/forks |
| [Hart Normalizes the Core](2026-04-30-hart-normalizes-the-core/WAVE.md) | backfilled | Retrospective wave record for normalization and View-based migration. | `plans/pulse-01.md` |
| [Art Ross Rewrites the Query](2026-05-06-art-ross-rewrites-the-query/WAVE.md) | backfilled | Retrospective wave record for query grammar, planner, executor, and parity hardening. | `plans/pulse-01.md` |
| [Foster Broadcasts the Night](2026-05-06-foster-broadcasts-the-night/WAVE.md) | backfilled | Retrospective wave record for favorites, date axes, sync, and live slate. | `plans/pulse-01.md` |
| [Norris Extracts the State](2026-05-07-norris-extracts-the-state/WAVE.md) | backfilled | Retrospective wave record for TUI state extraction. | `plans/pulse-01.md` |
| [Masterton Factors the Screens](2026-05-08-masterton-factors-the-screens/WAVE.md) | backfilled | Retrospective wave record for Screen trait/chrome/standalone mode. | `plans/pulse-01.md` |
| [Jack Adams Coaches the Bench](2026-05-08-jack-adams-coaches-the-bench/WAVE.md) | backfilled | Retrospective wave record for default TUI MDI dashboard and command bar. | `plans/pulse-01.md` |
| [Campbell Contracts the Platform](2026-05-09-campbell-contracts-the-platform/WAVE.md) | backfilled | Retrospective wave record for platform contracts and ViewModels. | `plans/pulse-01.md` |
| [Selke Poaches the Edge](2026-05-09-selke-poaches-the-edge/WAVE.md) | backfilled | Retrospective wave record for fantasy poacher, watch rules, and reports. | `plans/pulse-01.md` |
| [Messier Leads the Filters](2026-05-09-messier-leads-the-filters/WAVE.md) | backfilled | Retrospective wave record for TUI filter/sort consistency. | `plans/pulse-01.md` |
| [Lester Patrick Serves the CLI](2026-05-09-lester-patrick-serves-the-cli/WAVE.md) | backfilled | Retrospective wave record for CLI parity. | `plans/pulse-01.md` |
| [Ted Lindsay Chooses the Web](2026-05-09-ted-lindsay-chooses-the-web/WAVE.md) | backfilled | Retrospective wave record for web parity and route inventory. | `plans/pulse-01.md` |
| [Prince of Wales Polishes the Surfaces](2026-05-09-prince-of-wales-polishes-the-surfaces/WAVE.md) | backfilled | Retrospective wave record for shared visual system work. | `plans/pulse-01.md` |
| [Jim Gregory Hardens the Release](2026-05-09-jim-gregory-hardens-the-release/WAVE.md) | backfilled | Retrospective wave record for release gates and rollover discipline. | `plans/pulse-01.md` |
| [Presidents Measures the Season](2026-05-12-presidents-measures-the-season/WAVE.md) | backfilled | Retrospective wave record for team season performance. | `plans/pulse-01.md` |
| [Jack Adams Web Opens the Browser Bench](2026-05-12-jack-adams-web-opens-the-browser-bench/WAVE.md) | backfilled | Retrospective wave record for browser dashboard and command surface. | `plans/pulse-01.md` |

## Status Values

- `active` - current wave for pulse generation and fork dispatch.
- `planned` - defined but not yet executing.
- `blocked` - waiting on external input or architectural decision.
- `closed` - completed; see the wave closeout record.
