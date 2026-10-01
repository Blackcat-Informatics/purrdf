// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Execute the actual worker against a synthetic recipient server: no network,
// Rust build or timing workload. Every failure must refuse offline activation.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createHash, webcrypto } from "node:crypto";
import vm from "node:vm";

const source = readFileSync(new URL("../sw.mjs", import.meta.url), "utf8");
const scope = "https://example.invalid/playground/";
const hash = (text) => createHash("sha256").update(text).digest("hex");
const baseNames = ["LICENSE-MIT", "LICENSE-APACHE", "LICENSE-MULAN", "CC-BY-4.0.txt", "NOTICE.txt", "third-party/component/NOTICE"];
const runtimeNames = ["COPYRIGHT-library.html", "licenses/MIT.txt", "licenses/Apache-2.0.txt"];

async function install(change = () => {}) {
  const server = new Map();
  const base = { schema: 1, profile: "npm", components: [{ notices: [baseNames.at(-1)] }], files: {} };
  const runtime = { schema: 1, compiler: "fixture compiler\n", files: {} };
  for (const [inventory, prefix, names] of [[base, "purrdf/licenses/", baseNames], [runtime, "purrdf/licenses/runtime/", runtimeNames]]) {
    for (const name of names) {
      const text = `complete fixture grant: ${name}`;
      inventory.files[name] = hash(text);
      server.set(prefix + name, text);
    }
  }
  server.set("purrdf/build-compiler.txt", runtime.compiler);
  change({ server, base, runtime });
  server.set("purrdf/licenses/inventory.json", JSON.stringify(base));
  server.set("purrdf/licenses/runtime/inventory.json", JSON.stringify(runtime));
  const stored = new Map();
  const requested = [];
  let activated = false;
  const events = {};
  const key = (url) => new URL(typeof url === "string" ? url : url.url || url.href, scope).href;
  const fetch = async (url) => {
    const path = key(url).slice(scope.length);
    requested.push(path);
    return server.has(path) ? new Response(server.get(path)) : new Response("missing", { status: 404 });
  };
  const cache = {
    put: async (url, response) => stored.set(key(url), await response.clone().text()),
    match: async (url) => stored.has(key(url)) ? new Response(stored.get(key(url))) : undefined,
    addAll: async (urls) => {
      // Before the first module is cached, every complete notice is present.
      for (const name of baseNames) assert.ok(stored.has(scope + "purrdf/licenses/" + name));
      for (const name of runtimeNames) assert.ok(stored.has(scope + "purrdf/licenses/runtime/" + name));
      for (const url of urls) stored.set(key(url), "shell fixture");
    },
  };
  vm.runInNewContext(source, {
    URL, crypto: webcrypto, fetch,
    caches: { open: async () => cache },
    self: {
      registration: { scope }, location: { origin: "https://example.invalid" },
      addEventListener: (name, handler) => { events[name] = handler; },
      skipWaiting: async () => { activated = true; },
    },
  });
  let promise;
  events.install({ waitUntil: (value) => { promise = value; } });
  let error;
  try { await promise; } catch (failure) { error = failure; }
  return { error, activated, stored, requested, events, cache };
}

test("offline installation retains both complete grants before the module", async () => {
  const result = await install();
  assert.equal(result.error, undefined);
  assert.equal(result.activated, true);
  assert.ok(result.stored.has(scope + "purrdf/pkg/purrdf_wasm_bg.wasm"));
  assert.ok(result.stored.has(scope + "purrdf/licenses/inventory.json"));
  assert.ok(result.stored.has(scope + "purrdf/licenses/runtime/inventory.json"));
});

for (const [name, change] of [
  ["missing controlling text", ({ server }) => server.delete("purrdf/licenses/LICENSE-MULAN")],
  ["corrupt base text", ({ server }) => server.set("purrdf/licenses/LICENSE-MIT", "corrupt")],
  ["missing runtime report", ({ server }) => server.delete("purrdf/licenses/runtime/COPYRIGHT-library.html")],
  ["corrupt runtime text", ({ server }) => server.set("purrdf/licenses/runtime/licenses/MIT.txt", "corrupt")],
  ["wrong profile", ({ base }) => { base.profile = "python"; }],
  ["stale compiler", ({ runtime }) => { runtime.compiler = "other compiler"; }],
  ["missing compiler record", ({ server }) => server.delete("purrdf/build-compiler.txt")],
  ["empty base inventory", ({ base }) => { base.files = {}; }],
  ["non-object inventory", ({ base }) => { base.files = 1; }],
  ["empty runtime inventory", ({ runtime }) => { runtime.files = {}; }],
  ["stripped controlling roster", ({ base }) => { delete base.files["LICENSE-MULAN"]; }],
  ["stripped component roster", ({ base }) => { delete base.files[baseNames.at(-1)]; }],
  ["traversal", ({ base }) => { base.files["../outside"] = hash("x"); }],
  ["encoded traversal", ({ base }) => { base.files["%2e%2e/outside"] = hash("x"); }],
  ["absolute foreign URL", ({ base }) => { base.files["https://other.invalid/grant"] = hash("x"); }],
  ["private path", ({ base }) => { base.files[".stage/notes"] = hash("x"); }],
]) {
  test(`offline installation refuses ${name}`, async () => {
    const result = await install(change);
    assert.ok(result.error, "must reject installation");
    assert.equal(result.activated, false);
    assert.equal(result.stored.has(scope + "purrdf/pkg/purrdf_wasm_bg.wasm"), false);
    assert.ok(result.requested.every((path) => !path.includes("outside") && !path.includes(".stage")));
  });
}

test("same-origin caller URLs are not added to the offline inventory", async () => {
  const result = await install();
  let promise;
  const request = { method: "GET", url: scope + "caller-data.ttl" };
  result.events.fetch({ request, respondWith: (value) => { promise = value; } });
  assert.equal((await promise).status, 404);
  assert.equal(result.stored.has(request.url), false);
});
