# Project Process Archive

Spec-driven-development artifacts from the initial build. Inherit `../AGENTS.md`. Closest file wins.

## Rules
- These files are a release gate and audit trail, not developer-facing documentation. Do not link them from package READMEs.
- Requirement IDs in `requirements.md` must match `traceability.md`. Task IDs in `implementation-plan.md` must match `traceability.md`.
- Keep `DEC-014`, `DEC-015`, and `DEC-016` in the files `scripts/check-doc-contracts.sh` lists.
- After a conversation that changes or reviews the project, append `deferred-hardening-memo.md`.
- `dedicated-connectivity-plan.md` and `dedicated-connectivity-rfq.md` are vendor procurement notes, not engineering design; do not treat them as implemented behavior.

## Index
| File | Content |
| --- | --- |
| `EXECPLAN.md` | Original build plan: progress log, surprises, decisions, retrospective |
| `requirements.md` | Versioned scope, observable behavior, and accepted product decisions |
| `implementation-plan.md` | Task list with acceptance criteria |
| `traceability.md` | Requirement-to-design-to-test-to-proof matrix |
| `review-route.md` | Pre-implementation review pass order and evidence gates |
| `reviewer-convergence.md` | Independent reviewer reconstruction and agreement record |
| `mise-plan.md` | Tool/config/secret ownership decisions during the initial build |
| `dedicated-connectivity-plan.md` | Requirements for a qualified sub-50-microsecond Live Raw API cross-connect |
| `dedicated-connectivity-rfq.md` | Colocation vendor shortlist and RFQ questions for that cross-connect |
| `deferred-hardening-memo.md` | Running log of work intentionally deferred during online-beta iteration |
