# Databento adapter for Lightweight Charts

This repository is building a reusable TypeScript data-provider package and a Rust gateway that translate Databento historical and live OHLCV data into TradingView Lightweight Charts 5.2 data models without exposing Databento credentials to the browser.

The core local workflow is complete and the project is now in online beta testing and rapid feature iteration. The documents below are its implementation contract:

- [Requirements](docs/requirements.md) defines scope, observable behavior, assumptions, and accepted product decisions.
- [Technical design](docs/technical-design.md) fixes component ownership, public interfaces, transport messages, data invariants, dependencies, and lifecycle algorithms.
- [Implementation plan](docs/implementation-plan.md) is the living execution plan and task list; every task has acceptance criteria.
- [Test strategy](docs/test-strategy.md) defines the test layers, fixtures, failure cases, and release gates.
- [Traceability](docs/traceability.md) maps every requirement to design, tasks, and tests.
- [Decision log](docs/decision-log.md) records assumptions, authoritative sources, and accepted decisions.
- [Review route](docs/review-route.md) pins research inputs, pass order, evidence gates, and stop conditions.
- [Lightweight Charts core knowledge](docs/lightweight-charts-core-knowledge.md) records public API constraints and wrapper-replacement rules.
- [Lightweight Charts tutorial knowledge](docs/lightweight-charts-tutorial-knowledge.md) records user-surface, rendering, accessibility, and browser-test requirements.
- [Mise plan](docs/mise-plan.md) fixes tool/config ownership, secret handling, and the future task graph.
- [Reviewer convergence](docs/reviewer-convergence.md) records the two independent reconstructions, repaired gaps, and material agreement.
- [Dedicated connectivity qualification](docs/dedicated-connectivity-plan.md) defines the external physical evidence required for the sub-50-microsecond Live Raw API target.
- [Dedicated connectivity RFQ](docs/dedicated-connectivity-rfq.md) records the dated DC3 provider shortlist and ready-to-send commercial and technical questions.
- [Architecture](ARCHITECTURE.md) is the layout map and layer boundary. [Coding guidelines](CODING_GUIDELINES.md) records observed naming, test placement, and secret rules. [Project folder blueprint](Project_Folders_Structure_Blueprint.md) points at `ARCHITECTURE.md`.

The accepted decisions are session-pinned continuous subscriptions, parent resolution followed by explicit child selection, and an explicit atomic Go-live reset. They are traced as `DEC-014`, `DEC-015`, and `DEC-016`.

Run `scripts/check-doc-contracts.sh` after changing the planning documents. Run `scripts/check-structure.sh` after changing the workspace layout. Together they verify that required artifacts exist and that requirement IDs, task IDs, accepted decisions, release commands, and placement rules remain aligned.

Run `mise run check` for the complete offline graph. Start the official loopback beta with `mise run dev:gateway` and `mise run dev:demo`; the gateway task requires the ignored `DATABENTO_API_KEY`. Use `mise run dev:gateway:seeded` for deterministic offline UI work.

Run `mise run test:live-databento` with no arguments to inventory account-aware historical availability and execute a bounded `GLBX.MDP3` / `ES.FUT` / `ohlcv-1m` live replay session. The live proof requires subscription acknowledgement, replay completion, normalized bars when the venue emits any, and graceful close. Run `mise run test:live-integration` to drive the real gateway through the packaged provider and Lightweight Charts demo. `mise run test:package-consumer` verifies the publishable archive, installs it in an isolated project with `lightweight-charts`, and compiles history, handoff, streaming, symbology, metadata, teardown, candlestick, and volume usage.

Consumer installation and complete framework-neutral usage are documented in [`packages/databento-lightweight-charts/README.md`](packages/databento-lightweight-charts/README.md). The package is configured for public npm publication, but the first registry release still requires an approved package scope, license, and release identity.
