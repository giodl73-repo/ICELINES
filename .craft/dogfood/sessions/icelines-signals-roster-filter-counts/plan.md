---
wave: fire-the-kiln
pulse: icelines-signals-roster-filter-counts
date: 2026-06-22
status: planned
req:
  - REQ-DOGFOOD-001
wp:
  - WP-FIRE-DOGFOOD-WORK-SESSION-DESCRIPTOR-D1
dcr:
  - DCR-DOGFOOD-001
governing_roles:
  - rust-errors
  - rust-type-system
  - compiler-execution
  - agent-provider
  - agent-secure
  - craft-architect
  - craft-observability
  - craft-security
  - craft-testing
  - craft-dragon
depends_on:
  []
---

# Craft Chat Pulse icelines-signals-roster-filter-counts

## Mission

WP-010 Signals: make signals-roster --evidence filters auditable by reporting matched/total/filtered row counts without introducing ranking, cache, StatId, or prediction claims.

## Lane

rust

## Deliverables

- [ ] task-001: Complete the pulse intent through the selected lane.
- [ ] Record chat status and gate evidence before closeout.
- [ ] Preserve D1 non-claims and avoid unsupported capability promotion.

## Gate Results

- [x] Craft chat pulse plan is descriptor-ready.
- [x] D1 non-claims remain explicit in the Non-goals section.
- [x] `craft chat` is the operator cockpit for this session.

## Post-session gate targets

- `craft chat next --workspace-root <ROOT> --limit 3`
- `craft chat status --workspace-root <ROOT> --limit 3`
- `craft chat continue --workspace-root <ROOT> --limit 3`
- `craft chat summary --workspace-root <ROOT> --output text`
- `craft chat health --workspace-root <ROOT> --output text`
- `craft dogfood pulse gate --workspace-root <ROOT> --session icelines-signals-roster-filter-counts`
- `git diff --check`

## Non-goals

No provider or network call, no arbitrary shell wrapper, no autonomous coding
claim, no git commit or PR creation by CRAFT, no full self-hosting claim, and
no `REQ-DOGFOOD-001` verification promotion.
