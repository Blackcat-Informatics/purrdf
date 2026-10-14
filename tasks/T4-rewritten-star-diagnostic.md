# Rewritten ProjectStar serializer diagnostic

The initial persistent implementation emits lexical `SELECT *` for every
`ProjectStar` without serializer witnesses. A public algebra caller can retain
the star's saved projected-name snapshot while changing its inner pattern.

The unchanged regression `rewritten_star_inner_roundtrips_its_saved_projection`
parsed `SELECT * WHERE { ?s ?p ?o }`, retained its 3-name snapshot, and placed
it over the parsed body of `SELECT * WHERE { ?s ?p ?o BIND(1 AS ?extra) }`.
The optimized native focused test failed terminal 101 on 2026-10-13:

```
carrier: SELECT * WHERE { ?s ?p ?o . BIND("1"^^<http://www.w3.org/2001/XMLSchema#integer> AS ?extra) }
saved:    [?s, ?p, ?o]
reparsed: [?s, ?p, ?o, ?extra]
```

The evaluator projects the saved three-name snapshot; the rendered carrier
therefore changes observable columns. The serializer must preserve saved
projection semantics without expanding all parser-built nested stars into
quadratic source text. This was a confirmed defect.

The serializer now checks that the saved snapshot matches current inner
visibility before choosing lexical `*`. Parser-built nested
`Extend(inner ProjectStar, variable)` uses the shared persistent trie's prefix
relation, visiting only the copied path; a public rewrite of another shape
uses the existing visibility computation and emits the explicit saved names
when they differ. The unchanged focused regression passed after the fix, and
the original 2k/4k/8k carrier test passed with rendered bytes
177,776/357,776/717,776. Its allocator requested bytes were
3,276,656/6,553,456/13,107,056 and peak working bytes were
1,212,416/2,424,832/4,849,664. This is an affected native control;
optimized WASM and the final gate remain open.

A saved empty snapshot over a newly visible inner binding exposed a second
branch that would still emit `*`. Exact zero-column projection over visible
bindings has no SPARQL query-text spelling. The existing carrier validator now
returns typed `ParseError::Unsupported` for this state in either explicit or
persistent projection form, before emission. The unchanged empty wildcard
still renders, and the saved-empty rewrite control passed. The complete
9-test chain suite passed after this correction.
