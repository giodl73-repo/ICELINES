# R3 Review - wire

## Findings

### F-01 - WARN: Reuse the manifest cache, do not add a scoring-specific cache
File: `icelines-fetch/src/game_cache.rs`
Finding: Play-by-play artifacts already persist through `GameCacheArtifact::PlayByPlay` and `DataKind::PlayByPlay`.
Consequence: A second scoring cache would drift from records/streaks cache loading and reintroduce UI instructions to run separate commands.
Fix: Project scoring events from existing raw play-by-play entries and extend game-cache UI actions only when the requested artifact is missing.

### F-02 - WARN: No third-party scraping contract exists
File: `design/waves/2026-05-14-aim-the-rocket/WAVE.md`
Finding: The benchmark sites expose useful concepts, but their pages are not IceLines source contracts.
Consequence: Scraping them would create brittle data dependencies and possible policy/licensing risk.
Fix: Keep Rocket sources to official NHL API payloads, existing local bundles, and documented optional MoneyPuck season summaries.
