# R3 Review - wire

## Findings

### F-01 - WARN: Favorites context turns a bad active season into `Season(0)`
File: `icelines-web/src/handlers/favorites.rs:126`
Finding: `favorites_context` parses `cfg.active_season` and silently falls back to `Season(0)` on parse failure.
Consequence: HTML Favorites and Watchlist can build `ViewContext` values that do not correspond to a real season, making downstream labels and windows unreliable while still returning a success-shaped page.
Fix: Match the newer awards/records/streaks handlers: surface a typed error response for invalid config, or fall back to `CURRENT_SEASON` with an explicit visible warning.

### F-02 - WARN: Web favorite/watchlist links depend on first substring player match
File: `icelines-web/src/handlers/favorites.rs:165`
Finding: `resolve_favorite_player` calls `resolve_player_id_by_name`, which returns the first substring match, then looks that pid up in `find_player_candidates`.
Consequence: Ambiguous stored names such as common surnames can link to the wrong player while still producing a confident `/player/:id` URL.
Fix: Store canonical `player:<pid>` entity refs at write time where possible, and for legacy name keys require exact normalized-name matches before falling back to unresolved display text.
