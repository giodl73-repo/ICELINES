# R1 Review - keel

## Findings

### F-01 - WARN: Pulse graph needs an execution order beyond `depends_on: [pulse-01]`
File: `design/waves/2026-05-13-backcheck-the-phases/WAVE.md`
Finding: Pulses 03-08 are all marked as depending only on Pulse 01, but they share `COMMANDS.md`, `surface-parity.md`, and visual/report surfaces. Pulse 04 captures visual evidence that should represent the final wave state, and Pulse 08 is docs/parity-heavy.
Consequence: Running the pulses in raw numeric order can cause doc conflicts and stale visual captures, especially if Pulse 04 runs before team-season export, watch UX, admin, and career/docs changes settle.
Fix: Use an execution queue of Pulse 03 -> Pulse 05 -> Pulse 06 -> Pulse 07 -> Pulse 08 -> Pulse 04, or update `WAVE.md` / fork files with those dependencies before dispatch.

### F-02 - NOTE: Pulse 05 is the planning bridge before product followups
File: `design/waves/2026-05-13-backcheck-the-phases/plans/pulse-05.md`
Finding: Scenario classification is not product code, but it should run before UX/admin/doc pulses so new tests can be chosen from the scenario corpus rather than added after the fact.
Consequence: If Pulse 05 runs late, product pulses may add tests that duplicate or miss the highest-value persona coverage.
Fix: Run Pulse 05 immediately after Pulse 03's export slice or in parallel only if it does not edit shared docs.
