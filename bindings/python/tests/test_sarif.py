# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""SARIF 2.1.0 surface tests for the SHACL validation report binding.

Validates that ``ValidationReport.to_sarif()`` emits schema-valid, deterministic
SARIF against the vendored OASIS SARIF 2.1.0 JSON Schema.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

import purrdf

_SCHEMA_PATH = Path(__file__).parent / "data" / "sarif-schema-2.1.0.json"

# A shape requiring ex:age to be an xsd:integer; the data violates it.
_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .
"""

_DATA = (
    "<http://example.org/alice> "
    "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
    "<http://example.org/Person> .\n"
    "<http://example.org/alice> <http://example.org/age> \"not-an-int\" .\n"
)


def _report():
    return purrdf.shapes.Shapes(_SHAPES).validate_nt(_DATA)


def test_to_sarif_validates_against_sarif_schema() -> None:
    jsonschema = pytest.importorskip("jsonschema")
    report = _report()
    sarif = json.loads(report.to_sarif())
    schema = json.loads(_SCHEMA_PATH.read_text(encoding="utf-8"))
    # Raises jsonschema.ValidationError if the output is not schema-valid.
    jsonschema.validate(sarif, schema)
    assert sarif["version"] == "2.1.0"
    assert sarif["runs"][0]["tool"]["driver"]["name"] == "purrdf"


def test_to_sarif_reports_the_violation() -> None:
    report = _report()
    assert not report.conforms
    sarif = json.loads(report.to_sarif())
    results = sarif["runs"][0]["results"]
    assert results, "a datatype violation must produce at least one SARIF result"
    assert results[0]["level"] == "error"
    # The constraint component drives the ruleId.
    assert "ConstraintComponent" in results[0]["ruleId"]


def test_to_sarif_is_byte_deterministic() -> None:
    report = _report()
    assert report.to_sarif() == report.to_sarif()


# A property shape (a shapes-graph BLANK node) whose value is a data-graph BLANK node:
# both documents label their first blank node alike, and the report must not fuse them.
_BLANK_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/ns#> .
ex:S sh:targetNode ex:s ; sh:property [ sh:path ex:p ; sh:nodeKind sh:IRI ] .
"""

_BLANK_DATA = (
    "<http://example.org/ns#s> <http://example.org/ns#p> _:b0 .\n"
    '_:b0 <http://example.org/ns#q> "1" .\n'
)


def test_a_shapes_blank_and_a_data_blank_are_two_nodes_in_the_report() -> None:
    report = purrdf.shapes.Shapes(_BLANK_SHAPES).validate_nt(_BLANK_DATA)
    assert not report.conforms
    lines = report.to_ntriples().splitlines()
    shape = [ln for ln in lines if "<http://www.w3.org/ns/shacl#sourceShape>" in ln]
    value = [ln for ln in lines if "<http://www.w3.org/ns/shacl#value>" in ln]
    assert len(shape) == 1 and len(value) == 1
    shape_node = shape[0].split()[2]
    value_node = value[0].split()[2]
    assert shape_node.startswith("_:") and value_node.startswith("_:")
    assert shape_node != value_node, report.to_ntriples()

    sarif = json.loads(report.to_sarif())
    result = sarif["runs"][0]["results"][0]
    related = [
        loc["name"]
        for location in result["relatedLocations"]
        for loc in location["logicalLocations"]
        if loc.get("kind") == "sourceShape"
    ]
    assert related == [shape_node]
    assert value_node in result["message"]["text"]
    assert shape_node in result["message"]["text"]
