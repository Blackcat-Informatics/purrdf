# Evaluator demonstration contract

Initial preparation used source c1d5bcd259089769e3679c6e1926b5ce24fbc1c2.
Task 2 implementation starts at signed/hooked base synchronization commit
331c44e93aa6bcf8434dd027c4e0aae2b5b939b9, incorporating origin/main
b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac. Task 1 core source hashes are
unchanged. raw/T2-base-evaluator.patch retains the five changed evaluator files;
current language equality/query-option routing must be used in the demonstration.
No Task 2 demonstration has run yet.

Use NativeSparqlEngine::query_fallible_view and the prepared counterpart directly
over a retained PagedStackSnapshot's fresh PagedStackQueryView. The evaluator's
CompleteSparqlResult.result contains ordinary dataset-independent values, so
solutions can be compared by variable list and sorted Vec rows (retaining bags).
ASK compares booleans; CONSTRUCT compares actual canonical PackBuilder bytes or
typed dataset-value rows. Operational errors must remain the Operational variant
and cannot carry a CompleteSparqlResult. Reuse existing support::solutions and
public term_fixture homes; do not copy test fixture jobs.

## Independent oracle

Construct an explicit expected value-row set independent of stack mutation and
shadow code. Test ordinary query parity first using RdfDatasetBuilder directly
from a BTreeSet of (subject, predicate, object, graph) tuples. Apply value-set
insert/remove operations to that set as the stack is mutated. Typed RDF 1.2
fixtures explicitly build expected reifier/annotation/ordinary tables with
public builder methods, including graph-scoped promotion after removing the last
declaration. Do not obtain the oracle by draining the stack itself. For the
compaction identity the oracle enters the same declared canonical sealing law,
but must obtain its effective input independently.

## Ordinary layered fixture

Use independently sealed bases with colliding numerical term IDs/page ordinals,
an identical fact in two older generations, and cross-base joins. Suggested rows:
alice knows bob; bob knows carol; bob name Bob and age 25; carol name Carol and
age 17; an independent graph holds a separate value. First delta removes Bob's
old name and Carol's old age, adds name Robert and age 18, and introduces dave.
Second delta removes the repeated alice-knows-bob fact across both lower bases
and adds alice-knows-dave plus Dave's name/age. Head reinserts alice-knows-bob.
After each mutation/seal, compare exact effective rows and query bags. Assert
nonempty witnesses so an accidentally empty fixture cannot pass equivalence.

## Query matrix

| Family | Concrete input shape | Required observation |
| --- | --- | --- |
| Cross-layer BGP/FILTER | ?s knows ?o . ?o name ?n . ?o age ?a FILTER(?a >= 18) | Shared values join across dictionaries and tombstoned old names/ages disappear |
| OPTIONAL | ?s knows ?o OPTIONAL { ?o name ?n } | Missing names produce correctly unbound cells, not missing rows |
| UNION/DISTINCT | Two arms containing a repeated edge and another predicate | Bag multiplicity matches the eager oracle; DISTINCT removes only query duplicates |
| MINUS/NOT EXISTS | Edge filtered by absence of a name/removal marker | Removed facts affect non-monotone consumers correctly |
| Aggregate/subquery | COUNT with GROUP BY; nested grouped SELECT | Repeated physical copies count as one effective fact, query bags remain correct |
| Property path | alice knows+ ?o across different layers | Closure changes after retractions/reinsertion and matches the oracle |
| GRAPH variable/constant | GRAPH ?g { ?s ?p ?o }; GRAPH <g> {} | Delta-only graph identity and declaration-only graph membership agree |
| ASK | Present, tombstoned, then reinserted exact edge | True/false chronology survives seal boundaries |
| CONSTRUCT | Known edge join projected to new predicate | Emitted graph values equal eager evaluation |
| Prepared path | Same prepared BGP/ASK passed to query_prepared_fallible_view | Prepared consumers preserve the same identity and completeness boundary |
| Ordered slices | ORDER BY projected value with LIMIT and OFFSET | The same defined result subsequence survives different physical histories |
| RDF 1.2 | Triple terms, graph-scoped rdf:reifies and annotations | Stream classification and nested values survive removals and later declarations |

## Operational/evidence demonstrations

Use a counted controllable provider with atomic generation/page count/fault state.
Keep source and page ordinals in receipts distinguishable; exact totals span every
layer and the documented resident-head charge. Zero and inclusive limits must be
observed through the public query result boundary. A page over either limit is
requested but not consumed/materialized. Repeated main/side probes of an admitted
page do not charge it again. A constants-only query and a head-only query must
still detect drift in a source they did not read. Inject provider, cancellation,
deadline and invalid-content outcomes; no complete answer or compacted artifact
may escape. Once one source fails, resident-head rows and graph metadata yield
no later data. Use the final operation checkpoint, not iterator exhaustion.

## Runnable consumer and qualification

The existing umbrella API promises that consumers need no direct purrdf-core
dependency. crates/rdf/src/lib.rs selectively reexports the paging surface, so
add the new public stack/canonical symbols there and exercise them through the
existing crates/purrdf/src/lib.rs facade smoke test. Run the targeted facade test,
include these two changed packages in clippy, and build the umbrella on wasm32
to prove the supported consumer reaches the implementation. This is production
wiring acceptance in Task 2, not a new dependency or alternative engine.

Add an example to purrdf-sparql-eval that creates independently sealed layers,
mutates and seals the head, retains an old snapshot, runs public fallible queries,
prints asserted row/evidence observations, compacts, and compares actual ordered
page PackBuilder bytes with an independently built eager oracle. The example
must consume the shipped API directly and fail on a mismatched result.
Run the example, the new evaluator test target and affected existing paged/delta
targets; affected-package clippy; wasm32-unknown-unknown builds for core/evaluator.
Bind commands, exits, outputs and tested tree/toolchain/profile to validation.md.
No check result is established by this preparation document.
