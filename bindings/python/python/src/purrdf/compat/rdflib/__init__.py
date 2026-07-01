# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""``purrdf.compat.rdflib`` — the purrdf P0 rdflib drop-in surface.

Re-exports the names internal code imports from ``rdflib`` directly, so a
migration is a pure import-prefix swap:

    from rdflib import Graph, URIRef, Literal, RDF
    →
    from purrdf.compat.rdflib import Graph, URIRef, Literal, RDF

Submodules mirror RDFLib's layout (``.term``, ``.namespace``, ``.graph``,
``.collection``, ``.compare``, ``.util``, ``.query``) so ``from rdflib.namespace
import OWL`` → ``from purrdf.compat.rdflib.namespace import OWL`` and so on.
"""

from __future__ import annotations

from .graph import ConjunctiveGraph, Dataset, Graph
from .namespace import (
    DC,
    DCAT,
    DCTERMS,
    FOAF,
    OWL,
    PROV,
    RDF,
    RDFS,
    SDO,
    SH,
    SKOS,
    VOID,
    XSD,
    Namespace,
    NamespaceManager,
)
from .query import Result, ResultRow
from .term import BNode, Identifier, Literal, Node, URIRef, Variable

__all__ = [
    "DC",
    "DCAT",
    "DCTERMS",
    "FOAF",
    "OWL",
    "PROV",
    "RDF",
    "RDFS",
    "SDO",
    "SH",
    "SKOS",
    "VOID",
    "XSD",
    "BNode",
    "ConjunctiveGraph",
    "Dataset",
    "Graph",
    "Identifier",
    "Literal",
    "Namespace",
    "NamespaceManager",
    "Node",
    "Result",
    "ResultRow",
    "URIRef",
    "Variable",
]
