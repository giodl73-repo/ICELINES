# Goalie shot contexts — 2026-11-23

The existing Rust refresh now embeds optional shot-sequence contexts in the
attribute catalog. Native and browser cards reuse the current actor/season/GP
attachment; no new source loader or scoring recipe is introduced.

Validation completed:

- Six Rust source integration tests pass, including the new case separating
  misses, 0–1s follow-ups, later follow-ups, cross-slot flags and actual saves.
  Zero on-goal opportunities stay unknown; malformed intervals and contradictory
  rendered rows fail. Legacy minimal inputs retain estimates without detail.
- Native CLI build passes, including Askama templates. Rust 1.95.0 WASM build
  and matching bindings pass.
- Browser build and all 244 browser tests pass. A regression test checks that
  a newly attached catalog supplies detail ahead of an older embedded estimate.
- All 78 current goalies have contexts and pass actual WASM catalog attachment.
  All 3,948 supplied profiles retain exactly the same six 1–20 grades. Other
  numerical score fields agree within 1e-10; maximum float serialization
  difference from the prior catalog is 1.42e-14.
- Three contrasting actual cards pass the browser acceptance script, including
  Movement/Recovery/Rebounds dialogs, sequence disclosures, real worker parity,
  390px mobile width and no page errors. Screenshots were inspected.
- Three exports independently contain both Shot Record and Puck Record on one
  612×792-point Letter page; the acceptance script also enforces one page.

## Reproduction

From the owned Rinkcraft worktree:

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-refresh --season 20262027 --kind goalies --cache-dir target/goalie-refresh/cache --output-dir target/goalie-shot-context/refreshed --previous-catalog target/goalie-refresh/all-current-final/rinkcraft-stat-attributes.json
```

An independent repeat into `target/goalie-shot-context/replay` yields the same
catalog SHA-256:
`f4b92d95bd65b9c25c8bfb8300608d8c6ea29125e787543eae2b3ed0abe4a03a`.
The matching goalie-only season package also repeats exactly:
`e5a434928dac19d503768e24b8f3327c969ae8c729d902c2aa39c464de053413`.
The source archive is the previously captured
`https://peter-tanner.com/moneypuck/downloads/shots_2026.zip`, SHA-256
`8129832669e968fa7da93102311b72b864276eb9cf2011277760f0beb2322585`,
captured 2026-11-24T00:17:34.545762500+00:00 (November 23 locally).

## What the data teaches

The 78 goalies' contexts contain 2,457 provider-tagged rebound attempts. Of
these, 682 (27.8%) have `MISS` as their preceding event. Such quick follow-ups
cannot all be described as a goalie mishandling a save. Save percentages use
only on-goal attempts; event intervals do not measure goalie reset speed.

Shesterkin's archive context includes 4/5 cross-slot rebound shots saved,
11/13 rebound shots saved, and 20 generated follow-ups against 14.73 expected
over 177 unblocked attempts. These are sample-specific outcomes, not isolated
technique measurements. MoneyPuck.com is credited on the card and in its source
disclosure. See the [measurement definitions](../design/specs/rinkcraft-goalie-stat-attributes.md).

The loopback 8777 preview serves generated profile snapshots and the actual
matching worker/WASM from `target/goalie-refresh/site/`. Projection scripts,
new catalog, WASM and coverage results remain ignored under
`target/goalie-shot-context/`; current screenshots/PDFs and browser acceptance
results are under `target/goalie-puck-profile/`. No hosted publication is claimed.
