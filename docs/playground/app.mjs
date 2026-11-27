// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//
// The PurRDF console controller. Spawns the engine Web Worker, drives a
// promise-based request/reply map, wires every pane, renders results, and owns
// the aria-live error banner. The main thread NEVER touches the wasm engine —
// it is a pure view/message client over the worker.

import {
  VIGNETTES, findVignette, DEFAULT_STATE, matchesBundledMuseumQuery, vignetteState,
} from "./examples/gallery.mjs";
import {
  InterfaceError, interfaceFailure, localize, roundTripNote, sarifWarningLabels,
  selectedLocale, setLocale, withLocale,
} from "./locale.mjs";
import { decodeState, encodeState } from "./permalink.mjs";
import { EXPECTED_SARIF_VERSION, assertSarifVersion, describeSarif } from "./sarif.mjs";

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
  const { id, ok, result, error, canonicalizationJson, nativeCauseJson, code } = event.data ?? {};
  const entry = pending.get(id);
  if (!entry) return;
  pending.delete(id);
  if (ok) entry.resolve(result);
  else entry.reject(Object.assign(new Error(error), { canonicalizationJson, nativeCauseJson, code }));
});

worker.addEventListener("error", (event) => {
  showError(interfaceFailure("worker crashed", "工作线程异常终止", event));
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

function showError(error) {
  const banner = $("error-banner");
  const message = String(error?.message ?? error);
  const detail = el("span", { id: "error-message", text: message });
  if (error instanceof InterfaceError) localize(detail, message, error.chinese);
  banner.replaceChildren(detail);
  if (typeof error?.code === "string") {
    banner.append(" ", el("code", { id: "error-code", text: error.code }));
  }
  banner.hidden = false;
}

function clearError() {
  const banner = $("error-banner");
  banner.textContent = "";
  banner.hidden = true;
}

function showStatus(message, chinese = message) {
  const banner = $("status-banner");
  localize(banner, message, chinese);
  banner.hidden = false;
}

/** Run an async engine action, routing any failure to the visible error banner. */
async function guard(fn) {
  clearError();
  try {
    return await fn();
  } catch (e) {
    showError(e);
  }
}

function el(tag, props = {}, children = []) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "class") node.className = v;
    else if (k === "text") node.textContent = v;
    else if (k === "localeText") localize(node, ...v);
    else if (k === "localeTitle") localize(node, ...v, "title");
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
    try {
      await navigator.clipboard.writeText(text);
    } catch (error) {
      throw interfaceFailure("could not copy to clipboard", "无法复制到剪贴板", error);
    }
    return;
  }
  // Capability fallback — a hidden textarea + execCommand (documented, not a swallow).
  const ta = el("textarea", { value: text, style: "position:fixed;opacity:0" });
  document.body.append(ta);
  try {
    ta.select();
    document.execCommand("copy");
  } catch (error) {
    throw interfaceFailure("could not copy to clipboard", "无法复制到剪贴板", error);
  } finally {
    ta.remove();
  }
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
    canonicalSearch: $("canonical-search").value,
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
  $("canonical-search").value = s.canonicalSearch ?? "";
  $("sparql-text").value = s.query ?? "";
  $("shapes-text").value = s.shapes ?? "";
  $("graph-a-text").value = s.graphA ?? "";
  $("graph-a-format").value = s.graphAFormat ?? "turtle";
  $("graph-b-text").value = s.graphB ?? "";
  $("graph-b-format").value = s.graphBFormat ?? "turtle";
  if (s.activePane) selectPane(s.activePane);
}

async function copyPermalink(getState) {
  await guard(async () => {
    const locale = selectedLocale();
    const fragment = await encodeState(withLocale(await getState(), locale));
    const url = `${location.origin}${location.pathname}#${fragment}`;
    location.hash = fragment;
    await copyText(url);
    showStatus("Permalink copied to clipboard.", "已将永久链接复制到剪贴板。");
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

let latestParse;

async function doParse() {
  const request = { text: $("input-text").value, format: $("input-format").value };
  latestParse = request;
  return guard(async () => {
    const res = await call("parse", request);
    localize($("input-summary"), `Parsed ${res.size} quad(s) as ${request.format}.`,
      `已按 ${request.format} 格式解析出 ${res.size} 个四元组。`);
    renderQuadTable(res.rows, res.size, null);
    showStatus(`Graph updated: ${res.size} quad(s).`, `图已更新，共 ${res.size} 个四元组。`);
    return request;
  });
}

// ---------------------------------------------------------------------------
// Pane: Round-trip / differential
// ---------------------------------------------------------------------------

const SER_ORDER = [
  "turtle",
  "ntriples",
  "nquads",
  "trig",
  "rdfxml",
  "jsonld",
  "yamlld",
];
const MIME = {
  turtle: "text/turtle",
  ntriples: "application/n-triples",
  nquads: "application/n-quads",
  trig: "application/trig",
  rdfxml: "application/rdf+xml",
  jsonld: "application/ld+json",
  yamlld: "application/ld+yaml",
};
const EXT = {
  turtle: "ttl",
  ntriples: "nt",
  nquads: "nq",
  trig: "trig",
  rdfxml: "rdf",
  jsonld: "jsonld",
  yamlld: "yamlld",
};

async function doSerializeAll() {
  await guard(async () => {
    const res = await call("serializeAll", { canonicalSearch: $("canonical-search").value });
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
        text: failed ? entry.error : entry.text,
        "data-format": f,
        "data-failed": String(failed),
      });
      let badge;
      if (failed) {
        badge = el("span", { class: "badge badge-warn", localeText: ["unsupported", "不支持"] });
      } else if (entry.roundtripError) {
        badge = el("span", { class: "badge badge-warn", localeText: ["comparison refused", "无法比较"], title: entry.roundtripError.error });
      } else if (entry.roundtrips) {
        badge = el("span", { class: "badge badge-ok", localeText: ["round-trips", "可无损往返"] });
      } else {
        badge = el("span", { class: "badge badge-warn", localeText: ["degrades", "往返有损"] });
      }
      const copyBtn = el("button", {
        type: "button",
        class: "btn-mini",
        localeText: ["Copy", "复制"],
        "data-copy": f,
        disabled: failed,
      });
      copyBtn.addEventListener("click", () =>
        guard(async () => {
          await copyText(entry.text);
          showStatus(`Copied ${f}.`, `已复制 ${f}。`);
        }),
      );
      const dlBtn = el("button", {
        type: "button",
        class: "btn-mini",
        localeText: ["Download", "下载"],
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
      const code = failed ? entry.errorCode : entry.roundtripError?.code;
      const diagnosticCode = typeof code === "string"
        ? el("code", { class: "mono", text: code })
        : null;
      const refusal = failed
        ? el("p", { class: "muted", localeText: [
          `⚠ cannot serialize this graph to ${f}:`, `⚠ 无法将此图序列化为 ${f}：`,
        ] })
        : null;
      const comparison = entry.roundtripError
        ? el("pre", { class: "code-out ser-error", text: entry.roundtripError.error })
        : null;
      grid.append(el("div", { class: "ser-card" }, [header, refusal, pre, comparison, diagnosticCode]));
    }
    // Canonical form (the canonical line sequence) — the graph's deterministic identity.
    const canonCard = el("div", { class: "ser-card" }, [
      el("div", { class: "ser-head" }, [
        el("h4", { localeText: ["canonical (line-sequence N-Quads)", "规范形式（逐行 N-Quads）"], class: "ser-title" }),
        el("span", { class: "badge badge-ok", localeText: ["identity", "标识"] }),
      ]),
      el("pre", { id: "ser-canonical", class: "code-out", text: res.canonical }),
    ]);
    grid.append(canonCard);

    const note = $("serialize-note");
    if (res.hasQuotedObject) {
      const survivors = SER_ORDER.filter(
        (f) => res.formats[f].text != null && res.formats[f].roundtrips,
      );
      const degraders = SER_ORDER.filter(
        (f) => res.formats[f].text != null && !res.formats[f].roundtrips,
      );
      localize(note, ...roundTripNote(true, failedFormats, degraders, survivors));
    } else {
      localize(note, ...roundTripNote(false, failedFormats));
    }
    note.hidden = false;
    if (failedFormats.length) {
      // HARD FAIL surfaced in the aria-live banner — never swallowed.
      showError(
        new InterfaceError(
          `Some serializers cannot encode this graph: ${failedFormats.join(", ")}. See each format card for the engine error.`,
          `部分序列化格式无法编码此图：${failedFormats.join(", ")}。各格式卡片显示引擎错误。`,
        ),
      );
    } else {
      showStatus("Serialized to all formats.", "已序列化为所有格式。");
    }
  });
}

// ---------------------------------------------------------------------------
// Pane: SPARQL
// ---------------------------------------------------------------------------

let queryStarted = false;

async function doQuery() {
  queryStarted = true;
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
    showStatus("Query complete.", "查询完成。");
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
    el("p", { class: "muted", localeText: [`${rows.length} row(s).`, `共 ${rows.length} 行。`] }),
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
    showStatus(`SHACL validation produced ${count} result(s).`, `SHACL 验证得到 ${count} 条结果。`);
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
    const labels = sarifWarningLabels(versionWarning, sarif.version, EXPECTED_SARIF_VERSION);
    showError(new InterfaceError(...labels));
    container.append(
      el("p", { id: "sarif-version-warning", class: "ask-result", localeText: labels }),
    );
  }
  container.append(
    el("p", {
      class: "muted",
      localeText: [describeSarif(sarif), `SARIF ${sarif?.version ?? "（无）"} · 共 ${results.length} 条结果。`],
    }),
  );
  if (results.length === 0) {
    container.append(
      el("p", { id: "sarif-conforms", class: "ask-result", localeText: ["Conforms: no violations.", "符合形状，无违规。"] }),
    );
    return;
  }
  const table = el("table", { id: "sarif-table", class: "data-table" });
  table.append(
    el("thead", {}, [
      el("tr", {}, [
        el("th", { scope: "col", localeText: ["level", "级别"] }),
        el("th", { scope: "col", localeText: ["ruleId", "规则标识"] }),
        el("th", { scope: "col", localeText: ["message", "消息"] }),
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
    const res = await call("shaclEntail", { shapes, canonicalSearch: $("canonical-search").value });
    $("entail-output").textContent = res.ntriples;
    showStatus(
      res.diagnostics.length === 0
        ? "Materialized SHACL-AF entailments."
        : `Materialized SHACL-AF entailments; shapes graph diagnostics: ${res.diagnostics.join("; ")}.`,
      res.diagnostics.length === 0
        ? "已物化 SHACL-AF 蕴涵。"
        : `已物化 SHACL-AF 蕴涵。形状图诊断：${res.diagnostics.join("; ")}`,
    );
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
      canonicalSearch: $("canonical-search").value,
    });
    localize($("identity-result"), `isomorphic: ${res.isomorphic ? "yes" : "no"}`,
      res.isomorphic ? "同构" : "不同构");
    $("identity-result").className = `identity-verdict ${res.isomorphic ? "verdict-yes" : "verdict-no"}`;
    $("canon-a").textContent = res.canonicalA;
    $("canon-b").textContent = res.canonicalB;
    renderDiff(res.canonicalA, res.canonicalB);
    showStatus(`Compared graphs — isomorphic: ${res.isomorphic ? "yes" : "no"}.`,
      res.isomorphic ? "图比较完成：同构。" : "图比较完成：不同构。");
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
      el("p", { class: "ask-result", localeText: ["Canonical forms are byte-identical.", "规范形式逐字节相同。"] }),
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

const TERM_POSITIONS_ZH = { subject: "主语", predicate: "谓语", object: "宾语", graph: "图" };

function renderQuadTable(rows, size, filterInfo) {
  const container = $("quad-table");
  container.replaceChildren();
  localize($("filter-info"),
    filterInfo
      ? `Filtered on ${filterInfo.position} = ${filterInfo.label} — ${size} quad(s). `
      : `${size} quad(s).`,
    filterInfo
      ? `筛选条件：${TERM_POSITIONS_ZH[filterInfo.position]} = ${filterInfo.label}，共 ${size} 个四元组。`
      : `共 ${size} 个四元组。`,
  );
  $("btn-clear-filter").hidden = !filterInfo;

  const table = el("table", { class: "data-table" });
  table.append(
    el("thead", {}, [
      el("tr", {}, [
        el("th", { scope: "col", localeText: ["subject", "主语"] }),
        el("th", { scope: "col", localeText: ["predicate", "谓语"] }),
        el("th", { scope: "col", localeText: ["object", "宾语"] }),
        el("th", { scope: "col", localeText: ["graph", "图"] }),
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
      localeTitle: [`Filter where ${position} = ${label}`, `筛选 ${TERM_POSITIONS_ZH[position]} = ${label}`],
    });
    btn.addEventListener("click", () => filterByTerm(position, term));
    td.append(btn);
  } else {
    if (term?.termType === "DefaultGraph") localize(td, label, "（默认图）");
    else td.textContent = label;
  }
  return td;
}

async function filterByTerm(position, term) {
  await guard(async () => {
    const args = { subject: null, predicate: null, object: null, graph: null };
    args[position] = term;
    const res = await call("match", args);
    renderQuadTable(res.rows, res.size, { position, label: termLabel(term) });
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

let latestGalleryLoad;
let editorRevision = 0;

function buildGallery() {
  const list = $("gallery-list");
  list.replaceChildren();
  for (const v of VIGNETTES) {
    const loadBtn = el("button", {
      type: "button",
      class: "btn",
      localeText: ["Load & run", "加载并运行"],
      "data-load": v.id,
    });
    loadBtn.addEventListener("click", () => loadVignette(v.id));

    const permaBtn = el("button", {
      type: "button",
      class: "btn-mini",
      localeText: ["Copy permalink", "复制永久链接"],
      "data-permalink": v.id,
    });
    permaBtn.addEventListener("click", () => copyPermalink(() => vignetteState(v)));

    const item = el("li", { class: "gallery-item", id: `vignette-${v.id}` }, [
      el("h3", { localeText: [v.title, v.zhTitle] }),
      el("p", { class: "muted", localeText: [v.blurb, v.zhBlurb] }),
      el("div", { class: "gallery-actions" }, [loadBtn, permaBtn]),
    ]);
    list.append(item);
  }
}

async function loadVignette(id) {
  const v = findVignette(id);
  if (!v) {
    showError(new InterfaceError(`unknown vignette: ${id}`, `未知示例：${id}`));
    return;
  }
  const request = Symbol();
  const revision = editorRevision;
  latestGalleryLoad = request;
  const isCurrent = () => latestGalleryLoad === request && editorRevision === revision;
  await guard(async () => {
    let state;
    try {
      state = await vignetteState(v);
    } catch (error) {
      // A slower, superseded fetch must not replace the current banner.
      if (isCurrent()) throw error;
      return;
    }
    if (!isCurrent()) return;
    applyState(state);
    await runActiveForState(v, isCurrent);
  });
}

/** After loading a vignette, run the engine action its pane demonstrates. */
async function runActiveForState(v, isCurrent) {
  await doParse();
  if (!isCurrent()) return;
  switch (v.activePane) {
    case "roundtrip":
      await doSerializeAll();
      break;
    case "sparql":
      await doQuery();
      break;
    case "shacl":
      await doValidate();
      if (isCurrent()) await doEntail();
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
  $("interface-language").addEventListener("change", (event) => setLocale(event.target.value));
  // A gallery load replaces these fields, so a later edit wins even if the
  // bundled fixture is still being fetched. Keep the gallery selection token
  // separately so an edit never re-enables permalink startup after a click.
  const noteEdit = () => { editorRevision++; };
  for (const id of [
    "input-text", "canonical-search", "sparql-text", "shapes-text", "graph-a-text", "graph-b-text",
  ]) {
    $(id).addEventListener("input", noteEdit);
  }
  for (const id of ["input-format", "graph-a-format", "graph-b-format"]) {
    $(id).addEventListener("change", noteEdit);
  }
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
  $("btn-permalink").addEventListener("click", () => copyPermalink(collectState));
}

async function initHeader() {
  // The version handshake goes over the worker message port, not the network.
  // The gallery can fetch bundled assets, but evaluation runs entirely in the worker.
  await guard(async () => {
    const { version } = await call("version", {});
    $("app-version").textContent = `purrdf v${version}`;
  });
}

function registerServiceWorker() {
  if (!("serviceWorker" in navigator)) return;
  // The first worker parse can finish after window.load has already fired.
  navigator.serviceWorker
    .register(new URL("./sw.mjs", import.meta.url), { type: "module" })
    .catch((e) => {
      const detail = e.message ?? e;
      showStatus(`offline cache unavailable: ${detail}`, `离线缓存不可用：${detail}`);
    });
}

async function boot() {
  wireTabs();
  wireButtons();
  buildGallery();

  // Restore permalink state if present; otherwise the default console.
  let restored;
  let restoreError;
  if (location.hash.length > 1) {
    try {
      const state = await decodeState(location.hash.slice(1));
      if (state) {
        setLocale(state.locale);
        applyState(state);
        restored = state;
      }
    } catch (e) {
      restoreError = interfaceFailure("could not restore permalink", "无法恢复永久链接", e);
    }
  }
  if (!restored) {
    setLocale("en");
    applyState(DEFAULT_STATE);
  }

  await initHeader();
  // Parse the initial input so every pane has a live graph to work with.
  const parsed = await doParse();
  if (restoreError && $("error-banner").hidden) showError(restoreError);

  registerServiceWorker();

  // The general query worker has no startup deadline or result bound. Auto-run
  // only the known example, and leave the Run button in charge of other links.
  if (!restored || !parsed || !stillAtRestoredQuery(restored, parsed)) return;
  try {
    if (await matchesBundledMuseumQuery(restored) && stillAtRestoredQuery(restored, parsed)) {
      await doQuery();
    }
  } catch (error) {
    if (stillAtRestoredQuery(restored, parsed)) {
      showError(interfaceFailure("could not verify bundled museum example", "无法验证内置博物馆示例", error));
    }
  }
}

function stillAtRestoredQuery(state, parsed) {
  return !queryStarted && !latestGalleryLoad && latestParse === parsed &&
    activePane === "sparql" && state.activePane === "sparql" &&
    parsed.text === state.input && parsed.format === state.inputFormat &&
    $("input-text").value === parsed.text && $("input-format").value === parsed.format &&
    $("sparql-text").value === state.query;
}

boot();
