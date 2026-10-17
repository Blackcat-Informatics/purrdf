# Capacity source receipt renewal

The first hosted workspace check failed on 40 stale capacity proof records.
The checker deliberately pins full source files and the constructor visibility
closure. The API and tests change dataset.rs, canon.rs and pipeline_bundle.rs,
so the constructor identity changes even though no logical-capacity limit,
allocation admission, mint or frozen-table invariant changed. The permutation
sink also moved from line 1298 to line 1308; its body and sink identity are
unchanged.

Reviewed changes against the base: graph selection still constructs through
the original RdfDatasetBuilder and freeze path, propagates push/freeze failures,
and falls through the existing owned_snapshot path only when Arc unwrapping
cannot consume the frozen result. The new default and blank selectors change
only row membership. They introduce no unchecked constructor, narrowing mint,
capacity panic, raised ceiling or bypass. Canonicalization and bundle callers
only replace the old named selection with the typed IRI selector. Tests add
tiny graph fixtures, not alternate constructors in shipping code.

The native helper-census --index result is retained in capacity-source-index.json.
Its relevant constructor/source fields are projected into
capacity-source-evidence.json. It computes dataset identity
source:5bc0a9477986586c and constructor identity source:3d088bde0e0c4767.
The ledger renews these source receipts and that one moved line address only.
Proof classes, sinks, reasons, required authority sets, capacity limits and
checker implementation remain unchanged. The complete helpers-hygiene gate
must pass before this repair is claimed qualified.

The first local renewal attempt used the function declaration's line 1307;
the checker correctly refused it and reported the actual sink at line 1308.
The final receipt uses that observed sink address. The failed attempt remains
in capacity-receipt-hygiene.log.

The already-passed 1459 kernel and consumer tests still cover the unchanged
Rust implementation. This evidence-only ledger repair needs the actual checker
and normal hooks, not a duplicate source test run or a weakened gate.
