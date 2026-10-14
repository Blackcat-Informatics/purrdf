# Borrowed error-source sharing: independent source review

Verdict: PASS for the bounded seven-file production-source delta. This is not full S1 acceptance or a runtime/census pass.

Reviewed the actual diff in `crates/lex/src/message_error.rs` and the six replacement declarations in columnar/error.rs, conformance-kit/lib.rs, rdf-core/ir/segmented/mod.rs, datalog/chase.rs, validate/expr_selector.rs and xsd/exact/error.rs.

The existing lexical error-macro home now contains one `error_source!` implementation. Every current invocation selects the same original single-field Storage variant. Expansion implements the same Error trait (core::error::Error is the trait re-exported by std), borrows the original field with the receiver lifetime and exposes its original concrete type through the trait object. All other variants return None as before. There is no clone, rendering, allocation, replacement cause or change to Display/Debug/From behavior. The six handwritten bodies are removed; callers contain declarations rather than alternate implementations.

The macro remains exported from the existing private module through macro_export, with no new dependency, module graph, feature or tool machinery. Its inputs name an existing enum type and one or more wrapper variants; current callers already supply the necessary Debug/Display and static error field. The example exercises typed downcast of the original StorageError and the non-wrapper None case. No material source finding was identified. Final compiler/doctest and unchanged-census receipts belong to the executing owner and are still required; this reviewer started no build.

Reviewed independently while the separate remembered-graph full gate remained active. No source edits or sibling process changes were made by this reviewer.
