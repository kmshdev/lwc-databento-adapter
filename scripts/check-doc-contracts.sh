#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/lwc-doc-check.XXXXXX")
trap 'rm -rf "$tmp_dir"' EXIT HUP INT TERM

required_docs='README.md
ARCHITECTURE.md
CODING_GUIDELINES.md
Project_Folders_Structure_Blueprint.md
docs/EXECPLAN.md
docs/requirements.md
docs/technical-design.md
docs/implementation-plan.md
docs/test-strategy.md
docs/traceability.md
docs/decision-log.md
docs/review-route.md
docs/lightweight-charts-core-knowledge.md
docs/lightweight-charts-tutorial-knowledge.md
docs/mise-plan.md
docs/reviewer-convergence.md'

printf '%s\n' "$required_docs" | while IFS= read -r path; do
  if [ ! -s "$repo_root/$path" ]; then
    printf 'missing or empty required document: %s\n' "$path" >&2
    exit 1
  fi
done

for contract_phrase in \
  'barsInLogicalRange' \
  'lightweight-chart-react' \
  'user-resizable pane'; do
  if ! rg -Fq "$contract_phrase" "$repo_root/docs/requirements.md"; then
    printf 'required Lightweight Charts contract missing from requirements: %s\n' "$contract_phrase" >&2
    exit 1
  fi
done

rg -o 'REQ-(F|Q)-[0-9]{3}' "$repo_root/docs/requirements.md" \
  | sort -u >"$tmp_dir/requirements.ids"
rg -o 'REQ-(F|Q)-[0-9]{3}' "$repo_root/docs/traceability.md" \
  | sort -u >"$tmp_dir/traceability.ids"

if ! diff -u "$tmp_dir/requirements.ids" "$tmp_dir/traceability.ids"; then
  printf 'requirement IDs differ between requirements and traceability\n' >&2
  exit 1
fi

rg -o '^### TASK-[0-9]{2}' "$repo_root/docs/implementation-plan.md" \
  | sed 's/^### //' \
  | sort -u >"$tmp_dir/tasks.ids"
rg -o 'TASK-[0-9]{2}' "$repo_root/docs/traceability.md" \
  | sort -u >"$tmp_dir/traced-tasks.ids"

if ! diff -u "$tmp_dir/tasks.ids" "$tmp_dir/traced-tasks.ids"; then
  printf 'task IDs differ between implementation plan and traceability\n' >&2
  exit 1
fi

for decision in DEC-014 DEC-015 DEC-016; do
  for path in \
    README.md \
    docs/EXECPLAN.md \
    docs/requirements.md \
    docs/technical-design.md \
    docs/implementation-plan.md \
    docs/traceability.md \
    docs/decision-log.md; do
    if ! rg -q "$decision" "$repo_root/$path"; then
      printf '%s is missing from %s\n' "$decision" "$path" >&2
      exit 1
    fi
  done
done

if rg -q 'OPEN-00[123]' "$repo_root/README.md" "$repo_root/AGENTS.md" "$repo_root/docs"; then
  printf 'superseded open-decision marker remains in normative documentation\n' >&2
  exit 1
fi

for command in \
  'pnpm format:check' \
  'pnpm lint' \
  'pnpm typecheck' \
  'pnpm test' \
  'pnpm build' \
  'cargo fmt --all -- --check' \
  'cargo clippy --workspace --all-targets --all-features -- -D warnings' \
  'cargo test --workspace --all-features' \
  'cargo deny check' \
  'pnpm test:e2e:offline'; do
  if ! rg -Fq "$command" "$repo_root/docs/test-strategy.md"; then
    printf 'release command missing from test strategy: %s\n' "$command" >&2
    exit 1
  fi
  if ! rg -Fq "$command" "$repo_root/docs/implementation-plan.md"; then
    printf 'release command missing from implementation plan: %s\n' "$command" >&2
    exit 1
  fi
done

printf 'documentation contracts are consistent\n'
