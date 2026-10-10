# Native traversal owner implementation

The active delivery remains incomplete and uncommitted. This packet records
concrete implementation work for the sole shipping writer, not acceptance.

## Delivered source units

- `filter-bind-owner-draft.patch`: current-source apply check passed; writer
  reports integration. Bounded execution uses admitted schemas, linked VM,
  retained row construction and bag metadata. Resident parallel execution
  preserves source-order commits and its original metadata buffer; BNODE row
  identity is unchanged. No root build or semantic test was run.
- `native-heap-owner-draft.patch`: current-source apply check passed. Shared
  `AdmittedHeap` uses std BinaryHeap and the existing physical vector layout.
  Replacement retains both old and new arrays until transfer completes; sorted
  output carries the same grant. It does not clone element payloads.
- `native-heap-knn-caller-draft.patch`: current-source apply check passed. Both
  streaming resident selection and admitted selection use one rank-selection
  body; native capacity stays min(k, actual candidate count).
- `native-table-owner-draft.patch`: current-source apply check passed. Shared
  `AdmittedMap` uses the pinned hashbrown physical layout certificate, borrowed
  lookup and fallible replacement. Draining moves each payload once. ParsedCache
  is migrated to that home, retaining first-cache-entry semantics and distinguishing
  cached None from a miss. hashbrown 0.17.1 find_mut/try_reserve/drain APIs were
  inspected locally. Heap and table export hunks need reconciliation if combined.

Apply checks establish patch applicability only. Native integration compilation,
meaningful ordering/parity and allocation/failure/lifetime tests are still required.
Subsequent source inspection confirms AdmittedMap and AdmittedHeap now exist in
workspace.rs, RankHeap is wired in knn/metric.rs, and the FILTER/BIND resident
transfer homes exist in solution.rs. These are integrated unqualified source.

## HNSW next implementation

Keep the existing arithmetic dispatch, adjacency batch scoring, two heaps,
Ranked tie order, ef stopping rule and evaluation counts. Thread the execution
capability through the actual query, cache and scratch owners. Use the shared
sparse table for row-distance memoization, admitted vectors for batches/stamps
and the shared heap for both candidate and result arrays. Retain result admission
through the ranked cursor. Do not replace sparse memoization with dense-N cache
storage or linear searches as an ownership shortcut.

The current index scratch pool accepts newly allocated Visited buffers after a
query. Bounded query-created storage must remain query-owned and die with its
grant, rather than entering that caller-owned pool after releasing its charge.
Resident pooled scratch remains an implementation option under the same search
body. Native property invocation/cursor factories also need a public owner-safe
bridge from the evaluator's currently crate-private admitted construction.

## Regex source findings: required work

`expr::cached_regex` owns copied pattern/flags in nested maps and an Arc of
`xsd_regex::CompiledPattern`. `xsd_regex::compile` translates before entering
external RegexBuilder. CompiledPattern also contains OnceLock engine/prefilter
state, deferred source and lazily allocated matcher caches. A size_limit or
precharged guessed envelope is not proof of actual allocation ownership or a
typed allocator-failure path.

The native dated XPath compiler has fallible reserves and finite slot quotas,
but `xpath::Budget` stores counters only. `compile::owned` and `grow<T>` know
their concrete storage; their returned String/Vec owners do not retain query
byte-account grants. Matcher/replacement returned owners need the same treatment.
Parser::peek also formats scanner errors into an unowned String before creating
a syntax error. A quota-only Budget grant cannot cover programs after Budget dies.

Unselected compatibility matching and explicitly dated XPath matching have
documented different case and anchor behavior. Switching a bounded compatibility
query to a dated profile is not a valid implementation. Complete supported REGEX
requires an actual owner-safe compatibility implementation or a proven allocation
hook covering construction and lazy matching without changing the selected law.
An opaque-provider refusal does not close acceptance for this native supported
builtin. This remains required implementation work, not an authorized deferral.
