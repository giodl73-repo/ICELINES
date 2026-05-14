# R2 Review - edge

## Findings

### F-01 - WARN: Coordinates and participant IDs must remain optional
File: `design/waves/2026-05-14-aim-the-rocket/SCORING-DATA-INVENTORY.md`
Finding: The sampled official events include coordinates, shooter IDs, and goalie IDs, but source variation across seasons/game states can omit fields.
Consequence: Treating coordinates or participants as mandatory would make old, partial, or live-game payloads fail or silently coerce missing data to fake zeros.
Fix: Model `xCoord`, `yCoord`, shooter/scorer ID, blocker ID, goalie ID, and situation fields as `Option<T>` and add missing-field parser tests.

### F-02 - WARN: Rink orientation is an explicit edge case
File: `design/waves/2026-05-14-aim-the-rocket/SCORING-DATA-INVENTORY.md`
Finding: Play events expose `homeTeamDefendingSide`; coordinates need normalization before visual shot maps compare teams or periods.
Consequence: A rink plot can mirror one team's shots incorrectly and mislabel danger zones.
Fix: Keep raw coordinates in the first contract and add a separate tested normalization helper before rendering any rink visualization.
