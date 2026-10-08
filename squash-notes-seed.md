fix(datalog): prioritize variable connectivity in rule plans

<!-- stagectl: replace the TODO lines before merging -->

Closes #479

## What changed

TODO: what this does and why, in prose. The commit subjects below are
      the WHAT; this section is the WHY, and it is the part a reader
      six months from now cannot reconstruct from the diff.

## Commits squashed (7)

- `bb1437bb0` Preserve partial deltas in independent rule factors
- `87d6c3272` test(entail): refresh shared host contracts after evaluator v2
- `9867eba22` perf(datalog): skip impossible initial join variants and qualify rules scaling
- `3ec2dbba8` fix(datalog): share round admission before candidate expansion
- `3912955a5` fix(datalog): canonicalize assumed confirmation witnesses
- `f4873fabc` fix(datalog): factor independent rule bodies with exact witnesses
- `4ea5a0bc3` fix(datalog): prioritize variable connectivity in rule plans

## Files

```
Cargo.lock                                         |    1 +
 crates/cli/Cargo.toml                              |   10 +
 crates/cli/examples/rules_alloc.rs                 |   24 +
 crates/cli/examples/rules_campaign.rs              |  658 +++++++
 crates/datalog/Cargo.toml                          |    7 +
 crates/datalog/src/cache.rs                        |   28 +-
 crates/datalog/src/plan.rs                         |  302 ++-
 crates/datalog/src/schedule.rs                     |  391 +++-
 crates/datalog/src/seminaive.rs                    | 1154 ++++++++++--
 crates/datalog/src/seminaive/factors.rs            | 1964 ++++++++++++++++++++
 crates/datalog/tests/factor_allocation.rs          |   67 +
 crates/datalog/tests/negative_guard_admission.rs   |  117 ++
 crates/datalog/tests/rules_runtime.rs              |  398 ++++
 crates/entail/src/calculus/mod.rs                  |   32 +-
 .../entail/tests/goldens/all_values_filler.golden  |   44 +-
 .../tests/goldens/all_values_instance.golden       |   44 +-
 .../goldens/all_values_instance_near_miss.golden   |   44 +-
 .../tests/goldens/all_values_property.golden       |   44 +-
 crates/entail/tests/goldens/cax_adc_clash.golden   |   44 +-
 .../entail/tests/goldens/cax_adc_consistent.golden |   44 +-
 crates/entail/tests/goldens/cax_dw_clash.golden    |   44 +-
 .../entail/tests/goldens/cax_dw_consistent.golden  |   44 +-
 crates/entail/tests/goldens/class_typed.golden     |   44 +-
 .../tests/goldens/class_typed_near_miss.golden     |   44 +-
 crates/entail/tests/goldens/cls_com_clash.golden   |   44 +-
 .../entail/tests/goldens/cls_com_consistent.golden |   44 +-
 crates/entail/tests/goldens/cls_maxc1_clash.golden |   44 +-
 .../tests/goldens/cls_maxc1_consistent.golden      |   44 +-
 .../entail/tests/goldens/cls_maxqc1_clash.golden   |   44 +-
 .../tests/goldens/cls_maxqc1_consistent.golden     |   44 +-
 .../entail/tests/goldens/cls_maxqc2_clash.golden   |   44 +-
 .../tests/goldens/cls_maxqc2_consistent.golden     |   44 +-
 .../entail/tests/goldens/cls_nothing2_clash.golden |   44 +-
 .../tests/goldens/cls_nothing2_consistent.golden   |   44 +-
 .../tests/goldens/container_membership.golden      |   44 +-
 .../goldens/container_membership_near_miss.golden  |   44 +-
 .../entail/tests/goldens/datatype_declared.golden  |   44 +-
 .../goldens/datatype_declared_near_miss.golden     |   44 +-
 .../entail/tests/goldens/datatype_property.golden  |   44 +-
 .../tests/goldens/datatype_value_equality.golden   |   44 +-
 .../datatype_value_equality_near_miss.golden       |   44 +-
 .../tests/goldens/datatype_value_typing.golden     |   44 +-
 .../goldens/datatype_value_typing_near_miss.golden |   44 +-
 .../tests/goldens/divergence_broad_triggers.golden |   44 +-
 .../goldens/divergence_literal_subject.golden      |   44 +-
 crates/entail/tests/goldens/domain.golden          |   44 +-
 .../entail/tests/goldens/domain_inherited.golden   |   44 +-
 .../entail/tests/goldens/domain_near_miss.golden   |   44 +-
 crates/entail/tests/goldens/domain_widened.golden  |   44 +-
 crates/entail/tests/goldens/dt_diff_clash.golden   |   44 +-
 .../entail/tests/goldens/dt_diff_consistent.golden |   44 +-
 .../entail/tests/goldens/dt_not_type_clash.golden  |   44 +-
 .../tests/goldens/dt_not_type_consistent.golden    |   44 +-
 crates/entail/tests/goldens/empty.golden           |   44 +-
 crates/entail/tests/goldens/eq_diff1_clash.golden  |   44 +-
 .../tests/goldens/eq_diff1_consistent.golden       |   44 +-
 crates/entail/tests/goldens/eq_diff2_clash.golden  |   44 +-
 .../tests/goldens/eq_diff2_consistent.golden       |   44 +-
 crates/entail/tests/goldens/eq_diff3_clash.golden  |   44 +-
 .../tests/goldens/eq_diff3_consistent.golden       |   44 +-
 .../entail/tests/goldens/equivalent_class.golden   |   44 +-
 .../tests/goldens/equivalent_class_instance.golden |   44 +-
 .../equivalent_class_instance_near_miss.golden     |   44 +-
 .../tests/goldens/equivalent_property.golden       |   44 +-
 .../tests/goldens/equivalent_property_data.golden  |   44 +-
 .../equivalent_property_data_near_miss.golden      |   44 +-
 crates/entail/tests/goldens/functional.golden      |   44 +-
 crates/entail/tests/goldens/has_key.golden         |   44 +-
 .../entail/tests/goldens/has_key_near_miss.golden  |   44 +-
 .../entail/tests/goldens/has_value_assert.golden   |   44 +-
 .../tests/goldens/has_value_near_miss.golden       |   44 +-
 .../tests/goldens/has_value_recognize.golden       |   44 +-
 .../tests/goldens/has_value_restrictions.golden    |   44 +-
 .../has_value_restrictions_near_miss.golden        |   44 +-
 .../tests/goldens/intersection_instance.golden     |   44 +-
 .../goldens/intersection_instance_near_miss.golden |   44 +-
 .../goldens/intersection_member_typing.golden      |   44 +-
 crates/entail/tests/goldens/intersection_of.golden |   44 +-
 .../entail/tests/goldens/inverse_functional.golden |   44 +-
 crates/entail/tests/goldens/inverse_pair.golden    |   44 +-
 .../tests/goldens/inverse_pair_near_miss.golden    |   44 +-
 .../tests/goldens/max_cardinality_one.golden       |   44 +-
 .../goldens/max_cardinality_one_near_miss.golden   |   44 +-
 .../entail/tests/goldens/max_qualified_one.golden  |   44 +-
 .../goldens/max_qualified_one_near_miss.golden     |   44 +-
 .../tests/goldens/max_qualified_one_thing.golden   |   44 +-
 crates/entail/tests/goldens/mutual_subclass.golden |   44 +-
 .../entail/tests/goldens/mutual_subproperty.golden |   44 +-
 crates/entail/tests/goldens/named_graph.golden     |   44 +-
 .../tests/goldens/named_graph_closure.golden       |   44 +-
 .../goldens/named_graph_closure_near_miss.golden   |   44 +-
 crates/entail/tests/goldens/object_property.golden |   44 +-
 crates/entail/tests/goldens/one_of.golden          |   44 +-
 .../entail/tests/goldens/one_of_near_miss.golden   |   44 +-
 crates/entail/tests/goldens/owl_class.golden       |   44 +-
 crates/entail/tests/goldens/plain_triple.golden    |   44 +-
 crates/entail/tests/goldens/property_chain.golden  |   44 +-
 .../tests/goldens/property_chain_near_miss.golden  |   44 +-
 crates/entail/tests/goldens/property_typed.golden  |   44 +-
 .../tests/goldens/property_typed_near_miss.golden  |   44 +-
 crates/entail/tests/goldens/prp_adp_clash.golden   |   44 +-
 .../entail/tests/goldens/prp_adp_consistent.golden |   44 +-
 crates/entail/tests/goldens/prp_asyp_clash.golden  |   44 +-
 .../tests/goldens/prp_asyp_consistent.golden       |   44 +-
 crates/entail/tests/goldens/prp_irp_clash.golden   |   44 +-
 .../entail/tests/goldens/prp_irp_consistent.golden |   44 +-
 crates/entail/tests/goldens/prp_npa1_clash.golden  |   44 +-
 .../tests/goldens/prp_npa1_consistent.golden       |   44 +-
 crates/entail/tests/goldens/prp_npa2_clash.golden  |   44 +-
 .../tests/goldens/prp_npa2_consistent.golden       |   44 +-
 crates/entail/tests/goldens/prp_pdw_clash.golden   |   44 +-
 .../entail/tests/goldens/prp_pdw_consistent.golden |   44 +-
 crates/entail/tests/goldens/range.golden           |   44 +-
 crates/entail/tests/goldens/range_inherited.golden |   44 +-
 crates/entail/tests/goldens/range_near_miss.golden |   44 +-
 crates/entail/tests/goldens/range_widened.golden   |   44 +-
 .../tests/goldens/reifies_as_domain_class.golden   |   44 +-
 .../tests/goldens/reifies_as_range_class.golden    |   44 +-
 crates/entail/tests/goldens/reifies_domain.golden  |   44 +-
 .../tests/goldens/reifies_domain_near_miss.golden  |   44 +-
 .../tests/goldens/reifies_domain_widened.golden    |   44 +-
 .../goldens/reifies_inside_triple_term.golden      |   44 +-
 .../reifies_inside_triple_term_near_miss.golden    |   44 +-
 .../tests/goldens/reifies_object_position.golden   |   44 +-
 crates/entail/tests/goldens/reifies_range.golden   |   44 +-
 .../tests/goldens/reifies_range_near_miss.golden   |   44 +-
 .../tests/goldens/reifies_range_widened.golden     |   44 +-
 .../tests/goldens/reifies_subject_position.golden  |   44 +-
 .../tests/goldens/reifies_subproperty.golden       |   44 +-
 .../goldens/reifies_subproperty_near_miss.golden   |   44 +-
 crates/entail/tests/goldens/same_as.golden         |   44 +-
 crates/entail/tests/goldens/same_as_chain.golden   |   44 +-
 .../tests/goldens/same_as_chain_near_miss.golden   |   44 +-
 crates/entail/tests/goldens/same_as_object.golden  |   44 +-
 .../entail/tests/goldens/same_as_predicate.golden  |   44 +-
 crates/entail/tests/goldens/same_as_subject.golden |   44 +-
 .../entail/tests/goldens/shared_conclusion.golden  |   44 +-
 .../entail/tests/goldens/some_values_filler.golden |   44 +-
 .../tests/goldens/some_values_instance.golden      |   44 +-
 .../goldens/some_values_instance_near_miss.golden  |   44 +-
 .../tests/goldens/some_values_property.golden      |   44 +-
 .../entail/tests/goldens/some_values_thing.golden  |   44 +-
 crates/entail/tests/goldens/subclass_chain.golden  |   44 +-
 .../tests/goldens/subclass_chain_near_miss.golden  |   44 +-
 .../entail/tests/goldens/subclass_instance.golden  |   44 +-
 .../goldens/subclass_instance_near_miss.golden     |   44 +-
 .../entail/tests/goldens/subproperty_chain.golden  |   44 +-
 .../goldens/subproperty_chain_near_miss.golden     |   44 +-
 .../tests/goldens/subproperty_rewrite.golden       |   44 +-
 .../goldens/subproperty_rewrite_near_miss.golden   |   44 +-
 crates/entail/tests/goldens/symmetric.golden       |   44 +-
 crates/entail/tests/goldens/transitive.golden      |   44 +-
 crates/entail/tests/goldens/triple_term.golden     |   44 +-
 crates/entail/tests/goldens/union_instance.golden  |   44 +-
 .../tests/goldens/union_instance_near_miss.golden  |   44 +-
 crates/entail/tests/goldens/union_of.golden        |   44 +-
 crates/entail/tests/oracle.rs                      |   35 +
 crates/shapes/tests/rules_engine.rs                |   22 +
 crates/shapes/tests/srl_language.rs                |   51 +
 .../goldens/entailment/disjointclasses-001.report  |    4 +-
 .../tests/owl2_rl_conformance.rs                   |    3 +-
 .../tests/fixtures/regime-boundary.vectors         |   42 +-
 162 files changed, 10320 insertions(+), 1238 deletions(-)
```

## Conflicts

TODO: none, or how each was resolved and why.

---

Defect-Class: TODO
# One of the repo's named classes, or `none` for new work. This becomes
# a GhpRsq-Defect-Class trailer, and recurrence is COUNTED from those
# trailers - `prior-art.md` reports 'uncountable' for a repo that has
# none, which is the current state here. A class written once is worth
# more than a paragraph explaining the same thing three merges later.

