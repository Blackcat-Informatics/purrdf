// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: this bounded acceptance probe drives the actual published JavaScript package entry and its thrown/rejected Error properties.
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';

assert.ok(process.argv[2]?.startsWith('/'), 'pass the absolute current package-root index.mjs path');
const entry = process.argv[2];
const { ready, Dataset, QueryEngine, shaclValidateToSarif, shaclValidateToSarifAsync } =
  await import(pathToFileURL(entry).href);
await ready();
const engine = new QueryEngine();
const dataset = Dataset.parse('<http://example.org/s> <http://example.org/p> "aa" .\n', 'ntriples');
const laws = [
  [undefined, true, false],
  ['xpath-2.0-2010-12-14', false, true],
  ['xpath-3.1-2017-03-21', true, true],
];
const ask = (value, pattern, flags = '') =>
  `ASK { FILTER(REGEX(${JSON.stringify(value)}, ${JSON.stringify(pattern)}, ${JSON.stringify(flags)})) }`;
const shapes = (pattern, flags = '') =>
  `@prefix sh: <http://www.w3.org/ns/shacl#> . <http://example.org/S> a sh:NodeShape; sh:targetNode <http://example.org/s>; sh:property [sh:path <http://example.org/p>; sh:pattern ${JSON.stringify(pattern)}; sh:flags ${JSON.stringify(flags)}] .`;
const conforms = text => {
  const document = JSON.parse(text);
  assert.equal(document.version, '2.1.0');
  assert.equal(document.runs.length, 1);
  return document.runs[0].properties.shaclConforms;
};
const syncValidation = (pattern, law, flags = '') =>
  shaclValidateToSarif(shapes(pattern, flags), '<http://example.org/s> <http://example.org/p> "aa" .\n',
    undefined, undefined, undefined, undefined, undefined, undefined, law);
const asyncValidation = (pattern, law, flags = '') =>
  shaclValidateToSarifAsync(shapes(pattern, flags), '<http://example.org/s> <http://example.org/p> "aa" .\n',
    undefined, undefined, undefined, undefined, undefined, undefined, { xpathRegex: law });
let passed = 0;
const check = async (label, action) => {
  await action();
  passed += 1;
  console.log(`PASS ${label}`);
};
const refusal = async action => {
  let error;
  try { await action(); } catch (caught) { error = caught; }
  assert.ok(error instanceof Error, 'must throw/reject an actual JavaScript Error, not publish an answer/report');
  assert.equal(error.code, 'xpath-pattern-bytes');
  assert.match(error.message, /xpath-pattern-bytes/);
};
for (const [law, noncapturing, backreference] of laws) {
  for (const [label, pattern, value, expected] of [
    ['noncapturing', '(?:a)b', 'ab', noncapturing],
    ['backreference', '^(a)\\1$', 'aa', backreference],
  ]) {
    await check(`query-sync ${law ?? 'compatibility'} ${label}`, () =>
      assert.equal(engine.ask(dataset, ask(value, pattern), { xpathRegex: law }), expected));
    await check(`query-async ${law ?? 'compatibility'} ${label}`, async () =>
      assert.equal(await engine.askAsync(dataset, ask(value, pattern), { xpathRegex: law }), expected));
    // SHACL's fixed data is aa: the noncapturing pattern tests aa instead of ab.
    const shapePattern = label === 'noncapturing' ? '(?:a)a' : pattern;
    await check(`shacl-sync ${law ?? 'compatibility'} ${label}`, () =>
      assert.equal(conforms(syncValidation(shapePattern, law)), expected));
    await check(`shacl-async ${law ?? 'compatibility'} ${label}`, async () =>
      assert.equal(conforms(await asyncValidation(shapePattern, law)), expected));
  }
  if (law === undefined) continue;
  const replaceQuery = `SELECT (REPLACE("aa", ${JSON.stringify('^(a)\\1$')}, "b") AS ?r) WHERE {}`;
  await check(`replace-sync ${law}`, () => {
    const result = engine.select(dataset, replaceQuery, { xpathRegex: law });
    assert.equal(result.rowCount, 1);
    assert.deepEqual(result.rows.toArray().map(row => row.r.value), ['b']);
  });
  await check(`replace-async ${law}`, async () => {
    const result = await engine.selectAsync(dataset, replaceQuery, { xpathRegex: law });
    assert.equal(result.rowCount, 1);
    assert.deepEqual(result.rows.toArray().map(row => row.r.value), ['b']);
  });
  const admitted = 'a' + ' '.repeat(65535);
  const withheld = 'a' + ' '.repeat(65536);
  assert.equal(Buffer.byteLength(admitted), 65536);
  assert.equal(Buffer.byteLength(withheld), 65537);
  // Exact source-neighbor inputs normalize to the same small x-flag pattern.
  await check(`query-sync source-refusal ${law}`, () => refusal(() =>
    engine.ask(dataset, ask('a', withheld, 'x'), { xpathRegex: law })));
  await check(`query-sync source-neighbor ${law}`, () =>
    assert.equal(engine.ask(dataset, ask('a', admitted, 'x'), { xpathRegex: law }), true));
  await check(`query-async source-refusal ${law}`, () => refusal(() =>
    engine.askAsync(dataset, ask('a', withheld, 'x'), { xpathRegex: law })));
  await check(`query-async source-neighbor ${law}`, async () =>
    assert.equal(await engine.askAsync(dataset, ask('a', admitted, 'x'), { xpathRegex: law }), true));
  await check(`shacl-sync source-refusal ${law}`, () => refusal(() => syncValidation(withheld, law, 'x')));
  await check(`shacl-sync source-neighbor ${law}`, () => assert.equal(conforms(syncValidation(admitted, law, 'x')), true));
  await check(`shacl-async source-refusal ${law}`, () => refusal(() => asyncValidation(withheld, law, 'x')));
  await check(`shacl-async source-neighbor ${law}`, async () => assert.equal(conforms(await asyncValidation(admitted, law, 'x')), true));
}
for (const invalid of ['xpath-3.1', 'XPATH-3.1-2017-03-21', '']) {
  for (const [label, action] of [
    ['sync', () => engine.ask(dataset, 'ASK {}', { xpathRegex: invalid })],
    ['async', () => engine.askAsync(dataset, 'ASK {}', { xpathRegex: invalid })],
  ]) {
    await check(`unknown-name ${label} ${JSON.stringify(invalid)}`, async () => {
      let error;
      try { await action(); } catch (caught) { error = caught; }
      assert.ok(error instanceof Error);
      assert.match(error.message, /xpath-2\.0-2010-12-14/);
      assert.match(error.message, /xpath-3\.1-2017-03-21/);
    });
  }
}
console.log(JSON.stringify({ entry, passed, failed: 0, skipped: 0 }));
