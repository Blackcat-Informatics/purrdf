<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Compiled glob matching

Compile one anchored Unicode glob and reuse it without matching allocations:

```rust
use purrdf_glob::{CompiledGlob, Mode};
let pattern = CompiledGlob::compile("**/a*b/c", Mode::Path)?;
assert!(pattern.match_literal("axxx/y/ab/c", 1_000)?.matched);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The umbrella exposes the same implementation as `purrdf::glob`. Compilation
accepts an owned, dedicated physical admission account through
`compile_with_storage`; syntax and storage refusals remain distinct.
[SPEC.md](SPEC.md) defines the exact versioned language, work accounting and
separator rules. It includes ordered pattern sets without implicit IRI decoding.

Compile an ordered set once and supply its result workspace when matching:

```rust
use purrdf_glob::{CompiledGlobSet, Mode};
let set = CompiledGlobSet::compile(&["**", "other", "**"], Mode::Path)?;
let mut indices = [0; 3];
let result = set.match_literal("a", 100, &mut indices)?;
assert_eq!(result.indices, [0, 2]);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The set returns every matching source index, preserving order and duplicates.
One aggregate work allowance covers the complete result. Matching allocates
nothing; the caller supplies one index slot per pattern. On refusal, no completed
report is returned and workspace contents are scratch.

The CLI requires an explicit mode, surface and work allowance:

```sh
purrdf glob '**/a*b/c' 'axxx/y/ab/c' --mode path --surface literal --max-work 1000
purrdf glob '/docs/*.ttl' 'https://example.org/docs/a.ttl?download' --mode path --surface iri-path --max-work 1000
purrdf glob-set 'a' --pattern '**' --pattern 'other' --pattern '**' --mode path --surface literal --max-work 1000
```

Completed decisions write `matched true` or `matched false`, then `work N`, one
line each, and exit successfully. Syntax, storage, IRI parsing and work exhaustion
fail without a decision. An absent component is a negative match; a present empty
component matches a pattern that accepts empty text. Quote patterns to prevent
shell expansion.

`glob-set` writes `matches N`, each matching `index I` on its own line, then
`work N`. It publishes output only after the complete set succeeds. With no
`--pattern`, it returns an empty result using zero work.

`rdf::PreparedPlan` reads an RDF-authored ordered source plan using an explicit
caller vocabulary. Execute against an original RDF term, then export the portable
description with RDF 1.2 quoted membership provenance. Duplicate patterns preserve
their distinct collection cells. The facade and `glob-rdf` CLI command use the
existing Turtle and N-Triples codecs. [SPEC.md](SPEC.md) defines the schema, required
CLI arguments, admission and refusal behavior. No namespace or identity is invented.

## License

Licensed under any one of the following, at your option:

- [MIT license](licenses/LICENSE-MIT)
- [Apache License, Version 2.0](licenses/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](licenses/LICENSE-MULAN)
