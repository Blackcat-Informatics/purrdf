# Documentation and measurement provenance review

**ACCEPT.** Read-only review on 2026-10-03 of the unstaged public-contract and evidence annotations in `/home/paudley/Active/purrdf/.worktrees/387-blank-scope-investigation`. Current HEAD observed: `aa529ee697baf39aaea2148db25bc03dfff66e5e` (`Preserve scope evidence columns in Markdown table`). Executable comparison base: `92842627b840a755a3fca31a78d8aa274a34fa4b`. No repository edits, builds, commits or GitHub mutations were performed by this review. Pending SIMD table cells and manifest repair are excluded from its acceptance.

## Executable identity

The entire base-to-current diff has only four Rust files, documentation/evidence JSON, and the separately owned SIMD manifest. Every changed line of each of the four Rust files is a documentation comment. Independently removing only lines beginning with `///` or `//!` leaves an exactly byte-identical line stream against the comparison base. No declarations, executable bodies, attributes, tests, imports or signatures changed.

| Rust source | Captured/base SHA-256 | Current documented SHA-256 |
|---|---|---|
| `crates/rdf-core/src/blank_label.rs` | 454f34898272d5651db0ac75685855ba0ce15d38988743452c7eb26e647d445e | cbfc0c17bccff23dd1e2a6585069bb87bfc10ed66fbfbb5413b01f1fe2d711e7 |
| `crates/sparql-algebra/src/scope.rs` | e22db9620a973dc873877e14aa5c757b3771adcaf3f128143df3b994828c6246 | c4fd672e1e28197995d340cfd669a6e2dc4d3110ff5c64bc8841d1ff32220f6b |
| `crates/sparql-eval/src/eval.rs` | f096ccae8851104a48df146132023854a4901f2a874ed3d283e859bf79e5a681 | a2320fbf17fb1fa7ca822de8a86d49050464eed0976b8b65549df53b313676c8 |
| `crates/sparql-eval/src/remote.rs` | 16d4e48616d65ce94194566b35afab22463d4bf9824afe1d0a062a0e1b027a3f | 2e72a02d4773d71fed5d3acb2d6b5bb2db97d4c6c6e0dbc490f10e42951f4320 |

The first row is a base hash, not a falsely asserted benchmark-capture record: neither benchmark archive lists `blank_label.rs` among its captured source files. The other three old hashes exactly match both the retained measurement source records and the comparison-base files.

## Contract accuracy

* `blank_label.rs` adds the real `service` mint stem and correctly attributes the final legal digit to the completed `{prefix}{stem}{n}` label, rather than the alphabetic stem. The prefix helper remains the authoritative lexical implementation.
* `scope.rs::visit_spine_leaves` actually constructs `WorkList<_, 8>`. It pushes right then left and pops left first, stops at the first matching leaf, and can exceed eight pending nodes. `WorkList::push` places further pending nodes in its spill `Vec`. The correction accurately removes the unconditional zero-allocation claim for a no-rename pattern: deep pending work can allocate heap scratch even though machine-stack usage stays bounded. It says a deeper spine *can* spill, not that depth alone always forces a spill.
* `eval.rs` separates formatting from freshness. The shared mint seam increments the candidate counter, checks the lazy reservation index and reverse lookup of `TermValue::Blank { label, scope: DEFAULT }`, skips occupied identities, reserves accepted candidates and observes the scratch budget. The optional prefix is deterministic caller data, but prefix choice can change which counter values are skipped; it does not promise the same suffix as an unprefixed evaluation. The four documented stems are actual users of the seam.
* `remote.rs` now describes the actual fallible standalone SELECT carrier API. `try_pattern_to_select_query` validates representability and invokes the hygienic rendering home. Hidden witnesses receive legal internal names with a visible projection; a zero-visible-column body receives a constant unit transport column. Successful response handling clears the transport variables and each row's cells without changing the row vector's length before governed ingestion. The comments correctly describe bag preservation of this schema restoration; they do not weaken later governor/source failure handling.

The design document corrects the Markdown pipe in the path-alternative example and explains the post-capture comment changes. Its finite prototype, unsupported boundaries, native-only allocation/timing bounds and wasm layout distinction remain intact. No new normative claim or broader semantic proof is introduced.

## Immutable evidence and exact drift annotations

For each JSON file, this review parsed the current archive and the exact comparison-base archive, removed only `environment.post_capture_documentation` from a deep copy of the current object, and required complete object equality. That assertion passed. The diff independently shows only the added annotation object, preserving the original scalar spellings. Thus all original environment strings, dates, compiler/hardware/load identities, source hashes, capture heads/working-diff identity, profile and workload records, measurements, intervals, outlier records and sample values are preserved.

* `scope-benchmarks.json`: all 30 measurement payloads retain 100 samples each, totaling 3,000. Of its ten captured source records, exactly `scope.rs` has a current full-file hash drift. Its annotation's captured and documented hashes match the table above exactly; the other nine records still match current files.
* `scope-runtime-benchmarks.json`: all three workload payloads retain ten baseline and ten current samples each, totaling 60. All baseline and current capture records remain unchanged. Of the nineteen current capture source records, exactly `eval.rs` and `remote.rs` have current full-file hash drifts. The annotation identifies precisely those two, with no omitted or invented drift and exact hashes matching the table.

The archives explicitly say that timings refer to the original recorded captures, executable bodies are unchanged, and no additional performance measurement is claimed. The design prose preserves the original capture identities and points to the separate annotations. Original head-at-capture values are not relabeled as the current HEAD. The runtime table remains two native host observations, with the moving blank-free control and severe outliers explicitly limiting causal interpretation. The comparative timing table remains prototype costs without a speed threshold or cross-hardware claim; wasm layout/test evidence is not represented as wasm timing.

Reviewed current artifact SHA-256 values:

```text
8e0cb6c26abea68c683750894fc1ae479e7b3f84e2cb32a072400031ee2d8a79  docs/design/purrdf-sparql-scope.md
38be2f1161df20b641aad0931356c8c45571b7d2c1f7b83d2680e8982fa04d25  docs/design/evidence/scope-benchmarks.json
2f69ee869b45d78e50018695b434335757c31c7bf4ca63909e81802b68093e0a  docs/design/evidence/scope-runtime-benchmarks.json
```

`git diff --check` also passed. This was source/provenance review only, not execution qualification.

## Remaining gate boundary

Root reports that the first full assembly `--write-doc` attempt refused its overall result after a mid-run commit changed source identity. Individual seven-configuration symbol/floor observations do not constitute a passed full gate. This review does not infer an assembly pass or instruction-count/compiler parity from those observations, nor from unchanged executable source. Root must freeze the new documentation identity and rerun the normal generation, comparison/report verification and applicable hosted qualification. No toolchain change, gate weakening, fabricated counts or replacement benchmark capture is authorized by this acceptance.

The separate issue 384 review remains unchanged at `/tmp/purrdf-384-compliance-review.md`, SHA-256 `fb8385d3c5578d9555f30f88b45441c4c9dd096dcbe82e22fd33d000cd133278`.
