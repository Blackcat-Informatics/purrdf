# Issue #553: Segmented stack view: bound-reifier patterns drop annotations and scan the reifier table; block cache lookup is linear

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

At `c2c1c8c78`, `SegmentedStackQuery` (`crates/rdf-core/src/ir/segmented/query/view.rs`) overrides neither `reifier_quads_of` nor `annotations_of_with_graph`. sparql-eval's statement layer calls both for a bound reifier (`crates/sparql-eval/src/statement_layer.rs:131,152`). The trait defaults (`crates/rdf-core/src/dataset_view.rs:1082,1096`) are a full reifier-table filter and an empty iterator. So `SELECT (COUNT(*) AS ?n) WHERE { <r> ?p ?o }` over a one-layer stack answers 1 (`rdf:reifies` only) where the image holds 4 or more rows for that reifier, and reads 172.6 MB of a 190.8 MB image. `SegmentedSession` overrides both (`crates/rdf-core/src/ir/segmented/session.rs:1726,1729`).

Separately, `SegmentedSession::block` (`crates/rdf-core/src/ir/segmented/session.rs:955-976`) keeps its block cache as a `Vec`: a hit is a linear `position` plus `remove`/`push`, so each block access costs O(cache entries). In a release profile of the query above it is 84% of self time (cache of 4,096 entries).

Measured on a 200,000-claim image (600,263 terms, 4 KiB blocks, 190,832,640 bytes): the bound-subject count took 3,145 ms cold / 429 ms warm with 636,769 block reads, while `SELECT ?r WHERE { ?r rdf:reifies ?t } LIMIT 10` took 12 ms and 2,703 reads. `COUNT(*)` over everything answered 1,000,000 (5 rows a claim through the unkeyed walks, which do include annotations), so only the keyed path loses them.


## Comments (0)

