<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# purrdf-toml

TOML 1.1 authoring as a queryable RDF 1.2 document with exact source
reconstruction. Like `purrdf-json` and `purrdf-markdown`, the codec first analyzes
an untouched source into a typed stand-off model, then projects that model into
the shared native RDF dataset. Scalar verbatim spans and maximal structure runs
cover every byte, including comments, whitespace, CRLF, quotes and numeric
spellings. Decoded semantic values remain separate from those original bytes.

```rust
use purrdf_toml::{Profile, SourceDocument, analyze, project, decode_document};

let profile = Profile::standard()?;
let source = SourceDocument {
    id: "https://example.org/ledger.toml",
    bytes: b"# authored ledger\r\n[[job]]\r\nid = 'native'\r\nenabled = true\r\n",
};
let model = analyze(source, &profile)?;
let graph = project(&model, &profile)?;
let restored = decode_document(&graph, model.id(), &profile)?;
assert_eq!(restored.as_bytes(), source.bytes);
# Ok::<(), purrdf_toml::TomlError>(())
```

The same API is available at `purrdf::toml`. `encode` combines analysis and
projection. Every operation uses an explicit profile. `Profile::standard()`
deliberately selects the shipped `https://w3id.org/purrdf/toml#` vocabulary;
`Profile::new` accepts a caller's name, version, vocabulary and resource bounds.
Changing any profile field changes its identity and the derived document IRI.

`Document::nodes()` is a flat arena with typed values, parent and ordinal,
source spans, decoded keys and escaped paths. Arrays of tables retain their
individual element identity; implicit, explicit, dotted and inline tables are
distinguished. `statements()` keeps authored declarations and assignments in
source order, `keys()` keeps every quoted/bare key occurrence, and `structure()`
iterates maximal gaps between scalar verbatim spans. Strings expose decoded
Unicode; integers, floating values, signed infinity/NaN and four date/time kinds
remain typed. Multiline string semantics normalize CRLF to LF; their source
spans and the decoded document retain the original line endings.

For example, query jobs and their identifiers from a projected ledger:

```sparql
PREFIX t: <https://w3id.org/purrdf/toml#>
SELECT ?id WHERE {
  ?job t:kind "table-element" .
  ?field t:parent ?job; t:key "id"; t:value ?id .
}
```

`decode_document` requires both the document IRI and expected profile. It reads
the selected source cover through `purrdf_core::cover::reconstruct`, reparses
those bytes and verifies the entire projected metadata graph. Correct bytes
with fabricated semantic values, paths, parentage, keys or declaration facts
are refused. Other selected-source revisions may coexist in the same dataset.
RDF 1.2 statement roles preserve set semantics without hiding conflicting facts.

Source, node, depth and retained model bounds are explicit; grammar and resource
refusals carry byte, line and column locations. Containers are parsed with heap
frames and stored in a flat arena, so nested input and model destruction consume
no native call-stack frame per level. The library uses no files, processes,
clocks or randomness and is built for WebAssembly alongside the release crates.
Application schemas, required keys and unknown-field policy belong to callers.

[SPEC.md](SPEC.md) defines the wire and identity contract.
[PROVENANCE.md](PROVENANCE.md) identifies the normative grammar and frozen
native conformance corpus. Run `cargo test -p purrdf-toml` for grammar, typed
values, native RDF round trips, real SPARQL queries and tampering acceptance.

## License

Licensed under any one of the following, at your option:

- [MIT license](licenses/LICENSE-MIT)
- [Apache License, Version 2.0](licenses/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](licenses/LICENSE-MULAN)
