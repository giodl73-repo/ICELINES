# Pulse 06: Layout persistence hardening proof

## Goal

Use the existing WP-002 layout persistence capability in the Rangers round
without rebuilding the schema or overstating interactive TUI/browser evidence.

## Scope

Added `scripts/layout-proof.ps1`, a temp-home proof that exercises the
durable layout CLI:

```powershell
icelines layout save rangers-stats --center stats --left favorites-left --right schedule-right
icelines layout list
icelines layout show rangers-stats
icelines layout delete rangers-stats
```

The script asserts stable center and pane IDs plus the
`preserve-active-context` policy, then deletes the temp-home store.

## Non-claims

- No layout schema change was made.
- No new interactive TUI restore claim was added beyond existing WP-002 evidence.
- No Web dashboard behavior was changed.
- No user home/config layout store is touched; the proof runs in an isolated
  temp home.

## Validation

| Command | Result |
|---|---|
| `powershell -ExecutionPolicy Bypass -File scripts\layout-proof.ps1` | passed |
| `cargo fmt --check` | passed |
| `cargo test -p icelines-core workbench_layout --lib` | passed |
| `cargo test -p icelines-cli --bin icelines layout` | passed |
| `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only` | passed |
| `git diff --check` | passed |

## Result

Status: passed.

Rangers now has an isolated workflow proof for durable layout persistence. The
remaining Rangers item is the lean CLI audit/fence.
