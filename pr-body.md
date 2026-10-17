The v5 current-support recording changed canonical refutation bytes, but the shared cross-host proof artifact still expected the older layout. Packaged WASM and Python therefore failed on realize. Refresh all16 cases through the actual native producer/checker, preserving every source, question and expected answer byte-for-byte.

Replace the obsolete schema-only test sealer with one explicit Rust refresh operation for the complete artifact. Normal tests stay read-only and keep exact mismatches. Add a portable entry over the same shared production checker, so every vector runs on actual baseline WASM as well as the existing optimized packaged door.

Validation: all16 vectors passed natively and on actual WASM; source/question/answer byte comparison and fixed-point refresh passed; strict affected all-target Clippy and complete helper hygiene passed. Ten proof digests intentionally change for v5, six remain fixed. No runtime decision or expected answer changes. Optimized package/Python hosted acceptance remains pending; this PR does not claim that separate qualification.

Refs #588. Keep the issue open until the actual packaged-host acceptance passes. Review to date is self-review; no independent review is claimed.
