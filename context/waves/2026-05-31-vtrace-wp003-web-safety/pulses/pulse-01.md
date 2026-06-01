# Pulse 01: Season-Type Mutation Boundary

## Pulse Control

| Field | Value |
|---|---|
| Wave | `2026-05-31-vtrace-wp003-web-safety` |
| Work package | `WP-003` |
| Status | closed_with_risk |
| Gate decision | passed_with_risk |
| Date | 2026-05-31 |

## VTRACE IDs

| Type | IDs |
|---|---|
| Parent requirements | `REQ-WEB-001`; `REQ-WEB-002`; `REQ-WB-002`; `REQ-PARITY-001`; `REQ-CODE-001` |
| Interfaces | `IF-WEB-001`; `IF-VIEW-001`; `IF-DATA-001` |
| Design / code rigor | `DES-007`; `DES-014`; `CR-014`; `CR-015`; `CR-027`; `CR-032` |
| Validation / evidence | `VAL-003`; `VAL-004`; `EVID-WP003-SEASON-TYPE-L0`; `EVID-WP003-SEASON-TYPE-L1`; `EVID-CR-006`; `EVID-CR-014`; `EVID-CR-018`; `EVID-CODE-001` |

## Objective

Close the observed browser safety gap where `GET /season-type/:kind` mutated the
server-side active season type. The Web route now uses a POST-backed mutation
path while preserving no-JS navigation affordances through forms.

## Affected Surfaces

| Surface | Expected Change |
|---|---|
| Web router | `/season-type/:kind` is registered as POST-only instead of GET. |
| Web shell template | Global season-type toggle renders POST forms/buttons rather than mutation links. |
| Web CSS | Toggle buttons visually match the existing navigation affordance. |
| Route inventory | Route matrix records `POST /season-type/:kind`. |
| Surface parity spec | GET mutation risk is documented as closed for this route; POST owns the mutation. |
| Route tests | GET returns method-not-allowed and preserves `active_season_type`; POST still flips state and redirects safely. |

## Allowed / Forbidden Scope

Allowed:

- Convert the observed season-type mutation route from GET to POST.
- Preserve no-JS behavior with HTML forms.
- Add route tests and route-inventory evidence for the changed boundary.
- Update VTRACE evidence and wave records tied to the pulse.

Forbidden:

- Changing season semantics, leader/query ranking semantics, or active context
  meaning.
- Claiming full `VAL-003` browser launch/no-JS/viewport/host inspection.
- Claiming full `WP-003` closure from this single route boundary.

## Validation Levels

| Level | Required Check | Status |
|---|---|---|
| L0 | Route tests prove POST mutation still works and GET is read-only/method-not-allowed. | passed |
| L1 | Formatting, route inventory tests, and affected Web clippy pass. | passed |
| L2 | Browser/no-JS/viewport inspection for all `VAL-003` routes. | pending_overall |

## Evidence Log

| Evidence | Result |
|---|---|
| Inventory | `GET /season-type/:kind` was the only observed route in this slice that changed `WebState.config.active_season_type` through a GET request. |
| L0 | `cargo test -p icelines-web --test l1_router season_type -- --nocapture` passed: 9 tests, including `l1_season_type_get_is_read_only_and_method_not_allowed`. |
| L1 | `cargo fmt --check` passed. `cargo test -p icelines-web --test ted_lindsay_route_inventory -- --nocapture` passed: 3 tests. `cargo clippy -p icelines-web --test l1_router --test ted_lindsay_route_inventory --no-deps -- -D warnings` passed. |
| Docs | Route inventory, surface parity spec, VTRACE docs, and this pulse record updated before mirror. |

## Gate

Gate is `passed_with_risk` for this pulse only. The selected season-type route no
longer mutates state through GET, the POST-backed no-JS control preserves the
existing user flow, and route tests prove GET rejection leaves active state
unchanged. Broader `WP-003`, `VAL-003`, host/bind launch behavior, narrow
viewport, JSON twins, and full browser/no-JS inspection remain open.
