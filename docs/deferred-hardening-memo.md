# Deferred hardening memo

## Purpose

Track valuable work intentionally deferred while the project performs online beta testing and rapid feature iteration. This memo does not waive essential boundary safety or turn beta evidence into a production claim.

## Current maturity assessment

- Phase: online beta testing and rapid feature iteration.
- Repository state: the local workflow, real-provider path, public adapter API, and isolated package-consumer proof are present; an online deployment target has not yet been identified in repository evidence.
- Active priority: full workflow integration, basic functions, algorithm and execution-pipeline refinement, ToB/C APIs, reusable adapter consumption, and deployment to an identified online-beta environment.

## Non-deferrable beta guardrails

- Keep `DATABENTO_API_KEY` in the Rust gateway; never expose it to browser JavaScript, assets, logs, fixtures, or errors.
- Validate and bound browser requests, protocol messages, queues, ranges, and subscription counts at system boundaries.
- Preserve deterministic history-to-live handoff, instrument isolation, timestamp ordering, and typed failure behavior.
- Run focused tests for each implemented slice and the offline workflow needed to establish that the beta feature works.

## Deferred work register

| Item | Why deferred now | Trigger to schedule | Expected evidence |
| --- | --- | --- | --- |
| Full network-security architecture and public-edge hardening | Rapid online-beta delivery is the active priority; essential access and credential boundaries remain non-deferrable | Representative external traffic, multi-tenant onboarding, or release-candidate work | Threat model, authentication/entitlement design, ingress policy, abuse/rate-limit tests |
| Broad security and dependency scans | Focused boundary safety is sufficient before runtime dependencies exist | Workspace and lockfiles stabilize or a release candidate is cut | SAST/dependency/license reports with triaged findings |
| Penetration testing | No deployed public attack surface exists yet | Public beta or contracted ToB/C security requirement | Scoped test report and remediation record |
| Long-running soak, fault-injection, and chaos testing | Runtime/session behavior must exist before these tests provide signal | Stable beta sessions and representative reconnect/backpressure behavior | Bounded soak results, recovery timelines, resource high-water marks |
| Extensive mutation/property/fuzz campaigns | Focused invariant tests should accompany implementation first | Normalization, aggregation, protocol, and handoff APIs stabilize | Mutation score and retained fuzz/property regression corpus |
| Generation-safe terminal subscription cleanup | Typed terminal errors are emitted, but a forwarding failure can leave process-scoped subscription accounting until explicit client cleanup | Long-lived beta sessions, reconnect churn, or subscription-capacity pressure | Transport test proving terminal/error/slow-consumer paths remove only the matching subscription generation and release registry references |
| Dataset actor compaction after canonical churn | Databento has no per-subscription live unsubscribe; actor registration is idempotent, so reactivation cannot duplicate routing, but inactive upstream requests live until the dataset client closes | Representative symbol churn on shared dataset sessions | Bounded churn test plus an actor rotation/compaction policy that preserves active streams and replay continuity |
| Production SLOs, alerting, capacity planning, and disaster recovery | Meaningful targets require representative beta traffic and operational ownership | Sustained beta traffic or production-readiness planning | SLOs, dashboards, alerts, capacity model, recovery drill |
| Full upstream Lightweight Charts browser matrix | The upstream checkout currently lacks installed Puppeteer dependencies and standalone bundles | Adapter demo exists and browser behavior is integration-critical | Puppeteer results across required DPR, resize, pane, accessibility, and cleanup cases |

## Update log

### 2026-08-31 — Dedicated-connectivity target

- Added `REQ-Q-006` and `TASK-12` for the user-selected sub-50-microsecond target.
- Preserved Databento's exact published boundary: Live Raw API TCP, 10G or 25G at DC3 or Equinix NY4/5, 90th percentile, boundary switch to customer cross-connect.
- Recorded the current external block: no site, provider, port, commercial approval, installed circuit, or circuit-specific measurement receipt is available. Browser, public-network, and gateway timings are non-qualifying.
- Added a dated DC3 shortlist and RFQ packet. Provider facility presence is evidenced, but direct Databento cross-connect availability remains unverified until written responses are received.
- Verified official outreach routes and added a response ledger. Outreach, quote collection, provider selection, and circuit provisioning remain approval-gated external work.

### 2026-08-31 — Contract loop, browser isolation, and page budget

- Repaired all self-starting Puppeteer harnesses so foreign services on fixed development ports cannot satisfy readiness; explicit external URLs remain supported, while default runs allocate isolated gateway and demo ports.
- Added default-feature gateway compilation evidence alongside the all-features gate and corrected the feature-gated transport import it exposed.
- Requalified the bounded real provider and browser flows without exposing credentials. Account-aware historical ranges include `EQUS.MINI`, `OPRA.PILLAR`, and `GLBX.MDP3`; only the bounded `GLBX.MDP3` live session is claimed.
- Added the active 50 ms page-load release criterion as a repeatable production-preview gate. The current one-page route inventory passes; production real-user monitoring and service-level objectives remain deferred until representative beta traffic exists.
- Pinned older-history pagination to the request that produced the visible chart, invalidated delayed request results on disconnect, and limited the browser-observable app handle to explicit E2E builds.
- Enforced the accepted live-edge-only `open_bars` range before upstream work and consumed live DBN mapping records without silently mixing a changed continuous instrument.
- Added structural TypeScript models for all official reference-data enum tables. The package deliberately does not bundle a value snapshot that would age independently of Databento's catalog.
- Repaired the UTC-rollover handoff by clamping historical resolution to account-aware availability while retaining the current live edge, draining replay concurrently with history, and treating instrument ID as the continuity identity when live mapping adds a raw symbol.
- Removed open-market bar receipt from the deferred register after the bounded `GLBX.MDP3` proof observed 18 normalized `ohlcv-1m` bars and a complete replay boundary. Broader dataset entitlements remain unclaimed.

### 2026-08-30 — Phase policy and option research

- Added the beta-phase prioritization policy to root `AGENTS.md`.
- Recorded deeper security, network, performance, resilience, and testing work without moving it into the active implementation path.
- Assessed the three selected product decisions against the current architecture; no security overhaul or live-provider call was performed.
- Next maturity review: after the first end-to-end history, live handoff, chart rendering, and local beta workflow operate with representative usage.

### 2026-08-31 — Account-aware metadata discovery

- Replaced the seven-argument market-data probe with a zero-argument metadata inventory using the official Rust client.
- Kept the global dataset catalog and publisher mappings distinct from the account-aware ranges returned by `get_dataset_range`.
- Kept live licensing, paid record requests, and dataset-specific live-session qualification deferred because the official API has no bulk live-entitlement endpoint.

### 2026-08-31 — Live provider and external-consumer qualification

- Used the key for a bounded `GLBX.MDP3` live session and observed subscription acknowledgement, replay completion, continuous-symbol resolution, and graceful close.
- Drove `openBars` and `subscribeBars` through the real gateway using the Codex in-app browser; both reached `live`, returned the resolved instrument mapping, and closed without browser errors or credential exposure.
- Fixed millisecond live-edge timestamps, subscription failure cleanup, historical-availability lag during handoff, invalid unbounded `subscribeBars` resolution, and readonly arrays that prevented direct Lightweight Charts `setData` integration.
- Added an isolated packed-tarball consumer compile. Registry publication and an open-market bar-event receipt remain deferred.

### 2026-08-31 — Hierarchical agent instructions

- Combined the folder blueprint into `ARCHITECTURE.md` and added `CODING_GUIDELINES.md` plus nested `AGENTS.md` files. Closest instruction file wins.
- Did not add a `docs/codebase/` encyclopedia; existing `docs/` contracts remain the long-form truth.
- Deferred work is unchanged: network-security overhaul, broad scans, penetration, soak, and production SLOs stay off the active path.

### 2026-08-31 — Online-beta phase and package integration

- Advanced the project instructions from loopback-only local beta to online beta testing and rapid feature iteration.
- Made the adapter npm-publication compatible with public exports, generated declarations, side-effect metadata, a Lightweight Charts 5.2 peer range, package-level integration documentation, and a prepack build.
- Expanded the isolated consumer proof across history, live handoff, streaming updates, symbology, metadata, and disposal.
- Deferred deep security, penetration, soak, broad browser-matrix, and production SLO work. Essential server-side credentials, bounded input, typed continuity failures, and the application authentication/entitlement boundary remain mandatory.
- Registry release remains blocked on package-scope ownership, license selection, and release identity. Online deployment remains blocked on a named target and its basic access/entitlement integration.
