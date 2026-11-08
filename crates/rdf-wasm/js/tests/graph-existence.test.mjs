// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: checks omitted JavaScript arguments and Promise/AbortSignal behavior through the optimized public WASM package.

import { test } from "node:test";
import assert from "node:assert/strict";
import { Dataset, QueryEngine, hasAsyncQueries, ready } from "../index.mjs";

await ready();

const G = "https://example.org/g";
const H = "https://example.org/h";
const TRIG = `<${G}> { <https://example.org/s> <https://example.org/p> <https://example.org/o> }`;
const modes = [undefined, true, false];
const parse = (mode) => mode === undefined
  ? Dataset.parse(TRIG, "trig")
  : Dataset.parse(TRIG, "trig", undefined, mode);
const graphs = (dataset) => JSON.parse(dataset.query("SELECT ?g WHERE { GRAPH ?g {} }").body)
  .results.bindings.map((row) => row.g.value).sort();

for (const mode of modes) {
  test(`graph lifetime ${String(mode)} survives reconstruction and synchronous UPDATE`, () => {
    const remember = mode !== false;
    const empty = mode === undefined ? new Dataset() : new Dataset(mode);
    assert.equal(empty.rememberEmptyGraphs, remember);
    empty.free();
    const engine = new QueryEngine();
    for (const governed of [false, true]) {
      const dataset = parse(mode);
      const snapshot = dataset.snapshot();
      const matched = dataset.match();
      assert.notEqual(snapshot.id, dataset.id);
      assert.equal(snapshot.generation, 0n);
      for (const rebuilt of [snapshot, matched]) {
        assert.equal(rebuilt.rememberEmptyGraphs, remember);
        const [quad] = rebuilt.quads();
        rebuilt.delete(quad);
        quad.free();
        assert.deepEqual(graphs(rebuilt), remember ? [G] : []);
        rebuilt.free();
      }
      const clear = `CLEAR GRAPH <${G}>`;
      if (governed) assert.equal(engine.updateGoverned(dataset, clear, { noCeiling: true }).isApplied, true);
      else assert.equal(engine.update(dataset, clear).dataset, dataset);
      assert.equal(dataset.rememberEmptyGraphs, remember);
      assert.deepEqual(graphs(dataset), remember ? [G] : []);
      engine.update(dataset, `CREATE GRAPH <${H}>`).dataset;
      assert.deepEqual(graphs(dataset), remember ? [G, H] : []);
      engine.update(dataset, "DROP NAMED").dataset;
      assert.deepEqual(graphs(dataset), []);
      if (remember) assert.throws(() => engine.update(dataset, `COPY GRAPH <${G}> TO GRAPH <${H}>`).dataset);
      else engine.update(dataset, `COPY GRAPH <${G}> TO GRAPH <${H}>`).dataset;
      engine.update(dataset, `CLEAR SILENT GRAPH <${G}>`).dataset;
      assert.deepEqual(graphs(dataset), []);
      dataset.free();
    }
    engine.free();
  });

  test(`graph lifetime ${String(mode)} survives actual async UPDATE and cancellation`, async () => {
    assert.equal(hasAsyncQueries(), true, "this acceptance requires JSPI, never a skipped async case");
    const remember = mode !== false;
    const engine = new QueryEngine();
    for (const governed of [false, true]) {
      const dataset = parse(mode);
      const id = dataset.id;
      const before = dataset.serialize("trig");
      const controller = new AbortController();
      let loads = 0;
      const options = {
        signal: controller.signal,
        resolveLoad: () => {
          loads += 1;
          controller.abort();
          return new Promise(() => {});
        },
      };
      const request = `CREATE GRAPH <${H}>; LOAD <https://example.org/document>`;
      if (governed) {
        const outcome = await engine.updateGovernedAsync(dataset, request, options);
        assert.equal(outcome.isApplied, false);
        assert.equal(outcome.tripped.cause, "cancelled");
      } else {
        await assert.rejects(engine.updateAsync(dataset, request, options).then(result => result.dataset), { name: "AbortError" });
      }
      assert.equal(loads, 1, "cancellation arrived in the real asynchronous effect");
      assert.equal(dataset.id, id);
      assert.equal(dataset.generation, 0n);
      assert.equal(dataset.serialize("trig"), before, "no CREATE prefix or rows were published");
      assert.equal(dataset.rememberEmptyGraphs, remember);
      const clear = `CLEAR GRAPH <${G}>`;
      if (governed) {
        const outcome = await engine.updateGovernedAsync(dataset, clear, { noCeiling: true });
        assert.equal(outcome.isApplied, true);
      } else assert.equal(await engine.updateAsync(dataset, clear).then(result => result.dataset), dataset);
      assert.equal(dataset.id, id);
      assert.equal(dataset.generation, 1n);
      assert.equal(dataset.rememberEmptyGraphs, remember);
      assert.deepEqual(graphs(dataset), remember ? [G] : []);
      await engine.updateAsync(dataset, `CREATE GRAPH <${H}>`).then(result => result.dataset);
      assert.deepEqual(graphs(dataset), remember ? [G, H] : []);
      assert.equal(dataset.generation, 2n);
      dataset.free();
    }
    engine.free();
  });
}
