Expose borrowed native SPARQL value equality

Refs #590

Integrity consumers holding native RDF values could not reach SPARQL equality without copying semantics or constructing queries. Make the existing admitted comparator public as `equal_values`, export its existing `LanguageStringEquality` profile, and retain the caller's workspace capability. Internal expression callers use the same function; the obsolete private name is removed.

Public native and actual WASM checks match the query operator over14 representative pairs, including scoped blanks, numeric lexical differences/NaN, language/direction, triple components, composites and expression errors. An explicit bounded-account control verifies the original typed allocation refusal and release. Strict affected all-target Clippy, full helper hygiene and normal commit hooks passed. One additional portable case is registered under the existing WASM target and its exact inventory is updated.

The GMEOW integrity consumer is being wired separately; this API alone does not complete GMEOW1843. Hosted CI is separate from the executed local checks. The imported-closure proof replay failure in merged589 is owned by #591 and is not resolved here.
