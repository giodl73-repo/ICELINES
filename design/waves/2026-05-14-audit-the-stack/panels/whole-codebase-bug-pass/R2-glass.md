# R2 Review - glass

## Findings

### F-01 - BLOCK: Watchlist players were not rendered as first-class player links
File: `icelines-web/templates/watchlist.html:45`
Finding: Watchlist teams rendered as `/team/:abbr` anchors, but Watchlist players rendered as plain text even when the player could be resolved.
Consequence: The Watchlist repeated the Favorites bug class: player entities looked like arbitrary search text while team entities received first-class navigation affordances.
Fix: Resolve Watchlist player rows to canonical names and `/player/:id` URLs before rendering. This pass implements the fix and adds an L1 route regression.

### F-02 - WARN: TUI Favorites chrome advertised an unaccepted command form
File: `icelines-cli/src/tui/screens/favorites.rs:78`
Finding: The Favorites screen footer advertised `:fav add`, but the command parser accepts `fav add` in command mode or `/fav add` from normal typing.
Consequence: The help chrome coached users toward a form that does not match the command bar vocabulary.
Fix: Advertise `fav add` as the command-mode form and keep a regression assertion that `:fav add` is not shown. This pass implements the fix.
