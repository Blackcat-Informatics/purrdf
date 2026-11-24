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

expected, *platform = sys.argv[1:]
for project, reciprocal in (("purrdf", "purrdf-rdflib"), ("purrdf-rdflib", "purrdf")):
    assert importlib.metadata.version(project) == expected, (project, expected)
    pins = {req.partition(";")[0].strip() for req in importlib.metadata.requires(project) or []}
    assert f"{reciprocal}=={expected}" in pins, (project, pins)
if platform:
    wheel = importlib.metadata.distribution("purrdf").read_text("WHEEL")
    tags = [line for line in (wheel or "").splitlines() if line.startswith("Tag:")]
    assert tags == [f"Tag: cp313-abi3-{platform[0]}"], wheel
assert rdflib.Graph is compat.Graph
graph = Graph()
subject = URIRef("https://example.org/subject")
predicate = URIRef("https://example.org/predicate")
graph.add((subject, predicate, Literal("candidate")))
assert len(graph) == 1
assert list(graph.objects(subject, predicate)) == [Literal("candidate")]
assert str(RDF.type) == "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
