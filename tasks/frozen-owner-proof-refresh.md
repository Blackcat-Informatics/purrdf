# Frozen-owner proof refresh proposal

No ledger change applied. The final stale capture has 40 errors and 40 exact sinks. The explicit-owner negative seed and warning-denied helper Clippy passed in session73646.

Constructor aggregate: source:49c7c48bf647382f -> source:ce38cd3e1a89d2c7; 156 source files. All 40 classes, reasons and sink literals remain identical. Add explicit frozen_terms.rs=source:6e6f2414525e9c15 to the 34 rows in the four reviewed classes.

| Authority | Previous | Current |
| --- | --- | --- |
| crates/lex/src/allocation.rs | source:5100728db53dae7f | source:bd1cfeddaeae116a |
| crates/rdf-core/src/ir/builder.rs | source:aa47200c5548da15 | source:285ec037c8fdd275 |
| crates/rdf-core/src/ir/dataset.rs | source:328419615aacbd47 | source:0308b0ce5ff24a18 |
| crates/rdf-core/src/ir/validate.rs | source:cc105b93beed4742 | source:bb7eaea66da9a620 |

| Previous site | Current site | Unchanged sink |
| --- | --- | --- |
| crates/rdf-core/src/ir/builder.rs::lookup_hashed:565 | crates/rdf-core/src/ir/builder.rs::lookup_hashed:610 | source:bcb4a119fb003223 |
| crates/rdf-core/src/ir/builder.rs::lookup_owned_blank:591 | crates/rdf-core/src/ir/builder.rs::lookup_owned_blank:636 | source:bcb4a119fb003223 |
| crates/rdf-core/src/ir/builder.rs::transaction_try_with_memory:817 | crates/rdf-core/src/ir/builder.rs::transaction_try_with_memory:862 | source:68501dd74980d201 |
| crates/rdf-core/src/ir/builder.rs::transaction_try_with_memory:817 | crates/rdf-core/src/ir/builder.rs::transaction_try_with_memory:862 | source:469fd0ba6adc3424 |
| crates/rdf-core/src/ir/builder.rs::append_builder_with_memory:2194 | crates/rdf-core/src/ir/builder.rs::append_builder_with_memory:2231 | source:f9e400eaac2b01f1 |
| crates/rdf-core/src/ir/dataset.rs::permutation:1162 | crates/rdf-core/src/ir/dataset.rs::permutation:1213 | source:93359fa72451a90c |
| crates/rdf-core/src/ir/dataset.rs::find_term_hashed:1502 | crates/rdf-core/src/ir/frozen_terms.rs::find_term_hashed:59 | source:bcb4a119fb003223 |
| crates/rdf-core/src/ir/dataset.rs::find_term_hashed:1505 | crates/rdf-core/src/ir/frozen_terms.rs::find_term_hashed:62 | source:bcb4a119fb003223 |
| crates/rdf-core/src/ir/validate.rs::validate_triple_terms_with_memory:453 | crates/rdf-core/src/ir/validate.rs::validate_triple_terms_with_memory:471 | source:55afea662f67f62d |
| crates/rdf-core/src/ir/validate.rs::validate_triple_terms_with_memory:501 | crates/rdf-core/src/ir/validate.rs::validate_triple_terms_with_memory:519 | source:55afea662f67f62d |

The aggregate includes the reviewed full shipping IR visibility domain. Excluded/out-of-package shipping mounts fail before they can be skipped. Since the first stale capture, shared declaration changes affect dataset.rs, triple.rs, triple/read.rs and dataset_view.rs; checker and Text documentation changes are outside this aggregate. The original 40-error capture remains retained. This proposal does not itself establish a passing enforced gate.
