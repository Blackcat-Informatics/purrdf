# Native exhaustive kNN allocation ownership

This proposal changes the existing exhaustive search and heap selection bodies;
it does not qualify the producer or close its acceptance requirements.

- Admit the actual N-element distance and scored buffers before allocation.
- Retain the distance iterator's allocation while scored rows are constructed.
- Admit min(k, N) heap entries while scored rows remain live. Selection uses one
  shared ordering body and converts the heap to its sorted Vec without allocating.
- Retain that grant with the sorted result via the existing admitted-vector home.
- Keep the resident search signature for existing callers and tests; it delegates
  to the same body under a resident capability. No extra numerical implementation.
- Checked layout/count overflow and fallible buffer allocation use existing typed
  operational errors. No arbitrary k-sized preallocation for resident best().

Required next integration: route the concrete Scan and RankSource dispatch through
search_admitted, retain the ranked owner in RankedCursor, admit invocation copies
before cloning, and use the admitted row factory with the native inline IEEE text
renderer. HNSW requires its own actual traversal owners; an opaque-producer refusal
does not prove that native index producer supports bounded queries.

The finite-distance diagnostic still constructs a dynamic String on the error
path and must gain actual diagnostic ownership before acceptance. Relation cursor
boxing, query/count parsing and membership-row buffers also need their actual
owners. Independent low-capacity, real allocation-failure, cancellation, retained
result/drop and resident/bounded semantic checks remain unrun. Sole shipping
writer performs source integration and grouped qualification.
