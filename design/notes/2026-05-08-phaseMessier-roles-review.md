# Phase Messier — Roles review (2026-05-08)

Six role agents reviewed `phase-messier-overview.md` v1 + plan v1: FORGE, GLASS, EDGE, BENCH, WIRE, PACE. Severity: 7 BLOCKING / 16 HIGH / 14 MEDIUM / 8 LOW.

---

## BLOCKING (7) — must address before Messier.1 commit

| # | Issue | Role | Suggested resolution |
|---|-------|------|---------------------|
| 1 | `country_filter: Option<&'static str>` is a soundness lie — cmdbar receives borrowed user input, can't return `'static` without leaking | FORGE | `CountryCode([u8; 3])` newtype, `Copy`, fits in a register, free `Eq` |
| 2 | `Vec<(String, String)>` kv shape rots — duplicate keys silently last-write-wins, stringly-typed re-validation per consumer | FORGE | Typed `RosterKvArgs { sort, pos, country, min_gp, columns }` parsed at parse time; `ParseError::DuplicateKey` |
| 3 | kv parser uses `split_whitespace` — breaks on `:team EDM country="Czech Republic"` | EDGE | Reuse `command.rs` quoted-string lexer (already used for IN/LIKE) |
| 4 | `:stats country=CAN` (kv path) collides with Art Ross `country=CAN` atom (filter pipeline) — two paths, two semantics, AND-merge unspecified | EDGE | Stats-screen kv lowers to Art Ross atoms (writes `free_filter`), bit-identical with `c` shortcut. `RosterFilterState.country_filter` reserved for screens without overlay |
| 5 | "bit-for-bit parity" Messier.1 claim has no ground truth — `bool → Vec<ColumnId>` rename ≠ behaviorally identical | BENCH | `insta` ratatui buffer snapshot pre-refactor; replay post-refactor |
| 6 | `team EDM` positional + new kv form — disambiguation rule undefined; `team EDM season pos=LW` ambiguous | WIRE | Spec rule: tokens with `=` are kv, without are positional modifiers (`season`); unknown bare positional flashes error |
| 7 | `free_filter: Option<QueryPlan>` re-evaluated every ~10fps render frame → 7,000 plan-evals/sec/screen, hammers `fetch_game_lines` / `fetch_career_history` | PACE | Cache filtered `Vec<PlayerId>` keyed by `(repo_generation, plan_hash, pos_filter, country_filter, min_gp)`; rebuild only on input event or `repo_swap` |

---

## HIGH (16) — fold into spec v0.2

| # | Issue | Role | Resolution |
|---|-------|------|-----------|
| 8 | `PosFilter::G` collides with `GoalieRoleFilter` — invalid states representable | FORGE | Drop `G` from `PosFilter`; goalies own their axis exclusively. Or unify into `enum RosterFilter { Skater(PosFilter), Goalie(GoalieRoleFilter) }` |
| 9 | `forced_columns: Vec<ColumnId>` allows duplicates; `h` toggle = "remove if present, else push" drifts under `:team hits=on` | FORGE | `EnumSet<ColumnId>` (enumset crate) or bitflags newtype |
| 10 | `QueryPlan: Clone` unverified — derive `Clone` on `RosterFilterState` may break | FORGE | Confirm before Messier.1; if `!Clone`, wrap `Option<Arc<QueryPlan>>` |
| 11 | Chrome row overflow with 6+ keybinds at 80-col / 120-col terminals | GLASS | `KeyHint::priority` (Primary/Secondary), Primary inline + secondaries collapsed; or per-chip truncation budget (`country` → `cty`) |
| 12 | Chrome title unbounded growth — `Team · sort=Hits · pos=F · country=CAN · hits=on` is 49 chars *idle* | GLASS | Render only non-default chips; collapse free-form to `f=…`; assert title ≤60 chars at default state |
| 13 | `c` is not obvious — 5-second test: users guess column/clear/compare/career before country | GLASS | Rename to `n` (nationality, matches StatId catalog field), OR self-document chip (`c=CAN`) |
| 14 | Goalies "Starters/Backups" opaque mid-season — every goalie is a backup on Nov 1 | GLASS | GP-share threshold (≥60% of team's goalie minutes), AND surface threshold in chrome: `pos=Starters(GP≥27)` |
| 15 | `min-gp` hyphen ambiguity in Art Ross atoms — `gp` is the canonical stat key | EDGE | Normalize at kv layer (`min-gp` → `gp>=N` atom for Stats); document hyphens are layer-1 only |
| 16 | Repeated keys / ordering undefined — `:team EDM pos=LW pos=C` last-write-wins implicit | EDGE | (a) positional args precede all kv pairs (reject otherwise); (b) repeated keys = error (matches Art Ross "each atom appears once") |
| 17 | `free_filter` AND/OR semantic with structured filters unspecified | EDGE | Structured filters lower into synthetic `Constraint::Atom`s, wrapped with `free_filter` under one `Constraint::All`. Single eval pass |
| 18 | Test budget under-calibrated by 30-50% — Adams.10 needed 9 for Team alone; Messier.2 budgeted 10 for 5 keybinds + role-class + chrome | BENCH | Realistic budget: ~75-90, not 52 |
| 19 | Messier.6 L0/L1 split wrong — verb-kv requires App-level round-trip, 3 L1 covers ~1 verb | BENCH | 6 L0 (parsers) + 10 L1 (5 verbs × happy + error) |
| 20 | Goalies role-class threshold non-determinism — `season_length / 3` drifts as games are played | BENCH | Threshold from fixed `expected_season_games = 82`, OR freeze a `Clock` like Foster did |
| 21 | `SYSTEM_PROMPT_VERSION` bump mentioned but not specified | WIRE | Lock `"v2"` for v0.24.0 + L0 test asserting it; document grammar-change ⇒ version bump contract |
| 22 | `UNSUPPORTED` token contract drift after kv grammar expansion | WIRE | Add `"sort=gaa"`, `":goalies"`, `"min-gp=20"` to `l0_adams_system_prompt_has_grammar_landmarks` |
| 23 | Multi-pass filter chain allocates `Vec` per frame (Vec/frame × 10fps × 5 screens) | PACE | Single-pass `iter().filter(...).collect()` + memoize per (1) |
| 24 | Goalies role-class threshold recomputed every frame | PACE | Cache on season change; store as u32 in GoaliesState, invalidate on `repo_swap` |

---

## MEDIUM (14) — note in spec, address in implementation

25. `set_sort_by_label(v: &str)` is fallible-marked-infallible (`unwrap_or(...)` swallows garbage) — use `TryFrom<&str>` *(FORGE)*
26. AI prompt cache invalidation underspecified — verify cache chunk boundary above the version constant *(FORGE)*
27. `f` (free-form filter) only on Team violates "consistency" pillar — ship as thin overlay-wrapper across all player-list screens in Messier.3 *(GLASS)*
28. Filter-stack count row truncation at narrow widths — `(+N)` chip in dim style; `?` lists all active filters *(GLASS)*
29. 5-row MDI vertical pressure on 24-row terminals (~21% chrome) — collapse cyan + yellow into single 2-tone row when terminal height <30 *(GLASS)*
30. Heterogeneous kv value typing — `KvValue::parse(key, raw) -> Result<TypedValue, KvError>` per-key typing table *(EDGE)*
31. AI prompt grammar drift — prompt should prefer kv form for screen-targeted intent, Art Ross form for stat queries; 3-4 disambiguating few-shot examples *(EDGE)*
32. No render-level smoke per affected screen — `insta` goldens (in-process L0/L1, not L2) *(BENCH)*
33. Persona harness untouched — scenarios that exercise s/Team, s/Stats, s/Depth need updated assertions for standardized matrix *(BENCH)*
34. `forced_columns` invariant tests missing — idempotent toggle (`h h` → empty), no dedup, ordering contract *(BENCH)*
35. `country_filter` rejects unknown silently — flash `unknown country "ITA"` at parse time *(WIRE)*
36. `forced_columns: Vec<ColumnId>` future enum growth breaking under serde — document in-memory only; if serialized, switch to string discriminator *(WIRE)*
37. Saved queries persistence — `country=CAN` round-trip must produce identical IR; L1 test save→load *(WIRE)*
38. SYSTEM_PROMPT_VERSION cache discard timing — bump in single commit at Messier.6 (one cache miss, not 6) *(PACE)*

---

## LOW (8) — drive-by polish

39. `TeamPosFilter` deletion cascade — `pub use PosFilter as TeamPosFilter;` shim for one commit then remove *(FORGE)*
40. `min_gp: Option<u32>` (spec) vs `min_gp: u32` (Goalies) — pick one *(FORGE)*
41. `h` toggle column ambiguity (Hits on Team, Saves on Goalies) — surface resolved column name in chip: `h=Hits` / `h=Saves` *(GLASS)*
42. Color discoverability missing — active-non-default chips render cyan-bold; defaults dim-gray *(GLASS)*
43. Country cycle vs cmdbar accept different sets — cmdbar-set country renders as chip outside cycle, `c` resets to cycle position 0 *(EDGE)*
44. Cmdbar unknown-key error path not tested as contract — assert literal string for AI prompt versioning *(BENCH)*
45. Per-screen state migration is non-issue — document explicitly in Messier.1 commit message *(WIRE)*
46. No perf regression test — criterion-style L0: filter+sort cycle ≤ 1ms for N=700 *(PACE)*

---

## Recommended reshape for spec v0.2

1. **Add a §"Type modeling" section** locking decisions on `CountryCode`, `RosterKvArgs`, `EnumSet<ColumnId>`, single-pass + memoization.
2. **Add a §"Performance budget" section** declaring filter chain runs in state-update path, not render path; cache invalidated on input event or repo swap.
3. **Reshape §6.1 keybind matrix** — flag `c` rename to `n` as an open-for-decision item (BLOCKING-adjacent UX call); add chrome-priority and chip-truncation rules.
4. **Reshape §6.2 grammar** — quoted-string tokenization, kv positional ordering rule, repeated-key error, `min-gp → gp>=N` lowering.
5. **Reshape §"Test budget"** — 95 tests, not 52; insta goldens; persona harness deltas.
6. **Reshape §"Acceptance"** — add `SYSTEM_PROMPT_VERSION = "v2"`, perf bound for filter chain, parity-snapshot harness for Messier.1.

---

## Decision points for the user

A. **Locked decision review**: the spec locked Stats `s` = save (muscle memory). GLASS pushes back on `c` for country (similar muscle-memory issue — `c` reads as column/clear/compare). Should `c` rename to `n`?

B. **Locked decision review**: Goalies position semantic — Starters/Backups via GP threshold. GLASS + BENCH both flag mid-season fragility. Either freeze threshold to a constant (BENCH) or use GP-share-of-team-minutes (GLASS). Pick one.

C. **Type modeling depth**: FORGE's `CountryCode([u8;3])` newtype + `RosterKvArgs` typed parse argues for a more careful type-level pass before any code lands. That's ~2 hours of design work before Messier.1 starts. Worth it given the soundness lie?

D. **Test budget realism**: BENCH's call for 95 tests vs the spec's 52. Bigger tests catch more bugs but slow shipping. Pick.

E. **Performance memoization**: PACE's call for cached filter results keyed on `(repo_generation, plan_hash, pos_filter, country_filter, min_gp)` is a real engineering item. Implementing it adds ~half-day to Messier.1. Worth it given idle CPU spike risk?

Once decisions A–E land, I produce spec v0.2 + plan v0.2, and Messier.1 starts.
