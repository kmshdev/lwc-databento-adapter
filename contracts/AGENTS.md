# Contracts Agent Guidance

Shared browser/gateway wire contract. Inherit root `AGENTS.md`. Closest file wins.

## Rules
- `protocol-v1.md` is the JSON contract. Objects use `v: 1`. Unknown command fields are rejected.
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
