// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
'use strict';
const assert = require('node:assert/strict');
const fixture = require(process.argv[2]);
const dataset = fixture.__purrdf_test_large_dataset_identity();
assert.equal(typeof dataset.id, 'bigint');
assert.equal(typeof dataset.generation, 'bigint');
assert.equal(dataset.id, 9_007_199_254_740_993n);
assert.equal(dataset.generation, 9_007_199_254_740_995n);
const engine = new fixture.QueryEngine();
engine.update(dataset, 'INSERT DATA { <https://example.org/s> <https://example.org/p> <https://example.org/o> }');
assert.equal(dataset.id, 9_007_199_254_740_993n);
assert.equal(dataset.generation, 9_007_199_254_740_996n);
assert.equal(JSON.stringify({ id: dataset.id.toString() }), '{"id":"9007199254740993"}');
dataset.free();
engine.free();
console.log('Dataset wasm getters preserve bigint identities above 2^53 and mutation advances exactly');

const firstId = 9_007_199_254_740_992n;
const secondId = firstId + 1n;
const first = fixture.__purrdf_test_open_exchange(firstId);
const second = fixture.__purrdf_test_open_exchange(secondId);
assert.equal(typeof first.exchangeId, 'bigint');
assert.equal(first.exchangeId, firstId);
assert.equal(second.exchangeId, secondId);
const ids = new Map([[first.exchangeId, first], [second.exchangeId, second]]);
assert.equal(ids.size, 2, 'adjacent identities never alias in a host exchange map');
for (const invalid of [Number(firstId), 0n, -1n, (1n << 64n) + firstId, '9007199254740992']) {
  assert.equal(fixture.AsyncJob.exchangeIsOpen(invalid), false);
  assert.equal(fixture.AsyncJob.deliverExchangeBindings(invalid, new Uint8Array()), fixture.DeliveryStatus.Finished);
  assert.equal(fixture.AsyncJob.deliverExchangeFailure(invalid, 'transport', 'invalid'), fixture.DeliveryStatus.Finished);
  assert.equal(fixture.AsyncJob.faultExchange(invalid, 'invalid'), fixture.DeliveryStatus.Finished);
  assert.equal(fixture.AsyncJob.exchangeIsOpen(firstId), true);
  assert.equal(fixture.AsyncJob.exchangeIsOpen(secondId), true);
}
assert.equal(fixture.AsyncJob.deliverExchangeBindings(second.exchangeId, new Uint8Array()), fixture.DeliveryStatus.Accepted);
assert.equal(fixture.AsyncJob.exchangeIsOpen(secondId), false);
assert.equal(fixture.AsyncJob.exchangeIsOpen(firstId), true);
assert.equal(fixture.AsyncJob.deliverExchangeFailure(first.exchangeId, 'transport', 'down'), fixture.DeliveryStatus.Accepted);
const maximum = (1n << 64n) - 1n;
const last = fixture.__purrdf_test_open_exchange(maximum);
assert.equal(last.exchangeId, maximum);
for (const invalid of [-1n, (1n << 64n) + maximum]) {
  assert.equal(fixture.AsyncJob.faultExchange(invalid, 'invalid'), fixture.DeliveryStatus.Finished);
  assert.equal(fixture.AsyncJob.exchangeIsOpen(maximum), true, 'negative or oversized input never wraps to MAX');
}
assert.equal(fixture.AsyncJob.faultExchange(last.exchangeId, 'fixture fault'), fixture.DeliveryStatus.Accepted);
assert.equal(fixture.__purrdf_test_exchange_terminal(true), maximum);
for (let attempt = 0; attempt < 2; ++attempt) {
  assert.throws(() => fixture.__purrdf_test_exchange_terminal(false), { code: 'native-sparql-exchange-id-exhausted' });
}
first.free();
second.free();
last.free();
console.log('Exchange wasm bigint identities settle exactly above 2^53, reject wrapping inputs and refuse exhaustion');
