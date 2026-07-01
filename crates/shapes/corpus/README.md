# purrdf-shapes conformance corpus

Each directory contains `data.nt` (N-Triples data graph), `shapes.ttl` (Turtle shapes graph), and `expected-report.nt` (frozen N-Triples validation report). The conformance test compares produced tuple sets against frozen expected reports.

| Case | Constraint | Conforms | Expected violations |
|------|-----------|---------|---------------------|
| 01-min-count | `sh:minCount 1` on `ex:name` | false | 1 — focus `ex:alice`, path `ex:name`, MinCountConstraintComponent |
| 02-max-count | `sh:maxCount 1` on `ex:nickname` (2 values present) | false | 1 — focus `ex:alice`, path `ex:nickname`, MaxCountConstraintComponent |
| 03-class | `sh:class ex:Person` on `ex:author` (value is Animal) | false | 1 — focus `ex:doc`, path `ex:author`, value `ex:bob`, ClassConstraintComponent |
| 04-datatype | `sh:datatype xsd:decimal` on `ex:price` (plain string) | false | 1 — focus `ex:item`, path `ex:price`, value `"expensive"`, DatatypeConstraintComponent |
| 05-node-kind | `sh:nodeKind sh:IRI` on `ex:link` (literal present) | false | 1 — focus `ex:s`, path `ex:link`, value `"not-an-iri"`, NodeKindConstraintComponent |
| 06-in | `sh:in` list of 3 hex codes; `ex:purple` has `"#800080"` | false | 1 — focus `ex:purple`, path `ex:hex`, value `"#800080"`, InConstraintComponent |
| 07-has-value | `sh:hasValue ex:petrol` on `ex:fuel`; car has diesel | false | 1 — focus `ex:car`, path `ex:fuel`, HasValueConstraintComponent |
| 08-pattern | `sh:pattern "^[a-z0-9]+$"` on `ex:username`; value has space | false | 1 — focus `ex:user`, path `ex:username`, value `"invalid user"`, PatternConstraintComponent |
| 09-min-length | `sh:minLength 3` on `ex:name`; value `"ab"` has length 2 | false | 1 — focus `ex:tag`, path `ex:name`, value `"ab"`, MinLengthConstraintComponent |
| 10-unique-lang | `sh:uniqueLang true` on `ex:label`; two `@en` literals | false | 1 — focus `ex:alice`, path `ex:label`, UniqueLangConstraintComponent |
| 11-min-inclusive | `sh:minInclusive 5` on `ex:rating`; value is 3 | false | 1 — focus `ex:product`, path `ex:rating`, value `"3"^^xsd:integer`, MinInclusiveConstraintComponent |
| 12-max-inclusive | `sh:maxInclusive 100` on `ex:score`; value is 110 | false | 1 — focus `ex:exam`, path `ex:score`, value `"110"^^xsd:integer`, MaxInclusiveConstraintComponent |
| 13-or | `sh:or ([sh:nodeKind sh:IRI] [sh:nodeKind sh:BlankNode])` on `ex:p`; integer literal fails both | false | 1 — focus `ex:val`, path `ex:p`, value `"123"^^xsd:integer`, OrConstraintComponent |
| 14-and | `sh:and ([sh:nodeKind sh:IRI] [sh:class ex:Bar])`; node is IRI but not typed Bar | false | 1 — focus `ex:node`, value `ex:node`, AndConstraintComponent |
| 15-xone | `sh:xone ([sh:nodeKind sh:Literal] [sh:nodeKind sh:BlankNode])`; IRI target matches neither | false | 1 — focus `ex:thing`, value `ex:thing`, XoneConstraintComponent |
| 16-node | `sh:node ex:KidShape` on `ex:child`; kid has non-integer age | false | 1 — focus `ex:parent`, path `ex:child`, value `ex:kid`, NodeConstraintComponent |
| 17-inverse-path | `sh:inversePath ex:hasMember` minCount 1; orphan has no members | false | 1 — focus `ex:orphan`, path `ex:hasMember`, MinCountConstraintComponent |
| 18-target-subjects-of | `sh:targetSubjectsOf ex:email`; sender lacks `ex:verified` | false | 1 — focus `ex:sender`, path `ex:verified`, MinCountConstraintComponent |
| 19-target-objects-of | `sh:targetObjectsOf ex:knows`; target lacks `ex:name` | false | 1 — focus `ex:target`, path `ex:name`, MinCountConstraintComponent |
| 20-no-inference-contract | `sh:targetClass ex:Person` but bob is typed `ex:Employee` (no direct Person type) | **true** | 0 — SHACL has no rdfs:subClassOf inference |
| 21-rdf12-statement-layer | Statement node whose `rdf:reifies` object is a genuine RDF 1.2 triple term `<<( ex:alice ex:knows ex:bob )>>`; `sh:minCount 1` on `rdf:reifies` | **true** | 0 — proves oxigraph (rdf-12) ingests + validates triple-term data that rdflib/pySHACL cannot represent |
| 22-min-length-pattern | `sh:minLength 1` + `sh:pattern "^[a-z0-9-]{1,20}$"` on `ex:slug`; value exceeds 20 chars (pattern fails) | false | 1 — focus `ex:post`, path `ex:slug`, PatternConstraintComponent |
| 23-datatype-lexical | `sh:datatype xsd:decimal` on `ex:value`; value `"1e3"^^xsd:decimal` uses scientific notation (not lexically valid for `xsd:decimal`) | false | 1 — focus `ex:measurement`, path `ex:value`, value `"1e3"^^xsd:decimal`, DatatypeConstraintComponent |

## Notes

- **Case 21**: Uses the genuine RDF 1.2 **triple-term** syntax `<<( s p o )>>` (object position, with `rdf:reifies`) — verified accepted by oxigraph 0.5's N-Triples 1.2 parser. The older RDF-star asserted-triple syntax `<< s p o >>` (subject or object) is *rejected* by oxigraph 0.5; only the parenthesised RDF 1.2 triple term is accepted. This case demonstrates ingestion + validation of statement-layer data that rdflib/pySHACL cannot represent at all.
- **Case 22** (`22-min-length-pattern`): Previously misnamed `max-length`. `sh:maxLength` is in the hard-fail unsupported constraint set; this case actually tests `sh:minLength` + `sh:pattern` with a bounded character class to achieve length-limiting intent.
- **Case 23** (`23-datatype-lexical`): Pins char-by-char XSD lexical validation in `check_datatype`. `"1e3"^^xsd:decimal` is rejected because scientific notation is outside the `xsd:decimal` lexical space (unlike `xsd:double`); previously a native `f64` parse wrongly accepted it. Companion unit tests also pin unbounded `xsd:integer` (no `i64` overflow).
- Expected reports encode the intended outcomes (the table above is the spec). They are cross-checked against the engine output, but the intended violations — not a blind snapshot — are the source of truth.
