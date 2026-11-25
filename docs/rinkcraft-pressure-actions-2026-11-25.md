# Protecting-lead actions — 2026-11-25

Under Pressure now distinguishes outcome samples from contextual action counts.
The new **Protecting lead** observation counts credited opponent-shot blocks and
takeaways in regulation period three at 15:00 or later, when the actor's team was
leading by exactly one before the event. Its primary text contains counts, never
a percentage. The disclosure shows all-situations counts and roster-game coverage.
All manpower situations are included; these are not adjusted five-on-five counts.

This adds something observable for defensemen without claiming to measure entry
denial, retrieval contests, assignments, possession retention or defensive skill.
Those gaps remain. A zero means no credited action in the supplied sample, not
that the player had no deployment or failed an opportunity. Roster appearances
are not on-ice exposure. The context boundary is authored, not a validated measure
of psychological pressure. No feat, attribute, class or gameplay bonus changes.

## Source findings

The same frozen 377-game NHL public play-by-play cache is reused, with its
original URL/capture/SHA receipts; no new statistical source was fetched.
The cache contains:

| Admitted action | All situations | Protecting a late one-goal lead |
|---|---:|---:|
| Opponent-shot blocks | 10,546 | 231 |
| Takeaways | 3,026 | 33 |

The blocked-shot owner is the shooter's team, not the blocker's team. Roster teams
identify the opposing blocker. There are 963 teammate-blocked shots that receive
no defensive block credit here. One `other-block` event has no named blocker and
also receives no player credit. Other missing/unknown actors and inconsistent
team ownership fail admission. The sample does not enumerate attempted or
available block/takeaway opportunities, so neither column supplies a success rate.

The unassigned event is retained in the original capture of
[game 2026020218](https://api-web.nhle.com/v1/gamecenter/2026020218/play-by-play).
Reproduction uses the captured bytes; this live URL may subsequently change.

All 787 covered skaters receive an observed sample: 273 defensemen and 514
forwards. Positive late-lead counts occur for 90 defensemen and 105 forwards.
These are opportunity/deployment-sensitive counts, not a comparison of quality.
Fox has 39 blocks and ten takeaways in all situations, including one late-lead
block and no late-lead takeaways across 23 roster games in the cached sample.

## Integration

Rust produces and validates optional `action_sample` and derived `actions`,
separate from the faceoff sample and success/opportunity entries. The raw sample
binds actor, season, source capture/fingerprint, roster-game coverage and global
cached-game coverage. Offline `profile-records`, routine `profile-refresh`, native
projection and WASM projection preserve the sample. Old catalogs omit these
fields and do not acquire invented zero counts. Goalies retain their separate
shot-response catalog.

Browser and Askama render the same Rust result under **Context actions · counts
only**, retaining the compact sheet and existing expanders. No hockey arithmetic
is performed in JavaScript. Prior pressure acceptance scripts now select outcome
rows separately from the new action rows.

## Reproduction

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-records --season 20262027 --catalog target/pressure-states-study/enriched/rinkcraft-stat-attributes.json --output-dir target/pressure-actions-study/enriched --game-cache-dir target/goalie-puck-study/cache
```

Choose a fresh output directory. Generated files and independent audit helpers
remain ignored under `target/pressure-actions-study`. Measurement definitions:
[Under Pressure v1](../design/specs/rinkcraft-under-pressure-v1.md).

Input catalog SHA-256:
`498a707658d2ccf7d31346bbe37887fe6d4122da13418967646ce9f3f5319b05`.
Output catalog SHA-256:
`4a8fd139f45540b84a324f59bc3fe7f45b165a5b5d7dafa696c0b17aef6dcb9a`.
The catalog, report and source receipts replay byte-for-byte. An independent
cache scan matches every action count and roster-game count for all 787 skaters.
All 3,948 profile score arrays and existing pressure outcome entries are unchanged;
historical profiles are unchanged. A full offline routine refresh produces
identical action samples, including source fingerprints, for all 787 skaters.

Browser acceptance:
`node icelines-browser/e2e/rinkcraft-pressure-actions.mjs`.
Local preview: [Fox](http://127.0.0.1:8780/?player=8479323), expand **Protecting lead**.
No hosted publication or merge is part of this change.

## Local checks

- 249 browser tests passed.
- Twenty focused core tests, nine source tests and three refresh integration
  tests passed. Cases cover opponent/owner attribution, teammate and unassigned
  exclusions, malformed actors/clocks, score context, overtime exclusion,
  count bounds, old records and preserving samples through a profile rebuild.
- Targeted Rust formatting checks passed.
- Native CLI/Askama and pinned unoptimized WASM builds passed.
- Eight live cards passed action text, preserved outcome entries, actual worker
  parity, retained goalie responses, defensive gaps, legacy fallback, mobile
  overflow and single-page print checks. No page errors. Fox's expanded action
  disclosure and a full skater sheet were visually inspected.
- Eight one-page 612×792-point Letter exports were independently checked with
  pypdf. Attributes and Estimated Record remain present; expanded explanations
  stay off the printed sheet even when their disclosure is open.
