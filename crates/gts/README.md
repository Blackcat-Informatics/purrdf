<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-gts` — GTS Graph-Transport Container Engine

[![crates.io](https://img.shields.io/crates/v/purrdf-gts.svg)](https://crates.io/crates/purrdf-gts)
[![docs.rs](https://docs.rs/purrdf-gts/badge.svg)](https://docs.rs/purrdf-gts)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-gts` is the GTS (Graph Transport Substrate) container engine of the
PurRDF toolkit: a single-file, content-addressed, append-only format for
shipping RDF 1.2 graphs — and the binary blobs they reference — between
systems. A GTS file is a CBOR Sequence of segments, each an append-only log of
frames chained by BLAKE3 content id; the reader verifies the chain and folds
the log into a container graph, degrading undecodable frames to opaque nodes
instead of aborting — **the reader is total**.

The crate owns the wire-format machinery:

- **`reader`** — chain-verified reading and deterministic folding of GTS bytes
  into the container graph model (quads, reifiers, annotations, blobs).
- **`writer`** — frame authoring and byte-deterministic single-segment
  snapshots (`Writer::deterministic`), with optional per-frame COSE signing.
- **`model`** — the folded transport-graph rows and fold diagnostics.
- **`verify`, `cose`, `openpgp`, `policy`** — integrity, signature, and
  trust-policy checks; encryption and signing use pure-Rust crypto, so the
  whole engine stays wasm-friendly.
- **`files`, `tar`, `stream`** — content/file transport helpers and streaming
  state.
- **`mldsa65`** — native pure-message ML-DSA-65 (FIPS 204), with seed key
  expansion, validated expanded-key import, deterministic signing, explicit
  caller-supplied hedged randomness and strict signature verification. The
  primitive's independent NIST fixtures are separate from the GTS wire corpus.

Both this engine and its sibling implementations are gated against the same
frozen, language-neutral conformance vectors, byte-exact. The format is
specified in
[`docs/GTS-SPEC.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/GTS-SPEC.md).

`cose::composite` supports the ML-DSA-65 + Ed25519 pairing pinned to
`draft-ietf-lamps-pq-composite-sigs-19` and
`draft-ietf-jose-pq-composite-sigs-04`. It uses the draft's requested COSE
algorithm `-58`, a provisional identifier whose final registration can change.
Both signature components are mandatory under one detached Sign1. The existing
Ed25519 contract retains its exact frozen bytes. Composite primary-source
known answers are independent IETF fixtures under `tests/composite/`; shared
cross-engine GTS composite vectors are not published in the captured corpus,
so no composite shared-engine interoperability claim follows from those tests.

Public COSE callers use `sign_id_hedged` with `SigningKeyRef::Composite`, a
dedicated composite key and fresh caller-provided 32-byte cryptographic
randomness for every signature. `sign_id_deterministic` is explicitly the
fixture variant. Component seeds must be fresh, independent and dedicated to
the composite; imports cannot detect reuse outside this crate. Private seed
encoding is ML-DSA seed followed by Ed25519 seed (64 bytes), public encoding
is 1984 bytes and signature encoding 3373 bytes. These portable APIs obtain
no ambient entropy and never silently replace failed signing with an output.

`parse_sign1` is the one strict signing-envelope parser. It preserves exact
received protected bytes for authentication, requires a supported protected alg
and detached null payload, and rejects wrong/nested tags, trailing items,
duplicate or ambiguous headers and unsupported critical instructions. Only alg and kid are processed
as critical headers in this GTS signing contract; counter-signature headers
are unsupported. Unknown noncritical metadata remains allowed. Typed keys and
`verify_sig_with_key` dispatch the algorithm and require both composite halves.
Kids are optional opaque byte strings per RFC 9052; typed APIs preserve absence
distinctly from an explicit empty identifier, without an implicit encoding
fallback. Supplied-key verification requires no kid. Existing String
conveniences remain text-only and absent IDs never enter empty-ID lookup.
Malformed/unsupported envelopes are invalid before lookup; supported envelopes
without a resolved key remain unverified.

### A note on `vectors/manifest*.json`'s `generated_by` field

The four vector manifests at the corpus root (`vectors/manifest.json`,
`manifest.core.json`, `manifest.profiles.json`, `manifest.transforms.json`)
each carry `"generated_by": "scripts/check_vector_manifest.py --write"`. No
such script exists in this repository, and `git log` over the manifests shows
no commit ever added one — the field names a generator that was never built
here. It is not corrected in place: `vectors/` is governed upstream by
[`gmeow-gts`](https://github.com/Blackcat-Informatics/gmeow-gts) and carried
into this repository verbatim, so it is hand-edited by nobody, in either
direction — not regenerated to "fix" the field, and not resynced from
upstream to match it. `scripts/check-corpus-frozen.py` does not byte-freeze
this root either (its `GUARDED_ROOTS` covers `vectors/shacl` and
`vectors/shexTest`, not the corpus root, by the same upstream-governance
reasoning its own comment states for `vectors/*.gts`). The manifests are
therefore maintained by hand upstream and carried here as-is; the only
guarantee this repository makes over them is the review-time rule that
nothing under `vectors/` is hand-edited locally (AGENTS.md/CLAUDE.md), not a
machine-verified freeze gate.

## Usage

```sh
cargo add purrdf-gts
```

```rust
use purrdf_gts::reader;

// Fold GTS bytes into the container graph model, verifying the BLAKE3 chain.
let graph = reader::read(&bytes, /* allow_segments */ true, /* expected_head */ None);

// The fold is total: quads, reifiers, annotations, and blobs are all rows,
// and anything undecodable is preserved as an opaque node plus a diagnostic.
println!("{} quads, {} blobs", graph.quads.len(), graph.blobs.len());
```

RDF text formats and the `RdfDataset` import/export path deliberately live one
layer up: use the umbrella [`purrdf`](https://crates.io/crates/purrdf) crate
(its `gts` module combines this engine with the RDF-level adapter) for
RDF-facing GTS work.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C (the GTS container itself reaches Python and C, not the wasm package). Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate; depend on `purrdf-gts`
directly only when you want the container engine alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.98, stable toolchain only).

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)
