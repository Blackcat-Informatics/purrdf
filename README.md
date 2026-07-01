# purrdf

PURRDF is the extracted RDF toolkit from the GMEOW stack. It owns the RDF 1.2
primitive model, native text/XML/JSON-LD adapters, GTS transport integration,
SHACL/shape validation, SPARQL support, slice/dataset carrier IR, and language
bindings.

This branch is the copy-and-rename staging branch. The source repositories stay
read-only until purrdf is published and the later cutover branch in
`gmeow-ontology` can replace the old in-tree crates.

## Layout

- `crates/rdf-core`: transport-independent RDF primitives, diagnostics, IR, and
  store traits.
- `crates/rdf`: top-level purrdf Rust API and first-class GTS/text-codec
  adapters.
- `crates/gts`: inlined GTS container engine, stripped of standalone native RDF
  text/native-store codecs.
- `crates/slice`: carrier IR for ontology slices, dataset-level wrappers,
  ownership/dependency analysis, and projection inputs.
- `crates/shapes`: SHACL and shape validation.
- `crates/sparql-*`: SPARQL parser, evaluator, result handling, and conformance.
- `crates/iri`, `crates/xsd`, `crates/rdf-events`: support crates.
- `python`: Python package sources copied as `purrdf`.
- `docs` and `vectors`: GTS specification assets and conformance vectors.

## Validation

```sh
make metadata
make check
```

See `PROVENANCE.md` for the source commits and extraction policy.
See `docs/CUTOVER.md` for the `gmeow-ontology` cutover checklist.
See `docs/RELEASE.md` for the crates.io trusted-publishing release process.
