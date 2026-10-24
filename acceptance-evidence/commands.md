# Frozen qualification commands

Commands ran from the issue checkout with the normal Cargo wrapper, eight build
jobs, four test/Rayon threads and the locked dependency graph.

| Output stem | Command / actual result |
| --- | --- |
| openapi-native | cargo test --locked -p purrdf-shapes --lib json_schema::tests::test_openapi_embeds_components_schemas -- --exact; exit 0, one test. |
| schema-native | cargo test --locked -p purrdf-shapes --lib json_schema::tests -- --nocapture; exit 0, 85 tests. |
| golden-generation | PURRDF_WRITE_GOLDEN=1 cargo test --locked -p purrdf-shapes --test ontology_schema_surface iri_only_output_matches_its_golden -- --exact; exit 101 after writing the fixture, comparing with the embedded old fixture. |
| ontology-native | cargo test --locked -p purrdf-shapes --test ontology_schema_surface; exit 0, 29 tests including rebuilt golden comparison. |
| clippy | Initial strict all-target Shapes Clippy: exit 101, five redundant test clones. Fixed before final qualification. |
| clippy-final | cargo clippy --locked -p purrdf-shapes --all-targets -- -D warnings; exit 0. |
| ref-fixture-final | cargo test --locked -p purrdf-shapes --lib json_schema::tests::openapi_relocates_recursive_escaped_refs_without_rewriting_instance_data -- --exact; exit 0, one test after the final clone removal. |
| commit / push | Normal git commit and git push -u origin; exit 0, no bypass. |

The .txt files contain byte-identical settled stdout/stderr. Formatting was
corrected normally; final focused rustfmt and git diff --check returned zero.
