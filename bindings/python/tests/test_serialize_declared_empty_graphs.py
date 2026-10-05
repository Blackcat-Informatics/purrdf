# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: asserts the SerializeLoss attributes the installed CPython extension exposes to Python callers.
"""Declared empty named graphs are counted, kept and withdrawn through `dump_with_loss`.

A graph a document declares with no row (`<g> { }`) owns nothing a row count can see,
so a target that cannot write an empty graph would drop it silently.
`empty_named_graphs_dropped` is that count; the neighbouring dataset reports 0.
"""

from __future__ import annotations

import purrdf

EX = "https://example.org/"

# A default-graph row, a named graph with a row, and two graphs declared with no row:
# one IRI-named, one blank-named.
_DECLARED_TRIG = (
    f"<{EX}s> <{EX}p> <{EX}o> .\n"
    f"<{EX}g1> {{ <{EX}s> <{EX}p> <{EX}o2> . }}\n"
    f"<{EX}empty> {{ }}\n"
    "_:bg { }\n"
).encode()


def test_declared_empty_graphs_a_target_cannot_spell_are_counted() -> None:
    """A loaded `<g> { }` is kept, written where it can be, and counted where not."""
    for store in (purrdf.Store(), purrdf.MutableDataset()):
        store.load(_DECLARED_TRIG, format=purrdf.RdfFormat.TRIG)
        for lossy in (
            purrdf.RdfFormat.N_QUADS,
            purrdf.RdfFormat.HEXTUPLES,
            purrdf.RdfFormat.TURTLE,
        ):
            loss = store.dump_with_loss(lossy)
            assert loss.empty_named_graphs_dropped == 2, lossy
            assert f"{EX}empty" not in loss.bytes.decode(), lossy
        for spelled in (
            purrdf.RdfFormat.TRIG,
            purrdf.RdfFormat.TRIX,
            purrdf.RdfFormat.JSON_LD,
            purrdf.RdfFormat.YAML_LD,
        ):
            loss = store.dump_with_loss(spelled)
            assert loss.empty_named_graphs_dropped == 0, spelled
            assert f"{EX}empty" in loss.bytes.decode(), spelled
        # The counts that already existed keep their meaning: the empty graphs own no
        # row, so N-Quads drops no row at all.
        assert store.dump_with_loss(purrdf.RdfFormat.N_QUADS).named_graph_rows_dropped == 0


def test_a_store_without_declared_empty_graphs_counts_none() -> None:
    """The neighbouring dataset: the same rows, no empty graph, nothing counted."""
    store = purrdf.Store()
    store.load(
        f"<{EX}s> <{EX}p> <{EX}o> .\n<{EX}g1> {{ <{EX}s> <{EX}p> <{EX}o2> . }}\n".encode(),
        format=purrdf.RdfFormat.TRIG,
    )
    for format in (
        purrdf.RdfFormat.N_QUADS,
        purrdf.RdfFormat.HEXTUPLES,
        purrdf.RdfFormat.TURTLE,
        purrdf.RdfFormat.TRIG,
    ):
        assert store.dump_with_loss(format).empty_named_graphs_dropped == 0, format
    assert "empty_named_graphs_dropped=0" in repr(store.dump_with_loss(purrdf.RdfFormat.N_QUADS))


def test_a_dropped_declared_graph_is_withdrawn_and_an_untouched_one_survives() -> None:
    """A loaded `<g> { }` follows the Update withdrawal rules: DROP removes it."""
    for store in (purrdf.Store(), purrdf.MutableDataset()):
        store.load(_DECLARED_TRIG, format=purrdf.RdfFormat.TRIG)
        store.update(f"DROP GRAPH <{EX}empty>")
        trig = store.dump_with_loss(purrdf.RdfFormat.TRIG).bytes.decode()
        assert f"{EX}empty" not in trig
        assert "_:" in trig  # the untouched blank-named declaration survives
        assert store.dump_with_loss(purrdf.RdfFormat.N_QUADS).empty_named_graphs_dropped == 1
        store.update("DROP ALL")
        assert store.dump_with_loss(purrdf.RdfFormat.N_QUADS).empty_named_graphs_dropped == 0
