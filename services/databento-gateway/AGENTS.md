# Gateway Agent Guidance

Rust crate `databento-gateway`. Inherit root `AGENTS.md`. Closest file wins.

## Commands
| Task | Command |
| --- | --- |
| Crate tests | `cargo test -p databento-gateway --all-features` |
| Contract filter | `cargo test --workspace --all-features protocol_contract` |
| Seeded loopback | `mise run dev:gateway:seeded` |
| Official loopback | `mise run dev:gateway` |
| Bounded live probe | `mise run test:live-databento` |

## Boundaries
- Own credentials, Databento clients, decode, symbology, normalization, aggregation, replay, and continuity.
- `DATABENTO_API_KEY` stays in this process. Do not log it or put it in errors, fixtures, or browser responses.
- `DATABENTO_GATEWAY_SOURCE=historical` requires feature `databento-compat` and the key. Default source is `seeded`.
- `tests/real_databento.rs` is `#[ignore]`. `scripts/test-live-databento.sh` accepts no arguments and exits `not run` without the key.

## Conventions
- Modules: `snake_case`. Public types: `PascalCase`.
- Map failures through `GatewayError` to public `ProviderErrorCode`.
- Include `protocol_contract` in shared-fixture test names.
- Integration tests stay in this crate.

## Links
- `../../ARCHITECTURE.md`
- `../../CODING_GUIDELINES.md`
- `../../contracts/protocol-v1.md`
- `src/main.rs`
