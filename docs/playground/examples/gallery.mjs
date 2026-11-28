// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//
// Curated RDF-1.2 vignettes for the PurRDF console. Fixtures use example.org
// identifiers and provide their own vocabulary alongside standard terms.
// Each vignette is a complete console state:
// selecting one loads the Input/SPARQL/SHACL/graph-B panes and runs it.

/** @typedef {{
 *   id: string,
 *   title: string,
 *   blurb: string,
 *   zhTitle: string,
 *   zhBlurb: string,
 *   input?: string,
 *   inputFormat: string,
 *   query?: string,
 *   files?: { input: string, query: string },
 *   shapes?: string,
 *   graphB?: string,
 *   graphBFormat?: string,
 *   activePane?: string,
 * }} Vignette */

import { InterfaceError, interfaceFailure } from "../locale.mjs";

const RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/** @type {Vignette[]} */
export const VIGNETTES = [
  {
    id: "museum-attributions",
    zhTitle: "博物馆藏品（中文标签）",
    zhBlurb: "两份资料对莲纹碗的年代记载不同。RDF 1.2 注解将各个日期与其来源配对。",
    title: "Museum collection (Chinese labels)",
    blurb:
      "Two sources disagree on the date of a lotus-pattern bowl. " +
      "RDF 1.2 annotations keep each date paired with its source.",
    inputFormat: "turtle",
    files: { input: "./museum/collection.ttl", query: "./museum/attributions.rq" },
    activePane: "sparql",
  },
  {
    id: "quoted-triple",
    zhTitle: "所引用三元组的具体化",
    zhBlurb: "三元组项位于宾语位置。ex:alice 陈述 «ex:bob ex:likes ex:carol» 并记录其溯源信息。",
    title: "Quoted-triple reification",
    blurb:
      "An RDF-1.2 triple term in object position: ex:alice asserts the statement " +
      "«ex:bob ex:likes ex:carol» and stamps it with provenance.",
    inputFormat: "turtle",
    input: `@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# The object of ex:says is itself a triple term (RDF-1.2 «<<( s p o )>>»).
ex:alice ex:says <<( ex:bob ex:likes ex:carol )>> .
ex:alice ex:statedOn "2026-07-06"^^xsd:date .
ex:carol ex:name "Carol" .
`,
    query: `PREFIX ex: <http://example.org/>
SELECT ?who ?claim WHERE {
  ?who ex:says ?claim .
}`,
    activePane: "roundtrip",
  },
  {
    id: "directional-literals",
    zhTitle: "带书写方向的字面量（ltr 和 rtl）",
    zhBlurb: "两个字面量均显式指定基础方向：英语从左到右，阿拉伯语从右到左。",
    title: "Directional literals (ltr + rtl)",
    blurb:
      "RDF-1.2 base-direction literals: an English left-to-right string and an " +
      "Arabic right-to-left string, each carrying an explicit base direction.",
    inputFormat: "turtle",
    input: `@prefix ex: <http://example.org/> .

ex:greetingEn ex:text "Hello, world"@en--ltr .
ex:greetingAr ex:text "مرحبا بالعالم"@ar--rtl .
`,
    query: `PREFIX ex: <http://example.org/>
SELECT ?subject ?text WHERE {
  ?subject ex:text ?text .
}`,
    activePane: "quads",
  },
  {
    id: "shacl-af-rule",
    zhTitle: "SHACL-AF 的 sh:rule 蕴涵",
    zhBlurb: "SHACL-AF 的三元组规则推导每位 ex:Person 的 ex:adult 属性为 ex:yes。验证可查看 SARIF 报告，物化可显示推导出的三元组。",
    title: "SHACL-AF sh:rule entailment",
    blurb:
      "A SHACL Advanced-Features triple rule: every ex:Person is entailed to be " +
      "ex:adult ex:yes. Validate for the SARIF report, then Materialize to see " +
      "the inferred triple.",
    inputFormat: "turtle",
    input: `@prefix ex: <http://example.org/> .
@prefix rdf: <${RDF}> .

ex:alice a ex:Person .
ex:bob a ex:Person .
`,
    shapes: `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix rdf: <${RDF}> .

ex:PersonShape
  a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:rule [
    a sh:TripleRule ;
    sh:subject sh:this ;
    sh:predicate ex:adult ;
    sh:object ex:yes ;
  ] .
`,
    activePane: "shacl",
  },
  {
    id: "isomorphic-pair",
    zhTitle: "同构图（空节点标签不同）",
    zhBlurb: "图 A 和图 B 只在空节点标签和排列顺序上不同。比较可验证同构并显示相同的规范形式。",
    title: "Isomorphic graphs (relabeled blank nodes)",
    blurb:
      "Graph A and Graph B name the same RDF graph under blank-node relabeling and " +
      "reordering. Compare confirms isomorphic: yes and shows identical canonical forms.",
    inputFormat: "turtle",
    input: `@prefix ex: <http://example.org/> .

_:a ex:knows _:b .
_:b ex:name "Bob" .
ex:root ex:member _:a .
`,
    graphBFormat: "turtle",
    graphB: `@prefix ex: <http://example.org/> .

ex:root ex:member _:x .
_:y ex:name "Bob" .
_:x ex:knows _:y .
`,
    activePane: "identity",
  },
  {
    id: "service-hardfail",
    zhTitle: "SERVICE 显式报错",
    zhBlurb: "浏览器没有服务器，无法解析联邦 SERVICE 子句。运行查询会显示引擎错误，不会悄悄返回空结果。",
    title: "SERVICE hard-fail (never-silent errors)",
    blurb:
      "There is no server: a federated SERVICE clause cannot be resolved in-browser. " +
      "Running this query surfaces the engine error in the live banner — never a " +
      "silent empty result.",
    inputFormat: "turtle",
    input: `@prefix ex: <http://example.org/> .

ex:alice ex:knows ex:bob .
`,
    query: `PREFIX ex: <http://example.org/>
SELECT ?remote WHERE {
  SERVICE <http://example.org/sparql> {
    ?remote ex:knows ex:bob .
  }
}`,
    activePane: "sparql",
  },
];

/** Look up a vignette by id. */
export function findVignette(id) {
  return VIGNETTES.find((v) => v.id === id);
}

async function fixtureText(path, fetchAsset) {
  const english = `could not load gallery fixture ${path}`;
  const chinese = `无法加载示例文件 ${path}`;
  let response;
  try {
    response = await fetchAsset(new URL(path, import.meta.url));
  } catch (error) {
    throw interfaceFailure(english, chinese, error);
  }
  if (!response.ok) throw new InterfaceError(
    `${english} (HTTP ${response.status})`,
    `${chinese}（HTTP ${response.status}）`,
  );
  try {
    return await response.text();
  } catch (error) {
    throw interfaceFailure(english, chinese, error);
  }
}

/** Build the same complete state for Load & run and Copy permalink. */
export async function vignetteState(v, fetchAsset = (url) => fetch(url)) {
  const [input, query] = v.files
    ? await Promise.all([fixtureText(v.files.input, fetchAsset), fixtureText(v.files.query, fetchAsset)])
    : [v.input, v.query];
  const s = { ...DEFAULT_STATE };
  s.input = input;
  s.inputFormat = v.inputFormat;
  if (query != null) s.query = query;
  if (v.shapes != null) s.shapes = v.shapes;
  // Identity vignettes can supply a distinct comparison graph.
  s.graphA = input;
  s.graphAFormat = v.inputFormat;
  if (v.graphB != null) {
    s.graphB = v.graphB;
    s.graphBFormat = v.graphBFormat ?? "turtle";
  }
  s.activePane = v.activePane ?? "input";
  return s;
}

/** Only the bundled museum graph and query can run on permalink startup. */
export async function matchesBundledMuseumQuery(state, fetchAsset) {
  const museum = findVignette("museum-attributions");
  if (state?.activePane !== "sparql" || state.inputFormat !== museum.inputFormat ||
      typeof state.input !== "string" || typeof state.query !== "string") return false;
  const bundled = await vignetteState(museum, fetchAsset);
  return state.input === bundled.input && state.query === bundled.query;
}

/** The default console state used on a cold load (no permalink). */
export const DEFAULT_STATE = {
  input: `@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# RDF-1.2 highlights, all in the http://example.org/ namespace:
#   * a triple term in object position (a quoted triple), and
#   * base-direction literals (ltr + rtl).
ex:alice ex:says <<( ex:bob ex:likes ex:carol )>> .
ex:alice ex:statedOn "2026-07-06"^^xsd:date .
ex:carol ex:name "Carol" .
ex:greetingEn ex:text "Hello, world"@en--ltr .
ex:greetingAr ex:text "مرحبا بالعالم"@ar--rtl .
`,
  inputFormat: "turtle",
  query: `PREFIX ex: <http://example.org/>
SELECT ?s ?p ?o WHERE {
  ?s ?p ?o .
}
ORDER BY ?s ?p ?o`,
  shapes: `@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix rdf: <${RDF}> .

# Every ex:Person must carry an ex:name; a triple rule stamps them ex:adult ex:yes.
ex:PersonShape
  a sh:NodeShape ;
  sh:targetClass ex:Person ;
  sh:property [
    sh:path ex:name ;
    sh:minCount 1 ;
  ] ;
  sh:rule [
    a sh:TripleRule ;
    sh:subject sh:this ;
    sh:predicate ex:adult ;
    sh:object ex:yes ;
  ] .
`,
  graphA: `@prefix ex: <http://example.org/> .

_:a ex:knows _:b .
_:b ex:name "Bob" .
ex:root ex:member _:a .
`,
  graphAFormat: "turtle",
  graphB: `@prefix ex: <http://example.org/> .

ex:root ex:member _:x .
_:y ex:name "Bob" .
_:x ex:knows _:y .
`,
  graphBFormat: "turtle",
  activePane: "input",
};
