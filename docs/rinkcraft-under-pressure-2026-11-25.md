# Under Pressure pilot validation — 2026-11-25

The first five-definition catalog is implemented in shared Rust and displayed in
native and browser player cards. Definitions, limits and primary references:
[Under Pressure v1](../design/specs/rinkcraft-under-pressure-v1.md).

## Frozen data and result

The existing verified cache contains 377 finalized 2026–27 NHL games and 21,664
faceoffs. Events are not newly fetched for this pilot. Faceoff receipts run
through `2026-11-24T00:36:35.224942700+00:00`; goalie samples retain their existing
MoneyPuck.com shot-archive capture. Samples are not represented as full-season or
player-GP coverage.

Current selected-season population: 865 cards. Measured entries: 449 forwards and
78 goalies. The remaining 65 forwards and 273 defensemen keep explicit sample or
tracking gaps. No grade, feat, class or simulation input changes.

| Player | Sample | Result |
|---|---|---|
| Sebastian Aho | All credited draws | 233/415 won, 56.1% |
| Sebastian Aho | Late-close draws | 12/20 won, 60.0% |
| Matty Beniers | All credited draws | 145/298 won, 48.7% |
| Matty Beniers | Late-close draws | 6/10 won, 60.0% |
| Nathan MacKinnon | Late-close draws | 2/5 won, 40.0% |
| Alex Killorn | All credited draws | 1/6 won, 16.7% |
| Igor Shesterkin | Rebound shots on goal | 11/13 saved, 84.6% |
| Igor Shesterkin | Rapid follow-ups on goal | 7/7 saved, 100.0% |
| Adam Fox | Entry denial / retrievals | No complete tracked opportunity sample |

These are count-first observations, including small samples. They are not new
skill rankings. Aho's late sample and Shesterkin's follow-ups describe different
challenges and different source windows; neither supplies a universal pressure score.

## Reproduction

From the owned IceLines worktree, choose a new output directory:

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-records --season 20262027 --catalog target/shooting-card-study/enriched/rinkcraft-stat-attributes.json --output-dir target/pressure-card-study/enriched-v2 --game-cache-dir target/goalie-puck-study/cache
```

The same command into `replay-v2` produced byte-identical catalogs, reports and
`pressure-sources.json`. Source input catalog SHA-256:
`9fc4ee1625e407c88592bffe80c11bc0b6cc566105d282d4ae080cb86a574e77`.
Output catalog SHA-256:
`e759cfcc93f5081fc9cebea0dacc91006e77841a9172fb173eb367bd24cc8624`.
All 3,948 retained profile score arrays are unchanged.

The regular `profile-refresh --game-cache-dir` path now parses assists and
contextual faceoffs from the same captured game bytes. A full offline skater
refresh completed; every one of its 787 faceoff samples matched the standalone
enrichment, including source fingerprint and coverage. Its scalar-source refresh
is separate from the unchanged-grade enrichment comparison above. Missing a game
cache produces named faceoff gaps. Goalies derive responses from their admitted
shot contexts without a game cache.

Generated catalogs, coverage, receipts, projection helper, screenshots and PDFs
remain ignored under `target/pressure-card-study`. The preview roster now contains
skaters and goalies in the same selector. Source code does not embed player results.

## Checks

- Native CLI/Askama, pinned WASM and browser builds passed.
- 248 browser tests passed, including stale actor/season/group/GP rejection and
  missing tracking without invented ratings.
- 17 core tests and seven source tests passed: opportunity arithmetic, window
  binding, nested faceoff counts, goalie misses/empty samples, pre-draw score,
  overtime/shootout handling, duplicate/wrong actors and missing goal sequences,
  plus existing snapshot and skater-input contracts. Raw faceoff season mismatch
  is rejected; a goalie archive from another year is withheld.
- Three refresh integration tests passed, including deterministic replay,
  cache verification, missing-source handling and safe archive admission.
- Eight live cards passed displayed names/results, context disclosures, actual
  worker parity, goalie recovery from older catalogs, defense gaps, legacy
  fallback, mobile overflow and no page errors.
- Eight single-page 612×792-point Letter PDFs independently verified with pypdf,
  retaining each measured response. Skater and goalie disclosures were inspected.
- A copied cache body with altered bytes failed SHA verification before any output
  directory was created. Original source cache was untouched.

Live acceptance: `node icelines-browser/e2e/rinkcraft-under-pressure.mjs`. It uses
the existing loopback server and does not start one. No hosted build/publication.

Examples: [Aho](http://127.0.0.1:8780/?player=8478427),
[Beniers](http://127.0.0.1:8780/?player=8482665),
[Fox](http://127.0.0.1:8780/?player=8479323),
[Shesterkin](http://127.0.0.1:8780/?player=8478048).
