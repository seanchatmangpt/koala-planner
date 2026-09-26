# koala-planner: commit or clean uncommitted changes

- Standing: OPEN
- Created: 2026-09-19 (v26.9.19 gh survey wave)
- Source: working tree dirty at survey time
- Evidence: `git status --porcelain` → 41 path(s) (tracked-modified: 41, untracked: 0); sample:  M planner/src/domain_description/classical_domain/definition.rs; M planner/src/domain_description/classical_domain/mod.rs; M planner/src/domain_description/facts.rs;

## Work to complete
- Review the 41 tracked-modified path(s); commit them as atomic pieces on a purpose branch, or revert what is transient.
- Note: this survey's ticket files under docs/jira/v26.9.19/ are intentionally uncommitted; include or exclude them deliberately in the commit plan.

## Acceptance
- `git status --porcelain` is clean (except items deliberately deferred and recorded here).

## History
- 2026-09-19 | OPEN | survey found dirty tree | 41 paths (T41/U0) | commit/clean pending
