<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# GTS Third-Party Implementer Guide

This guide is for implementers who want to build an independent GTS reader and
make a testable Baseline Reader conformance claim. It is not normative. The
wire format remains defined by [`GTS-SPEC.md`](./GTS-SPEC.md), and conformance
claims remain defined by [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md).

Use this guide as a build order and checklist:

1. Implement the smallest reader that can parse, verify, and fold the baseline
   corpus.
2. Port the vector-manifest harness.
3. Publish a conformance claim that names the exact corpus revision and command.

## Normative Anchors

Do not copy these rules into an implementation guide or README as independent
requirements. Link to the owning section and implement against it.

| Topic | Normative owner |
|---|---|
| File structure, segments, and cat-append composition | [`GTS-SPEC.md` sections 3 and 3.1](./GTS-SPEC.md#3-file-structure) |
| CBOR conventions and deterministic encoding | [`GTS-SPEC.md` section 4](./GTS-SPEC.md#4-cbor-conventions) |
| Header map | [`GTS-SPEC.md` section 5](./GTS-SPEC.md#5-header) |
| Frame map and payload resolution | [`GTS-SPEC.md` sections 6 and 6.1](./GTS-SPEC.md#6-frames) |
| Graph model and fold algorithm | [`GTS-SPEC.md` section 7](./GTS-SPEC.md#7-graph-data-model-and-fold) |
| Opaque nodes | [`GTS-SPEC.md` section 7.6](./GTS-SPEC.md#76-opaque-nodes) |
| Mandatory codecs | [`GTS-SPEC.md` section 8.4](./GTS-SPEC.md#84-mandatory-core-set-and-durability) |
| Frame chain verification | [`GTS-SPEC.md` section 9.1](./GTS-SPEC.md#91-per-frame-self-hash-and-content-id-chain-mandatory) |
| Complete CDDL | [`GTS-SPEC.md` section 21](./GTS-SPEC.md#21-complete-cddl-appendix) |
| Hash, signature, and extension-key preimages | [`GTS-SPEC.md` section 22](./GTS-SPEC.md#22-hash-signature-and-extension-key-preimages) |
| Baseline Reader tier | [`GTS-CONFORMANCE.md` section 3](./GTS-CONFORMANCE.md#3-tiers) |
| Vector manifest schema | [`GTS-CONFORMANCE.md` section 5](./GTS-CONFORMANCE.md#5-vector-manifest-schema) |
| Diagnostics registry | [`GTS-CONFORMANCE.md` section 6](./GTS-CONFORMANCE.md#6-diagnostics-registry) |
| Read and verify modes | [`GTS-CONFORMANCE.md` section 7](./GTS-CONFORMANCE.md#7-read-and-verify-modes) |
| Third-party profile registration | [`GTS-SPEC.md` section 13](./GTS-SPEC.md#13-profiles) |

## Minimum Baseline Reader Work

A Baseline Reader is the smallest useful independent implementation. It reads
GTS bytes in permissive-read mode, verifies the content-id chain far enough to
surface diagnostics, folds recoverable graph content, and preserves unknown or
unsupported content as opaque nodes.

Minimum work:

- Parse a CBOR Sequence, not a single whole-file CBOR object.
- Detect segment headers and frames.
- Accept the optional CBOR self-describe tag `55799` when it tags a segment
  header.
- Decode the Header and Frame shapes from the CDDL appendix.
- Recompute Header and Frame ids using the preimage table.
- Check each frame's `prev` link against the previous item id in the same
  segment.
- Implement the mandatory transform stack: `identity`, `gzip`, and `zstd`.
- Fold `terms`, `quads`, reifiers, annotations, suppressions, blobs, metadata,
  diagnostics, segment ledgers, signatures, and opaque nodes according to the
  fold algorithm.
- Return diagnostics rather than panicking on malformed corpus inputs.
- Preserve undecodable, unsupported, encrypted-without-key, or damaged content
  as opaque nodes when recovery is possible.
- Compare your folded output to the expected JSON fields named by the vector
  manifest.

A Baseline Reader does not need to implement:

- COSE signature verification or encryption support.
- OpenPGP key extraction.
- Nested-GTS recursion.
- MMR/index proof validation.
- Stream events.
- Writer determinism.
- Strict publish tooling.
- Profile-aware policy validation.
- Database, Parquet, object-store, or range-fetch helpers.

Those capabilities can be added later and claimed under the appropriate
Streaming Reader, Full Reader, Writer, Validating Tool, or Profile-Aware Tool
tier.

## Suggested Reader Pipeline

The exact API is implementation-specific, but this pipeline matches the
conformance documents:

```text
bytes
  -> CBOR Sequence item iterator
  -> segment boundary detector
  -> Header validator
  -> Frame validator
  -> transform resolver
  -> frame-payload decoder
  -> fold accumulator
  -> Graph plus diagnostics, segment heads, opaque nodes, and metadata
```

Pseudocode:

```text
read_gts(bytes):
  items = parse_cbor_sequence(bytes)
  result = empty_graph()
  current_segment = none
  previous_id = none

  for item in items:
    if is_segment_header(item):
      current_segment = validate_header(item)
      previous_id = current_segment.id
      result.segments.append(current_segment.summary)
      continue

    frame = validate_frame_envelope(item, previous_id)
    previous_id = frame.id

    if frame.envelope_is_damaged:
      result.add_diagnostic("DamagedFrame")
      result.add_opaque(frame, reason="damaged")
      continue

    payload = resolve_transforms(frame)
    if payload.is_unsupported:
      result.add_diagnostic(payload.diagnostic)
      result.add_opaque(frame, reason=payload.opaque_reason)
      continue

    fold_payload(result, frame.type, payload)

  return result
```

The important properties are totality and observability: malformed or
unsupported corpus inputs must return a result with diagnostics rather than
aborting the process.

## Using `vectors/manifest.core.json`

The core manifest is the portable Baseline Reader conformance index. It names
the input file, expected graph JSON, required capabilities, subsets, tiers,
diagnostics, and notes for each vector. The aggregate `vectors/manifest.json`
also includes optional profile, transform, crypto, proof, and human-hash
fixtures that are useful for full repository checks but are not the Baseline
Reader starting point.

Start with vectors in the core manifest whose `tiers` contain
`baseline-reader`:

```bash
python - <<'PY'
import json
from pathlib import Path

manifest = json.loads(Path("vectors/manifest.core.json").read_text())
for vector in manifest["vectors"]:
    if "baseline-reader" in vector["tiers"]:
        expected = vector["expected"].get("graph")
        print(vector["id"], vector["input"]["path"], expected)
PY
```

For each selected vector:

1. Read `input.path` as bytes.
2. Run the reader in permissive-read mode.
3. Load `expected.graph` when it is not `null`.
4. Compare the expected fields that the manifest names: counts,
   diagnostics, segment heads, opaque reasons, blob summaries, streamable state,
   profiles, and N-Quads.
5. Treat `negative: true` as "expect diagnostics/refusal behavior", not "the
   process should fail or panic".
6. Record skipped vectors only when `required_capabilities` names a capability
   outside the tier being claimed.

Validate the manifest itself before using it as a release or report artifact:

```bash
python scripts/check_vector_manifest.py
python scripts/check_vector_manifest.py --self-test
```

Release reports should not cite the checked-in placeholder
`git:repository-commit-containing-manifest` as the corpus. Stamp an exact
revision for reports:

```bash
python scripts/check_vector_manifest.py \
  --release-manifest dist/vector-manifest.release.json
```

## Expected JSON Comparison

The current top-level corpus compares folded graph summaries rather than a
private internal object model. An implementation can use its own data
structures as long as it can emit equivalent fields.

Compare at least:

- `diagnostics`: ordered diagnostic code list.
- `terms`, `quads`, `segments`, and `suppressions`: folded count summaries.
- `segment_heads`: segment head ids in file order.
- `profiles`: folded profile declarations.
- `streamable`: per-segment layout state.
- `opaque_reasons`: sorted opacity reasons.
- `blobs`: inline blob digest, media type, and decoded size summaries.
- `nquads`: sorted RDF projection lines.

Blank-node labels are expected to match the reference renderer unless the
manifest narrows a vector to isomorphism-only comparison. See the expected graph
format in [`GTS-CONFORMANCE.md` section 4](./GTS-CONFORMANCE.md#4-expected-graph-format).

## Diagnostics And Opaque Nodes

Diagnostics are part of the public behavior of a conformance claim. Do not
rename the codes owned by the tier you claim.

Baseline Reader diagnostics include malformed or hostile input behavior such as
`EmptyFile`, `DamagedFrame`, `BrokenChain`, `TornAppendError`, `UnknownCodec`,
`ConflictingReifier`, `PositionConstraint`, `ForwardReference`,
and `SegmentBoundary`.

Opaque-node behavior is what keeps the reader total:

- Unknown codec: preserve the frame as an opaque node with
  `reason:"unknown-codec"`.
- Missing decrypt key: preserve the frame as an opaque node with
  `reason:"missing-key"` when encryption support is present but the key is not.
- Damaged recoverable frame: isolate the damaged content as opaque when item
  boundaries are known.
- Unknown structural frame type: preserve chain verification and either ignore
  the payload or surface it as opaque until a supported profile handles it.
  `UnknownFrameType` is a Profile-Aware Tool diagnostic in the conformance
  registry, not part of the Baseline Reader claim string.

An opaque node is not data loss. It is a machine-readable statement that the
reader carried content it could not safely decode or interpret.

## Profile Registration Basics

Profiles sit above the core wire format. A domain profile can define vocabulary,
validation rules, trust policy, publication workflow, and profile-specific
vectors, but it must not change:

- Header or frame grammar.
- Segment-boundary detection.
- Content-id, signature, or hash preimages.
- Transform-catalog resolution.
- Deterministic fold semantics.

A Baseline Reader should expose profile declarations and requirements as folded
metadata, diagnostics, or opaque reasons. It does not need to enforce profile
policy to claim Baseline Reader conformance.

Third-party profiles should publish the profile registration fields listed in
[`GTS-SPEC.md` section 13](./GTS-SPEC.md#13-profiles), including a stable token
or URI, owner/change controller, purpose, required vocabularies, validation
rules, failure taxonomy, security/privacy considerations, versioning policy, and
conformance vectors.

## Example Baseline Reader Claim

```text
Implementation: ExampleGTS Reader 0.1.0
Conformance tier: GTS Baseline Reader
Corpus revision: git:0123456789abcdef0123456789abcdef01234567
Read mode: permissive-read
Vector subsets passed: wire-core, total-reader, graph-fold
Capabilities enabled: cbor, blake3, identity, gzip, zstd
Command: example-gts-conformance --manifest vectors/manifest.core.json --tier baseline-reader
Skipped vectors: none for the claimed tier
Optional capabilities not claimed: signatures, encryption, nested GTS, MMR proofs, profile policy
```

If you claim only a subset of Baseline Reader behavior, do not call it Baseline
Reader conformance. Use an implementation-specific phrase such as "experimental
reader" until the required subsets pass.

## Common Pitfalls

- Treating the file as one enclosing CBOR object instead of a CBOR Sequence.
- Hashing the Header with the `id` key included.
- Hashing a Frame with the `id` or `sig` keys included.
- Dropping unknown codecs instead of preserving opaque nodes.
- Failing the process on negative vectors instead of returning diagnostics.
- Ignoring unknown extension keys when recomputing preimages.
- Treating profile policy failures as core wire-format invalidity.
- Claiming Full Reader behavior because signatures are parsed, even when
  signature verification, key resolution, or trust-policy behavior is missing.
- Comparing only N-Quads while ignoring diagnostics, opaque reasons, segment
  heads, streamable state, profiles, and blob summaries.
