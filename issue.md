# Issue #572: Model the XSD anyURI value space in the native datatype engine

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

The native XSD engine leaves xsd:anyURI uninterpreted, so GMEOW maintains whitespace, equality and length branches (Blackcat-Informatics/gmeow-ontology#1803). Implement native XsdDatatype/XsdValue parsing, identity, storage accounting and data ranges. Use fixed XML whitespace collapse and XML Char validation; accept empty, relative and fragment references without URL normalization. Keep the primitive disjoint from string. Verify enumerations, Boolean range combinations, length facets, sound cardinality, governed parsing and WASM portability. Then integrate downstream and delete the superseded branches with no fallback. Normative source: https://www.w3.org/TR/xmlschema11-2/#anyURI

## Comments (0)

