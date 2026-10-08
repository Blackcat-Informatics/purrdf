# Native row protocol adaptation — independent source review

VERDICT: PASS for the assigned scoped source/test design. Runtime/artifact
qualification remains REQUIRED and NOT established by this review. Original473
is not complete; no production host artifact or measured cost claim is made.

Reviewed exact qualified pre-row tree01fa4c179b0fe354d187ca4100b01231b7534b38
to staged adaptationb73fdd9b69bc0493feefcd65bad75500dc2ba998 on2026-10-08,
plus the subsequently staged six-line private iterator stub addition (current
stub blob b270caa05). This exact small addition was read in both source and
index before the final verdict; no final-tree identity is invented here.
The delta is exactly native py_store/query.rs, shipped root __init__.pyi and
new130line test_solution_row_protocol.py (240insertions/28deletions at b73,
plus six stub lines in the final reviewed scope).
No compat Graph.query, evaluator, foundation, dependency or feature delta is
present. The retained dirty sibling was not changed or imported wholesale.
Source/criterion proposal is portfolio-row-iteration-review.md and live473body
in raw/portfolio-row473-live-issue.md. No build, Python execution, source/index
edit or forge post occurred in this independent review.

## Actual implementation judgment

- Native QuerySolution owns its existing materialized cells and Arc projection
  names. __iter__ creates a new PyQuerySolutionIterator with Py<row> ownership
  and pos0. Creation performs no cell conversion or projected tuple/Vec copy.
  Each iterator independently retains the row beyond its result and caller's
  references; the row itself is repeatably iterable, not a consuming iterator.
- __next__ checks width before indexing, converts exactly the current cell through
  the single cell home, converts an unbound cell into Some(py.None()), then
  increments position. Outer None means only exhaustion. A conversion error
  leaves position unchanged; it is propagated rather than swallowed/advanced.
  __iter__ on the iterator returns itself and exhaustion is stable.
- __length_hint__ reads width minus position without term allocation/conversion.
  Its subtraction invariant follows from private immutable row width, pos0,
  and the sole increment after a checked in-range successful cell. No other
  setter can push position past width; no saturating fallback is needed.
- Signed integer admission accepts valid negative endpoints through checked_sub
  and unsigned_abs, so isize minimum cannot overflow. Nonnegative positions
  are checked against width. Extract-overflow Python ints receive IndexError;
  unknown names/Variables remain KeyError and other unsupported keys TypeError.
  Existing native first-name-match behavior is unchanged. Native slice indexing
  is not newly promised. All cells still use the existing RDF-term conversion.
- __len__ reports the projected cell width, including None. Empty rows have
  width0 and immediately exhausted iterators. Result materialization/ordinary
  query methods are untouched. Iteration state is O(1) in width and converts
  only requested cells, addressing the retained eager tuple's concrete cost.
- Stub imports Iterator, advertises Iterator[_Term|None] and len, and explicitly
  documents negative-index/IndexError and existing first-name-match behavior.
  The subsequently added private _QuerySolutionIterator stub declares actual
  __iter__/__next__/__length_hint__ without advertising a module constructor.
  This is required by scripts/check-python-stub-parity.py, whose structural
  forward arm derives EVERY pyclass, including return-only private types; the
  initial b73-only stub was incomplete for that owning gate. The source-only
  correction is coherent and included in this verdict, not left as future work.
  It does not register a new module export or change runtime behavior. Current compat
  ResultRow tuple iteration/len/asdict/last-label mapping and positional
  Graph.query conversion are preserved without another Python body.

No substantive source defect or uncovered required implementation behavior was
found. Lazy creation/error-position preservation are established here by exact
source control flow; the host tests do not instrument native conversion counts,
and this review does not relabel them as executed allocation or timing proof.

## Focused actual-host test design

The seven test functions expand to exactly13 pytest cases (six positional-error
parameters and two duplicate-label parameters). There are no skips, oracle
fixtures/vendor imports or broad semantic matrix in the new file. Fixed simple
queries only obtain real native row wrappers; protocol checks use actual Python
objects/indexing rather than duplicating Rust query algorithms.

|Required protocol/neighbor|Concrete source fixture|
|---|---|
|Bound/unbound projection order, len, list/tuple/unpacking|Three-cell actual native row; middle None cannot prematurely stop iteration|
|Repeated/interleaved iteration and row/result lifetime|Two distinct iterators, result gone on helper return, row deleted miditeration; remaining cells and repeat exhaustion checked|
|Iterator identity/remaining hint|iter(iterator) is iterator; hint3→2→0 and empty0|
|Signed/name/Variable/type neighbors|First/middle/last positions, negative endpoints, bool, int subclass, missing name/Variable and unsupported object/list/slice|
|Past endpoints/huge ints|3,-4,±2**100,-2**63,2**63 all require IndexError plus unchanged valid tuple neighbor|
|Zero-width|Actual ordinary SELECT-star/no-domain native row has len0/no values; index0/-1 refused; direct compat empty tuple/asdict tested|
|Compat empty-mapping law|Actual Graph.query SELECT-star/no-domain still yields no public rows, preserving454 presentation policy|
|SELECT-star public order|Native iteration compared to actual projection list; compat compared to actual result.vars; both must obtain required nonvacuous width3|
|Native duplicates|Actual query_rdflib exposes two projected positions; both retained by len/iteration; existing first-name lookup checked|
|Compat duplicate bound/unbound|Direct distinguishable (first,last) cells; negative/slice, name/Variable/attribute/get/asdict, final-label rule and copy-isolated labels|

The native duplicate fixture intentionally has equal cells, so it proves width
and duplicate-position retention rather than distinguishable duplicate-name
values. Compat direct fixtures do use different cells/None and prove last-label
access. Native name-selection implementation remains explicit first-position
source control, and the existing Rust duplicate-column/governed fixtures own
query semantics; no extra semantic-oracle rerun is required by this distinction.

## Laws, static checks and qualification handoff

Both new/source paths have applicable SPDX headers. The new Python file's line3
Why-not-Rust names Python dispatch, PyO3 integer extraction, references and tuple
accessors, with a concrete reason exceeding30characters. It is absent from
origin/main, so this is an explained new host-only path rather than growing or
moving an existing ratcheted semantic test. The shipping stub is unratcheted;
normal non-Rust ratchet/interface gates still must execute on the actual index.
Exact scoped git diff --check exited0. No source/test runtime command was run.

Proceed with the root-owned current-source artifact build and exactly13 host
protocol cases, existing row-lifetime neighbor, affected strict binding targets,
stub/interface and normal ratchet. Require actual counts/zero essential skips
and preserve errors. Qualified Rust semantics remain reusable because their
shipping implementation did not change; the changed Python binding artifact
must be qualified before completion and included in later current454 native-cost
attribution. No new ordinary parser/evaluator/source cache path was introduced,
and no broad Python/vendor/oracle semantic or unrelated suite repeat is added.
