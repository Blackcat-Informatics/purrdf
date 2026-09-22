# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Proving the pre-flight extension-usage report on the Python surface.

A host wires a relation, writes a shapes graph that names it, validates, and gets
``conforms: True``. Did the relation run? The report cannot say. An IRI the
environment does not recognize lowers to an ordinary triple pattern, matches
whatever the data holds for that predicate -- usually nothing -- and answers,
which is also exactly what a correctly-resolved relation over no matching rows
returns. The two outcomes are indistinguishable after the fact.

``Shapes.extension_usage`` answers the question before the fact, from the parse
rather than from the run. The claim under test is that the Python surface reports
what the environment ACTUALLY makes of the graph -- the same answer the Rust side
computes -- rather than merely returning a well-shaped dictionary.

Every positive case is executed beside its neighbour, because "this IRI is a call"
is only meaningful if some IRI is not: a binding that reported every predicate as
a call would satisfy the first half alone.
"""

from __future__ import annotations

import purrdf

REL = "http://example.org/rel/flagged"
PLAIN = "http://example.org/ns#plain"

SHAPES = f"""
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/ns#> .

ex:S
    a sh:NodeShape ;
    sh:targetNode ex:a ;
    sh:sparql [
        a sh:SPARQLConstraint ;
        sh:select "SELECT $this ?v WHERE {{ $this <{REL}> ?v . $this <{PLAIN}> ?w }}" ;
    ] .
"""


def test_a_declared_iri_is_a_call_and_an_undeclared_one_is_data() -> None:
    """The same text, two declarations, two answers -- which is the whole point.

    The fixture's two predicates differ in exactly one respect: one is declared and
    one is not. So the split between ``calls`` and ``data`` cannot come from the
    shapes graph, the site, or the ordering -- only from the declaration.
    """
    shapes = purrdf.shapes.Shapes(SHAPES)

    declared = shapes.extension_usage(relation_iris=[REL])
    assert declared["complete"] is True
    site, used = next(iter(declared["sites"].items()))
    assert "sh:sparql" in site
    assert used["calls"] == [REL], f"the declared IRI is a call: {used}"
    assert PLAIN in used["data"], f"the undeclared sibling stays data: {used}"

    # The neighbour: declare nothing, and the very same predicate is ordinary data.
    bare = shapes.extension_usage()
    _, bare_used = next(iter(bare["sites"].items()))
    assert bare_used["calls"] == [], f"nothing declared, so nothing is a call: {bare_used}"
    assert REL in bare_used["data"], f"it is reported, as data: {bare_used}"


def test_a_declared_namespace_reaches_the_same_answer_as_an_exact_iri() -> None:
    """A whole-prefix declaration is the other half of the seam, and it is reported."""
    shapes = purrdf.shapes.Shapes(SHAPES)

    by_namespace = shapes.extension_usage(relation_namespaces=["http://example.org/rel/"])
    _, used = next(iter(by_namespace["sites"].items()))
    assert used["calls"] == [REL]

    # The neighbour: a namespace that does NOT cover the IRI leaves it as data. This
    # is the prefix-vs-exact trap -- a declaration is a prefix match, not a guess.
    elsewhere = shapes.extension_usage(relation_namespaces=["http://example.org/other/"])
    _, used_elsewhere = next(iter(elsewhere["sites"].items()))
    assert used_elsewhere["calls"] == []
    assert REL in used_elsewhere["data"]


def test_a_site_the_declarations_cannot_read_is_reported_rather_than_dropped() -> None:
    """An unreadable site is named, and ``complete`` says so.

    The parser refuses a property-function call in a CONSTRUCT template -- a template
    writes triples, it does not read them -- so a rule whose template names a declared
    relation IRI loads fine and then cannot be read under the declaration. Dropping it
    would make the report say the shape carries no SPARQL at all, which is
    indistinguishable from a shape that genuinely carries none.
    """
    rule_shapes = purrdf.shapes.Shapes(f"""
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/ns#> .

ex:R
    a sh:NodeShape ;
    sh:targetNode ex:a ;
    sh:rule [
        a sh:SPARQLRule ;
        sh:construct "CONSTRUCT {{ $this <{REL}> ?w }} WHERE {{ $this <{PLAIN}> ?w }}" ;
    ] .
""")

    unreadable = rule_shapes.extension_usage(relation_iris=[REL])
    assert unreadable["complete"] is False
    assert len(unreadable["unreadable"]) == 1
    site, why = next(iter(unreadable["unreadable"].items()))
    assert "sh:rule" in site
    assert why, "the parser's own reason is carried, not a bare flag"

    # The neighbour: with nothing declared the identical graph reads completely.
    readable = rule_shapes.extension_usage()
    assert readable["complete"] is True, readable["unreadable"]
