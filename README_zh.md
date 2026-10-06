<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->
<p align="center">
  <a href="https://blackcatinformatics.ca/purrdf/">
    <img src="./docs/purrdf-logo.svg" alt="PurRDF logo — a black cat holding an RDF triple" width="128" height="128">
  </a>
</p>

<h1 align="center">PurRDF</h1>

<p align="center">
  <em>RDF 1.2、推理、检索与图传输——一个 Rust 引擎，为多种语言所共享。</em>
</p>

<p align="center">
  <strong>同一个 RDF 引擎。同一套行为。每一种语言。</strong>
</p>

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf/actions/workflows/ci.yaml"><img src="https://github.com/Blackcat-Informatics/purrdf/actions/workflows/ci.yaml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/purrdf"><img src="https://img.shields.io/crates/v/purrdf.svg?label=crates.io" alt="crates.io"></a>
  <a href="https://pypi.org/project/purrdf/"><img src="https://img.shields.io/pypi/v/purrdf.svg?label=PyPI" alt="PyPI"></a>
  <a href="https://www.npmjs.com/package/@blackcatinformatics/purrdf"><img src="https://img.shields.io/npm/v/%40blackcatinformatics%2Fpurrdf.svg?label=npm" alt="npm"></a>
  <a href="https://doi.org/10.67342/pkg8gpp4no/v1"><img src="https://img.shields.io/badge/DOI-10.67342%2Fpkg8gpp4no%2Fv1-blue" alt="DOI: 10.67342/pkg8gpp4no/v1"></a>
  <a href="./LICENSING.md"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0-blue.svg" alt="License: MIT OR Apache-2.0 OR MulanPSL-2.0"></a>
  <img src="https://img.shields.io/badge/MSRV-1.98-orange.svg" alt="MSRV 1.98">
</p>

<p align="center">
  <a href="https://blackcat-informatics.github.io/purrdf/playground/"><img src="https://img.shields.io/badge/RDF--1.2%20playground-try%20it%20live-brightgreen" alt="Try the RDF-1.2 playground in your browser"></a>
</p>

<p align="center">
  <a href="./README.md">English</a> · 简体中文 ·
  <a href="https://blackcat-informatics.github.io/purrdf/">英文手册</a>
</p>

---

> 译注：本文是 [`README.md`](./README.md) 的简体中文译本，与英文原文逐段对应。代码块、标识符与一致性数字与英文原文逐字相同；指向本书的链接改为指向中文版手册，原文中指向中文版手册的交叉链接与语言切换行相应改为指向英文原版，其余链接目标与英文原文相同；如有出入，以英文原文（[English](./README.md)）为准。

PurRDF 是一个用于围绕知识图谱构建应用的 Rust 工具包。它把 RDF 1.2、SPARQL、验证、
推理、文本与向量检索、文档编解码器以及图传输汇聚到同一个引擎之中。Python、
JavaScript / WebAssembly 与 C 调用这一引擎，而不是重新实现它。

一张图可以承载关于陈述的陈述、多语言字面量、命名图、溯源以及所链接的二进制内容。
PurRDF 在查询、验证以及能够表示这些区分的载体中始终保留它们；会丢失信息的投影则返回
一份定位到具体位置的损失台账。

**同一个 RDF 引擎。同一套行为。每一种语言。**各语言接口暴露的是工具包的不同部分，
详见下文。每一个已发布的 Rust crate 也都能构建到 `wasm32-unknown-unknown`。

[试用浏览器演练场](https://blackcat-informatics.github.io/purrdf/playground/)
· [阅读《PurRDF 之书》](https://blackcat-informatics.github.io/purrdf/zh-Hans/)
· [Rust API](https://docs.rs/purrdf)
· [迁移到 3.0](./docs/MIGRATION-3.0.md)

## 你可以构建什么

| 任务 | PurRDF 提供 |
| --- | --- |
| 让 RDF 1.2 贯穿整个应用 | 驻留（interned）数据集、三元组项、具体化节点（reifier）、注解、基础方向字面量与命名图；原生文本编解码器、规范化、差异比较与同构判定。 |
| 查询与修改图 | SPARQL 1.1/1.2 查询与 Update、预备执行、属性路径、宿主扩展、资源 governor（执行调控器）与解释回执（explain receipt）。 |
| 验证与推导知识 | SHACL 1.2、ShEx 2.1、确定性 Datalog、RDF/RDFS/OWL-RL/D 物化、OWL-Direct 推理与 RIF-Core。 |
| 跨多种信号检索 | 精确定点 BM25、GeoSPARQL 谓词、精确嵌入 k 近邻、确定性 HNSW，以及附带逐生产者（per-producer）证据的倒数排名融合（reciprocal-rank fusion）。 |
| 让文档可被查询 | 结构化 Markdown 与有序 JSON 编解码器，带按字节寻址的出现（occurrence）、显式 profile 与精确重建。 |
| 交换图数据 | 带二进制载荷的 GTS 容器；规范的五表 Parquet；带损失记录的图、表格与 Research Object（RO）投影。 |
| 在预算之内读取不可变快照 | 经认证（certified）的分段存储、经鉴别（authenticated）的区间读取、钉住（pinned）的词项、稀疏缓存，以及经由普通数据集/求值器扩展点进行的共享工作区准入。 |

在 Rust 中，从 [`purrdf`](./crates/purrdf/) 门面 crate 开始。它在根部暴露 RDF 接口，
并把其他引擎作为模块暴露。应用自行提供其词汇表、扩展注册、网络传输与嵌入模型。标准
RDF 词汇表中的词项是内置的；应用命名空间是显式配置。

## 查询、推理与检索协同进行

Rust 宿主可以在同一个 SPARQL 求值器中，把图模式与文本检索、空间关系和嵌入近邻连接
起来。每种关系都注册在由应用提供的谓词 IRI 之下：

```sparql
PREFIX ex:  <https://example.org/>
PREFIX geo: <http://www.opengis.net/ont/geosparql#>

SELECT ?doc ?score ?distance WHERE {
  ?doc ex:search ( "harbour dredging" ?score ?rank ?lang ?matched ) .
  ?doc ex:locatedIn ?feature .
  ?feature geo:sfWithin ex:PortDistrict .
  ?doc ex:nearest ( ex:doc-42 5 ?distance )
}
ORDER BY ?rank
```

该查询假定宿主已经注册了文本、空间与向量关系，并提供了坐标系配置。未注册的谓词仍然
只是一个普通的 RDF 模式。

- **文本：**[`purrdf-text`](./crates/text/) 为 RDF 字面量（含注解层）建立索引，采用
  Unicode 规范化、大小写折叠与切分。BM25 分数使用精确的定点算术。带排名的检索与词项
  出现关系支持在 SPARQL 中组合短语与邻近查询。索引常驻内存，在冻结数据集上构建。默认
  保留词法拼写；调用方可以选择英文词干提取。停用词词典与独立的查询方言不在其接口之内。
- **几何：**[`purrdf-geo`](./crates/geo/) 在精确有理数的 WKT/GeoJSON 几何上实现
  GeoSPARQL 1.1 拓扑谓词，外加访问器以及可精确计算的度量与构造器。坐标变换、椭球
  大地测量、缓冲区与叠加集合运算均未实现；已注册但不受支持的函数按名称拒绝。宿主声明
  CRS，以及哪些坐标系以米计量。
- **向量：**[PURREMB](./docs/PURREMB.md) 承载调用方产出的嵌入（向量嵌入）、其坐标及其
  派生同一性。精确 k 近邻支持余弦、负点积与平方欧氏距离。
  [`purrdf-hnsw`](./crates/hnsw/) 增加了一个近似索引，在其默认算术下，层级、构建调度
  与载荷字节都是确定的。近似候选绝不证明（certify）不存在更近的行。PurRDF 写出并读取
  嵌入工件；它不运行模型来生成这些向量。
- **融合：**[`purrdf-retrieval`](./crates/retrieval/) 规划一个带类型的请求，将其编译为
  逐生产者的 SPARQL，执行它并融合各条带排名的流。精确定点的倒数排名融合携带计划、融合
  法则与证据的同一性，逐层（stratum）溯源、索引代次证明（index-generation
  attestation）以及所声明的检索保真度（search fidelity）。未得到服务的请求词项与不完整
  的生产者在结果中保持可见。生产者、层与权重由调用方提供。

默认的距离算术在原生与 WASM 路径之间固定累加顺序。显式的 `Reassociated` 算术允许最后
几位不同，以换取不同的代码生成；HNSW 工件把这一选择绑定到其所记录的构建与执行路径上。
近似与算术是两份独立的契约。参见[嵌入 k 近邻](./docs/design/purrdf-embedding-knn.md)、
[HNSW 的召回率与构建证据](./crates/hnsw/README.md)、
[检索组合](./docs/design/purrdf-retrieval-ladder.md)以及
[SIMD 与算术契约](./docs/design/purrdf-simd.md)。

这些带排名生产者（ranked producer）的集成是 Rust API，编译到 WASM 的 Rust 宿主同样
可以使用。已发布的 Python 与 npm 包暴露数据形态的属性函数与路径见证；它们不暴露文本、
空间或向量生产者的注册。

## 文档、载体与存储

**文档可以成为其自身的图。**
[`purrdf-markdown`](./crates/markdown/SPEC.md) 把一种经规范定义的 Markdown 方言投影为
标题、段落及其他结构单元，各单元落在逐字对应的字节区间之上。
[`purrdf-json`](./crates/json/SPEC.md) 记录有序的 JSON 出现及其字节覆盖（byte cover）。
二者都要求显式 profile，把同一性绑定到 profile 的法则与源字节，并逐字节重建源文档。
Markdown 方言由该编解码器自行规定；它并非 CommonMark 解析器。

**转换对损失如实交代。**原生编解码器覆盖 Turtle、TriG、N-Triples、N-Quads、RDF/XML、
TriX、HexTuples、JSON-LD 与 YAML-LD。JSON-LD 上下文透镜（context lens）编译可复用的
离线上下文；它提供展开、压缩与派生前缀三种模式，上下文注册表由调用方提供，不含网络
加载器，也没有 framing API。图与 Research Object 投影包括 Neo4j CSV、openCypher、
GraphML、CSVW、OBO Graphs、SKOS、DCAT、VoID、RO-Crate、Croissant、DataCite 与
Frictionless。受支持的可逆载体经由精确的边带（sideband）保留 RDF 1.2；有损视图报告
丢失了什么。参见[投影](./docs/book/src/concepts/projections.md)与
[生成的损失矩阵](./generated/transcode-loss-matrix.json)。

**唯一一种规范的列式投影。**
[`purrdf-columnar`](./crates/columnar/) 提供原生的五表 v1 契约：`terms`、`quads`、
`reifiers`、`annotations` 与 `blobs`。Python 的 SQLite、DuckDB 与 Parquet 导出器使用
其规范 ID 与表结构，保留带作用域的空节点、被引用的词项、方向、空的命名图以及经过验证的
blob。GTS 按追加顺序分配的检视 ID 仍是另一套独立的权威。

**两条不可变存储路径。**即时（eager）加载的 pack 快照提供前端编码（front-coded）的
字典、位图索引与内容验证。新的 [`SegmentedSession`](./crates/rdf-core/STORAGE.md)
通过宿主的区间提供者（range provider）读取一种经认证的持久表示，鉴别所准入的块，并在
稀疏缓存、钉住、证据与算子工作区之间共享一份实时台账。它在封存与重新打开之间保持全局
ID 不变；持久化的句柄标识确切的快照。

常驻内存的数据集保留紧凑的四字节词项 ID、十六字节的四元组行以及借用式的词项访问。
操作性读取经由同一个 `DatasetView` 扩展点，使用带类型的失败与钉住守卫。存储故障会在
操作的最终检查点丢弃其结果。有界查询准入目前覆盖基于基本图模式、仅投影变量的普通
`SELECT`；尚未定价的操作性形式返回带类型的拒绝。构建、完整认证、宿主缓冲区与调用方
拥有的输出各有独立的内存义务。内核不执行任何文件系统或网络 I/O。确切边界见
[存储契约](./crates/rdf-core/STORAGE.md)与 [3.0 迁移指南](./docs/MIGRATION-3.0.md)。

Rust 的 `explain_query_fallible_view` 直接度量操作性视图，并连同最终的存储证据返回其
解释。它的选项与停止信号变体共用查询准入规则；存储故障会丢弃整个解释，而仅发生停止时，
停止会出现在解释的 governor 证据中。

**图连同其载荷一起传输。**[GTS](./docs/GTS-SPEC.md) 是一种内容寻址、仅追加的容器，
具备确定性折叠、二进制载荷、链式 CBOR 段、COSE 签名/加密与纯 Rust 密码学。其容器 API
覆盖 Rust、Python 与 C，CLI 把它作为输入格式读取。npm / JavaScript 包不暴露它。冻结的
传输向量与[权威的 GTS 项目](https://github.com/Blackcat-Informatics/gmeow-gts)共享。

## 验证、推理与可问责的执行

SHACL 1.2 涵盖 Core、SPARQL 扩展、节点表达式、推理规则与 SPARQL 1.2 RL，SHACL-AF 的
拼写映射到同一种共享表示。报告是 RDF 数据集。ShEx 2.1 提供 ShExC/ShExJ 模式与验证，
包括导入与语义动作。导入从显式的宿主注册表解析。Rust 模式编译器还把形状投影为
JSON Schema、OpenAPI、Pydantic、LinkML、TypeScript 与 GraphQL，并附覆盖率与损失报告；
这些模式通道仅限 Rust。

蕴涵引擎实现了**全部 78 条 OWL 2 RL 规则**与全部 18 条 RDF + RDFS 模式。这是规则表
覆盖率，有别于蕴涵一致性：在随库固化（vendored）的 W3C 语料上，chase 的成绩是
**正例 27/27、负例 23/23**，后者记录的是未发现不可靠之处。额外的 `ext-eq-diff-sym`
规则在推理报告中披露，且不计入上述规则数。OWL-Direct 提供开放世界的 SHOIQ(D) tableau
以及附带证书的推理服务；被上限截停的搜索返回 `unknown`。RIF-Core 与蕴涵感知的 SPARQL
与各物化机制并行运行。参见[规则清单](./docs/book/src/entailment-rules.md)与
[推理服务](./docs/book/src/entailment.md)。

查询 governor 约束执行，并在触顶时返回相关证据，包括在非单调算子之下哪些行可以得到
认证。受调控的 Update 要么完整提交，要么完全不提交。预备执行与有作用域的回调复用同一个
求值器。计费表与冻结的 50 例 governor 语料发布在
[governor profile](./docs/SPARQL-GOVERNOR-PROFILE.md) 中。结构化诊断保留稳定的代码与
可读的英文，并在 JSON、SARIF 与 C 诊断记录中带有具名参数与精确的逻辑锚点。

## 快速入门

### Rust

```sh
cargo add purrdf
```

```rust
use purrdf::{parse_dataset, serialize_dataset, SerializeGraph};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        @prefix ex: <https://example.org/> .
        ex:alice ex:knows ex:bob .
    "#;
    let dataset = parse_dataset(source.as_bytes(), "text/turtle", None)?;
    assert_eq!(dataset.quad_count(), 1);
    let turtle = serialize_dataset(&dataset, "text/turtle", SerializeGraph::Dataset)?;
    let restored = parse_dataset(&turtle, "text/turtle", None)?;
    assert_eq!(restored.quad_count(), 1);
    Ok(())
}
```

随着应用的增长，可使用门面 crate 的 `sparql`、`shapes`、`shex`、`entail`、`retrieval`、
`columnar`、`json` 与 `markdown` 模块。
[更多 Rust 示例](./docs/book/src/getting-started/rust.md)。

### Python

```sh
pip install purrdf
```

```python
import purrdf

quads = purrdf.parse(
    '<https://example.org/alice> <https://example.org/name> "Alice" .',
    purrdf.RdfFormat.TURTLE,
)
print(quads)
```

Python 3.13+ 的 wheel 携带原生扩展。`Store` 暴露 SPARQL 与 Update；`shapes`、`shex` 与
`entail` 暴露验证与推理。数据集与 GTS 的列式导出使用原生投影。
[Python 指南](./bindings/python/README.md)。

`purrdf.compat.rdflib` 提供一个显式的兼容层。对于需要顶层 `rdflib` 导入名的应用：

```sh
pip install 'purrdf[rdflib]'
```

这会安装版本相匹配的独立 `purrdf-rdflib` 分发包。它的 `rdflib` 包与真正的 rdflib 不能
共存于同一环境；请让真实 rdflib 的环境保持独立，并在那里省略这一影子 extra。

### JavaScript / WebAssembly

```sh
npm install @blackcatinformatics/purrdf
```

```javascript
import { ready, DataFactory, Dataset } from "@blackcatinformatics/purrdf";

await ready();
const f = new DataFactory();
const dataset = new Dataset();
dataset.add(f.quad(
  f.namedNode("https://example.org/alice"),
  f.namedNode("https://example.org/greeting"),
  f.directionalLiteral("مرحبا", "ar", "rtl"),
));
const restored = Dataset.parse(dataset.serialize("nquads"), "nquads");
console.log(restored.size, dataset.id.toString());
```

RDF/JS 形态的 API 还暴露 SPARQL、SHACL、推理、投影与静态的 RDF 1.2 SVG 可视化。
异步查询可经由 JSPI 使用宿主提供的 `SERVICE` / `LOAD` 解析器。在 3.0 中，数据集的
同一性与代次（generation）为 `bigint`；在 JSON 中请使用十进制字符串。
[JavaScript 指南](./crates/rdf-wasm/js/README.md)。

### C 与 CLI

[`libpurrdf`](./crates/rdf-capi/) 通过一个 panic 安全的 C ABI 暴露解析、序列化、迭代、
变更、SPARQL、验证、推理与 GTS。`make capi-build` 用 cargo-c 构建该库；
`make capi-bundle` 准备一个可重定位的头文件/库分发包，附带面向接收者的声明。已提交的
[`purrdf.h`](./crates/rdf-capi/include/purrdf.h) 会对照生成输出进行检查。

[CLI](./crates/cli/) 提供 `convert`、`query`、`update`、`reason`、`entails`、
`consistency`、`validate`、`shex`、`describe`、`project`、`lift` 与 `pack verify`。
请从本仓库构建它；CLI crate 未发布到 crates.io。参见 [CLI 用法](./crates/cli/README.md)。

每个解析接口都接受一个显式的文档基准 IRI，用于解析相对 IRI。当既没有显式基准 IRI，
也没有文档内基准 IRI 可用时，未解析的相对 IRI 会被诊断出来。网络访问由宿主提供：随库
发布的同步接口不安装任何联邦解析器，核心也不附带 HTTP 客户端。

## Crate 一览

| Crate | 角色 |
| --- | --- |
| [`purrdf`](./crates/purrdf/) | 从这里开始：总括门面 crate。 |
| [`purrdf-rdf`](./crates/rdf/) | 原生 RDF 编解码器、GTS 适配器、投影与规范化。 |
| [`purrdf-core`](./crates/rdf-core/) | 驻留 IR、读取会话、分段存储、诊断、溯源、pack 与 PURREMB。 |
| [`purrdf-sparql-algebra`](./crates/sparql-algebra/) | SPARQL 解析与代数。 |
| [`purrdf-sparql-eval`](./crates/sparql-eval/) | Query/Update 求值、governor 与扩展点。 |
| [`purrdf-sparql-results`](./crates/sparql-results/) | 结果的 JSON、XML、CSV 与 TSV。 |
| [`purrdf-shapes`](./crates/shapes/) | SHACL 验证/规则与模式编译。 |
| [`purrdf-shex`](./crates/shex/) | ShEx 模式与验证。 |
| [`purrdf-datalog`](./crates/datalog/) | 确定性的半朴素规则基底。 |
| [`purrdf-entail`](./crates/entail/) | 物化、OWL-Direct 与 RIF-Core。 |
| [`purrdf-text`](./crates/text/) | 精确定点的全文检索。 |
| [`purrdf-geo`](./crates/geo/) | 精确的 GeoSPARQL 几何与关系。 |
| [`purrdf-hnsw`](./crates/hnsw/) | 确定性的近似最近邻索引。 |
| [`purrdf-retrieval`](./crates/retrieval/) | 带类型的规划、执行以及附带证据的排名融合。 |
| [`purrdf-columnar`](./crates/columnar/) | 规范的五表 Parquet 编解码器。 |
| [`purrdf-gts`](./crates/gts/) | 容器、折叠、验证与密码学。 |
| [`purrdf-markdown`](./crates/markdown/) | 结构化 Markdown 编解码器。 |
| [`purrdf-json`](./crates/json/) | 有序 JSON 字节覆盖编解码器。 |
| [`purrdf-jsonschema`](./crates/jsonschema/) | 原生 JSON Schema，支持 draft 2020-12、2019-09 与 07。 |
| [`purrdf-slice`](./crates/slice/) | 切片目录、工件所有权与依赖。 |
| [`purrdf-validate`](./crates/validate/) | 共享的验证、governor 与诊断宿主边界。 |
| [`purrdf-iri`](./crates/iri/) | IRI/URI、语言标签、基准 IRI 解析与标准词汇表。 |
| [`purrdf-xsd`](./crates/xsd/) | XSD 值空间、精确数值与时间算术。 |
| [`purrdf-cdt`](./crates/cdt/) | SPARQL 复合数据类型及其函数库。 |
| [`purrdf-events`](./crates/rdf-events/) | 零依赖的摄入协议与文本方向。 |
| [`purrdf-lex`](./crates/lex/) | 共享的终结符、Unicode，以及 JSON/YAML/CBOR/XML 编解码器。 |
| [`purrdf-hash`](./crates/hash/) | 零依赖的哈希与共享的同一性内核。 |
| [`purrdf-deflate`](./crates/deflate/) | 原生、确定性的 DEFLATE/gzip。 |
| [`purrdf-ed25519`](./crates/ed25519/) | Ed25519 签名与严格验证。 |
| [`purrdf-stack`](./crates/stack/) | 原生/WASM 栈准入。 |
| [`purrdf-wasm`](./crates/rdf-wasm/) | 面向 JavaScript 的引擎与 ESM 绑定。 |

C ABI、CLI、Python 扩展以及测试/基准工具位于同一个工作区，但不作为 Cargo 包发布。
共享的第一方词法、编解码、哈希与签名基础层缩小了外部依赖面；每项共享工作都有唯一一个
强制执行的归属。没有任何语义性的 Cargo feature，因此安装时的 feature 选择无法改变
载体的行为。

## 证据与性能

[一致性记分板](./docs/CONFORMANCE.md) 区分官方套件、第一方语料、经批准的分歧与未经
测试的边界。SPARQL 求值有 **911 个通过**，0 例入台账。SHACL 在随库固化的
W3C SHACL 1.0 测试套件上 **129/129 通过**，0 例入账；在随库固化的 W3C SHACL 1.2 测试
套件上 **538/544 通过**（6 个经批准的结果以非规范形式拼写计算所得的小数，按规范的 XSD
拼写评分）。


一致性门禁区分官方套件、第一方冻结语料与明确记录的边界。完整记分板与运行命令见
[`docs/CONFORMANCE.md`](./docs/CONFORMANCE.md)：

| 引擎 | 套件 | 结果 |
| --- | --- | --- |
| ShEx 2.1 验证 | shexTest v2.1.0（`vectors/shexTest/`） | **1,105 / 1,105** 尝试，0 xfail |
| ShEx 模式 / 负例语法 / 结构 | shexTest v2.1.0 | **425/425 · 99/99 · 14/14** |
| SHACL | W3C data-shapes（`vectors/shacl/`） | **129 / 129** 通过 · 0 例入账 |
| SHACL 1.2 | W3C shacl12-test-suite（`vectors/shacl12/`） | **538 / 544** 通过 · 6 个非规范形式的预期小数 · 0 例入账；3 个未列入清单的随库固化文件单独评分 |
| SHACL（第一方冻结语料） | `crates/shapes/corpus/` | **73 / 73** |
| SHACL Rules | DASH + 第一方（`vectors/shacl/af/rules/`） | **20 / 20** |
| 语法编解码器 | W3C rdf-tests 往返 | **264 / 264** |
| JSON-LD 1.1 上下文透镜 | W3C JSON-LD 1.1 REC toRDF + 压缩（`crates/rdf/tests/fixtures/jsonld-w3c-rec/`） | **73 / 73** 适用的 toRDF · **13 / 13** 精确压缩 |
| SPARQL 1.1/1.2 | 完整的 W3C sparql11 + sparql12 + 第一方，经由 `purrdf-sparql-conformance` | **911** 通过 · 0 例入台账 |
| SPARQL CDT（SEP-0009） | 随库固化的 `awslabs/SPARQL-CDTs`（`vectors/sparql-cdt/`） | **658 / 658**，0 例入账——词法空间分歧见 [`docs/CONFORMANCE.md`](./docs/CONFORMANCE.md) |
| SPARQL 执行 governor | 第一方冻结语料（`vectors/sparql-governors/`） | **50 / 50**，0 例入账 |
| 蕴涵（SPARQL 蕴涵机制） | W3C sparql11 `entailment/` 组 | **70 / 70**，0 例入账 |
| 蕴涵（OWL 2 DL 相容性） | 随库固化的 W3C OWL 2 套件 | **258 / 262** 一致，4 例入台账，0 例未入台账 |
| 蕴涵（OWL 2 RL，W3C 蕴涵测试） | 随库固化的 W3C OWL 2 蕴涵套件 | **50 / 50** 一致，0 例入账，0 例未入台账——负例通道 **23 / 23**（未发现不可靠之处），正例通道 **27 / 27** |
| RDFC-1.0 | W3C 规范化夹具 | 绿 |
| RDF 1.2 规范化 profile（`purrdf-rdfc12` v2） | 第一方向量（`vectors/rdf12-canon/`） | **12 / 12** |
| GTS | 冻结的跨语言向量（`vectors/`） | **38 / 39** 逐字节折叠为其已提交的期望值，1 处入台账的分歧 |

性能证据因工作负载与宿主而异。基准测试框架记录分布与分配证据；`make bench` 运行原生
微基准。规模语料、LUBM 与 WatDiv 对比通道仅供报告，并需要其文档所述的输入。对比性
运行需要受控的机器条件、匹配的构建以及保留下来的原始样本。浏览器内存上限、读取器台账
与物理设备限制所确立的是不同的事实。参见[基准测试方法论](./docs/BENCHMARKS.md)。

## 发展方向

PurRDF 正在成长为一个基础，供在各种部署规模下结合图推理、检索与文档数据的应用使用。
下一步的进展应当使这些能力更易于组合，并能带入更多工作流，同时保持同一性、资源上限与
证据的显式性。公共契约以及上述已实现的接口是这一成长的基础。

## 文档与开发

- [PurRDF 之书](https://blackcat-informatics.github.io/purrdf/zh-Hans/)：各语言指南、
  概念与引擎契约；另有[英文原版](https://blackcat-informatics.github.io/purrdf/)。
- [浏览器演练场](https://blackcat-informatics.github.io/purrdf/playground/)：
  在浏览器中本地解析、查询、验证、序列化与比较图。
- [迁移到 3.0](./docs/MIGRATION-3.0.md)与[变更日志](./CHANGELOG.md)。
- [GTS](./docs/GTS-SPEC.md)、[PURREMB](./docs/PURREMB.md)、
  [RDF 1.2 规范化](./docs/RDF12-CANON-PROFILE.md)、
  [存储契约](./crates/rdf-core/STORAGE.md)与
  [发布流程](./docs/RELEASE.md)。

```sh
make metadata      # regenerate and verify projections and license bundles
make check         # formatting, clippy, build, tests and hygiene
make bench         # report-only microbenchmarks
make scale-corpus  # deterministic corpus generation
make lubm          # comparison lane; pinned network inputs and a JRE
make watdiv        # comparison lane; frozen network dataset
```

使用者需要 stable Rust **1.98** 或更新版本。贡献者使用 `rust-toolchain.toml` 中声明的
nightly 分析工具链；源码不使用任何 nightly 独有特性。CI 单独强制执行 stable MSRV，并为
WASM 构建每一个已发布的 crate。发布工件使用 stable Rust 构建。

Cargo 套件、Python 分发包与 npm 包共享同一个协调一致的版本，并遵循语义化版本。C ABI
单独编号，当前为 **0.8**，带有签名检查，并可在运行时经由 `purrdf_abi_version` 读取。
经过验证的提交工作流见[贡献指南](./CONTRIBUTING.md)。

PurRDF 由 Blackcat Informatics® Inc. 开发，是
[GMEOW](https://github.com/Blackcat-Informatics/gmeow-ontology) 的库骨干。
[抽取历史与溯源](./PROVENANCE.md)记录了它与
[GTS 项目](https://github.com/Blackcat-Informatics/gmeow-gts)的关系。

## 许可

第一方代码以 [MIT](./LICENSE-MIT)、[Apache License 2.0](./LICENSE-APACHE) 或
[MulanPSL-2.0](./LICENSE-MULAN) 提供，由你任选其一。单独许可的文档与第三方材料保留其
各自的条款。实际的分发归档包含适用的完整文本、面向接收者的声明与溯源清单。
[许可指南](./LICENSING.md)说明其适用范围，并
[提供中文说明](./docs/LICENSING.zh-Hans.md)。

若在研究中使用 PurRDF，请引用 [CITATION.cff](./CITATION.cff)。
