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
