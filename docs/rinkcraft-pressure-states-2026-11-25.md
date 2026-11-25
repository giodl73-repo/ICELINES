# Late-close draw score-state breakdown — 2026-11-25

Under Pressure now expands the combined late-close draw result into tied,
leading-by-one and trailing-by-one cases. The challenge definition is unchanged:
regulation third period, 15:00 or later, pre-draw gap at most one.

Rust assigns the state using each actor's roster team and the reconciled score
before the faceoff. A leader and trailer have opposite states in the same draw;
both actors are tied when the score is level. State taken/won counts must sum to
the existing combined counts. Unknown keys, impossible wins and non-partitioning
counts fail validation. Empty observed states show “No observed opportunities”.

Raw partitions and rendered breakdowns are optional additions to the existing
schema. Old aggregate-only samples retain their combined result; their missing
splits remain absent. Both offline enrichment and routine refresh aggregate the
same source-level partitions. No new source fetch, attribute score, feat, task,
class assignment or simulation modifier is involved.

## Frozen examples

| Player | Combined | Tied | Leading by one | Trailing by one |
|---|---|---|---|---|
| Sebastian Aho | 12/20 | 5/6 | 4/11 | 3/3 |
| Nathan MacKinnon | 2/5 | 1/2 | 1/3 | No observed opportunities |
| Matty Beniers | 6/10 | 4/5 | 1/2 | 1/3 |

All are observed wins/opportunities, not expected future win probabilities.
Aho's three trials while trailing do not establish a 100% underlying ability.
The combined rate blends different contexts and opportunity volumes.

212 current cards have late-close opportunities and show 636 state rows; 197
rows have zero observed opportunities. Across the complete player sample, there
are 498 leading and 498 trailing trials, with 242 and 256 wins respectively.
Tied trials number 624 with 312 wins. These accounting symmetries confirm that
the 810 underlying late-close draws contribute both opposing actors correctly;
the league-wide 50% result is a consequence of one winner per draw.

## Reproduction and evidence

Input catalog SHA-256:
`e759cfcc93f5081fc9cebea0dacc91006e77841a9172fb173eb367bd24cc8624`.
Output catalog SHA-256:
`498a707658d2ccf7d31346bbe37887fe6d4122da13418967646ce9f3f5319b05`.
The 377-game cache and its original capture receipts are reused.

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-records --season 20262027 --catalog target/pressure-card-study/enriched-v2/rinkcraft-stat-attributes.json --output-dir target/pressure-states-study/enriched --game-cache-dir target/goalie-puck-study/cache
```

Choose a fresh directory for replay. Catalog, report and pressure receipts replay
byte-for-byte. All 3,948 retained score arrays are unchanged. A complete offline
routine refresh produced matching faceoff samples for all 787 skaters.
Primary measurement definitions and research limits remain in
[Under Pressure v1](../design/specs/rinkcraft-under-pressure-v1.md).

## Validation

- Native CLI/Askama, pinned WASM and browser builds passed.
- 248 browser tests, 19 focused core tests, eight source tests and three refresh
  tests passed. New cases cover actor-relative score, tied draws, partition
  validation, missing historical splits and empty observed states.
- Eight live cards passed rendered state labels/results, actual worker parity,
  retained goalie outcomes, defense gaps, legacy aggregate fallback, mobile
  overflow and print checks with the split open. No page errors.
- Eight one-page 612×792-point Letter PDFs independently retain the original
  primary counts and omit the expanded state details. Aho's split was inspected.

Generated records, helper, coverage, screenshots and PDFs remain ignored under
`target/pressure-states-study`. Browser acceptance:
`node icelines-browser/e2e/rinkcraft-pressure-states.mjs`. No hosted publication.

Preview: [Aho](http://127.0.0.1:8780/?player=8478427); expand **Late-close draws**.
