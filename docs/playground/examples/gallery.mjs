// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Curated RDF-1.2 vignettes for the PurRDF console. Every fixture uses
// http://example.org/ IRIs and only standard rdf:/sh:/xsd: vocabulary — PurRDF
// mints no vocabulary of its own. Each vignette is a complete console state:
// selecting one loads the Input/SPARQL/SHACL/graph-B panes and runs it.

/** @typedef {{
 *   id: string,
 *   title: string,
 *   blurb: string,
 *   input: string,
 *   inputFormat: string,
 *   query?: string,
 *   shapes?: string,
 *   graphB?: string,
 *   graphBFormat?: string,
 *   activePane?: string,
 * }} Vignette */

const RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/** @type {Vignette[]} */
export const VIGNETTES = [
  {
    id: "quoted-triple",
    title: "Quoted-triple reification",
    blurb:
      "An RDF-1.2 triple term in object position: ex:alice asserts the statement " +
      "«ex:bob ex:likes ex:carol» and stamps it with provenance.",
    inputFormat: "turtle",
    input: `@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# The object of ex:says is itself a triple term (RDF-1.2 «<<( s p o )>>»).
ex:alice ex:says <<( ex:bob ex:likes ex:carol )>> .
ex:alice ex:statedOn "2026-07-06"^^xsd:date .
ex:carol ex:name "Carol" .
`,
    query: `PREFIX ex: <http://example.org/>
SELECT ?who ?claim WHERE {
  ?who ex:says ?claim .
}`,
    activePane: "roundtrip",
  },
  {
    id: "directional-literals",
    title: "Directional literals (ltr + rtl)",
    blurb:
      "RDF-1.2 base-direction literals: an English left-to-right string and an " +
      "Arabic right-to-left string, each carrying an explicit base direction.",
    inputFormat: "turtle",
    input: `@prefix ex: <http://example.org/> .

ex:greetingEn ex:text "Hello, world"@en--ltr .
ex:greetingAr ex:text "مرحبا بالعالم"@ar--rtl .
`,
    query: `PREFIX ex: <http://example.org/>
SELECT ?subject ?text WHERE {
  ?subject ex:text ?text .
}`,
    activePane: "quads",
  },
  {
    id: "shacl-af-rule",
    title: "SHACL-AF sh:rule entailment",
    blurb:
      "A SHACL Advanced-Features triple rule: every ex:Person is entailed to be " +
      "ex:adult ex:yes. Validate for the SARIF report, then Materialize to see " +
      "the inferred triple.",
    inputFormat: "turtle",
    input: `@prefix ex: <http://example.org/> .
@prefix rdf: <${RDF}> .

ex:alice a ex:Person .
ex:bob a ex:Person .
`,
    shapes: `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix rdf: <${RDF}> .

ex:PersonShape
  a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:rule [
    a sh:TripleRule ;
    sh:subject sh:this ;
    sh:predicate ex:adult ;
    sh:object ex:yes ;
  ] .
`,
    activePane: "shacl",
  },
  {
    id: "isomorphic-pair",
    title: "Isomorphic graphs (relabeled blank nodes)",
    blurb:
      "Graph A and Graph B name the same RDF graph under blank-node relabeling and " +
      "reordering. Compare confirms isomorphic: yes and shows identical canonical forms.",
    inputFormat: "turtle",
    input: `@prefix ex: <http://example.org/> .

_:a ex:knows _:b .
_:b ex:name "Bob" .
ex:root ex:member _:a .
`,
    graphBFormat: "turtle",
    graphB: `@prefix ex: <http://example.org/> .

ex:root ex:member _:x .
_:y ex:name "Bob" .
_:x ex:knows _:y .
`,
    activePane: "identity",
  },
  {
    id: "service-hardfail",
    title: "SERVICE hard-fail (never-silent errors)",
    blurb:
      "There is no server: a federated SERVICE clause cannot be resolved in-browser. " +
      "Running this query surfaces the engine error in the live banner — never a " +
      "silent empty result.",
    inputFormat: "turtle",
    input: `@prefix ex: <http://example.org/> .

ex:alice ex:knows ex:bob .
`,
    query: `PREFIX ex: <http://example.org/>
SELECT ?remote WHERE {
  SERVICE <http://example.org/sparql> {
    ?remote ex:knows ex:bob .
  }
}`,
    activePane: "sparql",
  },
];

/** Look up a vignette by id. */
export function findVignette(id) {
  return VIGNETTES.find((v) => v.id === id);
}

/** The default console state used on a cold load (no permalink). */
export const DEFAULT_STATE = {
  input: `@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# RDF-1.2 highlights, all in the http://example.org/ namespace:
#   * a triple term in object position (a quoted triple), and
#   * base-direction literals (ltr + rtl).
ex:alice ex:says <<( ex:bob ex:likes ex:carol )>> .
ex:alice ex:statedOn "2026-07-06"^^xsd:date .
ex:carol ex:name "Carol" .
ex:greetingEn ex:text "Hello, world"@en--ltr .
ex:greetingAr ex:text "مرحبا بالعالم"@ar--rtl .
`,
  inputFormat: "turtle",
  query: `PREFIX ex: <http://example.org/>
SELECT ?s ?p ?o WHERE {
  ?s ?p ?o .
}
ORDER BY ?s ?p ?o`,
  shapes: `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix rdf: <${RDF}> .

# Every ex:Person must carry an ex:name; a triple rule stamps them ex:adult ex:yes.
ex:PersonShape
  a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [
    sh:path ex:name ;
    sh:minCount 1 ;
  ] ;
  sh:rule [
    a sh:TripleRule ;
    sh:subject sh:this ;
    sh:predicate ex:adult ;
    sh:object ex:yes ;
  ] .
`,
  graphA: `@prefix ex: <http://example.org/> .

_:a ex:knows _:b .
_:b ex:name "Bob" .
ex:root ex:member _:a .
`,
  graphAFormat: "turtle",
  graphB: `@prefix ex: <http://example.org/> .

ex:root ex:member _:x .
_:y ex:name "Bob" .
_:x ex:knows _:y .
`,
  graphBFormat: "turtle",
  activePane: "input",
};
