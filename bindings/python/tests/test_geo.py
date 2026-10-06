# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: These tests execute Python object coercion, GIL-bound configuration, and the installed extension's public host APIs.
"""The Python surface executes the Rust geographic codec and query resolver."""

import json

import pytest
import purrdf

DISTANCE = '{"version":1,"operation":"distance","a":{"longitude":"0","latitude":"0"},"b":{"longitude":"1","latitude":"0"}}'
QUERY = 'SELECT (<http://www.opengis.net/def/function/geosparql/metricDistance>("POINT(0 0)"^^<http://www.opengis.net/ont/geosparql#wktLiteral>,"POINT(1 0)"^^<http://www.opengis.net/ont/geosparql#wktLiteral>) AS ?distance) WHERE {}'


def test_scalar_profile_and_prepared_queries_share_the_rust_resolver() -> None:
    profile = purrdf.geo.GeoProfile()
    session = purrdf.geo.GeoSession(profile)
    response = json.loads(session.call(DISTANCE))
    assert response["result"]["metres"] == "111319.490793"
    assert len(response["result"]["certificate"]) > 64
    assert json.loads(session.profile().to_json()) == json.loads(profile.to_json())
    assert purrdf.geo.GeoSession(session).call(DISTANCE) == session.call(DISTANCE)
    assert purrdf.geo.GeoSession(profile.to_json()).call(DISTANCE) == session.call(DISTANCE)
    store = purrdf.Store()
    for geo in (None, profile, session, profile.to_json()):
        result = store.query(QUERY, geo=geo)
        rows = list(result)
        assert rows[0]["distance"].value == "1.11319490793E5"
        prepared = store.prepare(QUERY, geo=geo)
        prepared_result = prepared.run()
        assert list(prepared_result)[0]["distance"].value == rows[0]["distance"].value


def test_exact_range_and_nonfinite_thresholds_are_typed_responses() -> None:
    session = purrdf.geo.GeoSession()
    record = json.loads(session.call(DISTANCE.replace('"1"', '"180.0000000000000000000001"')))
    assert record["error"]["code"] == "coordinate-range"
    assert "result" not in record
    threshold = '{"version":1,"operation":"within","a":{"longitude":"0","latitude":"0"},"b":{"longitude":"0","latitude":"0"},"threshold_metres":{"kind":"double-bits","value":"7ff0000000000000"}}'
    assert json.loads(session.call(threshold))["error"]["code"] == "nonfinite-threshold"
    with pytest.raises(ValueError):
        purrdf.geo.GeoProfile('{"version":1,"extra":true}')


def test_reusable_index_negative_reported_search_and_repeated_metadata() -> None:
    session = purrdf.geo.GeoSession()
    source = '{"version":1,"operation":"point-index","grid":"wgs84","level":2,"points":[{"key":"7","point":{"longitude":"0","latitude":"0"}}]}'
    index = session.point_index(source)
    before = json.loads(index.call('{"version":1,"operation":"point-index"}'))
    search = '{"version":1,"operation":"search-reported","center":{"longitude":"0","latitude":"0"},"threshold_metres":{"kind":"integer","value":"-1"}}'
    assert json.loads(index.call(search))["result"]["keys"] == []
    assert json.loads(index.call('{"version":1,"operation":"point-index"}')) == before


def test_point_buffer_exports_the_frozen_grid_and_native_certificate() -> None:
    session = purrdf.geo.GeoSession()
    request = '{"version":1,"operation":"buffer-points","geometry":{"kind":"prepared","points":[{"x":"0.1234567890123456789","y":"0"}]},"radius_metres":"0"}'
    result = json.loads(session.call(request))["result"]
    assert "0.1234567890123456789" in result["geometry"]["value"]
    assert result["outward_error_metres"] == "0.1"
    assert len(result["certificate"]) > 64
    negative = json.loads(session.call(request.replace('"radius_metres":"0"', '"radius_metres":"-1"')))["result"]
    assert "EMPTY" in negative["geometry"]["value"]


def test_explicit_transform_grids_and_batch_refusals_use_the_rust_codec() -> None:
    profile = {
        "version": 1,
        "operations": [{
            "name": "http://example.org/identity",
            "source_crs": "http://example.org/source",
            "target_crs": "http://example.org/target",
            "chain": [{
                "source": {"realization": "00" * 32, "unit": "metres", "swapped_axes": False},
                "target": {"realization": "01" * 32, "unit": "metres", "swapped_axes": False},
                "model": {"law": "similarity2d-v1", "translation_metres": ["0", "0"],
                    "scale": "1", "rotation_degrees": "0", "convention": "position-vector",
                    "inverse": False},
            }],
        }],
    }
    session = purrdf.geo.GeoSession(json.dumps(profile))
    point = {"x": "1.2345678912", "y": "2.3456789123"}
    request = {"version": 1, "operation": "transform", "name": "http://example.org/identity",
        "point": point}
    ordinary = session.call(json.dumps(request))
    request["metric_decimal_places"] = 6
    assert session.call(json.dumps(request)) == ordinary
    request["metric_decimal_places"] = 9
    result = json.loads(session.call(json.dumps(request)))["result"]
    assert (result["x"], result["y"]) == ("1.234567891", "2.345678912")
    assert result["metric_decimal_places"] == 9
    batch = {"version": 1, "operation": "transform-batch", "name": request["name"],
        "points": [point, point], "metric_decimal_places": 9}
    assert json.loads(session.call(json.dumps(batch)))["result"] == [result, result]
    batch["metric_decimal_places"] = 5
    refused = json.loads(session.call(json.dumps(batch)))
    assert refused["error"]["code"] == "precision-exhausted"
    assert "result" not in refused
