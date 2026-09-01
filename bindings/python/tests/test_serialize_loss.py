# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Dumping a store to a syntax that cannot carry it reports what it left behind.

`dump` is the transcode lane, not the query lane: asking a multi-graph store for
Turtle is a legitimate "give me the default graph", so — unlike a graph-carrying
`QueryQuads`, whose graph names came from the query the caller wrote — it does not
refuse. What it must not do is drop silently, and before `dump_with_loss` existed it
had no way to say anything at all: a store holding two named graphs dumped to Turtle
came back well-formed, missing every graph-scoped statement, with no exception and no
count.

The counts partition the loss by CAUSE, so their sum is the total and no row is
charged twice. That partition is the point: N-Triples is star-capable, so its
statement-layer count reads `0` however many named graphs it just discarded — reading
that one number alone would say "nothing was lost".

The same three numbers are `purrdf_serialize`'s out-params on the C ABI and
`Dataset.serializeWithLoss`'s getters on wasm, so one serialization reports one answer
on every host.
"""

from __future__ import annotations

import purrdf

EX = "https://example.org/"

# One default-graph base quad, two base quads in two DIFFERENT named graphs, one
# RDF-1.2 reifier binding in the default graph and one scoped to a named graph.
_MIXED = (
    f"<{EX}s1> <{EX}p> <{EX}o1> .\n"
    f"<{EX}s2> <{EX}p> <{EX}o2> <{EX}g1> .\n"
    f"<{EX}s3> <{EX}p> <{EX}o3> <{EX}g2> .\n"
    f"<{EX}r1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> "
    f"<<( <{EX}s1> <{EX}p> <{EX}o1> )>> .\n"
    f"<{EX}r2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> "
    f"<<( <{EX}s2> <{EX}p> <{EX}o2> )>> <{EX}g1> .\n"
).encode()

# The three rows a single-graph target discards: two graph-scoped base quads plus the
# graph-scoped reifier row.
_GRAPH_SCOPED_ROWS = 3


def _store() -> purrdf.Store:
    store = purrdf.Store()
    store.load(_MIXED, format=purrdf.RdfFormat.N_QUADS)
    return store


def _mutable() -> purrdf.MutableDataset:
    dataset = purrdf.MutableDataset()
    dataset.load(_MIXED, format=purrdf.RdfFormat.N_QUADS)
    return dataset


def test_a_star_capable_single_graph_target_reports_its_named_graph_loss() -> None:
    """N-Triples: `statement_rows_dropped == 0` while three rows vanish."""
    loss = _store().dump_with_loss(purrdf.RdfFormat.N_TRIPLES)
    assert loss.statement_rows_dropped == 0
    assert loss.directional_literals_dropped == 0
    assert loss.named_graph_rows_dropped == _GRAPH_SCOPED_ROWS
    text = loss.bytes.decode()
    assert f"{EX}g1" not in text
    assert f"{EX}g2" not in text


def test_turtle_reports_the_same_loss_as_n_triples() -> None:
    """The cause is the flattening, not the syntax family."""
    loss = _store().dump_with_loss(purrdf.RdfFormat.TURTLE)
    assert loss.named_graph_rows_dropped == _GRAPH_SCOPED_ROWS


def test_a_dataset_capable_target_loses_nothing_and_says_so() -> None:
    """Every count is zero for N-Quads, and every graph survives into the bytes."""
    loss = _store().dump_with_loss(purrdf.RdfFormat.N_QUADS)
    assert loss.statement_rows_dropped == 0
    assert loss.directional_literals_dropped == 0
    assert loss.named_graph_rows_dropped == 0
    text = loss.bytes.decode()
    assert f"{EX}g1" in text
    assert f"{EX}g2" in text


def test_the_count_follows_the_target_capability_not_the_document() -> None:
    """One document, three targets, three different answers about the same rows.

    TriX and HexTuples carry named graphs, so the two graph-scoped base quads are not
    lost at all; N-Triples has no graph construct, so the same two rows are the whole
    of its loss. A count that did not distinguish the targets would have to be wrong
    about at least one of them.
    """
    plain = purrdf.Store()
    plain.load(
        (
            f"<{EX}s1> <{EX}p> <{EX}o1> .\n"
            f"<{EX}s2> <{EX}p> <{EX}o2> <{EX}g1> .\n"
            f"<{EX}s3> <{EX}p> <{EX}o3> <{EX}g2> .\n"
        ).encode(),
        format=purrdf.RdfFormat.N_QUADS,
    )
    for dataset_capable in (purrdf.RdfFormat.TRIX, purrdf.RdfFormat.HEXTUPLES):
        loss = plain.dump_with_loss(dataset_capable)
        assert loss.named_graph_rows_dropped == 0
        assert loss.statement_rows_dropped == 0
    assert plain.dump_with_loss(purrdf.RdfFormat.N_TRIPLES).named_graph_rows_dropped == 2


def test_the_bytes_are_exactly_what_dump_produces() -> None:
    """`dump_with_loss` is `dump` plus numbers, never a second serializer."""
    store = _store()
    assert store.dump_with_loss(purrdf.RdfFormat.N_QUADS).bytes == store.dump(
        format=purrdf.RdfFormat.N_QUADS
    )


def test_the_mutable_dataset_reports_the_same_loss_as_the_store() -> None:
    """Both quad-bearing entry points route through one core, so they cannot drift."""
    store_loss = _store().dump_with_loss(purrdf.RdfFormat.N_TRIPLES)
    dataset_loss = _mutable().dump_with_loss(purrdf.RdfFormat.N_TRIPLES)
    assert dataset_loss.named_graph_rows_dropped == store_loss.named_graph_rows_dropped
    assert dataset_loss.statement_rows_dropped == store_loss.statement_rows_dropped
    assert (
        dataset_loss.directional_literals_dropped
        == store_loss.directional_literals_dropped
    )


def test_a_dropped_base_direction_is_counted_too() -> None:
    """HexTuples keeps the language tag but has no direction surface."""
    store = purrdf.Store()
    store.add(
        purrdf.Quad(
            purrdf.NamedNode(f"{EX}s"),
            purrdf.NamedNode(f"{EX}p"),
            purrdf.Literal("hello", language="en", direction="ltr"),
        )
    )
    loss = store.dump_with_loss(purrdf.RdfFormat.HEXTUPLES)
    assert loss.directional_literals_dropped == 1
    assert loss.named_graph_rows_dropped == 0


def test_repr_names_every_count() -> None:
    """A caller who prints the object sees all three numbers, not an opaque handle."""
    text = repr(_store().dump_with_loss(purrdf.RdfFormat.N_TRIPLES))
    assert "named_graph_rows_dropped=3" in text
    assert "statement_rows_dropped=0" in text
    assert "directional_literals_dropped=0" in text
