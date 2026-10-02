<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Implementation references

The new `src/ir/segmented/` representation and read-session implementation were
written in this repository over its existing RDF interner, term walker, term
emitter, fixed table hasher, little-endian byte readers and native BLAKE3 home.
Front-code comparisons call `purrdf_deflate::common_prefix_len`; hashing calls
`purrdf_hash::blake3`. Their existing provenance and notice obligations remain
at those implementation homes. The format and certification laws are specified
in the new source and `STORAGE.md`; this note does not assert a clean-room process.
No external storage implementation source, corpus, or model artifact was copied
or consulted for these additions during this change.

The resident IR and retained eager pack are pre-existing first-party source;
the repository-level `PROVENANCE.md` records their extraction history. Unicode
and other inherited source/fixture notices are distributed by the repository's
license inventory and this crate's `licenses/` directory. Adding the read-session
seam does not remove those notices or alter the rights of caller-supplied data.
