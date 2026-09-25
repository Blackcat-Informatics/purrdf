// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// The package README's two federation recipes, EXECUTED. The fenced examples tagged
// `js resolve-service-recipe` (a plain `fetch`-based `resolveService`) and
// `js worker-recipe` (a complete Cloudflare Worker) are pulled out of `../README.md`
// verbatim and run against the optimized wasm module. The only rewrites are the ones a
// bundler or workerd performs: the package specifiers resolve to this directory, and the
// Worker's `.wasm` import becomes the compiled `WebAssembly.Module` workerd hands it.
// A missing fence fails the test, so the README cannot lose a recipe silently.
//
// No network is touched: `fetch`, the Worker's service binding, `caches.default` and
// `ctx` are deterministic doubles that RECORD what they were handed, so the assertions
// observe the request the recipe actually sent. The book's JavaScript chapter carries the
// same recipes; any tagged fence there must be byte-identical to the README's, so the
// execution here covers both.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const HERE = new URL("../", import.meta.url);
const README = new URL("README.md", HERE);
const BOOK_PAGE = new URL("../../../docs/book/src/getting-started/javascript.md", HERE);
const WASM_PATH = fileURLToPath(new URL("pkg/purrdf_wasm_bg.wasm", HERE));

const REMOTE = "https://remote.example.org/sparql";
const QUERY = `SELECT ?s ?x WHERE { ?s <https://example.org/p> ?o SERVICE <${REMOTE}> { ?o <https://example.org/q> ?x } }`;
const SILENT_QUERY = QUERY.replace("SERVICE <", "SERVICE SILENT <");

/** The body of every ```` ```js <tag> ```` fence in `markdown`. */
function fences(markdown, tag) {
  const pattern = new RegExp("^```js " + tag + "\\n([\\s\\S]*?)^```$", "gm");
  return [...markdown.matchAll(pattern)].map((match) => match[1]);
}

/** The one fence tagged `tag` in the README; missing or duplicated is a failure. */
function readmeFence(tag) {
  const found = fences(readFileSync(README, "utf8"), tag);
  assert.equal(
    found.length,
    1,
    `README.md must carry exactly one \`\`\`js ${tag} fence (found ${found.length})`,
  );
  return found[0];
}

/** Resolve the package specifiers the way the published package's `exports` map does. */
function localized(source) {
  const at = (path) => JSON.stringify(new URL(path, HERE).href);
  return source
    .replaceAll('"@blackcatinformatics/purrdf/cloudflare"', at("cloudflare.mjs"))
    .replaceAll('"@blackcatinformatics/purrdf"', at("index.mjs"));
}

/** Import `source` as an ES module; each call is a distinct module instance. */
let generation = 0;
async function importSource(source) {
  generation += 1;
  const tagged = `${source}\n// docs-worker-recipe instance ${generation}\n`;
  return import(`data:text/javascript;base64,${Buffer.from(tagged).toString("base64")}`);
}

/** SPARQL Results JSON joining `?o` (the local object) to `?x`. */
function remoteAnswer() {
  return JSON.stringify({
    head: { vars: ["o", "x"] },
    results: {
      bindings: [
        {
          o: { type: "uri", value: "https://example.org/o1" },
          x: { type: "literal", value: "joined" },
        },
      ],
    },
  });
}

/** What the recipe's `fetch` (or the Worker's binding) received, normalized. */
function recorded(input, init) {
  const headers = new Headers(init?.headers);
  return {
    url: String(input instanceof Request ? input.url : input),
    method: init?.method,
    contentType: headers.get("Content-Type"),
    accept: headers.get("Accept"),
    body: init?.body,
  };
}

test("the README and the book say the remote endpoint's CORS policy governs a browser host", () => {
  for (const url of [README, BOOK_PAGE]) {
    const text = readFileSync(url, "utf8");
    const corsLines = text.split("\n").filter((line) => line.includes("CORS"));
    assert.ok(corsLines.length > 0, `${fileURLToPath(url)} never mentions CORS`);
    const prose = text.replace(/\s+/g, " ");
    assert.match(
      prose,
      /In a browser, the remote endpoint's CORS policy governs whether `fetch` can read its answer/,
      `${fileURLToPath(url)} must say the remote endpoint's CORS policy governs browser hosts`,
    );
  }
});

test("the book's recipes are the README's, byte for byte", () => {
  const book = readFileSync(BOOK_PAGE, "utf8");
  for (const tag of ["resolve-service-recipe", "worker-recipe"]) {
    const inBook = fences(book, tag);
    assert.equal(inBook.length, 1, `the book's JavaScript chapter must carry one ${tag} fence`);
    assert.equal(inBook[0], readmeFence(tag), `the book's ${tag} fence differs from the README's`);
  }
});

test("the resolveService recipe joins the remote rows, and reports a network failure as transport", async () => {
  const source = readmeFence("resolve-service-recipe");
  const sent = [];
  let network = "up";
  const printed = [];
  const originalFetch = globalThis.fetch;
  const originalLog = console.log;
  globalThis.fetch = async (input, init) => {
    sent.push(recorded(input, init));
    if (network === "down") throw new TypeError("fetch failed: the network is down");
    return new Response(remoteAnswer(), {
      headers: { "Content-Type": "application/sparql-results+json" },
    });
  };
  let recipe;
  try {
    console.log = (...args) => printed.push(args.join(" "));
    recipe = await importSource(
      `${localized(source)}\nexport { resolveService, engine, dataset };\n`,
    );
  } finally {
    console.log = originalLog;
  }

  try {
    // The recipe itself ran its query and printed the joined row.
    assert.deepEqual(printed, ["https://example.org/a joined"]);
    assert.equal(sent.length, 1);
    assert.equal(sent[0].url, REMOTE);
    assert.equal(sent[0].method, "POST");
    assert.equal(sent[0].contentType, "application/sparql-query");
    assert.equal(sent[0].accept, "application/sparql-results+json");
    assert.match(sent[0].body, /<https:\/\/example\.org\/q>/);

    const { resolveService, engine, dataset } = recipe;
    const joined = (await engine.selectAsync(dataset, QUERY, { resolveService })).rows
      .toArray()
      .map((row) => [row.s.value, row.x?.value]);
    assert.deepEqual(joined, [["https://example.org/a", "joined"]]);

    // A network rejection comes back from the handler as a typed transport failure …
    network = "down";
    const failure = await resolveService(
      {
        kind: "service",
        endpoint: REMOTE,
        queryText: "SELECT * WHERE { ?s ?p ?o }",
        accept: "application/sparql-results+json",
        contentType: "application/sparql-query",
        userAgent: "docs-test",
        timeoutMs: 5_000,
        headers: [],
      },
      { signal: new AbortController().signal, silent: false },
    );
    assert.equal(failure.kind, "transport");
    assert.match(failure.message, /the network is down/);

    // … which fails a plain SERVICE …
    await assert.rejects(engine.selectAsync(dataset, QUERY, { resolveService }), (error) => {
      assert.ok(error instanceof Error);
      assert.match(error.message, /the network is down/);
      return true;
    });

    // … and is the join identity under SERVICE SILENT: the local row, unextended — which
    // differs from the joined row above, so a swallowed success cannot pass for it.
    const silent = (await engine.selectAsync(dataset, SILENT_QUERY, { resolveService })).rows
      .toArray()
      .map((row) => [row.s.value, row.x?.value]);
    assert.deepEqual(silent, [["https://example.org/a", undefined]]);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("the Worker recipe answers a SPARQL Protocol request with the joined results", async () => {
  const source = readmeFence("worker-recipe");
  const wasmImport = 'import wasm from "@blackcatinformatics/purrdf/purrdf_wasm_bg.wasm";';
  assert.ok(source.includes(wasmImport), "the Worker recipe imports the wasm module");
  const runnable = localized(
    source.replace(
      wasmImport,
      `const wasm = new WebAssembly.Module(readFileSync(${JSON.stringify(WASM_PATH)}));\n` +
        'import { readFileSync } from "node:fs";',
    ),
  );

  const stored = new Map();
  const originalCaches = globalThis.caches;
  globalThis.caches = {
    default: {
      async match(request) {
        return stored.get(request.url)?.clone();
      },
      async put(request, response) {
        stored.set(request.url, response);
      },
    },
  };
  try {
    const worker = (await importSource(runnable)).default;
    const sent = [];
    const env = {
      REMOTE: {
        async fetch(input, init) {
          sent.push(recorded(input, init));
          return new Response(remoteAnswer(), {
            headers: { "Content-Type": "application/sparql-results+json" },
          });
        },
      },
    };
    const pending = [];
    const ctx = { waitUntil: (promise) => pending.push(promise) };
    const request = () =>
      new Request(`https://worker.example.org/sparql?query=${encodeURIComponent(QUERY)}`, {
        headers: { Accept: "application/sparql-results+json" },
      });

    for (const round of [1, 2]) {
      const response = await worker.fetch(request(), env, ctx);
      await Promise.all(pending.splice(0));
      assert.equal(response.status, 200, `round ${round}`);
      assert.match(response.headers.get("Content-Type"), /^application\/sparql-results\+json/);
      assert.equal(response.headers.get("Access-Control-Allow-Origin"), "*");
      assert.match(response.headers.get("Server-Timing"), /eval;dur=/);
      const body = JSON.parse(await response.text());
      assert.deepEqual(body.head.vars, ["s", "x"]);
      assert.deepEqual(
        body.results.bindings.map((row) => [row.s.value, row.x.value]),
        [["https://example.org/a", "joined"]],
        `round ${round}`,
      );
    }

    // The first round went through the service binding; the second was answered from
    // the cache the recipe configured.
    assert.equal(sent.length, 1);
    assert.equal(sent[0].url, REMOTE);
    assert.equal(sent[0].method, "POST");
    assert.equal(sent[0].contentType, "application/sparql-query");
    assert.equal(sent[0].accept, "application/sparql-results+json");
    assert.equal(stored.size, 1);
  } finally {
    if (originalCaches === undefined) delete globalThis.caches;
    else globalThis.caches = originalCaches;
  }
});
