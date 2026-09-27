# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Every report states ``sh:shapesGraphWellFormed`` (SHACL 1.2 Core section 6.7.1.4).

``True`` for a shapes graph that met every syntax rule; ``False`` for an empty
``sh:in`` list, which the approved W3C tests require validating although
``in-minListLength`` says the list SHOULD have a member. The neighbour row differs
only in the list having a member.
"""

from __future__ import annotations

import purrdf

_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:in {members} .
"""

_DATA = "<http://example.org/a> <http://example.org/p> <http://example.org/b> .\n"
_TRIPLE = "<http://www.w3.org/ns/shacl#shapesGraphWellFormed> \"{value}\"^^<http://www.w3.org/2001/XMLSchema#boolean>"


def test_py_reports_state_shapes_graph_well_formed() -> None:
    for members, expected in (("( <http://example.org/a> )", True), ("( )", False)):
        shapes = _SHAPES.replace("{members}", members)
        as_dict = purrdf.shapes.validate(shapes, _DATA)
        assert as_dict["shapes_graph_well_formed"] is expected
        report = purrdf.shapes.Shapes(shapes).validate_nt(_DATA)
        assert report.shapes_graph_well_formed is expected
        assert _TRIPLE.replace("{value}", "true" if expected else "false") in report.to_ntriples()
