# koala-planner migration plan v26.11.16

Standing: UNVERIFIED

## Identity
- Path: /Users/sac/koala-planner
- Remote: https://github.com/seanchatmangpt/koala-planner
- Branch: main@d050e1f (origin/main: ahead 4)
- Last commit: 2026-09-29; commits: 222; size: 102 MB
- Languages/build: Rust (planner/, crates/, cargo make), C++ (parser/, grounder/), Python
  (solve.py, serializer/), ggen.toml
- Nested repos: none in census; `grounder/pandaPIgrounder` is an unmapped submodule path
  (`git submodule status` fatal: no mapping in .gitmodules)

## Observed state
- Dirty: 1 — readme.md modified
- Untracked: 0
- Ahead 4 / behind 0
- Stash: 0
- Unpushed local branches: 2 — exp/cli-contract (merged at 632dbea),
  modep/preserve-20260926-l6-dirty (preserved pre-merge dirty state, 42 paths)
- Build dirs: none present
- Submodules: broken mapping for grounder/pandaPIgrounder

## What it is
Koala: FOND HTN planner (HDDL with `oneof`), pipeline C++ parse -> PANDA ground -> Python
serialize -> Rust AO* search. Fork prepared for upstream submission.

## Classification
ACTIVE (census agrees).

## Migration risk
- 4 unpushed commits on main, 1 modified readme, 2 unpushed branches (one is a deliberate
  preservation branch of dirty state).
- Broken submodule mapping could make a fresh clone incomplete.
Rate: MEDIUM

## Recommended action
COMMIT-PUSH
1. `cd /Users/sac/koala-planner && git add readme.md && git commit -m "docs: readme"`
2. `git push origin main`; verify `git ls-remote origin | grep $(git rev-parse HEAD)`.
3. `git push origin modep/preserve-20260926-l6-dirty exp/cli-contract`
4. DECISION: fix grounder/pandaPIgrounder — add to .gitmodules or vendor it; otherwise the
   new clone lacks the grounder.
5. Clone fresh on new machine.

## First commands on the new machine
`git clone https://github.com/seanchatmangpt/koala-planner && cd koala-planner/planner &&
cargo check` ; then `cd ../parser && make` (stated, not run)

## See Also
- /Users/sac/docs/jira/v26.11.16/README.md (no previous jira dir)
