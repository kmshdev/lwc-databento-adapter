# Contracts Agent Guidance

Shared browser/gateway wire contract. Inherit root `AGENTS.md`. Closest file wins.

## Rules
- `protocol-v1.md` is the JSON contract. Objects use `v: 1`. Unknown command fields are rejected.
- `protocol-v1.schema.json` is machine-generated from the gateway's `protocol.rs` (`schemars`); never edit it by hand. Regenerate with `UPDATE_PROTOCOL_SCHEMA=1 cargo test -p databento-gateway --features json-schema protocol_contract_schema`; a freshness test fails when it is stale.
- The generated schema is structural only; semantic invariants (safe-integer times, volume/bar time equality, event ordering) live in the Rust and Zod validators.
- Change the wire shape and add valid/invalid fixtures here before implementation.
- Fixtures are plain JSON under `fixtures/http/{valid,invalid}/` and `fixtures/websocket/{valid,invalid}/`.
- Each fixture has `valid`, `direction` (`http-response` | `client-command` | `server-event`), and `payload`.
- No language-specific fixtures, credentials, or unsanitized market data.

## Commands
| Task | Command |
| --- | --- |
| Both sides | `mise run test:contracts` |
| TypeScript | `pnpm --filter @lwc-databento/adapter test:contract` |
| Rust | `cargo test --workspace --all-features protocol_contract` |

## Links
- `protocol-v1.md`
- `../ARCHITECTURE.md`
- `../docs/technical-design.md`
