Columnar integrity and RDF reconstruction failures previously rendered native
errors into `Malformed` strings or extracted storage errors while discarding
their enclosing diagnostic. They now move the original causes into
`ColumnarError::ContentStore`, `BlobContent { row, cause }` and
`Reconstruction(RdfDiagnostic)`. The standard source chain and structured visitor
borrow those original objects, including complete digest bytes, zero-based rows
and diagnostic attachments. Human message prefixes and successful Parquet bytes
remain unchanged; existing `Clone`/`Eq` traits remain.

Closes #677. This delivers one remaining native-cause unit under #558; the parent
retains its full shipping inventory, other malformed/UTF-8 producers and host
acceptance. The existing generic source-session transport remains at its merged
home. No donor replay, new hashing/rendering owner, dependency or feature change.

Validation covers actual core insertion and integrity verification; public
corrupt-blob and invalid-RDF reads after input disposal; full-width fields;
original standard-error downcasts; storage diagnostics with detail/location and a
native attachment whose owner is released with the error; and zero allocations,
requested bytes, retained bytes and peak working bytes for transport/recursive
inspection. Safe public insertion cannot create a corrupt global store, so that
actual verifier is exercised at its private owner rather than through a new test
seam. The complete native columnar package and actual packaged-WASM inspection
and frozen-byte tests pass. Seven core integrity tests, 31 columnar library cases,
six native and six actual packaged-WASM inspection cases, and ten frozen file
digests pass. Affected all-target denied-warning Clippy, strict rustdoc, complete
helper/domain hygiene, production facade/Python library compilation and prose/CI
route checks pass. Production consumer compilation is not host runtime acceptance.

The conservative core constructor closure pins whole source files containing
opaque macros/derives. Reporting additions in `content_store.rs` invalidate its
identity. The 40 affected proof records refresh only that closure pin after
checking unchanged constructor membership, every original sink and all capacity
authority identities against main. No capacity exemption or gate rule changes.

Independent review of all ten files found one migration omission: former
storage-bearing freeze failures also require updating `Storage` matches to
`Reconstruction`. The guide now states that explicitly, and the correction is
independently confirmed. All 50 hosted checks pass with five configured skips;
the final review thread is resolved. The actual CI merge and current candidate
have the same base, head and tree.
