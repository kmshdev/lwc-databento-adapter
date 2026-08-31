# Coding Guidelines

Observed from this repository. Formatter and linter configs remain authoritative: `.oxfmtrc.json`, `.oxlintrc.json`, `tsconfig.base.json`, `cargo fmt`, `cargo clippy -- -D warnings`.

## Naming

- Rust files and modules: `snake_case`. Public Rust types: `PascalCase`.
- TypeScript source files: `kebab-case` (`http-client.ts`, `live-socket.ts`). Exported classes/types: `PascalCase`. Functions: `camelCase`.
- Package names: `@lwc-databento/adapter`, `@lwc-databento/demo`. Crate name: `databento-gateway`.

## TypeScript

- ESM only (`"type": "module"`). Relative imports keep the `.js` suffix (`../errors/index.js`).
- Public surface is `packages/databento-lightweight-charts/src/index.ts`. Do not add a second public entry.
- Adapter depends on `zod` and peers `lightweight-charts` `^5.2.1`. It does not import a UI framework or create `ISeriesApi`.
- Typed failures use `DatabentoProviderError` and public `ProviderErrorCode` values. Do not put upstream bodies or credentials in errors.

## Rust

- Workspace edition 2021, rust-version `1.97.1`.
- Official Databento client is optional feature `databento-compat`.
- `DATABENTO_GATEWAY_SOURCE` is `seeded` or `historical`. `historical` requires `databento-compat` and `DATABENTO_API_KEY`.
- Domain errors are `GatewayError` mapped to the same public error codes as TypeScript. Sanitize messages.

## Tests and fixtures

- Adapter: `test/unit/`, `test/contract/` (Vitest).
- Gateway: crate tests plus `tests/real_databento.rs` (`#[ignore]`, needs the key).
- Demo: `test/e2e/*.e2e.ts` (Puppeteer via `tsx`).
- Shared JSON fixtures live in `contracts/fixtures`. Both languages consume them. Rust contract tests must include `protocol_contract` in the test name (`mise.toml` / `package.json` filter).
- Fixtures are sanitized synthetic data only.

## Generated and secrets

- Do not edit or commit `node_modules/`, `dist/`, `coverage/`, `target/`, `.cache/`.
- `DATABENTO_API_KEY` belongs in ignored `mise.local.toml` only. `scripts/scan-secrets.sh` rejects credential-looking values in tracked files.
- Demo scripts unset the key: `env -u DATABENTO_API_KEY`.

## Commands for a single tree

Prefer package filters over the full graph when the change is local:

| Scope | Command |
| --- | --- |
| Adapter tests | `pnpm --filter @lwc-databento/adapter test` |
| Adapter contracts | `pnpm --filter @lwc-databento/adapter test:contract` |
| Gateway tests | `cargo test -p databento-gateway --all-features` |
| Demo offline e2e | `pnpm --filter @lwc-databento/demo test:e2e` |
