# Small Unblocks Inventory

Pulse 01 reviewed the Tier 2 backlog in `design/plans/INDEX.md` and checked the
current code/spec state for the three listed small unblocks.

## Residual Map

| Residual | Source | Decision | Pulse | Notes |
|---|---|---|---|---|
| `headshot.rs` test coverage | `design/plans/INDEX.md`; `design/specs/headshot-rendering.md` | spec drift | 02 | `icelines-cli/src/tui/headshot.rs` already has L0 coverage for braille bit layout, threshold constants, cache markers, clone sharing, and disk-cache roundtrips. The spec still says no `#[cfg(test)]` blocks exist. |
| `tui-admin-overlay` test coverage | `design/plans/INDEX.md`; `design/specs/tui-admin-overlay.md` | spec drift | 02 | `app.rs`, `screens/misc.rs`, and `screens/mod.rs` already cover capital-F toggle, Esc close, blocked keys, lowercase `f`, render phases, and overlay style. The spec still says dedicated tests are missing. |
| Bundle shift data for historical seasons | `design/plans/INDEX.md`; `design/specs/data-sources.md`; `icelines-fetch/src/shift_profile.rs` | decision | 03 | `ShiftProfile` derives linemate summaries from boxscore-shaped data, but no `data/seasons/**/shift*` bundles exist and the sync capability matrix currently keeps shifts disabled. This needs a source/capability decision, not a casual bundle claim. |

## Pulse Map

| Pulse | Owner surfaces | Owned files / discovery scope | Gates |
|---|---|---|---|
| 02 - Headshot and admin-overlay spec truth | TUI specs, plans index | `design/specs/headshot-rendering.md`; `design/specs/tui-admin-overlay.md`; `design/plans/INDEX.md`; focused test references only | proof on touched docs; `cargo test -p icelines-cli headshot --quiet`; `cargo test -p icelines-cli admin_overlay --quiet` |
| 03 - Shift-data bundle decision | Data specs, capability docs | `design/specs/data-sources.md`; `design/specs/foster-data-architecture.md`; `design/plans/INDEX.md`; `icelines-fetch/src/shift_profile.rs` discovery | proof on touched docs; focused fetch tests if code changes |
| 04 - Docs, regression gates, and closeout | Docs and wave records | README/COMMANDS only if user-facing status changes; `PHASES.md`; wave docs | docs proof; `cargo fmt --check`; focused crate tests from Pulses 02-03 |

## Stop Conditions

- Stop if shift bundling requires live network data without fixtures.
- Stop if resolving shifts requires changing the sync capability contract from
  `off` to a live mode in this wave.
- Stop if a test would need external CDN/NHL calls.
- Stop if a spec-truth update starts changing runtime behavior.
