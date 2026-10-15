# Complete graph-free RDF 1.2 carrier


Issue #492 remains wholly OPEN. No implementation or qualification is claimed.
The isolated branch paudley/492-triple-only-carrier-for-rdf-1-2 was opened by
stagectl at origin/main at commit `5144f5c1a`, then explicitly fast-forwarded to published
S1 commit `10316f2c1`. The original open.json setup base remains unchanged. PR #544 is an
unmerged source dependency: its dictionary-parametric Datalog and operational
DatasetView interfaces are reused, not reimplemented. Its checks qualify only
its original behavior. Source and review work on #492 cannot mutate PR #544.
No foundation PR or hidden followup is authorized.

## Governing decisions and prior art

Root AGENTS, .baseline, .goals, one-home ledger, layers, portability, determinism
and normal hooks apply. Current Stage intake captures the complete issue and its one approved design
comment. The discarded ADR-0032 remark supplies no approval or requirement. Its unresolved consumer ADR is available at
/home/paudley/Active/katamari/docs/adr/0038-rdf-1-2-claims-one-graph-derived-views-and-refinements.md;
38.1 requires one graph with no graph name or dataset-kind in authority,
projection, index, export or wire format. This is a consumer constraint; the
existing quad carrier remains fully supported and semantically unchanged.

Reuse the original term interner/frozen lexical and reverse lookup machinery,
DatasetView checked-read law, permutation/query kernels, RDFC graph-view path,
and Datalog semi-naive/planning/arrangement homes. There will be no second term
decoder, rule evaluator, hash implementation or analyzer. The canonical graph view
from #292 and the default projection from #383 are precedent, not a substitute
for graph-free physical storage. Current ClauseAtom/compiled arrays always have
four positions and RelationStore keys are(predicate,graph); wrapping that store
with a constant graph does not satisfy this contract. Forge recurrence coverage
is exactly prior-art.md's bounded 200-commit tally, not a complete failure count.

## Design

### Physical graph and shared term ownership

Publish TripleDataset and its explicit builder, with base rows three native
TermIds (12 bytes), reifier rows(reifier,triple) (8 bytes) and annotation rows
(reifier,predicate,object) (12 bytes). No graph slot/table is stored in these
carrier tables. SPO is physical freeze order, with only POS and OSP lazy ordinal
indexes, plus the actual side-table indexes. Share/refactor the existing frozen
term-table home so both carriers use the same arena, resolution, store-once
reverse lookup, RDF 1.2 triple terms, blank scopes, direction, content recognition
and caller configurations. Do not retain an empty RdfDataset as a disguised
graph-free dictionary. Fallible capacity/admission follows the original homes.

Public triple ingestion exposes no graph parameter. An explicit conversion
from a quad dataset validates all base/side rows and declared graphs before
publication; named content, including remembered empty named graphs, is a typed
refusal rather than discarded information. Move the original term owner instead
of cloning strings. Immutable graph-free publication retains no quad-row copy.

### Views and typed named-graph refusal

The frozen TripleDataset owns graph-free immutable payload only. Its explicit
fallible read door constructs a fresh operation-local TripleRead session, which
implements the existing DatasetView and FallibleDatasetView protocol, retaining
the frozen owner and one shared sticky typed error owner for its iterator,
term-guard and parallel/correlated children. This follows the existing segmented
snapshot/session split rather than embedding read failure in frozen data.
Reader control allocation is admitted/fallible before construction. A new read
session over the SAME frozen data is a healthy request, not a global reset.
The compatibility view projects ephemeral default QuadIds only at its existing
protocol boundary; physical rows and identities stay graph-free. Core public
query/canonical/text/rule callers obtain this read session explicitly; no
infallible DatasetView impl on the frozen payload silently bypasses the door.
Add an explicit named-graph support contract consumed by the actual engine
before GRAPH enumeration/term lookup, including GRAPH variable, constant,
FROM NAMED, prepared/correlated/nested forms and an empty graph. Thus absence of
a graph term cannot turn an unsupported request into an empty successful answer.
Use a typed refusal, retaining its original code/cause at public native boundaries.
Direct quads_for_pattern/statement graph requests on TripleRead with Named
latch NamedGraphsUnsupported in that operation-local shared error owner before
any iterator output. Its read_error and every typed term-read/reservation door
retain that first cause; checked_read therefore refuses the entire drain,
including an empty input/empty iterator. There is no empty successful fallback.
Explicit require_named_graphs preflight is implemented by that same session and
used by actual engine graph/dataset-spec entry paths, before missing-term or
empty-row shortcuts. Once an operational read fails, its children are invalid;
the source TripleDataset stays unchanged. Healthy default SPARQL retry uses a
fresh public read session over the same frozen data. Existing quad readers use
their unchanged support/status law. This is the concrete request/session API
selected for independent review, not a source edit or executed demonstration.
Default/Any internal scans enumerate precisely the original triple and virtual
statement graph. Real default SPARQL callers use those original indexes.

### Compile-time Datalog graph-free shape

Generalize the original clause/compiled-position/planning/store waist by a
closed compile-time triple versus quad shape. Triple atoms have exactly three
stored/visited positions and predicate-only partition keys; no default graph
constant participates in SIPS priority, binding arrays, relation keys, row logs,
proof/model identity or emission. Quad shape retains four positions and its
original partition semantics. Reuse one semi-naive join, negation, arrangement,
factor, chase/proof and rule execution implementation with shape-parametric
parts, rather than a triple evaluator or post-hoc projection of arity 4 output.
Use the dictionary-parametric D::Id waist being implemented by 490 when the
changes integrate; native/host IDs must not be remapped at integer joins.
SPARQL 1.2 RL translation/materialization chooses triple shape from the actual
input carrier and returns graph-free RDF 1.2 data through the public entail path.
Standard default-graph rule model equality is established against the existing
quad all-default carrier; variable predicates and recursive/cyclic rules remain.

### Canonical and text identities

Use the original fault-aware flat-presentation RDFC view canonicalizer
(`try_canonicalize_flat_view`), with exact equality of canonical bytes against
the all-default quad form under that same presentation (including RDF 1.2 statement data and
blank scopes). Document this byte identity explicitly. Dataset digest may retain
that identity only if its actual preimage is demonstrated identical; otherwise
register an explicit distinct graph-free hash domain and tested versioned law.
No independent canonicalization algorithm or stringified graph sentinel.

Text uses a distinct explicit graph-free corpus profile: documents(subject,
language), partitions(language), with original direction merging, exclusions,
field/token/analyzer and zero-token laws. Share the original corpus/ranking
construction using a compile-time key shape whose graph component is absent,
not serialized as None/default. Corpus fingerprint/profile identity, actual
scoring, retrieval source binding and deterministic egress use that profile.
Existing graph-language profile stays unchanged. The pack partition precedent
does not authorize claiming a graph-partitioned pack is graph-free.

## Completeness contract

| Requirement | Actual demonstration | Task |
| --- | --- | --- |
| 12N row storage, compact statement rows, three orders | Actual public builder publishes nontrivial RDF 1.2 N-row data; frozen row-byte counter=12N,8R,12A; compile-time layout assertions; real POS/OSP and all side probes compare expected original IDs without graph tables; scaling bench retains actual memory/index counters | 1,2 |
| Named graphs typed refusal, additive quad behavior | Real engine SELECT/ASK/CONSTRUCT/prepared/nested GRAPH and FROM NAMED over fresh empty/populated TripleRead sessions refuse with exact typed diagnostic; same frozen TripleDataset healthy default retry on a fresh session succeeds; direct checked_read over named base/statement pattern refuses before output, including empty input and worker/correlated children; quad controls retain original behavior | 1,2 |
| Default SPARQL conformance | Existing official corpus runner executes its unchanged default-graph tests on TripleDataset, with an explicit manifest-derived inclusion inventory/count, original solution/report comparison and XPASS discipline; no arbitrary exclusions or self-oracle | 1,2 |
| Arity 3 RL, predicate-only keys, same model | Public entail materialization on triple input uses compiled three-position atoms/keys; original standard RL/recursive/variable-predicate fixtures compare exact model against original all-default quad form and independent expected answers; no arity 4 surrogate | 1,2 |
| RDFC/dataset digest law | Actual canonical graph-view output and identity preimage are compared for RDF 1.2 reifiers, annotations, nested triples, blank relabelings and directional strings; distinct domain only if explicit and documented/tested | 1,2 |
| Graph-free text corpus/profile/identity | Real Index construction and query/retrieval exercise subject-language keys, language partitions, original rank/token/exclusion laws and profile binding; frozen identity vectors show absent graph component and preserve existing quad profile | 1,2 |
| Native/portable/hygiene/docs/landing | Complete affected native and actual WASM executions, original controls/scale bench, strict workspace consumers, generated docs/book, one complete final gate after corrections settle; independent full-contract audit, normal hooks, required hosted checks, ghprsq and owned cleanup | 2,3 |

## Task 1: Complete original-home implementation and consumer wiring

Implement the whole graph-free term/table/view, Datalog/entail and text/canonical
production contract as one coherent source batch. Coordinate dictionary changes
through the published `10316f2c1` dependency without adopting its prior checks as 492 qualification. Review
actual ingress/egress and typed boundaries independently; no partial foundation
publication. Focused compiler/runtime checks start after coherent source/caller
migration, with unchanged controls and resource assertions retained.

## Task 2: Qualify the complete public contract and fix concrete findings

Run the completeness table's actual native/portable/corpus/scale controls and
independent source/consumer review. Batch real related corrections; reuse unaffected
evidence and repeat only invalidated coverage. Reserve one full make check or
required equivalent for settled source. No baseline whole-workspace discovery.
Record truthful executions in validation.md; an aborted or failed gate is not
qualification. Commit/push the complete issue normally after required review/gates.

## Task 3: Publish, finalize, merge and clean the complete issue

Open the complete PR, publish authoritative plan/acceptance/uncertainty through
Stagectl, resolve actual feedback and current-head hosted prerequisites, assess
the actual integration candidate and finalize using ghprsq only. Verify issue
closure, archive and cleanup before recording completion. No unrelated donor,
active source, audit evidence or benchmark baseline is disposable.

## Declined additions and boundaries

No semantic feature, implicit vocabulary, alternate engine, fixed graph sentinel,
graph-discarding conversion or arity 4 wrapper is accepted. Standalone new host
product APIs and graph-free pack/wire redesign are not requested by this issue;
existing published crates/hosts must still build and preserve original behavior,
and actual portable execution of the new core/rule/text contract is required.
No requested triple-only storage, rule, canonical, identity or query capability
is deferred to another issue. Planning recheck must adjudicate the concrete
operation-local TripleRead request/status interface above before source edits.




## Implementation order inside Task 1

Coherent compiler slices are internal checkpoints of the complete issue, never
independently delivered foundation PRs:

1. Extract the original frozen term owner and lookup/resolve bodies into one
   graph-independent native home. Both resident carriers own that same concrete
   dictionary. Build the explicit graph-free row/side-table carrier and its
   builder with original validation and admission. Keep all frozen quad callers
   working and preserve term/configuration/blank identity in ownership transfer.
2. Publish the operation-local TripleRead and original-session first-cause
   transport; wire a named-graph preflight through the actual engine and wrapper
   views before all missing-term/enumeration shortcuts. Exercise fresh-session
   retry and RDF 1.2 statement-layer reads before broader query qualification.
3. Generalize the original Datalog atoms, compiled slots and predicate partition
   key by a closed static shape, preserving the S1 D::Id dictionary waist. Wire
   actual entail materialization to the triple shape. No arity-four staging
   relation, default graph constant or duplicate rule interpreter is allowed.
4. Route original RDFC and text/retrieval homes through graph-free physical keys
   and explicit identity laws. Refactor shared computation instead of copying
   canonicalization, analyzer, ranking, hashing or serialization bodies.
5. Complete the matrix above, including manifest-derived unchanged default-graph
   SPARQL corpus, arity-three RL and actual native/WASM execution. Only then
   publish the complete contract and assess integration.

The first focused checks target the actual changed packages/interfaces, using
standard directories and installed tools. No baseline full suite is scheduled.
N owns the current heavy Rust lane; implementation/read work proceeds without a
competing Cargo invocation until explicit handoff and resource admission.

## Current unresolved implementation questions

These are design work within scope, not permission requests or deferrals:
- Choose the smallest frozen dictionary extraction that retains all old public
  methods and exact source-backed mint invariants without duplicate lookup bodies.
- Keep triple-only ingestion graph-free during accumulation as well as frozen
  publication; any shared builder control must not retain quad rows or expose a
  graph parameter on the triple public API.
- Preserve the existing typed operational error at all actual engine boundaries;
  an unsupported named graph must not become a successful empty result through
  DatasetView wrappers or an unbound GRAPH variable.
- Keep compile-time shape identity explicit across rule proofs/cache/model keys
  and graph-free text corpus/source binding, not merely change row widths.

Independent planning recheck must accept the concrete TripleRead and closed
static shape design before production edits. Open implementation questions are
resolved in that design/source review; none authorizes omitting a requirement.

## Concrete native ownership and identity decisions

The frozen dictionary extraction moves original resolve/reverse-lookup bodies,
not a second parser. RdfDataset's existing public methods continue to delegate
to that one concrete owner. TripleDataset holds the same owner by value plus
only compact triple/statement tables; no empty RdfDataset is retained. Its
builder stores compact rows while the original mutable term home provides
interning, scope/CDT binding and term validation. Factor freeze's term ownership
transfer so this path does not create a temporary frozen quad dataset.

The existing flat canonical presentation already emits actual RDF 1.2 assertion
triples for reifier/annotation rows. Use it explicitly for graph-free canonical
bytes/digest and compare the same presentation on the all-default quad oracle.
Do not describe the overlay presentation's annotation graph sentinel as a
graph-free identity. `try_flat_digest_view` hashes precisely the canonical
document via original ContentDigest; equality of preimages is testable without
a new hash implementation or domain.

TripleRead may borrow the immutable payload for its lifetime; its child readers
and term guards retain the same fallibly allocated Shared status control. Fresh
read() constructs a new control over the same frozen owner. Control birth and
lazy permutation buffers use the original Memory/Resident or caller admission
home; no external grant may be refunded while a returned owner still uses it.
Resident reserve_workspace still checks first-cause status on acquisition and
resize, rather than returning an unconditional no-op after a named-graph fault.
The named-graph capability is distinct from observed named_graphs content:
empty quad datasets remain capable; triple sessions refuse even empty graph
patterns. Actual engine preflight covers authored dataset specs and all nested
algebra paths before data-dependent shortcuts.

