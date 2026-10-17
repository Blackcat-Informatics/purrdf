# Issue #590: SPARQL: expose borrowed native value equality for integrity consumers

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Expose borrowed native SPARQL value equality for integrity consumers

The downstream authored-integrity runtime must evaluate canonical equality/distinctness over original RDF values. Its actual source-to-enactment regression joins two distinct IRI metadata values but emits no violation, because the compiler currently treats termDistinct as stored data. ORDER BY compare_values is explicitly the wrong semantic relation. Copying the SPARQL equality implementation or rendering values into query text would duplicate authority and degrade the dataflow.

PurRDF already owns the complete implementation in expr::rdf_equal_in_admitted, shared with triple components/IN and supporting numeric value equality, language/direction, scoped blanks, triple terms, NaN, unknown/ill-typed literal expression errors and SEP-0009 composites. Expose a small public borrowed-value entry point using that implementation, preserving Ok(Some(bool)), Ok(None) expression errors and Err operational refusal. No alternate equality engine, source text parsing, query/store construction per comparison, runtime dependency or configuration weakening.

Acceptance: public entry point and facade export; focused native tests against the real query equality path for representative IRI, literal/numeric, scoped blank, triple, composite and expression-error cases; portable WASM coverage where supported; strict affected Clippy and normal hooks. Review actual feedback and merge only with ghprsq. This is a direct prerequisite of GMEOW1843 acceptance2, not a replacement for downstream executable enforcement or the full logic backlog.


## Comments (0)

