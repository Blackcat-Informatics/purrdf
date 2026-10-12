# WASM assembly call-site diagnosis

Recorded 2026-10-11. Read-only diagnosis of the four missing-symbol failures in
`recovery-resumed-assembly-write.log`; no compilation, tests, gate execution,
shipping-source edits or process changes. The existing gate's demangler, ASM
parser and instruction counter were imported with Python `-B` to inspect the
already emitted artifacts. Only this ignored evidence report was written.

## Conclusion

Both computations remain present on both WASM configurations. The manifest's
call-site selectors no longer name where the compiler emits those computations.
This is a real evidence-registration failure, not evidence that escaping or its
scanner disappeared. Repair the two WASM selector mappings and their explanatory
prose; preserve every scanner measurement, floor, mnemonic requirement,
`max_fma = 0` and `forbid_relaxed = true`. No production-code change or threshold
relaxation is indicated by these observations.

The log completed all seven builds, but the full gate failed with terminal
status 2. These focused artifact observations do not turn that run into a pass.

## 1. `results.json-escape`

Current manifest: `scripts/simd-asm-manifest.toml:3673–3685`, call-site measure
for all seven configurations:

```text
crate = purrdf_lex
purrdf_lex::json_escape::escape_body::<purrdf_sparql_results::json::json_escape_body<purrdf_core::sink::TextSink>
```

Actual production path: `crates/sparql-results/src/json.rs:342` calls
`escape_body(value, JsonEscapes::Minimal, |piece| out.push_str(piece))`.
`crates/lex/src/json_escape.rs:154–175` now makes `escape_body` an infallible
adapter over the sole fallible `try_escape_body` computation. On WASM the
adapter inlines; the emitted driver is the `try_escape_body` instantiation:

```text
purrdf_lex::json_escape::try_escape_body::<!, purrdf_lex::json_escape::escape_body<purrdf_sparql_results::json::json_escape_body<purrdf_core::sink::TextSink>::{closure#0}>::{closure#0}>
```

This exact path was demangled from both emitted result-crate units. The driver
contains a direct call to
`purrdf_lex::scan::find_first_json_string_special`. The existing instruction
counter reports **vector 0, FMA 0, relaxed 0** on each configuration. This is
the same writer-specific escape computation under its new shared fallible home,
not a substitute unrelated scanner or another writer's instantiation.

Attribution is concrete: `git diff origin/main -- crates/lex/src/json_escape.rs`
shows this adapter/driver extraction in the delivery, and blame identifies
`675df1658` for lines 155–175. The results writer itself is unchanged from
`origin/main`. Thus the stale results selector is introduced by the delivery's
shared-home change, rather than a renamed public results API.

Bounded repair: retain the original call-site measure for the five native
configurations which the run already found. Add a separate call-site measure
for `wasm32` and `wasm32-simd128`, `crate = "purrdf_lex"`, with the full exact
demangled path above. Preserve its original scalar floor and prohibition checks.
Update the audit explanation to describe native versus WASM emitted drivers.

## 2. `slice.prefix-json-string`

Current manifest: `scripts/simd-asm-manifest.toml:6110–6120` names
`purrdf_slice::prefix_emit::json_string` for every configuration.

The source helper still exists at `crates/slice/src/prefix_emit.rs:132`; it
creates the string, calls the shared `push_string` with `JsonEscapes::Minimal`,
and returns the string. Its caller `emit_jsonld_context` at line 85 invokes it
for the vocabulary, prefix and namespace strings. The helper was not renamed.

On both WASM targets the helper is inlined into the emitted function:

```text
purrdf_slice::prefix_emit::emit_jsonld_context
```

The parsed body contains **three direct calls** to
`purrdf_lex::json_escape::push_string`, matching those three source call sites.
For example, baseline assembly lines 108283 onward define the caller; line
108768 calls `push_string` after preparing the inlined string allocation.
The existing instruction counter reports **vector 0, FMA 0, relaxed 0** on
both configurations. The SIMD body contains five `v128.load` and five
`v128.store` operations, which the existing gate correctly excludes from
vector-work counts; no counting rule is changed.

`prefix_emit.rs` is unchanged from `origin/main`, as are the two old call-site
selectors. The shared escape implementation it calls changed in this delivery
as described above. Current emitted evidence proves inlining, but without an
attributable baseline ASM artifact this review cannot prove whether this
particular slice inlining choice first appeared here or was already stale under
the current compiler. Do not call it a proven pre-existing failure or a proven
semantic regression. It is now a required gate failure to repair at its actual
consumer mapping.

Bounded repair: keep the existing helper selector for the five native
configurations; measure `purrdf_slice::prefix_emit::emit_jsonld_context` with
`crate = "purrdf_slice"` on the two WASM configurations. Describe the inlined
helper and its original shared writer calls in the audit row. Retain the
original scalar floor, zero-FMA and no-relaxed checks and all scanner evidence.

## Artifact identity and applicability

Both artifact sets are under:

```text
/home/paudley/Active/purrdf/.worktrees/purrdf-geo-qualification/target/simd-asm/contexts/
```

| Configuration | Context | Result unit | Slice unit |
|---|---|---|---|
| wasm32 | `c9ae6ea930fa8418486b571587954e44a982935b4e6cea62f80acb6bad862412` | `purrdf_sparql_results-bc022f984690b815.s` | `purrdf_slice-2eb11d5c50dd11d1.s` |
| wasm32-simd128 | `e6dbcca3c0e01e178e5721b4a6983ab3e811769fc934bf7f224dc4dd6b896d72` | `purrdf_sparql_results-b1aa2793614239a4.s` | `purrdf_slice-9b0dcc0a5ea597c9.s` |

Units reside below each context's
`build/wasm32-unknown-unknown/release/build/<package>/<artifact-hash>/out/`.
The existing parsed cache retained only manifest-selected functions, so its
absence of the proposed new selectors did not establish missing code; the raw
emitted `.s` files above supplied the diagnosis.

`scripts/check-simd-asm.py:1066–1083` uses anchored demangled-path matching;
do not weaken it to substring matching. After the bounded mapping repair and
already authorized duplicate-bench cleanup, run the unchanged normal full gate
with its persistent artifact caches to generate final-source evidence and
document projections. No forced fresh build is justified by this diagnosis.
The gate remains NOT MET until that real execution succeeds.

## Applied repair reread

Root subsequently applied the bounded selector splits. The reviewed manifest
diff retains both original native-five selectors and introduces the exact full
results `try_escape_body` path above and the exact slice
`emit_jsonld_context` path for only the two WASM configurations. The new paths
match the observed emitted evidence. Scanner floors/mnemonics, scalar floors,
zero-FMA and no-relaxed constraints are unchanged. Audit prose now describes the
correct configuration-specific computation; measured cells were not edited.
Source verdict for this repair: PASS; full gate execution remains pending.

The same reread confirms removal of only the two redundant kernel benchmark
files, their two target registrations and their two bench-map rows, as requested
in `recovery-benchmark-duplication.md`. Existing facade benchmarks and the four
distinct kernel targets remain. No additional build or process action occurred
during this reread.
