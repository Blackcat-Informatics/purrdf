# Issue #492: Triple-only carrier for RDF 1.2 consumers that use no named graphs (rows, permutations, Datalog atoms, identities)

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

A triple-only mode for consumers that use RDF 1.2 graphs and never named graphs: rows without a graph slot, three permutations instead of six, arity-3 Datalog atoms, and identities (canonicalization, text corpus partitions) that do not carry a graph component. Purely additive: the quad carrier and its named-graph semantics stay as they are.

## Current behaviour (`main` at `6273b6173`)

- `QuadRow { s, p, o, g: Option<TermId> }`, 16 bytes with the niche-packed graph (`crates/rdf-core/src/ir/dataset.rs:100-115`); a triple would be 12.
- Six permutations, three of them graph-led (`Spog, Pos, Osp, Gspo, Gpos, Gosp`, `dataset.rs:462-491`); the non-identity ones are built lazily, and the three non-graph ones still end in `G`.
- Reifier and annotation side-table rows carry a graph slot (`ReifierRow`, `AnnotationRow`, `dataset.rs:59, 62, 279-288`).
- Datalog: "ONE arity-4 relation `triple(subject, predicate, object, graph)`, physically partitioned by its `(predicate, graph)` positions" (`crates/datalog/src/id.rs:196-200`); `POSITION_GRAPH = 3`, `ATOM_ARITY = 4` (`crates/datalog/src/plan.rs:397-400`). Every atom carries the default-graph constant, which also enters `sips_order`'s ranking key on every atom (see the #479 comment).
- RDFC: "its output is always quad-shaped" (`crates/rdf-core/src/ir/canon.rs`, module docs near line 120).
- Text: documents are `(graph, subject, language)` and partitions `(graph, language)` (`crates/text/src/ranking.rs:19-23`, `INDEX_CORPUS_PROFILE_ID`).
- Packs store `(s, p, o)` per graph partition (`ir/pack/triples.rs:12-18`), so the graph is carried there too, as a partition.

## Why it matters

Katamari's data model is one RDF 1.2 graph per vault, by decision: "No named graph, graph name, quad or dataset-kind shall appear in authority, projection, page layout, index key, export or wire format" (Katamari ADR-0038 38.1). Claims are reifiers with annotations; standpoint and lifecycle are annotations, not graphs.

At Katamari's scale (one mail vault: about 300 million claims, 2-3 billion naive RDF 1.2 triples, the large majority in reifier and annotation rows) the graph slot is a constant cost: a quarter of every in-memory row, a fourth column in every Datalog atom and partition key, and a component of canonical and text identities that is always the default graph. More importantly it is a semantic surface the consumer has to prove is unused at every boundary. A mode where the graph does not exist makes that a type-level fact.

The upcoming v4.0 change to remembered empty named graphs (#471) is a good moment to make the graph-free shape explicit, since it moves the default further toward named-graph semantics.

## Proposed direction (a sketch)

- A `TripleDataset` (or a carrier parameter) whose rows are `(s, p, o)` and whose side tables are `(reifier, triple)` and `(reifier, predicate, object)`. It implements `DatasetView` with `GraphMatch::Default` only; any named-graph request is a typed refusal, never an empty answer.
- Three permutations (`SPO`, `POS`, `OSP`) plus the side-table orders.
- Datalog over an arity-3 `triple(s, p, o)` relation partitioned by predicate, selected at compile time when the input is triple-only.
- RDFC and the dataset digest over triples, with an identity distinct from the quad-shaped one (or a documented proof that the quad form with every graph default is byte-identical, in which case only the documentation changes).
- Text corpus profile without the graph component, as a distinct profile identity.

## Acceptance criteria

- A triple-only dataset holding N triples uses 12N bytes of row storage (plus permutations), not 16N.
- SPARQL conformance (the default-graph portion) passes unchanged on the triple-only carrier; `GRAPH ?g { … }` is refused with a typed diagnostic.
- SPARQL 1.2 RL rules over a triple-only input produce the same model as over the quad carrier with all-default graphs (test), with arity-3 atoms.
- RDFC over a triple-only dataset equals the quad form's canonical output for the same triples (or the distinct identity is documented and tested).

## Related

#471, #383, #400.


## Comments (0)

