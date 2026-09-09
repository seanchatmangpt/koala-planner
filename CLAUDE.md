# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Koala solves FOND (Fully Observable Non-Deterministic) HTN planning problems, extended
HDDL syntax (`oneof` = non-deterministic effects). Pipeline, four stages, four
tools/languages, glued by Python: HDDL parse (C++) → ground (C++, PANDA grounder) →
serialize to JSON (Python) → search (Rust). `solve.py` runs all four end to end.

## Repo layout

- `parser/` — C++ HDDL parser (unmodified pandaPIparser fork), `make`. Produces
  `pandaPIparser`.
- `grounder/` — C++ grounder (unmodified pandaPIgrounder fork), `./build.sh`. Consumes
  parser output → `result.sas+`.
- `serializer/` — Python: `htn_parser.py` (sas+ → planner JSON), `htn_serializer.py`,
  `fond_merger.py`, `prob_preprocessor.py` (strips/reattaches probabilistic annotations
  around the parser stage).
- `planner/` — Rust solver. Primary dev surface; everything below is scoped here.
- `solve.py` / `batch_solve.py` — orchestration for one problem / a `domains/<name>/`
  directory across heuristic×mode combinations.

## Build

```bash
cd parser && make
cd grounder && ./build.sh
cd planner && cargo build --release   # or `cargo make`
```

## Run planner directly (input = serialized JSON, not raw HDDL)

```bash
cd planner
cargo run --release -- <problem.json> [--fixed|--flexible|--andstar-fond|--fixed-ld] [--ff|--add|--max|--lmcut] [--tiebreak policy-size|closure|combined] [--threshold N]
```

No mode → `--flexible` (AO*). Heuristic flag is required by the binary (panics without
one); `solve.py` defaults to `--ff`. `--tiebreak` only applies to `--andstar-fond`.
Example JSON: `planner/test_domains/*.json`,
`planner/src/domain_description/htn_domain/test_case.json`.

## Full pipeline

```bash
python solve.py /path/domain.hddl /path/problem.hddl [mode] [heuristic] [--threshold N] [--tiebreak ...] [--mem-limit GB] [--output path] [--keep-json]
```

Requires `parser/pandaPIparser` and `grounder/pandaPIgrounder/pandaPIgrounder` already
built, and invocation from repo root (paths are `cwd`-relative). Probabilistic domains
auto-detected and preprocessed.

## Tests (Rust)

```bash
cd planner
cargo test                        # all
cargo test <substring>            # filter
cargo test --package planner mod::path::test_name   # single
```

Colocated `#[test]` / sibling `test_cases/` modules, not a top-level `tests/` dir — e.g.
`search/fixed_method/test_cases/`, `search/acyclic_plan/acyclic_space/test_cases/`,
`search/htn_andstar/domain_tests.rs`.

## Planner architecture (`planner/src/`)

Four search modes (CLI-selected in `main.rs`), same `FONDProblem`, different search
loops — they deliberately do not share one generic search, so a fix in one mode's
`astar.rs`/`mod.rs` does not propagate to the others:

- `--fixed` (`search/fixed_method/`): classical A* over a fixed decomposition method
  (method choice excluded from the search space). Goal: `is_goal_strong_od`.
- `--fixed-ld`: same A* loop as `--fixed`, goal check swapped to `is_goal_strong_ld`.
- `--flexible` (default, `AOStarSearch`): AO* over the full HTN+FOND space including
  method choice, via `search_graph/`.
- `--andstar-fond` (`search/htn_andstar/`): min-cost search building an explicit
  per-partial-policy Reach graph (`compute_reach`, `partial_policy.rs`); supports
  `TiebreakerKind::PolicySize | ClosureFirst | Combined`.

Shared layers (changes here affect all four modes):

- `domain_description/`: JSON → `FONDProblem`; `classical_domain/` (STRIPS facts/actions)
  + `htn_domain/` (task network: `domain.rs`, `task_defs.rs`, `domain_reader.rs`).
- `task_network/`: `HTN`/`Task`/`CompoundTask`/`Applicability`.
- `relaxation/`: `OutcomeDeterminizer` + `RelaxedComposition` — bridges classical
  heuristics onto the non-deterministic domain; used by `--flexible` and
  `--andstar-fond`.
- `heuristics/`: `h_add`, `h_max`, `h_ff`, `h_lmcut*` over the relaxed domain, plus `TDG`
  and graphplan structures. `HeuristicType` (`search/h_type.rs`) threads
  `--ff|--add|--max|--lmcut` through all four modes;
  `fixed_method::heuristic_factory` adapts it for the classical A* signature.
- `graph_lib/`: generic graph utils incl. VF2 subgraph isomorphism (`vf2.rs`).
- `search/acyclic_plan/`: acyclic policy/search-space representation, shared plumbing
  not a fifth mode.
- `search/search_graph/`: AO* search graph (nodes, connectors) used by `--flexible`.

## Operating rules

- **Repair, don't escalate.** Reversible drift (stale build artifact, dead cache, stale
  test JSON) — fix it directly. Stop only for irreversible actions (force-push, deleting
  data, credential rotation) or genuine ambiguity that changes the outcome. No
  classification report in place of the fix.
- **Verify before claiming.** "Fixed"/"passing"/"landed" requires: re-read the file on
  disk, run the actual command (`cargo build`, `cargo test`, `make`, `./build.sh`) and
  paste real output — pass/fail counts, exit code — not a description of it.
  `git log --oneline -1` + `git status` before claiming a commit exists. No citing
  prior-session state without re-verifying it's still on disk.
- **Git hygiene.** Stage explicit paths, never `git add -A`/`git add .`. `cargo fmt`
  before commit (this repo has no clippy/CI config yet — if one is added, run it too).
  If a build looks stale or flaky, clean `target/` before debugging phantom failures.
- **Distinguish pre-existing vs. introduced.** When reporting build/test status, state
  explicitly which failures predate this session's changes and which didn't.
- **Prior art before invention.** This is already routing known problem classes to
  known formalisms (HTN→HDDL, planning→PDDL-family solver via AO*/AND*, classical
  relaxation→h_add/h_max/h_ff/h_lmcut). Extend that instinct: check
  literature/panda-planner-dev upstream and existing heuristic/search literature before
  hand-rolling a new algorithm variant. Only invent where an exact gap is demonstrated.
- **No unbounded loops.** Any multi-step or looped task here (batch solves, benchmark
  sweeps, multi-domain regression) needs a finite, checkable exit condition — a named
  test count, a specific domain set, a tagged state — stated before starting.
