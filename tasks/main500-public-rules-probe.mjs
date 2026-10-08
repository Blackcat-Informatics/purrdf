// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: this bounded acceptance probe exercises the actual exported JavaScript package, synchronous throws, asynchronous rejections and generated WASM-owned result objects.
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';

assert.ok(process.argv[2]?.startsWith('/'), 'pass the absolute combined package index.mjs');
const entry = process.argv[2];
const api = await import(pathToFileURL(entry).href);
for (const name of ['ready', 'shaclEntail', 'shaclEntailAsync', 'shaclApplyRules',
  'shaclApplyRulesAsync', 'entailCheckGoldenVectors']) {
  assert.equal(typeof api[name], 'function', `actual public export ${name} required`);
}
await api.ready();

const prefixes = '@prefix ex: <http://example.org/> .\n@prefix sh: <http://www.w3.org/ns/shacl#> .\n';
const data = '<http://example.org/n> <http://example.org/p> "ab" .\n' +
  '<http://example.org/m> <http://example.org/p> "aa" .\n';
const quote = text => JSON.stringify(text);
const shapes = (pattern, sparql) => {
  const construct = `CONSTRUCT { $this <http://example.org/hit> ?v } WHERE { $this <http://example.org/p> ?v FILTER(REGEX(?v, ${quote(pattern)})) }`;
  const rule = sparql
    ? `[ a sh:SPARQLRule ; sh:construct ${quote(construct)} ]`
    : `[ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:hit ; sh:object [ sh:path ex:p ] ; sh:condition [ sh:property [ sh:path ex:p ; sh:pattern ${quote(pattern)} ] ] ]`;
  return `${prefixes}ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:p ; sh:rule ${rule} .`;
};
const srl = pattern => `PREFIX : <http://example.org/>\nRULE { ?s :hit ?v } WHERE { ?s :p ?v FILTER(REGEX(?v, ${quote(pattern)})) }`;
const claims = text => {
  assert.equal(typeof text, 'string');
  const lines = text.split('\n').filter(line => line !== '');
  assert.equal(new Set(lines).size, lines.length, 'duplicate claims are forbidden');
  return lines.sort();
};
const expected = values => values.map(value =>
  `<http://example.org/${value === 'ab' ? 'n' : 'm'}> <http://example.org/hit> ${quote(value)} .`).sort();
const baseClaims = claims(data);
const routes = [
  { name: 'triple-entail', materialized: true, source: pattern => shapes(pattern, false), door: 'entail' },
  { name: 'sparql-entail', materialized: true, source: pattern => shapes(pattern, true), door: 'entail' },
  { name: 'triple-apply', materialized: false, source: pattern => shapes(pattern, false), door: 'apply' },
  { name: 'sparql-apply', materialized: false, source: pattern => shapes(pattern, true), door: 'apply' },
  { name: 'srl-apply', materialized: false, source: srl, door: 'srl' },
];
const invoke = (route, pattern, law, asynchronous) => {
  if (route.door === 'entail') {
    const args = [route.source(pattern), data, ...Array(8).fill(undefined)];
    return asynchronous
      ? api.shaclEntailAsync(...args, { xpathRegex: law })
      : api.shaclEntail(...args, law);
  }
  const args = [data, route.door === 'srl' ? undefined : route.source(pattern),
    route.door === 'srl' ? route.source(pattern) : undefined, ...Array(10).fill(undefined)];
  return asynchronous
    ? api.shaclApplyRulesAsync(...args, { xpathRegex: law })
    : api.shaclApplyRules(...args, law);
};
let passed = 0;
const check = async (label, action) => {
  await action();
  passed += 1;
  console.log(`PASS ${label}`);
};
const success = async (route, pattern, law, asynchronous, values) => {
  const result = await invoke(route, pattern, law, asynchronous);
  assert.ok(result && typeof result.free === 'function', 'actual WASM-owned result required');
  try {
    assert.deepEqual(claims(route.materialized ? result.ntriples : result.inferred),
      route.materialized ? [...baseClaims, ...expected(values)].sort() : expected(values));
    assert.deepEqual(result.diagnostics, [], 'no extra shape diagnostics');
  } finally {
    result.free();
  }
};
const refused = async (route, pattern, law, asynchronous) => {
  let error;
  try {
    const unexpected = await invoke(route, pattern, law, asynchronous);
    unexpected?.free();
  } catch (caught) { error = caught; }
  assert.ok(error instanceof Error, 'resource refusal must throw/reject Error, not return a dataset');
  assert.equal(error.code, 'xpath-pattern-bytes');
  assert.match(error.message, /xpath-pattern-bytes/);
};

for (const [law, noncapturing, backreference] of [
  [undefined, ['ab'], []],
  ['xpath-2.0-2010-12-14', [], ['aa']],
  ['xpath-3.1-2017-03-21', ['ab'], ['aa']],
]) {
  for (const route of routes) {
    for (const asynchronous of [false, true]) {
      const label = `${route.name} ${asynchronous ? 'async' : 'sync'} ${law ?? 'compatibility'}`;
      for (const [name, pattern, values] of [
        ['noncapturing', '(?:a)b', noncapturing],
        ['backreference', '^(a)\\1$', backreference],
      ]) {
        await check(`${label} ${name}`, () => success(route, pattern, law, asynchronous, values));
      }
      if (law === undefined) continue;
      const at = 'a'.repeat(65536);
      const over = 'a'.repeat(65537);
      assert.equal(Buffer.byteLength(at), 65536);
      assert.equal(Buffer.byteLength(over), 65537);
      await check(`${label} PatternBytes-over`, () => refused(route, over, law, asynchronous));
      await check(`${label} PatternBytes-at`, () => success(route, at, law, asynchronous, []));
    }
  }
}
await check('actual exported entailCheckGoldenVectors', () => api.entailCheckGoldenVectors());
assert.equal(passed, 101, 'all registered route/law/boundary checks must execute');
console.log(JSON.stringify({ entry, passed, failed: 0, skipped: 0 }));
