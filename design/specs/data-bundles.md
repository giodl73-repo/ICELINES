# Data Bundles — Specification

**Version**: 1.0
**Date**: 2026-04-28
**Status**: Implemented

---

## Purpose

Distribute and install **historical NHL season data** as
self-contained bundles so users can analyze any of 38 seasons
(1987-88 through 2025-26, excluding the 2004-05 lockout) without
hitting the live API.

All 38 supported seasons are **embedded in the binary** via
`include_bytes!()`. GitHub Releases remain the refresh/install path for
corrected or manually refreshed bundles via `icelines data install`.

This spec covers: bundle format, install/list/remove operations,
storage layout, and the install lifecycle.

---

## Two-tier model

| Tier | Source | Location | Lifecycle |
|------|--------|----------|-----------|
| **Bundled** | `include_bytes!()` in `icelines-fetch::bundled` | Binary (38 seasons, excluding 2004-05) | Refreshed during release/data-prep work; ships with each release |
| **Installed** | GitHub Releases tarballs | `~/.icelines/seasons/{SEASON}/bundle-{SEASON}/` | User-controlled refresh/override; `data install` adds, `data remove` deletes |

Loaders prefer in this order:
1. Live snapshot store (post-`icelines fetch`)
2. Bundled in-binary
3. Installed on disk

---

## Available seasons

`AVAILABLE_SEASONS` in `icelines-cli/src/commands/data.rs` lists 38
season IDs, newest first:

```
20252026 20242025 20232024 20222023 20212022           ← bundled in binary
20202021 20192020 20182019 20172018 20162017
20152016 20142015 20132014 20122013 20112012
20102011 20092010 20082009 20072008 20062007
20052006                                                ← salary-cap era end
20032004 20022003 20012002 20002001
                                                          ← 2004-05 lockout, OMITTED
19992000 19981999 19971998 19961997 19951996
19941995 19931994 19921993 19911992 19901991
19891990 19881989 19871988                              ← bundled in binary
```

The lockout year `20042005` is not in the list. Attempting to install
it returns a friendly error explaining no games were played.

---

## CLI commands

```
icelines data install [--seasons N | --season SEASON_ID] [--force]
icelines data list
icelines data remove <SEASON_ID>
```

### install

Without flags: refresh the current season (`--seasons 1`).

```
icelines data install                          # current season only
icelines data install --seasons 5              # newest 5 (already bundled — no-op unless --force)
icelines data install --seasons 38             # full history
icelines data install --season 19931994        # specific season
icelines data install --season 19931994 --force  # re-download
```

Without `--force`, an already-installed season is skipped with a
status line. Installs are sequential (one at a time) so a slow
connection doesn't fan out 38 parallel requests against GitHub.

The `--seasons N` flag picks the **newest N** from `AVAILABLE_SEASONS`,
which means any season already present in the binary is a no-op unless
`--force` is provided to install a fresher override from GitHub Releases.

### list

Prints one row per installed season with size and install date.
Bundled-only seasons are listed as "(bundled)" with no install date.

### remove

Removes one season's directory. The season's data is no longer
accessible until reinstalled. Bundled seasons cannot be removed (the
bytes live in the binary); attempting to remove a bundled-only season
errors with a clarifying message.

---

## Bundle format

Each bundle is a gzip'd tarball at:

```
https://github.com/giodl73-repo/ICELINES/releases/download/data-{SEASON}/data-{SEASON}.tar.gz
```

Contents (extracted to `~/.icelines/seasons/{SEASON}/bundle-{SEASON}/`):

```
bundle-{SEASON}/
├── bios.json     ← all skater bios (~500 KB compressed, ~2 MB raw)
├── stats.json    ← all skater season stats (~400 KB / ~1.5 MB)
└── (future) realtime.json, moneypuck.csv, playoffs.json, schedule.json
```

The minimum required files are `bios.json` and `stats.json`; presence
of `bios.json` is the canonical "is installed" check (`bundled::is_installed()`).

Schemas match the live NHL API response shape used by
`icelines-fetch::schema::SkaterBio` and `SkaterStats`. Bundle JSON is
the same payload that would be produced by `icelines fetch stats
--season SEASON` minus pagination wrapping.

---

## Storage layout

```
~/.icelines/
├── icelines.db                   ← SQLite (groups, queries, fantasy)
├── snapshots/                    ← live-fetched data (see cache-model.md)
└── seasons/
    └── 19931994/
        └── bundle-19931994/
            ├── bios.json
            └── stats.json
```

Path resolution:
- `$USERPROFILE` (Windows) or `$HOME` (Unix) → base
- Append `.icelines/seasons/{SEASON}/bundle-{SEASON}/`

The double-nesting (`{SEASON}/bundle-{SEASON}`) matches the tarball's
top-level directory, so `tar -xzf` extracts cleanly without
`--strip-components`.

---

## Install lifecycle

`install_season(seasons_dir, season, force)`:

1. **Skip-if-installed** — unless `--force`, return early if
   `bundled::is_installed(season)` is true.
2. **Download** — GET the tarball URL with a streaming reqwest body.
   Total size is shown in the progress line.
3. **Extract** — `tar::Archive::new(GzDecoder::new(body))`. Files
   land in `{seasons_dir}/{season}/`.
4. **Verify** — assert `bios.json` exists at the expected path. If
   not, the bundle is malformed; print error, do not delete the
   partial extract (so the user can inspect).
5. **Confirm** — print `✓ {SEASON} installed ({KB}KB)`.

Any error mid-stream (network, disk, malformed tarball) is reported with
the underlying cause; the partial directory is left in place for
debugging.

---

## TUI integration

The Season Picker (`y` key, see `season-timetravel.md`) integrates with
this spec:

- Bundled seasons render with `▶`/`✓` markers — selecting them is instant.
- Uninstalled seasons render dimmed with `[not installed]`.
- Pressing `i` on a dimmed season triggers `install_season_tui()` —
  the same code path as `icelines data install --season X`, but
  spawned async with a spinner in the status bar.

`InstallPhase` (in `loader.rs`) tracks one install at a time:
`Idle | Downloading(season) | Done(season, kb) | Error(season, msg)`.
Concurrent installs are rejected with "install already in progress".

---

## Decisions (Open Questions resolved)

1. **Why GitHub Releases instead of CDN?** Free for public repos,
   versioned URLs, no infrastructure to maintain. Tags are
   `data-{SEASON}` so URLs don't collide with code releases.

2. **Tarball, not zip?** Smaller for text-heavy JSON (gzip vs deflate
   matters less, but `tar.gz` ships with every Unix and is easy on
   Windows via `tar` Win10+).

3. **No checksums in v1**: GitHub's HTTPS is the trust anchor. A
   future `--verify` could add SHA-256 from a sidecar `data-{SEASON}.sha256`
   file. Punted because nothing in v1 needs it.

4. **No automatic install on first query**: Explicit `data install` is
   required. Keeps the binary's behavior predictable; matches how
   `icelines fetch` works.

5. **Lockout year handling**: 20042005 is a hardcoded special case in
   `run_install` (returns early with explanation). Lives in two
   places (`AVAILABLE_SEASONS` omits it; `run_install` rejects with
   friendly text) for defense-in-depth.

6. **Refresh cadence for bundled seasons**: release/data-prep work
   refreshes the embedded season files before publishing. Users on
   stable releases get whatever was current at release time; running
   `data install --force` can install a corrected or fresher override
   from GitHub Releases where one exists.

7. **CDN / mirror fallback**: Not in v1. If GitHub Releases is down,
   `data install` errors. Tracked in backlog; punted because GitHub's
   uptime is acceptable for this use case.

---

## Test coverage

L0/L1: `bundled::is_installed()`, path resolution, `BUNDLED_SEASONS`
membership, tarball extraction round-trip (against a fixture file in
`tests/fixtures/`).

L2 (subprocess):
- `l2_cmd_data_list_exits_zero` — exit code + bundled status output
- `l2_cmd_data_install_lockout_year_friendly_error` — 20042005 path
- `l2_cmd_data_install_unknown_season_friendly_error` — non-AVAILABLE input

Live network is **not** tested in CI (would be flaky); manual
verification per release.

---

## Future work (v2+)

| Item | Priority | Why deferred |
|------|----------|--------------|
| SHA-256 verification | MED | Trust GitHub HTTPS for now |
| CDN / mirror fallback | MED | Single point of failure today; acceptable |
| Bundle compression tuning (zstd?) | LOW | gzip is fine; zstd saves ~30% but adds dep |
| Background pre-fetch on `tui` launch | LOW | Surprises user with bandwidth use |
| Per-season changelog (when CI updated each bundle) | LOW | Useful for reproducibility |
| Realtime / shifts / playoffs in bundle | depends | Tier 2/3 data — see data-sources.md |
