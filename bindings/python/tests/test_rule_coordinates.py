# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: these cases exercise installed Python argument extraction and the
# SHACL/SRL host routes, error precedence and static Python keyword refusal.
"""Exact coordinates reach every installed Python rule execution route."""

from __future__ import annotations

import json
import pytest
import purrdf

EX = "http://example.org/"
NOW = "2020-01-02T03:04:05.12345678901234567890123456789+05:30"
BCE = "-0001-02-03T04:05:06.98765432109876543210987654321-04:00"
SHAPES = '''@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:S a sh:NodeShape ; sh:targetNode ex:s ; sh:rule ex:r .
ex:r a sh:SPARQLRule ; sh:runOnce true ; sh:construct """
  PREFIX ex: <http://example.org/>
  CONSTRUCT { ex:s ex:time ?time ; ex:uuid ?uuid ; ex:text ?text }
  WHERE { BIND(NOW() AS ?time) BIND(UUID() AS ?uuid)
    BIND(STRUUID() AS ?text) FILTER(REGEX("ok", "^ok$")) }
""" .'''
SRL = '''PREFIX : <http://example.org/>
RULE { :s :time ?time ; :uuid ?uuid ; :text ?text }
WHERE { SET(?time := NOW()) SET(?uuid := UUID()) SET(?text := STRUUID())
  FILTER(REGEX("ok", "^ok$")) }'''
ROUTES = ("entail", "shapes", "srl")


def _run(route, law, **coordinates):
    options = {"xpath_regex": law, "max_term_generating_rounds": 8,
               "max_generated_terms": 32, **coordinates}
    if route == "entail":
        result = purrdf.shacl.entail(SHAPES, "", **options)
        text = result["ntriples"]
    else:
        source = {"shapes_ttl": SHAPES} if route == "shapes" else {"srl": SRL}
        result = purrdf.shapes.apply_rules("", **source, explain=True, **options)
        text = result["inferred"]
        assert result["proof"] and "derived" in result["proof"]
    assert result["diagnostics"] == [] and len(json.loads(result["execution_receipt_json"])["programIdentity"]) == 64
    store = purrdf.Store()
    store.load(text, purrdf.RdfFormat.N_TRIPLES)
    rows = list(store.query(f"SELECT ?p ?o {{ <{EX}s> ?p ?o }} ORDER BY ?p"))
    assert len(rows) == 3
    return tuple(row[1].value for row in rows)


@pytest.mark.parametrize("route", ROUTES)
@pytest.mark.parametrize("law", (None, *purrdf.XPATH_REGEX_PROFILES))
def test_rule_routes_preserve_exact_coordinates_and_independent_defaults(route, law):
    first = _run(route, law, now=NOW, entropy_seed=0)
    assert first[1] == NOW and first[2].startswith("urn:uuid:")
    assert _run(route, law, now=NOW, entropy_seed=0) == first
    changed = _run(route, law, now=BCE, entropy_seed=0)
    assert changed[1] == BCE and changed[0] == first[0] and changed[2] == first[2]
    changed = _run(route, law, now=NOW, entropy_seed=2**64 - 1)
    assert changed[1] == NOW and changed[0] != first[0] and changed[2] != first[2]
    assert _run(route, law, now=NOW)[1] == NOW
    omitted = _run(route, law, entropy_seed=0)
    assert omitted[1] != NOW and omitted[0] == first[0] and omitted[2] == first[2]


def test_rule_coordinate_refusals_precede_document_parsing_and_preserve_xpath_precedence():
    calls = (
        lambda **kw: purrdf.shapes.entail("invalid shapes", "invalid data", **kw),
        lambda **kw: purrdf.shacl.apply_rules("invalid data", "invalid shapes", **kw),
        lambda **kw: purrdf.shapes.apply_rules("invalid data", srl="invalid rules", **kw),
    )
    for call in calls:
        with pytest.raises(ValueError, match="now") as caught:
            call(now="invalid", imports=[("relative", "invalid imported document")])
        assert caught.value.message_id is None and caught.value.presentation is None
        for seed in (-1, 2**64):
            with pytest.raises(OverflowError):
                call(entropy_seed=seed)
        for arguments in ({"now": 3}, {"entropy_seed": "0"}):
            with pytest.raises(TypeError):
                call(**arguments)
        with pytest.raises(ValueError, match="xpath_regex"):
            call(xpath_regex="invalid", now="invalid")
    for arguments in ({"now": NOW}, {"entropy_seed": 0}):
        with pytest.raises(TypeError):
            purrdf.shapes.check_rules(SRL, **arguments)
