// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: exercises the optimized WebAssembly package's exported handles, JavaScript exceptions and asynchronous profile snapshots.

import { test } from "node:test";
import assert from "node:assert/strict";
import { Dataset, GeoSession, QueryEngine, ready } from "../index.mjs";

await ready();

const DISTANCE = '{"version":1,"operation":"distance","a":{"longitude":"0","latitude":"0"},"b":{"longitude":"0","latitude":"0"}}';
const INDEX = '{"version":1,"operation":"point-index","grid":"wgs84","level":2,"points":[{"key":"7","point":{"longitude":"0","latitude":"0"}},{"key":"11","point":{"longitude":"1","latitude":"0"}}]}';
const SEARCH = '{"version":1,"operation":"search-physical","center":{"longitude":"0","latitude":"0"},"radius_metres":"0"}';
const NEGATIVE = '{"version":1,"operation":"search-reported","center":{"longitude":"0","latitude":"0"},"threshold_metres":{"kind":"integer","value":"-1"}}';
const AXES = '{"version":1,"references":[{"crs":"http://www.opengis.net/def/crs/EPSG/0/4326","ellipsoid":"wgs84","axes":"lat-lon"}]}';
const AXES_QUERY = `ASK { FILTER(<http://www.opengis.net/def/function/geosparql/distance>(
  "<http://www.opengis.net/def/crs/EPSG/0/4326> POINT(0 1)"^^<http://www.opengis.net/ont/geosparql#wktLiteral>,
  "POINT(1 0)"^^<http://www.opengis.net/ont/geosparql#wktLiteral>,
  <http://www.opengis.net/def/uom/OGC/1.0/metre>) = 0) }`;

test("native geographic response and strict semantic refusal cross the package root", () => {
  const session = new GeoSession();
  try {
    const text = session.call(DISTANCE);
    const response = JSON.parse(text);
    assert.equal(response.version, 1);
    assert.equal(response.result.micrometres, "0");
    assert.equal(session.call(DISTANCE), text);
    const refused = JSON.parse(session.call('{"version":1,"operation":"not-an-operation"}'));
    assert.equal(refused.error.code, "invalid-record");
    assert.deepEqual(refused.identity, response.identity);
  } finally { session.free(); }
});

test("an original work refusal throws instead of allocating a partial JSON response", () => {
  const session = new GeoSession('{"version":1,"limits":{"max_work_items":1}}');
  try {
    assert.throws(() => session.call(DISTANCE), /work/i);
  } finally { session.free(); }
});

test("point buckets and their profile outlive the creating session", () => {
  const session = new GeoSession();
  const index = session.pointIndex(INDEX);
  session.free();
  try {
    const first = index.call(SEARCH);
    assert.deepEqual(JSON.parse(first).result.keys, ["7"]);
    assert.equal(index.call(SEARCH), first);
    assert.deepEqual(JSON.parse(index.call(NEGATIVE)).result.keys, []);
  } finally { index.free(); }
});

test("explicit axes reach sync queries and an already started async profile snapshot", async () => {
  const dataset = Dataset.parse("", "ntriples");
  const engine = new QueryEngine();
  const session = new GeoSession(AXES);
  try {
    assert.throws(() => engine.ask(dataset, AXES_QUERY).boolean, /unregistered/i);
    assert.throws(() => engine.setGeoSession(), /expected instance of GeoSession/);
    engine.setGeoSession(session);
    session.free();
    assert.equal(engine.ask(dataset, AXES_QUERY).boolean, true);
    assert.throws(() => engine.setGeoProfile('{"version":1,"unknown":true}'));
    assert.equal(engine.ask(dataset, AXES_QUERY).boolean, true);
    const started = engine.askAsync(dataset, AXES_QUERY).then(result => result.boolean);
    engine.setGeoProfile('{"version":1}');
    assert.equal(await started, true);
    assert.throws(() => engine.ask(dataset, AXES_QUERY).boolean, /unregistered/i);
  } finally { engine.free(); dataset.free(); }
});
