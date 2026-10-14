# Issue #491: RDF 1.2: index reifiers by triple term and annotations by (predicate, object); reifiers_of is a linear scan

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

Index RDF 1.2 reifier and annotation rows from the directions queries actually come from: triple term to reifiers, and (annotation predicate, object) to reifiers, alongside today's reifier-keyed lookups. Expose them through `DatasetView` so the statement layer and the planner use them when the reifier is unbound.

## Current behaviour (`main` at `6273b6173`)

- Reifiers and annotations live in side tables beside the quad table: `reifiers: Box<[ReifierRow]>` sorted by `(reifier, triple, graph)` and `annotations: Box<[AnnotationRow]>` (`crates/rdf-core/src/ir/dataset.rs:279-288`; row types at `dataset.rs:59, 62`).
- Reifier-keyed lookups are range-addressed: `reifier_quads_of` (`dataset.rs:1730`), `annotations_of_with_graph` (`dataset.rs:1635`); the pack side tables likewise (`ir/pack/side.rs:563`).
- `reifiers_of(triple)` is a linear scan of the whole reifier table (`dataset.rs:1609-1617`), justified as "The table is small (a few bindings per statement), so this is not a hot path". The field doc above it (`dataset.rs:282-284`) still says `reifiers_of` stays "range-addressable", which contradicts the implementation.
- In the statement layer (`crates/sparql-eval/src/statement_layer.rs:126-168`), a bound subject (reifier) uses the keyed lookup; an unbound subject narrows only by graph and scans (`reifier_quads_in_graph`, `annotation_quads_in_graph`). The `DatasetView` defaults are full scans with a filter (`crates/rdf-core/src/dataset_view.rs:736-738, 773-790`).
- So `?r rdf:reifies <<( :s :p :o )>>`, `?r rdf:reifies <<( :s :p ?o )>>` and `?r :validFrom ?t FILTER(?t > …)` all scan every reifier or every annotation.

## Why it matters

For Katamari the reifier table is not small: **every claim is a reifier** (ADR-0038 38.2). A mail vault holds about 300 million claims, each with six to eight annotations (standpoint, valid time, asserted-at, observed-at, ingested-at, producer, confidence, operation), so the side tables are the bulk of the data, around 2-3 billion rows. The common questions start from the statement or from an annotation value, not from the reifier:

- "What does anyone claim about `:msg123 :from ?x`?" (triple term, or its subject and predicate, to reifiers)
- "Which claims did this producer make?" / "Which claims are valid at this time?" (annotation predicate and object to reifiers)
- Retraction and supersession chains ("which claims name this one as prior?") are annotation-object lookups.

Each is a full scan today. Katamari's scale review found the same pattern in Oxigraph (scans of `rdf:reifies` for a component). Katamari will keep its own one-row-per-claim index (ADR-0038 38.13) and present the RDF 1.2 triples virtually over it, but it needs purrdf's evaluator to ask for these lookups through `DatasetView` so that a backend that has the index can answer them, and so purrdf's own datasets do not scan.

## Proposed direction (a sketch)

- **Two more orders on the side tables**, built at freeze like the quad permutations: reifiers by `(triple, reifier)`; annotations by `(predicate, object, reifier)`. Optionally reifiers by the triple term's `(subject, predicate)` components, since triple terms are interned with their components.
- **`DatasetView` methods** with scanning defaults for backends that lack them: `reifiers_of_triple(triple)`, `reifiers_of_components(s?, p?, o?)`, `reifiers_with_annotation(predicate, object?)`, each range-addressed where the backend can.
- **Statement layer**: when the reifier is unbound and the triple term or annotation (predicate, object) is bound, use them.
- **Planner**: `cardinality_estimate` for these patterns uses the new orders (ties in with the statistics issue #484).
- Fix the `dataset.rs:282-284` doc.

## Acceptance criteria

- `reifiers_of(triple)` is a range lookup (test: equals the filtered full scan; benchmark: sublinear in table size).
- With 10^7 reifiers and 6 annotations each, `?r rdf:reifies <<( :s :p ?o )>>` and `?r :producer :x` touch only matching rows (governor evidence: candidates examined proportional to matches).
- The same lookups work on `RdfDataset`, packs, paged and segmented datasets (or report their scanning default honestly in EXPLAIN).
- Results unchanged across the RDF 1.2 conformance suites.

## Related

#484 (statistics, including reifier and annotation rows), #288, #401.


## Comments (0)

