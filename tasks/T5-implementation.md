<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 5 implementation and executable caller evidence

Status: SUCCESS

The approved implementation unit is complete and source is frozen for independent
review. No commit, push, forge post, delegation, lifecycle action or private memory
write occurred. This implementation verdict is not an independent review, whole
issue completion, hosted CI, PR, publication or merge verdict.

## Exact source identity

Worktree: /home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Branch: paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Unchanged HEAD: 39f0d4dce74c7635a8d4675406cc37eb763cb590.
Captured integration base: ce3c07192aba1e36666062c00f958670a827cfb5.
Approved plan SHA-256:
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.

Complete THIRTEEN-file manifest: raw/T5-files.sha256, SHA-256
4b6369ae05670afeb0d11f0a566e75229075831308c2c7253ea760cd72b6d1ff.
Complete tracked-and-new-file patch: raw/T5-source.diff, SHA-256
b74ce609f9ef484c131ff0f9e2cb572275a1677446ed3d5660dbaccae3c983b6.
The patch explicitly appends the new integration target using git diff --no-index;
that command's exit 1 means the new file differs from /dev/null, not an error.
All thirteen manifest entries match the frozen source; raw/T5-manifest-check.log.
Index remains untouched and empty. The selected .stage directory stays separate.

Files: GTS compact.rs, model.rs, reader.rs, policy.rs, fixture.rs, cose/tests.rs;
GTS tests compaction_signatures.rs, cose_composite.rs and composite_support/mod.rs;
RDF Cargo.toml, gts_certify.rs, tests/gts_certify.rs and NEW
tests/gts_composite_compaction.rs. No dependency, lockfile, semantic feature,
primitive/combiner body, governed vector, exemption, version, domain declaration
or generated projection changed.

## Mandatory packaging through the actual authoring body

CompactionParams is generic over a sealed PackagingSigner, defaulting to the
existing owned (Ed25519 key, String kid) tuple. Existing tuple callers remain
source coherent and produce identical frozen bytes. CompositePackaging owns a
dedicated composite key, deliberate textual kid and REQUIRED provider. Its
constructor needs all three; an external unsigned signer cannot implement the
private sealing trait. Signing is mandatory, never Option or a default.

ONE compact_streamable body authors the existing frames unsigned, then delegates
the final index to the required signer. The Ed tuple installs the original signer
and calls the actual Writer index method. Composite packaging converts that same
Writer into its hedged mode and calls its fallible add_index. Provider/native
signing errors propagate to the existing typed CompactRefusedError boundary with
actionable context; no output bytes are returned on failure. There is no alternate
compactor, entropy fallback, host RNG or second signing implementation.

ONE RDF compact_and_certify body accepts that same contract, records kid() as the
existing certificate's textual packaging_kids, and delegates to compact_streamable.
Certificate wire schema is unchanged. Binary carried authorship is resolved
losslessly by the typed keyring and never converted to invented text.

Owned randomizers and signing keys retain the existing Writer/primitive Drop
guards. Unavailable and partially failing providers execute real certification
calls returning Err and no artifact. A deliberate fresh-provider invocation
successfully produces an independently verified pack. The lower native signing
error propagation is inspected code wiring, not an injected budget-exhaustion
execution at the compactor. Primitive budget refusals retain earlier Task 2
evidence; no new exhaustion execution is claimed.

## One strict carried-evidence and identity boundary

detached_signature_pairs now owns the sorted/deduplicated authored-and-carried
union. Carried typed nodes require exactly one literal sourceFrame and cose;
missing predicates globally, repeated fields at equivalent distinct vocabulary
ids, nonliteral objects and invalid encodings refuse explicitly. Every envelope
uses strict parse_sign1; malformed fresh signature observations also refuse.
The public leaf/proof helpers become Result so malformed evidence cannot turn
into an empty set or silently missing proof. Production streaming_index and RDF
certification use this same boundary. Existing helper callers handle its result.

IRI identity uses one Term-owned iri_value home: the original private policy
implementation moved there, with the old body removed. Compaction, its observed
role and RDF projection share that exact kind/value operation; the existing
sealed-source policy caller also uses it. Duplicate vocabulary ids are recognized.
Literal rdf:type objects resembling reserved class IRIs remain real content and
cannot fabricate provenance. The earlier projection defect was an implementation
source observation, not a claimed pre-fix runtime execution.

Signature gains a reader-observed packaging bool, explicitly not authentication.
The sole shared h_quads observes compaction provenance in segment-local state;
materializing and evented readers both classify the corresponding streamable
index via the same role helper. The streaming state retains that flag between
frames and resets it for each new segment, even though evented reading does not
retain quads. Fresh authored history after an earlier pack is retained instead
of losing all fresh signatures merely because a union contains Compaction.
Both read paths agree over an actual pack followed by a fresh streamable signed
segment, including its newly authored index. Existing constructor consumers set
the observation explicitly. Canonical JSON/vector wire projections stay unchanged.

RDF signatures_verify_ok routes strict parsed envelopes to SignatureKeyring and
the sole typed COSE verification home. Legacy text/Ed maps still implement that
same contract. signatures_bound compares exact pre/post authorship pairs and the
root/proofs; presence of some other valid signature does not substitute for
preserving the original evidence. Repack packaging stays distinct from authorship.

Packaging acceptance obtains the actual final index identity/type/layout from
the existing replication inventory, authenticates its exact signature with the
typed key, and requires clean shared file verification with zero invalid or
unresolved observations. It rejects layout/framing diagnostics, missing/unsigned
final index, contradictory signed ordering count and unrelated valid survivors.
The report retains separate authorship and packaging booleans. OpenPGP discovery,
trust, encryption and opaque capability behavior are unchanged.

## Executed final qualification

All Cargo commands used CARGO_BUILD_JOBS=4, ordinary repository profiles and
warning policy. Native target x86_64-unknown-linux-gnu; actual runtime target
wasm32-unknown-unknown. raw/T5-environment.log captures rustc
1.100.0-nightly (4b6d04e706108ccfeafe2547fbe857dfe8972bad), LLVM 23.1.1,
Cargo 1.100.0-nightly (7941be6fb), Node v26.10.0 and wasm-bindgen 0.2.125.
The test profile inherits opt-level 3, debug assertions and overflow checks from
dev; no gates, warnings or checks were weakened.

- raw/T5-rdf-complete.log: cargo test -p purrdf-rdf --test
  gts_composite_compaction --test gts_certify --test streamable_vectors --test
  dict_vectors --test pinned_dict_compaction; terminal exit 0, FORTY-TWO cases
  across 7+17+4+11+3. Frozen streamable and dictionary reproductions remain exact;
  the tests read the governed corpus and never regenerate its files.
- raw/T5-gts-complete.log: cargo test -p purrdf-gts --test compaction_signatures
  --test pinned_dict_compaction --test cose_composite --test hedged_writer;
  terminal exit 0, FORTY cases across 6+10+11+13.
- raw/T5-final-gts-unit.log: cargo test -p purrdf-gts --lib compact::;
  terminal exit 0, TWELVE real compaction/encoding cases, 108 intentionally filtered.
- raw/T5-final-runtime-wasm.log: cargo test -p purrdf-rdf --target
  wasm32-unknown-unknown --test gts_composite_compaction with the exact worktree's
  scripts/wasm-test-runner.sh runner; terminal exit 0, SEVEN groups actually run
  in wasm/Node, not compilation alone.
- raw/T5-policy-wasm-complete.log: the same runner executes purrdf-gts
  hedged_writer; terminal exit 0, all THIRTEEN groups, including unsigned/profile
  refusals, sealed-source exception, OpenPGP and authenticated opaque neighbors.
- raw/T5-final-clippy.log: cargo clippy -p purrdf-gts -p purrdf-rdf
  --all-targets -- -D warnings; terminal exit 0. Includes affected generators,
  benches and integration targets as compilation/lint evidence.
- raw/T5-final-docs.log: RUSTDOCFLAGS='-D warnings' cargo doc -p purrdf-gts
  -p purrdf-rdf --no-deps; terminal exit 0 on the frozen production inputs.
- raw/T5-final-consumers.log: cargo check -p purrdf-wasm -p purrdf-capi
  -p purrdf-python; terminal exit 0. This is NATIVE binding compilation, not
  binding runtime or a whole-workspace wasm claim.
- raw/T5-helpers-complete.log: check-shared-helpers.py terminal exit 0;
  81 jobs, 91 distinct rows, 1819 files, no open copies or escaping includes.
- raw/T5-format.log and raw/T5-tracked-whitespace.log: terminal exit 0.
  raw/T5-new-whitespace.log is empty with no-index exit 1 (expected new-file
  difference, no whitespace diagnostics). raw/T5-deferral-scan.log has zero
  added marker hits (rg exit 1 is the expected no-match result).

The public native/wasm groups execute real mixed composite/binary-id and Ed
Writer source -> both packaging algorithms -> compact/certify -> reader/typed
verify_compaction; exact original COSE survives, content digests and selective
MMR proofs verify, and a second repack preserves those same original claims.
Each composite component is separately corrupted in carried and packaging
evidence; wrong algorithm, different same-type composite key and unresolved key
refuse. An unrelated valid frame signature cannot conceal unsigned or invalid
index/evidence. A validly rehashed/re-signed contradictory index still refuses
despite TWO crypto-valid signatures. Provider failures, malformed/ambiguous
carried nodes, valid duplicate vocabulary and literal-content neighbors execute.
The full primary path also runs with host clock/entropy sealed; fixture draws
are explicit test inputs, not production cryptographic-quality claims.

Bounded reused evidence: raw/T5-layers.log (207 edges, 42 members) and
raw/T5-shards.log (six shards, 42 members) passed after the sole Cargo registration;
their Cargo/layers/shard inputs remain byte-identical. Public documentation example
checks raw/T5-gts-public-doc-tests.log (one no-run compile and one missing-provider
compile-fail) and raw/T5-rdf-public-doc-tests.log (one no-run compile) passed.
The subsequent shared IRI-home movement did not change these example bodies,
signer/API signatures or imports; final strict docs and actual caller compilation
qualify the final production inputs. These three doc checks are compile evidence,
not runtime demonstrations. No stale whole-package pass is relabeled current.

## Preserved failed attempts and workflow limits

Initial check failed on an obsolete now-unused RDF private helper; it was removed.
Initial new harness compilation failed on test API assumptions (private base64
encoder, boxed key ownership, generic strings/CBOR/blank constructor names); it
was corrected against the actual homes. The old byte-only leaf-order fixture
was replaced with real signed COSE inputs under the now-strict boundary.
The first malformed-node assertion overlooked the reader's existing
PositionConstraint refusal for a literal predicate; the test now observes that
real refusal and the shared missing-field boundary. The first added ordering
probe incorrectly looked for payload key p; inspecting Writer identified its
actual d field and the corrected native/wasm contradiction test passes.
Initial clippy type-complexity and assert-is-empty findings were fixed without
allows; initial strict rustdoc private-link failure was fixed without weakening
warnings. All original failed logs remain raw/T5-*-initial/second/final logs and
are historical failures, not final qualifying evidence.

raw/T5-public-wasm-initial.log has exit 101 and explicitly NEVER EXECUTED its wasm
binary because that invocation omitted the runner. Corrected invocations use the
actual pinned runner and terminal logs prove their nonzero executed case counts.

No full workspace make check, release gate, hosted CI, hook, commit, push, PR,
publication, model/GPU/lifecycle action, hardware timing audit, formal FIPS
certification or post-drop memory experiment was run by this implementer.
Provider quality/full-fill/freshness and dedicated key policy remain caller
obligations. Original compiler/register/historical copies are not claimed erased.
Pinned provisional composite mapping and conditional shared-corpus evidence
retain earlier task records; external registration/vector refresh belongs to the
approved final qualification unit, not a claim made by this implementation.

No required Task 5 implementation or focused acceptance behavior remains undone.
Parent must independently review this frozen identity, then perform normal signed
hook-verified transport and publication. No source mutation will occur before
that adjudication.
