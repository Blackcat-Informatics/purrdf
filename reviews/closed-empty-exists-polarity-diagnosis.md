# Completed empty EXISTS lacks its closed-absence proof

Read-only source diagnosis following the reported wrong W3C answer bags. No
build, test, golden regeneration, source edit or process change. Removing the
owner exception exposed a separate semantic error; ownership repair alone does
not qualify negation.

## Concrete source path

`QueryEvidence::empty_existential` constructs uniform Neither for every empty
bag. Its combination with default supported search evidence remains Neither.
`RowEvidence::native_negative` correctly applies the core FOUR negation after
checking closure/completeness; FOUR negation maps Neither to Neither. Explicit
`EvidenceInput::negative_evidence` has the same outcome after its source-scope
guard. The missing step is a proof from **actual complete closed emptiness**, not
a different FOUR truth table.

`vm/mod.rs:672–687` treats the resulting non-Opposed record as a retained true
NOT EXISTS value. `expr::filter_evidence` consequently attaches Neither to the
otherwise supported outer binding. Later `binop::minus_evidence` coalesces those
right-side witnesses, negates them, and removes a matching left row only for
uniform Opposed. Neither remains Neither, so a right-side row which really
exists no longer excludes its compatible left-side row. This explains excess
answers in set-equals-1 and subset-01/02/03, including their nested MINUS forms.
It also threatens sequential/nested negative expressions under actual evidence
execution, independently of the harness's unused registry.

The conformance registry activates evidence handling for these plain RDF queries.
Restricting that activation to reachable declarations may restore their ordinary
path, but cannot substitute for repairing the evidenced closed-absence law.

## Coherent correction boundary

At the point which knows there were **no matching witnesses**, distinguish an
actual complete closed search from incomplete or unclosed search. A licensed
empty existential proves the existential proposition false: represent that
opposition before applying NOT EXISTS, so its negation supplies Supported
evidence to the retained row. Equivalently, construct the supported negative
proof at that same original empty-search boundary, without confusing an arbitrary
Neither-valued claim with absence. Reuse the query owner and record construction
home, retaining search frame, reasons, bounds and membership facts.

For native RDF completion, the original default search is complete and closed.
For native producers, use their retained original search completeness/closure;
cursor exhaustion alone is insufficient. For explicit sources, additionally
require `source.scope(graph).closure_declared` and complete scope, as the existing
`absence_evidence` guard does. If either source or native search is incomplete or
unclosed, retain Neither/undecided and original reasons; do not fabricate a
negative proof. Nonempty explicit Neither, Both, scoped and Opposed witnesses
must keep their established evidence-negation semantics.

Do not change core `Information::negate`, treat every Neither as absence, erase
real native metadata to restore W3C answers, or change MINUS to ignore evidence
universally. These would conceal the concrete missing closed-absence inference.

The existing `native_empty_witness_preserves_search_owner_and_aggregate_closure`
test currently expects Neither for *all* empty-search cases. Its complete and
closed existential-predicate case needs reconciliation with the repaired law;
the incomplete/unclosed cases must remain Neither. A top-level empty solution
bag's information summary is still Neither: no row was observed. Do not change
that separate status law merely to repair a computed EXISTS proposition.

## Required focused evidence after repair

Keep the W3C expectations unchanged: set-equals-1 has 2 answers, subset-01/02
have 11, and subset-03 has 7. Exercise the same nested negative shapes with an
explicit complete/closed source and with a native producer, then their incomplete
and no-closure neighbors. Assert retained successful NOT EXISTS has support and
can exclude through an outer MINUS; open/incomplete emptiness remains unknown
and preserves reasons. Preserve nonempty Both/scoped polarity controls and
ordinary query controls. Existing trace goldens remain unaccepted until semantic
failures are corrected and the full changed traces are understood.
