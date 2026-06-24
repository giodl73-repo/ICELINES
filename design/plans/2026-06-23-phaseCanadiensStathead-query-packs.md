# Phase Canadiens Stathead - Query packs

## Status

Closed - 2026-06-23

## Goal

Start the editorial/stathead workflow track with a lightweight, discoverable
command that turns existing IceLines query power into reusable starter packs.

## Scope

- Add `icelines stathead [pack] [--json]`.
- Ship curated packs for era leaders, young stars, playoff runs, goalie
  notebook, records notebook, fantasy prep, and draft scouting workflows.
- Add Markdown rendering for selected packs or the complete pack index with
  `--markdown` and `--out`.
- Add `--commands` for script-friendly one-command-per-line output.
- Add `--commands --read-only` to omit file-writing recipes from command-list
  output.
- Add `--commands --writes-only` to inspect only file-writing recipes.
- Use Clap-level conflicts for mutually exclusive output modes and command
  effect filters, with dispatch validation retained as a backstop.
- Cover `--read-only` misuse with a clean CLI error.
- Add per-recipe `requires` notes so JSON, text, and Markdown outputs name the
  source/data dependency behind each starter command.
- Add per-recipe `effect` notes so JSON, text, and Markdown outputs distinguish
  read-only recipes from file-writing recipes.
- Centralize read-only effect classification and guard effect values with L0
  shape tests.
- Add L0 command-shape coverage so curated commands keep a supported IceLines
  top-level command and balanced quoting.
- Surface stathead packs through the bare command banner, README quick start,
  `report list` discovery catalog, and the base `icelines stathead` listing.
- Keep `report list` metadata aligned with the read-only and writes-only
  command filters.
- Keep packs as recipes over existing commands; do not introduce new metric
  semantics or hidden execution.
- Document the command in `COMMANDS.md`.
- Cover pack contracts with L0 tests and CLI behavior with L2 subprocess tests.

## Non-Claims

- This does not execute query packs or save user query collections.
- This does not add new stat definitions, filters, data sources, or leaderboards.
- This does not add web/TUI stathead surfaces yet.

## Validation

```powershell
cargo test -p icelines-cli stathead -- --nocapture
cargo run -p icelines-cli -- stathead young-stars --json
cargo run -p icelines-cli -- stathead young-stars --commands
cargo run -p icelines-cli -- stathead --markdown
cargo run -p icelines-cli -- stathead goalie-notebook --markdown
cargo test -p icelines-cli report_list -- --nocapture
git diff --check
```
