# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Regression coverage for ``Graph`` parse-base handling."""

from __future__ import annotations

from types import ModuleType

EX = "http://example.org/"


def test_derive_parse_base_rejects_bnode_public_id(compat: ModuleType) -> None:
    """A ``BNode`` publicID must not be used as an absolute base IRI."""
    g = compat.Graph()
    bnode = compat.BNode("not-a-base")
    base = g._derive_parse_base(bnode, None, None, None, None)
    assert base != "not-a-base"
    assert base.startswith("file://")


def test_parse_with_bnode_public_id_works(compat: ModuleType) -> None:
    """Parsing with a ``BNode`` publicID falls back to the document/cwd base."""
    g = compat.Graph()
    turtle = f"""@prefix ex: <{EX}> .
<#s> ex:p "o" .
"""
    g.parse(data=turtle, publicID=compat.BNode("pub"), format="turtle")
    assert len(g) == 1
    ((s, p, o),) = list(g)
    assert isinstance(s, compat.URIRef)
    assert str(s).startswith("file://")
    assert str(p) == f"{EX}p"
    assert str(o) == "o"
