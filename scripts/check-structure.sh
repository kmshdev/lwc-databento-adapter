#!/bin/sh
set -eu

required_paths='AGENTS.md
ARCHITECTURE.md
CODING_GUIDELINES.md
Project_Folders_Structure_Blueprint.md
contracts/AGENTS.md
contracts/protocol-v1.md
packages/databento-lightweight-charts/AGENTS.md
packages/databento-lightweight-charts/package.json
packages/databento-lightweight-charts/src/index.ts
services/databento-gateway/AGENTS.md
services/databento-gateway/Cargo.toml
services/databento-gateway/src/main.rs
examples/lightweight-charts-demo/AGENTS.md
examples/lightweight-charts-demo/package.json
examples/lightweight-charts-demo/src/main.ts
docs/AGENTS.md
scripts/AGENTS.md
Cargo.toml
package.json
pnpm-workspace.yaml
mise.toml'

missing=0
for required_path in $required_paths; do
  if [ ! -e "$required_path" ]; then
    printf 'missing required path: %s\n' "$required_path" >&2
    missing=1
  fi
done

if [ "$missing" -ne 0 ]; then
  exit 1
fi

tracked_generated=$(git ls-files 'node_modules/**' 'dist/**' 'coverage/**' 'target/**' '.cache/**')
if [ -n "$tracked_generated" ]; then
  printf '%s\n' 'generated output is tracked:' >&2
  printf '%s\n' "$tracked_generated" >&2
  exit 1
fi

misplaced_source=$(rg --files -g '*.rs' -g '*.ts' -g '*.tsx' | awk '
  $0 !~ /^packages\/databento-lightweight-charts\// &&
  $0 !~ /^services\/databento-gateway\// &&
  $0 !~ /^examples\/lightweight-charts-demo\// {
    print
  }
')
if [ -n "$misplaced_source" ]; then
  printf '%s\n' 'source files are outside the approved workspace roots:' >&2
  printf '%s\n' "$misplaced_source" >&2
  exit 1
fi

if rg -n 'lightweight-chart-react' packages examples package.json pnpm-lock.yaml; then
  printf '%s\n' 'lightweight-chart-react is forbidden; use lightweight-charts directly.' >&2
  exit 1
fi

if rg -n --glob 'packages/**' --glob 'examples/**' 'services/databento-gateway|databento_gateway' .; then
  printf '%s\n' 'browser packages may not import gateway internals.' >&2
  exit 1
fi
