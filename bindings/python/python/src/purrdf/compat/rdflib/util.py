# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Utility helpers for the purrdf compat shim (RDFLib ``rdflib.util``)."""

from __future__ import annotations

from pathlib import PurePath

#: File-suffix → RDFLib format name (the subset the native surface serves, plus
#: the suffixes the toolchain detects before dispatching).
_SUFFIX_FORMATS: dict[str, str] = {
    ".ttl": "turtle",
    ".turtle": "turtle",
    ".n3": "n3",
    ".nt": "nt",
    ".ntriples": "nt",
    ".nq": "nquads",
    ".nquads": "nquads",
    ".trig": "trig",
    ".jsonld": "json-ld",
    ".json": "json-ld",
    ".rdf": "xml",
    ".xml": "xml",
    ".owl": "xml",
}


def guess_format(
    path: str | PurePath, fmap: dict[str, str] | None = None
) -> str | None:
    """Guess an RDFLib format name from a path's suffix (RDFLib parity)."""
    suffix = PurePath(path).suffix.lower()
    table = fmap if fmap is not None else _SUFFIX_FORMATS
    return table.get(suffix)
