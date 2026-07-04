# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Acceptance driver: sssom against the purrdf ``rdflib`` shadow.

Core path: build a minimal in-memory mapping set (one ``skos:exactMatch``
mapping over ``example.org`` CURIEs) and serialize it to RDF via
:func:`sssom.writers.to_rdf_graph`, which routes through linkml's
``rdflib_dumper`` and returns an rdflib ``Graph`` — the rdflib-backed path whose
plugin lookups must resolve to purrdf.
"""

from __future__ import annotations

import _harness

_PACKAGE = "sssom"

_harness.require_installed(_PACKAGE)
_harness.require_shadow(_PACKAGE)

try:
    import sssom
    import sssom.writers as sssom_writers
    from sssom.util import MappingSetDataFrame
except BaseException as exc:  # noqa: BLE001 - report as a ledgered row
    _harness.failed(_PACKAGE, "import", exc)

try:
    from sssom_schema import Mapping, MappingSet  # type: ignore[import-untyped]

    mapping = Mapping(
        subject_id="ex:alice",
        predicate_id="skos:exactMatch",
        object_id="ex:alice_other",
        mapping_justification="semapv:ManualMappingCuration",
    )
    mapping_set = MappingSet(
        mappings=[mapping],
        mapping_set_id="http://example.org/mappings/demo",
        license="https://example.org/license",
    )
    converter = {
        "ex": "http://example.org/",
        "skos": "http://www.w3.org/2004/02/skos/core#",
        "semapv": "https://w3id.org/semapv/vocab/",
    }
    msdf = MappingSetDataFrame.from_mapping_set(mapping_set, converter=converter)

    graph = sssom_writers.to_rdf_graph(msdf)
    graph_type = f"{type(graph).__module__}.{type(graph).__name__}"
    assert type(graph).__module__.startswith("purrdf"), graph_type
    assert len(graph) >= 1, len(graph)

    _harness.passed(
        _PACKAGE,
        version=getattr(sssom, "__version__", "unknown"),
        graph_type=graph_type,
        detail="sssom.writers.to_rdf_graph produced a purrdf-backed graph",
    )
except SystemExit:
    raise
except BaseException as exc:  # noqa: BLE001 - report as a ledgered row
    _harness.failed(_PACKAGE, "serialize", exc)
