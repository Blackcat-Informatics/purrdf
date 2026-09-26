# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""The ``conformance_disallows=`` keyword of ``purrdf.shapes.validate``.

SHACL 1.2 Core judges conformance against "the set of disallowed severity
levels", defaulting to ``sh:Violation``, ``sh:Warning`` and ``sh:Info``. The
keyword names that set; every treatment row below has a control row that answers
differently, so a keyword that was silently ignored would fail.
"""

from __future__ import annotations

import pytest

import purrdf

_SH = "http://www.w3.org/ns/shacl#"

# `ex:age` must be an xsd:integer, graded `severity` rather than sh:Violation.
_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ; sh:severity sh:{severity} ] .
"""

_DATA = (
    "<http://example.org/alice> "
    "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
    "<http://example.org/Person> .\n"
    '<http://example.org/alice> <http://example.org/age> "not-an-int" .\n'
)


def _validate(severity: str, **kwargs: object) -> dict[str, object]:
    return purrdf.shapes.validate(_SHAPES.replace("{severity}", severity), _DATA, **kwargs)


def test_py_validate_conformance_disallows() -> None:
    default = _validate("Warning")
    assert default["conforms"] is False
    assert default["conformance_disallows"] == [
        _SH + "Violation",
        _SH + "Warning",
        _SH + "Info",
    ]

    relaxed = _validate("Warning", conformance_disallows=[_SH + "Violation"])
    assert relaxed["conforms"] is True
    assert relaxed["conformance_disallows"] == [_SH + "Violation"]
    results = relaxed["results"]
    assert isinstance(results, list) and len(results) == 1
    assert results[0]["severity"] == _SH + "Warning"


def test_py_debug_results_conform_by_default() -> None:
    debug = _validate("Debug")
    assert debug["conforms"] is True
    results = debug["results"]
    assert isinstance(results, list) and results[0]["severity"] == _SH + "Debug"
    assert _validate("Debug", conformance_disallows=[_SH + "Debug"])["conforms"] is False


def test_py_an_empty_or_non_iri_set_is_refused() -> None:
    with pytest.raises(ValueError, match="empty"):
        _validate("Warning", conformance_disallows=[])
    with pytest.raises(ValueError, match="not an absolute IRI"):
        _validate("Warning", conformance_disallows=["Violation"])


def test_py_every_message_is_kept_with_its_language() -> None:
    shapes = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ;
                  sh:message "Too many"@en , "Zu viele"@de ] .
"""
    results = purrdf.shapes.validate(shapes, _DATA)["results"]
    assert isinstance(results, list) and len(results) == 1
    assert results[0]["messages"] == [
        {"text": "Too many", "language": "en"},
        {"text": "Zu viele", "language": "de"},
    ]
    plain = purrdf.shapes.validate(shapes.replace('"Too many"@en , "Zu viele"@de', '"one"'), _DATA)
    assert plain["results"][0]["messages"] == [{"text": "one"}]


# The W3C SHACL 1.2 vocabulary's declaration of the built-in
# ``sh:SPARQLExprExpression``, verbatim: a ``sh:NamedParameterExpressionFunction``
# with the two ``sh:Parameter``s ``-prefixes`` and ``-sparqlExpr`` and no
# ``sh:bodyExpression``.
_SPARQL_EXPR_DECLARATION = '''@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

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

# A Warning-graded property shape whose one value node is computed by
# ``sh:sparqlExpr "ex:active"`` through ``sh:prefixes``; ``sh:in ( ex:retired )``
# refuses it at every ``ex:Person``.
_SPARQL_EXPR_SHAPES = '''
@prefix ex: <http://example.org/> .
ex:Prefixes sh:declare [ sh:prefix "ex" ; sh:namespace "http://example.org/"^^xsd:anyURI ] .
ex:PersonShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [
    sh:path ex:status ;
    sh:values [ sh:sparqlExpr "ex:active" ; sh:prefixes ex:Prefixes ] ;
    sh:in ( ex:retired ) ;
    sh:severity sh:Warning
  ] .
'''


def test_py_validate_evaluates_sparql_expr_beside_its_vocabulary_declaration() -> None:
    """The issue's reproducer: the declaration loads, and the ``sh:sparqlExpr`` +
    ``sh:prefixes`` expression is evaluated natively -- the result's value is the
    computed ``<http://example.org/active>``, which only the prefix-expanded
    expression yields. The Warning result does not conform under the default set
    and conforms under ``sh:Violation`` alone."""
    shapes = _SPARQL_EXPR_DECLARATION + _SPARQL_EXPR_SHAPES
    default = purrdf.shapes.validate(shapes, _DATA)
    assert default["conforms"] is False
    results = default["results"]
    assert isinstance(results, list) and len(results) == 1
    assert results[0]["value"] == "<http://example.org/active>"
    assert results[0]["severity"] == _SH + "Warning"

    relaxed = purrdf.shapes.validate(shapes, _DATA, conformance_disallows=[_SH + "Violation"])
    assert relaxed["conforms"] is True
    assert relaxed["results"][0]["value"] == "<http://example.org/active>"
