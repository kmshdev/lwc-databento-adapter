# Changelog

All notable changes to this project are documented in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

No version has been published to npm or crates.io yet. `@lwc-databento/adapter` and `databento-gateway` are both `0.1.0` and under active online-beta iteration on `main`.

### Added

- `@lwc-databento/adapter`: framework-neutral `getBars`, `openBars`, and `subscribeBars` for loading Databento history and going live in Lightweight Charts, with typed errors and symbol-mapping notifications.
- `databento-gateway`: Rust HTTP/WebSocket gateway that owns `DATABENTO_API_KEY`, decodes DBN records, normalizes and aggregates bars, and streams replay-to-live continuity over `databento-lwc.v1`.
- `examples/lightweight-charts-demo`: reference Vite application demonstrating history load, infinite scroll-back, live go-live, and reconnect handling.
- Versioned wire contract and shared fixtures in `contracts/`.
