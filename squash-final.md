Use snapshot statistics in query and rule planning

The planner previously estimated joins from constant-only row counts and one
global distinct-term count, missing RDF 1.2 reifier and annotation populations.
Snapshot-bound predicate/class counts, positional distinct domains, selected
value frequencies and statement counts now drive SPARQL ordering/forecasts and
native rule ordering. EXPLAIN reports source, revision and exact, upper-bound or
unknown evidence. Consumer-supplied providers use the same seam.

Core owns the metadata, sorted-domain codec and native/paged/persisted providers.
Physical segment domains merge after RDF-value translation. Chronological views
retain conservative physical bounds rather than adding distinct counts or
claiming exact effective-set statistics. Original allocation, header-only open,
snapshot and first-fault ownership are preserved. Estimates remain distinct from
output certificates and memory admission.

All five issue demonstrations pass. The actual million-reifier/six-million-
annotation query estimates and returns one million rows with two million examined
candidates. Native/paged rare-class ordering, reifier-only metadata plan changes,
supplied-provider EXPLAIN and two physical segments versus one combined image
are exercised. Rule derivations, full-width identity, allocation/refund, original
thousand-source open and shallow historical ownership remain covered.

Governor profile 20 records changed planning charges. All 15 normative controls,
712 query traces, 21 completion controls, exact SHACL allocation, 37 integrated
stack controls, three scalar WASM runtime controls and 154 manual claims pass
within the source scopes indexed in validation.md. Three trace charges change;
complete answers and peak cells remain unchanged. Current hosted CI supplies the
full final gate: 49 checks succeed and four optional jobs skip, including passing
workspace/test aggregates, all seven assembly configurations, conformance,
portable execution, host packages and architecture checks. No timing acceptance
or duplicate local full-suite pass is claimed.

Both material hosted review threads are resolved. Metadata readiness is fixed
and confirmed by the reviewer; the reviewer withdrew the replay proposal after
the original relation-local admission law and seven-answer/140-cell control were
explained. Owner completeness assessment and actual hosted review remain distinct.
There is no unresolved material finding or scope cut.

Main integration preserves both sides of the paged-stack test and proof-ledger
conflicts: landed primary-row classification and new statistics assertions remain
together. Proof contracts/sinks are unchanged and rebound to actual source.
Final CI repairs preserve exact zero-headroom refusal, retain unreleased
deprecation warnings without inventing a release version, use the equivalent
stemmer character range and standard local future pinning. The scoped assembly
writer preserves other columns; final acceptance still requires all seven reports.
The Neoverse BGP cell is measured by current CI; older local compiler differences
are not substituted for current CI counts.

Fresh integration predicts tree 84d7d797e891e5dacab4243b610e5d86fd7bf9af from main
already included in the qualified branch. The authoritative plan is
.stage/planner-statistics-per-predicate-per/plan.md; selected evidence is
.stage/planner-statistics-per-predicate-per, with current acceptance in validation.md
and material dispositions in review-debt.md. Normal signing and hooks passed.

Closes #484

Defect-Class: none
