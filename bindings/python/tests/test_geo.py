# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: verifies installed CPython geographic exports, exception channels, keyword forwarding and retained prepared/index handle lifetimes.

"""The Python surface consumes the native geographic producer without mathematics."""

from __future__ import annotations

import json

import pytest

import purrdf
from purrdf import geo

DISTANCE = '{"version":1,"operation":"distance","a":{"longitude":"0","latitude":"0"},"b":{"longitude":"0","latitude":"0"}}'
INDEX = '{"version":1,"operation":"point-index","grid":"wgs84","level":2,"points":[{"key":"7","point":{"longitude":"0","latitude":"0"}},{"key":"11","point":{"longitude":"1","latitude":"0"}}]}'
SEARCH = '{"version":1,"operation":"search-physical","center":{"longitude":"0","latitude":"0"},"radius_metres":"0"}'
NEGATIVE = '{"version":1,"operation":"search-reported","center":{"longitude":"0","latitude":"0"},"threshold_metres":{"kind":"integer","value":"-1"}}'
AXES = '{"version":1,"references":[{"crs":"http://www.opengis.net/def/crs/EPSG/0/4326","ellipsoid":"wgs84","axes":"lat-lon"}]}'
AXES_FILTER = '''FILTER(<http://www.opengis.net/def/function/geosparql/distance>(
 "<http://www.opengis.net/def/crs/EPSG/0/4326> POINT(0 1)"^^<http://www.opengis.net/ont/geosparql#wktLiteral>,
 "POINT(1 0)"^^<http://www.opengis.net/ont/geosparql#wktLiteral>,
 <http://www.opengis.net/def/uom/OGC/1.0/metre>) = 0)'''
AXES_QUERY = "ASK { " + AXES_FILTER + " }"


def test_session_exports_canonical_success_and_complete_semantic_refusal() -> None:
    """Success and refusal retain the same native identity and stable response bytes."""
    session = geo.GeoSession()
    text = session.call(DISTANCE)
    response = json.loads(text)
    assert response["result"]["micrometres"] == "0"
    assert session.call(DISTANCE) == text
    refused = json.loads(session.call('{"version":1,"operation":"not-an-operation"}'))
    assert refused["error"]["code"] == "invalid-record"
    assert refused["identity"] == response["identity"]
    canonical_profile = session.profile().to_json()
    assert geo.GeoProfile(canonical_profile).to_json() == canonical_profile


def test_operational_refusal_uses_exception_channel_and_keeps_a_healthy_neighbor() -> None:
    """A work-stopped call publishes no JSON; independent admitted calls still answer."""
    refused = geo.GeoSession('{"version":1,"limits":{"max_work_items":1}}')
    with pytest.raises(ValueError, match="work"):
        refused.call(DISTANCE)
    assert json.loads(geo.GeoSession().call(DISTANCE))["result"]["micrometres"] == "0"


def test_index_retains_source_and_profile_after_parent_deletion() -> None:
    """The index's physical and reported comparisons share retained native buckets."""
    session = geo.GeoSession()
    index = session.point_index(INDEX)
    del session
    text = index.call(SEARCH)
    assert json.loads(text)["result"]["keys"] == ["7"]
    assert index.call(SEARCH) == text
    assert json.loads(index.call(NEGATIVE))["result"]["keys"] == []


def test_explicit_axes_reach_query_governed_entailment_and_retained_preparation() -> None:
    """Each actual evaluating door forwards the selected immutable profile."""
    store = purrdf.Store()
    with pytest.raises(ValueError, match="unregistered"):
        store.query(AXES_QUERY)
    session = geo.GeoSession(AXES)
    profile = session.profile()
    for selection in (session, profile, AXES):
        assert bool(store.query(AXES_QUERY, geo=selection))
        assert bool(store.query_rdflib(AXES_QUERY, geo=selection))
        governed = store.query_governed(AXES_QUERY, geo=selection)
        assert governed.is_complete and bool(governed.result)
        entailed = store.query_entailment_governed(AXES_QUERY, "rdfs", geo=selection)
        assert entailed.outcome is not None and entailed.outcome.is_complete
        assert bool(entailed.outcome.result)
    prepared = store.prepare(AXES_QUERY, geo=session)
    del session, profile
    assert bool(prepared.run())
    assert bool(prepared.run())
    with pytest.raises(ValueError, match="unregistered"):
        store.query(AXES_QUERY)


def test_explicit_axes_reach_both_update_where_doors() -> None:
    """An UPDATE's WHERE evaluates the same selected scalar as ordinary queries."""
    for governed in (False, True):
        store = purrdf.Store()
        command = "INSERT { <http://example.org/s> <http://example.org/p> <http://example.org/o> } WHERE { " + AXES_FILTER + " }"
        if governed:
            assert store.update_governed(command, geo=AXES).is_applied
        else:
            store.update(command, geo=AXES)
        assert bool(store.query("ASK { <http://example.org/s> <http://example.org/p> <http://example.org/o> }"))
