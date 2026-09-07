# Contributing

## Before you start

- Search [existing issues](https://github.com/kmshdev/lwc-databento-adapter/issues) before opening a new one.
- For a change larger than a small fix, open an issue describing the problem before writing code.
- Read [`ARCHITECTURE.md`](ARCHITECTURE.md) for layer ownership and [`CODING_GUIDELINES.md`](CODING_GUIDELINES.md) for naming and test placement.

## Reporting a bug

Include:
- The command you ran and its full output.
- Expected behavior versus actual behavior.
- Whether the issue reproduces with `mise run dev:gateway:seeded` (no Databento key required).

Report security vulnerabilities as described in [`SECURITY.md`](SECURITY.md), not as a public issue.

## Development setup

```sh
mise run setup      # installs toolchains and dependencies, no Databento key required
mise run check      # full offline gate: format, lint, typecheck, build, and tests
```

A real `DATABENTO_API_KEY` is only required for `mise run test:live-databento` and `mise run test:live-integration`. Put it in an untracked `mise.local.toml` (see `.env.example` for the variable name); never commit it.

## Making a change

1. Create a branch from `main`.
2. Keep the change in one workspace tree (`services/databento-gateway`, `packages/databento-lightweight-charts`, `examples/lightweight-charts-demo`) unless the wire contract in `contracts/protocol-v1.md` changes too.
3. Add or update tests in the same tree (`CODING_GUIDELINES.md` lists the test locations per tree).
4. Run the narrowest command that covers your change first (see the tables in `AGENTS.md` and each tree's `AGENTS.md`), then run `mise run check` before opening a pull request.
5. Do not commit `mise.local.toml`, `.env`, or any credential-looking value; `scripts/scan-secrets.sh` runs as a pre-commit hook and rejects them.

## Pull requests

- Describe the resulting change, the reason for it, and how you validated it (which commands you ran).
- Keep unrelated changes out of the diff.
- CI runs `mise run check` on every pull request; it must pass before merge.

## Commit messages

Write a clear summary line describing what changed, not just `fix bug`. Squash or rebase is fine before review; once review starts, prefer merge over rebase so reviewers can track incremental changes.
