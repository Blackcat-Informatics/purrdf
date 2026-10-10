# Native correlated traffic correction

Status: implementation proposal, UNCOMPILED/UNRUN. This packet changes no evidence capacity, data size, result/physical ownership assertions, producer refusal, or performance assertion.

The current Values-Insertion body preserves a leaf as the left child and adds its singleton restriction on the right. PositivePlan correctly excludes VALUES from a pure region, so ordinary left-first join evaluation scans the unbound leaf for every outer row. The native join now recognizes this exact copied-correlation shape, evaluates the original right singleton as child 1 and passes it into the original positive region as child 0. It uses the original indexed BGP/Join/Union bodies and native owner/checkpoint law. All other joins retain their existing route.

The same finish_seeded_join home serves ordinary seeded joins and this restriction. The correlation branch restores the original syntactic schema through reorder_like_admitted; it does not join the driver back into already-seeded rows or change bag multiplicity. Stateful functions, expression/filter/modifier/scope/SERVICE bodies are excluded by the original seed-eligibility law. Nested NOT EXISTS remains on its actual deferred/native route.

Two independently demonstrated fixture defects are corrected: the SERVICE EXISTS case must introduce a distinct VALUES variable while correlating its FILTER to the existing outer x; physical layout refusal uses the now-public typed ControlFailure::LayoutOverflow, without allocating a diagnostic after refusal. The writer's temporary error Debug probe is removed.

Qualification required: existing correlated_owned_admission all four entry doors/all four result forms (unchanged 16,384 request ceiling and actual counting allocator), existing correlation/Values-Insertion reference and governor gates, existing positive-region bag/scope/copy-count tests. Run in the writer's grouped lane, not this helper. A PASS here cannot be inferred until those real computations finish.
