# proof bug report — to file at https://github.com/giodl73-repo/PROOF/issues

(Couldn't auto-file from the EMU account — paste the body below.)

---

**Title:** `proof:chart` directives nested inside `proof:region` bodies are not processed

**Body:**

## Summary

When a `proof:chart` directive is placed inside a `proof:region` body within a `*.dashboard.source.md` file, proof passes the chart fence through as literal text instead of resolving it. `directives_resolved` reports `0` even though the file contains a chart directive.

This blocks the most natural use case for the dashboard compositor: rendering charts (sparklines, bars) inside positioned regions.

## Repro

Save as `repro.dashboard.source.md`:

````markdown
---
dashboard:
  width: 30
  height: 8
  regions:
    main: { x: 0, y: 0, width: 30, height: 8 }
---

```proof:region name=main
Trend:
```proof:chart kind=sparkline width=24
21-22: 44
22-23: 64
23-24: 32
24-25: 26
25-26: 48
```
```
````

(The inner triple-backticks are intentional — that's the chart directive nested inside the region.)

Compile:

```bash
proof compile repro.dashboard.source.md repro.md
```

## Expected

- `directives_resolved >= 1` (the chart counts).
- The region renders with an actual sparkline (`▁▂▄▇█▄▂` etc.) where the chart fence sits.

## Actual

- Output: `✓ … → repro.dashboard.md (0 directives)` — chart was never seen.
- `DASHBOARD-005` fires on overflow because the raw fence + label-value lines take more room than a rendered sparkline would.
- Compiled output contains the literal ` ```proof:chart kind=sparkline… ` fence:

```
<!-- proof:compiled from="proof:dashboard" -->
```dashboard
Trend:
```proof:chart kind=sparkline…
21-22: 44
22-23: 64
23-24: 32
24-25: 26
```
<!-- /proof:compiled -->
```

## Diagnosis (educated guess)

The dashboard region compositor in `src/dashboard/region.rs` appears to consume each region body as opaque text rather than recursively running the directive parser over it. So nested `proof:chart` (and presumably `proof:tree`, `proof:element`, `proof:figure`) directives never get reached.

## Why this matters for downstream

icelines wanted to use `proof:dashboard + proof:chart` to render player scout cards in the TUI: identity region + counting-stats region + sparkline-trend region. Without nested directive resolution, the dashboard compositor only adds value for plain text — which we already lay out cheaply with ratatui. Native sparkline rendering ended up being the right answer for our case (~30 lines of Rust), but the spec'd workflow should still work for users who want it.

## Workaround for now

Generate the chart with `proof:chart` to a separate `*.source.md`, compile it, then `cat` the result into a region body in the dashboard source. Two-pass, but it works.

## Suggested fix

Either:
1. Run the directive parser recursively on each region body before treating it as text, or
2. Document that regions are opaque text in `DASHBOARD-SPEC.md` and add a chart-aware region kind (e.g. `proof:chart-region`) that takes chart attrs directly.

Happy to test a patch if you have a preference on direction.

---

Filed from icelines after we tried `proof_lib::compile_file` against this fixture and `directives_resolved` returned 0. Versions: proof master @ `9c5d456e`, mdpath master @ `6666cb43`.
