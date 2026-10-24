# Implementation review

Scope: the two production compilation paths, private OpenAPI packaging, schema
policy key, native closure/equivalence regressions and supported golden delta.

Both production callers serialize the original JSON Schema before transferring
the definition catalog. The OpenAPI walk uses the shared catalog's schema-position
keyword sets, preserving const/enum/default/examples and property names such as
$ref. It rewrites only definition-local reference prefixes, retaining escaped
suffix tokens. It never dereferences an edge, so cycles do not unfold. Every
emitted node is visited at most once; no map lookup or catalog copy is introduced
per reference. The borrowing JSON macro is avoided at the catalog insertion.

The cache policy changes with the output. The full fixture diff has exactly 36
reference-prefix substitutions in OpenAPI sections, with no JSON Schema,
coverage, constraint, wire-format or frozen-epoch change. Native evidence covers
85 schema-module controls and all 29 ontology-surface controls, including actual
large/deep/tree callers and importer/determinism checks. Native checks, not WASM
execution, own these semantics.

This is the implementer's review, not independent judgment. Automated PR review,
normal hooks, strict all-target Clippy and hosted gates remain to be completed
before landing. No acceptance is inferred from artifact presence.
