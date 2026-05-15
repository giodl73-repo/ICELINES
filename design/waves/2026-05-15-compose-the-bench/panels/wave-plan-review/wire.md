# WIRE review - Compose the Bench plan

## Findings

- Pane composition is navigation/read state. It must not create GET-backed
  favorite, watch, admin, cache-load, or data-install operations.
- Web URL state must be allowlisted and canonicalized the same way workspace
  routes are today. Unsafe pane IDs should reject or normalize with explicit
  user-visible behavior.
- Source/data-state panes can show stale/missing/loaded coverage, but they must
  not silently fetch live data while rendering.

## Required checks

- Pulse 04 needs tests for unsafe pane/experience query parameters.
- Docs must explicitly preserve POST-backed mutation boundaries.
