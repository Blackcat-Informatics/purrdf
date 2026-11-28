// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: Locale changes apply to browser DOM labels and preserve live focus.

const staticEnglish = new WeakMap();
const dynamic = Symbol("localized UI");
const attributes = [
  ["data-zh-text", (node) => node.textContent, (node, text) => { node.textContent = text; }],
  // Only source-authored HTML in the shell uses this pair, never result data.
  ["data-zh-html", (node) => node.innerHTML, (node, html) => { node.innerHTML = html; }],
  ["data-zh-label", (node) => node.getAttribute("aria-label"), (node, text) => node.setAttribute("aria-label", text)],
  ["data-zh-placeholder", (node) => node.placeholder, (node, text) => { node.placeholder = text; }],
  ["data-zh-description", (node) => node.content, (node, text) => { node.content = text; }],
];

export const selectedLocale = () => document.documentElement.lang;

/** Older links and unsupported locale values use the English default. */
export const normalizeLocale = (value) => value === "zh-Hans" ? value : "en";

/** Share presentation preferences without adding them to gallery or worker inputs. */
export const withLocale = (state, locale) => ({ ...state, locale: normalizeLocale(locale) });

/** Keep raw browser details separate from the interface's description. */
export function interfaceFailure(english, chinese, error) {
  const detail = String(error?.message ?? error);
  const detailZh = error instanceof InterfaceError ? error.chinese : detail;
  return new InterfaceError(`${english}: ${detail}`, `${chinese}：${detailZh}`);
}

/** Describe the same serializer outcomes without translating format names. */
export function roundTripNote(quotedObject, failed, degraders = [], survivors = []) {
  const refused = failed.join(", ");
  if (!quotedObject) {
    return [
      "No object-position quoted triple in this graph. N-Quads and the canonical " +
        "line-sequence form remain the deterministic identity reference." +
        (refused ? ` Formats that hard-failed: ${refused}.` : ""),
      "此图的宾语位置没有所引用三元组。N-Quads 和规范化逐行形式仍可作为确定性图标识的依据。" +
        (refused ? `无法编码此图的格式：${refused}。` : ""),
    ];
  }
  const lossy = degraders.join(", ");
  const lossless = survivors.join(", ");
  return [
    "This graph carries a quoted triple in object position. " +
      (refused ? `These formats cannot even encode it (hard-fail): ${refused}. ` : "") +
      (lossy ? `These text formats round-trip lossily: ${lossy}. ` : "") +
      "N-Quads and the canonical line-sequence form are the reliable identity reference; " +
      `round-tripping formats here: ${lossless}.`,
    "此图的宾语位置包含所引用三元组。" +
      (refused ? `无法编码此图的格式：${refused}。` : "") +
      (lossy ? `往返转换有损的文本格式：${lossy}。` : "") +
      "N-Quads 和规范化逐行形式可作为可靠的图标识依据。" +
      `可无损往返转换的格式：${lossless}。`,
  ];
}

export function sarifWarningLabels(english, actual, expected) {
  return [english, `SARIF 版本不受支持：${actual ?? "（无）"}；控制台要求 SARIF ${expected}。`];
}

/** Translations belong to view nodes, so language changes do not rerun actions. */
export function localize(node, english, chinese, property = "textContent") {
  if (!node[dynamic]) {
    node[dynamic] = new Map();
    node.setAttribute("data-localized", "");
  }
  node[dynamic].set(property, [english, chinese]);
  node[property] = selectedLocale() === "zh-Hans" ? chinese : english;
}

/** A translated wrapper for UI failures, never for engine diagnostics. */
export class InterfaceError extends Error {
  constructor(english, chinese) {
    super(english);
    this.chinese = chinese;
  }
}

/** Missing and unsupported permalink values both use the English default. */
export function setLocale(value) {
  const locale = normalizeLocale(value);
  const chinese = locale === "zh-Hans";
  document.documentElement.lang = locale;
  document.getElementById("interface-language").value = locale;
  for (const [attribute, read, write] of attributes) {
    for (const node of document.querySelectorAll(`[${attribute}]`)) {
      if (!staticEnglish.has(node)) staticEnglish.set(node, new Map());
      const values = staticEnglish.get(node);
      if (!values.has(attribute)) values.set(attribute, read(node));
      write(node, chinese ? node.getAttribute(attribute) : values.get(attribute));
    }
  }
  for (const node of document.querySelectorAll("[data-localized]")) {
    for (const [property, [en, zh]] of node[dynamic]) node[property] = chinese ? zh : en;
  }
}
