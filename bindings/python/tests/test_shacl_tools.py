# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""The shapes-graph tools beside validation: ``purrdf.shapes.apply_rules``,
``purrdf.shapes.eval_node_expr`` and ``purrdf.shapes.lint_shapes``.

Every shapes graph carries the W3C SHACL 1.2 declaration of
``sh:SPARQLExprExpression`` verbatim -- the declaration that used to be refused as
a bodiless custom function -- beside shapes that call ``sh:sparqlExpr`` with
``sh:prefixes``.
"""

from __future__ import annotations

import pytest

import purrdf

_SNIPPET = '''
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
'''

_PREFIXES = """
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ex: <http://example.org/ns#> .
"""

# A sh:sparqlExpr node naming ex:yes through sh:prefixes, a labelled shnex:var
# node, a rule tagging every ex:Item through the same expression, and a counter
# rule stepping ex:n to 5 -- exactly four term-generating rounds.
_TOOLS = '''
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
'''

_SHAPES = _PREFIXES + _SNIPPET + _TOOLS

_DATA = (
    "<http://example.org/ns#a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
    "<http://example.org/ns#Item> .\n"
    '<http://example.org/ns#a> <http://example.org/ns#n> "1"^^<http://www.w3.org/2001/XMLSchema#integer> .\n'
)

_INTEGER = "<http://www.w3.org/2001/XMLSchema#integer>"

_INFERRED = "".join(
    f'<http://example.org/ns#a> <http://example.org/ns#n> "{n}"^^{_INTEGER} .\n' for n in range(2, 6)
) + ("<http://example.org/ns#a> <http://example.org/ns#tagged> <http://example.org/ns#yes> .\n")


def test_py_apply_rules() -> None:
    out = purrdf.shapes.apply_rules(_DATA, _SHAPES)
    assert out == {"inferred": _INFERRED, "proof": None}

    explained = purrdf.shapes.apply_rules(_DATA, _SHAPES, explain=True)
    assert explained["inferred"] == _INFERRED
    proof = explained["proof"]
    assert isinstance(proof, str)
    assert proof.count("derived ") == 5
    assert (
        "derived <http://example.org/ns#a> <http://example.org/ns#tagged> "
        "<http://example.org/ns#yes> .\n  rule _:" in proof
    )

    # The round limit: four term-generating rounds are needed.
    with pytest.raises(ValueError, match="past the limit of 3"):
        purrdf.shapes.apply_rules(_DATA, _SHAPES, max_term_generating_rounds=3)
    assert purrdf.shapes.apply_rules(_DATA, _SHAPES, max_term_generating_rounds=4)["inferred"] == _INFERRED

    # SPARQL 1.2 RL text through the same function.
    srl = (
        "PREFIX ex: <http://example.org/ns#>\n"
        "RULE { ?x ex:q ?y } WHERE { ?x ex:n ?y }\n"
        "DATA { ex:d ex:q 2 }\n"
    )
    ran = purrdf.shapes.apply_rules(_DATA, srl=srl, explain=True)
    assert ran["inferred"] == (
        f'<http://example.org/ns#a> <http://example.org/ns#q> "1"^^{_INTEGER} .\n'
        f'<http://example.org/ns#d> <http://example.org/ns#q> "2"^^{_INTEGER} .\n'
    )
    assert "  data-block\n" in ran["proof"]
    assert f'  premise <http://example.org/ns#a> <http://example.org/ns#n> "1"^^{_INTEGER} .\n' in ran["proof"]

    with pytest.raises(ValueError, match="two rule sources"):
        purrdf.shapes.apply_rules(_DATA, _SHAPES, srl=srl)
    with pytest.raises(ValueError, match="no rule source"):
        purrdf.shapes.apply_rules(_DATA)


def test_py_eval_node_expr() -> None:
    assert purrdf.shapes.eval_node_expr(
        _SHAPES, _DATA, "http://example.org/ns#Tag", "http://example.org/ns#a"
    ) == ["<http://example.org/ns#yes>"]
    assert purrdf.shapes.eval_node_expr(
        _SHAPES,
        _DATA,
        "_:suffix",
        f'"-3"^^{_INTEGER}',
        scope={"suffix": '"!"@en'},
    ) == ['"!"@en']
    with pytest.raises(ValueError, match="mentions no blank node _:nosuch"):
        purrdf.shapes.eval_node_expr(_SHAPES, _DATA, "_:nosuch", "http://example.org/ns#a")
    with pytest.raises(ValueError, match="can never be read"):
        purrdf.shapes.eval_node_expr(
            _SHAPES, _DATA, "_:suffix", "http://example.org/ns#a", scope={"focusNode": '"!"'}
        )


def test_py_lint_shapes() -> None:
    clean = purrdf.shapes.lint_shapes(_SHAPES)
    assert clean["clean"] is True
    assert clean["findings"] == 0
    assert clean["load_error"] is None
    assert clean["shacl_shacl"] == []
    assert {
        "binding": "native",
        "function": "http://www.w3.org/ns/shacl#SPARQLExprExpression",
        "owner": "sh:rule on <http://example.org/ns#Tagger>",
    } in clean["calls"]
    assert clean["report"].endswith("findings 0\nclean true\n")

    malformed = purrdf.shapes.lint_shapes(
        _PREFIXES + _SNIPPET + 'ex:S a sh:NodeShape ; sh:property [ sh:path ex:p ; sh:minCount "one" ] .\n'
    )
    assert malformed["clean"] is False
    assert malformed["findings"] >= 2
    assert isinstance(malformed["load_error"], str)
    assert malformed["calls"] is None
    assert any(
        result["path"] == "<http://www.w3.org/ns/shacl#minCount>" and result["superseded"] is None
        for result in malformed["shacl_shacl"]
    )

    by_types = purrdf.shapes.lint_shapes(_PREFIXES + _SNIPPET + "ex:S a sh:NodeShape ; sh:closed sh:ByTypes .\n")
    assert by_types["clean"] is True
    assert [result["superseded"] for result in by_types["shacl_shacl"]] == ["closed-by-types"]

    with pytest.raises(ValueError):
        purrdf.shapes.lint_shapes("@@@ not turtle")
