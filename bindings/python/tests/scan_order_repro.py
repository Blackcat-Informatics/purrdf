# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Child-process worker for the scan-order determinism regression pin.

Run standalone (``python scan_order_repro.py``) or spawned by
``test_scan_order_determinism.py`` — one fresh interpreter per invocation, so each
run draws its own process-random seeds (thread hash seed, ASLR, etc.) exactly the
way the field report that motivated this test did. It builds the SAME tiny dataset
via ``purrdf.Store`` (mirroring oxigraph's mutable, COW-delta-backed store), runs a
plain unordered projection plus GROUP_CONCAT/FIRST/LAST over the same scan, and
prints the results as one JSON line so the parent test can diff them across many
independent processes.

The dataset and queries mirror the field reproduction verbatim: three quads sharing
a predicate, no ``ORDER BY``, so solution order is whatever the engine's BGP scan
produced — the exact order `purrdf_core::ir::MutableDataset::freeze` used to
scramble across processes (see ``crates/rdf-core/src/ir/mutable.rs``).
"""

from __future__ import annotations

import json

import purrdf

EX = "http://example.org/"
AGG_NS = "https://example.org/agg#"

TTL = f"""
@prefix ex: <{EX}> .
ex:a ex:item "x" .
ex:b ex:item "y" .
ex:c ex:item "z" .
"""


def _run(store: purrdf.Store) -> dict[str, object]:
    # 1. Plain, unordered projection — the raw scan order a BGP over `?s ex:item ?v`
    #    produces (no ORDER BY to mask it).
    plain = [
        str(row["v"].value)
        for row in store.query(f"SELECT ?v WHERE {{ ?s <{EX}item> ?v }}")
    ]

    # 2. GROUP_CONCAT over the identical BGP — the field report's own reproduction.
    (gc_row,) = list(
        store.query(
            f'SELECT (GROUP_CONCAT(?v; SEPARATOR="|") AS ?g) '
            f"WHERE {{ ?s <{EX}item> ?v }}"
        )
    )
    group_concat = str(gc_row["g"].value)

    # 3. FIRST / LAST through the statistical-aggregate namespace, over the same
    #    BGP — the field report's other affected fold ("FIRST == plain-projection[0]
    #    every time", so a faithful fold over a scrambled scan still scrambles).
    (first_row,) = list(
        store.query(
            f"SELECT (AGG(<{AGG_NS}FIRST>, ?v) AS ?f) WHERE {{ ?s <{EX}item> ?v }}",
            aggregate_namespace=AGG_NS,
        )
    )
    (last_row,) = list(
        store.query(
            f"SELECT (AGG(<{AGG_NS}LAST>, ?v) AS ?l) WHERE {{ ?s <{EX}item> ?v }}",
            aggregate_namespace=AGG_NS,
        )
    )

    return {
        "plain": plain,
        "group_concat": group_concat,
        "first": str(first_row["f"].value),
        "last": str(last_row["l"].value),
    }


def main() -> None:
    store = purrdf.Store()
    store.load(TTL, format=purrdf.RdfFormat.TURTLE)
    print(json.dumps(_run(store)))


if __name__ == "__main__":
    main()
