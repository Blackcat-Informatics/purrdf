# Task 6 focused timestamp contract review

VERDICT: BLOCKED

Required finding T6-R1: invalid/non-UTC rewrite timestamps cause positive
packaging classification, silent exclusion of original authored COSE/content,
and successful actual compaction certification. This is a focused boundary
verdict; no final Task 6 qualification/publication review is claimed.

## Exact source and evidence

Worktree: /home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Branch: paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Signed HEAD: b2560386263ff53578b3a06e40f5739f577829ac.
Captured base: ce3c07192aba1e36666062c00f958670a827cfb5.
Approved immutable plan SHA256:
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.

Current complete 52-path manifest raw/T6-final-branch-files.sha256:
1bf6eded811940dba80f8b34156e2102b4a16ebb32a3b2b8ede0f60edf5fa989.
All 52 current paths independently match it. Current complete branch patch
raw/T6-final-branch.diff:
1d9dacca903f3649ee8de5e823e4a51e4bf359dd7fe5b61af744aaec00ae6741.
The independent captured raw/T6-timestamp-reviewed-source.diff has that same
hash. Post-gate prose/registry delta raw/T6-post-gate.diff:
f348161d50fb839e7ee488f1a1b24e3d5a28611cd1d5fb1f169cda3a7d860093.

Affected execution bodies remain those of the passing earlier full-gate
snapshot, whose manifest hash is
77082e9b925a52e168b3523d872174f96c39de49840fb11127a1ed6e04651eda:

- crates/gts/src/compact.rs:
  15f54786b7b9ba53c75f3428051b3d2f2aa5488fa7a6ebb7163b414d6ef4ecfe.
- crates/gts/src/reader.rs:
  fcab6b2db274a43d5ece7f264cedd0ed3f97bd52cdb7adf04f22a43714f3f653.
- crates/rdf/src/gts_certify.rs:
  37a1a92a34eec08c0ba5559a777039ff516f13baf789419909ec82f0771f1d9b.

Probe identities under raw/:

| Artifact | SHA256 |
|---|---|
| T6-review-timestamp-probe.rs | 192a0a36e4aac107e4e70bef09b1958314a6ced22d59ea1e68fd2fd11cdb27c9 |
| T6-review-timestamp-controls.rs | 5b7c6c030d276a3e747bf1a19a9f4685debe875be08011bb9af9ec9339e72cd5 |
| T6-review-timestamp-probe/Cargo.toml, including both current bin declarations | 958c96344a32b94a80dc12d5b0fa4982ead4f958343b413dedaf4b370e42e96f |
| T6-review-timestamp-probe/Cargo.lock | aca87e910734ea62b5d7070ca47c5c2f6b5e993d0ca8ab11744ec0a2efe3fafa |
| T6-review-timestamp-probe-terminal.log | fd6abca96ae8f5c88b57a89bb8267e82cca52c58b121f8056ca6d47442bc2990 |
| T6-review-timestamp-controls.log | 9f6fd9db39d28e6bbfad230597f67d78dbc6461b423f7168ff7aec77bafd489a |

Shipping source, index, prior probe bodies and existing logs were preserved.
Only this review/probes under selected Stage were written. No network, model,
GPU, service lifecycle, expensive blanket gate or forge mutation occurred.

## Attributable actual producer/consumer computations

Both commands ran from the issue worktree, using CARGO_BUILD_JOBS=2, locked
offline Cargo and the manifest above. The original command ran when the
manifest had one bin; its current replay adds
`--bin task6-timestamp-review-probe`. Both executions returned terminal exit 0.
The producer-control command uses `--bin task6-timestamp-controls`.
The separate manifest resolves actual GTS/RDF/XSD/IRI worktree crates, with
opt-level 3, debug assertions and overflow checks. Rustc is nightly
4b6d04e706108ccfeafe2547fbe857dfe8972bad, LLVM 23.1.1. This standalone lock is
public reproducer evidence, not another workspace dependency gate.

The first probe authors a streamable Writer source with all other required
Compaction fields: blank-node subject, one IRI class, closed predicate set,
single string agent and valid blake3 source head. Only its final index is
authentically Ed-signed under an explicit author key. It calls actual eager
read, read_to_sink, verify_file_with_keyring, detached_signature_pairs,
content_projection, compact_and_certify and verify_compaction. Eager/evented
signature observations are asserted equal. The classifier is shared with
composite keys; this probe makes no separate composite-crypto execution claim.

Every case below has zero reader diagnostics, source crypto_ok=true,
packaging=true, zero detached author pairs, zero projected content quads,
original_preserved=false after real compaction, and all six certification
report fields true:

- Valid positive controls: `2026-01-01T00:00:00Z` and
  `2026-01-01T00:00:00+00:00`.
- Non-UTC offset: `2026-01-01T00:00:00+01:00`.
- Missing timezone: `2026-01-01T00:00:00`.
- Invalid lexical: `not-a-time` and `2026-02-30T00:00:00Z`.

The actual shared XSD API invoked independently by the probe is
purrdf_xsd::temporal::parse_datetime. It rejects the last two strings with
InvalidLexical and parses the nonzero/missing offset as Some(60)/None.
DateTime::timezone_minutes() is the existing public accessor for this field.

The second probe directly exercises compact_streamable with each timestamp
parameter on a normal genuinely signed source, a mandatory distinct packaging
key, and DictPlan::undicted. All ten calls currently return a pack, including
the same four invalid/non-UTC values above. streaming_index directly puts the
caller string into an xsd:dateTime literal without validation.

The control probe also establishes XSD-compatible UTC lexicals beyond a
Z-only/four-digit-year subset:

- `2026-01-01T00:00:00-00:00`: parses with zero timezone.
- `12026-01-01T00:00:00Z`: extended positive year, zero timezone.
- `-0001-01-01T00:00:00Z`: negative year, zero timezone.
- `2026-01-01T24:00:00Z`: legal XSD end-of-day lexical, zero timezone.

These producer controls establish valid XSD parsing and actual emitter
acceptance; the first probe's full reader/certifier route covers Z/+00:00.
No full-path claim for the other four controls is inferred from parsing alone.
Probe process exit 0 records the observations; it is not passing acceptance
of their currently defective result.

## Normative authority and precise expected disposition

GTS-SPEC 13.3 compaction-provenance table explicitly defines timestamp as
"Rewrite time as xsd:dateTime in UTC." GTS-SPEC 10.1 requires every source
frame signature carried as original frame-id/COSE evidence, while genuine
ordering commitments are reissued. Current approved Task 5's positive provenance
anchor and public ordinary/incomplete-shape tests distinguish authored index
signatures from actual packaging using that standard shape.

GTS-SPEC 13.3 claim coupling says stream vocabulary in an unclaimed layout is
a warning because provenance survives round trips. It neither removes the
timestamp shape nor licenses treating malformed facts as genuine packaging.
These input segments explicitly claim streamable layout. Cryptographic
authentication does not establish truth of asserted facts, but does not turn
an invalid lexical into a well-formed rewrite-time fact either.

This finding does not demand general RDF datatype validation, proof of an
actual rewrite, truthful agent/time claims, or source-head history discovery.
It requires a valid UTC timestamp as part of the conservative positive shape
before the classifier may exclude content and original COSE evidence. Source
head already receives analogous syntax validation rather than datatype/presence
only. Therefore this is a required boundary defect, not an invented blanket
RDF constraint or false-positive concern.

For malformed/non-UTC source shapes, either retain ordinary authored content
and exact original COSE through actual pack/repack/certification, or refuse
actionably; never silently certify after dropping them. Under the existing
conservative classifier design, retention is the coherent expected neighbor.
For genuine valid UTC shapes, packaging classification and reissued ordering
COSE remain correct; no requirement says to preserve their old packaging COSE
as author evidence. Explicit invalid/non-UTC compact_streamable timestamp
parameters must refuse before returning a pack rather than minting malformed
provenance.

Use the existing XSD parser/accessor, requiring successful dateTime parsing
and timezone_minutes()==Some(0). Preserve Z, +00:00 and -00:00 as XSD UTC
offsets, valid extended/negative years and legal 24:00:00. Do not substitute
an RFC3339-only parser or Z-suffix check; do not silently normalize/rewrite
caller bytes. Reuse one shared rule for classifier and producer validation.

## Closure needed

Register malformed/non-UTC and valid lexical neighbors in the existing public
native/actual wasm compaction/certification suite. Cover eager/evented agreement,
exact original COSE/content retention, actual composite packaging and Ed repack,
genuine packaging classification, plus direct emitter hard failures and valid
UTC construction. Reexecute these unchanged probes with new output paths and
independently assess their original observations on corrected source.

Qualify the actual code delta with affected runtime, lint, documentation and
hygiene checks and exact new identities. Unchanged primitive/workspace evidence
may qualify reuse under Stage validation policy, but the previous full gate
cannot be relabeled as a new full-gate execution on corrected source.
Task 6/PR preparation is blocked until T6-R1 is discharged independently.
