<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Implementation origins

The initial source entered this repository in commit
`4741a9e371faa5a6646208f91c9c64951ded659c`, authored by Patrick Audley.
The tracked implementation carries the first-party license offer.
`src/select.rs` identifies SELECT-NEIGHBORS-HEURISTIC, Algorithm 4 of Malkov
and Yashunin, as its algorithmic reference. The level assignment, frozen-batch
construction and deterministic ordering are specified in the crate’s own
source. The distance kernels, rank comparator and integer mix are consumed
from the first-party workspace homes named there.

This is a record of the source history and declared references, not an
assertion of a clean-room development process. No external HNSW implementation
source or imported HNSW fixture is declared in this crate’s source or manifest.
The tests construct their data in this repository; they do not redistribute
model weights or external embedding corpora. Caller-supplied vectors and models
retain their own rights and are outside the package’s first-party offer.
