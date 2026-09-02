<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
<p align="center"><a href="./README.md">English</a></p>

<p align="center">
  <a href="https://blackcatinformatics.ca/purrdf/">
    <img src="./docs/purrdf-logo.svg" alt="PurRDF logo — a black cat holding an RDF triple" width="128" height="128">
  </a>
</p>

<h1 align="center">PurRDF</h1>

<p align="center">
  <em>自带呼噜声的 RDF 1.2 工具包：图原语、编解码器、SPARQL、SHACL、ShEx、蕴涵、全文检索、GeoSPARQL 与图传输。</em>
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
  <a href="./LICENSING.md"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0"></a>
  <img src="https://img.shields.io/badge/MSRV-1.96-orange.svg" alt="MSRV 1.96">
</p>

<p align="center">
  <a href="https://blackcat-informatics.github.io/purrdf/playground/"><img src="https://img.shields.io/badge/RDF--1.2%20playground-try%20it%20live-brightgreen" alt="Try the RDF-1.2 playground in your browser"></a>
</p>

---

> 译注：本文是 [`README.md`](./README.md) 的简体中文译本，对应 PurRDF 0.14.0 的英文文档。代码块、
> 标识符、链接与一致性数字与英文原文逐字相同；《PurRDF 之书》的英文页面链接保持原样。

PurRDF 是一个 [RDF 1.2](https://www.w3.org/TR/rdf12-concepts/) 工具包——图原语、
编解码器、SPARQL 1.1/1.2、SHACL、ShEx、蕴涵机制（entailment regime）与 GTS 图传输——
以 Rust 实现一次，并原样带入 Python、WebAssembly/JavaScript 与 C。每一个已发布的
crate 都能构建到 `wasm32-unknown-unknown`，因此在服务器上回答某条查询的引擎，也会在
浏览器标签页中逐字节地给出同一答案。使同一张图行于诸语言之间而其义不移——此即吾辈所架之
津梁。

**它从架构中移除了什么。**通常让一个 PostgreSQL 实例伴随三元组存储运行的三项工作——
带排名的全文检索、空间谓词与向量相似度——都在 PurRDF 内部得到回答：进程内、同一
数据集、同一条 SPARQL 查询，没有第二个数据库，没有同步作业，也没有一半 SPARQL 一半
SQL 的拆分提问。每个答案都是精确且确定的——同样的行、同样的顺序、同样的分数词法形式，
原生与 wasm32 上皆然——这是 Postgres 技术栈跨机器时并不作出的保证。这不是一个设想：
这些查询接口已经从一个 RDF 项目中移除了整个 PostgreSQL 依赖。经验证的能力表、每个
接口的边界，以及架构上的前后对照，见
[一个引擎取代三个数据库](#一个引擎取代三个数据库)。

## 它为何存在？

RDF 工具沿两条轴线碎片化。

**跨语言**：每个生态都有自己的解析器，带着自己的缺陷、自己对边界情况的解读，以及自己
所实现的规范子集。把一张图从 Rust 服务搬到 Python 流水线再搬到浏览器，数据的含义就已
被悄悄改变了三次。

**跨时间**：[RDF 1.2](https://www.w3.org/TR/rdf12-concepts/)——三元组项、具体化节点
（reifier）、基础方向字面量——是标准的现行修订版，而几乎没有哪个现存的库承载它。

PurRDF 之所以存在，是为了让一张图在任何地方都是**同一张图**。它是一个从零实现、依赖
极简的 Rust 核心——从解析器到 SPARQL 引擎到 SHACL 验证器再到二进制传输——原样带入
Python、WebAssembly/JavaScript 与 C。整个工作区刻意**没有任何 Cargo feature 标志**
（CI 强制执行）：数据载体不得有可选行为，因此每个消费者得到的是同样的、字节级一致的
语义。

PurRDF 是 [GMEOW](https://github.com/Blackcat-Informatics/gmeow-ontology) 技术栈的
数据骨干，也是 [GTS](./docs/GTS-SPEC.md) 图传输引擎的参考实现所在，但它对调用方的本体或应用
不作任何假设。

## 一个引擎取代三个数据库

需要带排名的文本检索、空间谓词或最近邻搜索的 RDF 项目，通常正是为了这三项工作而在
三元组存储旁边运行一个 PostgreSQL。PurRDF 直接在已加载到内存的数据集上、通过求值器提供的属性函数（property function）与标量函数
扩展接缝（extension seam，由调用方以自有 IRI 注册），用 SPARQL 回答全部三项。
下面的每一项能力都是打开 crate 就能找到的；每一条边界都是其测试所固定的。

| 所需能力 | 通常来自 | 如今在 PurRDF 之内 | 止步于何处 |
| --- | --- | --- | --- |
| 带排名的全文检索 | PostgreSQL `tsvector`/`tsquery` | [`purrdf-text`](./crates/text/)：一个覆盖 RDF 1.2 字面量（含注解层）的倒排索引，Unicode 大小写折叠与词边界切分，以及以精确的 `i128` 十进制定点数做的 BM25 排名，**crate 内没有浮点**（`#![deny(clippy::float_arithmetic)]`）。两个关系：`?doc <iri> ( "needle" ?score ?rank ?lang ?matched )` 用于带排名的检索，`?doc <iri> ( "term" ?lang ?position )` 用于词项出现位置，短语与邻近查询由此在普通 SPARQL 中组合而成。 | 是 BM25 排名，不是 Lucene：没有词干提取，没有停用词表，没有查询方言；`k1` 与 `b` 是固定常量；索引位于内存中，在冻结数据集上一次性构建（`TextIndex::from_dataset`）。 |
| 空间谓词 | PostGIS | [`purrdf-geo`](./crates/geo/)：GeoSPARQL 1.1（OGC 22-047r1）——WKT 与 GeoJSON 字面量解析为精确有理数，Simple Features、Egenhofer 与 RCC8 的每一种关系都在精确的 DE-9IM 上判定，`geof:` 函数落在标量接缝上，Query Rewrite 关系（要素之间的 `?a geo:sfWithin ?b`）落在属性函数接缝上。没有 GEOS，没有 PROJ，没有浮点运算；唯一的浮点边界是 `xsd:double` 结果字面量。 | 是矢量几何上的拓扑谓词、访问器与可精确计算的度量和构造器，不是 PostGIS：`geof:transform` 按名称硬错误（没有 CRS 数据库），`metric*` 度量只在调用方声明为以米计量的 CRS 中作答（没有椭球大地测量），而 `buffer`、`concaveHull`、`boundingCircle`、叠加集合运算（`intersection`/`union`/`difference`/`symDifference`）与 GML/KML/DGGS 编码已注册但按名称硬错误。没有栅格，没有持久化空间索引。 |
| 向量相似度 | pgvector | [`purrdf-sparql-eval`](./crates/sparql-eval/) 中的嵌入 k 近邻：`?neighbour <space> ( ?seed k ?distance )`，在 [PURREMB](./docs/PURREMB.md) 嵌入空间上（`EmbeddingSpace::from_artifact`、`EmbeddingKnnRelation`），按工件声明的度量做精确 top-k，binary64 且累加顺序固定、无融合乘加，并列按内容派生的行顺序打破。 | 精确搜索——每个候选都被评分，没有剪枝，没有近似索引——由调用方提供的 `KnnGuard` 限定（最大空间、最大 `k`；是拒绝，不是截断）。三种度量：余弦、负点积、平方欧氏距离。PurRDF 不计算嵌入，也不运行任何 ANN 载荷：向量来自调用方产出的 PURREMB 工件。 |

本版本中还有两项能力落在同一接缝上：**路径见证**（path witness），一个逐跳绑定遍历
过程的属性函数，每条被遍历的陈述都是一个 RDF 1.2 三元组项；以及 **SEP-0009 复合数据
类型**（`cdt:List`/`cdt:Map`，带 `FOLD` 与 `UNFOLD`）。后者有一处分歧被明确陈述而非
隐藏：PurRDF 允许 RDF 1.2 三元组项与带方向的语言标签字面量作为复合元素，这是一个词法
超集，符合 SEP-0009 的读取器会将其判为格式错误，且只在 SEP-0009 完全无法表达的值上
才会发出。

**确定，因而可移植。**Postgres 技术栈每个构建给出一种答案：`ts_rank` 与 pgvector 的
距离是浮点数，PostGIS 谓词运行在 GEOS 的浮点几何上。PurRDF 的三个接口在每个目标上
都是其输入的纯函数——BM25 用 `i128` 定点数配合固定迭代次数的整数对数，几何用精确
有理数配合整数 DE-9IM 判定，k 近邻用 binary64 配合单一的顺序累加——并且每一种排序
都是规范的：文档 id 在按 `(graph, subject, language)` 排序后分配，空间行按
`TermValue` 的全序排序，k 近邻的并列按内容派生的 `TargetId` 打破。这一声称是被执行
而非被论证的：文本与 k 近邻的确定性测试是同时带有 `#[test]` 与
`#[wasm_bindgen_test]` 的同一份测试体，由 `cargo test` 在原生上运行，由
`make wasm-test` 在 `wasm32-unknown-unknown` 上运行，而 `make geo-determinism` 在
两个目标上运行同一语料并比较字节。

**在浏览器中。**三个 crate 都是 `wasm32-unknown-unknown` 干净的，其确定性测试也在
那里执行——这是三个 Postgres 扩展没有一个能做到的。它们是 Rust 宿主上的接缝：宿主
构建一个索引或打开一个空间，并在自己的 IRI 之下注册它，而该宿主本身也可以编译到
wasm32（wasm 测试正是这样运行的）。已发布的 `@blackcatinformatics/purrdf` npm 包与
`purrdf` Python wheel 尚未暴露这三种关系；如今跨越这些边界的是数据形态的属性函数
（冻结表、以图为数据源的表、路径见证）。

**示意性的前后对照**（项目是真实的；此处不具名）：

- *之前*——一个三元组存储放图，旁边一个 PostgreSQL 实例，用 `tsvector`/`tsquery`
  处理标签与摘要，用 PostGIS 的 `ST_Within`/`ST_Intersects` 处理要素几何，用 pgvector
  的 `<->` 处理文档嵌入：三份数据副本，一个让它们保持对齐的同步作业，以及每一个横跨
  它们的问题都得一半写成 SPARQL、一半写成 SQL。
- *之后*——一个 PurRDF 数据集；一个 `PropertyFunctionRegistry`，持有一个
  `TextSearchRelation`、`geo:` 的 Query Rewrite 关系与一个 `EmbeddingKnnRelation`，
  各自在项目自己的 IRI 之下；一条通过基本图模式把三者连接起来的 SPARQL 查询；没有
  PostgreSQL。答案在服务器与浏览器中相同。

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

上面的每个谓词都是调用方的：PurRDF 不自行定义任何词汇表，因此 `ex:search`、`ex:nearest`
以及 `geo:sfWithin` 背后的 CRS 都是宿主提供的注册，而一条指名了无人注册的 IRI 的查询
只是一个普通的三元组模式。

## 里面有什么

- **RDF 1.2 图原语**——一个不可变的、按值驻留（interned）的数据集 IR（`TermId` 空间、
  字符串存储区、写时复制的变更），带有宾语位置的三元组项、具体化节点/注解侧表（side table），以及
  基础方向字面量（`rdf:dirLangString`）。
- **原生编解码器**——**Turtle、TriG、N-Triples、N-Quads、RDF/XML、TriX、HexTuples、
  JSON-LD (star) 与 YAML-LD** 的第一方解析器/序列化器，外加使用调用方提供词汇表的
  双向 OKF Markdown 包；输出字节级确定。
- **单一的基础解析层**——每个编解码器、SPARQL、ShEx 与 SHACL 都经由 `purrdf-iri` 中
  唯一的 RFC 3986 实现（`BaseIri`/`BaseScope`）解析相对 IRI（国际化资源标识符）引用，遵循 RFC 3986 §5.1
  的优先级链：文档内指令（`@base`/`BASE`/`xml:base`/`@context.@base`），否则调用方
  提供的基础 IRI，否则文档的检索 IRI（CLI 所打开文件的 `file://` IRI），否则硬错误
  `iri-relative-no-base`。相对 IRI 绝不会未经解析就进入图，而能够表达基础 IRI 的语法
  （Turtle、TriG、RDF/XML、JSON-LD、YAML-LD）在输出时会写出一个基础 IRI 并相对于它
  做相对化。参见 [基础 IRI 与相对引用](./docs/book/src/concepts/base-iris.md)。
- **规范化**——W3C **RDFC-1.0** 数据集规范化，对照 W3C 夹具套件测试。
- **投影与载体**——确定性的图、表格与 Research Object（RO）投影：一个规范的 LPG 模型、由 W3C
  把关的 CSVW（**270/270** 个 RDF 转换，**282/282** 个验证用例）、OBO Graphs 与
  SKOS 视图、原生的 DCAT/VoID 数据集描述，以及 RO-Crate 1.3 / Croissant 1.1 /
  DataCite 4.6 / DCAT 3 / Frictionless 载体——每个有损步骤都经由定位到具体位置的损失
  台账报告——外加 `purrdf-columnar` 中五表结构、字节级确定的 Parquet 编解码器。
- **RDF 1.2 可视化**——一个以陈述为中心的投影（`purrdf::viz`），为数据集做布局并渲染
  静态 SVG，并把 RDF 1.2 画成 RDF 1.2 本来的样子：三元组项是一个有边界的陈述图元
  （glyph）而不是一条箭头，具体化节点是一个经由 `reifies` 边与其所具体化的陈述相连的
  节点、其注解就在旁边，已断言的出现与被引用的出现保持区分，命名图上下文与方言诊断
  （dialect diagnostics，广义或对称位置）得以保留而不是被压平。同一模型之上有三种
  模式——`compact`（资源图）、`incidence`（精确的陈述/关联图）与 `table`（陈述行）。
  布局是确定的：id 由陈述键生成，每一种排序都是对陈述键的排序，破环方式固定，而绘图
  所依据的完整带版本号导出 JSON（`purrdf-viz-export-1`）内嵌在 SVG 的 `<metadata>`
  中，因此该文件既是一幅图，也是陈述模型的机器可读导出；`make check` 会重新渲染本书
  的十五幅示例 SVG，任何一个字节的漂移都会失败。止步于何处：仅限静态 SVG，没有交互式
  布局；一个由调用方设定的上限（`VizSpec::max_statements`，默认 500，以及
  `max_terms`，默认 1,500），超出上限的输入以 `VizError::TooLarge` 拒绝而不是截断；
  可从 Rust 与 JavaScript/wasm 包（`visualModel`、`visualExport`、`visualSvg`）访问，
  尚不能从 Python、C 或 CLI 访问。参见
  [RDF 1.2 可视化](./docs/book/src/concepts/visualization.md) 及其示例，例如
  [asserted-reified-compact](./docs/book/src/assets/visualization/purrdf-viz2-asserted-reified-compact.svg)。
- **SPARQL 1.1/1.2**——原生解析器 → 代数 → 驻留 IR 上的多重集求值器：全部四种查询
  形式加完整的 SPARQL Update、属性路径、基于代价的 BGP 规划，以及强制的 `VERSION`
  声明（含 `1.2-basic` profile）。`EXISTS`/`NOT EXISTS` 运行在 SEP-0007 可辩护的
  代换语义上（`Replace`/`PrjMap`，是 JOIN 而非词项重写，外加其第 3 部分的赋值限制），
  在准备期证明许可的情况下由带记忆化的存在性探测作答，否则按逐行定义作答。1.2 接口
  包括时间算术（SEP-0002：时刻、时长与五种公历部分日期类型，外加时长的 `SUM`/`AVG`
  与 `ADJUST`）与 `LATERAL`（SEP-0006，采用 Jena 的作用域规则）、SEP-0008 的 SHA-3
  内建函数，以及 SEP-0009 复合数据类型（`cdt:List`/`cdt:Map`、十五个函数的函数库、
  `FOLD` 聚合与 `UNFOLD` 图模式，在封闭叶 crate `purrdf-cdt` 中求值，并由随库固化（vendored）的
  `awslabs/SPARQL-CDTs` 语料把关）。有一处刻意的分歧被明确陈述而非隐藏：PurRDF 允许
  RDF 1.2 三元组项与带方向的语言标签字面量作为复合元素，这是一个词法超集，符合
  SEP-0009 的读取器会将其判为格式错误，且只在 SEP-0009 完全无法表达的值上才会发出。
  三个以调用方为键的扩展接缝——标量函数、属性函数（魔法谓词），以及经由
  `AGG(<iri>, …)` 的自定义聚合（在调用方提供的命名空间下有十个聚合的统计集合）——
  外加经由宿主可注入的 `ServiceResolver` 进行的 `SERVICE` 联邦查询，该解析器携带
  **逐服务上下文**（请求头、凭据、超时、能力；默认拒绝），其传输格式是确定性的
  序列化器，在 823 项随库固化的语料上往返扫描（含更新请求）。原生接缝上的宿主标量
  函数携带 SPARQL 的表达式错误通道：逐解的定义域错误在 `FILTER` 下消去该行，在
  `BIND`/`SELECT` 下让变量保持未绑定，而不是中止查询。由完整的 W3C SPARQL 1.1 + 1.2
  求值语料把关：**862 个通过**，5 个入台账的上游勘误夹具。结果以 SPARQL
  JSON/XML/CSV/TSV 给出。
- **核心之外的 SPARQL 扩展**——经由那些接缝、以兄弟 crate 形式到达的能力，每一个都在
  调用方提供的 IRI 之下注册（PurRDF 不自行定义任何 IRI），并且在原生与
  `wasm32-unknown-unknown` 上字节级一致：
  - **全文检索**（`purrdf-text`）——一个覆盖 RDF 1.2 字面量（含注解层）的内存倒排
    索引，先做 Unicode 规范化与完全大小写折叠（UAX 15、UAX 21），再做词边界切分
    （UAX 29），以及以精确的十进制定点数做的 BM25 排名，**crate 内没有浮点**（由 lint
    禁止），因此排名在每个目标上都是其输入的纯函数。两个属性函数：带排名的检索
    （`?doc <iri> ( "needle" ?score ?rank ?lang ?matched )`）与词项出现位置
    （`?doc <iri> ( "term" ?lang ?position )`），短语与邻近查询由此在普通 SPARQL 中
    组合而成。
  - **GeoSPARQL 1.1**（`purrdf-geo`，OGC 22-047r1）——WKT 与 GeoJSON 字面量解析为精确
    有理数，Simple Features、Egenhofer 与 RCC8 三族的每一种拓扑关系都在精确的 DE-9IM
    上判定，访问器以及可精确计算的度量与构造器，没有 GEOS，没有 PROJ，没有浮点运算
    （唯一的浮点边界是 `xsd:double` 结果字面量）。`geof:` 函数族落在标量接缝上，空间
    关系经由属性函数接缝重写；`geof:transform`、缓冲区、凹包、叠加集合运算与
    GML/KML/DGGS 编码已注册但**按名称硬错误**，而不是回答一个默认值（`geof:convexHull` 已实现），并且 `metric*`
    度量只在调用方声明为以米计量的 CRS 中作答（没有椭球大地测量）。
  - **嵌入 k 近邻**——在 [PURREMB](./docs/PURREMB.md) 嵌入空间上、以属性函数形式
    进行的最近邻搜索（`?neighbour <space> ( ?seed k ?distance )`）：按工件声明的度量
    做精确搜索，binary64 且累加顺序固定，governor（执行调控器）的计费与实际扫描的候选
    数成正比。
  - **路径见证**——一个绑定遍历*推导过程*而不只是其端点的属性函数：
    `?start <iri> ( ?end ?pathId ?len ?step ?node ?edge )`，每跳一行，每条被遍历的
    陈述都是一个可直接连接回数据集的 RDF 1.2 三元组项；每条简单前缀游走或每对端点
    一条最短见证，一个由内容派生的路径标识符，以及调用方必须声明的跳数上限。可从 CLI
    （`--path-relation`）与 Python（`path_relations`）访问；参考向量是在一个真实的
    Virtuoso `OPTION(TRANSITIVE …)` 实例上重新执行得到的，而不是从其手册抄录的。
- **受调控的执行**——每个查询/更新入口点都有一个对应的受调控版本，在调用方设定的
  上限（燃料、答案行数、中间单元格数、暂存字节数、远程请求数、截止时间）下运行，触及
  上限时返回经认证的行而不是错误答案，`--explain` 则在代价规划器的估计旁返回逐代数
  节点的计费台账。规范性的计费表与冻结的 50 例 governor 语料位于
  [`docs/SPARQL-GOVERNOR-PROFILE.md`](./docs/SPARQL-GOVERNOR-PROFILE.md)。
- **SHACL 验证**——一个原生验证器，具备完整的 SHACL Core 特性集（全部约束组件、完整
  属性路径、限定值形状、属性对）、原生引擎上的 SHACL-SPARQL 约束/目标、完整的
  SHACL-AF 接口（节点表达式、表达式约束、用户定义的 SPARQL 函数与目标类型，以及物化
  为新数据集的 SHACL Rules），与 SHACL 1.2 节点表达式（`shnex:`）、SPARQL 扩展与
  SPARQL 1.2 RL 工作草案对齐——节点表达式的 AF 拼写与 1.2 拼写解析为同一种表示，
  规则按 `sh:order` 分层运行并做 `once`/`general` 划分——外加对具体化节点形状的、有
  范围限定的 SHACL 1.2 支持。以上均不构成完整 SHACL 1.2 一致性的声称。在随库固化的
  W3C 测试套件上 **129/129 通过**，台账为空。答案是作为冻结 RDF 数据集的 W3C 验证报告
  （`ValidationReport::to_dataset()`），因此任何语法——以及 CLI 的 `validate --format`
  ——都是该数据集的一次序列化而非文本往返，报告所生成的空节点与数据图携带的每个
  空节点保持区分。
- **ShEx 2.1**——从零实现的 ShExC + ShExJ 模式层与验证器，对照官方 shexTest 套件把关：
  **1,105/1,105 个尝试的验证测试，零预期失败**（含 import 与语义动作），99/99 负例
  语法，14/14 负例结构。参见 [`docs/CONFORMANCE.md`](./docs/CONFORMANCE.md)。
- **蕴涵**——在确定性的半朴素不动点上进行的 Simple/RDF/RDFS/OWL-RL/D 前向物化
  （OWL 2 Profiles §4.3 表 4–9 的**全部 78 条 OWL 2 RL 规则**——这是*规则表覆盖率*，
  与蕴涵一致性不是同一个声称：在这份随库固化的 W3C OWL 2 RL 蕴涵测试语料上，chase
  的成绩是**正例 27/27、负例 23/23**，后者意味着未发现不可靠之处；全部 18 条
  RDF + RDFS 模式，其中四条存在性模式经由受限 chase 触发，其代理空节点保留在物化边界之内，
  不予输出）、一个开放世界的 OWL-Direct SHOIQ(D) hypertableau，以及 RIF-Core 规则。
  **每次求闭包都附带一份推理报告**，说明触发了什么、没触发什么、触及了哪些边界、
  消耗了多少预算，以及所运行演算的契约哈希——因此一个不完整的答案绝不可能被当作
  完整答案交付。有一条规则会触发而没有任何规范表格陈述它——`ext-eq-diff-sym`，
  `owl-rl` 下 `owl:differentFrom` 的对称性——它不在上面任一规则计数之内；
  `extensions(regime)` 会指名它，每份报告都在 `extension` 行披露它。逐规则清单：
  [`docs/book/src/entailment-rules.md`](./docs/book/src/entailment-rules.md)。
- **GTS 图传输**——面向 RDF 1.2 图及其引用的二进制对象的单文件、内容寻址、仅追加
  的容器：BLAKE3 链接的 CBOR 段、确定性的折叠、COSE 签名/加密、纯 Rust 密码学
  （对 wasm 友好）。规范见 [`docs/GTS-SPEC.md`](./docs/GTS-SPEC.md)，冻结的跨语言
  一致性向量见 [`vectors/`](./vectors/)。
- **切片、映射与溯源**——一个基于清单的切片目录，带内容寻址的工件 ID，一份显式的
  RDF↔GTS **损失台账**（[`generated/rdf-loss-matrix.json`](./generated/rdf-loss-matrix.json)），
  SSSOM 映射 TSV 支持，以及一个 FnO 函数目录编解码器。
- **零依赖的基础层**——`purrdf-iri`（RFC 3987/3986）与 `purrdf-xsd`（XSD 1.1 值
  空间）完全没有运行时依赖；`purrdf-events`（对象安全的摄入接缝）同样没有，而
  `purrdf-cdt` 是恰好建立在这两者之上的 `no_std` 封闭叶。

## 快速入门

### Rust

```sh
cargo add purrdf
```

```rust
use purrdf::{parse_dataset, serialize_dataset, RdfDatasetBuilder, RdfLiteral, SerializeGraph};

// Build a dataset in interned TermId space.
let mut b = RdfDatasetBuilder::new();
let alice = b.intern_iri("https://example.org/alice");
let knows = b.intern_iri("http://xmlns.com/foaf/0.1/knows");
let bob = b.intern_iri("https://example.org/bob");
let name = b.intern_iri("http://xmlns.com/foaf/0.1/name");
let hi = b.intern_literal(RdfLiteral::simple("Alice"));
b.push_quad(alice, knows, bob, None);
b.push_quad(alice, name, hi, None);
let ds = b.freeze().expect("freeze");

// Serialize to any native codec and parse back, losslessly.
let ttl = serialize_dataset(&ds, "text/turtle", SerializeGraph::Dataset).unwrap();
let back = parse_dataset(&ttl, "text/turtle", None).unwrap();
assert_eq!(back.quad_count(), 2);
```

### Python

```sh
pip install purrdf
```

```python
import purrdf

quads = purrdf.parse(
    '<https://example.org/alice> <http://xmlns.com/foaf/0.1/name> "Alice" .',
    purrdf.RdfFormat.TURTLE,
)

from purrdf import shapes, shex

report = shapes.validate(shapes_ttl=my_shapes, data_nt=my_data)
print(report["conforms"])

results = shex.validate(my_schema_shexc, my_data_ttl,
                        [("https://example.org/alice", "https://example.org/PersonShape")])
print(all(entry["conformant"] for entry in results))
```

每个解析入口点都为拼写了相对 IRI 的文档接受可选的 `base=`（`shapes.validate` 接受
`shapes_base=`）；作用域内没有基础 IRI 时，相对引用会抛出异常，而不是被错误解析。
`Store.query` 及其受调控与更新版本把属性函数当作数据来注册——冻结表、从存储
自身的图中读取的表，以及 `path_relations` 遍历——并在整个求值期间释放 GIL。

Python 包还附带一个 [rdflib 兼容层](./bindings/python/python/src/purrdf/compat/rdflib/)
（`from purrdf.compat.rdflib import Graph`）与 GTS 关系型导出
（`gts_to_sqlite`、`gts_to_duckdb`、`gts_to_parquet`）。

若需要逐字不改的 `import rdflib`，安装可选装的 extra：

```bash
pip install purrdf[rdflib]
```

这会拉入独立发行的 [`purrdf-rdflib`](./bindings/python-rdflib-shadow/) 分发包，其顶层
`rdflib` 包重新导出兼容层的 API，于是现有第三方代码中的 `import rdflib` /
`from rdflib.namespace import RDF` 便透明地运行在 `purrdf` 之上。**注意：**该影子包
占用了 `rdflib` 这一导入名，绝不可与真正的
[`rdflib`](https://pypi.org/project/rdflib/) 同时安装——二者无法共存于同一环境。它被
刻意做成独立分发包（从不打进主 `purrdf` wheel），正是为了让需要真实 rdflib 的环境
直接不装它即可。

### JavaScript / WebAssembly

同一引擎之上的 [RDF/JS](https://rdf.js.org/) 形态 API（`DataFactory` / `Dataset` /
`Stream`），包括没有哪个现存 RDF/JS 库承载的 RDF 1.2 特性——引用三元组项与基础方向
字面量：

```js
import { ready, DataFactory, Dataset } from "@blackcatinformatics/purrdf";

await ready(); // one-time async wasm instantiation

const f = new DataFactory();
const rtl = f.directionalLiteral("مرحبا", "ar", "rtl");

const ds = new Dataset();
ds.add(f.quad(f.namedNode("https://ex/s"), f.namedNode("https://ex/says"), rtl));

const nq = ds.serialize("nquads");           // directions survive the round-trip
const reparsed = Dataset.parse(nq, "nquads"); // Dataset.parse(input, format, base?)
```

同一个浏览器 bundle 还暴露 SHACL 验证（`shaclValidateToSarif`、`shaclEntail`，各接受
可选的 `shapesBase`）、蕴涵机制物化、带解释回执的受调控 SPARQL，以及 RDFC-1.0 图
同一性（`Dataset.canonicalize()`、`Dataset.isomorphic()`）。参见
[`crates/rdf-wasm`](./crates/rdf-wasm/)（`make wasm-pkg` 构建 ESM 包）。

### C

`libpurrdf`（[`crates/rdf-capi`](./crates/rdf-capi/)）在一个 panic 安全的 C ABI 之后
暴露解析、序列化、模式迭代、写时复制的变更、SPARQL、SHACL 验证/蕴涵与 GTS 往返，
附带一份已提交、可复现的头文件（[`include/purrdf.h`](./crates/rdf-capi/include/purrdf.h)），
CI 检查其漂移。用 cargo-c 构建：`make capi-build`。

## Crate 一览

| Crate | 它是什么 |
| --- | --- |
| [`purrdf`](./crates/purrdf/) | 门面 crate（umbrella crate）：根部是 RDF 接口，`slice` 与 `shapes` 作为模块。从这里开始。 |
| [`purrdf-rdf`](./crates/rdf/) | RDF 1.2 实现：原生编解码器、GTS 适配器、describe、规范化入口点。 |
| [`purrdf-core`](./crates/rdf-core/) | 内核：驻留 IR、诊断、存储 trait、溯源、损失台账、RDFC-1.0。 |
| [`purrdf-columnar`](./crates/columnar/) | 面向 RDF 1.2 与内容寻址 blob 的双向、字节级确定的五表 Parquet 编解码器。 |
| [`purrdf-gts`](./crates/gts/) | GTS 容器引擎：读取器、写入器、折叠、验证、COSE 签名/加密。 |
| [`purrdf-sparql-algebra`](./crates/sparql-algebra/) | SPARQL 1.1/1.2 解析器 → 查询代数 AST。 |
| [`purrdf-sparql-eval`](./crates/sparql-eval/) | 驻留 `TermId` 空间中的多重集 SPARQL 求值器，带有以调用方为键的扩展接缝（标量函数、属性函数——含路径见证与嵌入 k 近邻关系——自定义聚合，以及逐服务的 `ServiceResolver`）与执行 governor。 |
| [`purrdf-sparql-results`](./crates/sparql-results/) | SPARQL 结果的 JSON/XML/CSV/TSV，外加一个携带溯源的扩展。 |
| [`purrdf-cdt`](./crates/cdt/) | SEP-0009 SPARQL 复合数据类型（`cdt:List`/`cdt:Map`）：值空间、一个迭代式的有界词法扫描器、规范拼写，以及十五个函数的函数库。建立在 `purrdf-iri` + `purrdf-xsd` 之上的 `no_std` 封闭叶；经由求值器访问，不由门面 crate 重新导出。 |
| [`purrdf-shapes`](./crates/shapes/) | SHACL 验证引擎（完整 Core + SHACL-SPARQL + SHACL-AF，含 SHACL Rules）。 |
| [`purrdf-shex`](./crates/shex/) | ShEx 2.1：ShExC/ShExJ 模式与验证。 |
| [`purrdf-entail`](./crates/entail/) | 蕴涵机制：RDF/RDFS/OWL-RL/D chase、OWL-Direct tableau 与 RIF-Core 规则——每次求闭包都返回推理报告。 |
| [`purrdf-geo`](./crates/geo/) | GeoSPARQL 1.1：精确、无浮点的 WKT 与 GeoJSON 几何，标量接缝上的 `geof:` 函数族，以及属性函数接缝上的要素级查询重写——全部在调用方提供的 IRI 之下。 |
| [`purrdf-datalog`](./crates/datalog/) | chase 之下的不动点基底：一个列式关系存储与 DL 子句 IR 上的确定性半朴素求值器。不由门面 crate 重新导出。 |
| [`purrdf-text`](./crates/text/) | RDF 1.2 字面量上的确定性全文检索：一个内存倒排索引与精确定点 BM25 排名，从 SPARQL 经由调用方提供的属性函数 IRI 调用。 |
| [`purrdf-validate`](./crates/validate/) | 共享的宿主边界：SARIF 2.1.0 诊断，以及 Python/wasm/C 绑定所调用的蕴涵机制字符串接口。 |
| [`purrdf-slice`](./crates/slice/) | 切片目录：清单、带类型的工件、所有权/依赖分析。 |
| [`purrdf-iri`](./crates/iri/) | 零依赖的 IRI/URI 解析、规范化、CURIE，以及工作区唯一的 RFC 3986 基础解析层（`BaseIri`/`BaseScope`）。 |
| [`purrdf-xsd`](./crates/xsd/) | 零依赖的 XSD 1.1 值空间，带 SPARQL 数值提升。 |
| [`purrdf-events`](./crates/rdf-events/) | 零依赖、对象安全的 RDF 事件汇/源接缝。 |
| [`purrdf-wasm`](./crates/rdf-wasm/) | `purrdf` ESM 包背后的 wasm32 引擎。 |
| [`purrdf-capi`](./crates/rdf-capi/) | `libpurrdf` C ABI（不发布；经由 cargo-c 构建）。 |
| [`purrdf-cli`](./crates/cli/) | `purrdf` 命令行工具：`convert`、`query`、`update`、`reason`、`entails`、`consistency`、`validate`、`shex`、`describe`、`project`、`lift`、`pack`、`verify`（不发布）。 |
| [`purrdf-sparql-conformance`](./crates/sparql-conformance/) | W3C SPARQL、蕴涵机制与 OWL 2 一致性测试框架（不发布）。 |

## 文档

- **[RDF-1.2 演练场](https://blackcat-informatics.github.io/purrdf/playground/)**——
  零安装的浏览器控制台：解析、查询（SPARQL）、验证（SHACL）、序列化，以及 RDF-1.2
  （引用三元组、方向字面量）的规范化/比较，完全在 wasm 构建之上、于客户端完成。
  不需要工具链，不需要服务器。
- **[PurRDF 之书](https://blackcat-informatics.github.io/purrdf/)**——用户指南：各语言
  的入门、概念，以及每一个引擎（源码在 [`docs/book/`](./docs/book/)，`make book`
  在本地构建）。
- **API 参考**——门面 crate 见 [docs.rs/purrdf](https://docs.rs/purrdf)；每个成员
  crate 都从上面的 crate 一览链接到各自的 docs.rs 页面。
- **规范与报告**——[GTS 规范](./docs/GTS-SPEC.md)、
  [RDF 1.2 规范化 profile](./docs/RDF12-CANON-PROFILE.md)、
  [PURREMB 嵌入伴随件](./docs/PURREMB.md)、
  [SPARQL 执行 governor profile](./docs/SPARQL-GOVERNOR-PROFILE.md)、
  [一致性记分板](./docs/CONFORMANCE.md)、
  [基准测试](./docs/BENCHMARKS.md)、[发布流程](./docs/RELEASE.md)。
- **设计笔记**——核心 crate 之外的引擎为何在每个目标上给出相同答案：
  [全文评分](./docs/design/purrdf-text-scoring.md)、
  [GeoSPARQL 精确性](./docs/design/purrdf-geo-exactness.md)、
  [嵌入 k 近邻](./docs/design/purrdf-embedding-knn.md)。

## 快，靠测量而非断言

IR 把每个词项在字符串存储区中**只存一次**，以可复制的 `NonZeroU32` id 寻址，在所有
热点处用固定密钥的 `ahash` 做哈希，并把数据集冻结为 `Box<[QuadRow]>` 表，带惰性的
序数置换索引（每条四元组每个轴约 4 字节）。性能声称由 criterion 基准而非形容词
支撑——`crates/rdf-core/benches/ir_layout.rs` 度量结构数组、数组结构与谓词邻接三种
布局（分配次数、高水位、端到端延迟），最终采用的布局就是胜出的那个。用 `make bench`
运行它们。

还有一个仅供报告的 Python 测试框架，把原生支撑的 `purrdf.compat.rdflib` 直接替换包
与真实 `rdflib` 在解析、序列化、SPARQL 与三元组模式迭代上、于一个确定性的
`example.org` 语料上计时对比（`make bench-python`）。方法论、运行方式与一份有
代表性的（依宿主而异的）结果表见 [`docs/BENCHMARKS.md`](./docs/BENCHMARKS.md)。
数字因宿主而异——请在本地复现，而不要相信一个固定的倍数。

## 一致性

每个引擎都由其官方测试套件把关，随库固化并冻结在仓库内——完整记分板与运行方法见
[`docs/CONFORMANCE.md`](./docs/CONFORMANCE.md)：

| 引擎 | 套件 | 结果 |
| --- | --- | --- |
| ShEx 2.1 验证 | shexTest v2.1.0（`vectors/shexTest/`） | **1,105 / 1,105** 尝试，0 xfail |
| ShEx 模式 / 负例语法 / 结构 | shexTest v2.1.0 | **425/425 · 99/99 · 14/14** |
| SHACL | W3C data-shapes（`vectors/shacl/`） | **129 / 129**，0 例入账 |
| SHACL（第一方冻结语料） | `crates/shapes/corpus/` | **70 / 70** |
| SHACL Rules | DASH + 第一方（`vectors/shacl/af/rules/`） | **19 / 19** |
| 语法编解码器 | W3C rdf-tests 往返 | **264 / 264** |
| SPARQL 1.1/1.2 | 完整的 W3C sparql11 + sparql12 + 第一方，经由 `purrdf-sparql-conformance` | **862** 通过 · 5 例入台账（上游勘误） |
| SPARQL CDT（SEP-0009） | 随库固化的 `awslabs/SPARQL-CDTs`（`vectors/sparql-cdt/`） | **658 / 658**，0 例入账——词法空间分歧见 [`docs/CONFORMANCE.md`](./docs/CONFORMANCE.md) |
| SPARQL 执行 governor | 第一方冻结语料（`vectors/sparql-governors/`） | **50 / 50**，0 例入账 |
| 蕴涵（SPARQL 蕴涵机制） | W3C sparql11 `entailment/` 组 | **70 / 70**，0 例入账 |
| 蕴涵（OWL 2 DL 一致性） | 随库固化的 W3C OWL 2 套件 | **258 / 262** 一致，4 例入台账，0 未入台账 |
| 蕴涵（OWL 2 RL，W3C 蕴涵测试） | 随库固化的 W3C OWL 2 蕴涵套件 | **50 / 50** 一致，0 例入账，0 未入台账——负例通道 **23 / 23**（未发现不可靠之处），正例通道 **27 / 27** |
| RDFC-1.0 | W3C 规范化夹具 | 绿 |
| GTS | 冻结的跨语言向量（`vectors/`） | **38 / 39** 逐字节折叠为其已提交的期望值，1 处入台账的分歧 |

## 能力如何增长

SPARQL 的广度经由以调用方为键的扩展接缝增长——标量函数、属性函数、自定义聚合，以及
宿主注入的服务解析器——因此新能力总是以经由接缝的组合落地，绝不是 Cargo feature
标志，也绝不是 PurRDF 自行定义的词汇表。四元组形式的 `CONSTRUCT`、SEP-0008 的 SHA-3
内建函数、SEP-0009 复合数据类型、确定性全文检索、路径见证、嵌入 k 近邻与 GeoSPARQL 1.1
都是这样到达的：核心 crate 之外、在调用方提供的 IRI 之下、在每个目标上字节级一致，并接受与上述
一切相同的一致性纪律。

## 开发

```sh
make metadata   # regenerate + verify generated artifacts
make check      # fmt, build, tests, hygiene gates
make bench      # criterion benchmarks
```

发布由标签驱动，采用 OIDC 可信发布（crates.io 与 PyPI），附带构建溯源证明与 SPDX
SBOM——见 [`docs/RELEASE.md`](./docs/RELEASE.md)。

## 版本与 MSRV

**自 1.0.0 起的 semver。**自 1.0.0 起，本套件完整遵循语义化版本：**破坏性**变更提升
**主版本**——携带 `!` 或 `BREAKING CHANGE:` 的提交是主版本提升的触发条件，变更日志会把
每一条这样的条目标记为 **BREAKING**——**次版本**提升只做加法且 API 兼容，**修订版**提升
只含缺陷修复。这是版本号所承诺的内容；它并不是超出 semver 含义之外的稳定性声称。全部
三个已发布的包——crates.io 的 crate 套件、PyPI 的 `purrdf` 包与 npm 的
`@blackcatinformatics/purrdf` 包——共享**同一个**工作区版本并同步发布，CI 中的版本
一致性检查会在各版本来源（`Cargo.toml`、`pyproject.toml`、`package.json`、
`CITATION.cff`）不一致时让构建失败。唯一的例外是 C ABI。`libpurrdf` 的
[`purrdf.h`](./crates/rdf-capi/include/purrdf.h) 携带自己的
`PURRDF_ABI_MAJOR.PURRDF_ABI_MINOR`（当前为 **0.7**），在每次导出签名变更时提升，由
`crates/rdf-capi/tests/abi_signatures.rs` 固定，并在运行时经由 `purrdf_abi_version`
读回。它与工作区分开编号，并保持 `0.x`：它并未冻结，工作区的 1.0.0 对它不作任何承诺。

**MSRV 政策。**支持的最低 Rust 版本是根 `Cargo.toml` 中的 `rust-version`（当前为
**1.96**），位于 **stable** 通道，由专门的 CI MSRV 作业强制执行，发布工件也在 stable
上构建。提高 MSRV 是一项记入变更日志的显著变更；它随**次版本**提升进行，绝不在修订版发布中出现。
README 中的 MSRV 徽章由人工维护，必须与 `rust-version` 一同更新。

贡献者使用一个带日期的 nightly（`rust-toolchain.toml`）以获得更锐利的 clippy 与
rustdoc lint 覆盖面，但工作区**不含任何 nightly 独有特性**——MSRV 作业正是在每次变更上
证明这一点的手段。构建 PurRDF 只需要 stable 1.96，不需要其他任何东西。

## GMEOW 家族

PurRDF 是一小族关联数据项目的库层：

- [`gmeow-ontology`](https://github.com/Blackcat-Informatics/gmeow-ontology)——GMEOW
  以推理为核心的超级词汇表及其发布工具链（PurRDF 的主要消费者）。
- [`gmeow-gts`](https://github.com/Blackcat-Informatics/gmeow-gts)——GTS 规范及其
  多语言引擎；PurRDF 承载其中的 Rust 引擎。

抽取历史与源提交：[`PROVENANCE.md`](./PROVENANCE.md)。
品牌资产与使用规范：[`docs/BRAND.md`](./docs/BRAND.md)。

## 许可

以 [Apache License 2.0](./LICENSE-APACHE) 或 [MIT license](./LICENSE-MIT) 双许可发布，
由使用者任选其一，详见 [`LICENSING.md`](./LICENSING.md)。

若在研究中使用 PurRDF，请引用它——见 [`CITATION.cff`](./CITATION.cff)。
