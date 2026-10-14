# Native nesting certificate: independent source review

Verdict: PASS for the bounded dataset_view.rs and segmented/query/view.rs declaration change. Compiler/runtime acceptance and full S1 completion remain separate.

Read the actual two-file diff and original admission homes. The crate-private native_term_nesting_bound declaration replaces exactly the two identical constant-return methods for RdfDataset and SegmentedStackQuery. The native frozen dataset documentation and inline attribute remain attached to that implementation; the query's method has its original signature and constant. No trait default or other view certificate changes.

The certificate applies to these carriers: native datasets pass the original validated freeze path; segmented construction freezes the same native builder, and encoded validation checks expanded term depth and freezes its reconstructed builder before issuing the accepted receipt. Stack heads, removal carriers and declaration carriers own immutable RdfDataset values, not arbitrary DatasetView providers. Query-local ID translation does not construct a deeper term. Thus the declaration introduces neither a new ceiling nor a certificate for an unvalidated foreign view; it retains the existing MAX_TERM_NESTING_DEPTH promise at both original implementations.

The shared macro remains crate-private in the DatasetView home, with one method body and two declarations. There is no new scan, allocation, public API, wire change, dependency or tooling/ledger edit. No material source finding was identified. This reviewer started no build; strict57680 was pending when requested.
