# ViewModel Closeout Tasklist

Date: 2026-05-11

## Completed in this wave

- Added shared `MutationResultView` with `applied` / `noop` status.
- Added `FantasyLeagueView` for fantasy league/team management reads.
- Routed CLI fantasy league/team listing through `FantasyLeagueView`.
- Added mutation intent/result helpers for:
  - `FavoriteMutationIntent`
  - `WatchRuleMutationIntent`
  - `SeasonTypeMutationIntent`
  - `ConfigMutationIntent`
  - `DataMutationIntent`
  - `SnapshotMutationIntent`
- Routed existing mutation paths through shared intents where practical:
  - web favorites add/remove
  - CLI watch enable/disable
  - CLI config set/reset
  - CLI data install/remove/verify
  - CLI snapshot use/delete
- Mounted read-only web admin JSON endpoints for:
  - `DataStatusView` at `/api/v1/admin/data-status`
  - `SnapshotView` at `/api/v1/admin/snapshots`
  - runtime `ConfigView` at `/api/v1/admin/config`
- Mounted `/admin` as a read-only operational HTML shell consuming the same
  admin ViewModels.
- Added web JSON admin mutation endpoints returning `MutationResultView`:
  - `POST /api/v1/admin/config/set`
  - `POST /api/v1/admin/config/reset`
  - `POST /api/v1/admin/snapshots/activate`
  - `POST /api/v1/admin/snapshots/delete`
  - `POST /api/v1/admin/data/verify`
- Added web JSON mutation twins for favorites:
  - `POST /api/v1/favorites/add`
  - `POST /api/v1/favorites/remove`
  - both return `MutationResultView`.
- Added web JSON watch-rule enable/disable mutation:
  - `POST /api/v1/watch-rules/set-enabled`
  - returns `MutationResultView`.
- Added watchlist rule toggle UI:
  - `/watchlist` renders persisted watch rules with enable/disable forms.
  - `POST /watch-rules/set-enabled` uses the same `WatchRuleMutationIntent`
    path and redirects back to `/watchlist`.
- Added watchlist player-rule creation UI:
  - `/watchlist` includes an add-rule form for promotion/availability triggers.
  - `POST /watch-rules/create` persists a core `WatchRule` and redirects back.
- Added watchlist rule deletion UI:
  - `/watchlist` includes delete forms for persisted rules.
  - `POST /watch-rules/delete` removes a persisted rule and redirects back.
- Reconciled legacy invariant/pitfall language:
  - DI/AI/II/SI seed invariants now use `StatsRepository`, `PlayerView`, and
    ViewModel terms instead of retired CSV `Player` / `FitClass` / generated-site
    assumptions.
  - `PITFALLS.md` now marks early DP/AP/SP wording as legacy vocabulary.
  - `fantasy-leagues.md` no longer names the deleted `PlayerRepository` or
    `Player::pace_score` paths.
- Made fantasy mutation scope explicit:
  - main dashboard `/fantasy` stays read/product ViewModel-backed this phase;
  - CLI and legacy `fantasy serve` remain the league/team write surfaces until
    a dedicated web write-flow phase.
- Updated `viewmodels.md`, `surface-parity.md`, and `INVARIANTS.md`.

## Remaining

- Full web mutation parity:
  - richer arbitrary watch-rule edit UI;
  - fantasy league/team mutation UI is explicitly deferred out of this closeout.
- Admin HTML write controls:
  - web JSON mutation twins now exist for safe/runtime admin operations;
  - destructive install/remove and richer HTML controls remain deferred.
- TUI admin polish:
  - data/snapshot/config overlays should render directly from the new intent/result contracts when they expose mutations.
## Suggested next order

1. Add mutation JSON routes returning `MutationResultView`.
2. Add watch-rule editor/toggle UI.
3. Start a dedicated fantasy web write-flow phase when we want main-dashboard
   league/team roster mutations.
