# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""SHACL-SPARQL result annotations in ``purrdf.shapes.validate`` result dicts.

SHACL 1.2 SPARQL Extensions, "Annotation Properties": the processor "copies the
binding for the given variable as a value for the property specified using
sh:annotationProperty into the validation result". The annotated constraint's
result carries ``"annotations"``; the control row, the same constraint without the
annotation, carries no such key, so an annotation dropped at the boundary fails.
"""

from __future__ import annotations

import purrdf

_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .

ex:S a sh:NodeShape ;
    sh:targetNode ex:a ;
    sh:sparql [ {annotation}
        sh:select "SELECT $this ?value ?tag WHERE {{ $this <http://example.org/p> ?value . ?value <http://example.org/tag> ?tag }}" ] .
"""

_DATA = (
    "<http://example.org/a> <http://example.org/p> <http://example.org/b> .\n"
    '<http://example.org/b> <http://example.org/tag> "urgent"@en .\n'
)


def _results(annotation: str) -> list[dict[str, object]]:
    report = purrdf.shapes.validate(_SHAPES.format(annotation=annotation), _DATA)
    results = report["results"]
    assert isinstance(results, list)
    return results


def test_py_validate_result_annotations() -> None:
    annotated = _results(
        'sh:resultAnnotation [ sh:annotationProperty ex:label ; sh:annotationVarName "tag" ] ;'
    )
    control = _results("")
    assert len(annotated) == 1
    assert annotated[0]["annotations"] == [("http://example.org/label", '"urgent"@en')]
    assert len(control) == 1
    assert "annotations" not in control[0]
