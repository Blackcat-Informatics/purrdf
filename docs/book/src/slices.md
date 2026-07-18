<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Slices, Mappings & Provenance

Beyond the engines, PurRDF carries the plumbing a serious vocabulary or
data-pipeline project needs: a slice catalog for organizing authored RDF, an
explicit loss ledger for lossy projections, and native codecs for the SSSOM
and FnO interchange formats.

## The slice catalog

[`purrdf-slice`](https://docs.rs/purrdf-slice) (re-exported as
`purrdf::slice`) is tooling for ontology/vocabulary repositories organized as
*slices* — directories of authored RDF (`slices/<group>/<name>/`), each
described by a `manifest.ttl`:

- **Catalog** — manifest-based discovery (`SliceCatalog::discover`), typed
  slice metadata (`SliceRecord`, `SliceTier`), and artifact roles. Slice
  identity comes from the manifest, not the directory name.
- **Ownership & dependencies** — term-ownership analysis (every declared term
  has exactly one owning slice), dependency edges with evidence,
  forbidden-edge rules (extension slices depend only on core), and
  machine-applicable fix suggestions.
- **Content addressing** — deterministic artifact digests and cache keys for
  incremental pipelines.
- **Emitters** — projection/mapping emitters and lints: prefix maps, JSON-LD
  contexts, FnO function catalogs, claim views.

True to the rule that PurRDF mints no vocabulary IRIs, every term the slice
framework reads or emits belongs to the **caller's** vocabulary: a
`SliceVocab` is caller-constructed (it has no `Default`) and threaded through
every public entry point.

```rust,ignore
use std::path::Path;
use purrdf::slice::{SliceCatalog, SliceVocab};

// Your vocabulary namespace — PurRDF fabricates none.
let vocab = SliceVocab::for_namespace("https://example.org/vocab/");
assert_eq!(vocab.slice_class(), "https://example.org/vocab/Slice");

// Discover every slice under the repository root from its manifest.ttl.
let catalog = SliceCatalog::discover(Path::new("slices"), vocab)
    .expect("slices discovered");
for slice in catalog.records() {
    println!("{} ({:?})", slice.manifest.slice_iri, slice.manifest.tier);
}
```

## The loss ledger

PurRDF's projections are allowed to be lossy — but never silently. The kernel
carries a machine-readable **RDF↔GTS loss matrix**
([`generated/rdf-loss-matrix.json`](https://github.com/Blackcat-Informatics/purrdf/blob/main/generated/rdf-loss-matrix.json),
a generated artifact) and a `LossLedger` API: when a star-incapable codec
drops reifier bindings, or CSV results drop provenance, the realized count is
recorded and surfaced to the caller. See
[Codecs & Determinism](concepts/codecs.md#lossy-projections-are-loud) and
[Result Formats](sparql/results.md).

## Provenance

`purrdf-core` includes a generic **provenance sidecar** for the frozen IR —
attribution, origin sets, and per-quad provenance that engines can carry
without polluting the data graph. The SPARQL results extension
([Result Formats](sparql/results.md#the-provenance-extension)) and the SARIF
boundary both resolve these runtime-only provenance ids to public IRIs at
their serialization edges.

## SSSOM and FnO

Two native interchange codecs live in the kernel:

- **SSSOM** — [Simple Standard for Sharing Ontological
  Mappings](https://mapping-commons.github.io/sssom/) mapping-set TSV support
  (`SssomMappingSet`, `SssomMapping`, with typed diagnostics), for carrying
  cross-vocabulary mappings alongside your data. `SssomSetComment` models
  set-level ordinary comments and provenance as a lossless document envelope,
  independently of YAML-like header metadata. Newly appended provenance uses
  the interoperable metadata-to-table position; parsed after-table comments are
  retained as an explicit extension. Raw Unicode comment lines and their order
  are preserved, while physical line endings serialize deterministically as LF.
- **FnO** — a [Function Ontology](https://fno.io/) function-catalog codec
  (`FnoCatalog`, `fno_to_quads`, `fno_to_ntriples`), used by the slice
  emitters to describe function catalogs as RDF.

As with everything else in the toolkit, these are codecs for *caller* data —
PurRDF does not define mappings or functions of its own. SSSOM envelope comments
also remain caller-neutral projection data: they do not mint RDF predicates,
change mapping validation, or alter the RDF projection.

## Related

- [GTS Graph Transport](gts.md) — the other side of the RDF↔GTS ledger.
- [Design Rules & Invariants](project/design-rules.md) — the
  mints-no-IRIs rule in full.
