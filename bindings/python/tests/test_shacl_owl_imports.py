# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""One shapes graph, one ``owl:imports`` verdict, on every Python entry point.

A shapes graph that ``owl:imports <http://example.org/lib>`` is driven through every
shapes-graph function of ``purrdf.shapes``. Without an ``imports`` table each raises the
same typed ``ShapesImportError`` (kind ``unresolved-import``, naming the IRI) -- the refusal
the Rust API, the command line, WebAssembly and C raise for the same graph. With the
imported document in ``imports`` each APPLIES it, observed through a result the importing
document alone cannot produce.
"""

from __future__ import annotations

import pytest

import purrdf

LIB = "http://example.org/lib"

# The importing shapes graph: an ontology header and its import, no shape of its own.
IMPORTER = """@prefix owl: <http://www.w3.org/2002/07/owl#> .
<http://example.org/shapes> a owl:Ontology ;
  owl:imports <http://example.org/lib> .
"""

# The imported document: a shape needing ex:name, a rule tagging every ex:Person
# ex:checked ex:yes, and a node expression ex:Who reading the scope variable `who`.
IMPORTED = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix ex: <http://example.org/> .
ex:NameShape a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [ sh:path ex:name ; sh:minCount 1 ] ;
  sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:checked ;
            sh:object ex:yes ] .
ex:Who shnex:var "who" .
"""

# One ex:Person with no ex:name.
PERSON = (
    "<http://example.org/alice> "
    "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n"
)

TABLE = [(LIB, IMPORTED)]

MIN_COUNT = "http://www.w3.org/ns/shacl#MinCountConstraintComponent"
INFERRED = "<http://example.org/alice> <http://example.org/checked> <http://example.org/yes> ."

_ENTRY_POINTS = {
    "validate": lambda imports: purrdf.shapes.validate(IMPORTER, PERSON, imports=imports),
    "entail": lambda imports: purrdf.shapes.entail(IMPORTER, PERSON, imports=imports),
    "apply_rules": lambda imports: purrdf.shapes.apply_rules(
        PERSON, IMPORTER, imports=imports
    ),
    "eval_node_expr": lambda imports: purrdf.shapes.eval_node_expr(
        IMPORTER,
        PERSON,
        "http://example.org/Who",
        "http://example.org/alice",
        scope={"who": "http://example.org/bob"},
        imports=imports,
    ),
    "lint_shapes": lambda imports: purrdf.shapes.lint_shapes(IMPORTER, imports=imports),
    "pack_product": lambda imports: purrdf.shapes.pack_product(IMPORTER, imports=imports),
    "Shapes": lambda imports: purrdf.shapes.Shapes(IMPORTER, imports=imports),
}


@pytest.mark.parametrize("entry", sorted(_ENTRY_POINTS))
def test_every_entry_point_refuses_an_unresolved_import_with_the_typed_error(entry: str) -> None:
    with pytest.raises(purrdf.shapes.ShapesImportError) as raised:
        _ENTRY_POINTS[entry](())
    assert raised.value.kind == "unresolved-import"
    assert raised.value.iris == [LIB]
    # It is still a ValueError, so code catching the SHACL surface's ValueError keeps working.
    assert isinstance(raised.value, ValueError)


def test_every_entry_point_applies_a_supplied_import() -> None:
    report = _ENTRY_POINTS["validate"](TABLE)
    assert report["conforms"] is False
    assert [result["component"] for result in report["results"]] == [MIN_COUNT]

    assert INFERRED in _ENTRY_POINTS["entail"](TABLE)
    assert _ENTRY_POINTS["apply_rules"](TABLE)["inferred"].strip() == INFERRED
    assert _ENTRY_POINTS["eval_node_expr"](TABLE) == ["<http://example.org/bob>"]
    assert _ENTRY_POINTS["lint_shapes"](TABLE)["clean"] is True

    shapes = _ENTRY_POINTS["Shapes"](TABLE)
    assert [r["component"] for r in shapes.validate_nt(PERSON).results] == [MIN_COUNT]

    product = _ENTRY_POINTS["pack_product"](TABLE)
    prepared = purrdf.shapes.ShapesProduct.open(product).admit()
    restored = prepared.validate_nt(PERSON)
    assert [r["component"] for r in restored.results] == [MIN_COUNT]


def test_an_ontology_declared_in_place_resolves_and_its_neighbour_does_not() -> None:
    merged = IMPORTER + IMPORTED
    for declaration in (
        "<http://example.org/lib> a <http://www.w3.org/2002/07/owl#Ontology> .\n",
        "<http://example.org/lib-series> "
        "<http://www.w3.org/2002/07/owl#versionIRI> <http://example.org/lib> .\n",
    ):
        report = purrdf.shapes.validate(merged + declaration, PERSON)
        assert [r["component"] for r in report["results"]] == [MIN_COUNT], declaration
    with pytest.raises(purrdf.shapes.ShapesImportError) as raised:
        purrdf.shapes.validate(merged, PERSON)
    assert raised.value.kind == "unresolved-import"


def test_the_shapes_base_resolves_a_self_import_and_another_base_does_not() -> None:
    merged = IMPORTER + IMPORTED
    report = purrdf.shapes.validate(merged, PERSON, shapes_base=LIB)
    assert [r["component"] for r in report["results"]] == [MIN_COUNT]
    with pytest.raises(purrdf.shapes.ShapesImportError):
        purrdf.shapes.validate(merged, PERSON, shapes_base="http://example.org/elsewhere")


def test_a_table_entry_nothing_imports_is_refused_and_a_reached_one_is_not() -> None:
    plain = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n<http://example.org/S> a sh:NodeShape .\n"
    with pytest.raises(purrdf.shapes.ShapesImportError) as raised:
        purrdf.shapes.validate(plain, PERSON, imports=TABLE)
    assert raised.value.kind == "unreached-import"
    assert raised.value.iris == [LIB]
    purrdf.shapes.validate(IMPORTER, PERSON, imports=TABLE)


def test_a_relative_key_is_an_invalid_entry_and_an_absolute_one_is_not() -> None:
    with pytest.raises(purrdf.shapes.ShapesImportError) as raised:
        purrdf.shapes.validate(IMPORTER, PERSON, imports=[("lib", IMPORTED)])
    assert raised.value.kind == "invalid-import"
    purrdf.shapes.validate(IMPORTER, PERSON, imports=TABLE)
