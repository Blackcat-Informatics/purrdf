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

from . import parser, paths, plugin, serializer, store, util

__version__ = "7.6.0"

#: RDFLib's literal-normalization switch. pyshacl's monkey patch toggles this at
#: runtime; exposing it keeps the top-level module assignment from failing.
NORMALIZE_LITERALS = True

from .graph import (
    DATASET_DEFAULT_GRAPH_ID,
    BatchAddGraph,
    ConjunctiveGraph,
    Dataset,
    Graph,
    Seq,
)
from .namespace import (
    BRICK,
    CSVW,
    DC,
    DCAM,
    DCAT,
    DCMITYPE,
    DCTERMS,
    DOAP,
    FOAF,
    GEO,
    ODRL2,
    ORG,
    OWL,
    PROF,
    PROV,
    QB,
    RDF,
    RDFS,
    SDO,
    SH,
    SKOS,
    SOSA,
    SSN,
    TIME,
    VANN,
    VOID,
    WGS,
    XMLNS,
    XSD,
    ClosedNamespace,
    DefinedNamespace,
    DefinedNamespaceMeta,
    Namespace,
    NamespaceManager,
    URIPattern,
)
from .query import Result, ResultRow
from .resource import Resource
from .term import BNode, IdentifiedNode, Identifier, Literal, Node, URIRef, Variable

__all__ = [
    "NORMALIZE_LITERALS",
    "BRICK",
    "CSVW",
    "DC",
    "DCAM",
    "DCAT",
    "DCMITYPE",
    "DCTERMS",
    "DOAP",
    "FOAF",
    "GEO",
    "ODRL2",
    "ORG",
    "OWL",
    "PROF",
    "PROV",
    "QB",
    "RDF",
    "RDFS",
    "SDO",
    "SH",
    "SKOS",
    "SOSA",
    "SSN",
    "TIME",
    "VANN",
    "VOID",
    "WGS",
    "XMLNS",
    "XSD",
    "DATASET_DEFAULT_GRAPH_ID",
    "BNode",
    "BatchAddGraph",
    "ClosedNamespace",
    "ConjunctiveGraph",
    "Dataset",
    "DefinedNamespace",
    "DefinedNamespaceMeta",
    "Graph",
    "IdentifiedNode",
    "Identifier",
    "Literal",
    "Namespace",
    "NamespaceManager",
    "Node",
    "Resource",
    "Result",
    "ResultRow",
    "Seq",
    "URIPattern",
    "URIRef",
    "Variable",
    "parser",
    "paths",
    "plugin",
    "serializer",
    "store",
    "util",
]
