# R1 Review - tape

## Findings

### F-01 - WARN: Current typed projection drops shot events
File: `icelines-fetch/src/nhl_api.rs`
Finding: `parse_play_by_play` currently keeps only `goal` and `penalty` event families even though the official raw source contains `shot-on-goal`, `missed-shot`, and `blocked-shot`.
Consequence: Any scoring report built on the existing typed `PlayByPlay` would undercount attempts and could only explain goals, not chance generation.
Fix: Add a typed shot-event projection from raw `DataKind::PlayByPlay` before building scoring reports. Preserve the existing goal/penalty fields used by records.

### F-02 - WARN: Event-level xG is not currently sourced
File: `design/waves/2026-05-14-aim-the-rocket/SCORING-DATA-INVENTORY.md`
Finding: Official NHL play-by-play gives coordinates and shot context, but IceLines does not currently have an event-level xG model or allowed event-level xG source.
Consequence: A Rocket report that claims "xG" or "deserve-to-win" parity would overstate the data contract.
Fix: Ship Corsi/Fenwick/SOG/location/danger-proxy first. Defer xG until IceLines owns and tests a documented model or source.
