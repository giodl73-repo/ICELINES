# R5 Review - bench

## Findings

### F-01 - BLOCK: Watchlist lacked a player-link parity regression
File: `icelines-web/tests/l1_router.rs:1763`
Finding: The Watchlist route test covered notes and alert metadata but did not assert that a watched player renders as a canonical player link.
Consequence: The same player/team affordance mismatch that affected Favorites could recur in Watchlist without a failing test.
Fix: Seed a known bundled player and assert both the canonical display name and `/player/:id` anchor. This pass adds the regression.

### F-02 - WARN: Residual data-fallback findings need targeted tests before fixes
File: `icelines-web/src/handlers/favorites.rs:126`; `icelines-web/src/handlers/transactions.rs:164`; `icelines-fetch/src/records_provider.rs:127`
Finding: The audit found bad-season fallback, transaction-load fallback, and unknown-team fallback paths that are not pinned by focused tests.
Consequence: Fixing the behavior without tests risks replacing one success-shaped silent fallback with another.
Fix: Add L1 web tests for invalid active-season/transaction load failures and L0 fetch tests for malformed play-by-play team ownership before changing those paths.
