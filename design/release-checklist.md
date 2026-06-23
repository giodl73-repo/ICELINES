# IceLines release checklist

**Owner phase**: Jim Gregory - release and operations hardening
**Applies to**: tagged GitHub releases and local release candidate builds

This checklist is the release gate for IceLines. It assumes releases are cut
from `master` after CI is green.

## 1. Decide release type

| Type | Use when | Version example |
|---|---|---|
| Patch | bug fix, docs, visual polish, test gate, no incompatible output shape | `0.24.1` -> `0.24.2` |
| Minor | new user-facing surface, command, route, ViewModel, or output field | `0.24.1` -> `0.25.0` |
| Data-only | bundled data refresh with no code behavior change | keep code version unless a release artifact must be replaced; record data provenance |
| Pre-release | release candidate or risky packaging check | `0.25.0-rc.1` |

Do not tag a release without a changelog entry and a green CI run for the
commit being tagged.

## 2. Version and docs

- Update workspace version in `Cargo.toml`.
- Confirm `cargo metadata --no-deps` reports the expected workspace/package
  version.
- Update `CHANGELOG.md` with the release heading and date.
- Confirm `README.md` quick start still matches the binary.
- Confirm `COMMANDS.md` is current or intentionally unchanged.
- Confirm `design/plans/INDEX.md` and `design/phases.md` reflect any phase
  status changes.

## 3. Data and season sanity

- Confirm `icelines-core/src/lib.rs` has the intended `CURRENT_SEASON` and
  `CURRENT_SEASON_STR`.
- Confirm the bundled season list includes the current season and excludes the
  2004-05 lockout.
- Confirm README data-source claims match reality:
  - bundled binary data;
  - GitHub release data bundles;
  - live fetch commands.
- For October rollover, follow the current-season procedure before tagging.
  Do not silently roll only the README or only the constant. Full procedure:
  `design/current-season-rollover.md`.

## 4. Required local gates

Run the normal focused gates before creating a tag:

```powershell
cargo fmt --check
cargo check --workspace
cargo test -p icelines-fetch bundled
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-audit
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci-release
```

For code changes touching shared contracts or multiple surfaces, also run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 ci
```

The `ci` slice includes the dependency vulnerability audit, clippy, fmt, and
release build/smoke sequence after the split test gates.

For visual-system changes, run:

```powershell
cargo test -p icelines-cli prince_tui
cargo test -p icelines-cli --test prince_cli_visual
cargo test -p icelines-web l1_static_css_contains_prince_route_layout_classes
```

`scripts/release-smoke.ps1` must pass on the optimized binary. It verifies
version/help, representative CLI outputs, docs, markdown export, poach, and
`serve --no-open` URL printing without requiring live network fetches.

For local Windows artifact assembly before a tag or release draft, run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package-release.ps1
```

The script creates `dist\release\icelines-windows-x86_64.zip`, includes the
release binary plus `ICELINES-PACKAGE.txt`, verifies both files are present in
the archive, and then runs the release smoke unless `-SkipSmoke` is supplied.
GitHub Actions remains the canonical builder for macOS and Linux artifacts.

## 5. Manual release smoke

After `cargo build --release -p icelines-cli`, run:

```powershell
target\release\icelines.exe --version
target\release\icelines.exe query leaders --top 3 --season 20242025
target\release\icelines.exe query goalies --top 3 --season 20242025
target\release\icelines.exe tui --help
target\release\icelines.exe serve --help
target\release\icelines.exe export md leaders --out - --top 3
powershell -ExecutionPolicy Bypass -File scripts\release-smoke.ps1 -SkipBuild
```

Expected: all commands exit 0 and `serve --no-open` prints the localhost URL
before any browser-open behavior.

## 6. Artifact names

GitHub release workflow artifacts:

| Platform | Artifact |
|---|---|
| Windows x64 | `icelines-windows-x86_64.zip` |
| macOS Apple Silicon | `icelines-macos-arm64.tar.gz` |
| macOS Intel | `icelines-macos-x86_64.tar.gz` |
| Linux x64 | `icelines-linux-x86_64.tar.gz` |

Each archive should contain the single `icelines` binary (`icelines.exe` on
Windows). The release workflow verifies archive existence and expected binary
membership before upload.

## 7. Tag and release

```powershell
git status --short
git log --oneline -5
git tag vX.Y.Z
git push origin vX.Y.Z
```

The release workflow builds matrix artifacts and creates the GitHub Release.
After it finishes, download at least the Windows artifact and run:

```powershell
icelines.exe --version
icelines.exe query leaders --top 3
```

## 8. Rollback

If a tag was pushed but artifacts are wrong:

- Delete the GitHub Release draft/release.
- Delete the local and remote tag only if the release should not exist:
  `git tag -d vX.Y.Z` and `git push origin :refs/tags/vX.Y.Z`.
- Fix on `master`, rerun the gates, and retag.

If a release is public and users may have downloaded it, prefer a patch release
over replacing artifacts in place.

## 9. Known advisory gates and cargo-audit policy

- Browser screenshots are currently manual/advisory. Prince of Wales recorded
  the visual contract; Jim Gregory should add automated screenshots when the
  harness is stable.
- `cargo audit` is a blocking CI/local gate for vulnerability advisories. Run it
  with `powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1
  ci-audit`; the script installs `cargo-audit --locked` when missing.
- Warning-class advisories are not ignored in config and are not hidden. They
  remain visible in audit output and must be listed below with advisory ID,
  dependency path, risk owner, rationale, and removal condition.
- If `cargo audit` exits nonzero, fix or upgrade the dependency first. Only add
  a time-boxed exception after documenting advisory ID, reason, owner, and
  removal condition in this section.
- Performance benchmarks are advisory until a specific budget is documented.

Current warning-class advisory ledger:

| Advisory | Class | Dependency path | Owner | Rationale | Removal condition |
|---|---|---|---|---|---|
| `RUSTSEC-2025-0052` (`async-std`) | unmaintained warning | `httpmock` -> `icelines-fetch` tests | BENCH / FORGE | Test-only mock dependency; not shipped in the release binary. Keep visible so a fixture/mock replacement can retire it. | Remove when `httpmock` drops `async-std`, or replace the mock stack if the warning becomes a vulnerability or runtime dependency. |
| `RUSTSEC-2024-0436` (`paste`) | unmaintained warning | `ratatui` -> `icelines-cli` | FORGE | Transitive TUI dependency with no current vulnerability finding. Not ignored; audit continues to report it. | Remove when the TUI stack upgrades past the `paste` dependency, or replace the dependency if the warning becomes a vulnerability. |
| `RUSTSEC-2026-0002` (`lru`) | unsound warning | `ratatui` -> `icelines-cli` | FORGE | Transitive TUI dependency; IceLines does not call `lru` directly. Not ignored; audit continues to report it for upgrade pressure. | Remove when `ratatui` upgrades past affected `lru`, or patch/replace the TUI dependency if RustSec reclassifies the issue or a reachable unsafe path is identified. |
