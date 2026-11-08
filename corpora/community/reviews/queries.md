<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Query and SERVICE oracle derivations

These expectations follow from the source graphs and query text, independently
of engine observations. The review registry binds each case to its exact
query, data, expected-result and manifest bytes, and to this derivation.

`ex:` denotes `http://example.org/`. The interaction data has exactly `s t o`,
`s p a`, `s p b`, `a r o`, `b r o`, `o q s`, and `wrong r other`. The blank-cycle
source has exactly `A link B` and `B link A` for distinct blanks. The two named
sources independently spell `_:same`: g1 has `p "one"` and g2 has `p "two"`.
Local SERVICE data has one blank `p o` subject; each independently parsed
endpoint has one blank `p o` subject and tags `"one"`, `"two"`. Identical
endpoint bytes do not identify their independently scoped source blanks.

All fifteen cases require unordered SELECT bags with exact headers. Blank
letters below denote global equality classes rather than lexical labels.
Empty rows contribute multiplicity even though they contain no bound cells.

| Case | Independently derived header and bag | Controlling rule and neighboring control |
|---|---|---|
| mixed-predicate-inverse-sequence | Header o; four ex:o rows: direct 1, inverse 1, sequence 2. The disconnected wrong/r/other edge has no p predecessor from s. | SPARQL 1.1 path translation and Union/Project. Explicit UNION is the neighbor. |
| mixed-equivalent-union | Header o; the same four ex:o rows from the three explicit branches. | Join/Union/Project. The path expression is the neighbor. |
| mixed-permuted-union | Header o; four ex:o rows. Reversing conjunction order and permuting UNION branches preserve all compatible witnesses. | Join and Union. The preceding explicit UNION is the neighbor. |
| mixed-overlapping-arm | Header o; six ex:o rows, from 1+1+2+2. Repeating the sequence arm does not deduplicate its derivations. | Path translation into bag UNION. The four-row path is the neighbor. |
| all-unbound-visible-rows | Header never; two empty rows. Projecting the two VALUES mappings preserves cardinality. | VALUES and Project. The two-column UNDEF case is the neighbor. |
| duplicate-unbound-rows | Header a,b; two empty rows from the two UNDEF/UNDEF mappings. | VALUES bag semantics. The preceding unbound projection is the neighbor. |
| bnode-fresh-per-call | Header x,left,right; rows ("a",A,B), ("b",C,D), with four distinct blank classes. | BNODE without an argument is fresh per call. Literal-argument memoization is the neighbor. |
| bnode-string-memo | Header x,first,again,fresh; rows ("a",A,A,B), ("b",C,C,D), with four distinct classes. | Equal literal BNODE arguments share within one solution, while different mappings and argument-free calls remain distinct. The per-call case is the neighbor. |
| global-blank-cycle | Header from,to; rows (A,B), (B,A), with A distinct from B. | Basic pattern matching preserves source co-reference across rows. Independent-source scope is the neighbor. |
| separate-source-blank-scopes | Header graph,node,value; (g1,A,"one"), (g2,B,"two"), with A distinct from B despite equal source labels. | Dataset construction standardizes separate RDF sources apart. The cycle is the co-reference neighbor. |
| service-bnode-distinct | Header existing,local,remote; one row (A,B,C), with three distinct classes. | SERVICE response scope and BNODE freshness. The equality predicate below is the negative neighbor. |
| service-bnode-collision | Header existing,local,remote; zero rows. The one candidate mapping has no equal blank pair. An execution error cannot satisfy the empty bag. | Same source-scope and BNODE rules. The preceding inequality case is the positive neighbor. |
| service-response-shared | Header remote,copy,tag; (A,A,"one"), (A,A,"two"), with one global blank class. | One response retains blank co-reference across all rows and BIND preserves identity. Separate endpoints are the distinction neighbor. |
| service-responses-distinct | Header a,b; one row (A,B), with A distinct from B. Each independently parsed endpoint contributes one p/o subject. | Independent SERVICE source scopes. The preceding shared-response case is the neighbor. |
| service-all-unbound-duplicates | Header a,b; two empty rows. Remote SELECT projection and outer SERVICE join preserve both VALUES mappings. | SERVICE, VALUES and Project. The native duplicate-unbound case is the neighbor. |

The controlling Recommendation clauses are [property-path translation](https://www.w3.org/TR/sparql11-query/#sparqlTranslatePathPatterns),
[algebra operators](https://www.w3.org/TR/sparql11-query/#sparqlAlgebra),
[VALUES](https://www.w3.org/TR/sparql11-query/#inline-data),
[BNODE](https://www.w3.org/TR/sparql11-query/#func-bnode),
[dataset construction](https://www.w3.org/TR/sparql11-query/#queryDataset),
[SERVICE](https://www.w3.org/TR/2013/REC-sparql11-federated-query-20130321/),
and [JSON result blank identity](https://www.w3.org/TR/2013/REC-sparql11-results-json-20130321/#select-encoding).

This is a community normative review. It establishes neither Working Group
approval nor external-engine certification.
