# Pulse 02 - Shared Workbench Catalog, Fields, and Pane Models

## Goal

Add the typed foundation for the Call the Changes workbench. The catalog should
describe stable workbench IDs, labels, groups, aliases, zone defaults, pane
model capabilities, shared fields, and optional bound experience tabs. TUI and
web adapters may map those IDs to surface-specific targets, but they must not
invent independent screen lists or pane-only field vocabularies.

## Governing roles

- **keel**: one catalog identity and one field vocabulary must map to TUI
  screens, web workspaces, command aliases, pane models, and bound experiences.
- **glass**: labels, grouping, zones, pane kinds, and field summaries must be
  readable enough for a user to choose a workspace or pane without memorizing
  command verbs.
- **forge**: keep shared catalog/field/pane types pure and small. Concrete TUI
  `Screen` and web route lowering belong in their owning crates/adapters.
- **wire**: catalog targets and pane fields are navigation/read filters unless
  explicitly delegated to existing mutation intents. Action/status panes may
  show mutation results, but GET navigation must not mutate state.
- **bench**: add L0 tests for uniqueness, coverage, field sources, pane model
  zone compatibility, and ViewModel-backed field metadata.

## Owned scope

1. Add shared workbench catalog types and static entries.
2. Add shared `WorkbenchField` or equivalent field metadata for reusable pane
   inputs: entity IDs, route/workspace IDs, stat keys, dates, source states,
   summary values, comparison baselines, and command/mutation result fields.
3. Add shared pane model metadata for navigators, inspectors, filters/dimensions,
   summaries/KPIs, timelines/activity, comparisons, queues/checklists,
   source/data-state panes, action/status panes, and help/docs panes.
4. Add shared bound experience metadata for optional tabs that compose center
   workspace, left/right pane model bindings, ribbon scope, and active fields.
5. Add TUI adapter mapping catalog IDs to resolvable `Screen` targets where a
   no-argument target exists.
6. Add web adapter mapping catalog IDs to safe canonical dashboard workspace
   routes where a no-argument route exists.
7. Fence aliases, field IDs, pane models, bound experiences, source ownership,
   and zone compatibility with tests.
8. Do not change visible TUI/web layout yet.

## Non-goals

- No TUI activity rail rendering.
- No web dashboard template changes.
- No command-bar parser rewrite beyond optional use of shared IDs in tests.
- No new ViewModels or analytics.
- No pane-local hockey math.

## Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test -p icelines-core --quiet`
- [ ] `cargo test -p icelines-cli --quiet`
- [ ] `cargo test -p icelines-web --quiet`
- [ ] `cargo clippy -- -D warnings`
