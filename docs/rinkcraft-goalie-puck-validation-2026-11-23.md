# Goalie puck-event ledger validation — 2026-11-23

Implementation: Rust source parser, cached acquisition and `profile-puck-play`
CLI. No new rating formula or browser presentation is part of this change.

## Local checks

- Four source-parser tests pass: actor attribution and secondary assist,
  ten-second/stoppage/period boundaries, wrong-scope/duplicate/unknown actors,
  roster-only goalies and shootout exclusion.
- Cached acquisition test passes: schedule deduplication, regular/finalized
  selection, zero credited events, backup-safe roster counts, preserved existing
  outputs and failure on a missing offline game capture.
- Native CLI compilation passes (`cargo build -p icelines-cli --locked`).
- Final-source global `--no-live` replay passes twice over all 377 games;
  the reports are byte-identical.

## League evidence

Command from the owned Rinkcraft worktree:

```powershell
target/native-validation/debug/icelines.exe --no-setup profile-puck-play --season 20262027 --cache-dir target/goalie-puck-study/cache --output-dir target/goalie-puck-study/league-initial
```

The source capture spans 2026-11-24 00:35:59–00:36:35 UTC (November 23 locally).
The run captured all 377 finalized regular-season games discovered from 32 club
schedules. The 409 captures include those schedules and game logs.

| Credited event | Count |
| --- | ---: |
| Giveaways | 364 |
| Primary assists | 2 |
| Secondary assists | 14 |
| Goals | 1 |
| Committed penalties | 25 |
| Takeaways | 0 |
| Total | 406 |

There are 86 roster-listed goalies; 72 have a credited giveaway and 13 have an
assist. All 78 goalies in the existing current-season package have exactly matching
assist totals. The eight additional roster identities have no appearance in that
package; their roster membership is not evidence of GP or playing time.

The season package used for that cross-check is
`target/goalie-refresh/all-current-final/season-package.json`, SHA-256
`3536ed9cf5a43e669109444e391db354e8059ee9eb577630dde4696957b08f0c`.

All 377 raw play logs also pass an independent audit for nonempty plays and unique
event order. Generated reports and raw public captures remain ignored under
`target/goalie-puck-study/`; each report preserves the capture hashes and URLs.

Final report: `target/goalie-puck-study/league-final/goalie-puck-events.json`.
Independent repeat: `target/goalie-puck-study/league-replay/goalie-puck-events.json`.
Both SHA-256 hashes:
`fa95ea65da0676bbad186c6dc93af7b0bab8e7753a6261f84b8452c53a9b4123`.
In 87 of the 364 giveaway records, the temporal scan finds an opponent shot on
goal or goal within ten seconds before the specified stopping boundaries. This
is a play-review index, not a giveaway-to-shot causal probability.

## Interpretation

This adds substantial recorded activity beyond assists, but cannot establish
successful puck handling. No ordinary pass/touch denominator is present in this
ledger. Counts and following-shot context are available for profile evidence and
play review; they are not automatically capability ratings or error probabilities.

See the [source contract and command guide](../design/specs/rinkcraft-goalie-puck-events.md).

## Profile integration checkpoint

The native and browser cards now consume the same Rust `goalie_puck_events`
projection. The input game/event transport structs are shared with the source
parser. Counts are rebuilt from those events, not copied from summary counters.
Each record states its separate capture date and selected-game sample; none of
these counts is used as a capability score, projected total or exposure rate.

Completed local validation:

- Three focused core tests: event-derived counts despite contradictory summary
  counters, explicit zero for roster-listed backups, scope selection, stale-field
  clearing, incomplete coverage, missing captures, duplicate games and actor/team
  mismatches.
- `cargo check -p icelines-web -p icelines-cli --locked` passes, including the
  native Askama include and configured-source loading.
- Rust 1.95.0 WASM build and matching web bindings pass.
- Browser build and all 243 browser unit tests pass, including wrong-player,
  wrong-season, playoff and skater presentation checks.
- All 78 current goalie profiles were projected by the actual WASM export;
  468 counts match an independent count over captured game events. All 78 also
  return no evidence for playoff or skater variants.
- Three contrasting real profiles pass the committed browser acceptance script:
  a goalie goal, the largest event record and a zero-event record. Actual worker
  responses equal the saved Rust projection; game/source links and penalty
  descriptions render; details start closed; no page errors or 390px overflow.
- All three exported PDFs independently contain one 612×792-point Letter page
  with Attributes and Puck Record. Print testing exposed oversized attribute
  buttons inherited from screen styles; scoped print rules now remove that
  interactive sizing while preserving the numeric fields.
- Desktop/mobile screenshots were inspected. The new disclosure and links use
  card accent colors to stay readable within the paper card on a dark app theme.

Ignored artifacts: `target/goalie-puck-profile/` contains matching WASM bindings,
`projection-acceptance.json`, `browser-acceptance.json`, desktop/mobile images
and three Letter PDFs. The running 8777 preview serves generated profile
snapshots from `target/goalie-refresh/site/`, not a hosted deployment. It also
serves the actual worker and matching WASM used by the acceptance test.

Replay the prepared preview check with:

```powershell
node icelines-browser/e2e/rinkcraft-goalie-puck.mjs
```

Set `ICELINES_GOALIE_PUCK_URL` for a different prepared preview URL. The script
requires its player list, projected profile fixtures and source ledger; it does
not start a server. No publication or full native application runtime validation
is claimed at this checkpoint.
