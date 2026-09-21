<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# purrdf-json

PurRDF's ordered JSON codec carries a JSON document as RDF 1.2 and reconstructs
its original bytes. Object member order, duplicate names, whitespace, escapes,
number spellings and empty containers survive a production RDF serialization
and parse cycle. Every value occurrence has a kind, RFC 6901 path, parent,
ordinal and byte span. Containers expose their immediate member/element count. Repeated paths retain distinct occurrence identities.

```rust
use purrdf_json::{Profile, SourceDocument, analyze, project, decode_document};

let profile = Profile::standard()?;
let source = SourceDocument {
    id: "https://example.org/document.json",
    bytes: br#"{ "b": 2, "a": 1, "a": 3 }"#,
};
let document = analyze(source, &profile)?;
let dataset = project(&document, &profile)?;
let restored = decode_document(&dataset, document.id(), &profile)?;
assert_eq!(restored.as_bytes(), source.bytes);
# Ok::<(), purrdf_json::JsonError>(())
```

The same surface is always available as `purrdf::json`. The only runtime
dependency is `purrdf-core`; the codec uses no files, clocks, randomness or
platform services and builds for `wasm32-unknown-unknown`.

`analyze` borrows the original source and builds a validated occurrence model.
`project` constructs the shared IR directly. `encode` combines those operations.
`decode_document` requires an explicit document IRI and expected profile. It
validates metadata and ownership, applies the kernel's byte-cover and digest
checks, then reparses the result to prove every structural assertion. A graph
with correct bytes but a fabricated path or kind is rejected.

Choose a profile explicitly. `Profile::standard()` offers the named vocabulary
`https://w3id.org/purrdf/json#`; `Profile::new` accepts a caller's vocabulary,
name, version and resource bounds. No operation supplies an implicit namespace.
The profile identity includes the language, bounds and complete vocabulary law.

This codec accepts UTF-8 RFC 8259 JSON without BOM, requires paired surrogate
escapes in member names, and preserves duplicate object members. Scalar values
retain unpaired surrogate escapes verbatim. Scalar text is
the **raw lexical span**: `"\u0061"` and `"a"` remain different lexical strings.
A SPARQL query over `text` therefore matches raw spellings; it does not
unescape JSON strings or treat number spellings as numeric RDF literals.
Object keys are decoded only when constructing RFC 6901 paths. Number values
never pass through a floating-point or bounded-integer conversion.

For compact Turtle output, pass `Some(document.id())` as the explicit base to
`purrdf_rdf::serialize_dataset_to_format`. Document identities retain the full
SHA-256 binding; the serializer writes the common base once and uses short
`#vN` and `#sN` fragment references. Absolute RDF formats retain complete IRIs.

The complete wire contract is in [SPEC.md](SPEC.md). Run `cargo test -p
purrdf-json` for the frozen corpus and adversarial tests. `cargo bench -p
purrdf-json --bench ordered_json` measures parsing, projection, end-to-end
encoding and verified decoding. `cargo bench -p purrdf-json --bench
ordered_json_alloc` reports allocations and RDF expansion in a separate process
so allocation instrumentation does not affect latency measurements.

The same corpus executes in Node through `make wasm-test`, with a pinned digest
of all production RDF output bytes. For a report over real files, run `cargo
bench -p purrdf-json --bench ordered_json_corpus -- /path/to/corpus`. This native
profiling harness traverses JSON files in sorted order, reports accepted and
refused inputs separately, reports absolute and base-relative Turtle sizes,
and fails on I/O errors or any round-trip mismatch.
With no path it reports forward-parser scaling on the generated fixture.

[MEASUREMENTS.md](MEASUREMENTS.md) records the measured corpus, scaling and
allocation results with the exact command lines and representation choices.

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)
