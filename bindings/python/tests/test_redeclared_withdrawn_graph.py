# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: drives the installed CPython extension's Store and MutableDataset
# load, add, remove, compaction, query and dump methods as Python callers reach them.
"""A loaded `<bg> { }` re-declares a compacted graph whose rows were removed.

Compacting folds `<bg>` into the store's base. Removing its last row withdraws
the base's declaration, and loading a document that declares `<bg> { }` declares
it again. That must restore the one declaration rather than add a second, so
after a row lands in `<bg>` the graph is still enumerated, written and counted
exactly once.
"""

from __future__ import annotations

import purrdf

EX = "https://example.org/"
_GRAPH = f"<{EX}bg>"


def _quad(subject: str) -> purrdf.Quad:
    return purrdf.Quad(
        purrdf.NamedNode(f"{EX}{subject}"),
        purrdf.NamedNode(f"{EX}p"),
        purrdf.NamedNode(f"{EX}o"),
        purrdf.NamedNode(f"{EX}bg"),
    )


def _compact(store: purrdf.Store | purrdf.MutableDataset) -> None:
    if isinstance(store, purrdf.Store):
        store.checkpoint()
    else:
        store.compact()


def _graphs(store: purrdf.Store | purrdf.MutableDataset) -> int:
    return len(store.query("SELECT ?g WHERE { GRAPH ?g { } }"))


def test_a_redeclared_withdrawn_graph_is_listed_once() -> None:
    for store in (purrdf.Store(), purrdf.MutableDataset()):
        store.load(f"{_GRAPH} {{ <{EX}old> <{EX}p> <{EX}o> . }}\n".encode(), format=purrdf.RdfFormat.TRIG)
        _compact(store)
        store.remove(_quad("old"))
        assert _graphs(store) == 0, "removing the last row withdraws the graph"

        store.load(f"{_GRAPH} {{ }}\n".encode(), format=purrdf.RdfFormat.TRIG)
        assert _graphs(store) == 1
        assert store.dump_with_loss(purrdf.RdfFormat.N_QUADS).empty_named_graphs_dropped == 1

        store.add(_quad("new"))
        assert _graphs(store) == 1, "a row in the redeclared graph does not list it twice"
        trig = store.dump_with_loss(purrdf.RdfFormat.TRIG).bytes.decode()
        assert trig.count(f"{EX}bg") == 1, trig
        assert store.dump_with_loss(purrdf.RdfFormat.N_QUADS).empty_named_graphs_dropped == 0

        store.remove(_quad("new"))
        assert _graphs(store) == 0, "removing its last row withdraws it again"
        assert store.dump_with_loss(purrdf.RdfFormat.N_QUADS).empty_named_graphs_dropped == 0
