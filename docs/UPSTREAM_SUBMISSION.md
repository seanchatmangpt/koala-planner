# Upstream submission readiness

This document tracks the state of the fork's `main`-bound migration work (PRs
#1–#5, this fork: `seanchatmangpt/koala-planner`) and what is left before any
of it is proposed to the upstream project (`koala-planner/Planner`). No PR has
been opened against upstream — this file, and PR #6 that carries it, exist
entirely inside this fork.

## What's staged, in merge order

1. **PR #1 — `refactor/typed-planner-core`** → `main`
   Extracts a reusable typed `solve()` API around the existing AO*/HTN-AND*/
   fixed search implementations. No behavior change to the CLI/Python
   pipeline; purely an internal API surface addition.

2. **PR #2 — `feat/ggen-noun-verb-cli`** → PR #1
   Adds an optional `agent-cli` feature exposing `koala plan solve` via a
   ggen-projected clap-noun-verb surface. Opt-in (feature-gated); the default
   build is unaffected.

3. **PR #3 — `feat/wasm32-wasip1`** → PR #2
   Adds `crates/koala-wasm`, a `wasm32-wasip1` adapter over the typed core
   (`koala_alloc`/`koala_call`/`koala_dealloc`), taking grounded JSON input
   directly (no filesystem, no HDDL parsing in WASM). Explicitly reports
   `select_only: true`.

4. **PR #4 — `feat/differential-oracle`** → PR #3
   Adds `koala oracle compare`, a differential oracle comparing legacy
   Python-serializer output against any candidate Rust-serializer output.
   This is the falsifier that must pass before any Python code path is
   removed.

5. **PR #5 — `refactor/rust-grounded-serializer`** (this stack's tip) → PR #4
   Adds `crates/koala-serializer`, a pure-Rust reimplementation of PANDA's
   11-section grounded serialization format (FOND outcome coalescing,
   probability injection, methods/orderings, conditional-effect
   preservation), wired in as `koala problem serialize`. PANDA's C++
   parser/grounder remain untouched and are still the source of ground truth;
   only the downstream Python serialization step is challenged, and only
   behind the oracle from PR #4.

## Fixes applied while stabilizing the stack (this session)

All three were root-caused at the base of the stack they affect and merged
forward through every downstream branch via ordinary merge commits (no
rebases, no force-pushes), each merge re-verified with a real `cargo test`
run:

- **Missing `linkme` direct dependency** (PR #1/#2): pinning
  `clap-noun-verb`/`clap-noun-verb-macros` to the live `26.9.1` release
  wasn't sufficient — the `#[verb(...)]` macro expands to a
  `#[linkme::distributed_slice]` registration requiring `linkme` as a direct
  dependency of the consuming crate.
- **`rand = "*"` cross-crate resolution divergence** (PR #3): the independent
  `crates/koala-wasm` `Cargo.lock` resolved a different `rand` major version
  than `planner`'s, breaking the `wasm32-wasip1` build. Pinned to `"0.8"`,
  matching the one call site's API usage.
- **`SearchGraph::visited()` HashMap-iteration-order bug** (PR #1, the true
  root): the function only ever examined the *first* entry a `HashMap`
  iterator happened to yield — a real logic bug, not test flakiness — so
  correctness of duplicate-state detection depended on `HashMap`'s per-process
  randomized hash seed. Fixed to scan all entries.

All five PRs are green at their current head SHAs (`gh pr checks`, verified
per-PR, all `qualify` runs SUCCESS) and marked ready for review (draft lifted).
The `qualify` workflow was additionally run end-to-end locally via
`act pull_request` against the real `.github/workflows/rust-qualification.yml`
definition to confirm the fixes hold outside GitHub's own runners.

## Before proposing any of this upstream

- [ ] Maintainer review of PRs #1–#5 within this fork first (this is what
      they're open for).
- [ ] Decide submission granularity with upstream maintainers: one squashed
      PR per stack entry (5 PRs) vs. a single consolidated PR — upstream
      convention should decide this, not assumed here.
- [ ] Confirm upstream CI (if any) covers the same `agent-cli`/`wasm32-wasip1`
      feature combinations this fork's `rust-qualification.yml` exercises, or
      port the workflow alongside the code.
- [ ] Re-run the differential oracle (PR #4) against upstream's current
      Python serializer once rebased onto upstream's `main`, before treating
      PR #5's Rust serializer as a drop-in replacement there.
- [ ] Upstream's own contribution guidelines (CONTRIBUTING.md, issue/PR
      templates) have not been checked yet — do that before opening anything
      against `koala-planner/Planner`.

**No PR has been opened against `koala-planner/Planner`.** This document and
PR #6 are staging inside the fork only.
