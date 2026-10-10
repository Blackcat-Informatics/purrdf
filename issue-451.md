# xsd:dateTimeStamp ranges are not exactly decided (data-range boundary)

Source: https://github.com/Blackcat-Informatics/purrdf/issues/451
State: OPEN; updated: 2026-10-06T01:25:00Z; full comments: 0.

## Summary

`xsd:dateTimeStamp` is not exactly decided by `purrdf_xsd::range::is_exactly_decided`, so any ontology that uses it as a range makes `Reasoner::consistency` report `completeness decided-within-boundaries` with a `data-range` boundary. `xsd:dateTime` and `xsd:date` decide exactly.

## Reproduction (purrdf 3.0.0 from crates.io)

```
<http://example.org/p> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#DatatypeProperty> .
<http://example.org/p> <http://www.w3.org/2000/01/rdf-schema#range> <http://www.w3.org/2001/XMLSchema#dateTimeStamp> .
```

`Reasoner::new(&dataset)?.consistency()` with the range set to each datatype:

| range | answer | completeness | boundaries |
|---|---|---|---|
| `xsd:dateTimeStamp` | true | decided-within-boundaries | `data-range` |
| `xsd:dateTime` | true | decided | none |
| `xsd:date` | true | decided | none |
| `xsd:integer` | true | decided | none |

## Expected

`xsd:dateTimeStamp` is an OWL 2 datatype: `xsd:dateTime` restricted to values with a timezone (`explicitTimezone = required`). Its value space is a subset of `xsd:dateTime`'s, which is already decided exactly. The range should be decided exactly, with no boundary.

## Impact

This is the only boundary that the gmeow-ontology production class world `graph/imports` raises. That world declares `rdfs:range xsd:dateTimeStamp` on two properties. Because of the boundary, that world's consistency verdict can never be exact, whatever the reasoner's speed.


## Comments

None.

