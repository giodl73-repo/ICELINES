# Current season rollover

**Owner phase**: Jim Gregory - release and operations hardening
**Applies to**: October NHL season rollover and any release that changes
`CURRENT_SEASON`.

IceLines treats the newest bundled season as the release default. The default
must be updated in code, embedded data, docs, and release checks as one unit.

## Invariants

- `icelines-core/src/lib.rs` `CURRENT_SEASON_STR` equals
  `icelines_fetch::bundled::BUNDLED_SEASONS[0]`.
- `BUNDLED_SEASONS` is newest first.
- `BUNDLED_SEASONS` contains 38 supported seasons from 1987-88 through the
  current season, excluding the 2004-05 lockout (`20042005`).
- Current-season regular bios, regular stats, and goalie stats are embedded so
  the release binary cold-starts without network access.
- Playoff files may be empty for the active season before the playoffs are
  contested; that state must produce a clear `MissingBundle{Playoff}` path
  rather than falling through to regular-season data.

## Rollover Procedure

1. Update `icelines-core/src/lib.rs`:
   - `CURRENT_SEASON`
   - `CURRENT_SEASON_STR`
2. Add the new season under `data/seasons/YYYYZZZZ/` with at least:
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
6. Run the release/data gates.

## Required Gates

```powershell
cargo fmt --check
cargo test -p icelines-fetch bundled
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-release
```

The `bundled` test filter includes the release rollover fence that asserts the
current season is the newest bundled season and that the 2004-05 lockout stays
excluded.

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
