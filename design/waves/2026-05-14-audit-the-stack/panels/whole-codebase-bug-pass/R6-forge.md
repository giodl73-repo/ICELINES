# R6 Review - forge

## Findings

### F-01 - WARN: TUI Favorites renderer swallows storage/view construction errors
File: `icelines-cli/src/tui/screens/favorites.rs:93`
Finding: The Favorites screen converts `GroupDb::open()` failures to an empty member list and converts `compute_favorites_view` errors to `None`.
Consequence: A database or view-construction failure can render as "no favorites" or a plain member fallback instead of a user-visible error, making local data corruption hard to diagnose.
Fix: Preserve the error and surface it through the screen status/chrome flash while keeping the non-failing fallback for genuinely empty groups.
