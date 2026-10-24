Canonicalization can refuse reserved vocabulary or an exhausted search budget.
Public identity and comparison callers currently turn some refusals into panics.
Remove the parallel panic API family and propagate the original native error
through carrier/proof identities, dataset comparisons, GTS digests, flat output
and SPARQL conformance. The WASM comparison boundary constructs a JavaScript
Error carrying the original refusal.

This is a v4 API break. Callers, examples, exports and translated guidance use
the fallible homes. Successful canonical bytes, graph labels, digest algorithms
and refusal ordering are preserved. Search budgets/defaults, heap UDF execution
and Datalog policies remain in the parent contract.

Validation includes 294 passing selected native cases and strict checks of the
actual production hosts, 37 integration targets, three benchmarks and the
projection example. One unrelated existing large-compaction case retains its
explicit memory-ceiling requirement. The optimized WASM package and its single
documented JavaScript Error case pass; general semantics stay native.

The full helper hygiene and translated-book gates pass. Capacity authority and
constructor pins are rebound after reviewing unchanged minting, bounds,
ownership and original sinks. Normal commit/push hooks pass. Independent review
findings are resolved. All required final-head hosted checks pass: 50 successful
checks and six skipped jobs, with no remaining blocker or review debt.

Closes #704. Parent #489 remains open for its remaining complete requirements.
