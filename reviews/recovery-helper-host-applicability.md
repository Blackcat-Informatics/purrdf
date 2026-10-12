# Helper repair: installed-host evidence applicability

Bounded source review requested by root. No builds, tests, source edits, process
changes or private-input reads were performed. This report selects existing
checks; it does not record their execution. Root reports the repaired helper
census, 42 core distance cases, six HNSW environment cases, strict five-package
all-target compilation and layer check passing. The mandatory full gate remains
required, independently of the host refresh below.

## Changed production paths

- Python `src/py_geo.rs` and WASM `src/geo.rs` now each share one local
  `response_text` between session and point-index calls. They still move the
  native String, preserve semantic JSON and map operational refusal through
  the original exception/code. Python still finishes `py.detach` before
  constructing its exception. These are changed compiled host paths.
- Python `src/py_gts_dataset.rs:38` declares `from_frozen` through the existing
  `purrdf_lex::constructors!` typed-source arm. Its two actual callers are
  `py_projection.rs:248` (an Arc clone returned by the lift's dataset getter)
  and `py_entail.rs:275` (the materialized closure). The conversion and ownership
  operation are unchanged, but both should execute from the rebuilt wheel.
  The direct workspace `purrdf-lex` dependency and corresponding layer/lock
  edge also require a new source-distribution-to-wheel packaging check.
- `rdf-core/src/distance/mod.rs:164–166` now reaches XSD control/environment
  rather than duplicate files. Native typed environment-refusal coverage is
  the reported 42 + 6 focused cases. Existing Python HNSW/kNN producers also
  exercise the changed admission home through actual installed consumers.
  Normal host execution does not replace native trap-state refusal tests.
- The C functions retain their existing shared response/free helpers. No C ABI
  signature changed in this helper repair. Its installed library nevertheless
  needs rebuilding because its linked production dependency graph changed.

## Required affected refresh using existing checks

| Surface | Fresh artifact and existing checks |
| --- | --- |
| C | Run existing `make capi-bundle`: rebuild, audit provenance/licenses and headers, install/link, and execute the complete existing smoke. `crates/rdf-capi/tests/smoke.c:858` exercises both geographic handle kinds, semantic versus operational refusal, index lifetime after session free, and explicit versus undeclared query axes. There is no need to invent a separate geographic C smoke. |
| Python | Run existing `make python-release-check`, which builds the wheel **from the new sdist**, audits both distributions and installs them together. Against that exact isolated installation, run the five existing `tests/test_geo.py` cases plus the three named cases below. Record the actual imported extension/package path and fresh candidate path, following the earlier installed-wheel receipt. A development import or old installed wheel is not a refresh. |
| WASM | Rebuild through existing `make wasm-pkg` so the tested module includes release compilation, bindgen, optimization, post-linking and the actual SIMD-opcode check. On that artifact run all four `tests/geo.test.mjs` cases, the three previously repaired poison cases below, `npm run typecheck`, and `npm run smoke:pack`. The geographic async profile-snapshot case is affected and must run; the other long async cases are not invalidated solely by extracting these helpers. |

The three additional Python cases are:

1. `test_projection.py::test_lift_returns_a_frozen_dataset_and_write_only_profiles_fail_typed`
   exercises the retained Arc-backed dataset getter.
2. `test_entail_regimes.py::test_materialize_nt_matches_the_dataset_path_byte_for_byte`
   exercises the materialized-closure constructor and its actual serialized value.
3. `test_retrieval_vector_producers.py::test_the_two_vector_producers_differ_in_what_they_promise_and_not_in_what_they_rank`
   constructs and executes both installed HNSW and kNN producers over the same
   vectors and compares ranking to a host-computed oracle. Production registration
   is `py_retrieval.rs:852–903`. This checks healthy host admission, not deliberate
   trap-state manipulation or every arithmetic/backend configuration.

The three WASM poison cases remain relevant to the freshly linked artifact and
are individually selectable with the existing Node test-name filter:

- `async-concurrency.test.mjs`: `a trap poisons every entry point of the instance, and jobs that fault without trapping poison nothing`
- `panic-poison.test.mjs`: `a panic in a synchronous call poisons every entry point, and typed errors poison nothing`
- `trap-poison.test.mjs`: `a trap that is not a panic, in a synchronous call, poisons every entry point`

## Reuse boundaries

`validation.md:259–261` records earlier successful C/Python artifacts and composed
WASM acceptance. Preserve those receipts as historical evidence; do not relabel
their binaries as rebuilt. In particular, the earlier WASM justification that
the compiled module was unchanged no longer applies to this Rust helper delta.
Replace that justification with fresh artifact identity plus this explicit
affected-path refresh and unchanged-source applicability for retained results.

The prior 446 passing WASM cases, including the unrelated expensive async
correctness runs, need not all rerun merely because the same library was
recompiled. This helper delta does not change their continuation/stack machinery,
Dataset identity ABI, JS scheduler or linker implementation. The separately
passed identity-ABI fixture can likewise remain source-applicable to this delta;
it must not be described as executed against the new module. Any new evidence
of changed relevant code or failure overrides this reuse determination.

The selected refresh is composed affected acceptance, **not** a newly successful
whole `make wasm-pkg-test` invocation. That recipe runs the full runtime collection
and identity fixture; its old failed invocation must stay failed in the record.
Likewise, host refresh does not replace full `make check`, corpus completion,
assembly, architecture execution or any still-open issue acceptance.

This report addresses the helper consolidation only. Concurrent numerical
arena/retry changes require their own source-applicability assessment; their
presence in the same rebuilt artifacts must not silently inherit approval from
this bounded review. No new material source defect was found in the inspected
helper paths.
