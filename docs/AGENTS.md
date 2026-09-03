# Docs Agent Guidance

Planning and evidence docs. Inherit root `AGENTS.md`. Closest file wins.

## Commands
| Task | Command |
| --- | --- |
| Doc contracts | `scripts/check-doc-contracts.sh` |

## Rules
- Do not copy requirements, protocol, or test matrices into agent files; link them.
- Keep `DEC-014`, `DEC-015`, and `DEC-016` in the files `scripts/check-doc-contracts.sh` lists.
- Requirement IDs in `requirements.md` must match `traceability.md`. Task IDs in `implementation-plan.md` must match `traceability.md`.
- After a conversation that changes or reviews the project, append `deferred-hardening-memo.md`.
- Structural decisions: `adr/`. Product decisions: `decision-log.md`. Do not invent deployment targets or credentials.
- Repo is public: never add machine-specific absolute paths (e.g. `/Users/<name>/...`); use repo-relative paths or generic placeholders.

## Index
| Need | File |
| --- | --- |
| Exec plan | `EXECPLAN.md` |
| Requirements | `requirements.md` |
| Design | `technical-design.md` |
| Tasks | `implementation-plan.md` |
| Tests | `test-strategy.md` |
| Traceability | `traceability.md` |
| Decisions | `decision-log.md` |
| Mise | `mise-plan.md` |
