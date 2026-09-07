# Changelog

All notable changes to this project are documented in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

No changes since `0.1.0`.

## [0.1.0] - 2026-09-07

No version has been published to npm or crates.io yet. `@lwc-databento/adapter` and `databento-gateway` are both `0.1.0` and under active online-beta iteration on `main`.

### Added

- `@lwc-databento/adapter`: framework-neutral `getBars`, `openBars`, and `subscribeBars` for loading Databento history and going live in Lightweight Charts, with typed errors and symbol-mapping notifications.
- `databento-gateway`: Rust HTTP/WebSocket gateway that owns `DATABENTO_API_KEY`, decodes DBN records, normalizes and aggregates bars, and streams replay-to-live continuity over `databento-lwc.v1`.
- `examples/lightweight-charts-demo`: reference Vite application demonstrating history load, infinite scroll-back, live go-live, and reconnect handling.
- Versioned wire contract and shared fixtures in `contracts/`.
- `LICENSE` (MIT) and `license`/`repository` package metadata; `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, and GitHub issue/PR templates.

### Changed

- Reorganized `docs/` into developer/agent-facing reference docs and a `docs/project/` process archive, with `docs/README.md` as the index.
- Merged `Project_Folders_Structure_Blueprint.md` into `ARCHITECTURE.md`, which is now the single canonical layout, placement-rule, and layer-boundary document.
