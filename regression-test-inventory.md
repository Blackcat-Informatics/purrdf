Complete failing shapes-library test inventory from the native run at PR publication (947 passes, 41 failures):

```
engine::complete_reports::rec_core_messages_ignore_draft_reifier_overrides_and_legacy_keeps_them
lint::tests::a_data_graph_import_without_an_ontology_type_is_an_info_finding
lint::tests::an_imported_documents_unanchored_import_names_its_document
lint::tests::an_unanchored_import_is_listed_and_is_never_a_finding
product::tests::admit_restores_box_role_vocab
provenance::tests::provenance_reconstructs_identical_shapes
rules::tests::a_bare_blank_in_a_triple_rule_head_produces_nothing
rules::tests::parses_order_deactivated_and_conditions
schema_import::tests::emitted_logical_shapes_round_trip_byte_exactly
schema_import::tests::range_bounds_round_trip_exactly
shapes::parser::admission::tests::dated_product_sources_reconstruct_without_legacy_readmission_or_model_relinking
shapes::parser::admission::tests::rec_audits_unreached_constraint_declarations_but_not_unrelated_selects
shapes::tests::a_list_cell_typed_rdf_list_still_walks
shapes::tests::a_well_formed_list_walks_in_order
shapes::tests::an_rdf_nil_list_head_is_the_empty_list
shapes::tests::core_prefixes_import_resolves_registry_only_prefixes
shapes::tests::literal_and_triple_term_members_walk
shapes::tests::node_expr_filter_shape
shapes::tests::test_alternative_path_parses
shapes::tests::test_and_constraint
shapes::tests::test_closed_parses
shapes::tests::test_custom_component_constraint_detected
shapes::tests::test_in_list_constraint
shapes::tests::test_inverse_path
shapes::tests::test_language_in_parses
shapes::tests::test_max_length_parses
shapes::tests::test_nested_path_combination_parses
shapes::tests::test_node_kind_iri
shapes::tests::test_not_parses
shapes::tests::test_one_or_more_and_zero_or_one_paths_parse
shapes::tests::test_or_with_two_predicate_path_members
shapes::tests::test_pattern_with_flags
shapes::tests::test_property_pair_constraints_parse
shapes::tests::test_qualified_value_shape_disjoint_collects_siblings
shapes::tests::test_qualified_value_shape_parses
shapes::tests::test_sequence_path_parses
shapes::tests::test_shape_metadata
shapes::tests::test_sparql_constraint_parses
shapes::tests::test_sparql_prefixes_namespace_as_iri
shapes::tests::test_target_class_and_property_min_max_count
shapes::tests::test_zero_or_more_path_parses
```

Core library tests passed all 1398 cases in the same command. This inventory records failures, not waivers.
