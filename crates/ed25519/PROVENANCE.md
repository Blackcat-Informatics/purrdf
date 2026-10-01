<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Implementation origins and verification data

The initial source entered this repository in commit
`27f9ab2cb5518b234332a37720686b8bd98a73b3`, authored by Patrick Audley.
The tracked implementation carries the first-party license offer. Its source
identifies RFC 8032 as the Ed25519 specification and documents the field and
scalar representations. This history record does not assert a clean-room
process or grant rights in another implementation.

SHA-512 is computed by the separately licensed `sha2` dependency; linked
artifact inventories preserve that dependency’s actual license texts.
`tests/vectors/` records decoding and verification answers from the former
`ed25519-dalek` engine. Its file headers specify generated inputs, record counts
and SHA-256 body digests. These are recorded outcomes, not copied implementation
source. The Wycheproof tests read the separately licensed frozen corpus from
`vectors/wycheproof`; its original Apache-2.0 notice remains authoritative.
