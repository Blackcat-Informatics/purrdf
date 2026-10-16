# Snapshot statistics consumed by planning and rules

Deliver the entire issue: predicate/class frequencies, positional distinct
counts, selected joint frequencies, statement-layer counts, deterministic
snapshot provenance and exact-versus-upper-bound evidence. The SPARQL planner
and native rule-order caller must consume this seam; an unused statistics API
is not delivery. Counts add only over disjoint effective rows. Distinct domains
merge by sorted identity union, never by summing their sizes.

The checkout starts at the already published reifier-index implementation while
its PR finishes CI. This is an implementation dependency, not an extra copy of
that delivery. After it lands, synchronize normally with main and assess the
actual integration delta. Main remains protected. Use no subagents, added runtime
dependency, new custom compiler/cache machinery or timing acceptance. Retain
standard directories and bounded builds. Owner review is not independent approval.

## Task 1: Shared statistics contract and real storage providers

Put the typed count/provenance and provider seam in core, the common dependency
of the evaluator and rule consumers. Unknown distinct counts remain unknown;
upper bounds never become exact by addition. Reuse existing physical index,
PageSummary and segmented-summary homes and original allocation admission.
Cover predicate row/subject/object statistics, class frequencies, annotation
predicates, reifier target predicates and selected object-kind/datatype/value
frequencies. Bind every provider to immutable source/revision identity. Preserve
first operational errors and snapshot prefixes. Compile coherent production
slices as they are adapted; no hundreds-file uncompiled migration.

## Task 2: Estimator, caller-supplied provider and rule consumer

Replace global-term join selectivity where predicate-specific domains exist;
include structural triple components and statement streams. Use the same core
seam for rule connected-order tie-breaking. Expose actual exactness/source in
EXPLAIN and include changed provider identity in retained-plan cache inputs.
A real consumer-supplied provider must change an actual plan. Keep cost units,
output certificates and statistical estimates distinct. Preserve RDF answers,
graphs, multiplicity and original resource/error semantics.

## Task 3: Acceptance and protected delivery

Execute all five issue checks: actual million reified claims with estimate within
2x; paged plan change from reifier-count changes alone; rare-class-first native
and paged plans; supplied provider changes plan with exactness visible; property
merge parity between two segments and one combined segment. Include original
allocation/capacity, snapshot, portable and affected conformance controls.
Use layout/allocation/operation/instruction evidence, never elapsed time.
Run the single required full final gate after source settles. Resolve actual
material forge review and hosted checks, merge through ghprsq, verify closure,
evidence archive and mirror, then remove this delivery branch and update the
root tracker. No foundation-only PR or new follow-up discharges this contract.
