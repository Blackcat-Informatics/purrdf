# Named-graph support through existing views

Current source audit; runtime of the added adapter controls remains pending.

The source walk is retained in `dataset-adapter-source-census.txt`. The only
shipping DatasetView adapters parameterized by another arbitrary DatasetView
are Arc<T> (dataset_view.rs) and private ResidueView<D> (pipeline_bundle.rs).
Both forward supports_named_graphs and require_named_graphs to the original
read owner. Their read errors and borrowed term guards already forward that
source. The missing-IRI GraphSelector regression now exercises Arc<TripleRead>
for empty and populated carriers, with an empty quad dataset retaining Ok(None).

Closed concrete views cannot silently wrap TripleRead:

- CompositeSource Carrier contains Native(Arc<RdfDataset>), Delta, or selected
  CompositeDatasetView. DatasetHandle owns only RdfDataset. DeltaDatasetView
  contains that handle plus Arc<RdfDataset>.
- ShaclDatasetView Source contains native, composite, or delta sources;
  ClassMembershipView takes Arc<RdfDataset>. Their restricted, selected, and
  union projections do not accept an arbitrary DatasetView.
- PagedQueryView borrows PagedDataset; PagedStackQueryView owns its concrete
  snapshot and paged query. SegmentedStackQuery opens concrete certified quad
  providers. PackView validates its fixed quad/side-table format.
- LedgerDataset renders LedgerSource statements and its own term dictionary;
  it does not wrap DatasetView. ForeignDatatypeView owns Arc<RdfDataset>.

SPARQL ActiveDataset is resolved graph-scope state, not an adapter. The generic
evaluator retains the original D. Query preparation checks GRAPH (including
nested expressions/subqueries) and FROM/FROM NAMED before empty/missing-term
execution; raw eval checks the pattern too. ActiveDataset clause resolution
itself requires named support before IRI lookup. GraphSelector now requires
named support before resolving a possibly absent IRI. Canonical graph_scope
uses that source requirement. Concrete quad views retain their existing
healthy named-graph support; no redundant capability implementations added.
