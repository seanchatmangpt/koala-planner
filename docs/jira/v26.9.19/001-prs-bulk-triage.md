# koala-planner: triage 6 open PRs

- Standing: OPEN
- Created: 2026-09-19 (v26.9.19 gh survey wave)
- Source: 6 open PRs on seanchatmangpt/koala-planner
- Evidence: `gh pr list -R seanchatmangpt/koala-planner --state open` (limit 50)

## Work to complete
- Triage each PR below: review (`gh pr view N -R seanchatmangpt/koala-planner`, `gh pr checks N -R seanchatmangpt/koala-planner`), then merge, fix, or close with rationale.
- Record the decision per PR in this file's History.

## PR list (most recent first; showing first 20 of 6)
- #6 prep/upstream-submission docs: track upstream submission readiness for the PR #1-#5 stack
- #5 refactor/rust-grounded-serializer refactor: port PANDA grounded serialization and FOND merge to pure Rust
- #4 feat/differential-oracle feat: add an executable differential oracle for frontend replacement
- #3 feat/wasm32-wasip1 feat: compile the Koala search core behind a wasm32-wasip1 ABI
- #2 feat/ggen-noun-verb-cli feat: manufacture an agent-ready noun-verb planning surface
- #1 refactor/typed-planner-core refactor: expose typed Rust planner core
## Acceptance
- Every open PR is merged or closed; `gh pr list -R seanchatmangpt/koala-planner --state open` returns empty.

## History
- 2026-09-19 | OPEN | survey found 6 open PRs | bulk ticket raised | triage pending
