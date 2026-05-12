# Prince visual-system roles review

**Date**: 2026-05-12
**Scope**: `design/specs/visual-system.md` and Prince of Wales phase plan

## Summary

The baseline is directionally correct: Prince should make IceLines visually
composed without weakening ViewModel truth. The review found one blocking
issue and several high-value hardening items before implementation starts.

## Findings

| Role | Severity | Finding | Resolution |
|---|---|---|---|
| PACE | Blocking | ASPECT scores were presented as numbers without a scoring method, evidence source, or review cadence. | Add a baseline evidence ledger and require score changes to name evidence. |
| BENCH | High | Exit gates named snapshots/screenshots but did not define capture dimensions, commands, or deterministic fixtures. | Add a review artifact matrix with TUI, web, CLI, and markdown capture requirements. |
| FORGE | High | Semantic tokens were documented but had no ownership boundary; renderer-local color tables could still drift. | Add token source-of-truth policy and staged implementation path. |
| KEEL | High | Cross-surface visual convergence was described generally; no mapping table required TUI/CLI/web/markdown to share token names. | Add renderer mapping requirements and convergence checks. |
| GLASS | High | ASCII fallback was required generally, but token cues still used glyph-first examples such as star/up/down. | Add explicit ASCII fallback column and non-color rule. |
| CREST | High | Aesthetic direction was clear but lacked palette, density, and screenshot-quality constraints strong enough to reject default-looking UI. | Add aesthetic constraints and screenshot review questions. |
| broadcast | High | Web targets mentioned mobile and screenshots but lacked viewport, focus, touch target, and partial-fragment requirements. | Add web artifact and accessibility gates. |
| EDGE | Medium | Narrow, stale, partial, missing source, no-data, and high-density failure modes were named but not enumerated per artifact. | Add edge-state coverage requirements to the review artifact matrix. |
| HART/TAPE | Medium | Active season/type/source truth was required, but the visual spec should explicitly tie these to `ViewContext` and `SourceState`. | Add ViewModel context/state as the source for context chips. |
| SCOUT | Medium | Hockey-native was stated, but the spec did not define the visual hierarchy for game/team/player/role context. | Add hockey hierarchy primitives for player, team, game, role, and fantasy decision views. |

## Decisions

- Prince.1 remains a design/spec slice; no renderer implementation begins until
  the baseline has artifact requirements.
- Prince.2 must create the first token mapping before broad visual work.
- Prince.3 and Prince.4 must capture before/after artifacts, not only patch CSS
  or ratatui styles.

