# Yahoo Fantasy OAuth Sync

**Date**: 2026-08-25  
**Status**: Reviewed and amended; ready for implementation  
**Parent plan**: [`2026-07-18-fantasy-war-room-roadmap.md`](2026-07-18-fantasy-war-room-roadmap.md)  
**Authority**: Yahoo Fantasy Sports API for league context; official NHL data remains authoritative for hockey statistics
**Role review**: [`../../signals/roles/check/yahoo-fantasy-oauth-sync-roles-check-2026-08-25.md`](../../signals/roles/check/yahoo-fantasy-oauth-sync-roles-check-2026-08-25.md)

## Objective

Replace repeated third-party CSV exports and manual roster pastes with a secure,
read-only Yahoo Fantasy connection that keeps Felix's Five-Hole league context
current for draft and in-season IceLines decisions.

The first release discovers the signed-in user's hockey leagues and synchronizes
the selected league's settings, teams, rosters, platform player eligibility,
availability/ownership, draft results, transactions, standings, scoreboard, and
matchups when those fields are verified in the live API contract. It never adds,
drops, trades, drafts, or changes a lineup on Yahoo.

## Why This Is Worth Building

- Draft recommendations can mark taken players and current Yahoo eligibility
  without a fresh CSV export at every turn.
- Weekly pickup, matchup, trade, goalie, and readiness workflows can share one
  timestamped roster and league-state source all season.
- Yahoo league rules can be compared with the local Felix's Five-Hole contract so
  drift is visible instead of silently changing recommendations.
- The existing CSV path remains an offline/manual fallback rather than the only
  bridge to the platform.

## Non-Goals

- No Yahoo mutations in this plan: no draft pick, lineup edit, add/drop, waiver
  claim, trade, commissioner action, or pre-rank write.
- No scraping Yahoo HTML.
- No use of Yahoo statistics as canonical NHL performance data.
- No assumption that web-only fields such as XRank or ADP exist in the API.
  They enter the contract only after a captured, sanitized response proves it.
- No cloud account, IceLines server, shared credential service, or multi-user
  token store.
- No unattended background daemon in the first release. Sync is explicit; a
  bounded draft watcher is a later pulse after rate behavior is measured.

## External Contracts

- [Yahoo Fantasy Sports API guide](https://developer.yahoo.com/fantasysports/guide/)
  is the authority for supported hockey resources and provider request shapes.
- [Yahoo Fantasy Sports API access application](https://sports.yahoo.com/developer/access/)
  is the current gate for new applications. Access is reviewed separately from
  generic Yahoo Developer Network registration and is read-only today.
- [Yahoo OAuth authorization-code flow](https://developer.yahoo.com/oauth2/guide/flows_authcode/)
  is the authority for authorization, access-token lifetime, refresh, and token
  rotation behavior.
- [Yahoo OAuth troubleshooting](https://developer.yahoo.com/oauth2/guide/troubleshooting/)
  recommends the Installed Application type for a standalone app. Pulse 0 must
  verify current behavior rather than treating documentation examples as a live
  payload contract.

## Source and Trust Contract

| Data | Authority | IceLines use |
|---|---|---|
| NHL identity, team, statistics, schedule | Official cached NHL sources | Scoring, projections, player identity, game dates |
| Yahoo league key, settings, teams, rosters | Yahoo Fantasy API | Platform/league context |
| Yahoo eligible positions | Yahoo Fantasy API | League-specific slot eligibility; never overwrites canonical NHL position |
| Yahoo ownership/status | Yahoo Fantasy API | Availability and readiness as of `fetched_at` |
| Yahoo draft/transaction/matchup records | Yahoo Fantasy API | Local league event context and reconciliation |
| CSV/clipboard import | User-labeled manual source | Offline fallback with its own timestamp |

Every provider-derived record carries the Yahoo game/league key, provider object
key where available, source endpoint family, `fetched_at`, and sync-run ID. A
Yahoo field that is absent is unknown, never zero or false.

## Architecture

```text
Yahoo authorization page
        |
        v
icelines-cli: browser handoff + loopback/OOB completion + user-facing errors
        |
        v
icelines-fetch::yahoo_fantasy
  OAuth token exchange/refresh, HTTP acquisition, pagination, retry policy
        |
        +--> OS credential vault
        |      client secret + refresh token; never SQLite/config/log/JSON
        |
        v
icelines-sources::yahoo_fantasy
  deterministic provider DTO parsing + source-neutral normalization
        |
        v
FantasyDb staging transaction
  provider keys, league context, mappings, sync runs, freshness
        |
        v
existing icelines-core fantasy ViewModels and optimizers
  no renderer-local Yahoo logic
```

### Crate boundaries

- `icelines-sources`: deterministic parsing and normalization over caller-
  supplied bytes. No network, OAuth, filesystem, SQLite, or secrets.
- `icelines-fetch`: Yahoo HTTP client, OAuth protocol, rate handling, acquisition
  orchestration, credential-vault trait, and FantasyDb synchronization.
- `icelines-core`: provider-neutral sync/readiness ViewModels, reconciliation
  rules, and source-authority/freshness contracts.
- `icelines-cli`: thin commands, browser launch/callback experience, terminal
  rendering, and JSON projection.
- `icelines-web`: read-only connection/sync status after the shared contract is
  stable. OAuth start/callback is deferred unless it can preserve local-only
  bind, host validation, CSRF state, and explicit mutation semantics.

Yahoo is an explicit, user-invoked provider-ingestion path into FantasyDb. It
is not added to the NHL snapshot fallback chain, and ordinary fantasy reads
never fall through to a Yahoo network call. OAuth/acquisition DTOs remain
Send-clean; no OAuth task captures or transports `StatsRepository`, and
FantasyDb is opened/applied only after acquisition completes.

## Identity and State Model

Yahoo player keys are durable provider identities and must not be discarded in
favor of normalized names. Connection identity is separate from league identity,
because one Yahoo account can expose multiple leagues and every season can use a
new game key. The provider join records:

```text
(provider=yahoo, yahoo_game_key, yahoo_player_key)
    -> canonical PlayerId | unresolved | ambiguous
```

Resolution order is exact saved provider mapping, verified canonical external
ID if Yahoo exposes one, exact normalized name plus corroborating NHL team, then
an unresolved review proposal. A name-only collision, stale team, or duplicate
candidate never auto-resolves. Manual resolutions persist the chosen canonical
ID, evidence, resolver, and timestamp and survive later team changes. Provider
position lists live in FantasyDb league context, never enter `StatsRepository`,
and never mutate `SeasonStats.position`.

Proposed additive FantasyDb tables:

- `fl_provider_connections`: local connection UUID, provider, opaque account
  handle, status, scopes, created/updated timestamps, active credential
  generation, and last successful sync; unique on `(provider, account_handle)`;
  no secrets.
- `fl_provider_leagues`: Yahoo game/league keys and remote metadata linked to a
  local league; unique on `(connection_id, yahoo_game_key, yahoo_league_key)`
  and separately unique on the active `(connection_id, local_league_id)` link.
- `fl_provider_player_map`: Yahoo player key, canonical player ID when resolved,
  normalized display/team evidence, resolution status, resolver/evidence, and
  timestamps; unique on `(connection_id, yahoo_game_key, yahoo_player_key)`.
- `fl_provider_player_context`: league/player key, eligible positions,
  ownership/availability/status fields, source/freshness; unique on
  `(connection_id, yahoo_league_key, yahoo_player_key)`.
- `fl_provider_sync_runs`: scope, start/end, success/failure, counts, endpoint
  coverage, endpoint-family timestamps/watermarks, warnings, and a redacted
  error summary.
- `fl_provider_events`: stable provider event key plus normalized draft or
  transaction payload where endpoint contracts support it; unique on
  `(connection_id, yahoo_league_key, event_kind, provider_event_key)`.

Existing `fl_teams`, `fl_roster`, `fl_player_eligibility`, competition rules,
matchups, and observations remain the engine-facing state. A successful sync
reconciles provider staging into those tables in one SQLite transaction. The
field-ownership matrix is fixed: Yahoo owns only its provider mirrors and
provider-sourced roster/eligibility rows; user assistant policy and confirmed
local scoring remain user-owned; remote settings are compare-only until the user
explicitly accepts a drift update. Yahoo status remains platform context and
does not replace sourced IceLines medical/goalie/role observations.

## Security Contract

1. Use OAuth 2 authorization-code flow with an unpredictable state value.
2. Register IceLines as a native public client using authorization code + PKCE,
   then obtain separately reviewed, read-only Fantasy Sports API access. Generic
   YDN registration alone does not authorize the Fantasy API.
3. Prefer a loopback callback on `127.0.0.1` with an ephemeral port if Yahoo's
   registered-app contract accepts it; otherwise use Yahoo's documented OOB
   completion. The contract probe decides this before implementation.
4. Bind callback only to loopback, validate state exactly once, set a short
   timeout, and shut the listener down after success or failure.
5. Store the refresh token in the operating-system credential
   vault behind a `CredentialVault` trait. Windows Credential Manager is the
   first production adapter; other platforms require an audited native adapter
   before being marked supported. Stable entries are namespaced by
   `icelines/yahoo/<connection_uuid>/<credential_generation>/<kind>`. An
   unavailable vault is a hard, actionable error—there is no plaintext config,
   SQLite, file, or silent fallback. The public Client ID is non-secret
   registration metadata; no client secret is expected for the PKCE flow.
   Environment injection is permitted only for explicit automation/test mode
   and credential values never appear in diagnostics.
6. Treat the newest refresh token as authoritative because Yahoo may rotate it.
   Rotation uses two generations: write and read-verify the inactive generation,
   switch non-secret active-generation metadata atomically, validate the next
   access-token use, then retire the previous generation. Failure before the
   switch keeps the old token active; failure after the switch keeps both until
   cleanup. Every boundary receives a failure-injection test.
7. Redact authorization codes, access tokens, refresh tokens, client secrets,
   bearer headers, and provider payload fragments that could contain them.
8. `disconnect` deletes only the named Yahoo credential entries and local
   connection metadata after explicit confirmation. League data remains unless
   `--purge-synced-context` is separately confirmed.
9. Print the authorization URL before attempting browser launch so headless,
   WSL, SSH, and browser-policy failures remain recoverable. The callback accepts
   only loopback destination, the expected path, and the exact one-use state; it
   renders minimal accessible success/denial/error HTML and shuts down on every
   terminal path, including timeout and repeated callbacks.

## Reliability and Sync Semantics

- Manual sync is the v1 default. No network request occurs during ordinary
  fantasy reads unless the user requested sync.
- Every request has a timeout and bounded retry policy. Honor `Retry-After` for
  HTTP 429; use capped exponential backoff with jitter for retryable 429/5xx
  responses; do not retry ordinary 4xx responses.
- On 401, refresh once, atomically store a rotated refresh token, and retry the
  original request once. A second 401 marks the connection `reauth_required`.
- Paginated collections must prove termination, detect repeated pages/cursors,
  deduplicate provider keys, and enforce a defensive page ceiling.
- Every sync scope has a versioned required/optional capability matrix. The
  measured Pulse 0 contract supplies timeout, retry count, backoff cap, page
  ceiling, and freshness defaults; until then those values are explicitly TBD,
  not guessed constants.
- Fetch all required pages into a staged sync run. Required-endpoint failure
  aborts application and preserves the last successful local state. Optional
  endpoint failure commits only if the run explicitly records the missing
  capability and no engine-facing field is fabricated.
- Draft/roster scopes perform start/end sentinel re-reads or compare a verified
  stable event watermark. If remote state changed during pagination, retry
  boundedly; after exhaustion retain the old complete state and record the run
  as inconsistent/partial. Absence from an inconsistent scope never deletes a
  player, roster row, or event.
- Apply reconciliation in one SQLite transaction. Deletion is by complete,
  verified remote scope, never by absence from a partial page.
- Expose `last_attempt_at`, `last_success_at`, requested/covered capabilities,
  warnings, and whether the current state is fresh, stale, partial, or requires
  reauthorization.
- Keep the CSV importer operational and label API vs. CSV provenance clearly.
- Do not persist raw Yahoo responses by default. Store normalized typed state and
  redacted diagnostics only. Any future diagnostic capture must be explicit,
  short-lived, permission-restricted, and separately reviewed.
- Use a non-exhaustive typed error taxonomy separating OAuth denial/expiry,
  reauthorization, HTTP status/rate/timeout, pagination inconsistency, schema
  drift, identity ambiguity, vault failure, database rollback, and capability
  absence; every variant carries retryability and a recovery action.

## Command Surface

```powershell
icelines fantasy yahoo connect
icelines fantasy yahoo status
icelines fantasy yahoo leagues
icelines fantasy yahoo select --league-key <key> --local-league "Felix's Five-Hole 2026-27"
icelines fantasy yahoo sync --scope league
icelines fantasy yahoo sync --scope draft
icelines fantasy yahoo sync --scope matchup
icelines fantasy yahoo disconnect
```

All read and sync commands support `--json`. `connect`, `select`, `sync`, and
`disconnect` are explicit mutations and never occur through a GET route. Text
and JSON consume one shared ViewModel. JSON never contains credentials or raw
authorization responses.

Connection states are explicit: `disconnected`, `connected`,
`reauth_required`, `remote_missing`, and `relink_required`. Annual Yahoo game-
key rollover creates a new remote-league link; it never mutates the prior
season's provider keys in place.

## Delivery Plan

### Pulse 0 — Access gate, contract probe, and fixtures

- Create a Yahoo public-client registration and submit the separate read-only
  Fantasy Sports API access application. Generic App ID/Client ID issuance is
  not evidence that Fantasy API access has been granted.
- Verify authorization-code behavior, callback constraints, scope display,
  token rotation, JSON/XML response formats, pagination, and documented errors.
- Probe only the signed-in user's own league using read-only calls.
- Capture minimal sanitized fixtures for user games/leagues, league settings,
  teams, rosters, players, and each optional draft/transaction/scoreboard
  endpoint that actually exists.
- Build a deterministic sanitizer and fixture manifest recording endpoint
  family, capture date, source hash, sanitized hash, removed-field classes, and
  reviewer status. No manually edited real payload becomes a test fixture.
- Publish an endpoint capability matrix. Unsupported or undocumented fields
  remain blocked rather than inferred.
- Measure requests/pages per scope, p50/p95 latency, observed 429 behavior, and
  token/callback timing. These measurements—not intuition—set versioned retry,
  timeout, page-ceiling, watcher, and per-workflow freshness policies.

**Gate**: no production parser is written against an imagined payload; every v1
field exists in an audited fixture with secrets and private identifiers removed.

### Pulse 1 — Provider types and client boundary

- Add provider DTO parsing/normalization in `icelines-sources`.
- Add `YahooFantasyClient` and typed `YahooFantasyError` in `icelines-fetch`.
- Inject base URLs, clock, sleeper/backoff, and credential store for tests.
- Implement authorization URL construction, token exchange, refresh rotation,
  bearer requests, pagination, retry, and redaction.
- Use typed contract-bearing DTOs and a typed schema-drift error. Tolerated
  extension points must be named deliberately for noisy provider envelopes;
  production logic does not traverse ad hoc `serde_json::Value` trees.

**Gate**: L0 parser fixtures and L1 mocked HTTP tests cover success, malformed
payload, 401 refresh, rotated token, 403, 429 `Retry-After`, 5xx exhaustion,
timeout, and repeated pagination cursor/page.

### Pulse 2 — Secure local connection

- Add an operating-system credential-vault adapter and an in-memory test vault.
- Implement `connect`, `status`, and `disconnect` with loopback/OOB behavior
  selected by Pulse 0.
- Persist non-secret connection metadata and connection state.
- Ensure Ctrl-C, browser denial, callback timeout, port collision, and revoked
  access leave no half-connected record.

**Gate**: no token is present in config, SQLite, stdout/stderr, JSON, snapshots,
or test goldens; reconnect and two-generation refresh-token replacement are
recoverable. Windows Credential Manager passes create/read/rotate/delete and
redaction smoke checks; an unavailable vault proves hard failure without a
plaintext fallback.

### Pulse 3 — League discovery and atomic synchronization

- Implement `leagues`, `select`, and `sync --scope league`.
- Add additive FantasyDb migrations and provider-key identity joins.
- Stage and atomically reconcile settings, teams, rosters, player context, and
  eligibility into existing engine-facing tables.
- Generate a drift report before changing locally customized rules. Provider
  values do not silently overwrite user-confirmed scoring or assistant policy.
- Persist manual identity-resolution evidence and endpoint-family timestamps.
- Apply the versioned scope capability matrix and remote-consistency sentinel
  before permitting complete-scope deletion.

**Gate**: an interrupted/partial sync preserves the prior snapshot; repeated
sync is idempotent; removed roster players disappear only after complete-scope
proof; existing databases round-trip and foreign keys remain valid. Failpoints
after every required page, staging write, reconciliation step, and pre-commit
boundary prove the prior engine-facing state is byte-equivalent after rollback.

### Pulse 4 — Draft integration

- Implement the verified draft-result and available-player capabilities.
- Feed provider ownership/taken state and eligibility into the existing draft
  session/board without changing projection authority.
- Add `sync --scope draft`; measure request count and latency before considering
  a watcher.
- If a bounded watcher is later enabled, default to a conservative interval,
  serialize requests, stop on repeated rate limits, and show freshness visibly.
- Before watch mode can ship, publish requests per cycle, pages per collection,
  p50/p95 latency, observed 429s, and explicit stop/backoff thresholds.

**Gate**: API state and an equivalent CSV/taken-list fixture yield the same
available pool; an unresolved Yahoo player cannot leak into the available pool
as falsely free or be silently marked taken.

### Pulse 5 — In-season league operations

- Synchronize verified transactions, matchup/scoreboard context, standings,
  and roster changes with stable event IDs and idempotent reconciliation.
- Feed freshness into morning, matchup, pickup, trade, goalie, and readiness
  ViewModels.
- Preserve manual observations when Yahoo lacks equivalent evidence; source
  precedence is field-specific and disclosed.

**Gate**: repeated event sync creates no duplicates; changed roster ownership is
atomic; stale/partial/reauth states lower readiness and name a recovery command.

### Pulse 6 — Surface parity and operations

- Add TUI and Web read-only connection/freshness views after CLI/JSON contracts
  stabilize.
- Keep OAuth and sync mutations CLI-only for this plan. Web displays read-only
  status and the exact CLI recovery/connect command; a Web OAuth/POST flow is a
  separate security review.
- Document Yahoo app creation, reconnect/revoke, credential cleanup, offline CSV
  fallback, troubleshooting, and an optional user-scheduled sync command.
- Run full regression, secret scanning, redaction tests, and release packaging
  checks on Windows plus at least one non-Windows CI target.

**Gate**: CLI/TUI/Web show the same provider state and timestamps; narrow/no-
color terminal output remains actionable; a cold/offline run uses the last good
state without claiming it is current.

## Test Matrix

| Tier | Required proof |
|---|---|
| L0 | DTO parsing, unknown/null fields, provider-key identity, name collision, eligibility, state validation, redaction, freshness, deterministic reconciliation plan |
| L1 | Mock OAuth exchange/refresh, callback state/host/path/one-use behavior, timeout, pagination, remote-change sentinel, retries, 401/403/429/5xx, partial endpoint, atomic SQLite apply/rollback, migration preservation, two-generation vault failures |
| L2 | CLI status before/after connect, league select, idempotent sync, JSON parity, redacted failures, reauth recovery, disconnect confirmation, CSV fallback |
| Manual | Yahoo consent wording, installed-app callback behavior, browser-open failure/headless copy flow, Windows vault CRUD/rotation, real hockey league discovery, sanitizer/fixture audit, revoked access, draft-scale latency/rate observation |

CI never calls Yahoo or uses a real credential. Mock base URLs and in-memory
vaults are test-only injection points, not production flags exposed casually.

## Observability and Readiness

`fantasy yahoo status` and `fantasy readiness` must expose:

- connected / disconnected / reauthorization-required;
- selected remote and local league without exposing private account IDs;
- last attempt and last successful sync;
- capability coverage and warnings;
- player-map resolved/ambiguous/unresolved counts;
- endpoint-family ages plus versioned freshness classification per workflow;
- exact recovery command.

Logs include sync run IDs, endpoint family, HTTP status, retry count, page count,
record count, and elapsed time. They never include query/body/header values that
could disclose credentials or private roster payloads.

One provider-neutral `FantasyProviderStatusView` in core owns connection state,
scope coverage, timestamps, freshness, mapping counts, and recovery actions.
CLI, TUI, Web, and readiness render it without deriving their own policy.

## Build-Green Commit Ledger

Each commit is independently compilable and testable:

1. Add source-neutral provider contracts, typed errors, sanitized fixtures, and
   pure parser tests; no commands or persistence consumers.
2. Add `YahooFantasyClient`, injected clock/backoff/base URLs, and mocked HTTP
   tests; no production credential adapter.
3. Add `CredentialVault`, in-memory adapter, two-generation rotation state
   machine, redaction, and failure tests.
4. Add the Windows Credential Manager adapter plus manual smoke harness; an
   unsupported/unavailable vault fails closed.
5. Add additive FantasyDb migrations, keys, staging/reconciliation plan, and
   migration/rollback tests; no CLI invokes sync yet.
6. Add `connect/status/disconnect` CLI and JSON through shared core ViewModels.
7. Add league discovery/select and complete-scope synchronization with the
   versioned capability matrix and consistency sentinel.
8. Integrate roster/eligibility context into existing fantasy consumers and
   readiness while preserving CSV parity.
9. Add verified draft capabilities and measured manual draft sync; watch mode
   remains a distinct later commit only if its measurement gate passes.
10. Add verified in-season events/matchup capabilities, then read-only TUI/Web
    status surfaces and documentation.

## Rollout and Rollback

- Ship behind explicit `fantasy yahoo` commands; existing fantasy reads do not
  auto-sync.
- Migrations are additive. A binary that does not use Yahoo continues to work.
- The last successful provider snapshot remains usable after network/auth/API
  failure and is visibly stale.
- `disconnect` revokes local access by deleting vault credentials and connection
  metadata; provider data can be retained as stale audit context or separately
  purged with explicit scope.
- CSV import remains the recovery path throughout rollout.

## User Checkpoint

**Status (2026-08-25): submitted; Yahoo review pending.** The user created the
public-client registration and submitted Yahoo's separate Fantasy Sports API
access application for the personal, read-only IceLines use case. Live contract
probing remains gated on approval. The Client ID, authorization code, access
token, and refresh token must be entered only into the local IceLines connection
flow or OS credential vault—never chat, source, plan files, issue text,
screenshots, or fixtures. The current GitHub redirect must not be used for live
authorization; Pulse 0 will replace it with the verified PKCE loopback/OOB
callback contract before consent.

## Exit Criteria

- One explicit authorization supports season-long refresh without routine CSV
  exports until Yahoo revokes access or account security invalidates tokens.
- Felix's Five-Hole league settings, teams, rosters, eligibility, and verified
  capabilities synchronize atomically with source timestamps.
- Draft and in-season fantasy workflows consume synchronized provider context
  through existing shared ViewModels.
- NHL sources remain authoritative for hockey stats and schedule.
- No production or test artifact leaks a Yahoo credential.
- Network, schema, pagination, rate, partial-sync, identity, and revocation
  failures are tested and actionable.
- No Yahoo mutation is possible through the shipped client surface.
