# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""The GTS producer's ingestion receipt on the Python surface.

Every producer entry point returns bare ``bytes`` (or, for
``snapshot_content_id_native``, a bare ``str``), and neither can carry an extra
field. So the receipt rides a companion accessor instead of a broken return type:
``gts_ingest_report`` takes the same source arguments a producer call takes and
returns what that build's ``SnapshotBuilder`` accumulated.

The key that makes the accessor load-bearing is ``declarations_omitted``. A named
graph that holds no row has no slot in the frozen ``dist`` snapshot payload, and
interning its IRI would add a term row and shift ``snapshot_content_id``, so
ingestion omits it entirely — and an omission a caller cannot see is a silent drop.
"""

from __future__ import annotations

from typing import Any

import purrdf
from purrdf import RdfFormat

#: Three ordinary triples over four distinct IRIs. No RDF 1.2 statement layer and
#: no named graphs, so the whole receipt is attributable to the base document.
SOURCE = """
@prefix ex: <https://example.org/> .
ex:a ex:p ex:b .
ex:b ex:p ex:c .
ex:c ex:p ex:a .
"""

#: One further row, ingested as an extra named graph the way `compile_gts_native`
#: carries an alignment graph.
ALIGNMENT = "<https://example.org/x> <https://example.org/q> <https://example.org/y> .\n"

ALIGNMENT_GRAPH = "https://example.org/align"


def _report(**kwargs: Any) -> dict[str, Any]:
    """The receipt for `SOURCE`, with any extra producer sources layered on."""
    return purrdf.gts_ingest_report(SOURCE.encode(), RdfFormat.TURTLE, **kwargs)


def test_the_receipt_carries_every_counter_the_report_defines() -> None:
    report = _report()
    assert set(report) == {
        "rows_consumed",
        "terms_interned",
        "declarations_omitted",
        "scratch_bytes",
    }
    assert report["rows_consumed"] == 3
    assert report["terms_interned"] > 0
    assert report["scratch_bytes"] > 0


def test_a_source_declaring_no_empty_graph_omits_nothing() -> None:
    """The neighbouring case to an omission, which is what makes it readable.

    A document that withholds nothing reports an EMPTY list, not an absent key. A
    caller can therefore test `report["declarations_omitted"]` unconditionally,
    which is the whole point of stating the omission rather than logging it.
    """
    assert _report()["declarations_omitted"] == []


def test_the_receipt_covers_the_multi_source_producer_shape() -> None:
    """`compile_gts_native`'s ingest half, not just its base document.

    The accessor takes the same `rdf12_data` / `named_graphs` arguments the
    compiler does, so the receipt accounts for the WHOLE compile. A receipt that
    only ever described the base would under-report exactly the multi-source
    builds that can carry a declaration-only graph.
    """
    single = _report()["rows_consumed"]
    multi = _report(
        named_graphs=[(ALIGNMENT.encode(), RdfFormat.N_QUADS, ALIGNMENT_GRAPH, None)]
    )["rows_consumed"]
    assert multi == single + 1


def test_the_producer_and_the_accessor_take_the_same_sources() -> None:
    # Same bytes in, same build: one emits the container, the other accounts for
    # what went into it. Neither signature moved to make that possible.
    assert purrdf.gts_from_quads(SOURCE.encode(), format=RdfFormat.TURTLE)
    assert purrdf.snapshot_content_id_native(
        SOURCE.encode(), format=RdfFormat.TURTLE
    ).startswith("blake3:")
    assert _report()["rows_consumed"] == 3


def test_it_is_reachable_through_the_gts_namespace() -> None:
    assert purrdf.gts.gts_ingest_report is purrdf.gts_ingest_report
