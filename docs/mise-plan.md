# Mise configuration and task plan

## Current state

`mise.local.toml` contains an `[env]` declaration for `DATABENTO_API_KEY`. Its value is local-only, is not copied into any artifact, and the file is ignored by Git. Mise currently refuses to load the file because this project config has not been trusted.

The documentation loop does not run `mise trust` on the user's behalf. Trusting a directory permits its configuration to affect later shell and task execution and is therefore an explicit machine-state decision.

## Configuration ownership

| File | Committed | Responsibility |
| --- | --- | --- |
| `mise.toml` | yes, after `TASK-00` | Pinned public toolchain declarations and task includes only |
| `mise.local.toml` | no | Developer-specific secrets such as `DATABENTO_API_KEY`; never tool versions or shared task behavior |
| `mise.lock` | yes when generated | Resolved tool versions, if supported by the selected mise/tool plugins |

Environment precedence must be documented and deterministic: CI secret injection or the developer's local secret file supplies the key; the committed config must never provide a fallback credential.

## Proposed tool declarations

Exact versions are selected and locked by `TASK-00`, after compatibility spikes. The initial constraints are:

- Node.js satisfies Lightweight Charts 5.2.1's upstream `>=22.3` constraint and the adapter's selected package tools.
- Rust is a pinned stable release supported by the selected `databento`, Tokio, and WebSocket stack.
- `pnpm`, `cargo-mutants`, and any test tools are declared only if used by a release gate.
- No tool is declared both in mise and an unrelated bootstrap script.

## Proposed task hierarchy

```text
setup
├── setup:tools
└── setup:hooks

check
├── docs:check
├── fmt:check
├── lint
├── typecheck
└── test
    ├── test:rust
    ├── test:typescript
    ├── test:contracts
    ├── test:browser
    └── test:performance

test:live-databento  (manual, bounded live provider, never a dependency of check)
test:live-integration (manual, real gateway plus browser consumer)
test:package-consumer (offline packed-package compile, dependency of check)
```

Rules:

- Leaf tasks own one tool invocation and declare their source inputs.
- Aggregate tasks use dependencies rather than repeating shell commands.
- `check` is offline, deterministic, and safe for CI.
- `test:browser` builds the adapter's standalone browser bundle before invoking the Puppeteer runner.
- `test:performance` builds the production preview and requires every inventoried page to have cache-disabled local `PerformanceNavigationTiming.duration` p95 below 50 ms across 20 runs after three warmups.
- `test:live-databento` accepts no arguments. It inventories account-aware ranges, resolves `ES.c.0`, and opens one bounded `GLBX.MDP3` `ES.FUT` `ohlcv-1m` replay session. It requires subscription acknowledgement, replay completion, normalization of any received OHLCV, and graceful close without printing authorization.
- `test:live-integration` starts or reuses the official loopback gateway and demo, then requires the packaged provider to reach `live` through Lightweight Charts without a browser-visible credential.
- `test:package-consumer` packs the built adapter and compiles an isolated project against the tarball and the declared Lightweight Charts peer dependency.
- Destructive cleanup is not part of setup or check.

## Trust and bootstrap gate

When implementation is authorized, the developer performs and reviews:

```sh
cd /Users/kmsh/code/lwc-databento-adapter
mise trust
mise doctor
mise install
mise run docs:check
```

Before `mise trust`, review the committed `mise.toml` and the key names—not values—in `mise.local.toml`. A successful `mise env --json` check may assert that `DATABENTO_API_KEY` exists, but must never print its value.

## Acceptance criteria

- A fresh checkout can install the pinned public tools and run the offline `check` graph using documented commands.
- Absence of `DATABENTO_API_KEY` does not break offline tasks.
- The live task fails closed before network access when any bound is missing.
- Secret-bearing files are ignored, and repository scans find no credential value.
- Every task used in `implementation-plan.md` names its mise entry point once the task file exists.
