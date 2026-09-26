# Differential oracle

Koala's current raw-HDDL path is intentionally preserved as an oracle while the
frontend is migrated toward Rust.

```text
HDDL
  -> PANDA parser
  -> PANDA grounder
  -> Python serializer/FOND merger
  -> grounded JSON  -----------+
                               |
                               v
                         oracle compare
                               ^
                               |
                 candidate grounded JSON
```

The first admitted comparison surface is:

```text
koala oracle compare <legacy.json> <candidate.json>
```

It parses both artifacts as JSON and reports semantic equality. JSON object key
order is ignored. Array order is preserved because state-feature/task indices
can make array order semantically significant.

## Migration ladder

1. Preserve the existing pipeline unchanged.
2. Replace Python serialization with a Rust candidate.
3. Require `oracle compare` equality across admitted fixtures.
4. Run both grounded artifacts through the same typed planner core and compare
   solve standing/policy semantics.
5. Only after serialization is qualified, replace parsing/grounding behind the
   same oracle boundary.
6. Delete a legacy component only after its replacement is ALIVE on the same
   subject set.

## Standing law

A candidate frontend is not ALIVE because it parses a sample or because its
output looks plausible.

For every admitted fixture `f`:

```text
GroundedLegacy(f) ~= GroundedCandidate(f)
```

and, where multiple byte-distinct but semantically equivalent grounded forms are
allowed:

```text
Solve(GroundedLegacy(f)) ~= Solve(GroundedCandidate(f))
```

A mismatch is topology: it identifies the exact migration edge that remains
open. It is not permission to weaken the oracle.
