# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""The ShEx surface on the native binding: ``purrdf.shex``.

Two entry points, and between them the whole of what a host can ask this layer:
``validate`` runs a **fixed shape map** — a caller-supplied list of
``(node, shape)`` associations — against an RDF document and reports one verdict
per association, and ``parse`` reads a schema in either syntax and answers with
its canonical ShExJ text.

What these tests hold:

* **A verdict is per association, in input order, and carries its own reason.**
  There is no aggregate pass/fail: a map with a conforming node and a
  non-conforming one answers with both, the conformant entry carrying no reason
  and the nonconformant one carrying the deepest failure that produced it.
* **A focus node is spelled the way RDF spells it.** An IRI bare or
  ``<…>``-wrapped, a ``_:``-prefixed blank node, and every Turtle literal token —
  plain, language-tagged, datatype-tagged — decode to the term they name, and
  each decodes to a *different* term, which is what makes the verdict about the
  node the caller meant. A malformed literal token is a ``ValueError`` where it
  is written.
* **``"START"`` is a selector, not a shape label**, and a schema with no start
  shape says so in the reason rather than raising.
* **Canonical ShExJ is canonical.** The text ``parse`` emits reparses as
  ``"shexj"`` to the same bytes, which is what makes it usable as a schema
  interchange form at all.
* **Nothing is defaulted and nothing is guessed.** An unknown schema format and
  an unknown data format are each refused by name, and every accepted spelling is
  executed beside the refusal — an over-refusal here would be invisible until a
  host wrote the call that should work.
"""

from __future__ import annotations

import pytest

from purrdf import shex

EX = "https://example.org/"
XSD = "http://www.w3.org/2001/XMLSchema#"

#: One shape requiring exactly one ``ex:p`` arc. Everything the arc-based cases
#: below assert is "does this focus node have that arc in this document".
ARC_SCHEMA = f"PREFIX ex: <{EX}> ex:S {{ ex:p . }}"

#: The document the arc cases run against: one IRI-subject row and one
#: blank-node-subject row, so an IRI focus node and a blank focus node are each
#: really present and each really has the arc.
ARC_DATA = (
    f"<{EX}n> <{EX}p> 1 .\n"
    f"_:b0 <{EX}p> 2 .\n"
)

#: Three value-set shapes, one per literal axis. A value set is how a shape map
#: can be made to depend on a LITERAL focus node's exact term identity: a node
#: conforms only if the decoded term is in the set, so the language tag and the
#: datatype are load-bearing rather than incidental.
LITERAL_SCHEMA = (
    f"PREFIX ex: <{EX}>\n"
    f"PREFIX xsd: <{XSD}>\n"
    'ex:Plain ["chat"]\n'
    'ex:French ["chat"@fr]\n'
    'ex:Integer ["1"^^xsd:integer]\n'
)

#: The literal cases need a well-formed document, but not one that mentions the
#: focus nodes: a value set is a constraint on the node itself.
LITERAL_DATA = f"<{EX}s> <{EX}p> 1 .\n"

SHAPE = f"{EX}S"


def _verdict(schema: str, data: str, node: str, shape: str) -> dict[str, object]:
    """The single verdict for a one-association shape map."""
    entries = shex.validate(schema, data, [(node, shape)])
    assert len(entries) == 1, entries
    return entries[0]


def test_a_fixed_shape_map_reports_one_verdict_per_association() -> None:
    """Two associations, two verdicts, in input order, each with its own reason.

    The conformant entry carries no reason and the nonconformant one does, which
    is the asymmetry a host reads: a reason is the diagnosis of a failure, and a
    reason attached to a pass would be a failure wearing a verdict that denies it.
    """
    entries = shex.validate(
        ARC_SCHEMA,
        ARC_DATA,
        [(f"{EX}n", SHAPE), (f"<{EX}absent>", SHAPE)],
    )

    assert [entry["node"] for entry in entries] == [f"{EX}n", f"<{EX}absent>"], (
        "the verdicts come back in the order the map was written, echoing the "
        "node spelling the caller used"
    )
    assert entries[0]["conformant"] is True, "the node with ex:p conforms"
    assert entries[0]["reason"] is None, "a conformant entry has nothing to explain"
    assert entries[1]["conformant"] is False, "the node without ex:p does not"
    reason = entries[1]["reason"]
    assert isinstance(reason, str) and reason, (
        "a nonconformant entry carries the deepest failure that produced it"
    )
    assert f"{EX}p" in reason, f"and the reason names the arc that was missing: {reason}"


def test_the_start_selector_without_a_start_shape_is_nonconformant_with_a_reason() -> None:
    """``"START"`` against a schema that declares none is a verdict, not a crash.

    It is a perfectly well-formed question — "does this node conform to whatever
    this schema starts at?" — asked of a schema that starts at nothing. The honest
    answer is that it does not conform, and why; raising would make an
    interoperable shape map unusable against half the schemas it may be pointed
    at, and answering `True` would certify against nothing at all.
    """
    verdict = _verdict(ARC_SCHEMA, ARC_DATA, f"{EX}n", "START")
    assert verdict["conformant"] is False
    reason = verdict["reason"]
    assert isinstance(reason, str) and "start" in reason, (
        f"the reason names the missing start shape: {reason!r}"
    )

    # The neighbouring valid case: the very same node against the shape the
    # schema DOES declare conforms, so the verdict above is about the selector
    # and not about the node or the document.
    assert _verdict(ARC_SCHEMA, ARC_DATA, f"{EX}n", SHAPE)["conformant"] is True


def test_every_node_spelling_decodes_to_the_term_it_names() -> None:
    """The five focus-node spellings, each proved by the verdict it produces.

    A decode is only visible through its consequence, so each spelling is paired
    with a shape that conforms for the term it should decode to and fails for the
    others. The literal axes are the ones that would rot silently: dropping a
    language tag or a datatype during the decode turns ``"chat"@fr`` into
    ``"chat"``, a different RDF term, and the verdict would still be a verdict.
    """
    # An IRI, bare and wrapped, is the same term — and it is the term in the
    # document, not a neighbouring one.
    assert _verdict(ARC_SCHEMA, ARC_DATA, f"{EX}n", SHAPE)["conformant"] is True
    assert _verdict(ARC_SCHEMA, ARC_DATA, f"<{EX}n>", SHAPE)["conformant"] is True
    assert _verdict(ARC_SCHEMA, ARC_DATA, f"<{EX}absent>", SHAPE)["conformant"] is False

    # A `_:`-prefixed label is a blank node, and it is the document's blank node.
    assert _verdict(ARC_SCHEMA, ARC_DATA, "_:b0", SHAPE)["conformant"] is True
    assert _verdict(ARC_SCHEMA, ARC_DATA, "_:absent", SHAPE)["conformant"] is False

    # Every literal axis, each conforming to exactly its own value set. The
    # off-diagonal cases are the whole point: they are what fails if the decode
    # loses the tag or the datatype.
    matrix = {
        '"chat"': f"{EX}Plain",
        '"chat"@fr': f"{EX}French",
        f'"1"^^<{XSD}integer>': f"{EX}Integer",
    }
    for node, own_shape in matrix.items():
        for shape in matrix.values():
            verdict = _verdict(LITERAL_SCHEMA, LITERAL_DATA, node, shape)
            assert verdict["conformant"] is (shape == own_shape), (
                f"{node} against {shape}: a literal focus node conforms to the "
                "value set holding its own term and to no other"
            )

    # A language subtag is part of the term too, so a tag the set does not hold
    # is a different node.
    assert (
        _verdict(LITERAL_SCHEMA, LITERAL_DATA, '"chat"@en', f"{EX}French")["conformant"]
        is False
    )


def test_a_malformed_literal_node_is_refused_and_the_well_formed_ones_are_not() -> None:
    """An unterminated literal token is a typed error naming the token.

    It is refused rather than read as far as it parses, because a truncated
    lexical form is a different term and would produce a confident verdict about
    a node the caller never wrote. The neighbouring well-formed spellings are
    executed here too: refusing them as well would be the mirror failure, and it
    would look exactly like correct strictness.
    """
    with pytest.raises(ValueError, match="invalid literal node") as refused:
        shex.validate(LITERAL_SCHEMA, LITERAL_DATA, [('"unterminated', f"{EX}Plain")])
    assert "unterminated" in str(refused.value), (
        "the refusal quotes the token it could not read"
    )

    for node in ('"chat"', '"chat"@fr', f'"1"^^<{XSD}integer>'):
        assert isinstance(
            _verdict(LITERAL_SCHEMA, LITERAL_DATA, node, f"{EX}Plain")["conformant"],
            bool,
        ), f"{node} is a well-formed literal token and still decodes"


def test_a_schema_round_trips_through_canonical_shexj() -> None:
    """``parse`` emits ShExJ, and reparsing that ShExJ emits the same bytes.

    Byte-stability is what makes the output a schema interchange form rather than
    a rendering: a consumer can store it, diff it, or digest it, and a producer
    that reparsed and re-emitted has changed nothing.
    """
    shexj = shex.parse(ARC_SCHEMA)
    assert shexj.strip().startswith("{"), shexj[:80]
    assert shex.parse(shexj, format="shexj") == shexj, (
        "the ShExJ output is canonical: reparsing it reproduces it exactly"
    )

    # And it is the same schema, not merely the same text: the round-tripped
    # form validates the same document to the same verdicts.
    assert (
        shex.validate(shexj, ARC_DATA, [(f"{EX}n", SHAPE), (f"<{EX}absent>", SHAPE)],
                      schema_format="shexj")
        == shex.validate(ARC_SCHEMA, ARC_DATA, [(f"{EX}n", SHAPE), (f"<{EX}absent>", SHAPE)])
    )


def test_an_unknown_schema_format_is_refused_and_both_known_ones_answer() -> None:
    """Two syntaxes, named; a third spelling is refused naming the two."""
    with pytest.raises(ValueError, match="unknown schema format") as refused:
        shex.parse(ARC_SCHEMA, format="shexk")
    message = str(refused.value)
    assert '"shexc"' in message and '"shexj"' in message, (
        f"the refusal names both accepted spellings: {message}"
    )

    # The neighbouring valid cases: both accepted spellings answer.
    shexj = shex.parse(ARC_SCHEMA, format="shexc")
    assert shex.parse(shexj, format="shexj")

    # A malformed schema in an accepted syntax is a typed error, not a panic and
    # not an empty schema that conforms everything.
    with pytest.raises(ValueError):
        shex.parse("ex:S {")


def test_each_data_format_name_routes_the_document_and_an_unknown_one_is_refused() -> None:
    """Three document syntaxes, named; a fourth spelling is refused naming them.

    The routing is proved per format by syntax only that format admits: the
    Turtle document uses a prefixed name and a bare integer that the line-based
    syntaxes reject, and the N-Quads row carries a graph name that neither of the
    others can express. A name that routed to the wrong codec would fail to parse
    its own document.
    """
    turtle = f"PREFIX ex: <{EX}> ex:n ex:p 1 .\n"
    ntriples = f'<{EX}n> <{EX}p> "v" .\n'
    nquads = f'<{EX}n> <{EX}p> "v" <{EX}g> .\n'

    for data, data_format in ((turtle, "turtle"), (ntriples, "ntriples"), (nquads, "nquads")):
        entries = shex.validate(
            ARC_SCHEMA, data, [(f"{EX}n", SHAPE)], data_format=data_format
        )
        assert entries[0]["conformant"] is True, (
            f"{data_format}: the document routed to the codec that can read it"
        )

    # …and a Turtle-only document really is rejected by the line-based codecs, so
    # the three cases above are routing and not three readings of one grammar.
    with pytest.raises(ValueError):
        shex.validate(ARC_SCHEMA, turtle, [(f"{EX}n", SHAPE)], data_format="ntriples")

    with pytest.raises(ValueError, match="unknown data format") as refused:
        shex.validate(ARC_SCHEMA, ntriples, [(f"{EX}n", SHAPE)], data_format="trix")
    message = str(refused.value)
    for accepted in ('"turtle"', '"ntriples"', '"nquads"'):
        assert accepted in message, f"the refusal names {accepted}: {message}"
