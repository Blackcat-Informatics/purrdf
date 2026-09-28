# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Every report states ``sh:shapesGraphWellFormed`` (SHACL 1.2 Core section 6.7.1.4).

It is ``True`` for every shapes graph a validation runs over, an empty ``sh:in`` list
included: ``in-minListLength`` ("Each such list SHOULD have at least one member") is a
mandatory diagnostic that ``lint_shapes`` always reports by rule id, not an
ill-formedness. The empty row is observed to be validated (the focus node violates the
empty list) and its non-empty neighbour to conform, and only the empty row carries the
lint diagnostic.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

import purrdf

_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:in {members} .
"""

_DATA = "<http://example.org/a> <http://example.org/p> <http://example.org/b> .\n"
_TRUE = '<http://www.w3.org/ns/shacl#shapesGraphWellFormed> "true"^^<http://www.w3.org/2001/XMLSchema#boolean>'


def test_py_reports_state_shapes_graph_well_formed() -> None:
    for members, conforms in (("( <http://example.org/a> )", True), ("( )", False)):
        shapes = _SHAPES.replace("{members}", members)
        as_dict = purrdf.shapes.validate(shapes, _DATA)
        assert as_dict["shapes_graph_well_formed"] is True
        report = purrdf.shapes.Shapes(shapes).validate_nt(_DATA)
        assert report.shapes_graph_well_formed is True
        assert report.conforms is conforms
        assert _TRUE in report.to_ntriples()


def test_py_lint_reports_an_empty_in_list_by_rule_id() -> None:
    empty = purrdf.shapes.lint_shapes(_SHAPES.replace("{members}", "( )"))
    assert empty["diagnostics"] == [{"rule": "in-minListLength", "shape": "<http://example.org/S>"}]
    assert "diagnostics 1\ndiagnostic in-minListLength <http://example.org/S>\n" in empty["report"]
    assert empty["clean"] is False
    filled = purrdf.shapes.lint_shapes(_SHAPES.replace("{members}", "( <http://example.org/a> )"))
    assert filled["diagnostics"] == []
    assert "in-minListLength" not in filled["report"]
    assert filled["findings"] + 1 == empty["findings"]


# The approved W3C test core/node/in-002: a shape with an empty sh:in list, and a rule.
_IN_002 = """@prefix ex: <http://example.com/ns#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
ex:TestShape rdf:type rdfs:Class , sh:NodeShape ; sh:in {members} ;
  sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:seen ; sh:object ex:yes ] .
"""
_IN_002_DATA = (
    "<http://example.com/ns#Instance> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
    "<http://example.com/ns#TestShape> .\n"
)
_SCHEMA_PATH = Path(__file__).parent / "data" / "sarif-schema-2.1.0.json"


def test_py_every_run_reports_the_mandatory_diagnostic() -> None:
    """Maintainer decision, "Every run reports it": validation (dict, report object and
    SARIF), rules and entailment all state the in-minListLength diagnostic, beside the
    verdict and the results — which are the specification's — and never among them. The
    neighbour whose list has a member states none, so the diagnostic is about the list."""
    expected = [{"rule": "in-minListLength", "shape": "<http://example.com/ns#TestShape>"}]
    for members, diagnostics, conforms in (
        ("()", expected, False),
        ("( <http://example.com/ns#Instance> )", [], True),
    ):
        shapes = _IN_002.replace("{members}", members)
        as_dict = purrdf.shapes.validate(shapes, _IN_002_DATA)
        assert as_dict["diagnostics"] == diagnostics
        assert as_dict["conforms"] is conforms
        assert len(as_dict["results"]) == (0 if conforms else 1)
        report = purrdf.shapes.Shapes(shapes).validate_nt(_IN_002_DATA)
        assert report.diagnostics == diagnostics
        assert report.conforms is conforms
        assert "minListLength" not in report.to_ntriples()
        sarif = json.loads(report.to_sarif())
        notifications = [
            notification["descriptor"]["id"]
            for invocation in sarif["runs"][0].get("invocations", [])
            for notification in invocation.get("toolExecutionNotifications", [])
        ]
        assert notifications == [d["rule"] for d in diagnostics]
        jsonschema = pytest.importorskip("jsonschema")
        jsonschema.validate(sarif, json.loads(_SCHEMA_PATH.read_text(encoding="utf-8")))

        rules = purrdf.shapes.apply_rules(_IN_002_DATA, shapes)
        assert rules["diagnostics"] == diagnostics
        entailed = purrdf.shapes.entail(shapes, _IN_002_DATA)
        assert entailed["diagnostics"] == diagnostics
        assert "<http://example.com/ns#seen>" in entailed["ntriples"]
