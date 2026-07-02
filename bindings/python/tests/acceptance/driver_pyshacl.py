# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Acceptance driver: pyshacl against the purrdf ``rdflib`` shadow.

Core path: build a tiny ``example.org`` data graph and a trivial SHACL node
shape, then call :func:`pyshacl.validate`, asserting it returns the
``(conforms, results_graph, results_text)`` triple without crashing (validation
*semantics* are out of scope for the acceptance bar).

At the current shim revision this row is a ledgered strict-xfail; pyshacl fails
to import against the shim before ``validate`` is reached. The first observed gap
is a missing term class — ``pyshacl.pytypes`` does ``from rdflib.term import
IdentifiedNode``, but the shim's term model collapses rdflib's abstract node
bases and does not expose ``IdentifiedNode``. Behind that, pyshacl is not a
public-surface consumer at all: it reads ``rdflib.__version__`` to version-gate a
suite of monkeypatches against rdflib's PRIVATE Python internals
(``rdflib.term._toPythonMapping`` / ``_parseBoolean``, the ``rdflib.plugins.sparql``
Python algebra, etc.), which the public-surface, native-Rust-backed shim does not
reimplement. The driver still expresses the full core path so that, if the
coupling is ever bridged, this row XPASSes and forces the ledger to shrink.
"""

from __future__ import annotations

import _harness

_PACKAGE = "pyshacl"

_harness.require_installed(_PACKAGE)
_harness.require_shadow(_PACKAGE)

import rdflib  # noqa: E402 - after the shadow guard, resolves to purrdf

_DATA = '<http://example.org/alice> <http://example.org/name> "Alice" .'
_SHAPES = """
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .

ex:PersonShape a sh:NodeShape ;
    sh:targetNode ex:alice ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
"""

try:
    import pyshacl
except BaseException as exc:  # noqa: BLE001 - report as a ledgered row
    _harness.failed(_PACKAGE, "import", exc)

try:
    data_graph = rdflib.Graph()
    data_graph.parse(data=_DATA, format="nt")
    shapes_graph = rdflib.Graph()
    shapes_graph.parse(data=_SHAPES, format="turtle")

    conforms, results_graph, results_text = pyshacl.validate(
        data_graph, shacl_graph=shapes_graph
    )
    assert isinstance(results_text, str), type(results_text)
    assert type(results_graph).__module__.startswith("purrdf"), type(
        results_graph
    ).__module__

    _harness.passed(
        _PACKAGE,
        version=pyshacl.__version__,
        conforms=bool(conforms),
        detail="pyshacl.validate ran its core path on a purrdf-backed graph",
    )
except SystemExit:
    raise
except BaseException as exc:  # noqa: BLE001 - report as a ledgered row
    _harness.failed(_PACKAGE, "validate", exc)
