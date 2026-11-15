# Exploratory successful-goal study tools

Reusable Rust adapters build a frozen sample and descriptive review packet using
the existing shared goal-analysis service. Authored cohort/window/annotation
inputs and dated findings belong in an external research workspace. Raw NHL
captures and generated reports remain outside Git.

```powershell
cargo run -p icelines-fetch --example goal_mechanism_study --locked -- PROTOCOL_JSON OUTPUT_DIRECTORY
cargo run -p icelines-fetch --example goal_mechanism_study --locked -- PROTOCOL_JSON OUTPUT_DIRECTORY acquire
cargo run -p icelines-fetch --example goal_study_report --locked -- RECORDS_JSON OUTPUT_DIRECTORY ANNOTATIONS_JSON
```

The first command discovers official identities and scoring-game logs for one
regular season. It selects evenly spaced scoring-game ranks, including endpoints,
and writes `plan.json` before tracking acquisition. `acquire` selects the first
chronological player goal with a published replay in each chosen game. Shootout
conversions are excluded from player goal counts. Goal-log totals and landing
totals must agree. Selected games are not replaced after tracking is inspected.
Both planning and acquisition are explicit online operations; stored sources
and the existing goal service cache are reused. The example does not inherit
the separate CLI's global `--no-live` flag.

`records.json` preserves failures alongside captured cases, including identified
goal metadata when tracking fails after discovery. Keep prior run records before
rerunning. A changed frozen plan is rejected; use a separate output directory
for a revised protocol. No automatic substitution, coordinate repair or partial
frame omission is performed. The existing strict replay parser is retained.

The report produces `summary.json`, `summary.md`, `review_records.json` and a
filterable `index.html`. Optional authored annotations are keyed by captured
game/event pair; duplicates or out-of-study targets are rejected. Their
documentary status remains separate from machine observations. Browser text is
inserted with textContent; script data escapes HTML-sensitive characters.
Case links open the native IceLines sequence and official NHL sources. The
review packet is generated research output, not a new native route.

Counts describe selected captured goals. They are not opportunity-normalized
rates, frequencies among all goals, player grades or causal estimates. A
structured shot type does not necessarily identify the final contact or the
creation sequence. Video alignment, actual active personnel, stick/height/vision,
supporting actions and called duties remain separate evidence tasks. All six
positions stay in scope; goals conceded alone cannot assess goalie ability.

Checks:

```powershell
cargo test -p icelines-fetch --example goal_mechanism_study --example goal_study_report --locked
$env:ICELINES_GOAL_STUDY_ARTIFACTS='ABSOLUTE_GENERATED_DIRECTORY'
# Serve only that directory on loopback8787, or set ICELINES_GOAL_STUDY_URL.
node icelines-browser/e2e/goal-study-report.mjs
```

Initial100-case execution, review, exact source revisions and reproducibility
manifest are recorded in PUCK `scenarios/league/2023-24/goal-mechanisms/` and
Rinkcraft decision0066. No NHL source payloads or personal cohort fixtures are
committed to IceLines. No full tactical annotation or independent review is
implied by successful acquisition/report tests.


## Credited participants and separately supported actions

Each captured goal now gets `icelines.goal-contributors.v0.1`. The report verifies
its source report hash and game/event/context before using official PBP fields
`assist1PlayerId`, `assist2PlayerId` and `scoringPlayerId`. These identify primary,
secondary and scorer credits independently of the landing assist-array order.
Conflicting/duplicate credit identities are rejected.

The display orders secondary, primary and scorer as official credits. This is
not a verified pass sequence or an importance ranking. Primary assists can
include shot deliveries followed by redirects; the action needs its own source.
Unassisted goals can still have supporting contributions outside scoring credit.

Optional `contributor_actions` annotation rows join by player ID and require
linked source IDs. Duplicate/uncredited actors or unlinked sources are rejected.
Missing action evidence stays unassessed, including secondary credits. A
source-supported action remains an attributed documentary description, not a
moving-video observation or a player class. Frame-presence and puck-nearest
indices are retained as inspection anchors; they do not establish control or
infer a pass. The original source record and machine audit are unchanged.
