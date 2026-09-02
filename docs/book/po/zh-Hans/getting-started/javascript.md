<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->
<!--
zh-Hans 译稿（第一阶段）。与 docs/book/src/getting-started/javascript.md 逐段对应，
待 mdbook-i18n-helpers 的 .po 工作流落地后倒入 msgstr。代码块与英文原文逐字节相同。
-->

# 入门：JavaScript / WebAssembly

npm 包
[`@blackcatinformatics/purrdf`](https://www.npmjs.com/package/@blackcatinformatics/purrdf)
是同一个 Rust 引擎编译到 `wasm32` 后，以 [RDF/JS](https://rdf.js.org/) 形态的 API
（`DataFactory`、`DatasetCore`、`Stream`/`Sink`）呈现出来的结果。它在浏览器和 Node
中运行，完全驻留内存。

```sh
npm install @blackcatinformatics/purrdf
```

**中国大陆镜像。** 从中国大陆访问 npm 注册表时常有延迟或间歇性不可达。开发者通常
改用 `npmmirror.com`（即原 cnpm/淘宝源），它镜像整个注册表：
`npm config set registry https://registry.npmmirror.com`。本包会自动同步到镜像，
PurRDF 这一侧无需任何操作；镜像的配置属于读者自己的环境，本文不代为脚本化。

想在安装任何东西之前先试一试？
[RDF-1.2 演练场](https://blackcat-informatics.github.io/purrdf/playground/)
在浏览器中运行的正是这份 wasm 构建——解析、SPARQL、SHACL、序列化，以及 RDF-1.2 图的
规范化/比较，全部在客户端完成，不需要工具链，也不需要服务器。

## 第一个数据集

在做任何事之前先 `await ready()` 一次——它执行一次性的异步 wasm 实例化：

```js
import { ready, DataFactory, Dataset, QueryEngine } from "@blackcatinformatics/purrdf";

await ready(); // one-time async wasm instantiation

const f = new DataFactory();
const rtl = f.directionalLiteral("مرحبا", "ar", "rtl");

const ds = new Dataset();
ds.add(f.quad(f.namedNode("https://ex/s"), f.namedNode("https://ex/says"), rtl));

const nq = ds.serialize("nquads");           // directions survive the round-trip
const reparsed = Dataset.parse(nq, "nquads");

const engine = new QueryEngine();
const ask = engine.ask(reparsed, "ASK { <https://ex/s> <https://ex/says> ?msg }");
```

## RDF 1.2 这块楔子

没有哪个现存的 RDF/JS 库承载 RDF 1.2 的**引用三元组项**（quoted-triple term）或
**方向字面量**（directional literal）。PurRDF 的 `DataFactory` 两者都提供：

```js
// A quoted triple, usable as a subject/object (RDF-star / RDF 1.2).
const quoted = f.quotedTriple(
  f.namedNode("https://ex/alice"),
  f.namedNode("https://ex/knows"),
  f.namedNode("https://ex/bob"),
);

// A base-direction literal (rdf:dirLangString).
const hello = f.directionalLiteral("مرحبا", "ar", "rtl");
```

## API 表面

- **`ready(bytesOrUrl?)`**——在做任何事之前 await 一次。
- **`DataFactory`**——`namedNode`、`blankNode`、
  `literal(value, languageOrDatatype?)`、`typedLiteral`、
  `directionalLiteral`、`variable`、`defaultGraph`、`quad`、`quotedTriple`、
  `fromTerm`、`fromQuad`。
- **`Dataset`**（RDF/JS `DatasetCore`）——`Dataset.parse(input, format, base?)`、
  `serialize(format)`、`add`/`delete`/`has`/`match`/`quads`/`size`，以及
  迭代（`for (const quad of dataset)`）。格式：`turtle`、`ntriples`、
  `nquads`、`trig`、`rdfxml`（或其媒体类型）；`serialize` 另外接受
  `jsonld`。
- **图同一性**——`Dataset.canonicalize()` 返回该图在 RDFC-1.0 下的规范、扁平
  N-Quads；`Dataset.isomorphic(other)` 在空节点重命名下判定 RDF 图相等（一个由
  完整 RDFC-1.0 规范化支撑的精确判定器）。
- **图/表格/研究对象载体**——`Dataset.project(profile, configJson)` 返回规范的
  USTAR 字节与损失台账 JSON；`Dataset.projectWithAssets("ro-crate-1.3",
  configJson, payloadArchive)` 加入有界的、随附的 RO-Crate 载荷；
  `liftProjection(...)` 为各双向 profile 重建 RDF。参见
  [图、表格与研究对象投影](../concepts/projections.md)。
- **SPARQL**——`QueryEngine` 在多次调用之间保持原生计划缓存存活，并暴露带类型的
  `select` / `ask` / `construct` / `describe`、原子的 `update`，以及
  `queryRaw` 序列化。`Dataset.query(...)` 仍作为兼容用的裸字符串辅助方法保留。
- **SHACL**——`shaclValidateToSarif(shapesTtl, dataNt)` 用一份 Turtle 形状图验证一份
  N-Triples 数据图并返回 SARIF 2.1.0 报告；`shaclEntail(shapesTtl, dataNt)` 把
  SHACL-AF `sh:rule` 的推论物化为 N-Triples。
- **`Sink`**——流式消费者（`push(quad)` / `finish() → Dataset`）；
  `datasetToStream` / `streamToDataset` 是异步的 RDF/JS Stream/Sink 辅助方法。

RDF/JS 映射的更多内容见 [JavaScript 中的 RDF/JS](../interop/rdfjs.md)。

## 范围与当前限制

- **仅限内存。** SPARQL 查询在内存数据集上运行；本包不提供网络解析器，因此远程
  `SERVICE` 与 `LOAD` 会显式失败。
- `serialize` 是写入器原生通道：它发出目标写入器有表面承载的一切，并拒绝其没有的。
  因此宾语位置的引用三元组项与 RDF 1.2 陈述层在 Turtle、N-Triples、N-Quads 与 TriG
  （写作 `<<( … )>>`）、RDF/XML（写作 `rdf:parseType="Triple"`）以及 JSON-LD /
  YAML-LD（写作 `@triple`）中都得以保留。TriX 与 HexTuples 没有三元组项表面，因此把
  携带三元组项的数据集序列化到二者之一会**抛出异常**，而不是静默丢弃该层。单图目标
  （Turtle、N-Triples、RDF/XML）只发出默认图。

## 从源码构建

Rust cdylib 位于
[`crates/rdf-wasm`](https://github.com/Blackcat-Informatics/purrdf/tree/main/crates/rdf-wasm)；
发布的 ESM 包由它生成：

```sh
make wasm-pkg        # release wasm + wasm-bindgen ESM bindings → js/pkg/
make wasm-pkg-test   # the above + TypeScript, Node, and packed-tarball gates
```

这需要 `wasm32-unknown-unknown` Rust 目标，以及固定到本 crate 所用 `wasm-bindgen`
版本的 `wasm-bindgen-cli`。
