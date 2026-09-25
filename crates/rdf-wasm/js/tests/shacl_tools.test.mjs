// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Node real-execution coverage of the shapes-graph tools reached through the PUBLIC
// package root (`../index.mjs`): `shaclApplyRules`, `shaclEvalNodeExpr` and
// `shaclLintShapes`. Every shapes graph carries the W3C SHACL 1.2 declaration of
// `sh:SPARQLExprExpression` verbatim — once refused as a bodiless custom function —
// beside shapes that call `sh:sparqlExpr` with `sh:prefixes`.

import { test } from "node:test";
import assert from "node:assert/strict";

import { ready, shaclApplyRules, shaclEvalNodeExpr, shaclLintShapes } from "../index.mjs";

await ready();

const PREFIXES = `@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ex: <http://example.org/ns#> .
`;

const SNIPPET = `
sh:SPARQLExprExpression a sh:NamedParameterExpressionFunction ;
  rdfs:label "SPARQL expr expression"@en ;
  rdfs:comment "The class of node expressions based on SPARQL expressions (sh:sparqlExpr)."@en ;
  rdfs:isDefinedBy sh: ;
  rdfs:subClassOf sh:NamedParameterExpression,
  sh:SPARQLExecutable ;
  sh:parameter sh:SPARQLExprExpression-prefixes,
  sh:SPARQLExprExpression-sparqlExpr .

sh:SPARQLExprExpression-prefixes a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:description "The prefixes that shall be applied before parsing the SPARQL query that gets derived from the sh:sparqlExpr expression. The object should define those prefixes using sh:declare."@en ;
  sh:name "prefixes"@en ;
  sh:nodeKind sh:BlankNodeOrIRI ;
  sh:path sh:prefixes .

sh:SPARQLExprExpression-sparqlExpr a sh:Parameter ;
  rdfs:isDefinedBy sh: ;
  sh:datatype xsd:string ;
  sh:description "The SPARQL expression that is executed during evaluation of this node expression."@en ;
  sh:keyParameter true ;
  sh:name "SPARQL expr"@en ;
  sh:path sh:sparqlExpr .
`;

// A sh:sparqlExpr node naming ex:yes through sh:prefixes, a labelled shnex:var node, a
// rule tagging every ex:Item through the same expression, and a counter rule stepping
// ex:n to 5 — exactly four term-generating rounds.
const TOOLS = `
ex:Prefixes sh:declare [ sh:prefix "ex" ; sh:namespace "http://example.org/ns#"^^xsd:anyURI ] .
ex:Tag sh:sparqlExpr "ex:yes" ; sh:prefixes ex:Prefixes .
_:suffix shnex:var "suffix" .

ex:Tagger a sh:NodeShape ;
  sh:targetClass ex:Item ;
  sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:tagged ;
            sh:object [ sh:sparqlExpr "ex:yes" ; sh:prefixes ex:Prefixes ] ] .

ex:Counter a sh:NodeShape ;
  sh:targetSubjectsOf ex:n ;
  sh:rule [ a sh:SPARQLRule ; sh:construct """PREFIX ex: <http://example.org/ns#>
CONSTRUCT { $this ex:n ?m } WHERE { $this ex:n ?k . FILTER(?k < 5) BIND(?k + 1 AS ?m) }""" ] .
`;

const SHAPES = PREFIXES + SNIPPET + TOOLS;

const INTEGER = "<http://www.w3.org/2001/XMLSchema#integer>";

const DATA = `<http://example.org/ns#a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Item> .
<http://example.org/ns#a> <http://example.org/ns#n> "1"^^${INTEGER} .
`;

const INFERRED =
  [2, 3, 4, 5]
    .map((n) => `<http://example.org/ns#a> <http://example.org/ns#n> "${n}"^^${INTEGER} .\n`)
    .join("") +
  "<http://example.org/ns#a> <http://example.org/ns#tagged> <http://example.org/ns#yes> .\n";

test("shaclApplyRules writes the inference graph, its proof, and honours the round limit", () => {
  const plain = shaclApplyRules(DATA, SHAPES);
  assert.equal(plain.inferred, INFERRED);
  assert.equal(plain.proof, undefined);
  plain.free();

  const explained = shaclApplyRules(DATA, SHAPES, undefined, undefined, undefined, true);
  assert.equal(explained.inferred, INFERRED);
  assert.equal(explained.proof.split("derived ").length - 1, 5);
  explained.free();

  assert.throws(
    () => shaclApplyRules(DATA, SHAPES, undefined, undefined, undefined, false, 3n),
    (error) =>
      error.message ===
      "SHACL rules did not complete: 4 rounds inferred a term the evaluation graph " +
        "did not hold, past the limit of 3 such rounds (SHACL 1.2 Inference Rules: " +
        '"Rule engines MAY also report a failure after a pre-configured maximum ' +
        'iteration count has been exceeded"); if the rule set terminates, raise the ' +
        "limit with RuleOptions::with_max_term_generating_rounds",
  );
  const enough = shaclApplyRules(DATA, SHAPES, undefined, undefined, undefined, false, 4n);
  assert.equal(enough.inferred, INFERRED);
  enough.free();

  const srl = shaclApplyRules(
    DATA,
    undefined,
    "PREFIX ex: <http://example.org/ns#>\nRULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\nDATA { ex:d ex:q 2 }\n",
    undefined,
    undefined,
    true,
  );
  assert.equal(
    srl.inferred,
    `<http://example.org/ns#a> <http://example.org/ns#q> "1"^^${INTEGER} .\n` +
      `<http://example.org/ns#d> <http://example.org/ns#q> "2"^^${INTEGER} .\n`,
  );
  assert.ok(srl.proof.includes("  data-block\n"), srl.proof);
  srl.free();

  assert.throws(
    () => shaclApplyRules(DATA),
    (error) =>
      error.message === "no rule source: name a SHACL shapes graph or a SPARQL 1.2 RL rule set",
  );
});

test("shaclEvalNodeExpr evaluates one expression node, natively and with a scope", () => {
  assert.deepEqual(
    shaclEvalNodeExpr(SHAPES, DATA, "http://example.org/ns#Tag", "http://example.org/ns#a"),
    ["<http://example.org/ns#yes>"],
  );
  assert.deepEqual(
    shaclEvalNodeExpr(SHAPES, DATA, "_:suffix", "http://example.org/ns#a", ['suffix="!"@en']),
    ['"!"@en'],
  );
  assert.throws(
    () => shaclEvalNodeExpr(SHAPES, DATA, "_:nosuch", "http://example.org/ns#a"),
    (error) =>
      error.message ===
      "the shapes graph mentions no blank node _:nosuch, so there is no expression to " +
        "evaluate; name the expression node by the label the shapes document gives it, " +
        "or by its IRI",
  );
  assert.throws(
    () => shaclEvalNodeExpr(SHAPES, DATA, "_:suffix", "http://example.org/ns#a", ['focusNode="!"']),
    (error) =>
      error.message ===
      'scope variable "focusNode" can never be read: SHACL 1.2 Node Expressions §4.1.2 ' +
        'resolves shnex:var "focusNode" to the focus node before the scope is searched. ' +
        "Pass the node as the focus node instead",
  );
});

test("shaclLintShapes certifies the declaration-bearing graph clean and reports a malformed one", () => {
  const clean = shaclLintShapes(SHAPES);
  assert.equal(clean.clean, true);
  assert.equal(clean.findings, 0);
  assert.equal(clean.loadError, undefined);
  assert.ok(
    clean.report.includes(
      "call native <http://www.w3.org/ns/shacl#SPARQLExprExpression> in sh:rule on <http://example.org/ns#Tagger>\n",
    ),
    clean.report,
  );
  clean.free();

  const malformed = shaclLintShapes(
    PREFIXES + SNIPPET + 'ex:S a sh:NodeShape ; sh:property [ sh:path ex:p ; sh:minCount "one" ] .\n',
  );
  assert.equal(malformed.clean, false);
  assert.ok(malformed.findings >= 2, malformed.report);
  assert.equal(typeof malformed.loadError, "string");
  assert.ok(malformed.report.endsWith("clean false\n"));
  malformed.free();

  assert.throws(() => shaclLintShapes("@@@ not turtle"));
});
