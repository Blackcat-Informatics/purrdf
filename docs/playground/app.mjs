// SPDX-License-Identifier: MIT OR Apache-2.0
//
// The PurRDF console controller. Spawns the engine Web Worker, drives a
// promise-based request/reply map, wires every pane, renders results, and owns
// the aria-live error banner. The main thread NEVER touches the wasm engine —
// it is a pure view/message client over the worker.

import { VIGNETTES, findVignette, DEFAULT_STATE } from "./examples/gallery.mjs";
import { assertSarifVersion, describeSarif } from "./sarif.mjs";

const XSD_STRING = "http://www.w3.org/2001/XMLSchema#string";

// ---------------------------------------------------------------------------
// Worker request/reply plumbing
// ---------------------------------------------------------------------------

const worker = new Worker(new URL("./engine.worker.mjs", import.meta.url), {
  type: "module",
});

let nextId = 1;
const pending = new Map();

worker.addEventListener("message", (event) => {
  const { id, ok, result, error } = event.data ?? {};
  const entry = pending.get(id);
  if (!entry) return;
  pending.delete(id);
  if (ok) entry.resolve(result);
  else entry.reject(new Error(error));
});

worker.addEventListener("error", (event) => {
  showError(`worker crashed: ${event.message ?? event}`);
});

/** Send an op to the worker; resolves with its result or rejects on engine error. */
function call(op, args) {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    worker.postMessage({ id, op, args });
  });
}

// ---------------------------------------------------------------------------
// DOM helpers + banners
// ---------------------------------------------------------------------------

const $ = (id) => document.getElementById(id);

function showError(message) {
  const banner = $("error-banner");
  banner.textContent = message;
  banner.hidden = false;
}

function clearError() {
  const banner = $("error-banner");
  banner.textContent = "";
  banner.hidden = true;
}

function showStatus(message) {
  const banner = $("status-banner");
  banner.textContent = message;
  banner.hidden = false;
}

/** Run an async engine action, routing any failure to the visible error banner. */
async function guard(fn) {
  clearError();
  try {
    await fn();
  } catch (e) {
    showError(String(e?.message ?? e));
  }
}

function el(tag, props = {}, children = []) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "class") node.className = v;
    else if (k === "text") node.textContent = v;
    else if (k === "html") node.innerHTML = v;
    else if (k.startsWith("data-")) node.setAttribute(k, v);
    else if (k === "hidden") node.hidden = v;
    else node[k] = v;
  }
  for (const c of [].concat(children)) {
    if (c != null) node.append(c);
  }
  return node;
}

// ---------------------------------------------------------------------------
// Term rendering (for the SPARQL table + quad table)
// ---------------------------------------------------------------------------

/** Render a term descriptor into a human-readable N-Triples-ish label. */
function termLabel(t) {
  if (t == null) return "";
  switch (t.termType) {
    case "NamedNode":
      return `<${t.value}>`;
    case "BlankNode":
      return `_:${t.value}`;
    case "DefaultGraph":
      return "(default graph)";
    case "Variable":
      return `?${t.value}`;
    case "Literal": {
      let s = JSON.stringify(t.value);
      if (t.direction) s += `@${t.language || ""}--${t.direction}`;
      else if (t.language) s += `@${t.language}`;
      else if (t.datatype && t.datatype !== XSD_STRING) s += `^^<${t.datatype}>`;
      return s;
    }
    case "Quad":
      return `<<( ${termLabel(t.subject)} ${termLabel(t.predicate)} ${termLabel(t.object)} )>>`;
    default:
      return String(t.value ?? "");
  }
}

/** Convert an SRJ binding term (W3C shape) to our descriptor shape. */
function srjTermToDescriptor(b) {
  switch (b.type) {
    case "uri":
      return { termType: "NamedNode", value: b.value };
    case "bnode":
      return { termType: "BlankNode", value: b.value };
    case "literal":
    case "typed-literal":
      return {
        termType: "Literal",
        value: b.value,
        language: b["xml:lang"] || "",
        direction: b.direction || "",
        datatype: b.datatype || XSD_STRING,
      };
    case "triple":
      return {
        termType: "Quad",
        value: "",
        subject: srjTermToDescriptor(b.value.subject),
        predicate: srjTermToDescriptor(b.value.predicate),
        object: srjTermToDescriptor(b.value.object),
      };
    default:
      return { termType: "NamedNode", value: String(b.value ?? "") };
  }
}

// ---------------------------------------------------------------------------
// Clipboard + download utilities
// ---------------------------------------------------------------------------

async function copyText(text) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  // Capability fallback — a hidden textarea + execCommand (documented, not a swallow).
  const ta = el("textarea", { value: text, style: "position:fixed;opacity:0" });
  document.body.append(ta);
  ta.select();
  document.execCommand("copy");
  ta.remove();
}

function downloadText(filename, text, mime = "text/plain") {
  const blob = new Blob([text], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = el("a", { href: url, download: filename });
  document.body.append(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

// ---------------------------------------------------------------------------
// State collection + permalinks
// ---------------------------------------------------------------------------

let activePane = "input";

function collectState() {
  return {
    input: $("input-text").value,
    inputFormat: $("input-format").value,
    query: $("sparql-text").value,
    shapes: $("shapes-text").value,
    graphA: $("graph-a-text").value,
    graphAFormat: $("graph-a-format").value,
    graphB: $("graph-b-text").value,
    graphBFormat: $("graph-b-format").value,
    activePane,
  };
}

function applyState(state) {
  const s = { ...DEFAULT_STATE, ...state };
  $("input-text").value = s.input ?? "";
  $("input-format").value = s.inputFormat ?? "turtle";
  $("sparql-text").value = s.query ?? "";
  $("shapes-text").value = s.shapes ?? "";
  $("graph-a-text").value = s.graphA ?? "";
  $("graph-a-format").value = s.graphAFormat ?? "turtle";
  $("graph-b-text").value = s.graphB ?? "";
  $("graph-b-format").value = s.graphBFormat ?? "turtle";
  if (s.activePane) selectPane(s.activePane);
}

const b64urlEncode = (bytes) => {
  let bin = "";
  for (const byte of bytes) bin += String.fromCharCode(byte);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
};

const b64urlDecode = (s) => {
  const pad = s.length % 4 === 0 ? "" : "=".repeat(4 - (s.length % 4));
  const bin = atob(s.replace(/-/g, "+").replace(/_/g, "/") + pad);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
};

async function streamThrough(stream, bytes) {
  const writer = stream.writable.getWriter();
  writer.write(bytes);
  writer.close();
  const chunks = [];
  const reader = stream.readable.getReader();
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
  }
  let len = 0;
  for (const c of chunks) len += c.length;
  const out = new Uint8Array(len);
  let off = 0;
  for (const c of chunks) {
    out.set(c, off);
    off += c.length;
  }
  return out;
}

/** Encode state → (gzip when available) → base64url. All client-side, no network. */
async function encodeState(state) {
  const json = new TextEncoder().encode(JSON.stringify(state));
  if (typeof CompressionStream === "function") {
    const gz = await streamThrough(new CompressionStream("gzip"), json);
    return `g${b64urlEncode(gz)}`;
  }
  // Documented capability fallback: plain base64url of the JSON.
  return `j${b64urlEncode(json)}`;
}

async function decodeState(fragment) {
  if (!fragment) return null;
  const tag = fragment[0];
  const body = fragment.slice(1);
  const bytes = b64urlDecode(body);
  let jsonBytes = bytes;
  if (tag === "g") {
    if (typeof DecompressionStream !== "function") {
      throw new Error("this permalink is gzip-encoded but DecompressionStream is unavailable");
    }
    jsonBytes = await streamThrough(new DecompressionStream("gzip"), bytes);
  } else if (tag !== "j") {
    throw new Error("unrecognized permalink encoding");
  }
  return JSON.parse(new TextDecoder().decode(jsonBytes));
}

async function copyPermalink(state) {
  await guard(async () => {
    const fragment = await encodeState(state);
    const url = `${location.origin}${location.pathname}#${fragment}`;
    location.hash = fragment;
    await copyText(url);
    showStatus("Permalink copied to clipboard.");
  });
}

// ---------------------------------------------------------------------------
// Pane switching
// ---------------------------------------------------------------------------

const PANES = [
  "input",
  "roundtrip",
  "sparql",
  "shacl",
  "identity",
  "quads",
  "gallery",
];

function selectPane(pane) {
  if (!PANES.includes(pane)) return;
  activePane = pane;
  for (const p of PANES) {
    const section = $(`pane-${p}`);
    const tab = $(`tab-${p}`);
    if (section) section.hidden = p !== pane;
    if (tab) {
      tab.setAttribute("aria-selected", String(p === pane));
      tab.tabIndex = p === pane ? 0 : -1;
    }
  }
}

// ---------------------------------------------------------------------------
// Pane: Input
// ---------------------------------------------------------------------------

async function doParse() {
  await guard(async () => {
    const text = $("input-text").value;
    const format = $("input-format").value;
    const res = await call("parse", { text, format });
    $("input-summary").textContent = `Parsed ${res.size} quad(s) as ${format}.`;
    renderQuadTable(res.rows, res.size, null);
    showStatus(`Graph updated: ${res.size} quad(s).`);
  });
}

// ---------------------------------------------------------------------------
// Pane: Round-trip / differential
// ---------------------------------------------------------------------------

const SER_ORDER = ["turtle", "ntriples", "nquads", "trig", "rdfxml", "jsonld"];
const MIME = {
  turtle: "text/turtle",
  ntriples: "application/n-triples",
  nquads: "application/n-quads",
  trig: "application/trig",
  rdfxml: "application/rdf+xml",
  jsonld: "application/ld+json",
};
const EXT = {
  turtle: "ttl",
  ntriples: "nt",
  nquads: "nq",
  trig: "trig",
  rdfxml: "rdf",
  jsonld: "jsonld",
};

async function doSerializeAll() {
  await guard(async () => {
    const res = await call("serializeAll", {});
    const grid = $("serialize-output");
    grid.replaceChildren();
    const failedFormats = [];
    for (const f of SER_ORDER) {
      const entry = res.formats[f];
      const failed = entry.text == null;
      if (failed) failedFormats.push(f);
      const pre = el("pre", {
        id: `ser-${f}`,
        class: failed ? "code-out ser-error" : "code-out",
        text: failed
          ? `⚠ cannot serialize this graph to ${f}:\n${entry.error}`
          : entry.text,
        "data-format": f,
        "data-failed": String(failed),
      });
      let badge;
      if (failed) {
        badge = el("span", { class: "badge badge-warn", text: "unsupported" });
      } else if (entry.outputOnly) {
        badge = el("span", { class: "badge badge-info", text: "output only" });
      } else if (entry.roundtrips) {
        badge = el("span", { class: "badge badge-ok", text: "round-trips" });
      } else {
        badge = el("span", { class: "badge badge-warn", text: "degrades" });
      }
      const copyBtn = el("button", {
        type: "button",
        class: "btn-mini",
        text: "Copy",
        "data-copy": f,
        disabled: failed,
      });
      copyBtn.addEventListener("click", () =>
        guard(async () => {
          await copyText(entry.text);
          showStatus(`Copied ${f}.`);
        }),
      );
      const dlBtn = el("button", {
        type: "button",
        class: "btn-mini",
        text: "Download",
        "data-download": f,
        disabled: failed,
      });
      dlBtn.addEventListener("click", () =>
        downloadText(`graph.${EXT[f]}`, entry.text, MIME[f]),
      );
      const header = el("div", { class: "ser-head" }, [
        el("h4", { text: f, class: "ser-title" }),
        badge,
        el("span", { class: "spacer" }),
        copyBtn,
        dlBtn,
      ]);
      grid.append(el("div", { class: "ser-card" }, [header, pre]));
    }
    // Canonical form (RDFC-1.0) — the graph's deterministic identity.
    const canonCard = el("div", { class: "ser-card" }, [
      el("div", { class: "ser-head" }, [
        el("h4", { text: "canonical (RDFC-1.0 N-Quads)", class: "ser-title" }),
        el("span", { class: "badge badge-ok", text: "identity" }),
      ]),
      el("pre", { id: "ser-canonical", class: "code-out", text: res.canonical }),
    ]);
    grid.append(canonCard);

    const note = $("serialize-note");
    if (res.hasQuotedObject) {
      const survivors = SER_ORDER.filter(
        (f) => f !== "jsonld" && res.formats[f].text != null && res.formats[f].roundtrips,
      );
      const degraders = SER_ORDER.filter(
        (f) => f !== "jsonld" && res.formats[f].text != null && !res.formats[f].roundtrips,
      );
      note.textContent =
        "This graph carries a quoted triple in object position. " +
        (failedFormats.length
          ? `These formats cannot even encode it (hard-fail): ${failedFormats.join(", ")}. `
          : "") +
        (degraders.length
          ? `These text formats round-trip lossily: ${degraders.join(", ")}. `
          : "") +
        `N-Quads and the canonical RDFC-1.0 form are the reliable identity reference; ` +
        `round-tripping formats here: ${survivors.join(", ")}.`;
      note.hidden = false;
    } else {
      note.textContent =
        "No object-position quoted triple in this graph. N-Quads and the canonical " +
        "RDFC-1.0 form remain the deterministic identity reference." +
        (failedFormats.length ? ` Formats that hard-failed: ${failedFormats.join(", ")}.` : "");
      note.hidden = false;
    }
    if (failedFormats.length) {
      // HARD FAIL surfaced in the aria-live banner — never swallowed.
      showError(
        `Some serializers cannot encode this graph: ${failedFormats.join(", ")}. See each format card for the engine error.`,
      );
    } else {
      showStatus("Serialized to all formats.");
    }
  });
}

// ---------------------------------------------------------------------------
// Pane: SPARQL
// ---------------------------------------------------------------------------

async function doQuery() {
  await guard(async () => {
    const sparql = $("sparql-text").value;
    const res = await call("query", { sparql });
    const out = $("sparql-results");
    out.replaceChildren();
    if (res.kind === "srj") {
      renderSrj(res.srj, out);
    } else {
      out.append(
        el("pre", { id: "sparql-turtle", class: "code-out", text: res.turtle }),
      );
    }
    showStatus("Query complete.");
  });
}

function renderSrj(srj, container) {
  if (Object.hasOwn(srj, "boolean")) {
    container.append(
      el("p", {
        id: "sparql-boolean",
        class: "ask-result",
        text: `ASK → ${srj.boolean ? "true" : "false"}`,
      }),
    );
    return;
  }
  const vars = srj.head?.vars ?? [];
  const rows = srj.results?.bindings ?? [];
  const table = el("table", { id: "sparql-table", class: "data-table" });
  const thead = el("thead", {}, [
    el(
      "tr",
      {},
      vars.map((v) => el("th", { scope: "col", text: `?${v}` })),
    ),
  ]);
  const tbody = el("tbody");
  for (const binding of rows) {
    const tr = el(
      "tr",
      {},
      vars.map((v) => {
        const cell = binding[v];
        return el("td", {
          text: cell ? termLabel(srjTermToDescriptor(cell)) : "",
        });
      }),
    );
    tbody.append(tr);
  }
  table.append(thead, tbody);
  container.append(
    el("p", { class: "muted", text: `${rows.length} row(s).` }),
    table,
  );
}

// ---------------------------------------------------------------------------
// Pane: SHACL
// ---------------------------------------------------------------------------

let lastSarif = null;

async function doValidate() {
  await guard(async () => {
    const shapes = $("shapes-text").value;
    const res = await call("shaclValidate", { shapes });
    lastSarif = res.sarif;
    renderSarif(res.sarif);
    $("btn-download-sarif").disabled = false;
    const count = res.sarif.runs?.[0]?.results?.length ?? 0;
    showStatus(`SHACL validation produced ${count} result(s).`);
  });
}

function renderSarif(sarif) {
  const container = $("sarif-results");
  container.replaceChildren();
  const results = sarif.runs?.[0]?.results ?? [];
  // Assert the SARIF version contract. A drift is surfaced loudly (aria-live
  // error banner + an inline warning row), never silently echoed.
  const versionWarning = assertSarifVersion(sarif.version);
  if (versionWarning) {
    showError(versionWarning);
    container.append(
      el("p", { id: "sarif-version-warning", class: "ask-result", text: versionWarning }),
    );
  }
  container.append(
    el("p", {
      class: "muted",
      text: describeSarif(sarif),
    }),
  );
  if (results.length === 0) {
    container.append(
      el("p", { id: "sarif-conforms", class: "ask-result", text: "Conforms: no violations." }),
    );
    return;
  }
  const table = el("table", { id: "sarif-table", class: "data-table" });
  table.append(
    el("thead", {}, [
      el("tr", {}, [
        el("th", { scope: "col", text: "level" }),
        el("th", { scope: "col", text: "ruleId" }),
        el("th", { scope: "col", text: "message" }),
      ]),
    ]),
  );
  const tbody = el("tbody");
  for (const r of results) {
    tbody.append(
      el("tr", {}, [
        el("td", {}, [
          el("span", {
            class: `badge badge-${r.level === "error" ? "warn" : "info"}`,
            text: r.level ?? "",
          }),
        ]),
        el("td", { class: "mono", text: r.ruleId ?? "" }),
        el("td", { text: r.message?.text ?? "" }),
      ]),
    );
  }
  table.append(tbody);
  container.append(table);
}

async function doEntail() {
  await guard(async () => {
    const shapes = $("shapes-text").value;
    const res = await call("shaclEntail", { shapes });
    $("entail-output").textContent = res.ntriples;
    showStatus("Materialized SHACL-AF entailments.");
  });
}

// ---------------------------------------------------------------------------
// Pane: Graph identity / diff
// ---------------------------------------------------------------------------

async function doCompare() {
  await guard(async () => {
    const res = await call("identity", {
      aText: $("graph-a-text").value,
      aFormat: $("graph-a-format").value,
      bText: $("graph-b-text").value,
      bFormat: $("graph-b-format").value,
    });
    $("identity-result").textContent = `isomorphic: ${res.isomorphic ? "yes" : "no"}`;
    $("identity-result").className = `identity-verdict ${res.isomorphic ? "verdict-yes" : "verdict-no"}`;
    $("canon-a").textContent = res.canonicalA;
    $("canon-b").textContent = res.canonicalB;
    renderDiff(res.canonicalA, res.canonicalB);
    showStatus(`Compared graphs — isomorphic: ${res.isomorphic ? "yes" : "no"}.`);
  });
}

function renderDiff(a, b) {
  const container = $("canon-diff");
  container.replaceChildren();
  const linesA = a.split("\n");
  const linesB = new Set(b.split("\n"));
  const setA = new Set(linesA);
  const onlyA = linesA.filter((l) => l && !linesB.has(l));
  const onlyB = b.split("\n").filter((l) => l && !setA.has(l));
  if (onlyA.length === 0 && onlyB.length === 0) {
    container.append(
      el("p", { class: "ask-result", text: "Canonical forms are byte-identical." }),
    );
    return;
  }
  for (const l of onlyA) {
    container.append(el("div", { class: "diff-line diff-del", text: `- ${l}` }));
  }
  for (const l of onlyB) {
    container.append(el("div", { class: "diff-line diff-add", text: `+ ${l}` }));
  }
}

// ---------------------------------------------------------------------------
// Pane: Quad table (with click-to-filter via match())
// ---------------------------------------------------------------------------

function renderQuadTable(rows, size, filterInfo) {
  const container = $("quad-table");
  container.replaceChildren();
  $("filter-info").textContent = filterInfo
    ? `Filtered on ${filterInfo} — ${size} quad(s). `
    : `${size} quad(s).`;
  $("btn-clear-filter").hidden = !filterInfo;

  const table = el("table", { class: "data-table" });
  table.append(
    el("thead", {}, [
      el("tr", {}, [
        el("th", { scope: "col", text: "subject" }),
        el("th", { scope: "col", text: "predicate" }),
        el("th", { scope: "col", text: "object" }),
        el("th", { scope: "col", text: "graph" }),
      ]),
    ]),
  );
  const tbody = el("tbody");
  for (const row of rows) {
    tbody.append(
      el("tr", {}, [
        makeTermCell(row.subject, "subject"),
        makeTermCell(row.predicate, "predicate"),
        makeTermCell(row.object, "object"),
        makeTermCell(row.graph, "graph"),
      ]),
    );
  }
  table.append(tbody);
  container.append(table);
}

function makeTermCell(term, position) {
  const td = el("td");
  const label = termLabel(term);
  if (term && term.termType !== "DefaultGraph") {
    const btn = el("button", {
      type: "button",
      class: "term-link",
      text: label,
      title: `Filter where ${position} = ${label}`,
    });
    btn.addEventListener("click", () => filterByTerm(position, term));
    td.append(btn);
  } else {
    td.textContent = label;
  }
  return td;
}

async function filterByTerm(position, term) {
  await guard(async () => {
    const args = { subject: null, predicate: null, object: null, graph: null };
    args[position] = term;
    const res = await call("match", args);
    renderQuadTable(res.rows, res.size, `${position} = ${termLabel(term)}`);
    selectPane("quads");
  });
}

async function refreshQuads() {
  await guard(async () => {
    const res = await call("quads", {});
    renderQuadTable(res.rows, res.size, null);
  });
}

// ---------------------------------------------------------------------------
// Pane: Gallery
// ---------------------------------------------------------------------------

function buildGallery() {
  const list = $("gallery-list");
  list.replaceChildren();
  for (const v of VIGNETTES) {
    const loadBtn = el("button", {
      type: "button",
      class: "btn",
      text: "Load & run",
      "data-load": v.id,
    });
    loadBtn.addEventListener("click", () => loadVignette(v.id));

    const permaBtn = el("button", {
      type: "button",
      class: "btn-mini",
      text: "Copy permalink",
      "data-permalink": v.id,
    });
    permaBtn.addEventListener("click", () => copyPermalink(vignetteState(v)));

    const item = el("li", { class: "gallery-item", id: `vignette-${v.id}` }, [
      el("h3", { text: v.title }),
      el("p", { class: "muted", text: v.blurb }),
      el("div", { class: "gallery-actions" }, [loadBtn, permaBtn]),
    ]);
    list.append(item);
  }
}

function vignetteState(v) {
  const s = { ...DEFAULT_STATE };
  s.input = v.input;
  s.inputFormat = v.inputFormat;
  if (v.query != null) s.query = v.query;
  if (v.shapes != null) s.shapes = v.shapes;
  // Vignettes that exercise identity supply their own A/B graphs; others
  // compare the loaded input against the supplied graph B.
  s.graphA = v.input;
  s.graphAFormat = v.inputFormat;
  if (v.graphB != null) {
    s.graphB = v.graphB;
    s.graphBFormat = v.graphBFormat ?? "turtle";
  }
  s.activePane = v.activePane ?? "input";
  return s;
}

async function loadVignette(id) {
  const v = findVignette(id);
  if (!v) {
    showError(`unknown vignette: ${id}`);
    return;
  }
  applyState(vignetteState(v));
  await runActiveForState(v);
}

/** After loading a vignette, run the engine action its pane demonstrates. */
async function runActiveForState(v) {
  await doParse();
  switch (v.activePane) {
    case "roundtrip":
      await doSerializeAll();
      break;
    case "sparql":
      await doQuery();
      break;
    case "shacl":
      await doValidate();
      await doEntail();
      break;
    case "identity":
      await doCompare();
      break;
    case "quads":
      await refreshQuads();
      break;
    default:
      break;
  }
}

// ---------------------------------------------------------------------------
// Wiring + boot
// ---------------------------------------------------------------------------

function wireTabs() {
  for (const p of PANES) {
    const tab = $(`tab-${p}`);
    if (!tab) continue;
    tab.addEventListener("click", () => selectPane(p));
    tab.addEventListener("keydown", (e) => {
      const idx = PANES.indexOf(p);
      if (e.key === "ArrowRight" || e.key === "ArrowDown") {
        e.preventDefault();
        const next = PANES[(idx + 1) % PANES.length];
        selectPane(next);
        $(`tab-${next}`).focus();
      } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
        e.preventDefault();
        const prev = PANES[(idx - 1 + PANES.length) % PANES.length];
        selectPane(prev);
        $(`tab-${prev}`).focus();
      }
    });
  }
}

function wireButtons() {
  $("btn-parse").addEventListener("click", doParse);
  $("btn-serialize-all").addEventListener("click", doSerializeAll);
  $("btn-run-query").addEventListener("click", doQuery);
  $("btn-validate").addEventListener("click", doValidate);
  $("btn-entail").addEventListener("click", doEntail);
  $("btn-download-sarif").addEventListener("click", () => {
    if (lastSarif) {
      downloadText("report.sarif", JSON.stringify(lastSarif, null, 2), "application/json");
    }
  });
  $("btn-compare").addEventListener("click", doCompare);
  $("btn-refresh-quads").addEventListener("click", refreshQuads);
  $("btn-clear-filter").addEventListener("click", refreshQuads);
  $("btn-permalink").addEventListener("click", () => copyPermalink(collectState()));
}

async function initHeader() {
  // The version handshake is the ONLY startup traffic; it goes over the worker
  // message port, not the network. The console makes NO network request after
  // its assets load — that provable property (no server-side evaluation) is
  // worth more than a cosmetic wasm-size chip, so the size probe is gone.
  await guard(async () => {
    const { version } = await call("version", {});
    $("app-version").textContent = `purrdf v${version}`;
  });
}

function registerServiceWorker() {
  if (!("serviceWorker" in navigator)) return;
  window.addEventListener("load", () => {
    navigator.serviceWorker
      .register(new URL("./sw.mjs", import.meta.url), { type: "module" })
      .catch((e) => showStatus(`offline cache unavailable: ${e.message ?? e}`));
  });
}

async function boot() {
  wireTabs();
  wireButtons();
  buildGallery();

  // Restore permalink state if present; otherwise the default console.
  let restored = false;
  if (location.hash.length > 1) {
    try {
      const state = await decodeState(location.hash.slice(1));
      if (state) {
        applyState(state);
        restored = true;
      }
    } catch (e) {
      showError(`could not restore permalink: ${e.message ?? e}`);
    }
  }
  if (!restored) applyState(DEFAULT_STATE);

  await initHeader();
  // Parse the initial input so every pane has a live graph to work with.
  await doParse();

  registerServiceWorker();
}

boot();
