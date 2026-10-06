<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent Stage 2 initial completion audit

VERDICT: FINDINGS-OPEN

No new functional defect or missing public composite demonstration was established
on the assigned branch head. The requested functional behavior is demonstrated
by attributable public execution evidence below. Stage 2 exit remains open:
specialist judgments, later complete feedback/check readback and current-base
integration assessment are not yet qualified. These are workflow/evidence states,
not invented crypto bugs or authorized deferrals. A fresh independent exit audit
must adjudicate their outcomes; this initial report does not clear that gate.

## Identity, authority and independent work

STAGE=2; issue 458; PR 464; Blackcat-Informatics/purrdf.
ROOT=`/home/paudley/Active/purrdf`.
W=`/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
S=`/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519/.stage/purrdf-gts-composite-ml-dsa-65-ed25519`.
Branch=`paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
HEAD=`52988974f2d11a40214281648983f9145af7a3fb`;
tree=`43b0817e30a3ad110cd013796a064c177a27e067`.
Approved immutable plan SHA256
`dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.
Complete 52-path source manifest SHA256
`e899501200bd56af128b504c84c9ca1c8681f6c92fe3719815bb6fa635c6567d`;
all paths independently verified against current worktree files.

Implementation base is `ce3c07192aba1e36666062c00f958670a827cfb5`.
Initial PR/CI/review capture base is `0d6575a46088e9420d73654b7b5965b9580c5d36`.
Observed current `origin/main` is `090b14bb8c00e2504078cb694d278ee2d614443a`.
Parent's terminal successful read-only merge-tree predicts tree
`134045aacaeb48a7ac1bfb1d4f3c71f8ae62e2a4` in
`raw/S2-candidate-initial-tree.txt`; that combined tree has not been executed or
qualified by this audit. Head evidence does not prove the combined tree.

Applied the complete Stagectl completion-adversary rubric with Stage 2 and
validation reuse policy. Read applicable root/worktree AGENTS, root baseline
and goals, GTS signature/preservation/timestamp/conformance provisions, complete
literal issue and nine comments, immutable plan and current task/correction
reviews. No applicable ADR/constitution is found in tracked inventory or
captured briefs. Historical-fit was read before this judgment, SHA256
`4c37ac0152f5f6b4fdc0822a5459904ded60bdd92287688dca8e9cf3019cf813`.
The earlier issue analysis has been corrected: the latest issue checkpoint is
issuecomment-6019724668; issuecomment-6019728240 belongs to PR 464. Corrected
`issue-analysis.md` SHA256 is
`9665af0375750f18162cd13a44ba77eabc66e3120f0f7f539d14621d5093baf0`.

Independently read public test bodies, actual production computations/callers,
terminal logs/status records and source/artifact manifests; inspected intervening
diffs; recalculated log hashes and four retained current execution artifacts;
compared the three retained independently formatted NIST primary/frozen record
pairs with cmp (each matched). Ran a fresh branch deferral/laundering scan.
No new full suite or cryptographic execution was warranted by a missing changed
functional closure. Prior execution is explicitly adjudicated rather than
renamed as this auditor's execution. No source/index/forge/private-memory,
GPU/model/service or child-agent operation occurred. Only this report, the
checkpoint-link correction and two selected raw scan artifacts were written.

## Per-criterion execution judgment

The issue requests authoring and verification, not merely rejection of composite
input. Its rationale requires original head signatures to survive rewriting.
The plan concretizes that into real Writer, typed resolution and compaction/
certifier demonstrations. No comment grants a scope cut.

| Literal requirement / accepted contract | Actual public demonstration and observation | Judgment on assigned HEAD |
|---|---|---|
| Accept EdDSA and ML-DSA-65/Ed25519 in one Sign1, honor declared alg | Public COSE suite calls typed sign/parse/verify, inspects tag 18/four fields/null payload/protected -58/3373 bytes, and replays both frozen EdDSA envelopes with exact -8 bytes. Writer suite produces Ed/composite/Ed/composite history; reader folds four signatures; typed file verification observes `(signed,valid,invalid,unverified)=(4,4,0,0)`. | DEMONSTRATED by E1/E2/E3 below. |
| BOTH components mandatory; stripped/zeroed/replaced/swapped refused; no two-signer downgrade | Public verifier attacks each 3309/64-byte half separately, splices other-message and other-key halves, mixes public halves, reverses ordering, strips/truncates/appends, changes protected alg and uses COSE_Sign/wrong tags. All assert Invalid beside valid composite controls. Real Writer-produced file tampering and actual RDF carried/package half corruption distinguish signatures_verify and packaging_sig_ok. | DEMONSTRATED E1–E4; source combiner executes both checks and accepts their conjunction. |
| Exact IETF pairing | Public IETF test computes actual Sig_structure, representative, derived public key and deterministic signature against complete independently extracted external bytes. Published attached example is refused as attached GTS input; equivalent detached envelope verifies the exact unchanged preimage. Encoding is ML-DSA then Ed25519. Prefix/label/zero application context/SHA512 and ML-DSA label context are inspected in actual combiner. | DEMONSTRATED E1/E5, not merely self-generated round trips. |
| FIPS 204 deterministic variant for vectors | Public native/wasm primitive suite executes 25 complete keygen answers, 15 deterministic and 15 hedged signatures, 15 verification verdicts (3 valid/12 refused). Assertions compare every byte and exact counts. SHAKE precursor has four complete NIST and sixteen external boundary answers, independently audited. | DEMONSTRATED E3/E5/E6. |
| Hedged production signing | Actual Writer installs a mandatory caller provider through sealed Hedged mode. Two supplied randomizers yield different authentic COSE/file bytes but identical content IDs. Failure/partial-fill produces typed errors, unchanged existing bytes/head and no index advancement across thirteen append conveniences; explicit recovery succeeds. Composite packaging errors return no pack; valid retry certifies. | DEMONSTRATED E2–E4; provider quality/freshness remains explicit caller responsibility, without deterministic fallback. |
| Preserve original signed history and mandatory packaging across real consumers | Public RDF suite authors genuine mixed source via Writer, reads/verifies it, packs with Ed and composite, checks exact original author pairs/content digest/MMR proofs, repacks and runs verify_compaction with all six checks true. Corruption, wrong/unresolved key, unrelated valid signature and conflicting ordering refuse required certification. Final index is the authenticated packaging commitment. | DEMONSTRATED E4. The structural compactor alone is not mislabeled as supplied-key cryptographic verification. |
| Strict malformed/unresolved boundary and no evidence-dropping shortcut | Actual shared parser rejects malformed/unsupported before resolver calls; well-formed unresolved is Unverified; wrong type is Invalid. Public Writer/file checks unsigned integrity/profile controls and opaque payload preservation. Current actual pack/repack tests plus unchanged original probes retain malformed/non-UTC/incomplete/foreign source authorship, reject ambiguous/nonliteral roots and invalid producer time before entropy; valid UTC lexical neighbors retain bytes. | DEMONSTRATED E2–E4/E7. Original failed reports remain preserved; passing refusal alone does not substitute for positive pack/sign controls. |
| Shared cross-engine conformance when spec publishes vectors | Complete authoritative upstream snapshot at 2026-10-06T14:57:27Z has only the two EdDSA COSE fixtures. Existing governed bytes replay unchanged; no published composite fixture appears at commit 0d1c8299c9411ea4ead853e31721d42ea66f081e/tree bbac332e919339ab0b1a9b99dcba841f0b434464. | CONDITIONAL publication absent in captured evidence. Shared composite interoperability UNVERIFIED, not an implementation scope cut; merge-time refresh remains required. |
| One native portable implementation and required qualification | Shared SHAKE/Ed25519/SHA512/CBOR/XSD homes feed actual production callers. Current corrected complete GTS/RDF execution, real wasm consumers, strict docs/workspace clippy/generators/hygiene/release wasm are terminal successful. Older full make-check is attributed only to its captured source and unaffected closures. | DEMONSTRATED for branch-local qualified inputs E3/E4/E8; hosted/current-base qualification stays separate. |
| Final captured feedback and Stage 2 specialist/exit review | Initial complete review/inline/thread APIs are terminal empty on exact head; CodeRabbit supplied usage-cap text, no substantive review. Security/quality/structure judgments and reader performance review are still pending at assignment; fresh feedback and exit audit remain required. | UNVERIFIED Stage 2 workflow; finding W1. |
| Current-base combined tree, final Stage 3 and authorized ghprsq merge | Parent predicts a clean merge tree but no combined-tree execution is supplied. Hosted run on older PR base is queued/running. No merge/audit-ref/closure/cleanup receipt exists. | UNVERIFIED integration/workflow; W2/W3 below, not a branch functional failure. |

## Executed evidence, commands and identity applicability

All commands below are prior attributable executions from W, with normal optimized
profiles. Native target is x86_64-unknown-linux-gnu; wasm is wasm32-unknown-unknown;
rustc nightly full identity is `4b6d04e706108ccfeafe2547fbe857dfe8972bad`,
LLVM 23.1.1, Node 26.10.0 and wasm-bindgen 0.2.125. Test/dev retain opt3,
assertions and overflow checks. The actual wasm runner invokes Node, requires
an explicit harness status and refuses traps or silent returns; it is runtime
evidence, not build-only output. Fixture providers supply controlled test inputs
through the real provider contract, not an alternative production signer.

- E1: `cargo test --locked -p purrdf-gts --test cose_composite`, and the same
  command with `--target wasm32-unknown-unknown` plus
  `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=W/scripts/wasm-test-runner.sh`.
  Eleven actual groups pass, zero ignored/filtered, exit 0. Logs
  `raw/T3-R1-public-native-qualified.log` SHA256
  `27fa613ee20581587d40bfbb7a3b2a02bebcc480216cae3fac85a7f14b9c6c5a`
  and `raw/T3-R1-wasm-qualified.log`
  `221b70035aada19c168f9249281616e367150a3b895a80ed405b6a3b896b2082`.
  Captured T3 manifest SHA256 is
  `54d8e8204b39b971126d9398f0a3d216175edc1b9ee0ca309356af01ce67c0a6`.
  Subsequent sign1 delta only extracts the resolver loop/adds typed resolution;
  parser, dispatch, signature preimage and combiner bodies remain unchanged.
  Test delta shares an identical key helper and adds required packaging=false
  fields without weakening assertions. E2 qualifies the changed resolver path;
  E3 reruns all current native COSE groups. This is bounded reuse, not a claim
  that all T3 input files are byte-identical today.
- E2: `cargo test --locked -p purrdf-gts --test hedged_writer`, native and actual
  wasm with the runner/target above; thirteen actual groups pass, zero
  ignored/filtered, exit 0. Logs `raw/T4-R1-public-native-complete.log`
  `7ff2e74d7fb4d7a1c5f12941e163de6a6055b20dfe09d53b9b8780328eb98a3b`
  and `raw/T4-R1-wasm-complete.log`
  `5ee327050f84cedf49eeff4dabe3a6fe414513154e68136b3063e9043ba8f3c7`.
  Captured T4 manifest SHA256
  `9d4e72ee4bd4401648035678f33dbe5be90a7a7f93189cb78d028bd70bc95029`.
  Writer/provider/keyring/parser/resolver bodies are unchanged since that
  qualification; later reader/classifier/certifier closure is requalified by E4.
- E3: `CARGO_BUILD_JOBS=4 cargo test --locked -p purrdf-gts -p purrdf-rdf`,
  exit 0, current complete 1361 native cases and 13 doctests, no failed/ignored.
  `raw/T6-R1-affected-native-final.log` SHA256
  `04e45ba791708641486a0b2d4e2eb7b39b5c0cabc1e97093f303c7e543367aec`.
  It executes current COSE, primitive, Writer and real RDF consumer groups.
- E4: `CARGO_BUILD_JOBS=4 CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=W/scripts/wasm-test-runner.sh cargo test --locked --target wasm32-unknown-unknown -p purrdf-rdf --test gts_composite_compaction`,
  exit 0, all eleven groups actually pass, none ignored/filtered.
  `raw/T6-R1-public-wasm-final.log` SHA256
  `c9747faad41b7cfe569f35f799d76d566459802ce8fa6c37016dc3bd9a16fa6a`.
  E3/E4 use the final 52-path manifest above, followed only by exact-source
  signed transport. Independently recalculated retained executable hashes:
  current native COSE `80601cedcbf4edaed209743db401ea222b0e7e38409b2a1930a2056d3c6fe274`,
  Writer `ffd286e23f22977d212e0fde07b61411c7296d7534eac6434e48e8a74b99a937`,
  RDF consumer `a3e43d5cf6d8d03ea498b7bbd63a6e4e363f244cdd2ea8eacd721f2f32c4c109`,
  current RDF wasm `9ee7826b36054e53f333c827ded0c5899d496115342bfb3120bc72c36f70aa71`.
  All match `raw/T6-R1-runtime-artifacts.sha256`, manifest SHA256
  `2f5d60f28bbfa88ed315e15730341a863c6978ed2f73a3945d07f1bdd5260526`.
- E5: retained independent primary audits are not PurRDF-generated expected
  answers. The three NIST joins compare complete independently formatted
  primary/frozen records; fresh cmp again matches all three. IETF audit log
  `raw/T3-review-fixture-audit.log` SHA256
  `fbf675b5ef7188b256d1470ca36834697e5388b0474ea3b93a3f515f65223a6b`
  records all eight exact fields and twelve primary literal lengths, including
  complete 1984-byte key, 127-byte representative and 3373-byte signature.
  Actual public tests compare against those unchanged external fixture bytes.
- E6: primitive wasm command is the same target/runner command selecting
  `-p purrdf-gts --test mldsa65`, exit 0, six executed public groups/70 official
  cases. `raw/T2-wasm-runtime-qualified.log` SHA256
  `87d67927cc2eca83fcf06f6b51f405a2388091aacb468649deae5321f1db7a41`.
  ML-DSA modules, SHAKE and primitive test/fixture bytes remain unchanged under
  current manifest; NIST notice's whitespace refinement changes no answer.
  E3 additionally reruns current native official cases. T1 independent SHAKE
  log matches four complete 512-byte NIST answers and all sixteen OpenSSL
  answers; private arithmetic tests supplement but do not replace this evidence.
- E7: BOTH original unchanged native timestamp probes were independently
  replayed with `CARGO_BUILD_JOBS=2 cargo run --locked --offline --manifest-path S/raw/T6-review-timestamp-probe/Cargo.toml --bin <original-bin>`, exit 0.
  Consumer log SHA256
  `beac11c0f4365985cca60317bdfd23c2fe2c96c35d8c2322d56deadb7bf4bfdb`;
  controls log `de6a169d05a9e7135917ec34419aa31a8a44519b400b9f66cc94123402076fd0`.
  Actual rows show four malformed/non-UTC sources keep one exact author pair,
  four content quads and all six certifier fields true with real preserved
  history. Genuine Z/+00:00 packaging is reissued. Four invalid producer
  parameters refuse; six legal UTC XSD forms produce real packs. Process zero
  alone is not acceptance; these observed rows plus E4 assertions establish it.
- E8: current command statuses/actual task-qualified logs establish strict
  docs, workspace all-target clippy, normal release `make wasm`, generators and
  hygiene. Prior full `make check` log SHA256
  `7f7aea50e039f0d0643fd1ddb0170089290c8cf8f691385463e4e3c4d8562c99`
  belongs to captured gate manifest
  `77082e9b925a52e168b3523d872174f96c39de49840fb11127a1ed6e04651eda`;
  21045 native/460 docs passed, 37 default ignores separately recorded.
  The subsequent two-file functional correction invalidated provenance/
  compaction/certification and was covered by E3/E4/E7 plus current release
  compilation; unaffected closures may reuse the old gate. No old full pass
  becomes a full pass on HEAD or the predicted merge tree.

## Completion failure-shape and scan adjudication

DARK/TEST-ONLY/PRODUCER-WITHOUT-CONSUMER/VACUOUS: not established. External
integration targets import public shipping crates, produce nonempty actual
files via Writer, consume those bytes through readers/keyring verification and
real compaction/certifier APIs, and assert meaningful authentic output/history.
Fixed fixture seeds/randomizers do not bypass the production provider or combiner.
No installed CLI/binding composite-discovery demand is inferred: the literal
issue excludes discovery and requests purrdf-gts library support.

REFUSED/SILENT NARROWING: not established. Valid composite controls sign and
verify at every material boundary; refusal tests cannot alone qualify support.
Legacy Ed-specific convenience names use the sole generalized core and retain
their old key contracts; typed general APIs genuinely accept composite keys.
The IETF attached-vector refusal protects GTS's detached contract while its
exact preimage/signature successfully authenticates the detached positive.

DUPLICATE/LAUNDERED-TO-FOLLOWUP: not established. Keccak, native Ed25519,
SHA512, CBOR, XSD and the new ML-DSA/combiner each retain one governed home.
No scope is assigned to a new ticket or local deficiency ledger. Worktree
`.deficiencies` is empty below its marker. No user decision cuts required work.
Owned-storage clearing, timing/certification/entropy/key-reuse limits are openly
stated without pretending to prove hardware properties; they do not substitute
for missing requested functional signing. Provisional -58 and conditional shared
vectors remain disclosed; registered standalone -49 is not substituted.

WEAKENED TESTS: inspected relevant test/golden changes retain original assertions.
Arbitrary fake detached byte arrays become genuine public Sign1 fixtures because
the strict shared decoder now refuses malformed evidence. Ordering/absence/proof
assertions still run; added unwraps hard-fail real errors. No governed vector
path differs from captured base. No new allow/ignore/feature/optional-edge/SKIP/
xfail/unconditional passing assertion appears in inspected added witnesses.

Fresh exact deferral scan on base-to-HEAD added lines returns one hit, stored in
`raw/S2-completion-deferral-branch.log` SHA256
`1de3a053bba210dccdab057c287d8d416c83c8b91dba47ee1a6de228ee995894`:
the distinct unrequested HashML-DSA exclusion. Requested composite prehash is
implemented and externally compared. Fresh laundering scan
`raw/S2-completion-laundering.log` has exactly 39 witnesses, SHA256
`d855f32ac5f7930f8cd3910bd2b06aec2d74500befc8d6e70072077f190c8a5c`,
and reproduces guarded expects/test panics,
denied-fallback documentation and existing pre-fold optimization; each actual
witness was read along with all 39 recorded final T6 witnesses/dispositions.
Exact seed slicing follows validated length; Ed-only expect cannot accept a
composite key; sealed Writer Infallible cannot publicly install composite keys;
provider-panicking negative control enforces timestamp preflight beside positive
production packs. None discharges required work by a stub/error fallback.
Recorded immutable plan/commit/PR/confidence deferral scan has zero prose hits.

## Open workflow findings and required evidence

W1 — Stage 2 review/exit evidence, UNVERIFIED. Current independent functional
judgment does not supply pending specialist security/quality/structure or
reader-hot-path performance conclusions. Parent must read those concrete
reports, fix any actual required findings, publish dispositions and run the
fresh independent Stage 2 exit audit. No slowdown or security defect is inferred
from absent specialist evidence, and old T6 PASS does not preclear Stage 2.

W2 — Current-base integration evidence, UNVERIFIED. Base advanced beyond both
implementation and initial PR base. Parent must assess exact base delta and
candidate tree `134045a...`, including Cargo.lock/build/policy and affected
consumer closures, run named missing checks, and bind evidence to fresh inputs.
A clean merge-tree result proves syntactic combination only. No blanket rerun
is prescribed absent a concrete closure gap. This is required before final
Stage 3 readiness; this audit supplies no combined-tree pass.

W3 — Hosted/feedback/external/final integration states, UNVERIFIED. Captured
`raw/S2-initial-ci-run.json` SHA256
`6ee83b134bf38336f97a0c4d39073f325464f5d77384cbec019c6f29b4ea5a22`
identifies PR run 37487670367/head52988974/event pull_request, queued with no
conclusion; other captured jobs are queued/running. Full initial paginated
reviews/inline/threads are terminal empty, with exact head and older base.
CodeRabbit's cap comment supplies no review despite its green status. Parent
must refresh complete surfaces after processing, remediate real feedback,
qualify required CI by exact head/test-merge identity and refresh authoritative
shared vectors/allocation before merge. Final Stage 3/ghprsq audit refs,
signature/tree/notes, closure and owned-cleanup receipts remain authorized work,
not done by this audit. No release/deployment requirement is invented.

Functional acceptance on assigned HEAD is evidenced; whole workflow completion
remains open for W1–W3. No issue scope is silently reduced by this conclusion.
