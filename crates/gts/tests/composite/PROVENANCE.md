<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Pinned IETF composite known answer

`ietf-cose-example.txt` contains one complete published answer from Figure 10
of [JOSE/COSE composite signatures draft-04](https://www.ietf.org/archive/id/draft-ietf-jose-pq-composite-sigs-04.txt),
dated 10 September 2026. Its source text SHA-256 is
`6c4288d87a14eddf8c9eaa2453edb6ea96d7134fa8673c19910048bac3315de6`.
Construction is pinned to that revision and
[LAMPS composite signatures draft-19](https://www.ietf.org/archive/id/draft-ietf-lamps-pq-composite-sigs-19.txt),
source text SHA-256
`b4ee04416efc26e7d8de7c740dc3848ffe8b8cc9dcd035ad9d4e33e18844ec83`.
These Internet-Drafts are work in progress. Their requested COSE algorithm
`-58` remains a provisional draft identifier, not a final IANA assignment.

The record stores, in order, the complete ML-DSA seed (32 bytes), Ed25519 seed
(32), opaque kid (8), composite public key (1984), private seed encoding (64),
payload (29), representative (127) and signature (3373). Pagination is removed
from hexadecimal literals; no cryptographic byte is changed. A one-time Rust
import checked every literal length, repeated kid/payload consistency and seed
concatenation. No PurRDF output supplies an expected answer. Recorder's existing
count/body-SHA-256 format seals the exact one-record body; its SHA-256 is
`eae8f114bc35e75d9ddf26d5071bed87b437e05bfa8572f0197aaae3e46c4293`.
The IETF Trust Revised BSD terms and author attribution are retained in
`IETF-NOTICE.txt`.

Public tests reproduce the exact representative and derived public/private
encodings, verify the complete published signature with both primitives, and
compare deterministic signing byte-for-byte against all 3373 published bytes.
The example gives seeds and signature without an explicit randomizer field;
matching the explicitly deterministic API establishes that byte equality rather
than asserting a source-declared entropy policy. Separate caller-randomizer
tests exercise distinct valid hedged signatures.

The source example is attached and uses a binary protected kid. It is replayed
through the public composite/preimage APIs. An explicit detached-null neighbor
with its unchanged signature and preimage verifies through the strict typed
GTS parser and supplied-key API. The published attached envelope is refused by
the GTS detached parser. RFC 9052 defines kid as opaque bytes; legacy String
conveniences perform no implicit binary encoding conversion.

This independent cryptographic fixture lives outside governed root `vectors/`.
It is not a shared cross-engine GTS vector and establishes no shared-engine
composite interoperability. The frozen EdDSA Sign1 fixtures remain unchanged.
Original native implementation bodies use the workspace's one ML-DSA,
Ed25519, SHA-512 and CBOR homes. No external implementation body, arithmetic
table or coefficient array was imported.
