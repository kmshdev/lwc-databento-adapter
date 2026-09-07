# Docs Agent Guidance

Reference and evidence docs. Inherit root `AGENTS.md`. Closest file wins.

This directory has two tiers:
- `docs/*.md` and `docs/adr/`: reference material a developer or agent needs to build, test, or extend the project. See `README.md` for the index.
- `docs/project/`: spec-driven-development process artifacts (exec plan, requirements, task list, traceability, review notes, vendor RFQs). Not required reading to use or contribute to the code; kept for historical traceability. See `docs/project/AGENTS.md`.

## Commands
| Task | Command |
| --- | --- |
| Doc contracts | `scripts/check-doc-contracts.sh` |

## Rules
- Do not copy requirements, protocol, or test matrices into agent files; link them.
- Keep `DEC-014`, `DEC-015`, and `DEC-016` in the files `scripts/check-doc-contracts.sh` lists.
- Requirement IDs in `project/requirements.md` must match `project/traceability.md`. Task IDs in `project/implementation-plan.md` must match `project/traceability.md`.
- After a conversation that changes or reviews the project, append `project/deferred-hardening-memo.md`.
- Structural decisions: `adr/`. Product decisions: `decision-log.md`. Do not invent deployment targets or credentials.
- Repo is public: never add machine-specific absolute paths (e.g. `/Users/<name>/...`); use repo-relative paths or generic placeholders.
- New reference docs for developers/agents go in `docs/`. New process artifacts go in `docs/project/`.
