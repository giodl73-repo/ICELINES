---
wave: import-the-rosters
pulse: 04
date: 2026-05-16
status: complete
governing_roles:
  - wire
  - forge
  - bench
  - glass
---

# Pulse 04 - CLI, TUI, and Dashboard Import Surfaces

## Goal

Expose the roster import path through safe, discoverable surfaces after the
shared contract and data path exist.

## Owned Scope

- Add a CLI command such as `icelines fantasy import-yahoo --file <path>
  --league <name> [--my-team <name>] [--dry-run] [--json]`.
- Render text/JSON from the shared import contract.
- Add TUI command-bar and web-dashboard command handoffs that point to the CLI
  import flow or truthful POST-only/browser deferral.
- Add focused clap, CLI L2, TUI command parser, and dashboard command tests.
- Ensure no web/dashboard GET route mutates FantasyDb.

## Non-goals

- No web file upload unless a POST-backed route and multipart safety contract are
  explicitly added in this pulse.
- No full TUI import wizard.
- No automatic file discovery outside the user-supplied path.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-cli fantasy_import --quiet`
- [x] `cargo test -p icelines-cli l2_cmd_fantasy_import --test system_tests --quiet`
- [x] `cargo test -p icelines-web fantasy_import --quiet`

## Result

Added `icelines fantasy import-yahoo --file <path> --league <name>
[--my-team <name>] [--dry-run] [--json]` over the shared
`FantasyImportView` contract. TUI command-bar import phrases now hand off to the
CLI flow, and the web dashboard command parser rejects browser import as a
truthful POST-only deferral so GET navigation remains read-only.

## Stop Conditions

- Stop if dashboard/web import would mutate through GET.
- Stop if the CLI cannot dry-run without changing the isolated test DB.
- Stop if text output hides skipped/error rows.
