# Phase Nordiques - Historical MoneyPuck xG

> Add bounded multi-season MoneyPuck xG fetch and season-correct MoneyPuck
> snapshot reads without changing the optional-source or regular-season-only
> contract.

**Status:** Implemented - Phase Nordiques complete

## Goals

| Goal | Why | Result |
|---|---|---|
| 1. Bounded historical fetch | The backlog called out MoneyPuck historical xG as unplanned despite an existing single-season fetch path. | `icelines fetch money-puck --seasons N` fetches the selected season plus `N-1` prior regular seasons. |
| 2. Preserve single-season behavior | Existing users and tests expect `fetch money-puck` to target the current season. | Default `--seasons 1` keeps the existing URL and snapshot behavior. |
| 3. Season-correct reads | Historical queries must not reuse whichever MoneyPuck snapshot is active. | `load_into_repo` now uses `read_tier_any_for_season` for MoneyPuck. |
| 4. Keep source boundaries | MoneyPuck remains optional, regular-season only, and schema-checked at parse time. | No playoff flag or new live-query behavior was added. |

## Validation

- `cargo test -p icelines-cli commands::fetch::tests::l0_moneypuck`
- `cargo test -p icelines-cli --test system_tests moneypuck`
- `cargo test -p icelines-fetch --test stats_loader moneypuck`

## Closeout

Phase Nordiques clears the MoneyPuck historical xG backlog item. The phase does
not bundle historical MoneyPuck data, create playoff MoneyPuck support, add
goalie GSAx, or promote any new public leaderboard semantics beyond the existing
xG catalog keys.
