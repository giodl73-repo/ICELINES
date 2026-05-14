# Pulse 02 - Play-by-Play Fetch/Store Path

## Goal

Install the cached event source required by future event-backed records. This
pulse does not promote new records metrics yet; it creates the safe data path
those metrics will consume.

## Governing lenses

- **wire**: preserve raw NHL JSON before deriving any record rows.
- **tape**: keep `goalieInNetId` and fight participant ids as source fields,
  not inferred fields.
- **forge**: keep parsing/fetching in `icelines-fetch`; keep CLI as a thin
  command dispatcher.
- **edge**: model goalie and penalty participant ids as optional at the
  boundary.
- **glass**: expose one discoverable fetch command instead of hidden setup.

## Implementation

1. Added `DataKind::PlayByPlay` and a `play_by_play.json` manifest shard.
2. Added `DataStore::load_play_by_play_raw` for manifest-backed raw JSON reads.
3. Added `PlayByPlay`, `PlayByPlayGoal`, and `PlayByPlayPenalty` projections in
   `icelines-fetch::nhl_api`.
4. Added `fetch_play_by_play` and `fetch_play_by_play_with_raw` for
   `/v1/gamecenter/{id}/play-by-play`.
5. Added `icelines fetch play-by-play` / `icelines fetch pbp` with `--date`,
   `--for-favorites`, and `--dry-run`.
6. Updated `COMMANDS.md`, clap help, and `data-status --shard play_by_play`.

## Gates

- `cargo fmt --check`
- `cargo check -p icelines-cli`
- `cargo test -p icelines-fetch play_by_play`
- `cargo test -p icelines-fetch l1_trace_events_load_play_by_play_raw_reads_persisted_file`
- `cargo test -p icelines-cli l2_trace_events_fetch_play_by_play_invalid_date_clean_error`

## Result

Pulse 02 is complete. Event-backed records can now rely on cached raw
play-by-play data rather than live network calls or aggregate inference. The
next pulse should extend records ViewModels/providers with
`goalies-scored-against`, skipping empty-net/no-goalie goals.
