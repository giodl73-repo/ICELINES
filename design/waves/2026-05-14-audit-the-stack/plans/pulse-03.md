# Pulse 03 - Records ownership integrity

## Findings

- R4 F-01: `records_provider` groups unknown event-owner teams under a blank team key.
- R5 F-02: Malformed ownership needs a targeted fetch-side regression before changing builders.

## Scope

Change play-by-play record input construction so missing or unknown event-owner teams are explicit: skipped with a diagnostic, represented as an unknown marker, or surfaced through a typed error. Do not let blank team keys enter records ViewModels.

## Gates

- Add L0 tests for missing owner id and owner id not matching home/away teams.
- `cargo fmt --check`
- `cargo test -p icelines-fetch records_provider`
- `cargo clippy --workspace -- -D warnings`
