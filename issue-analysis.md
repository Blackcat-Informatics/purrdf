# Shared boundary regression

The prior ordinary-clash implementation changed proof encoding to v5 and strengthened replay classification. The16-case shared purrdf-validate artifact still expected older v3/v4 bytes for affected refutations. Full logs identify the same realize mismatch in packaged WASM and three Python/conformance lanes. The prior entailment-only tests were too narrow to qualify this artifact. Existing shared consumers and exact mismatch checks must remain intact.

Plan task1 owns one explicit Rust refresh operation and exact preservation of source/questions/answers. Task2 owns the native/portable production boundary checker, strict affected checks, hooks, publication and ghprsq. Optimized packaged-host CI is separate and keeps this issue open while pending. GMEOW1843 work remains separately isolated and is resumed after this introduced regression.
