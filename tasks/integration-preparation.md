# Production integration observations

Read-only inspection while Task 2 implementation runs. Source observations are
against Task 1 commit `74bf968ce28da00f9ab6ad054b150c4f5d91da18`; these are
implementation inputs, not task completion evidence or an alternate plan.

Task 3 strict envelope work must replace the existing single COSE parse home.
`cose::parse` currently uses `decode_prefix` without checking consumed bytes,
unwraps any tag, ignores detached payload and does not inspect protected alg.
The generic CBOR decoder intentionally accepts repeated map keys in ordinary
mode; the new COSE header validation therefore needs its own duplicate-label
check. Whole-item CBOR decode is already available. Preserve exact protected
bytes for signature preimages rather than re-encoding received headers.

Task 4 signing occurs in `Writer::add_frame_with_options` before offsets,
types, frame IDs, encoded buffer and previous head are changed. That location
can propagate entropy/signing errors atomically. `add_frame` currently calls
`.expect`, and every convenience method routes through it; adding a fallible
signer without resolving this contract creates a new panic path. Assess real
callers and API contracts before choosing the coherent implementation.

`cose::verify_signatures` resolves a kid before verification and currently
marks parse failures unverified if the resolver is absent. Parse/algorithm
validation must establish invalid first. `verify_file_with_keyring` and its
shared internal verifier currently accept Ed25519-only keys; generalize the
shared path rather than add a second verification implementation. OpenPGP
transport-key discovery stays Ed25519 under the issue's scope.

`SnapshotSigner` is an Ed25519/OpenPGP transport-metadata authoring contract,
not generic composite discovery. Its existing derived Debug exposes its
32-byte secret seed. Resolve this actual secret-redaction defect when touching
authoring signing contracts, without treating composite key discovery as an
implicit new requirement.

Task 5 RDF `gts_certify::signatures_verify_ok` calls `cose::parse` and
`verify_sig` directly over each carried detached signature. Route it through
the strict typed core and algorithm-aware keyring. CompactionParams requires
a packaging signer tuple; preserve mandatory signing when generalizing its
key/randomness contract. Certification must observe complete valid packaging,
not merely the existence of one valid signature while others are invalid.

The pinned IETF ML-DSA-65/Ed25519 COSE example begins after Figure 9 in
`raw/ietf-jose-composite-04.txt`: two zero seeds, AKP algorithm -58 and public
key, protected kid, attached hello payload, representative and composite bytes.
Replay its primitive/representative/encoding without loosening GTS's detached
null-payload contract. It is an external known-answer source, not a shared
GTS vector. Existing governed vectors must remain unchanged.

Composite protocol prefix/label spellings must use the existing `purrdf_hash::Domain`
type where they identify the representative's preimage family. Domain is a
const newtype whose declarations live beside their construction, not an enum
that needs a hash-crate edit. Preserve exact mandated IETF bytes; the local
`purrdf-.../vN` naming convention cannot rewrite a standard's domain. Run the
existing hash-domain hygiene gate for the actual declarations and uniqueness.
# Additional parent observations for the remaining approved caller tasks

Task 6 and final integration must refresh both the conditional shared GTS
composite corpus and live IANA allocation before reporting the provisional
registration/interoperability state. Keep the deliberately pinned draft
construction and primary fixture identities explicit; an earlier allocation
observation is not a current registry readback.

`compact::refusal_gate` checks framing, hash/codec diagnostics, profile and
suppression constraints; it has no supplied verification keyring. Do not claim
that this authoring gate authenticates carried signatures. Task 5's actual
certification verifier must refuse corrupted ML-DSA or Ed25519 evidence using
supplied algorithm-aware keys and preserve source signature bytes.

`gts_certify::verify_compaction` currently computes packaging acceptance from
`verify_file_with_keyring(post_bytes, keyring).valid >= 1`. Inspect the required
ordering/head signature and invalid/unverified counts before choosing the final
predicate; one valid signature must not conceal invalid required evidence.

`compact::carried_detached_pairs` currently skips malformed carried provenance
nodes. Evaluate this behavior in the actual carried-signature/certification
paths touched by Task 5, rather than calling a skipped node authenticated or
claiming that a clean hash chain alone proves signed history.

Task 5 preparation against committed Task 3 plus frozen Task 4 APIs:
`CompactionParams` currently requires an owned Ed25519 key/String tuple; the
single compaction body authors content with the default Writer and installs its
packaging signer only before the trailing index. Generalize that mandatory
signing contract without introducing optional signing or a duplicate compactor.
Composite packaging must request caller entropy and propagate errors rather
than publish an unsigned artifact. Existing Ed caller corpus and generated
streamable/dictionary fixtures must remain qualified and byte-preserved.

`compact_and_certify` similarly destructures the Ed tuple and records its kid
in `CompactionCertificate.packaging_kids: Vec<String>`. Inspect this actual
certificate schema before choosing opaque-id support: do not fabricate text
decoding for binary identifiers or alter existing frozen certificate bytes
without an explicit schema rationale. The typed keyring already resolves exact
optional opaque COSE kids; certification must use that resolver and strict Sign1
home. Mandatory packaging acceptance must identify the actual ordering/index
commitment, not substitute any unrelated valid frame signature.

Actual affected callers include GTS compaction unit/tests/authoring benchmark,
RDF gts_certify, streamable_vectors, dict_vectors, pinned_dict_compaction and
their Rust vector generators. The current public detached leaf/proof helpers
return plain values and silently skip malformed carried nodes; choose a coherent
shared strict boundary rather than making certification trust skipped evidence.
Parent read these bodies; these are implementation inputs, not acceptance.

Parent refreshed the installed merge helper read-only while Task 5 runs:
`/home/paudley/stage/root/bin/ghprsq` resolves to Stage's scripts/ghprsq.
That file and stage-scripts/ghprsq-stage.py have no working-tree diff; their last
tracked commit is `313b43c0a18a09fd2c0c8cb005f631b92a2be166`.
The current helper selects evidence before source cleanliness, captures raw
file bytes/modes with a separate temporary index, verifies the full file tree,
and fails capture before publication. It must receive this exact Stage dir.
No capture/publication was run during this inspection. Stage 3 still requires
all fresh gates, source/PR/input identities, completion audit and final archive
readback; this inspection is not merge readiness.

Parent inspected the actual final local gate and commit hooks while Task 5's
required corrections run. Makefile `check` runs workspace/preserve-order format,
all-target clippy with denied warnings, workspace lib/tests checking, hygiene
and generator drift checks, workspace tests, preserve-order consumer tests,
core hygiene and `make wasm`. The wasm target builds all 31 release libraries
plus the portable bench library, and may skip locally if its target is absent;
final qualification must confirm execution rather than accept a skip.
The normal pre-commit hook checks the staged snapshot's non-Rust ratchet,
Python test placement, formatting and fast hygiene gates. It explicitly does
not run clippy, builds, tests, census or hash-domain checks. Successful hooks
cannot qualify those missing full-gate obligations. No PRECOMMIT_GUIDANCE.md
was found in this worktree. Hook path remains .githooks, signing is enabled.

Current gate-source identities are captured in
raw/parent-final-gate-inputs.sha256. CI's actual file is ci.yaml; its distinct
jobs include workspace, native shards/conformance, docs, MSRV, wasm release,
wasm execution/package, bindings and cross-architecture checks. Native local
checks and current task wasm demonstrations cannot be called hosted CI passes.
The repository has no tracked .cargo/config.toml; a guessed read failed and
the final gate-input manifest was recaptured using verified existing files.
No source/gate mutation or broad qualification was performed in this inspection.

Stage 2 performance input from actual changed code: Folder::h_quads observes
every validated quad through compact::ProvenanceSubjects before delivering it
to a sink. The classifier stores per-subject facts and checks three closed
vocabularies; packaging role ultimately requires a streamable index. This is
an unmeasured hot-path/memory risk, not a claimed regression or required fix.
Its review should consider actual high-cardinality ordinary nonstreamable reads
as well as genuine streamable packs, with bounded representative input and
attributable baseline/current measurements if needed. Do not invent a slowdown
from code reading, skip the semantic foreign-before-type case, or introduce an
alternate parser. Existing actual wasm/native suites establish behavior rather
than throughput. Crypto/security and substantial-module structure also justify
scoped Stage 2 independent specialist judgment rather than a fixed reviewer count.
