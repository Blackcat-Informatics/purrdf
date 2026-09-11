# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""The GTS producer's ingestion receipt on the Python surface.

``compile_gts_with_report`` is the one-pass producer: it returns the container
bytes beside the receipt for the very ingestion that minted them, so both answers
cost a single parse and a single intern and are the same build by construction.

The frozen producer entry points return bare ``bytes`` (or, for
``snapshot_content_id_native``, a bare ``str``), and neither can carry an extra
field. So the receipt also rides a companion accessor rather than a broken return
type: ``gts_ingest_report`` takes the same source arguments a producer call takes
and returns what that build's ``SnapshotBuilder`` accumulated, for a caller who
wants the receipt and no bytes.

The key that makes the receipt load-bearing is ``declarations_omitted``. A named
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


#: The extra sources that make a build multi-source, reused by every test that has
#: to prove the two surfaces agree on more than a lone base document.
MULTI_SOURCE: dict[str, Any] = {
    "named_graphs": [(ALIGNMENT.encode(), RdfFormat.N_QUADS, ALIGNMENT_GRAPH, None)]
}


def _report(**kwargs: Any) -> dict[str, Any]:
    """The receipt for `SOURCE`, with any extra producer sources layered on."""
    return purrdf.gts_ingest_report(SOURCE.encode(), RdfFormat.TURTLE, **kwargs)


def _compile(**kwargs: Any) -> bytes:
    """The bare producer's bytes for `SOURCE`, with any extra sources layered on."""
    return purrdf.compile_gts_native(SOURCE.encode(), RdfFormat.TURTLE, **kwargs)


def _compile_with_report(**kwargs: Any) -> dict[str, Any]:
    """The one-pass producer's bytes-and-receipt answer for the same sources."""
    return purrdf.compile_gts_with_report(SOURCE.encode(), RdfFormat.TURTLE, **kwargs)


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
    multi = _report(**MULTI_SOURCE)["rows_consumed"]
    assert multi == single + 1


def test_the_producer_and_the_accessor_take_the_same_sources() -> None:
    # Same bytes in, same build: one emits the container, the other accounts for
    # what went into it. Neither signature moved to make that possible.
    assert purrdf.gts_from_quads(SOURCE.encode(), format=RdfFormat.TURTLE)
    assert purrdf.snapshot_content_id_native(
        SOURCE.encode(), format=RdfFormat.TURTLE
    ).startswith("blake3:")
    assert _report()["rows_consumed"] == 3


def test_the_one_pass_producer_carries_both_halves_of_one_build() -> None:
    result = _compile_with_report()
    assert set(result) == {"snapshot_bytes", "ingest_report"}
    assert isinstance(result["snapshot_bytes"], bytes)
    assert set(result["ingest_report"]) == {
        "rows_consumed",
        "terms_interned",
        "declarations_omitted",
        "scratch_bytes",
    }


def test_the_one_pass_producer_mints_the_bare_producers_bytes() -> None:
    """The receipt costs the caller nothing in the container it describes.

    If the with-report producer emitted so much as a different byte, its receipt
    would be about some other build and `compile_gts_native` would be the entry
    point nobody could get a receipt for. Proved on the single-source shape AND on
    the multi-source shape, because the extra sources are where an ingestion order
    could diverge.
    """
    assert _compile_with_report()["snapshot_bytes"] == _compile()
    assert (
        _compile_with_report(**MULTI_SOURCE)["snapshot_bytes"]
        == _compile(**MULTI_SOURCE)
    )


def test_the_one_pass_receipt_equals_the_standalone_report() -> None:
    """One ingestion sequence, so one set of figures — whoever asks for them.

    The with-report producer reads its receipt off the builder it is about to emit;
    the standalone accessor runs the same `IngestSources::ingest_into` and emits
    nothing. Equal receipts for equal sources is what says those are the same
    sequence rather than two that happen to agree today.
    """
    assert _compile_with_report()["ingest_report"] == _report()
    assert (
        _compile_with_report(**MULTI_SOURCE)["ingest_report"]
        == _report(**MULTI_SOURCE)
    )


def test_it_is_reachable_through_the_gts_namespace() -> None:
    assert purrdf.gts.gts_ingest_report is purrdf.gts_ingest_report
    assert purrdf.gts.compile_gts_with_report is purrdf.compile_gts_with_report
