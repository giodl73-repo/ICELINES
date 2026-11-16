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


## Credited touch order as a tracking constraint

Consumer schema `icelines.goal-contributors.v0.2` makes official credited puck-
touch order explicit: secondary assist, primary assist, scorer (with absent
assist slots omitted). This is stronger information than an unordered list of
contributors. It does not identify action type or enumerate every intervening
or repeated touch. Previous generated packets retain their original schema.

The exploratory ordered-anchor search groups consecutive frames with the same
nearest observed body point, then finds all chronological subsequences matching
the credited actors. Dynamic programming counts alternatives; up to16 examples
are retained, and four displayed in temporal enumeration order. No likelihood,
physical reach threshold or contact-time estimate is assigned. Intervening
nearest-body runs are preserved in JSON and do not assert an opposing touch or
possession change. A missing match retains the official touch order and unknown
tracking alignment. Native frame/time/goal alignment limitations still apply.

The method was designed after inspecting four cohort cases; the fixed100-case
sample was not changed. This is hypothesis generation, not a preregistered
validation of contact detection. It changes neither action definitions nor
player attributes or simulation probabilities. Seven focused report tests cover
ordering, multiple matches, missing anchors, bounded examples and the prior
identity/source/missingness contracts. The linked study records empirical
coverage and independent enumeration checks separately.


## Goalie-marker geometry: study consumer

Shared Rust `goalie_geometry::assess` supplies schema
`icelines.goalie-reference-geometry.v0.1`. The study packet is its first consumer;
this increment does not add the field to the native goal API or standalone WASM.
The native viewer and its prior fixed-goal model remain available for inspection.

Use only the recorded opposing `goalieInNetId`, matched by frame player ID and
roster goalie identity. Explicit empty-net goals are inapplicable. Missing keeper,
role metadata, reference, scale or coordinates stay unknown; conflicting identity
is separate. A missing frame breaks the displacement comparison rather than
imputing a stationary or perfectly aligned goalie.

For target G, puck P and observed goalie marker K, projection fraction is
`dot(K-G,P-G) / |P-G|²`. Perpendicular marker offset from the reference line is
`abs(cross(P-G,K-G)) / |P-G|`, converted using the existing derived rink scale.
Report signed goal-line marker depth, projection-between-endpoints, marker/puck
distance and separate frame-to-frame marker displacement and offset change.
Undefined ray at P=G remains explicit. Zero line offset behind the net does not
mean that the marker is between puck and goal.

This is a puck-to-net-centre reference, not the actual shot target, goalie facing,
body/gear coverage, skate location, gaze, readiness or optimal-position grade.
Its line changes with the puck: offset can change with zero goalie movement.
The existing camera-based goalie codebook is not silently relabeled by these
tracking-point metrics. Frame presence does not verify continuous active occupancy.
Candidate interval endpoints in the packet show these derived values; final
release/contact/goal alignment remains a separate task.

## Candidate event windows

Shared Rust `goal_event_windows::assess` returns
`icelines.goal-event-windows.v0.1`. All forward sample brackets intersecting the
fixed reference plane within the regulation-derived opening are candidates.
Interpolation is linear in2D; height, full-puck passage and true path are not
observed. Exact goal verification is always false. Multiple candidates and
missing matches remain explicit, with closest-plane/outside-opening diagnostics.

The study adapter recomputes credited-order anchors from the complete prefix
ending before each bracket and attaches earlier goalie-marker snapshots. No
best-event ranking, verified release, contact or reaction-time window is inferred.
The native API is unchanged. See the dated window validation record for actual
coverage and exceptions; source clips/criteria were not altered to fit outcomes.
