// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: These tests exercise JavaScript ownership/free methods and actual Promise query completion through the generated wasm host surface.

import { test } from "node:test";
import assert from "node:assert/strict";
import { ready, GeoSession, QueryEngine, Dataset } from "../index.mjs";
await ready();
const request = '{"version":1,"operation":"distance","a":{"longitude":"0","latitude":"0"},"b":{"longitude":"1","latitude":"0"}}';
const query = 'SELECT (<http://www.opengis.net/def/function/geosparql/metricDistance>("POINT(0 0)"^^<http://www.opengis.net/ont/geosparql#wktLiteral>,"POINT(1 0)"^^<http://www.opengis.net/ont/geosparql#wktLiteral>) AS ?distance) WHERE {}';

test("geographic scalar and synchronous/async query jobs share the Rust resolver", async () => {
  const session = new GeoSession();
  const record = JSON.parse(session.call(request));
  assert.equal(record.result.metres,"111319.490793");
  assert.ok(record.result.certificate.length>64);
  const engine = new QueryEngine();
  engine.setGeoSession(session);
  const dataset = new Dataset();
  assert.equal(JSON.parse(engine.queryRaw(dataset,query)).results.bindings[0].distance.value,"1.11319490793E5");
  const asynchronous = await engine.queryAsync(dataset,query);
  assert.equal(asynchronous.kind,"select");
  assert.equal(asynchronous.rows.toArray()[0].distance.value,"1.11319490793E5");
  asynchronous.free();
  engine.free(); dataset.free(); session.free();
});

test("immutable point index calls preserve content identity and typed refusal", () => {
  const session = new GeoSession();
  const index = session.pointIndex('{"version":1,"operation":"point-index","grid":"wgs84","level":2,"points":[{"key":"7","point":{"longitude":"0","latitude":"0"}}]}');
  const metadata = JSON.parse(index.call('{"version":1,"operation":"point-index"}'));
  const result = JSON.parse(index.call('{"version":1,"operation":"search-reported","center":{"longitude":"0","latitude":"0"},"threshold_metres":{"kind":"integer","value":"-1"}}'));
  assert.deepEqual(result.result.keys,[]);
  assert.equal(result.index,metadata.index);
  const invalid = JSON.parse(index.call('{"version":1,"operation":"search-reported","center":{"longitude":"0","latitude":"0"},"threshold_metres":{"kind":"double-bits","value":"7ff0000000000000"}}'));
  assert.equal(invalid.error.code,"nonfinite-threshold");
  assert.equal(Object.hasOwn(invalid,"result"),false);
  index.free(); session.free();
});


test("zero buffer exports preserve the exact original closure and certificate", () => {
  const session = new GeoSession();
  const request = '{"version":1,"operation":"buffer-points","geometry":{"kind":"prepared","points":[{"x":"0.1234567890123456789","y":"0"}]},"radius_metres":"0"}';
  const result = JSON.parse(session.call(request)).result;
  assert.ok(result.geometry.value.includes("0.1234567890123456789"));
  assert.equal(result.outward_error_metres,"0.1");
  assert.ok(result.certificate.length>64);
  const negative = JSON.parse(session.call(request.replace('"radius_metres":"0"','"radius_metres":"-1"'))).result;
  assert.ok(negative.geometry.value.includes("EMPTY"));
  session.free();
});

test("explicit transform quanta and complete batch refusal cross the shared codec", () => {
  const session = new GeoSession(JSON.stringify({
    version: 1,
    operations: [{
      name: "http://example.org/identity",
      source_crs: "http://example.org/source",
      target_crs: "http://example.org/target",
      chain: [{
        source: { realization: "00".repeat(32), unit: "metres", swapped_axes: false },
        target: { realization: "01".repeat(32), unit: "metres", swapped_axes: false },
        model: { law: "similarity2d-v1", translation_metres: ["0", "0"], scale: "1",
          rotation_degrees: "0", convention: "position-vector", inverse: false },
      }],
    }],
  }));
  const point = { x: "1.2345678912", y: "2.3456789123" };
  const scalar = { version: 1, operation: "transform", name: "http://example.org/identity", point };
  const ordinary = session.call(JSON.stringify(scalar));
  scalar.metric_decimal_places = 6;
  assert.equal(session.call(JSON.stringify(scalar)), ordinary);
  scalar.metric_decimal_places = 9;
  const result = JSON.parse(session.call(JSON.stringify(scalar))).result;
  assert.equal(result.x, "1.234567891");
  assert.equal(result.y, "2.345678912");
  assert.equal(result.metric_decimal_places, 9);
  const batch = { version: 1, operation: "transform-batch", name: scalar.name,
    points: [point, point], metric_decimal_places: 9 };
  assert.deepEqual(JSON.parse(session.call(JSON.stringify(batch))).result, [result, result]);
  batch.metric_decimal_places = 5;
  const refused = JSON.parse(session.call(JSON.stringify(batch)));
  assert.equal(refused.error.code, "precision-exhausted");
  assert.equal(Object.hasOwn(refused, "result"), false);
  session.free();
});
