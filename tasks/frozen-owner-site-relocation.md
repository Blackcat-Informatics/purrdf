# Exact reconstruction-site relocation proposal

No ledger edits applied. Matching is reviewed by file, symbol, unchanged literal sink and occurrence order. All40 reasons/classes stay unchanged.

| Existing site | Current site | Literal sink |
| --- | --- | --- |
| crates/rdf-core/src/ir/builder.rs::lookup_hashed:565 | crates/rdf-core/src/ir/builder.rs::lookup_hashed:610 | source:bcb4a119fb003223 |
| crates/rdf-core/src/ir/builder.rs::lookup_owned_blank:591 | crates/rdf-core/src/ir/builder.rs::lookup_owned_blank:636 | source:bcb4a119fb003223 |
| crates/rdf-core/src/ir/builder.rs::transaction_try_with_memory:817 | crates/rdf-core/src/ir/builder.rs::transaction_try_with_memory:862 | source:68501dd74980d201 |
| crates/rdf-core/src/ir/builder.rs::transaction_try_with_memory:817 | crates/rdf-core/src/ir/builder.rs::transaction_try_with_memory:862 | source:469fd0ba6adc3424 |
| crates/rdf-core/src/ir/builder.rs::append_builder_with_memory:2194 | crates/rdf-core/src/ir/builder.rs::append_builder_with_memory:2231 | source:f9e400eaac2b01f1 |
| crates/rdf-core/src/ir/dataset.rs::permutation:1162 | crates/rdf-core/src/ir/dataset.rs::permutation:1199 | source:93359fa72451a90c |
| crates/rdf-core/src/ir/dataset.rs::find_term_hashed:1502 | crates/rdf-core/src/ir/frozen_terms.rs::find_term_hashed:59 | source:bcb4a119fb003223 |
| crates/rdf-core/src/ir/dataset.rs::find_term_hashed:1505 | crates/rdf-core/src/ir/frozen_terms.rs::find_term_hashed:62 | source:bcb4a119fb003223 |
| crates/rdf-core/src/ir/validate.rs::validate_triple_terms_with_memory:453 | crates/rdf-core/src/ir/validate.rs::validate_triple_terms_with_memory:471 | source:55afea662f67f62d |
| crates/rdf-core/src/ir/validate.rs::validate_triple_terms_with_memory:501 | crates/rdf-core/src/ir/validate.rs::validate_triple_terms_with_memory:519 | source:55afea662f67f62d |

The two FrozenTerms rows additionally require the actual new frozen_terms.rs owner authority. All other exact consumers retain their source homes. The authority table is frozen-owner-stale-authorities.md; changed mint/validation/layout source is already part of the compact carrier review and requires explicit applicability to these proofs.
