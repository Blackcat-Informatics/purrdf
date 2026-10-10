# rdf:langString ranges are reported as unrecognized-term

Source: https://github.com/Blackcat-Informatics/purrdf/issues/453
State: OPEN; updated: 2026-10-06T03:30:59Z; full comments: 0.

## Summary

`rdf:langString` used as a datatype is reported as an `unrecognized-term` boundary. RDF 1.2 defines `rdf:langString`, and `rdf:dirLangString` for directional language strings, as the datatypes of language-tagged literals.

## Reproduction (purrdf 3.0.0 from crates.io)

```
<http://example.org/p> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#DatatypeProperty> .
<http://example.org/p> <http://www.w3.org/2000/01/rdf-schema#range> <http://www.w3.org/1999/02/22-rdf-syntax-ns#langString> .
```

Result: `true`, `decided-within-boundaries`, boundaries `{unrecognized-term}`.

## Expected

`rdf:langString` (and `rdf:dirLangString`) should be recognized datatypes whose value space is the language-tagged strings, decided exactly like `xsd:string`.

## Impact

The gmeow-ontology production class world `graph/logic` declares `rdfs:range rdf:langString` (4 uses), which contributes one of its boundaries.


## Comments

None.

