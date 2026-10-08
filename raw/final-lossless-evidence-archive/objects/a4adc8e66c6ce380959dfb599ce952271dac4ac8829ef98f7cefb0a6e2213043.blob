# Retained native row iteration: scoped454 integration proposal

Status: SOURCE REVIEW / ACCEPTANCE PROPOSAL, not473 completion or runtime PASS.
Read-only2026-10-08; no Cargo, Python execution, shipping/index/sibling mutation
or forge posting. Root must wait for live Rust87554 to settle before adapting
this source into the existing454 delivery and qualifying current host/native
cost. No new branch or foundation cherry-pick is warranted:863foundation is
already incorporated.

The live473 body was captured with supported Stagectl in
raw/portfolio-row473-live-issue.md. It requires row value iteration in projection
order, None for unbound, bound/unbound/empty-projection/SELECT-star coverage, and
working name/position/len/asdict neighbours. Its preserved comment explicitly
calls the two-file work unreviewed/in-progress. The exact dirty sibling currently
changes only bindings/python/src/py_store/query.rs and the shipped root .pyi:
77insertions/28deletions. This review reads that diff directly; it does not
restore compressed tracker material, change the sibling, or treat preservation
as evidence of qualification.

## Actual current surfaces and gaps

|Surface|Current contract and scoped action|
|---|---|
|Native PyQuerySolution|Owns Vec<Option<RdfTerm>> and Arc projection names; has __getitem__ only. Integer exhaustion raises KeyError, so Python sequence fallback does not terminate correctly. Missing __iter__ and __len__ is a real remaining issue.|
|Native integer/name access|Positive in-range cells and name/Variable lookups work by source; current negative indices fail type admission, integer out-of-range raises KeyError. Adopt explicit tuple-style negative indexing and IndexError while preserving unknown-name KeyError and unsupported-key TypeError.|
|Current compat ResultRow|Already inherits tuple iteration/len/positional-negative/slice behavior; named/get/attribute/asdict use the label table. No extra shim iterator/body is needed.|
|Current Graph.query|Already constructs ResultRow by indexing every native projected position. This preserves duplicate column cells; do not revert to native name lookup or import older Graph.query/foundation bodies.|
|asdict|Exists on compat ResultRow and omits unbound final-label cells. It never existed on native QuerySolution/stub; preserve and qualify the existing compat API rather than inventing a native mapping API to satisfy a supposed regression.|
|Duplicate names|Compat labels select the LAST duplicate position; native current named lookup uses the FIRST matching projected name. The retained native patch preserves that existing native behavior, while positional iteration preserves every column. Do not silently unify lookup rules or deduplicate columns.|
|Empty projected rows|Native ordinary SELECT-star over an empty variable domain can expose a zero-width row; its Python object must have len0 and iter() immediately exhausted.454compat deliberately suppresses empty actual mappings before public row construction; preserve that Rust presentation law. Compat ResultRow((),()) is independently a valid empty Python tuple. Do not force a compat query to emit an empty row to make the APIs' row bags appear equal.|

Actual code read: native query.rs704–791 and materialize_results1084 onward;
compat query.py128–190, graph.py1105–1193; root .pyi406 onward. Native result
iteration already moves each row's cells out and clones only Arc variables, so
a yielded row outlives its result. The row fix must preserve that ordinary
ownership path and not introduce evaluator/parse/materialization changes.

## Exact reuse and adaptation

Reuse the original Rust patch's private cell conversion home, checked signed
integer normalization (including isize minimum), huge-Python-int IndexError,
name/Variable lookup and row __len__, plus its matching .pyi Iterator/len contract
and tuple-style documentation. Reconcile imports against current454 source;
never wholesale-copy the stale query.rs or cherry-pick863. No dependencies,
features, new semantic engine or compat Python implementation are needed.

The proposed tuple-backed __iter__ is contract-correct by source, repeatable and
keeps the produced iterator alive independently of the row. Its cost is eager:
every iter(row) converts ALL projected cells and allocates both Vec storage and
a Python tuple before the first next(), even if a caller takes only one cell.
That cost is row-width bounded, not total result-count bounded; repeated iteration
repeats conversion. It does NOT change ordinary Store.query/Graph.query engine
execution or current positional Graph.query unless callers invoke row iteration.
An error converting a later cell would also be raised before an earlier cell is
yielded. No measured performance conclusion is available from this review.

Preferred performance adaptation is a small original Rust iterator owning a
Py<PyQuerySolution> and a next-position: __iter__ makes a fresh iterator,
__next__ converts one cell through the same cell home and stops at width.
This keeps O(1) iterator state, lazy conversion and independent/reentrant
iteration without copying native cells or caching Python terms. Never make
QuerySolution itself a stateful self-iterator: that would consume/restart shared
row state and break repeated/nested iteration. The root may choose the smaller
retained tuple implementation if its explicitly captured row-local allocation
trade-off satisfies the delivery's performance judgment; laziness is not a new
semantic acceptance requirement or excuse to defer the iteration fix. In either
case preserve native query/capture/allocation machinery outside the row protocol.

## Only irreducible Python acceptance

Use the existing actual current-source wheel/install and isolated interpreter
route. No broad Python suite, vendored RDFLib tests, semantic oracle-query matrix
or new Python semantic mirror. Each proposed check is about Python protocol
dispatch/object/lifetime/type behavior, not re-proving Rust SPARQL answers:

1. Actual native rows: for/list/tuple/unpacking yield projected cell order,
   including None in an unbound slot; len remains projection width. Interleave
   two iterators, iterate the same row twice, and finish an iterator after
   deleting row/result references. Reuse existing
   test_query.py::test_native_select_row_outlives_shared_result ownership control.
2. Native neighbours: string/Variable and integer access agree on their intended
   positions; negative first/last valid positions; positive/negative endpoints
   and huge positive/negative Python ints raise IndexError; missing names raise
   KeyError and unsupported types TypeError. Verify Python bool/int-subclass
   extraction at the boundary; arbitrary __index__/slice expansion is not an
   added native API contract (the typed native key remains str|Variable|int).
3. Zero-width actual native row: SELECT-star/no variables reaches the real row
   wrapper, len0, no values/no error, and both positional neighbours refused.
   Compat direct empty ResultRow has the corresponding tuple protocol; actual
   compat query's suppressed empty mapping stays an empty RESULT, not an emitted
   empty row. Keep exact Rust empty-domain semantics in existing Rust fixtures.
4. Actual SELECT-star native/compat rows: derive order from the actual public
   projection list/labels and compare row iteration to positional indexing.
   Do not invent a Python-engine column-order law or rerun a semantic oracle.
5. Compat direct duplicate-column bound/unbound rows: tuple positions retain
   both cells while named/attribute/get/labels/asdict retain LAST-label behavior;
   omitted unbound asdict entries, copied labels and valid negative/slice access
   remain unchanged. Native row iteration must likewise retain every positional
   duplicate. Reuse/adapt existing test_contextual_mappings.py duplicate Python
   accessor controls; do not run their query-oracle differential as a substitute
   for host protocol qualification or re-add native semantics in Python.

For bound/unbound/star native wrappers, simple fixed queries are only ingress
to real PyO3 row objects; semantic expectations remain governed by already
qualified Rust fixtures. A test must actually obtain the required row width,
not pass vacuously over zero rows. Public stubs must describe the actual iterator
return shape and len, and their existing owning interface gate must pass.

Existing evidence gaps: retained patch has no added executed row-protocol tests;
current454 tests prove row ownership and compat duplicate access, not native
row __iter__/len/negative/empty protocol. Existing native rdflib_contextual
fixtures already cover duplicate columns/governed publication and empty-domain
semantics: reuse their source-bound qualified result, not broad semantic reruns.
If a required duplicate/empty wrapper cannot be reached from its real native
door, report that exact boundary and add an original Rust/PyO3 contained fixture
within the binding home rather than fabricate a public test-only constructor.

## Ratchet and downstream qualification

Shipping root .pyi is an unratcheted surface. Existing test_query.py is ratcheted;
do not grow it casually or hide growth by moving code. Prefer the already new
454test_contextual_mappings.py host-only home with its explicit Why-not-Rust
header, or a focused original host-protocol file carrying the concrete reason
within40lines. Check the actual integration-base ratchet normally; no exemption
or env escape. Legacy Python semantic tests may shrink, never be replaced with
a new semantic suite. Frozen vendor bodies are untouched.

After root implements and independent source review accepts the delta, qualify
the affected Rust binding warning-denied build/alltargets/interface/ratchet and
the bounded real Python protocol cases on a current artifact. Then proceed to
current454 host/native-cost qualification with this delta included. Because
row iteration is a host operation, eager/lazy conversion costs must not be
misrepresented as evaluator/native ordinary-query regression; inspect the actual
owning cost surface and changed artifact before asserting reuse. Source-dependent
previous binding/native-cost artifact identities cannot silently qualify changed
query.rs. Root owns source edits, normal hooks/publication and final criteria.
Original473 is explicitly unfinished until actual protocol evidence and source
delivery are complete; this source review closes no issue.
