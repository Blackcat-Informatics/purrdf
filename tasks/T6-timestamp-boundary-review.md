# Focused timestamp boundary adjudication

Required finding T6-R1: OPEN. This is a focused source/contract finding, not the
Task 6 qualification or publication verdict.

Reviewed source is signed HEAD b2560386263ff53578b3a06e40f5739f577829ac plus
the current Task 6 prose/registry cleanup. Approved plan SHA256 remains
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.
The inspected execution bodies remain identical to the full-gate snapshot:
compact.rs 15f54786b7b9ba53c75f3428051b3d2f2aa5488fa7a6ebb7163b414d6ef4ecfe,
reader.rs fcab6b2db274a43d5ece7f264cedd0ed3f97bd52cdb7adf04f22a43714f3f653,
gts_certify.rs 37a1a92a34eec08c0ba5559a777039ff516f13baf789419909ec82f0771f1d9b.
Only probe/report artifacts under this selected Stage were written by reviewer;
shipping source and index were preserved. No network, model or broad checks ran.

## Actual computation

Command from the issue worktree:
`CARGO_BUILD_JOBS=2 cargo run --locked --offline --manifest-path .stage/purrdf-gts-composite-ml-dsa-65-ed25519/raw/T6-review-timestamp-probe/Cargo.toml`

Terminal exit 0; full output is raw/T6-review-timestamp-probe-terminal.log.
The probe is raw/T6-review-timestamp-probe.rs. Its separate Cargo manifest/lock
resolve the actual shipping GTS/RDF/XSD/IRI paths with opt-level 3, assertions
and overflow checks; this is a public reproducer, not workspace qualification.
Rustc is nightly 4b6d04e706108ccfeafe2547fbe857dfe8972bad, LLVM 23.1.1.

Each case builds a streamable source through public Writer with a blank-node
Compaction, exactly one class, closed predicates, one string agent, one typed
timestamp and a well-formed sourceHead. Its sole index has authentic Ed25519
COSE under an explicit author key. The same shared classifier serves composite
keys; this probe does not claim a new composite cryptographic execution.
Both materialized read and read_to_sink report the same signature observation.
Actual compact_and_certify then reauthors it with another mandatory signer;
verify_compaction checks the resulting bytes with both keys.

All six timestamp values produce zero reader diagnostics, source crypto_ok=true,
packaging=true, zero detached author pairs, zero projected content quads,
original_preserved=false and all six certificate report fields true:

- `2026-01-01T00:00:00Z`: positive UTC neighbor.
- `2026-01-01T00:00:00+00:00`: positive UTC neighbor, equivalent XSD zero offset.
- `2026-01-01T00:00:00+01:00`: XSD parses, timezone is Some(60).
- `2026-01-01T00:00:00`: XSD parses, timezone is None.
- `not-a-time`: XSD home returns InvalidLexical (missing T).
- `2026-02-30T00:00:00Z`: XSD home returns InvalidLexical (day out of range).

This log deliberately records computation without asserting the desired result
inside the probe: successful process execution is not a passing acceptance test.

## Required contract and disposition

docs/GTS-SPEC.md 13.3's compaction-provenance table defines timestamp as
"Rewrite time as xsd:dateTime in UTC." Section 10.1 requires carrying every
source frame signature as original frame-id/COSE evidence, while genuine
ordering commitments are reissued. The accepted Task 5 contract and current
tests already require conservative positive Compaction classification so that
ordinary/incomplete shapes cannot discard an authored index's original COSE.

Claim coupling at 13.3 says stream vocabulary in an unclaimed segment is a
warning because provenance survives round trips; it does not remove timestamp
shape constraints or authorize classifying malformed shapes as genuine rewrite
provenance. This probe's segments explicitly claim streamable layout. Signature
authentication proves who signed the bytes, not truth of claims, but that
distinction cannot supply a missing well-formed rewrite-time fact. This finding
does not demand validating arbitrary RDF literals or checking whether an agent
actually performed a rewrite, whether sourceHead refers to a real event, or
whether the asserted timestamp is factually true. It requires the explicit
standard provenance shape before using that shape to exclude original signature
evidence and content.

Therefore the invalid-lexical, absent-timezone and nonzero-offset neighbors
establish a required classifier defect, not a false positive. The valid UTC
neighbors are genuine shape matches and need not preserve their packaging COSE
as author evidence. The invalid neighbors must either remain ordinary authored
content with exact original COSE preserved or receive an actionable refusal;
they must not silently certify after exclusion.

## Concrete remediation and bounded validation

Use the existing purrdf_xsd::temporal::parse_datetime home to validate the
timestamp, requiring its timezone to represent UTC; do not duplicate a calendar
parser, require a narrower arbitrary RFC3339/Z-only subset, or treat nonzero
offset lexical strings as already UTC. Preserve both zero-offset spellings.
Apply the same rule to explicit compaction emitter input so the production
compactor cannot emit a shape its corrected reader then treats as ordinary
content. Invalid caller timestamp input should hard-fail before returning a pack.

Register these neighbors in the existing public compaction/certification suite,
including eager/evented agreement, actual pack/repack, exact authorship COSE,
and valid UTC plus valid XSD lexical boundaries. Reexecute the unchanged probe
and affected native/actual wasm caller, lint and applicable hygiene checks.
The fix will invalidate the old full gate for this changed behavior; qualify
reuse of unaffected primitive/workspace inputs explicitly rather than calling
the old run a final-source full reexecution.
