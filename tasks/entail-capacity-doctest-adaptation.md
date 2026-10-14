# Entailment capacity example adaptation

The twelve originally failed examples in eight entailment source files now propagate existing builder StorageError results through their existing fixtures and a hidden typed Ok return. RDF inputs, reasoning/proof/warrant/module/profile assertions and production bodies are unchanged. Independent source review PASS: reviews/entail-capacity-doctest-review.md.

Direct installed standard Cargo: CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=4 cargo test --locked -j1 -p purrdf-entail --doc, terminal0. All24 doctests PASS, including all12 adapted cases; log entail-capacity-doctest.log. Edition2024 rustfmt and git diff --check PASS. Normal commit hooks terminal0; published48bc028b8. Commit and normal push terminal0 in entail-capacity-doctest-commit.log and entail-capacity-doctest-push.log.

This repairs the final doc-package failure from the original full-gate receipt. That original full gate remains FAILED, not retroactively passed. Segmented residency and the enforced capacity census remain incomplete; no pair completion, PR or issue closure is claimed.
