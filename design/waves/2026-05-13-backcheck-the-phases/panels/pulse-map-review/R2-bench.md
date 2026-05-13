# R2 Review - bench

## Findings

### F-01 - WARN: Filtered cargo gates can pass with zero matching tests
File: `design/waves/2026-05-13-backcheck-the-phases/plans/pulse-06.md`
Finding: Gates such as `cargo test -p icelines-cli tui_watch` and similar filtered commands in later pulses are useful only if matching tests exist. Cargo can exit successfully with zero tests run.
Consequence: A pulse could check off a gate without proving the new behavior.
Fix: During each pulse's first discovery step, record the matched test names/counts. If no matching tests exist, add a focused test or replace the gate with an existing precise test target before checking it off.

### F-02 - WARN: Pulse 05 needs a named output artifact
File: `design/waves/2026-05-13-backcheck-the-phases/plans/pulse-05.md`
Finding: Pulse 05 requires a scenario inventory with counts and classifications, but it does not name the output file.
Consequence: Future agents may produce ad hoc notes that are hard to find or impossible to diff.
Fix: Add a durable artifact path, preferably under `design/waves/2026-05-13-backcheck-the-phases/`, such as `SCENARIO-INVENTORY.md`, and make the pulse gate check that file.
