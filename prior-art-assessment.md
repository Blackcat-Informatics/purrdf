# Independent prior-art assessment

This is a read-only source assessment of the captured complete issue (zero
comments), generated brief/prior-art coverage, intake observations, published
profile notes and independent issue analysis. Standing AGENTS.md, main-root
`.baseline` and `.goals` apply. No source edits, builds, tests, Java/JRE execution,
UBA acquisition, forge refresh, Git mutation or memory mutation occurred. This
Stage assessment is the only written artifact.

## Already implemented and reusable

| Existing home | What it provides and what it does not prove |
|---|---|
| `crates/bench` | Unpublished original Rust benchmark-tool home, with normal hash/IRI/XSD edges and test-only RDF/lex/testkit edges. Its `bench-corpus` binary already has strict argument admission, checked buffered writes and actual CLI tests. Its mixed scale profile is a separate workload, not a university generator to rename. Preserve that profile and its frozen digest. |
| `purrdf_hash::mix` | One specified deterministic integer home: `splitmix64_next`, `splitmix64_step` and finalizer. The counter stream and self-composed stream differ; explicitly select and version a native sampling law. No Java random implementation or external RNG is required. Existing scale `draw(seed,tag,index)` illustrates index-pure design, not an established LUBM law. |
| `purrdf_iri` and re-exported lexical homes | Actual absolute-IRI validation and the shared `term_syntax::write_iri`, `write_literal`, `iri_escape`, `literal_escape` homes. Use these for caller ontology/document-base content rather than interpolating unchecked strings or creating another escaping body. W3C vocabulary spellings come from the existing vocabulary home. |
| `purrdf_hash::{blake3,hex,frame,Domain}` and `purrdf_lex::json::record` | Shared content identities, framing and strict typed manifests. The existing scale manifest deliberately records configuration without digesting payload; copying its label does not establish output completeness. A native multi-file receipt must verify actual files/bytes and exact configuration. First-party lex is available through existing IRI re-exports; any explicit dependency/layer change must be accurately declared. |
| Production RDF reader/writer and CLI | The current lane already converts each generated document through `purrdf convert` with an explicit document base. Parser/round-trip tests can reuse the existing test-only RDF edge. Direct native N-Triples generation is possible without writing an RDF/XML codec; if selected, the real lane must still exercise the documented production conversion path and retain the document metadata contract. |
| `scripts/lane-common.sh` | Sole shared locale, checked write/mkdir/nonempty payload, executable probe, scratch cleanup, certificate revocation, integer admission, query-set integrity and pin lookup laws. Its Rust integration tests test actual shell entry points and inventory both helper directions. Preserve the home and WatDiv/scale callers. |
| `scripts/benchmark-acquire.py` | Verified ignored external cache with corrupt-hit hard refusal, named artifact selection and independent upstream ontology/query digests/license posture. Retain these protections and unrelated WatDiv artifacts/pins. Remove UBA/Linux-fix production acquisition and related obsolete self-test assumptions. Existing tooling can shrink; new tooling/tests must be Rust. |
| `scripts/lubm-queries.py` | All 14 queries, five mechanical transformations (prefix rebind, bare IRI brackets, projection commas, pattern commas, trailing spaces), original provenance and regime index. Preserve their semantics and query-set checks. Do not vendor externally fetched query text or replace the workload with synthetic fixtures. Any newly implemented normalization/oracle machinery belongs in Rust. |
| Existing bench integration tests / testkit | `corpus_cli.rs` tests real generator invocation; `make_bench_lanes.rs` reaches documented make admission failures; `lane_common_laws.rs` directly reaches shared helper failures. Testkit owns new temporary paths and seeded test support. Old tests' early refusal coverage does not constitute a successful LUBM run. Old bespoke scratch helper is not a template for a new duplicate. |

Source inspection confirms there is no native LUBM implementation in the current
bench crate. Existing production LUBM step 3 unpacks UBA and inspects class
collections; step 4 invokes Java; subsequent steps repair misplaced Windows-style
filenames. These are replacement targets, not useful native implementation bodies.
Current Makefile and workspace CI run acquisition/normalizer self-tests; current
Rust lane tests and benchmark documentation retain Java/JRE assumptions. A
semantic retirement sweep must include AGENTS.md command prose, Makefile,
`crates/bench/README.md`, `docs/BENCHMARKS.md`, lane-law design/test comments and
acquisition command examples. Do not confuse unrelated JavaScript surface tests
with a Java dependency.

## Missing complete delivery

1. **Original schema-aware generator.** Define a named native profile, deterministic
   sampling/rounding and finite assignment algorithms using the published profile
   facts already captured. Generate university/department membership, category
   distinctions, disjoint teaching ownership, research groups, student course
   assignments/advisors, assistant roles, degree origins, publications and coauthors.
   Preserve inferred-only Student/Professor/Chair behavior. Names and references
   must remain unique across departments/universities and resolve to valid targets.
   Published ranges are not permission to copy UBA coefficient arrays or bodies.
2. **Stable seed/index/configuration semantics.** Seed a university from its absolute
   index and native seed so generating it alone and within a larger range agrees.
   Check count/index arithmetic before creating partial output. Version stable
   ordering, file naming and serialization; avoid path, locale, time or default
   randomized-hasher leakage. Native identity includes profile, seed, range,
   ontology and document base. An ontology change must affect actual schema terms,
   not merely the manifest; a document-base change must affect intended metadata.
3. **Independent graph-derived statistics.** Count the graph actually parsed from
   emitted bytes, not the generator's own counters. Cover every published range,
   ratio and relationship constraint across a finite stated seed/index matrix.
   Distinct assignment and integer rounding are particularly easy to get wrong:
   repeated random picks must not quietly reduce course loads or TA cardinality;
   publication coauthor assignment must enforce the graduate 0–5 constraint.
   Validate advisor class restrictions, faculty degree triples and referential
   closure. Approximately 103k rows alone is not schema/statistical acceptance.
4. **Actual CLI, multi-file receipt and failures.** Exercise the executable in
   separate directories/processes and compare every filename and byte. Test
   duplicate/missing flags, malformed IRIs/numbers, zero universities, range
   overflow, unavailable executable, failed writes/flush, missing/empty/truncated
   files, extra unexpected files and inconsistent receipts. A failed run must
   not preserve a usable native certificate or publish an empty-corpus digest.
   If no reuse is needed, do not add speculative cache machinery; if reuse exists,
   bind it to checked payload/configuration and shared revocation laws.
5. **Real lane and independent answers.** Wire `make lubm` to native generation
   and the actual CLI without UBA downloads, unzip/class inspection or Java. Run
   the real default lane and nondefault university/index/seed/base/ontology cases,
   including relative/absolute output and prebuilt CLI paths. Derive native Q14
   from distinct undergraduate subjects in the actual graph and Q1 from actual
   GraduateStudent/takesCourse bindings for the normalized concrete course target.
   Independently check those sets/counts against query output; do not bless engine
   output as its own oracle. Historical Q1=4/Q14=5916 and version-keyed UBA corpus
   hashes are invalid native acceptance pins and must retire from this path.
6. **Preserved regimes and truthful report.** Keep Q1/Q2/Q14 no inference,
   Q3/Q4 RDFS subclasses, Q5 RDFS subproperties/subclasses, Q6–Q10 derived Student,
   Q11 transitivity, Q12 realized Chair and Q13 inverse/subproperty alumni.
   Keep full/one-file/slice dataset identities distinct. A nonzero query exit is
   CANNOT-EXECUTE, malformed successful stdout BAD-RESULTS, and a slice answer
   cannot be promoted to full workload acceptance. A concrete Q1 target outside a
   nonzero university range can legitimately yield zero; non-vacuity must not
   assert universal Q1 positivity. Native comparisons require exact same native
   bytes, query identities and regime, not historical UBA row counts.
7. **Retirement and qualification.** Delete obsolete production dependencies,
   assumptions and pins while preserving external ontology/query licensing and
   WatDiv behavior. Run meaningful Rust generator/graph/CLI/lane-law tests,
   affected hygiene/layer/metadata checks and the settled required full gate.
   Source inspection and stand-in admission tests are not a production lane PASS.
   Report-only performance remains measured evidence rather than a new CI timing
   threshold. Host tooling must not regress shipping wasm/dependency constraints.

## Historical evidence bounds and risks

The brief searched the complete issue plus zero comments, found no linked issue
references or cited ADRs, searched three broad forge-title terms, and examined
trailers in only the last 200 commits. Its 12 `none` and one `performance`
occurrences are not verified Java-generator defect recurrences. Exact recurrence
and the count of prior unpublished attempts remain **UNKNOWN**, not zero. Related
closed performance/helper issues are useful context, not proof this generator
was delivered. Old source comments describing three log-redirection attempts
show why existing shared failure laws matter; they do not establish native
generator acceptance or permit skipping an actual successful lane run.

The largest risks are a native generator that merely approximates volume while
violating relationship constraints; copying an implementation under the guise of
specification matching; retaining historical UBA oracle/payload identity; allowing
unchecked custom IRIs/output paths to undermine determinism; and testing only
helpers instead of the production lane. No Java baseline or UBA inspection is
needed or authorized. The primary profile/paper URLs and specification facts in
`published-profile-notes.md` suffice for original work; unresolved required facts
must be resolved before completion, never silently treated as accepted gaps.

Assessment: proceed with an original generator in the existing unpublished Rust
tooling home, independently validated emitted graphs, explicit native identity
and full production-lane replacement. Existing native/shared homes substantially
reduce implementation needs, but none currently closes the delivery.
