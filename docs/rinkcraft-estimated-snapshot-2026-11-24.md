# Complete estimated record from the selected snapshot — 2026-11-24

Some ordinary player cards showed contact and blocking as unknown in Estimated
Record while the newer admitted NHL snapshot contained both rates. The front
panel was falling back to older descriptive attributes. The shared Rust
attachment now projects the admitted snapshot and native/browser cards prefer it.

## Calculation and scope

`rinkcraft_snapshot_record::estimate` delegates to the existing season-pace
calculator. Five count fields use NHL all-strength rates: shots, assists, goals,
hits and blocks. Per-game rate = per-60 rate × observed minutes / (60 × GP).
Season pace = per-game rate × target games. TOI remains minutes per appearance;
shot conversion remains goals/60 divided by shots/60. Neither is multiplied by
the season length. MoneyPuck archive goals, attempts and xG are not inputs.

The existing season-length policy defaults to 84 games from 2026–27 and 82 for
earlier seasons, with the supplied maximum override supported and a floor at
observed GP. This is straight pace, not an injury, availability, deployment or
team-schedule forecast. Rounded front totals are presentation only; raw values
and per-game rates remain in the record. Native projected totals now explicitly
round halves upward like the browser, avoiding different display of 10.5 goals.

The optional record lives inside its actor/season/group/GP-bound stat profile;
catalog validation checks its season, target, shape and finite nonnegative
values. Shared attachment computes it even for older catalogs lacking the field.
Inactive windows and goalies get no new snapshot projection. Known zero remains
zero; missing hits/blocks remain unavailable; zero shots have no conversion.

No six-attribute grade, feat qualification, task selection or simulation input
changes. Older numerical-card projections remain the fallback when no admitted
snapshot projection is available.

## Local result

Existing frozen 2026–27 NHL snapshot, fetched 2026-11-23. Input catalog:
`target/shooting-card-study/enriched/rinkcraft-stat-attributes.json`, SHA-256
`9fc4ee1625e407c88592bffe80c11bc0b6cc566105d282d4ae080cb86a574e77`.
No new source fetch. All 787 skaters have five count projections, including hits
and blocks. Projection preserves each achievement object and all retained
integer attribute grades.

| Player | Shots | Assists | Goals | Hits | Blocks |
|---|---:|---:|---:|---:|---:|
| Alex Killorn | 116 | 21 | 11 | 60 | 21 |
| Nathan MacKinnon | 298 | 84 | 42 | 46 | 21 |
| Adam Fox | 131 | 88 | 7 | 29 | 142 |
| Matty Beniers | 126 | 25 | 29 | 59 | 55 |

These are rounded 84-game pace totals from the selected observations, not
predicted season outcomes. Killorn's observed 0.7083 hits/game and 0.25 blocks/game
now appear beneath the corresponding projected totals.

## Validation

247 browser tests and 22 focused Rust tests passed. Coverage includes restoring
missing presentation fields, zero/missing distinction, ratio preservation,
custom target length, archive exclusion, adjacent-season isolation, unchanged
grades, record binding, old-catalog recovery and existing deployment contracts.

Five live cards (Killorn, MacKinnon, Fox, Aho, Beniers) passed exact displayed
totals/rates, actual worker recovery from an older catalog, retained task and
workload sections, mobile overflow, one-page printing and no page errors. Five
Letter PDFs were independently checked with pypdf for all seven fields and the
pace basis. Desktop screenshot inspected. Native CLI/Askama and WASM builds
validate shared types and native template rendering contracts.

Ignored helpers/outputs: `target/estimated-card-study/`. Replay the retained
projection with `node target/estimated-card-study/project.mjs`; live acceptance
is `node icelines-browser/e2e/rinkcraft-estimated-snapshot.mjs` against the existing
8780 server. It does not start one. No hosted publication.

Preview: <http://127.0.0.1:8780/?player=8473986>.
