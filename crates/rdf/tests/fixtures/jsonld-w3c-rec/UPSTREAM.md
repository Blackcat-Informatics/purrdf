<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Pinned W3C JSON-LD 1.1 conformance vectors

This directory vendors 73 positive, offline, star-free `toRdf` cases and 13
exact compaction cases from the W3C JSON-LD 1.1 Recommendation test suite. Each
case retains the upstream input, oracle output, name, purpose, and byte-level
SHA-256 checksums in `vectors.json` or `compaction_vectors.json`.

- Upstream: <https://github.com/w3c/json-ld-api>
- Tag: `REC-2020-07-16`
- Revision: `3e7fa5377b2b3c5176eacf8bde8e01fdb7c4a062`
- Upstream paths: `tests/toRdf-manifest.jsonld`, `tests/toRdf/`,
  `tests/compact-manifest.jsonld`, and `tests/compact/`
- Imported: 2026-07-17
- Upstream terms: W3C Test Suite License, referenced in `LICENSE.md`

The selection includes every positive, local, default-option core `toRdf` case
in the upstream 0001-0036 and 0113-0132 ranges that does not require an implicit
document URL, generalized RDF, or the standalone RFC 3986 stress matrix. It also
covers full and compact IRIs, blank nodes, language and typed
literals, type coercion, lists, reverse properties, scoped contexts, language,
graph, ID, and type maps, JSON literals, transparent nesting, mapped indexes,
protected terms, and order independence. Cases needing network retrieval,
document-location injection, HTML processing, generalized RDF, or non-default
API options are not applicable to PurRDF's offline dataset codec. The exact
selection is executable and reviewable in `scripts/vendor-jsonld-rec.py`.

The harness compares PurRDF expansion with the pinned N-Quads using RDF dataset
isomorphism and compares compaction with the exact pinned W3C JSON after removing
only PurRDF's documented single-node carrier wrapper. Both expected result sets
are independent W3C oracles; no implementation output was used to generate them.
The harness has no xfail path and asserts exact 73-case expansion/to-RDF and
13-case compaction pass counts, revision, tag, unique IDs, and every checksum.
