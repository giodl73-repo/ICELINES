# R1 Review - keel

## Findings

### F-01 - WARN: Player web route mutates shared repository per request
File: `icelines-web/src/handlers/player.rs:41`
Finding: `/player/:id` takes a write lock on `WebState.repo` and fans out the requested player's full career into the process-wide repository for every player-page request.
Consequence: A read-only web request grows and mutates shared server state. Long-lived servers can accumulate more player-season windows than the active surface needs, and concurrent player-card traffic serializes on the write lock.
Fix: Move career fan-out behind a bounded player-career cache or build the player-card view from a request-local repository snapshot. Keep the shared active-season repo read-only during page rendering.

### F-02 - WARN: Web transactions collapses fallback load failure into an empty table
File: `icelines-web/src/handlers/transactions.rs:164`
Finding: `/transactions` maps `load_transactions_with_fallback` errors to `Err(())`, then `unwrap_or_default()` renders zero rows.
Consequence: A missing/corrupt transaction bundle looks identical to a valid season with no transactions, while the CLI fallback path can surface the data-source problem.
Fix: Preserve the load error and render an explicit unavailable/error state, or return a typed handler error that the route can display consistently with the CLI.
