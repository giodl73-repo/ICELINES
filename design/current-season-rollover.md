# Current season rollover

**Owner phase**: Jim Gregory - release and operations hardening
**Applies to**: October NHL season rollover and any release that changes
`CURRENT_SEASON`.

IceLines treats `CURRENT_SEASON` as the active schedule/roster context and the
newest bundle as the latest completed embedded-stat season. Those seasons may
match or differ by exactly one year and must move through code, data, docs, and
release checks as one coordinated rollover.

## Invariants

- `icelines-core/src/lib.rs` `CURRENT_SEASON_STR` matches or immediately follows
  `icelines_fetch::bundled::BUNDLED_SEASONS[0]`. The active schedule/roster
  season can be one year ahead of the newest completed stats bundle.
- `BUNDLED_SEASONS` contains completed stat seasons, newest first.
- `BUNDLED_SEASONS` contains 38 completed seasons from 1987-88 through the
  newest completed season, excluding the 2004-05 lockout (`20042005`).
- Newest-completed-season regular bios, regular stats, and goalie stats are
  embedded so the release binary cold-starts without network access.
- Playoff files may be empty for the active season before the playoffs are
  contested; that state must produce a clear `MissingBundle{Playoff}` path
  rather than falling through to regular-season data.

## Rollover Procedure

1. Update `icelines-core/src/lib.rs`:
   - `CURRENT_SEASON`
   - `CURRENT_SEASON_STR`
2. Add the newly completed stats season under `data/seasons/YYYYZZZZ/` with at least:
   - `bios.json`
   - `stats.json`
   - `goalie-stats.json`
   - `playoff-bios.json`
   - `playoff-stats.json`
   - `playoff-goalie-stats.json`
3. Insert the new season at the head of every bundled table in
   `icelines-fetch/src/bundled.rs`:
   - `BUNDLED_BIOS`
   - `BUNDLED_STATS`
   - `BUNDLED_GOALIES`
   - playoff tables
   - `BUNDLED_TRANSACTIONS` if transaction data exists
   - `BUNDLED_SEASONS`
   - `MODERN_BUNDLED_SEASONS`
4. Drop the old tail season only if the product decision is to keep exactly 38
   seasons. Never add `20042005`.
5. Update docs that mention the season range:
   - `README.md`
   - `docs/guides/04-data.md`
   - `src/guides/04-data.source.md`
   - `design/specs/data-bundles.md`
   - `design/release-checklist.md`
6. Regenerate prospective season-card fixtures when the schedule/calendar
   changes. Regenerate completed replay fixtures only from a leakage-safe
   rolling replay with all games final. Never relabel an older card fixture.
7. Run the release/data/card gates.

## Required Gates

```powershell
cargo fmt --check
cargo test -p icelines-fetch bundled
powershell -ExecutionPolicy Bypass -File scripts/validate-card-document.ps1 -Path examples/season-simulation-card-nyr-2026-27.json
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-release
```

The `bundled` test filter includes the release rollover fence that asserts the
current season matches or immediately follows the newest completed bundled
season and that the 2004-05 lockout stays excluded.

## Data Freshness Contract

- GitHub Actions `data-bundle.yml` refreshes the current-season GitHub data
  tarball weekly.
- Embedded binary data changes only when refreshed `data/seasons/` files are
  committed to the repository and the binary is rebuilt.
- Stable release binaries therefore carry the data that was committed at tag
  time. Users who need fresher current-season data should run `icelines fetch
  all` for snapshots or `icelines data install --season YYYYZZZZ --force` when a
  fresher data release exists.

Do not describe embedded binary data as automatically refreshed weekly unless
the workflow also commits the refreshed files before the release build.
