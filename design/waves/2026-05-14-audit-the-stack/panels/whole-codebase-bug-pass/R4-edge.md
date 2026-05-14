# R4 Review - edge

## Findings

### F-01 - WARN: Records provider can group malformed team ownership under a blank team
File: `icelines-fetch/src/records_provider.rs:127`
Finding: `team_abbrev_for_id` returns an empty string when an event has no owner team id or the id cannot be matched to home/away metadata.
Consequence: Event-backed records can accumulate under a blank team key instead of being excluded, marked unknown, or reported as malformed source data.
Fix: Return `Option<TeamAbbr>` or a typed unknown-team marker and make scorer/fight record builders decide explicitly whether to skip, label, or surface malformed events.

### F-02 - WARN: Ambiguous player identity resolution affects multiple surfaces
File: `icelines-fetch/src/stats_loader.rs:699`
Finding: `resolve_player_id_by_name` intentionally returns one pid, while `find_player_candidates` exists for all matches. Several CLI/TUI/web paths still use the single-pid helper for user-entered names.
Consequence: Short or ambiguous inputs can resolve differently than the user intended, especially for common surnames and duplicate NHL names.
Fix: Reserve `resolve_player_id_by_name` for exact or already-disambiguated paths. For interactive/user-entered input, use `find_player_candidates` and present or enforce disambiguation.
