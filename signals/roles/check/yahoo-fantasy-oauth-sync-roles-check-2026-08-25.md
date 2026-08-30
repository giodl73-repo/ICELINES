---
skill: roles-check
topic: yahoo-fantasy-oauth-sync
date: 2026-08-25
roles_used: 9
initial_p1_count: 7
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Yahoo Fantasy OAuth Sync — Roles Check

## Artifact identification

- **Artifact**: `design/plans/2026-08-25-yahoo-fantasy-oauth-sync.md`
- **Type**: architecture and implementation plan
- **Domain signals**: OAuth/security, external API acquisition, provider identity,
  SQLite reconciliation, fantasy source authority, CLI/browser UX, polling and
  freshness

## Role selection

| Role | Why selected |
|---|---|
| HART | Provider keys, league axes, and separation from canonical player/season state |
| KEEL | New source, persistence path, CLI/TUI/Web convergence, and build-green delivery |
| TAPE | Yahoo-to-NHL identity resolution and field-level source authority |
| FORGE | Rust crate boundaries, typed errors, async callback flow, and credential adapter |
| PACE | Polling, retry, freshness, and latency numbers must be measured assumptions |
| BENCH | Mocked OAuth/API tests, migration rollback, vault failures, and fixture discipline |
| EDGE | Token rotation, pagination races, revocation, collisions, and multi-account cases |
| WIRE | External schema, retry, pagination, partial response, and degradation policy |
| broadcast | Browser authorization handoff, loopback callback, headless behavior, and safe web deferral |

SCOUT, GLASS, and CREST were not selected. This artifact does not change hockey
methodology or design a rendered decision surface; their review becomes relevant
when synchronized data changes recommendations or a concrete TUI/Web screen is
proposed.

## HART review

| # | Finding | Severity | Section | Recommendation |
|---:|---|---|---|---|
| H1 | Proposed tables name key fields but do not lock the complete uniqueness axes for account, Yahoo game/season, remote league, local league, player, and provider. An under-keyed row can leak one season or league into another. | P1 | Identity and State Model | Specify primary/unique keys including connection/account ID, Yahoo game key, Yahoo league key, and player key; require migration tests for two seasons and two leagues. |
| H2 | Reconciliation into existing eligibility, roster, settings, and observation tables needs a field-level ownership rule. “Provider staging into those tables” could overwrite a user-confirmed local value. | P2 | Identity and State Model | Add a reconciliation matrix: provider-owned mirrors, user-owned policy, and compare-only settings drift. |
| H3 | The plan correctly separates Yahoo positions from canonical position, but it should explicitly prohibit provider context from entering `StatsRepository` or its `(player_id, season, season_type)` records. | P2 | Source and Trust Contract | State that Yahoo context remains FantasyDb/provider ViewModel state and joins only at fantasy decision construction. |

## KEEL review

| # | Finding | Severity | Section | Recommendation |
|---:|---|---|---|---|
| K1 | Pulses describe feature gates but not commit-sized build-green boundaries. OAuth, vault, schema, migration, and command changes could leave intermediate commits uncompilable. | P1 | Delivery Plan | Add a commit ledger where each commit compiles and tests independently, with traits/fixtures before adapters and migrations before consumers. |
| K2 | Yahoo is an explicit live query path, unlike NHL snapshot write paths. That exception is legitimate but must be named so it is not mistaken for another general query-time tier. | P2 | Architecture | Define Yahoo sync as an explicit user-invoked provider ingestion path into FantasyDb; ordinary reads never fall through to Yahoo. |
| K3 | TUI/Web parity is deferred correctly, but the plan should name the shared status/readiness ViewModel that prevents each surface from deriving freshness independently. | P3 | Pulse 6 | Define one provider connection/sync status ViewModel in core before renderer work. |

## TAPE review

| # | Finding | Severity | Section | Recommendation |
|---:|---|---|---|---|
| T1 | Exact normalized name plus current NHL team is unsafe for traded players, duplicate names, prospects, and stale Yahoo teams. The plan needs an explicit manual-resolution state and durable evidence history. | P1 | Identity and State Model | Never auto-resolve an ambiguous join; persist proposals, evidence, resolution actor/time, and Yahoo key. Preserve mappings across team changes. |
| T2 | A single run-level `fetched_at` is insufficient when settings, rosters, players, and matchups are fetched at different times or retried. | P2 | Source and Trust Contract | Persist endpoint-family observed/fetched timestamps and sync-run lineage on every normalized scope. |
| T3 | Yahoo injury/availability/status is platform context, not verified medical or NHL roster authority. | P2 | Source and Trust Contract | Keep Yahoo status in provider context and attach source/confidence; do not replace sourced IceLines observations without explicit precedence. |

## FORGE review

| # | Finding | Severity | Section | Recommendation |
|---:|---|---|---|---|
| F1 | “Operating-system credential vault” is not yet an implementable contract. Platform support, unavailable-vault behavior, credential names, and plaintext fallback policy are missing. | P1 | Security Contract | Define a `CredentialVault` trait, Windows Credential Manager production target, supported-platform adapters, stable entry names, and a hard no-plaintext-fallback error. |
| F2 | The loopback callback and HTTP client must not accidentally capture or transport `StatsRepository: !Send + !Sync`. | P2 | Crate boundaries | Keep OAuth/acquisition DTOs Send-clean in fetch; open/apply FantasyDb only after acquisition and never carry `StatsRepository` across the async boundary. |
| F3 | Yahoo response envelopes are often nested and may grow fields. “Unknown/null tests” does not define strictness. | P2 | Pulse 1 | Use typed contract-bearing DTOs, explicit extension/tolerated-envelope points, and a typed schema-drift error instead of ad hoc `Value` traversal. |

## PACE review

| # | Finding | Severity | Section | Recommendation |
|---:|---|---|---|---|
| P1 | “Conservative interval,” “short timeout,” “defensive page ceiling,” and capped retries are numerical assumptions without values or evidence. | P2 | Security/Reliability | Mark them TBD-measured in Pulse 0, record observed request count/latency/rate behavior, then version the selected defaults. |
| P2 | Workflow freshness classes are named but thresholds are absent. Draft freshness and weekly matchup freshness should not share an arbitrary age. | P2 | Observability and Readiness | Define per-workflow freshness policy only after measuring endpoint behavior; expose age and policy version. |
| P3 | Draft watcher performance claims need request-budget accounting, not an interval chosen by intuition. | P3 | Pulse 4 | Publish requests per cycle, pages per collection, p50/p95 latency, 429 observations, and stop/backoff thresholds before enabling watch mode. |

## BENCH review

| # | Finding | Severity | Section | Recommendation |
|---:|---|---|---|---|
| B1 | Atomic rollback is a key promise, but the test matrix does not enumerate failure injection after each acquisition and apply boundary. | P1 | Test Matrix | Add failpoints for every required endpoint/page, token write, staging write, reconciliation step, and final commit; assert byte-equivalent prior engine-facing state. |
| B2 | An in-memory vault proves trait logic but not production credential behavior. | P2 | Pulse 2 | Add platform adapter contract tests plus a manual Windows Credential Manager smoke checklist that verifies create/read/rotate/delete and log redaction. |
| B3 | Sanitized real fixtures can drift from the captured payload or leak private league data if sanitization is manual. | P3 | Pulse 0 | Create a deterministic sanitizer and fixture manifest containing source endpoint family, capture date, hashes, removed fields, and review status. |

## EDGE review

| # | Finding | Severity | Section | Recommendation |
|---:|---|---|---|---|
| E1 | Credential vault writes are not transactional. If Yahoo rotates a refresh token and the process dies during replacement, the only valid token may be lost. | P1 | Security Contract | Use two credential slots plus active-generation metadata: write/verify new, switch active generation, then retire old; test failure at each boundary. |
| E2 | An active draft can change while paginated players, draft results, and rosters are fetched. Local atomic commit does not make the remote reads a point-in-time snapshot. | P1 | Reliability and Sync Semantics | Add start/end sentinel re-reads or stable event watermark checks; if the scope changed mid-run, retry boundedly or commit as inconsistent/partial without deleting by absence. |
| E3 | Multiple Yahoo accounts, multiple hockey leagues, annual game-key rollover, revoked access, and league deletion need explicit state transitions. | P2 | Identity/Connection Model | Key connections separately from leagues and define disconnected, connected, reauth-required, remote-missing, and relink-required transitions. |

## WIRE review

| # | Finding | Severity | Section | Recommendation |
|---:|---|---|---|---|
| W1 | The plan does not say whether raw Yahoo responses are cached. They are useful for debugging but contain private league data and increase the breach surface. | P2 | Reliability | Do not persist raw responses by default. Persist normalized typed state and redacted diagnostics; require an explicit short-lived diagnostic capture mode if ever added. |
| W2 | “Required” versus “optional” endpoints is not yet a stable capability contract, so a partial success could vary by code path. | P2 | Reliability | Version a scope-to-required/optional capability matrix and include it in every sync run and status ViewModel. |
| W3 | HTTP/OAuth/schema/database/vault errors need distinct remediation rather than one redacted summary. | P3 | Pulse 1 | Define a non-exhaustive typed error taxonomy with retryability and user recovery action. |

## broadcast review

| # | Finding | Severity | Section | Recommendation |
|---:|---|---|---|---|
| BRC1 | Browser auto-open can fail in headless, WSL, SSH, or policy-restricted environments. | P2 | Pulse 2 | Print the authorization URL before attempting open and support copy/paste completion without assuming a GUI. |
| BRC2 | State validation and loopback binding are present, but callback Host handling, repeated callback, denial/error rendering, and shutdown behavior need explicit tests. | P2 | Security Contract | Accept only loopback destination and expected path/state, consume once, render a minimal accessible success/denial/error page, and shut down on every terminal path. |
| BRC3 | Future Web OAuth would expand the local web mutation/security boundary substantially. | P3 | Pulse 6 | Keep it explicitly out of this plan; Web shows read-only status and directs users to the CLI connection command. |

## Initial synthesis

```text
Roles reviewed: 9
P1 blockers: 7  |  P2 issues: 15  |  P3 notes: 5

Verdict: NEEDS-WORK

Top finding: Token rotation and provider synchronization each cross boundaries
that are not transactionally atomic; the plan must define recoverable generation
switching and remote-consistency checks before implementation.

Cross-role consensus: secure recovery, complete-scope proof, provider identity,
and tested rollback must be structural contracts rather than implementation notes.
```

## Required amendments

1. **Security and recovery** — specify the production credential adapter, hard
   no-plaintext fallback, two-generation refresh-token rotation, callback/headless
   behavior, and failure tests.
2. **Consistency and authority** — version the scope capability matrix, add
   start/end remote consistency checks, endpoint timestamps, field-level
   reconciliation ownership, and no raw-response persistence.
3. **Keys and delivery proof** — define complete database uniqueness axes,
   durable manual identity resolution, build-green commit slices, and failpoint
   coverage that proves prior state survives every partial failure.

## Post-amendment verification

The plan was amended in place after this review:

| Review area | Resolution in plan |
|---|---|
| Credential safety | Production vault contract, Windows target, hard no-plaintext fallback, stable entry namespace |
| Refresh rotation | Two-generation write/verify/switch/retire protocol plus failure injection |
| Remote consistency | Start/end sentinel or stable watermark, bounded retry, no deletion from inconsistent scope |
| Provider axes | Connection, game, league, player, event, and local-link uniqueness keys specified |
| Identity | Durable provider keys, manual-resolution evidence, no collision auto-resolution |
| Authority | Field-level provider/user ownership; Yahoo context barred from `StatsRepository` |
| Reliability | Versioned capability matrix, endpoint timestamps, typed errors, no raw response persistence |
| Methodology | Pulse 0 measurements own timeout/retry/page/freshness/watch defaults |
| Tests | Boundary failpoints, callback host/path/state tests, vault rotation, platform smoke, sanitizer manifest |
| Delivery | Ten independently build-green commit slices |
| Surfaces | Shared `FantasyProviderStatusView`; Web remains read-only and CLI-directed |

```text
Roles reviewed: 9
Remaining P1 blockers: 0

Final verdict: APPROVED-WITH-CONDITIONS

Conditions: Pulse 0 must verify Yahoo's real public-client PKCE callback, payload,
pagination, token-rotation, capability, and rate behavior before production DTOs
or numerical defaults are committed. No real credential or unsanitized private
league payload may enter the repository.
```

Post-review source update (2026-08-25): Yahoo's current portal issues a generic
public Client ID first and gates new Fantasy Sports access through a separate
review application. The plan now uses public-client authorization code + PKCE,
expects no client secret, records the submitted/pending approval state, and
keeps the live contract probe blocked until approval.
