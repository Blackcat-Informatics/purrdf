<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 5 independent caller and integrity review

VERDICT: BLOCKED

Two required findings remain. Task 5 must not be committed, published as complete,
or used to qualify a PR until they are fixed and independently rechecked.
This review makes no whole-issue, hosted CI, release or merge claim.

## Source and review identity

Worktree: /home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Branch: paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Unchanged HEAD: 39f0d4dce74c7635a8d4675406cc37eb763cb590.
Captured integration base: ce3c07192aba1e36666062c00f958670a827cfb5.
Approved plan SHA-256:
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.

Complete thirteen-file manifest raw/T5-files.sha256 SHA-256:
4b6369ae05670afeb0d11f0a566e75229075831308c2c7253ea760cd72b6d1ff.
Complete tracked-and-new patch raw/T5-source.diff SHA-256:
b74ce609f9ef484c131ff0f9e2cb572275a1677446ed3d5660dbaccae3c983b6.
Independently recalculated the three hashes and checked every manifest entry
against the frozen source twice: all thirteen match. The index remains empty.
No shipping source/index mutation, child delegation, commit, push, forge post,
model/lifecycle action or private memory write occurred.

Read applicable AGENTS.md, parent .baseline/.goals, Stage 1/Stagectl and their
quality/validation/delegation/no-deferrals references; complete issue, intake
analysis, prior-art assessment, approved completeness contract/Task 5, Task 4
review, integration preparation and Task 5 implementation report. Read the
actual changed production paths, affected callers and public regression bodies.
No deeper applicable AGENTS/constitution/ADR file was found in the affected crates.

## T5-R1: an ordinary authored type quad discards the original signed head

Owner: Task 5 implementation agent; actual reader role classification and
compact::detached_signature_pairs. REQUIRED.

compact.rs:496-504 treats any preceding IRI-typed rdf:type Compaction quad as
sufficient to mark a streamable index as packaging. reader.rs:1008 stores that
observation; both reader paths apply it to their Signature. compact.rs:601 then
excludes the signature from the authorship union.

Executed counterexample: use the public Writer with layout=streamable; author
ordinary content with rdf:type stream:Compaction AND a foreign application
predicate, then sign the final index with the genuine author key. The fold has
no diagnostics; public file verification authenticates its sole signature
(ok=true, valid=1, invalid=0, unverified=0). Nevertheless packaging=true and
detached_signature_pairs returns zero pairs.

The same public source then goes through compact_and_certify and
verify_compaction. The produced pack has zero carried pairs, the ORIGINAL
authorship COSE is absent, and ALL SIX report fields are true. This is actual
history loss hidden by successful certification, not a private-helper conjecture.
The RDF projection's own contract explicitly allows ordinary content to use the
reserved type IRI when foreign predicates distinguish it from provenance.

Required correction: distinguish actual packaging provenance from ordinary
content using its semantic shape/position rather than a type quad alone.
Preserve the exact authored index signature in this neighbor. Keep the shared
materializing/evented observation coherent and segment-local; do not fix this by
forbidding valid ordinary RDF content or discarding genuine packaging separation.

Narrowed acceptance: the original public reproducer must preserve the exact
author pair before/after real compaction and certify it; both reader paths agree.
Retain positive genuine pack/repack and pack-plus-new-authored-tail cases,
including composite authorship and actual wasm execution.

## T5-R2: malformed or ambiguous root commitments certify successfully

Owner: Task 5 implementation agent; RDF signatures_bound_ok and its root
resolution/shape boundary. REQUIRED.

gts_certify.rs:566-589 resolves a predicate by the first matching lexical value
and returns the first object's unrestricted lexical value. signatures_bound_ok
at 608 uses that result as the root, without checking literal kind or ambiguity.

Executed counterexamples rebuild a genuine signed pack using the PUBLIC Writer
and its actual final ordering index. They retain the exact original carried
signature set, correct root value and genuine packaging key, so the framing,
layout, cryptography and content-projection checks are valid. A positive
unchanged reconstructed pack independently yields all six report fields true.

Two malformed neighbors ALSO yield no diagnostics and all six fields true:
1. Add a SECOND, contradictory root value to the SAME Compaction subject.
2. Replace that root's literal object with a blank node carrying the same lexical
   digest spelling.

The root binding is therefore not validating the claimed RDF commitment shape.
Cryptographically valid packaging cannot make ambiguous/nonliteral metadata
an unambiguous literal root.

Required correction: resolve root commitments by actual IRI identity and the
relevant Compaction node, validate the literal/cardinality/encoding shape, and
refuse malformed or conflicting commitments instead of trusting a first global
lexical match. Share the applicable boundary with certificate extraction.
Legitimate historical Compaction nodes can carry their own earlier roots;
do not replace this with a blanket one-root-per-entire-union restriction.
Equivalent vocabulary IRIs at distinct term ids must be recognized, and ordinary
literal/content spellings must not fabricate the predicate or commitment.

Narrowed acceptance: both original malformed neighbors return
signatures_bound=false (or actionable refusal) and all_ok=false; positive rebuilt
pack remains accepted. Exercise equivalent duplicate vocabulary identities and
legitimate multi-hop roots/new authored history, preserving original proofs and
exact carried COSE. Execute relevant current native and wasm public regressions.

## Independent executed evidence

Stage-only public source: raw/T5-review-shape-probe.rs, SHA-256
77d97a275ea5eacf6785da4b91493d44373c190430b25c1a423cef8cc4a322b8.
Manifest raw/T5-review-probe/Cargo.toml, SHA-256
193f03b9a55425b0899ed3b20ebf0fa387dacd571297cb312309544042733f71.
Standalone lock raw/T5-review-probe/Cargo.lock, SHA-256
e4f08890b8899241285d91f9f2c38d996937c9b4f9371fc5c44314ec6aa8e1c2.
This lock binds the actual path crates for a reproducer; it is not workspace
dependency qualification. Opt-level 3, assertions and overflow checks remain
enabled. CARGO_BUILD_JOBS=4 caps CPU concurrency only.

Initial execution: cargo run --offline --manifest-path with the absolute
Stage manifest; terminal EXIT 101 from the THREE actual expected-refusal/
preservation assertion failures. Original source is preserved verbatim at
raw/T5-review-shape-probe-initial.rs (SHA-256
9cc26fd8097e09b4cd28d11cc44fe9696453b0da73a70d65b184477584b7167f);
original log raw/T5-review-shape-probe.log.

The complete source adds actual compact/certify execution to the same unchanged
three assertions and positive neighbor. Command:
CARGO_BUILD_JOBS=4 cargo run --locked --offline --manifest-path
<absolute STAGE>/raw/T5-review-probe/Cargo.toml

Terminal EXIT 101, raw/T5-review-shape-probe-complete.log, SHA-256
ae3a93c78cfd7ed0bdd63e4256f5a960b6c39eadd7588b76b3cb35b04135fc83.
This is a FAILED execution, not a passing check. It reports original signature
preserved=false and a successful all_ok certificate for the R1 counterexample.
No probe exists in shipping source. Keep both logs/source versions on remediation.

## Other observations and qualification adjudicated

The sealed PackagingSigner requires supported signing; its private seal blocks
external unsigned implementations. CompositePackaging requires owned key,
deliberate textual kid and provider. One compact_streamable body delegates its
mandatory final index to the actual Writer; the composite implementation calls
the fallible hedged add_index and propagates entropy/native errors. One
compact_and_certify body uses that contract and preserves the certificate's
textual packaging_kids schema. Binary carried authorship remains opaque.

The strict carried decoder now checks exact literal sourceFrame/cose fields,
equivalent vocabulary ids, malformed encodings and shared strict Sign1; its
Result reaches the production root/proof/compaction/certification paths.
Typed cryptographic verification uses the sole parsed-envelope/core resolver.
Actual final-index authentication rejects missing/unsigned/contradictory ordering,
invalid/unresolved observations and unrelated valid survivors in the executed
public suite. These are useful improvements; they do not cure the two findings.

Adjudicated actual frozen-source native logs: RDF 42 cases, GTS 40 cases,
compact module 12 cases; final wasm/Node RDF 7 groups and Writer/profile 13
groups genuinely executed. Read their test bodies and nonzero case counts.
The seven RDF public groups cover mixed/binary authorship, both packaging
algorithms, repacks, component corruption, wrong/unresolved keys, entropy errors,
strict carried nodes and literal-class neighbors, but omit the counterexamples
above. Existing Ed/streamable/dictionary byte-oracle assertions remain meaningful.
Replacing fake COSE bytes in the leaf-order test with real signed envelopes
retains exact independent sort/leaf-hash assertions under the stricter boundary.

Read final all-target clippy, strict docs, native binding compile and helper
census logs (81 jobs, 91 distinct rows, 1819 files). Builds are compile evidence,
not binding runtime. Primary primitive/combiner/fixture/dependency/feature inputs
are unchanged; earlier independent crypto qualification is not repeated or
relabeled. Initial runner-omission exit 101 did not execute wasm and remains a
failure distinct from later real runtime success.

No discretionary full workspace gate, whole binding runtime, hosted CI, hook,
publication, hardware timing/formal certification or post-drop memory experiment
was run by this reviewer. Caller entropy quality/freshness/full fill and dedicated
keys remain caller obligations. Conditional shared-vector and allocation refresh
remain part of approved final qualification. Two required Task 5 defects must
be fixed now; no scope cut or deferral is authorized.
