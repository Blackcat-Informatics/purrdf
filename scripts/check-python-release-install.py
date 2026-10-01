#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check both freshly installed candidate distributions outside the oracle env."""

import importlib.metadata
import sys

import rdflib
from purrdf.compat import rdflib as compat
from rdflib import Graph, Literal, URIRef
from rdflib.namespace import RDF

expected = sys.argv[1]
for project in ("purrdf", "purrdf-rdflib"):
    actual = importlib.metadata.version(project)
    assert actual == expected, (project, actual, expected)
requirements = importlib.metadata.requires("purrdf-rdflib")
assert requirements and f"purrdf=={expected}" in requirements, requirements
assert rdflib.Graph is compat.Graph
graph = Graph()
subject = URIRef("https://example.org/subject")
predicate = URIRef("https://example.org/predicate")
graph.add((subject, predicate, Literal("candidate")))
assert len(graph) == 1
assert list(graph.objects(subject, predicate)) == [Literal("candidate")]
assert str(RDF.type) == "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
print(
    f"installed purrdf and purrdf-rdflib {expected}: metadata, shadow imports and graph operations passed"
)
