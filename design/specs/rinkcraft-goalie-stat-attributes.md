# Goalie stat estimates and routine refresh

Goalies use their own six 1–20 attributes. These are practical public-stat
approximations, following the same simple scoring direction as skaters.
They do not measure technical goalie skills directly.

| Attribute | Favorable measurement and weight |
| --- | --- |
| Tracking | 100% NHL saves / shots against |
| Positioning | 100% MoneyPuck low-danger on-goal save fraction |
| Movement | 50% high-danger on-goal save fraction + 50% cross-slot rebound on-goal save fraction |
| Rebounds | 100% fewer generated rebound attempts / expected rebound attempts |
| Recovery | 100% rebound on-goal save fraction |
| Puck Play | 100% NHL assists / 60 minutes |

MoneyPuck low danger means xG < 0.08; high danger means xG >= 0.20.
For Movement's cross-slot term, both `shotRebound` and
`shotAngleReboundRoyalRoad` must be true. The latter flag is present on some
non-rebound shots in the actual files, so it is never used alone. MoneyPuck's
rebound flag describes a shot following a shot within three seconds.

Save fractions include only on-goal shots. Missed attempts are not saves.
Expected and generated rebounds both use the same shot archive's unblocked
attempts, matching the provider's rebound model. Rebounds is a model-relative
follow-up outcome, not measured rebound placement. Puck Play is deliberately a
coarse assist proxy; rare events and tied zeroes limit its discrimination.

Each measurement is ranked against same-season goalies with at least 10 GP and
100 minutes; 20 measured peers are required. Weighted favorable percentiles are
divided by five, rounded and capped to 1–20. Smaller recipients can receive
marked estimates. Missing opportunities remain unknown, while genuine zeroes
remain measured. No skater-to-goalie ranking or scale substitution occurs.

NHL counts and time supply the profile's GP/minutes, Tracking and Puck Play.
The shot-based attributes use MoneyPuck numerators and denominators together.
Provider differences in GP or on-goal totals are recorded as source notes;
a summary count never silently replaces a shot-archive denominator. These
annual source snapshots can differ and should not be treated as synchronized
tracking measurements. Capture dates and hashes remain in the refresh report.

Sources: [NHL public goalie summary](https://api.nhle.com/stats/rest/en/goalie/summary),
[MoneyPuck downloads](https://moneypuck.com/data.htm),
[MoneyPuck glossary](https://www.moneypuck.com/glossary.htm), and
[shot dictionary](https://peter-tanner.com/moneypuck/downloads/MoneyPuck_Shot_Data_Dictionary.csv).
MoneyPuck.com data is credited in the formulas and refresh report.

## Routine command

```powershell
icelines profile-refresh --season 20262027 --kind all `
  --cache-dir F:/hockey-cache/attributes `
  --output-dir F:/hockey-output/attributes-2026-11-23 `
  --previous-catalog F:/hockey-output/previous/rinkcraft-stat-attributes.json
```

Use `--kind goalies` or `--kind skaters` to refresh one family while preserving
other families and seasons from the previous catalog. Omit `--previous-catalog`
for the first run. The output directory must be new, preserving the last good
catalog. The command writes `rinkcraft-stat-attributes.json`, a matching
`season-package.json` and `refresh-report.json`, including source URLs, hashes, capture dates, notes and
normalized scoring observations. Regular seasons remain separate.

Online runs refresh the bulk public source files. Skating can reuse a cached
EDGE detail only after checking its actor, season and GP. A mismatched detail is
requested again. Optional unavailable EDGE measurements leave Skating unknown
without erasing the other five estimates. Four bounded workers acquire EDGE
reports. `--edge-captures DIRECTORY` can reuse an existing source-bound capture
archive with `capture.json`; original hashes and access dates are retained.

Repeat a run with the same arguments plus `--offline` and a different output
directory to replay checksum-verified cached inputs without network requests.
It reproduces the catalog and season package, including original source dates. It does not claim a
new observation date for cached data. Offline source absence or corruption is
explicit, and existing outputs are never overwritten.

To use the result, set `ICELINES_RINKCRAFT_STAT_ATTRIBUTES` to its catalog for
native profiles, or copy the catalog into `icelines-browser/public/` before
building the browser app. Import the matching `season-package.json` through
IceLines' data library (or use the normal live-data refresh): attachment requires matching player, season, GP and
position family, so an older season snapshot does not silently acquire newer
attribute estimates. A family-only run exports a family-only season package; use `--kind all` for
the complete league dataset. Global `--no-live` / disabled live-feed settings
also force cache-only acquisition. The command does not publish a site or
enable a scheduler.

## Next technical measurements

### Shot-sequence detail

The routine refresh now embeds optional `shot_context` data in each goalie
attribute profile. It travels through the existing GP/season/player-matched
catalog attachment for native and WASM; there is no second import or extra API
request when opening a card. Existing catalogs without this detail still load.

The compact **Shot Record** shows cross-slot rebound saving, all rebound saving
and generated follow-ups versus MoneyPuck's expected rebound total. Its expanded
breakdown and the Movement/Recovery/Rebounds formula views include:

- High-danger on-goal saves/faced, using the provider's xG ≥ 0.20 threshold.
- Cross-slot on-goal saves/faced, requiring both rebound and royal-road flags.
- Follow-up outcomes within 0–1 second and over 1 through 3 seconds.
- Follow-up outcomes after `SHOT` versus `MISS` previous events.
- Generated follow-up count and summed expected rebounds over the same unblocked
  attempts; rebound-attempt xG, attempts and misses.

All save fractions use on-goal opportunities. A missed follow-up is not a save;
no on-goal opportunity displays as unavailable rather than 0% or 100%. The
archive's `timeSinceLastEvent` is an event interval, not measured goalie reset
time. Cross-slot classification is the provider's shot-location flag, not tracked
goalie travel. In the inspected current archive, rebound flags include attempts
after misses, so these cannot all be attributed to a goalie surrendering a rebound.
Generated-follow-up figures likewise remain provider/model outcomes with team effects.

Detailed shot contexts require both sequence columns. Older minimal inputs with
neither column retain their existing grades and omit this optional detail;
partially supplied sequence columns fail explicitly. The refresh binds contexts
to the shot archive's URL, capture time and SHA-256. MoneyPuck.com is credited
on the primary record and in the expanded view.

This adds interpretable support to the existing estimates; it does not change
the six scoring recipes or translate event intervals into technique ratings.
The probability and expected-rebound definitions follow the
[MoneyPuck glossary](https://www.moneypuck.com/glossary.htm).

The [goalie puck-event ledger](rinkcraft-goalie-puck-events.md) now collects
officially credited goalie actions with source-complete game coverage. It adds
event detail without treating giveaways as failed passes or roster games as GP.

The first deeper priorities are puck-touch/pass opportunities for Puck Play,
lateral movement and reset timing for Movement/Recovery, and rebound destination
and threat for Rebounds. Those observations can replace these specific proxies
without pretending that assists, save percentage or follow-up attempts directly
measure the corresponding technique.
