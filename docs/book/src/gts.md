<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# GTS Graph Transport

**GTS (Graph Transport Substrate)** is an ontology-independent binary
container and transport format for RDF 1.2 datasets and content-addressed
binary payloads. PurRDF hosts the reference Rust engine,
[`purrdf-gts`](https://docs.rs/purrdf-gts) (re-exported as `purrdf::gts`).

This chapter is a user-level tour. **The wire format itself is specified in
[`docs/GTS-SPEC.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/GTS-SPEC.md)**
— consult the spec for framing, fold semantics, registries, and conformance
classes; nothing here supersedes it.

## The container model, at a high level

A GTS file is a **CBOR Sequence of one or more append-only segments**. Each
segment is a deterministic CBOR header followed by deterministic CBOR frames
chained by **BLAKE3 content identifiers**. The logical dataset is obtained by
a **deterministic fold** over the segment sequence: quads, reifiers,
annotations, and binary blobs all become rows of the folded container graph.

Properties that fall out of this design:

- **Content-addressed and append-only** — history is never rewritten;
  suppression is itself an appended record. Multi-segment files compose by
  simple concatenation.
- **Partial readability, total reader** — the reader verifies the BLAKE3
  chain and folds what it can; undecodable frames (unknown codec, encrypted
  without a key) degrade to *opaque nodes* plus a diagnostic instead of
  aborting.
- **Binary payloads ride along** — the blobs a graph references travel in the
  same file, content-addressed like everything else.
- **RDF 1.2-native** — the spec formalizes the triple-term and `rdf:reifies`
  mapping, blank-node scoping, and multi-segment value union.

## Reading and writing from Rust

The container engine (`purrdf-gts`) owns the wire-format machinery — reader,
writer, fold, verify, COSE, trust policy:

```rust
use purrdf::gts::reader;
use purrdf::gts::{model::{Graph, Term}, writer::Writer};

let mut source = Graph::default();
source.terms.extend([
    Term::iri("https://example.org/alice"),
    Term::iri("https://example.org/knows"),
    Term::iri("https://example.org/bob"),
]);
source.quads.push((0, 1, 2, None));
let bytes = Writer::deterministic(&source, "example-profile")?.into_bytes();

// Fold GTS bytes into the container graph model, verifying the BLAKE3 chain.
let graph = reader::read(&bytes, /* allow_segments */ true, /* expected_head */ None);

// The fold is total: quads, reifiers, annotations, and blobs are all rows,
// and anything undecodable is preserved as an opaque node plus a diagnostic.
println!("{} quads, {} blobs", graph.quads.len(), graph.blobs.len());
assert_eq!(graph.quads.len(), 1);
assert!(graph.blobs.is_empty());
assert!(graph.diagnostics.is_empty());
assert_eq!(bytes, Writer::deterministic(&graph, "example-profile")?.into_bytes());
# Ok::<(), Box<dyn std::error::Error>>(())
```

The writer authors frames and produces **byte-deterministic single-segment
snapshots** (`Writer::deterministic`) — the GTS writer is under the same
determinism invariant as every PurRDF serializer.

The `RdfDataset` import/export path lives one layer up: the umbrella crate's
`gts` module combines the container engine with the RDF-level adapter
(snapshot composition, content-chain verification), so RDF-facing GTS work
goes through `purrdf` directly.

## Indexed dictionary access

`purrdf_gts::reader::indexed` accepts a caller-owned `Read + Seek` source and
caller-owned `TermStorage`. Construct an `IndexedInput`, bind it to the actual
`DecodeCapabilities` and `LookupPolicy` in an `IndexedCatalog`, then retain that
catalog and storage in an `IndexedDictionary`. Opening these owners does not
scan the source. `get(TermKey)` discovers the requested segment and admits its
selected dictionary block through the original term decoder. Its result contains
the original admitted term lease and any selected-block diagnostics.

The source callback must report authoritative current snapshot ownership,
generation and length. Capability identity retains the same allocation as the
actual key resolver; policy identity retains the actual immutable options and
resource ceilings. Supply their `owner()` values in the callback's
`AccessBinding`. Replacing source bytes, rotating keys or changing policy must
change the corresponding owner or generation. These identities are checked
before retained metadata and term-cache entries are used. Rewrapping an old
store does not certify its previous bindings; warm reuse retains the verified
dictionary owner itself. Warm results retain the original diagnostics.

Cold discovery walks CBOR structure to locate the segment's latest index and
legal tail. It checks every present offset, type or dictionary locator against
observed frame boundaries and contributor coverage. Plain predecessor term
counts and reference projections skip lexical bodies; transformed predecessors
need their bounded decoder to establish those counts. Indirect quoted terms
require a separate explicit `prepare_reference_access(segment)` call. Preparation
authenticates all current-wire dictionary/snapshot contributors, including tails,
one bounded encoded frame at a time. It can read every dictionary frame's bytes;
it admits no raw terms, performs no RDF row fold and verifies no global trust
policy. An unprepared indirect lookup returns `ReferencePreparationRequired`
without poisoning the session, so the caller may prepare and retry.

Prepared indirect lookup inspects numeric reference projections and binding rows
in source order, including later legacy ambiguity. Row-position and recursion
checks use that metadata without loading unrelated raw term blocks or treating
their unadmitted lexical defects as global diagnostics. Selected dictionary
blocks are admitted through the original term decoder and retain every block
diagnostic, including defects on other terms in that same block. RDF interning
then follows the requested dependency closure. These
operations can require more work than the selected block alone. Buffered
prefetch may read unselected bytes, particularly for short adjacent values.
`work()` reports actual source reads, bytes, seeks, identity checks, frame hashes
and logical payload/capability decoder calls rather than an inferred selected-byte
count. Encoded CBOR admission during preparation contributes its actual bytes and
hashes; it is separate from the logical payload-decoder call counter.

`purrdf_rdf::import_gts_indexed_term` resolves a selected term and its dependency
closure through the authoritative RDF importer callbacks. It returns a native
dataset handle and the term id owned by that handle. Each call uses fresh
destination resolution state, so ids from separate returned owners must not be
mixed. This operation does not import unrelated quad, reifier or annotation
rows and never initiates reference preparation implicitly. Full import and
N-Quads export remain explicit operations through the
existing forward reader and dictionary-backed importer/exporter APIs.

The frame byte ceilings and combined metadata ceiling cover the selected reader
windows, catalog, verification records and active resolution state. Raw term
allocations and escaped pins remain owned by caller storage; a lease reports its
retained bytes. The native RDF result is separate caller-owned output. Metadata
entry counts are not a process-memory bound.

Selective frame hashes and index coverage do not certify unvisited payloads or
global signatures. A requested `expected_head` produces
`FullVerificationRequired`; use the existing full-reader options for explicit
freshness verification. A footer without usable locators produces
`IndexUnavailable`, with no implicit full-reader fallback. Signature and trust
verification also remain explicit operations.

## Signing and encryption

Frames can be signed and encrypted with **COSE**; OpenPGP-based checks and a
trust-policy layer are also part of the engine. All cryptography is
**pure Rust** — no C toolchain, threads, or syscall dependencies — which is
what keeps the whole engine wasm-friendly. An encrypted frame you cannot
decrypt is simply an opaque node in the fold: the container remains readable.

## Rust-library-only: compaction certificates, proofs and keyring verification

Beyond reading, writing and folding, `purrdf-gts` and the umbrella's `gts`
module carry a verification stack that no other surface reaches — not the
CLI, Python, wasm or C:

- **Streamable compaction with a certificate** — `compact_streamable`
  rewrites an accretive log into one delivery-ordered segment (leading stream
  index, trailing offset footer, in-band dictionaries), and
  `compact_and_certify` / `verify_compaction` / `compose` bind a content
  projection so that the pre- and post-compaction containers fold to
  byte-identical canonical content; the verifier recomputes from the pre bytes
  with its own keyring. Compaction never changes content and mints no content
  signatures.
- **Merkle mountain range proofs** — `mmr::{root, prove, verify_proof,
  prove_file}` (`gts-mmr-proof-v1`), and `verify_content_chain`, which walks
  COSE signature → head replay → digest inclusion. There is no transparency
  log and no signed checkpoint service.
- **OpenPGP transport keys** — armored Ed25519 certificates and unencrypted
  secret keys (`parse_transport_key`, `parse_secret_signing_key`) with
  `verify_file_with_keyring` for rotation; other algorithms, encrypted keys
  and v5/v6 packets are rejected, and there is no keyring or revocation store.

Native unit tests cover file media-type mapping and tar/PAX serialization and
parsing. Frozen vectors and compaction/certification fixtures exercise additional
paths. These checks cover their stated cases; they do not establish complete
coverage of the files profile, nested GTS-in-blob reader, trust policies or ULID
behavior. The upstream `gmeow-gts` specification and frozen vectors remain the
format authority.

## Conformance vectors

GTS conformance is defined against a **frozen, language-neutral vector
corpus** (`vectors/` in the repository), shared **byte-exact** across the
sibling GTS engines in other languages. The vectors are never regenerated or
"fixed" in this repository — the format is governed in the
[`gmeow-gts`](https://github.com/Blackcat-Informatics/gmeow-gts) project,
alongside the specification and the other reference engines.

## The C ABI's star-layer round-trip

The GTS star-layer round-trip of a dataset containing quoted triples /
reifier bindings succeeds through the kernel
`to_gts` → `read_graph` → `import_gts_graph` path, the same as a star-free
round-trip; `crates/rdf-capi/tests/abi.rs`'s
`gts_star_roundtrip_preserves_the_statement_layer` pins it. See
[Getting Started: C](getting-started/c.md#gts-star-layer-round-trip).

## Related

- [`docs/GTS-SPEC.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/GTS-SPEC.md)
  — the normative wire format (version 0.9-draft, wire-format major 1).
- [Slices, Mappings & Provenance](slices.md) — the RDF↔GTS loss ledger.
- [Getting Started: Python](getting-started/python.md#gts-relational-rows)
  — reading a container back as relational rows.
