# Phase Canadiens Signals - Markdown authority

Status: Closed

## Intent

Carry the shared Signals source-authority contract into `export md signals` so
the Markdown report packet names the same covered inputs, covered metrics,
blocked claims, and limitations as the JSON/Web Signals surfaces.

## Scope

- Render `PlayerSignalsView.source_authority` in Markdown before the signal
  table.
- Keep disclosure, non-claim copy, evidence tiers, missing-input labels,
  methodology, and limitations intact.
- Preserve missing evidence as `unavailable`, never zero.
- Document that Markdown exports share the Signals authority vocabulary.

## Validation

- `cargo fmt --check`
- `cargo test -p icelines-cli export_signals`
- `cargo test -p icelines-cli --test signals_system export_md_signals`
- `git diff --check`
