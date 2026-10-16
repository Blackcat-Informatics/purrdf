# Generated query trace disposition

The unchanged Rust writer regenerated the complete query trace, exit 0. All
712 case headers remain. The entire generated diff was inspected: only list-4,
paper-sparqldl-Q2 and paper-sparqldl-Q3 change. Complete solution variables,
canonical RDF answer lines, row counts and peak cells are unchanged in every
case. No input, expected answer, corpus floor, charge point or comparison rule
was changed.

These structural/list BGPs choose a different order under the distinct-aware
cost model. Actual BGP candidate charges rise from 8 to 10, 7 to 12 and 8 to 11;
total fuel is respectively 14, 16 and 17. This is a real small-query work
increase, not a claimed universal planner win. The new provider supplies better
column information, not exact arbitrary join correlations. Profile 20 identifies
the changed ordering/charge behavior; the point schedule itself is unchanged.
The complete-million acceptance remains scoped evidence for its actual workload.

Fuel-sweep cut points move with those executed candidate charges. Q3 also
observes a one-row certain prefix before its complete two-row result. The
complete answer digests and eventual completion are unchanged. The frozen
trace continues to assert every ceiling and its actual partial rows rather
than dropping the moved cases or weakening comparisons.

The standard Cargo-produced current evaluator_trace executable reran the normal
query assertion against the generated file, exit 0, with the original optimized
profile and assertions/overflow checks. It reads the golden at runtime, so this
is not an old embedded expected value. The run used one test thread and a 1 GiB
process address-space cap while host memory was low; full cases and ceilings
were retained. Logs: review-trace-regenerate.log and review-trace-assert.log.
Elapsed libtest output is not performance evidence.
