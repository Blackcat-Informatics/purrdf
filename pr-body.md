Triple-only callers can build and retain RDF 1.2 data without a graph coordinate
in their statement tables, rule partitions or text keys. `TripleDataset` stores
12-byte triples, 8-byte reifier rows and 12-byte annotations, using the original
lexical owner and validator. Its explicit fallible read session supports the
existing default-graph query/canonical paths and preserves a typed first cause
when a named-graph operation is requested, including empty and missing-term paths.

The existing Datalog compiler, store, scheduled evaluator, chase and proof homes
now carry a closed static arity. The triple shape uses three positions and a
predicate-only partition key; quad callers retain their original shape and
identities. Public SRL triple inference preserves immutable Base reads and
no-output rules through that shared engine. Graph-free text uses distinct
physical keys and identity preimages while retaining the existing analyzer,
ranking, dictionary and property-function bodies.

The frozen lexical extraction also extends the existing capacity proof census
to its actual shipping IR visibility domain. Exact reviewed sites retain their
reasons and sink identities; changed owner sources, unreviewed sibling writers
and excluded shipping mounts fail closed. This is a source ratchet, not a formal
compiler proof.

Validation draft, not publication readiness:

- Native carrier, typed-refusal, query, rule, canonical and text controls pass.
  The original SRL target passes all 25 cases with compact/quad parity on
  successful fixtures and the original invalid-rule checks retained.
- The official query adapter grades 590 cases with the original comparator and
  records exactly 45 typed named-graph refusals among 635 QueryEval cases. Normal
  replay verifies the complete 1,303-row inventory; independent review checked
  every refusal against its manifest/query source.
- Portable execution passed all 15 controls: core 5/5, Datalog 4/4, query 2/2,
  SRL 2/2 and text 2/2 through the unchanged WASM runner.
- The reviewed proof refresh passes its negative controls, final index and
  unchanged enforced helper gate (82 jobs, 118 distinct-body records). Fixed-work
  layout controls at 1,000/10,000 records retain exactly 32N base bytes and 20N
  lazy-index bytes, with five index allocations at both scales. No timing claim.
  Projections/book checks, settled full gate and hosted integration
  remain required before publication.

Current blockers: the provided managed linker failed the metadata gate after
generator execution; the book suffix was not reached. A final blank-relabeling
neighbor is written in the existing identity test and awaits native/WASM runtime.
No toolchain workaround or host qualification is claimed.

The branch currently depends on unmerged PR #544 at `10316f2c1` and its separately
qualified counted-membership correction. Final source integration and applicable
assembly qualification must be reconciled before delivery.

Closes #492.
