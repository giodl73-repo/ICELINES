# IceLines release checklist

## Organization Window

- [ ] Inventory readiness totals and blocked authorities match the shipped registry.
- [ ] Official Frame fingerprint and all-32 cohort are stable across input order/platforms.
- [ ] Missing or uneven evidence withholds rank and is not zero-filled.
- [ ] Saved board, CLI/TUI/Web projections, API JSON, and cards agree.
- [ ] Historical/scenario comparisons reject incompatible manifests or incomplete sources.
- [ ] Calibration claims include leakage audit, baseline, ablation, and multi-season evidence.
- [ ] Profile-author, custom-Frame, compatibility, cache, and changelog docs match the release.

Local W9 evidence recorded 2026-07-27: fixed-hash replay passes on Windows;
focused CLI/TUI/Web tests and nine schema checks pass; `cargo audit` has no
blocking vulnerability; offline release smoke includes The Window; the Windows
ZIP/checksum/package manifest verify; and the live desktop/tablet/mobile Window
browser-review script passes. Leave the checklist open until remote three-OS
CI, full package matrix, real multi-season calibration evidence, and automated
cross-surface golden parity complete. The local keyboard/reduced-motion and
390px containment review passes.

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
- Confirm card fixtures, routes, and `design/specs/surface-parity.md` reflect
  any changed card builder or IceCast contract.

## 3. Data and season sanity

- Confirm `icelines-core/src/lib.rs` has the intended `CURRENT_SEASON` and
  `CURRENT_SEASON_STR`.
- Confirm the current season matches or immediately follows the newest
  completed bundled-stat season and that the list excludes the 2004-05 lockout.
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

For UI-neutral card or IceCast simulation/replay changes, also run:

```powershell
cargo test -p icelines-core --test season_simulation_card
cargo test -p icelines-web --test team_card_routes
cargo test -p icelines-cli --bin icelines tui::screens::team_card::tests
powershell -ExecutionPolicy Bypass -File scripts/validate-card-document.ps1 -Path examples/season-simulation-card-nyr-2026-27.json
powershell -ExecutionPolicy Bypass -File scripts/test-card-reference-renderer.ps1
```

The NYR and SEA focused season cards must share the same complete league-run
fingerprint for a given artifact. Completed replay cards must label actual and
calibration metrics as confirmed evidence and retain zero pending games.

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
release binary plus `ICELINES-PACKAGE.txt`, records the binary SHA-256 in the
package manifest, writes `icelines-windows-x86_64.zip.sha256`, verifies the
archive contents, and then runs the release smoke unless `-SkipSmoke` is
supplied.
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
Windows) plus `ICELINES-PACKAGE.txt` with the release version, source commit,
binary SHA-256, and build timestamp. Each archive is published with a same-name
`.sha256` sidecar. The release workflow verifies archive existence, checksum
sidecar integrity, expected binary membership, and manifest membership before
upload.

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
Get-FileHash -Algorithm SHA256 .\icelines-windows-x86_64.zip
Get-Content .\icelines-windows-x86_64.zip.sha256
powershell -ExecutionPolicy Bypass -File scripts\verify-release-artifact.ps1 -ArtifactPath .\icelines-windows-x86_64.zip
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
| `RUSTSEC-2026-0190` (`anyhow`) | unsound warning | direct workspace dependency | FORGE | The advisory affects `Error::downcast_mut`; IceLines does not call that API. Audit remains visible and blocking for vulnerability-class findings. | Upgrade immediately when a fixed compatible `anyhow` release is published; reassess sooner if `downcast_mut` enters the codebase. |
