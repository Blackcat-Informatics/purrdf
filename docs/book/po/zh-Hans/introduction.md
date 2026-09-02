<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->
<!--
zh-Hans 译稿（第一阶段）。与 docs/book/src/introduction.md 逐段对应，
待 mdbook-i18n-helpers 的 .po 工作流落地后倒入 msgstr。代码块与英文原文逐字节相同。
-->

# 引言

> 本译文对应 PurRDF 0.14.0 发布时的英文文档。尚未译出的段落以英文原样显示；运行时
> 字符串——命令行 `--help`、诊断消息与 Python `__doc__`——在本版本中不作本地化，
> 其稳定诊断代码的中文参考见 [诊断代码参考](project/diagnostic-codes.md)。

**PurRDF** 是一个 [RDF 1.2](https://www.w3.org/TR/rdf12-concepts/) 工具包：图原语、
编解码器、SPARQL、SHACL、ShEx、蕴涵与图传输，以 Rust 实现一次，并原样带入 Python、
WebAssembly/JavaScript 与 C。每一个已发布的 crate 都能构建到 `wasm32-unknown-unknown`，
因此在服务器上回答某条查询的引擎，也会在浏览器标签页中逐字节地给出同一答案。它由
黑猫信息科技（Blackcat Informatics® Inc.）开发，以 MIT OR Apache-2.0 发布。

> **同一个 RDF 引擎，同一套行为，每一种语言。**

**它从架构中移除了什么。**通常让一个 PostgreSQL 实例伴随三元组存储运行的三项工作——
带排名的全文检索、空间谓词与向量相似度——都在 PurRDF 内部得到回答：进程内、同一
数据集、同一条 SPARQL 查询，没有第二个数据库，也没有同步作业。每个答案都是精确且
确定的，原生与 wasm32 上皆然。这不是一个设想：这些查询表面已经从一个 RDF 项目中
移除了整个 PostgreSQL 依赖。见下文
[一个引擎取代三个数据库](#一个引擎取代三个数据库)。

## PurRDF 为何存在？

RDF 工具沿两条轴线碎片化。

**跨语言**：每个生态都有自己的解析器，带着自己的缺陷、自己对边角情形的解读，以及自己
所实现的规范子集。把一张图从 Rust 服务搬到 Python 流水线再搬到浏览器，数据的含义就已
被悄悄改变了三次。

**跨时间**：RDF 1.2——三元组项、具体化节点（reifier）、基础方向字面量——是标准的现行
修订版，而几乎没有哪个现存的库承载它。

PurRDF 之所以存在，是为了让一张图在任何地方都是**同一张图**。它是一个从零实现、依赖
极简的 Rust 核心——从解析器到 SPARQL 引擎到 SHACL 验证器再到二进制传输——经由原生
绑定暴露，而不是逐语言重新实现。使同一张 RDF 1.2 图行于 Rust、Python、WebAssembly
与 C 之间而其义不移——此即吾辈所架之津梁。

## 里面有什么

- **RDF 1.2 图原语**——一个不可变的、按值驻留的数据集 IR（`TermId` 空间、字符串存储区、
  写时复制的变更），带有宾语位置的三元组项、具体化节点/注解侧表，以及基础方向字面量。
  参见 [驻留数据集 IR](concepts/interned-dataset.md)。
- **原生编解码器**——Turtle、TriG、N-Triples、N-Quads、RDF/XML、TriX、HexTuples、
  JSON-LD (star) 与 YAML-LD 的第一方解析器/序列化器，输出字节级确定。参见
  [编解码器与确定性](concepts/codecs.md)。每种语法都经由同一个 RFC 3986 层解析相对
  IRI（国际化资源标识符）引用，而作用域内没有基础 IRI 的相对引用是硬错误。参见
  [基础 IRI 与相对引用](concepts/base-iris.md)。
- **规范化**——W3C RDFC-1.0，外加数据集 diff 与同构判定。参见
  [规范化与 Diff](concepts/canonicalization.md)。
- **投影与载体**——确定性的 LPG、CSVW、OBO Graphs、数据集描述与研究对象投影，每一种
  都带有定位到具体位置的损失台账。参见
  [图、表格与研究对象投影](concepts/projections.md)。
- **SPARQL 1.1/1.2**——原生解析器 → 代数 → 多重集求值器，带有完整的 Update、SEP-0002
  时间算术、`LATERAL`（SEP-0006）、SEP-0008 的 SHA-3 哈希内建函数
  （`SHA3-224`/`256`/`384`/`512`）、能 `CONSTRUCT` 进命名图的四元组模板（这是第一方
  扩展，并非 SPARQL 1.2 特性）、SEP-0009 复合数据类型（`cdt:List`/`cdt:Map`、`FOLD`、
  `UNFOLD`——并有一处明确陈述的分歧：PurRDF 允许 RDF 1.2 三元组项与带方向的语言标签
  字面量作为复合元素，这是一个词法超集，符合 SEP-0009 的读取器会将其判为格式错误）、
  调用方注册的聚合函数与属性函数（包括逐跳绑定遍历过程的路径见证）、带逐节点解释回执
  的受调控执行，以及经由宿主注入的、携带逐服务上下文的解析器进行的 `SERVICE` 联邦
  查询，全部由 W3C 一致性套件把关。参见 [SPARQL](sparql/querying.md)。
- **核外 SPARQL 扩展**——采用精确定点 BM25 的确定性全文检索
  （[全文检索](sparql/full-text.md)）、精确且无浮点的 GeoSPARQL 1.1
  （[GeoSPARQL](sparql/geosparql.md)），以及在 PURREMB 嵌入空间上的最近邻搜索
  （[嵌入最近邻](sparql/embedding-knn.md)）——每一个都是扩展接缝的消费者，在调用方
  提供的 IRI 之下注册。
- **SHACL 与 ShEx**——两种形状语言的原生验证器；SHACL 引擎覆盖 Core、SHACL-SPARQL 与
  SHACL-AF，并与 SHACL 1.2 的节点表达式与规则分层草案对齐。参见
  [验证](validation/shacl.md)。
- **蕴涵**——Simple/RDF/RDFS/OWL-RL/D 物化（全部 78 条 OWL 2 RL 规则均已实现——这是
  规则表覆盖率，有别于蕴涵一致性；在这份随库固化的 W3C 语料上，OWL 2 RL 蕴涵测试的
  成绩是正例 27 中 27、负例 23 中 23）、一个 OWL-Direct tableau，以及 RIF-Core 规则，
  每次求闭包都附带推理报告。参见 [蕴涵](entailment.md)，其求值基于
  [Datalog 不动点引擎](datalog.md)。
- **GTS 图传输**——面向 RDF 1.2 图与二进制载荷的单文件、内容寻址、仅追加的容器。
  参见 [GTS 图传输](gts.md)。
- **切片、映射与溯源**——切片目录、显式的 RDF↔GTS 损失台账、SSSOM 与 FnO。参见
  [切片、映射与溯源](slices.md)。

## 一个引擎取代三个数据库

需要带排名的文本检索、空间谓词或最近邻搜索的 RDF 项目，通常正是为了这三项工作而在
三元组存储旁边运行一个 PostgreSQL。PurRDF 从 SPARQL 中回答全部三项，在已经位于内存中
的数据集上，经由求值器的、以调用方为键的扩展接缝——下面每个页面都以「它的表面取代了
什么、止步于何处」开篇。

| 你需要的 | 通常来自 | 如今在 PurRDF 之内 | 止步于何处 |
| --- | --- | --- | --- |
| 带排名的全文检索 | PostgreSQL `tsvector`/`tsquery` | [全文检索](sparql/full-text.md)：`purrdf-text`，一个覆盖 RDF 1.2 字面量的倒排索引，以精确的 `i128` 定点数做 BM25 排名，crate 内没有浮点。 | 是 BM25 排名，不是 Lucene：没有词干提取，没有停用词表，没有查询方言；一个在冻结数据集上一次性构建的内存索引。 |
| 空间谓词 | PostGIS | [GeoSPARQL](sparql/geosparql.md)：`purrdf-geo`，GeoSPARQL 1.1，WKT 与 GeoJSON 读作精确有理数，Simple Features、Egenhofer 与 RCC8 的每一种关系都在精确的 DE-9IM 上判定；没有 GEOS，没有 PROJ。 | 是矢量几何上的拓扑谓词、访问器与可精确计算的度量，不是 PostGIS：没有 CRS 变换，没有椭球大地测量，没有缓冲区、凸包/凹包、叠加集合运算或栅格——每个未实现的函数都按名称硬错误。 |
| 向量相似度 | pgvector | [嵌入最近邻](sparql/embedding-knn.md)：在 PURREMB 嵌入空间上的精确 top-k，binary64 且累加顺序固定。 | 由调用方提供的 `KnnGuard` 限定的精确扫描，三种度量，没有近似索引；PurRDF 不计算嵌入——向量来自调用方产出的工件。 |

三者在每个目标上都是其输入的纯函数——定点数、精确有理数，或固定顺序的 binary64，
外加规范的并列判定——而这一声称是被执行而非被论证的：文本与 k 近邻的确定性测试在
原生与 `wasm32-unknown-unknown` 上运行同一份测试体，`make geo-determinism` 则逐字节
比较两个目标。它们是 Rust 宿主上的接缝：宿主在自己的 IRI 之下注册一个索引或一个空间，
而该宿主本身也可以编译到 wasm32。已发布的 npm 包与 Python wheel 尚未暴露这三种关系。

## 第一天就值得知道的两条设计规则

**没有 feature 标志——永远没有。**整个工作区刻意不含任何 Cargo feature 标志，CI 强制
执行这一点。数据载体不得有可选行为：可选性会让语义因消费者而异，因此每个消费者得到的
是同样的、字节级一致的语义。

**PurRDF 是工具包，不是本体——它不铸造任何词汇表 IRI。**库所读写的每一个词汇表都是
调用方提供的配置，没有任何杜撰的默认值。在缺少其词汇表的情况下使用某项功能，要么
硬错误，要么保持不激活；它绝不会替你发明一个 IRI。（测试夹具使用 `example.org`。）

完整的不变量清单见 [设计规则与不变量](project/design-rules.md)。

## 为什么是 RDF 1.2？

RDF 1.2（以及 SPARQL 1.2）为数据模型加入了一等的陈述级元数据：可以出现在宾语位置的
**三元组项**、为三元组的某次出现命名的**具体化节点**，以及用于双向文本的**基础方向
字面量**（`rdf:dirLangString`）。PurRDF 把它们当作核心数据模型而非扩展：它们流经 IR、
编解码器、SPARQL、SHACL（作为一项有范围限定的 SHACL 1.2 特性）、RDF/JS 表面与 GTS
传输。参见 [RDF 1.2 特性](concepts/rdf12.md)。

## PurRDF 的位置

PurRDF 是一小族关联数据项目的库层：它是
[GMEOW](https://github.com/Blackcat-Informatics/gmeow-ontology) 技术栈的数据骨干，
也是 Rust 版 [GTS](gts.md) 引擎的参考归宿——但它对你的本体或应用不作任何假设。

## 如何阅读本书

- 新用户：从你所用语言的 [入门](getting-started/rust.md) 开始，然后阅读
  [概念](concepts/interned-dataset.md) 各章。
- 引擎用户：直接跳到 [SPARQL](sparql/querying.md)、[验证](validation/shacl.md)
  或 [蕴涵](entailment.md)。
- 集成者：参见 [互操作](interop/rdflib.md) 与 [GTS 图传输](gts.md)。
- 贡献者：阅读 [项目](project/design-rules.md) 各章，然后阅读仓库中的
  [AGENTS.md](https://github.com/Blackcat-Informatics/purrdf/blob/main/AGENTS.md)。

API 参考文档位于 [docs.rs/purrdf](https://docs.rs/purrdf)；仓库是
[github.com/Blackcat-Informatics/purrdf](https://github.com/Blackcat-Informatics/purrdf)。
