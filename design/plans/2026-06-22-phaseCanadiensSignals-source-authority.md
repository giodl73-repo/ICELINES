# Phase Canadiens Signals - Source authority

Status: Closed

## Intent

Promote Signals toward major-stats readiness by giving single-player Signals a
machine-readable source authority contract without adding rankings, filters, or
new signal math.

## Scope

- Add shared `PlayerSignalsView.source_authority`.
- Expose the authority in CLI `signals --json`, Web `/player/:id/signals`, and
  `/api/v1/player/:id/signals`.
- Name covered inputs: season summary, realtime stats when loaded, ice time when
  loaded, and the minimum-games threshold.
- Name blocked claims: prediction, betting edge, injury signal, deployment
  recommendation, player-quality grade, autonomous coaching, StatId promotion,
  and leaderboard ranking.
- Preserve missing evidence as `unavailable`/`null`, not zero.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-core view_model::signals`
- `cargo test -p icelines-web --test l1_router player_signals`
- `git diff --check`
