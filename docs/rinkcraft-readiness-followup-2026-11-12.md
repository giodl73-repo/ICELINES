# Playable readiness follow-up

The separately versioned follow-up 0.1 starts from an exactly verified readiness
report, preserving that report and its unit 0.4 source. It records its own seed
and every action. Old unit 0.1–0.4 and readiness 0.1 verification stay unchanged.

Controlled receivers choose Shoot or Hold. Shooting contests assigned gap
Defending before an on-target flight. Ready goalies choose Tracking-based save
or Movement-based scramble; unready goalies choose scramble or a Recovery race
against receiver Skating. Recovery success resets readiness without controlling
or stopping the puck, opening tracking or smothering. Failure/tie ends unresolved.
Save contact stays uncontrolled. Containment failure stays loose and unreset.
Only an explicit possession contest can award the loose puck: Smother uses
Rebounds vs Handling; Gather uses Handling vs goalie Recovery in either state.
Receiver control opens a fresh Shoot/Hold opportunity. Hold consumes no dice.

Readiness adds options without weakening retained responses. Ready goalies keep
Scramble, and Gather never changes its opposing capability with readiness. No
arithmetic bonus or sum of goalie attributes is introduced. All pairings and
legal choices come from Rust. The UI looks up source scores for presentation.
Goals require failed chosen save contests, never merely a failed readiness check.

Bound branches at eight actions and preserve exactly one held/flight/loose/
scored puck disposition. Only actual contests consume two d20 in this branch's
separate stream. A failed or tied recovery is unresolved, not a defensive win;
unresolved source readiness offers no responses. The user/GM chooses both sides.
There is no calibrated policy, timing, geometry, fatigue, match scoring or full
game. Goal fractions would be censored and are not NHL probabilities or balance.

New Rust module/CLI `rinkcraft_readiness_followup` owns state, pairings, exact
counts, bounded import/export and deterministic continuation. Input/replay/
session have v1 schemas with frozen rule metadata. Studio start/step/import
commands carry only their self-contained report/replay, keeping commands below
the existing 768 KiB bound; follow-up files are bounded at 640 KiB. Explicit
import loads the embedded parent snapshot and report. Optional response fields
are omitted from unrelated older responses. Changing the parent/report clears
the displayed branch; identical refreshes and pending setup retain its recorded
export. Invalid operations preserve valid state.

```powershell
cargo run -p icelines-core --example rinkcraft_readiness_followup --locked -- INPUT.json
cargo run -p icelines-core --example rinkcraft_readiness_followup --locked -- --verify REPLAY.json
cargo run -p icelines-core --example rinkcraft_readiness_followup --locked -- --step REPLAY.json recover_set
```

101 focused Rust tests and 26 Node tests pass. New checks cover frozen readiness
equality, changed legal choices, ready preserving scramble/gather arithmetic,
recovery opening tracking without bonus/control, contact versus possession,
unknown/tied outcomes, puck conservation, eight-action cap, exact continuation,
source/result forgery, bounds and command-level parent preservation. The 96
property paths use a compact neutral game fixture and direct prefixes, with
final serialization checks. Full authored fixtures retain separate boundaries.

The frozen readiness fixture was captured before edits from prior `0b794d6`,
WASM `28041d81ba6aea984286dc85a98fa51e06ef49155e78b68fbbffc7e60f1a5271`.
The selected packet is
`C:/src/Rinkcraft/game/playtests/2026-11-12-readiness-followup/`; full inputs,
replays and UI artifacts stay ignored under `target/readiness-followup`. UI
acceptance is `icelines-browser/e2e/rinkcraft-readiness-followup.mjs` with prepared
CLI cases and native 8772/browser 8773. It does not start those servers.
No hosted build, push, merge, human/independent playtest or empirical claim.

Actual native and offline-browser play matches all eight CLI paths, preserves
parent/report/primary card, verifies imports and exact continuation, rejects a
forged replay without mutation, clears stale branches and exports the recorded
branch while main-unit setup edits are pending. Keyboard checks retain focus on
branch choices or its download control. Independent pypdf confirms one-page
612 by 792-point Letter character exports; 390-pixel overflow checks pass.

Regression validation exposed an early-click hazard in existing game navigation:
buttons could accept a click before asynchronous build had created the panels.
They now stay disabled through initial build/rebuild/import and enable only with
a current character. `rinkcraft-game-navigation.mjs` holds the real studio at an
explicit promise gate and verifies both surfaces. A first parallel legacy-panel
wait and a later injected-config wait failed; an isolated pass alone did not
establish the cause. The controlled test and final reruns are retained, rather
than silently raising timeouts or claiming all first attempts passed.
