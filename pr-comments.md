# PR #534 — comments and review threads

1 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->

> [!IMPORTANT]
> ## Review skipped
> 
> Too many files!
> 
> This PR contains 360 files, which is 60 over the limit of 300.
> 
> To get a review, reduce the PR to 300 files or fewer by splitting it into smaller PRs or changing its base branch.
> 
> Usage-priced reviews support at most 300 files.
> 
> <details>
> <summary><strong>⚙️ Run configuration</strong></summary>
> <dl>
> <dd>
> 
> - **Configuration used**: defaults
> - **Review profile**: CHILL
> - **Plan**: Team
> - **Run ID**: `b72bed10-4f73-46a1-ad90-081d5978938a`
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📥 Commits</strong></summary>
> <dl>
> <dd>
> 
> Reviewing files that changed from the base of the PR and between 29661e74bdfa5e760fca5595e72b572176d7dd0e and 3c92cbeebad8d52bac064f1d4f9a714e7d9b7fb6.
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>⛔ Files ignored due to path filters (1)</strong></summary>
> <dl>
> <dd>
> 
> * `Cargo.lock` is excluded by `!**/*.lock`
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📒 Files selected for processing (360)</strong></summary>
> <dl>
> <dd>
> 
> * `.github/workflows/ci.yaml`
> * `AGENTS.md`
> * `Cargo.toml`
> * `Makefile`
> * `bindings/python/Cargo.toml`
> * `bindings/python/python/src/purrdf/__init__.py`
> * `bindings/python/python/src/purrdf/__init__.pyi`
> * `bindings/python/python/src/purrdf/geo.pyi`
> * `bindings/python/src/lib.rs`
> * `bindings/python/src/py_geo.rs`
> * `bindings/python/src/py_gts_dataset.rs`
> * `bindings/python/src/py_store/prepared.rs`
> * `bindings/python/src/py_store/quad_store.rs`
> * `bindings/python/src/py_store/query.rs`
> * `bindings/python/src/py_store/store.rs`
> * `bindings/python/tests/test_geo.py`
> * `crates/cdt/src/ops.rs`
> * `crates/cli/README.md`
> * `crates/cli/src/cli.rs`
> * `crates/cli/src/lib.rs`
> * `crates/cli/src/query.rs`
> * `crates/cli/src/update.rs`
> * `crates/cli/tests/geo_cli.rs`
> * `crates/geo-kernel/Cargo.toml`
> * `crates/geo-kernel/README.md`
> * `crates/geo-kernel/benches/cells.rs`
> * `crates/geo-kernel/benches/covers.rs`
> * `crates/geo-kernel/benches/geodesic.rs`
> * `crates/geo-kernel/benches/transforms.rs`
> * `crates/geo-kernel/examples/chinese_fixture.rs`
> * `crates/geo-kernel/examples/cover_receipts.rs`
> * `crates/geo-kernel/examples/cube/constants.rs`
> * `crates/geo-kernel/examples/gen_cube_constants.rs`
> * `crates/geo-kernel/examples/geodesic_allocations.rs`
> * `crates/geo-kernel/examples/geodesic_corpus.rs`
> * `crates/geo-kernel/examples/operation_cost.rs`
> * `crates/geo-kernel/examples/qualify_determinism.rs`
> * `crates/geo-kernel/licenses/CC-BY-4.0.txt`
> * `crates/geo-kernel/licenses/LICENSE-APACHE`
> * `crates/geo-kernel/licenses/LICENSE-MIT`
> * `crates/geo-kernel/licenses/LICENSE-MULAN`
> * `crates/geo-kernel/licenses/NOTICE.txt`
> * `crates/geo-kernel/licenses/inventory.json`
> * `crates/geo-kernel/src/atlas.rs`
> * `crates/geo-kernel/src/atlas/LAW.md`
> * `crates/geo-kernel/src/atlas/arcs.rs`
> * `crates/geo-kernel/src/atlas/arcs/arrangement.rs`
> * `crates/geo-kernel/src/atlas/arcs/arrangement/relation.rs`
> * `crates/geo-kernel/src/atlas/arcs/axis_overlap.rs`
> * `crates/geo-kernel/src/atlas/arcs/constant.rs`
> * `crates/geo-kernel/src/atlas/arcs/polynomial.rs`
> * `crates/geo-kernel/src/atlas/arcs/source_join.rs`
> * `crates/geo-kernel/src/atlas/arcs/subset.rs`
> * `crates/geo-kernel/src/atlas/area.rs`
> * `crates/geo-kernel/src/atlas/area/hemisphere.rs`
> * `crates/geo-kernel/src/atlas/arrangement.rs`
> * `crates/geo-kernel/src/atlas/boundary.rs`
> * `crates/geo-kernel/src/atlas/chart_index.rs`
> * `crates/geo-kernel/src/atlas/enclosure.rs`
> * `crates/geo-kernel/src/atlas/native.rs`
> * `crates/geo-kernel/src/atlas/oriented.rs`
> * `crates/geo-kernel/src/atlas/point.rs`
> * `crates/geo-kernel/src/atlas/relate.rs`
> * `crates/geo-kernel/src/atlas/tests.rs`
> * `crates/geo-kernel/src/atlas/union.rs`
> * `crates/geo-kernel/src/binding.rs`
> * `crates/geo-kernel/src/binding/prepared_image.rs`
> * `crates/geo-kernel/src/buffer.rs`
> * `crates/geo-kernel/src/buffer/materialize.rs`
> * `crates/geo-kernel/src/buffer/materialize/covering.rs`
> * `crates/geo-kernel/src/buffer/materialize/cut.rs`
> * `crates/geo-kernel/src/buffer/materialize/global.rs`
> * `crates/geo-kernel/src/carrier.rs`
> * `crates/geo-kernel/src/carrier/admission.rs`
> * `crates/geo-kernel/src/carrier/output.rs`
> * `crates/geo-kernel/src/carrier/storage.rs`
> * `crates/geo-kernel/src/carrier/term.rs`
> * `crates/geo-kernel/src/carrier/tests.rs`
> * `crates/geo-kernel/src/carrier/writer.rs`
> * `crates/geo-kernel/src/cells.rs`
> * `crates/geo-kernel/src/cells/LAW.md`
> * `crates/geo-kernel/src/cells/carrier.rs`
> * `crates/geo-kernel/src/cells/constants.rs`
> * `crates/geo-kernel/src/cells/cover.rs`
> * `crates/geo-kernel/src/cells/cover/chart.rs`
> * `crates/geo-kernel/src/cells/cover/disk.rs`
> * `crates/geo-kernel/src/cells/cover/tests.rs`
> * `crates/geo-kernel/src/cells/fixed.rs`
> * `crates/geo-kernel/src/cells/hierarchy.rs`
> * `crates/geo-kernel/src/cells/index.rs`
> * `crates/geo-kernel/src/cells/profile.rs`
> * `crates/geo-kernel/src/cells/region.rs`
> * `crates/geo-kernel/src/cells/region/tests.rs`
> * `crates/geo-kernel/src/cells/scale.rs`
> * `crates/geo-kernel/src/cells/tests.rs`
> * `crates/geo-kernel/src/construct.rs`
> * `crates/geo-kernel/src/construct/arithmetic.rs`
> * `crates/geo-kernel/src/context.rs`
> * `crates/geo-kernel/src/context/integer_observer_tests.rs`
> * `crates/geo-kernel/src/context/native_memory.rs`
> * `crates/geo-kernel/src/context/output.rs`
> * `crates/geo-kernel/src/context/preparation.rs`
> * `crates/geo-kernel/src/context/retained.rs`
> * `crates/geo-kernel/src/de9im.rs`
> * `crates/geo-kernel/src/determinism.rs`
> * `crates/geo-kernel/src/ellipsoidal.rs`
> * `crates/geo-kernel/src/ellipsoidal/distance.rs`
> * `crates/geo-kernel/src/ellipsoidal/distance/selected.rs`
> * `crates/geo-kernel/src/ellipsoidal/jet.rs`
> * `crates/geo-kernel/src/ellipsoidal/native.rs`
> * `crates/geo-kernel/src/error.rs`
> * `crates/geo-kernel/src/exact.rs`
> * `crates/geo-kernel/src/geodesic.rs`
> * `crates/geo-kernel/src/geodesic/arc_view.rs`
> * `crates/geo-kernel/src/geodesic/astroid.rs`
> * `crates/geo-kernel/src/geodesic/band.rs`
> * `crates/geo-kernel/src/geodesic/coefficients.rs`
> * `crates/geo-kernel/src/geodesic/direct.rs`
> * `crates/geo-kernel/src/geodesic/integral_fallback.rs`
> * `crates/geo-kernel/src/geodesic/inverse.rs`
> * `crates/geo-kernel/src/geodesic/inverse/area.rs`
> * `crates/geo-kernel/src/geodesic/line.rs`
> * `crates/geo-kernel/src/geodesic/normal.rs`
> * `crates/geo-kernel/src/geodesic/point.rs`
> * `crates/geo-kernel/src/geodesic/preparation.rs`
> * `crates/geo-kernel/src/geodesic/seed.rs`
> * `crates/geo-kernel/src/geographic.rs`
> * `crates/geo-kernel/src/geojson.rs`
> * `crates/geo-kernel/src/geom.rs`
> * `crates/geo-kernel/src/geom/arbitrary.rs`
> * `crates/geo-kernel/src/geom/tree.rs`
> * `crates/geo-kernel/src/index.rs`
> * `crates/geo-kernel/src/index/geographic.rs`
> * `crates/geo-kernel/src/index/projection.rs`
> * `crates/geo-kernel/src/index/projection/tests.rs`
> * `crates/geo-kernel/src/json.rs`
> * `crates/geo-kernel/src/lib.rs`
> * `crates/geo-kernel/src/measure.rs`
> * `crates/geo-kernel/src/metric.rs`
> * `crates/geo-kernel/src/metric/input.rs`
> * `crates/geo-kernel/src/metric/report.rs`
> * `crates/geo-kernel/src/metric/unit.rs`
> * `crates/geo-kernel/src/numerical.rs`
> * `crates/geo-kernel/src/operation.rs`
> * `crates/geo-kernel/src/operation/admission.rs`
> * `crates/geo-kernel/src/operation/affine.rs`
> * `crates/geo-kernel/src/operation/forward.rs`
> * `crates/geo-kernel/src/operation/gcj_inverse.rs`
> * `crates/geo-kernel/src/operation/gcj_inverse/tests.rs`
> * `crates/geo-kernel/src/operation/geometry.rs`
> * `crates/geo-kernel/src/operation/geometry/critical.rs`
> * `crates/geo-kernel/src/operation/geometry/curve.rs`
> * `crates/geo-kernel/src/operation/geometry/curve/selected.rs`
> * `crates/geo-kernel/src/operation/geometry/curve/selected/range.rs`
> * `crates/geo-kernel/src/operation/geometry/directional.rs`
> * `crates/geo-kernel/src/operation/geometry/point.rs`
> * `crates/geo-kernel/src/operation/grid.rs`
> * `crates/geo-kernel/src/operation/grid/tests.rs`
> * `crates/geo-kernel/src/operation/inverse.rs`
> * `crates/geo-kernel/src/operation/models.rs`
> * `crates/geo-kernel/src/operation/models_inverse.rs`
> * `crates/geo-kernel/src/operation/models_inverse/tests.rs`
> * `crates/geo-kernel/src/operation/source_admission_tests.rs`
> * `crates/geo-kernel/src/operation/transverse.rs`
> * `crates/geo-kernel/src/operation/transverse_inverse.rs`
> * `crates/geo-kernel/src/prepared.rs`
> * `crates/geo-kernel/src/prepared/LAW.md`
> * `crates/geo-kernel/src/prepared/tests.rs`
> * `crates/geo-kernel/src/profile.rs`
> * `crates/geo-kernel/src/relations.rs`
> * `crates/geo-kernel/src/topology/arrangement.rs`
> * `crates/geo-kernel/src/topology/locate.rs`
> * `crates/geo-kernel/src/topology/mod.rs`
> * `crates/geo-kernel/src/topology/noding.rs`
> * `crates/geo-kernel/src/topology/relate.rs`
> * `crates/geo-kernel/src/topology/segment.rs`
> * `crates/geo-kernel/src/vocab.rs`
> * `crates/geo-kernel/src/wkt.rs`
> * `crates/geo-kernel/tests/cells_determinism.rs`
> * `crates/geo-kernel/tests/geo_determinism.rs`
> * `crates/geo-kernel/tests/vectors/cube_assignment.txt`
> * `crates/geo/Cargo.toml`
> * `crates/geo/src/error.rs`
> * `crates/geo/src/exact.rs`
> * `crates/geo/src/geom/tree.rs`
> * `crates/geo/src/lib.rs`
> * `crates/geo/src/topology/segment.rs`
> * `crates/geo/tests/determinism.rs`
> * `crates/hash/README.md`
> * `crates/hash/src/frame.rs`
> * `crates/hash/src/impls.rs`
> * `crates/helper-census/src/glossary/mod.rs`
> * `crates/hnsw/src/relation.rs`
> * `crates/iri/src/vocab.rs`
> * `crates/iri/src/vocab/ogc.rs`
> * `crates/json/src/error.rs`
> * `crates/json/src/parse.rs`
> * `crates/lex/src/allocation.rs`
> * `crates/lex/src/allocation/shared.rs`
> * `crates/lex/src/allocation/shared/buffers.rs`
> * `crates/lex/src/assoc.rs`
> * `crates/lex/src/json.rs`
> * `crates/lex/src/json/error.rs`
> * `crates/lex/src/json/number.rs`
> * `crates/lex/src/json/output.rs`
> * `crates/lex/src/json/read.rs`
> * `crates/lex/src/json/record.rs`
> * `crates/lex/src/json/value.rs`
> * `crates/lex/src/json/write.rs`
> * `crates/lex/src/json_escape.rs`
> * `crates/lex/src/json_pointer.rs`
> * `crates/lex/src/walk.rs`
> * `crates/lex/src/walk/sort.rs`
> * `crates/rdf-capi/README.md`
> * `crates/rdf-capi/include/purrdf.h`
> * `crates/rdf-capi/src/geo.rs`
> * `crates/rdf-capi/src/lib.rs`
> * `crates/rdf-capi/src/query.rs`
> * `crates/rdf-capi/src/status.rs`
> * `crates/rdf-capi/src/version.rs`
> * `crates/rdf-capi/tests/abi.rs`
> * `crates/rdf-capi/tests/abi_signatures.snapshot`
> * `crates/rdf-capi/tests/smoke.c`
> * `crates/rdf-core/src/distance/control.rs`
> * `crates/rdf-core/src/distance/mod.rs`
> * `crates/rdf-core/src/distance/tests.rs`
> * `crates/rdf-core/src/ir/term.rs`
> * `crates/rdf-core/src/ir/term_walk.rs`
> * `crates/rdf-core/src/small.rs`
> * `crates/rdf-wasm/js/index.d.ts`
> * `crates/rdf-wasm/js/index.mjs`
> * `crates/rdf-wasm/js/tests/geo.test.mjs`
> * `crates/rdf-wasm/src/async_query.rs`
> * `crates/rdf-wasm/src/geo.rs`
> * `crates/rdf-wasm/src/lib.rs`
> * `crates/rdf-wasm/src/operation.rs`
> * `crates/rdf-wasm/src/query.rs`
> * `crates/slice/src/mapping_support.rs`
> * `crates/sparql-eval/Cargo.toml`
> * `crates/sparql-eval/src/agg_fn.rs`
> * `crates/sparql-eval/src/engine.rs`
> * `crates/sparql-eval/src/error.rs`
> * `crates/sparql-eval/src/eval.rs`
> * `crates/sparql-eval/src/expr.rs`
> * `crates/sparql-eval/src/geo/error.rs`
> * `crates/sparql-eval/src/geo/functions.rs`
> * `crates/sparql-eval/src/geo/functions/standard_tests.rs`
> * `crates/sparql-eval/src/geo/mod.rs`
> * `crates/sparql-eval/src/geo/relation.rs`
> * `crates/sparql-eval/src/governor/mod.rs`
> * `crates/sparql-eval/src/knn/invocation.rs`
> * `crates/sparql-eval/src/knn/mod.rs`
> * `crates/sparql-eval/src/lib.rs`
> * `crates/sparql-eval/src/modifier.rs`
> * `crates/sparql-eval/src/property_fn.rs`
> * `crates/sparql-eval/src/property_fn_eval.rs`
> * `crates/sparql-eval/src/user_fn.rs`
> * `crates/sparql-eval/src/workspace.rs`
> * `crates/sparql-eval/tests/evidence_ingress.rs`
> * `crates/text/src/dictionary_relation.rs`
> * `crates/text/src/relation.rs`
> * `crates/validate/Cargo.toml`
> * `crates/validate/src/geo.rs`
> * `crates/validate/src/geo/cover.rs`
> * `crates/validate/src/geo/decode.rs`
> * `crates/validate/src/geo/encode.rs`
> * `crates/validate/src/geo/geometry.rs`
> * `crates/validate/src/geo/invocation.rs`
> * `crates/validate/src/geo/operation.rs`
> * `crates/validate/src/geo/output.rs`
> * `crates/validate/src/geo/point_index.rs`
> * `crates/validate/src/geo/profile.rs`
> * `crates/validate/src/geo/request.rs`
> * `crates/validate/src/geo/session.rs`
> * `crates/validate/src/geo/tests.rs`
> * `crates/validate/src/lib.rs`
> * `crates/xsd/Cargo.toml`
> * `crates/xsd/benches/numerical.rs`
> * `crates/xsd/src/bigint.rs`
> * `crates/xsd/src/bigint/scratch.rs`
> * `crates/xsd/src/bigint/storage.rs`
> * `crates/xsd/src/exact/binary.rs`
> * `crates/xsd/src/exact/certified_cost.rs`
> * `crates/xsd/src/exact/decimal.rs`
> * `crates/xsd/src/exact/integer.rs`
> * `crates/xsd/src/exact/mod.rs`
> * `crates/xsd/src/exact/rational.rs`
> * `crates/xsd/src/exact/rational/certified.rs`
> * `crates/xsd/src/exact/rational/geometry.rs`
> * `crates/xsd/src/ieee/control.rs`
> * `crates/xsd/src/ieee/dyadic.rs`
> * `crates/xsd/src/ieee/environment.rs`
> * `crates/xsd/src/ieee/mod.rs`
> * `crates/xsd/src/ieee/ratio.rs`
> * `crates/xsd/src/integer.rs`
> * `crates/xsd/src/lib.rs`
> * `crates/xsd/src/math/fixed.rs`
> * `crates/xsd/src/math/fixed/range.rs`
> * `crates/xsd/src/math/fixed/sum.rs`
> * `crates/xsd/src/math/fixed/transcend.rs`
> * `crates/xsd/src/math/floating.rs`
> * `crates/xsd/src/math/floating/fast.rs`
> * `crates/xsd/src/math/floating/packed.rs`
> * `crates/xsd/src/math/floating/word.rs`
> * `crates/xsd/src/math/matrix.rs`
> * `crates/xsd/src/math/mod.rs`
> * `crates/xsd/src/math/roots.rs`
> * `crates/xsd/src/math/roots/tests.rs`
> * `crates/xsd/src/math/solver.rs`
> * `crates/xsd/src/math/taylor.rs`
> * `crates/xsd/src/math/taylor/scratch.rs`
> * `crates/xsd/src/math/tests.rs`
> * `crates/xsd/src/numeric.rs`
> * `crates/xsd/src/value.rs`
> * `crates/xsd/src/wide.rs`
> * `crates/xsd/tests/certified_numerical.rs`
> * `docs/CONFORMANCE.md`
> * `docs/GEO-PROFILE.md`
> * `docs/RELEASE.md`
> * `docs/SPARQL-GOVERNOR-PROFILE.md`
> * `docs/WASM_TESTING.md`
> * `docs/book/po/glossary-zh-Hans.md`
> * `docs/book/po/zh-Hans.po`
> * `docs/book/src/project/releases.md`
> * `docs/book/src/sparql/geosparql.md`
> * `docs/design/h3-v4-grid-contract.md`
> * `docs/design/purrdf-geo-exactness.md`
> * `docs/design/purrdf-geodesic-numerics.md`
> * `docs/design/purrdf-simd.md`
> * `helpers-ledger.toml`
> * `layers.toml`
> * `scripts/check-doc-claims.py`
> * `scripts/check-python-stub-parity.py`
> * `scripts/check-terminal-predicates.py`
> * `scripts/conformance-frozen/vectors-sparql-governors.sha256`
> * `scripts/conformance-matrix.py`
> * `scripts/release-crates.sh`
> * `scripts/simd-asm-manifest.toml`
> * `vectors/sparql-governors/README.md`
> * `vectors/sparql-governors/expected/aggregate-accumulation-fuel-boundary.charges`
> * `vectors/sparql-governors/expected/aggregate-accumulation-fuel-over-bound.charges`
> * `vectors/sparql-governors/expected/aggregate-custom-fuel-boundary.charges`
> * `vectors/sparql-governors/expected/aggregate-custom-fuel-over-bound.charges`
> * `vectors/sparql-governors/expected/aggregate-custom-scratch-bytes-boundary.charges`
> * `vectors/sparql-governors/expected/aggregate-custom-scratch-bytes-over-bound.charges`
> * `vectors/sparql-governors/expected/aggregate-invocation-fuel-boundary.charges`
> * `vectors/sparql-governors/expected/aggregate-invocation-fuel-over-bound.charges`
> * `vectors/sparql-governors/expected/exists-inner-counters.charges`
> * `vectors/sparql-governors/expected/property-function-cooperating-relation-cancel-mid-invocation.charges`
> * `vectors/sparql-governors/expected/property-function-deaf-relation-cancel-mid-invocation.charges`
> * `vectors/sparql-governors/expected/property-function-first-row-fuel-ceiling.charges`
> * `vectors/sparql-governors/expected/property-function-fuel-boundary.charges`
> * `vectors/sparql-governors/expected/property-function-fuel-over-bound.charges`
> * `vectors/sparql-governors/expected/property-function-fuel-zero.charges`
> * `vectors/sparql-governors/expected/property-function-invocation-fuel-boundary.charges`
> * `vectors/sparql-governors/expected/property-function-invocation-fuel-over-bound.charges`
> * `vectors/sparql-governors/expected/property-function-parallel-fuel-boundary.charges`
> * `vectors/sparql-governors/expected/property-function-parallel-fuel-over-bound.charges`
> * `vectors/sparql-governors/expected/property-function-row-fuel-boundary.charges`
> * `vectors/sparql-governors/expected/property-function-row-fuel-over-bound.charges`
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>
> 
> You can disable this status message by setting the `reviews.review_status` to `false` in the CodeRabbit configuration file.

<!-- end of auto-generated comment: rate limited by coderabbit.ai -->

<!-- autopilot:start -->
- [ ] <!-- {"checkboxId":"2708ad07-9f24-4260-9c11-7dc76a49f2e3"} --> <strong title="Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts">Autofix</strong> · Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts
<!-- autopilot:end -->
<!-- tips_start -->

---




<sub>Comment `@coderabbitai help` to get the list of available commands.</sub>

<!-- tips_end -->

