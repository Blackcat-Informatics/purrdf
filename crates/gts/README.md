# purrdf-gts

`purrdf-gts` is the PURRDF-owned GTS container engine. It owns the
Graph Transport Substrate wire format pieces: CBOR sequence parsing and writing,
frame transforms, append-only chain verification, folding, signatures,
encryption, blobs, stream state, and transport policy.

It intentionally does not expose the old standalone RDF text/native-store codec
surface from `gmeow-gts`. RDF text formats, RDF/XML, JSON-LD/YAML-LD, SHACL,
ShEx, SPARQL, and package-level projections belong in the higher `purrdf`
crates, implemented on top of purrdf primitives.

The narrow API boundary is:

- `purrdf_gts::reader`: read and fold GTS bytes into the container graph model.
- `purrdf_gts::writer`: author GTS frames and deterministic snapshots.
- `purrdf_gts::model`: folded transport graph rows and diagnostics.
- `purrdf_gts::verify`, `cose`, `openpgp`, `policy`: integrity and trust checks.
- `purrdf_gts::files`, `tar`: optional content/file transport helpers.

Use the top-level `purrdf` crate for RDF-facing GTS import/export.
