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
  <a href="https://coderabbit.com"><img src="https://img.shields.io/coderabbit/prs/github/Blackcat-Informatics/purrdf?utm_source=oss&utm_medium=github&utm_campaign=Blackcat-Informatics%2Fpurrdf&labelColor=171717&color=FF570A&link=https%3A%2F%2Fcoderabbit.ai&label=CodeRabbit+Reviews"></a>
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

**5.0 版使规范形式与内容同一性具备抗碰撞性，并为网络操作提供由调用方提供的统一交换。**
默认的规范形式通过嵌套的颜色细化（colour refinement）为空节点标号，并用 BLAKE3 对
所得的行序列求哈希。4.x 算法仍可作为 `purrdf_core::rdfc12` 调用，所有已存储的 4.x
摘要都能在其中重新计算（[迁移指南](./docs/MIGRATION-5.0.md)指明了唯一一个 IRI 拼写
方面的例外）。行集合的同一性是对已排序行摘要求得的 BLAKE3，数据集的内容同一性仅在
调用方提出要求时才计算。`SERVICE` 与 `LOAD` 共用一个 `HttpExchange`。GTS 读取器把不
陈述任何三元组的三元组项投影为一个全新的空节点，损失条目记录带类型的出现，不受支持的
签名算法验证结果为 `Unverified`。下文的逻辑引擎、文档载体、经认证的大地测量与操作性
存储保有其显式、可检视的契约。

本检出版本为 PurRDF 5.0.0。下文的注册表徽章与安装命令指的是各注册表中可获得的版本。
从 4.x 迁移时请阅读 [5.0 迁移指南](./docs/MIGRATION-5.0.md)；
[4.0 迁移指南](./docs/MIGRATION-4.0.md)涵盖 3.x。

**同一个 RDF 引擎。同一套行为。每一种语言。**各语言接口暴露的是工具包的不同部分，
详见下文。每一个已发布的 Rust crate 也都能构建到 `wasm32-unknown-unknown`。

[试用浏览器演练场](https://blackcat-informatics.github.io/purrdf/playground/)
· [阅读《PurRDF 之书》](https://blackcat-informatics.github.io/purrdf/zh-Hans/)
· [Rust API](https://docs.rs/purrdf)
· [迁移到 5.0](./docs/MIGRATION-5.0.md)
· [早期指南：迁移到 4.0](./docs/MIGRATION-4.0.md)

## 你可以构建什么

| 任务 | PurRDF 提供 |
| --- | --- |
| 让 RDF 1.2 贯穿整个应用 | 驻留（interned）数据集、三元组项、具体化节点（reifier）、注解、基础方向字面量与命名图；原生文本编解码器、规范化、差异比较与同构判定。 |
| 查询与修改图 | SPARQL 1.1/1.2 查询与 Update、预备执行、属性路径、宿主扩展、资源 governor（执行调控器）与解释回执（explain receipt）。 |
| 验证与推导知识 | SHACL 1.2、ShEx 2.1、JSON Schema、增量 Datalog、经认证的存在式 chase、目标导向的基础归结、OWL-Direct 与 RIF-Core。 |
| 跨多种信号检索 | 精确定点 BM25、GeoSPARQL 谓词、精确嵌入 k 近邻、确定性 HNSW，以及附带逐生产者（per-producer）证据的倒数排名融合（reciprocal-rank fusion）。 |
| 让文档可被查询 | 有序 JSON、结构化 Markdown、无损 TOML 1.1 与保留原始字节的 MIME 覆盖，带显式 profile 与经验证的重建。 |
| 交换图数据 | 带二进制载荷的 GTS 容器；规范的五表 Parquet；带损失记录的图、表格与 Research Object（RO）投影。 |
| 在预算之内读取不可变快照 | 经认证（certified）的分段存储、经鉴别（authenticated）的区间读取、钉住（pinned）的词项、稀疏缓存，以及经由普通数据集/求值器扩展点进行的共享工作区准入。 |

在 Rust 中，从 [`purrdf`](./crates/purrdf/) 门面 crate 开始。它在根部暴露 RDF 接口，
并把其他引擎作为模块暴露。应用自行提供其词汇表、扩展注册、网络传输与嵌入模型。标准
RDF 词汇表中的词项是内置的；应用命名空间是显式配置。

## 逻辑引擎

PurRDF 既支持物化推论，也支持回答某个特定目标。共享的
[`datalog`](./crates/datalog/src/lib.rs) 模块经由门面 crate 提供，与
[`entail`](./crates/entail/) 和 [`shapes`](./crates/shapes/) 并列。程序、所选上限与
语义法则都参与内容同一性，因此一个结果标识的是它所运行的契约。

**一种规则表示，多种执行策略。**DL 子句表达原子、合取、存在、析取与不一致性的头部。
分层的半朴素求值器、受限存在式 chase 与 OWL-Direct tableau 消费各自支持的片段，并对
不支持的形式显式拒绝。三元组程序与四元组程序使用同一套原生机制。守卫、原生过滤器与
有序调度经由共享求值器承载 SHACL 与 SPARQL 的规则表达式。

**一次准备，跨变更维护。**需求（demand）查询把一个目标及其规则编译为可复用的程序。
带符号的插入与撤回更新一个被保留的模型，包括递归支持与分层否定，同时保持原有的字典
同一性。暂停的维护保留其工作；答案与净成员变化在事务完成的边界处才变得可用。宿主显式
选择逻辑上限与存储准入。未声明的事实、连接与词项区域（arena）上限不施加逻辑上的限制；
物理存储与计数器的限制仍然适用。生成词项的规则有各自声明的上限。资源拒绝绝不会冒充
完整的模型。

**在执行之前准入存在式程序。**受限 chase 检查经常量细化的弱无环性（weak acyclicity），
并把预备执行绑定到原始程序及其终止证书。合取头部共享其存在见证。准入失败会指明不受
支持的形式或终止义务；它并不声称每一个被拒绝的程序都会发散。参见
[chase 契约](./crates/datalog/src/chase.rs)。

**提出一个目标并检视其证明。**原生的反向归结作用于共享的复合词项 DAG，采用有序排序
合一与表化（tabled）求值。拥有所有权的基础答案在工作区域释放之后，仍保留其结构绑定、
原始 RDF 值与检查证据。真值探测区分已确立为真、为假或良基（well-founded）未定义，
与未求值的原子或不完整的搜索。部分搜索可以确立一个正面的证明；它不能仅仅因为答案尚未
出现就确立为假。独立的证明检查把证明绑定到源、程序、排序以及原始运行契约。这是所发布
的基础答案契约；符号化的残余答案与高阶执行不在其范围之内。参见
[归结](./crates/datalog/src/resolve_fol.rs)与
[证明检查](./crates/datalog/src/resolve_fol/proof.rs)。

**使用成熟的 RDF 推理机制。**RDF、RDFS、OWL 2 RL 与数据类型物化、开放世界的 OWL-Direct
服务以及 RIF-Core 位于原生引擎之上。SHACL 推理规则使用同一套有序规则基底。Python、C、
JavaScript 与 CLI 暴露选定的推理操作；上文所述的复合词项构造、维护与结构化证明 API
是 Rust API。[推理指南](./docs/book/src/entailment.md)描述了每种机制及其完成边界。

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
  GeoSPARQL 1.1 拓扑谓词。其共享的[`地理内核`](./crates/geo-kernel/)提供经认证的椭球
  大地测量、显式的坐标操作链、缓冲区、叠加运算、层级式单元/覆盖与空间索引。标准 profile
  把 `CRS84` 与 GeoJSON 解释为 WGS 84 经度/纬度；其他参考系、单位与变换需要显式注册。
  数值证明与资源策略产生已完成的值或带类型的拒绝。[地理 profile](./docs/GEO-PROFILE.md)
  规定了运算法则、坐标网格与宿主请求接口。
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

[`purrdf-toml`](./crates/toml/) 保留 TOML 1.1 的书写出现，并通过重新解析来验证重建的
字节。[`purrdf-mime`](./crates/mime/) 在不可变的已分析文档中保存 RFC 5322/MIME 的源
字节、有序的头部、嵌套的部分与带类型的缺陷。附件解码与严格的 RDF 重建使用该原始文档。
编辑其源需要重新分析。[`purrdf-yaml`](./crates/yaml/) 涵盖原始的 YAML 呈现，包括其无环
源 profile 中的有序出现、标签与别名；这一源载体与 YAML-LD 的语义处理相互独立。
[`purrdf-glob`](./crates/glob/) 在显式的字面量或 IRI 词法面之上编译 Unicode 模式，并
以确定性的方式拒绝过量工作。

**转换对损失如实交代。**原生编解码器覆盖 Turtle、TriG、N-Triples、N-Quads、RDF/XML、
TriX、HexTuples、JSON-LD 与 YAML-LD。JSON-LD 上下文透镜（context lens）编译可复用的
离线上下文；它提供展开、压缩、派生前缀模式以及原生的 JSON-LD/YAML-LD framing，上下文
注册表由调用方以离线方式提供，并带有显式的策略与上限。它不安装网络加载器。图与
Research Object 投影包括 Neo4j CSV、openCypher、GraphML、CSVW、OBO Graphs、SKOS、
DCAT、VoID、RO-Crate、Croissant、DataCite 与 Frictionless。受支持的可逆载体经由精确的
边带（sideband）保留 RDF 1.2；有损视图报告丢失了什么。损失条目记录带类型的出现、
一个 `site`、一个 `subject` 以及有序的 `detail` 成员，运行时台账 JSON 的模式版本为 2。
参见[投影](./docs/book/src/concepts/projections.md)与
[生成的损失矩阵](./generated/transcode-loss-matrix.json)。

**唯一一种规范的列式投影。**
[`purrdf-columnar`](./crates/columnar/) 提供原生的五表 v1 契约：`terms`、`quads`、
`reifiers`、`annotations` 与 `blobs`。Python 的 SQLite、DuckDB 与 Parquet 导出器使用
其规范 ID 与表结构，保留带作用域的空节点、被引用的词项、方向、空的命名图以及经过验证的
blob。GTS 按追加顺序分配的检视 ID 仍是另一套独立的权威。

**两条不可变存储路径。**即时（eager）加载的 pack 快照提供前端编码（front-coded）的
字典、位图索引与内容验证。[`SegmentedSession`](./crates/rdf-core/STORAGE.md) 通过宿主的
区间提供者（range provider）读取一种经认证的持久表示，鉴别所准入的块，并在
稀疏缓存、钉住、证据与算子工作区之间共享一份实时台账。它在封存与重新打开之间保持全局
ID 不变；持久化的句柄标识确切的快照。

常驻内存的数据集保留紧凑的四字节词项 ID、十六字节的四元组行以及借用式的词项访问。
操作性读取经由同一个 `DatasetView` 扩展点，使用带类型的失败与钉住守卫。存储故障会在
操作的最终检查点丢弃其结果。有界查询准入覆盖基于基本图模式、仅投影变量的普通
`SELECT`；尚未定价的操作性形式返回带类型的拒绝。构建、完整认证、宿主缓冲区与调用方
拥有的输出各有独立的内存义务。内核不执行任何文件系统或网络 I/O。确切边界与已存储工件
的重建要求见[存储契约](./crates/rdf-core/STORAGE.md)与
[4.0 迁移指南](./docs/MIGRATION-4.0.md)。`SegmentedBuilder` 写出分段镜像格式第 8 版
（`PURRSEG8`），其源检查点带有 BLAKE3 Merkle 节点；第 7 版镜像仍可认证并打开。

Rust 的 `explain_query_fallible_view` 直接度量操作性视图，并连同最终的存储证据返回其
解释。它的选项与停止信号变体共用查询准入规则；存储故障会丢弃整个解释，而仅发生停止时，
停止会出现在解释的 governor 证据中。

**图连同其载荷一起传输。**[GTS](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md) 是一种内容寻址、仅追加的容器，
具备确定性折叠、二进制载荷、链式 CBOR 段、COSE 签名/加密与纯 Rust 密码学。其容器 API
覆盖 Rust、Python 与 C，CLI 把它作为输入格式读取。npm / JavaScript 包不暴露它。冻结的
传输向量与[权威的 GTS 项目](https://github.com/Blackcat-Informatics/gmeow-gts)共享，
`vectors/` 顶层的平铺字节冻结守卫覆盖所携带的向量。
`purrdf_gts::writer::StreamWriter` 每次一帧地把日志按帧写入调用方的接收端，与
`Writer` 的字节逐一相同。

**GTS 读取投影文档所陈述的内容。**既无 `tt` 也无 `rf` 的三元组项，以及指向自身的项，
都不陈述任何三元组。每条公共路径（折叠视图、数据集与图导入、流式导入器以及 N-Quads
导出）都把这样的项投影为一个全新的空节点 `_:unbound_triple_{id}`，任何文档都无法指称
它。被具体化者是字面量或被引用三元组的 `reifies` 行会被丢弃，并附带一条
`PositionConstraint` 诊断。载荷不可能是其编解码器所对应流的 blob，作为不透明节点折叠，
并带有 `DamagedFrame`。把文件读入数据集时，语言标签被折叠为小写，每一次折叠都报告为
一条 `language-tag-case-folded` 损失。对于已损坏的文件，Python 的折叠视图抛出
`ValueError`。

**签名结果区分被检查过的内容。**受保护头部未按确定性方式编码的 COSE_Sign1 为
`Invalid`。其所声明的算法为本引擎所未实现的签名，验证结果为
`SigStatus::Unverified`；`Invalid` 仍留给已检查但失败或格式错误的签名。要求已验证签名
的调用方只接受 `SigStatus::Valid`。certify 的调用方在解析时拒绝不受支持的算法。已签名
语料中的 68 个 `sigalg-*` 向量陈述了一种受保护头部语法，而 GTS 规范第 1 版并不包含
这种语法，因此它们不在一致性声明之内。

## 规范形式与同一性

**规范输出是嵌套细化标号。**Rust、Python、WebAssembly、C 与 CLI 中的默认入口都路由到
标号 `nested-refinement-labelling` 第 1 版。仅在细化无法拆分的单元上运行搜索，有限的
搜索界限是带类型的 `limit` 拒绝，嵌套在另一个三元组项中的三元组项降低为一个词项节点，
因此降低后的集合随嵌套深度线性增长。规范文档是以 N-Quads 1.2 拼写、采用十进制空节点
标签（`_:0`、`_:1`）的规范行序列；规范摘要是其 32 字节的 BLAKE3 摘要，并对每个数据集
都发生变化。5.0 的文档把 U+FFFE 与 U+FFFF 写作 `\uFFFE` 与 `\uFFFF`，这是 RDF 1.2
N-Quads 第 3 节所要求的。参见
[空节点的规范标号](./docs/design/purrdf-canonical-labelling.md)。

**4.x 算法仍可调用。**RDFC-1.0 仍是一个具名入口，W3C 套件原样通过。4.x 算法位于
`purrdf_core::rdfc12`（亦即 `purrdf_rdf::rdfc12`，profile 第 3 版）之下，作为已存储
摘要的读取侧重新计算。它如 4.0.2 一样把 U+FFFE 与 U+FFFF 原样写出，因此每个已存储的
4.x 摘要都能重新计算；唯一的例外是调用方自己的 `DatasetView` 所提供的、含有 U+007F
或 U+0080 至 U+009F 的 IRI。没有任何写入器在新工件中记录 4.x 摘要：PACK 与 PURREMB
写出格式 2，GTS 压缩证书携带 `refold_canon`，读取器同时接受两代。参见
[profile](./docs/RDF12-CANON-PROFILE.md)与
[迁移指南](./docs/MIGRATION-5.0.md#stored-digests)。

**行集合的同一性是对已排序行摘要求得的 BLAKE3。**内容同一性、执行回执、
`SourceVersion::content_fingerprint`、文本索引的源指纹与代次，以及表面（surface）代次，
不再使用选择行的一方可以使之碰撞的加法求和。4.x 所记录的值不再匹配，需要从数据重新
计算。`Multiset256::fingerprint` 是 `accounting_check`，`GeoIndex::source_fingerprint`
是 32 字节摘要，`TextIndex::with_delta` 拒绝移除索引中并不持有的行
（`TextDeltaError::UnheldRemoval`）。

**内容同一性按需计算。**冻结、视图构造或查询都不会计算它；`ContentIdentified::content_identity`
才会，因此新数据集上的首次受调控查询不会对每一行求哈希。宿主在 CLI 中用
`query --content-identity`、在 Python 中用 `QuadStore.bind_execution_receipt`、在
WebAssembly 中用 `Dataset#bindExecutionReceipt`、在 C 中用
`purrdf_execution_receipt_bind`、在 Rust 中用
`purrdf_validate::query::bind_execution_receipt(receipt, &view)`，把查询回执绑定到数据集
的内容，Rust 中的后者接受一个实现了 `ContentIdentified` 的视图。宿主回执携带
`runIdentity` 与 `readsFocusGraph`。

**守卫以拒绝代替关闭。**无法读取的栈下限通过 `purrdf_stack::try_remaining` 与
`FloorError` 拒绝，而不是停用守卫。一个链接了求值器但未链接 `purrdf-wasm` 的
`wasm32-unknown-unknown` 模块，要么安装宿主时钟与熵源，要么提供查询时间与种子，否则
请求以 `EvalError::HostSourceMissing` 被拒绝。`Iri::normalize` 保留已编码的点段
（`a/%2e/b`），并把百分号解码所暴露的主机字母转为小写。
`InferOptions::with_never_derive_declarations` 拒绝推导受保护谓词的 SRL 规则。

## 验证、推理与可问责的执行

SHACL 1.2 涵盖 Core、SPARQL 扩展、节点表达式、推理规则与 SPARQL 1.2 RL，SHACL-AF 的
拼写映射到同一种共享表示。报告是 RDF 数据集。ShEx 2.1 提供 ShExC/ShExJ 模式与验证，
包括导入与语义动作。导入从显式的宿主注册表解析。Rust 模式编译器还把形状投影为
JSON Schema、OpenAPI、Pydantic、LinkML、TypeScript 与 GraphQL，并附覆盖率与损失报告；
这些模式通道仅限 Rust。

原生的 [`JSON Schema`](./crates/jsonschema/) 以精确的十进制数、动态/递归引用与
ECMA-262 模式求值 draft 2020-12、2019-09 与 07。元模式、资源注册表与引用策略由调用方
提供。资源耗尽是带类型的求值错误，有别于无效的实例。ShEx 同样区分匹配器拒绝与已完成
的验证结果。

蕴涵引擎实现了**全部 78 条 OWL 2 RL 规则**与全部 18 条 RDF + RDFS 模式。这是规则表
覆盖率，有别于蕴涵一致性：在随库固化（vendored）的 W3C 语料上，chase 的成绩是
**正例 27/27、负例 23/23**，后者记录的是未发现不可靠之处。额外的 `ext-eq-diff-sym`
规则在推理报告中披露，且不计入上述规则数。OWL-Direct 提供开放世界的 SHOIQ(D) tableau
以及附带证书的推理服务；被上限截停的搜索返回 `unknown`。RIF-Core 与蕴涵感知的 SPARQL
与各物化机制并行运行。参见[规则清单](./docs/book/src/entailment-rules.md)与
[推理服务](./docs/book/src/entailment.md)。

查询 governor 约束执行，并在触顶时返回相关证据，包括在非单调算子之下哪些行可以得到
认证。受调控的 Update 要么完整提交，要么完全不提交。预备执行与有作用域的回调复用同一个
求值器。计费表与冻结的 57 例 governor 语料发布在
[governor profile](./docs/SPARQL-GOVERNOR-PROFILE.md) 中。结构化诊断保留稳定的代码与
可读的英文，并在 JSON、SARIF 与 C 诊断记录中带有具名参数与精确的逻辑锚点。

## 快速入门

### Rust

```sh
cargo add purrdf --git https://github.com/Blackcat-Informatics/purrdf.git
```

该 Git 依赖使用当前源码。需要由注册表固定版本的依赖时，请使用 crates.io 上已发布的版本。

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

随着应用的增长，可使用门面 crate 的 `sparql`、`shapes`、`shex`、`datalog`、`entail`、
`retrieval`、`geo`、`columnar`、`json`、`yaml`、`toml`、`mime` 与 `markdown` 模块。
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
异步查询可经由 JSPI 使用宿主提供的 `SERVICE` / `LOAD` 解析器。数据集的同一性与代次
（generation）为 `bigint`；在 JSON 中请使用十进制字符串。
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
也没有文档内基准 IRI 可用时，未解析的相对 IRI 会被诊断出来。
网络访问由宿主提供：`SERVICE` 与 `LOAD` 共用一个由调用方提供的 `HttpExchange`，当宿主
把每一次重定向交还给它时，`HttpGraphResolver` 会对其所跟随的每一跳重定向进行授权与
准入。随库发布的同步接口不安装任何联邦解析器，核心也不附带 HTTP 客户端。

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
| [`purrdf-datalog`](./crates/datalog/) | 半朴素求值、增量维护、预备需求、存在式 chase 与基础归结。 |
| [`purrdf-entail`](./crates/entail/) | 物化、OWL-Direct 与 RIF-Core。 |
| [`purrdf-text`](./crates/text/) | 精确定点的全文检索。 |
| [`purrdf-geo`](./crates/geo/) | 精确的 GeoSPARQL 几何与关系。 |
| [`purrdf-geo-kernel`](./crates/geo-kernel/) | PurRDF 各宿主共享的确定性几何、大地测量与空间索引。 |
| [`purrdf-hnsw`](./crates/hnsw/) | 确定性的近似最近邻索引。 |
| [`purrdf-retrieval`](./crates/retrieval/) | 带类型的规划、执行以及附带证据的排名融合。 |
| [`purrdf-columnar`](./crates/columnar/) | 规范的五表 Parquet 编解码器。 |
| [`purrdf-gts`](./crates/gts/) | 容器、折叠、验证与密码学。 |
| [`purrdf-markdown`](./crates/markdown/) | 结构化 Markdown 编解码器。 |
| [`purrdf-json`](./crates/json/) | 有序 JSON 字节覆盖编解码器。 |
| [`purrdf-toml`](./crates/toml/) | 无损的 TOML 1.1 模型与经验证的 RDF 字节覆盖编解码器。 |
| [`purrdf-mime`](./crates/mime/) | 保留原始字节的无损 MIME 编解码器，带有序出现位置与共享覆盖验证。 |
| [`purrdf-glob`](./crates/glob/) | 编译式 Unicode glob 匹配，带显式匹配面与计量执行。 |
| [`purrdf-jsonschema`](./crates/jsonschema/) | 原生 JSON Schema，支持 draft 2020-12、2019-09 与 07。 |
| [`purrdf-slice`](./crates/slice/) | 切片目录、工件所有权与依赖。 |
| [`purrdf-validate`](./crates/validate/) | 共享的验证、governor 与诊断宿主边界。 |
| [`purrdf-iri`](./crates/iri/) | IRI/URI、语言标签、基准 IRI 解析与标准词汇表。 |
| [`purrdf-xsd`](./crates/xsd/) | XSD 值空间、精确数值与时间算术。 |
| [`purrdf-cdt`](./crates/cdt/) | SPARQL 复合数据类型及其函数库。 |
| [`purrdf-events`](./crates/rdf-events/) | 摄入协议与共享的 RDF 1.2 文本方向类型。 |
| [`purrdf-lex`](./crates/lex/) | 共享的终结符、Unicode，以及 JSON/YAML/CBOR/XML 编解码器。 |
| [`purrdf-hash`](./crates/hash/) | 零依赖的哈希与共享的同一性内核。 |
| [`purrdf-deflate`](./crates/deflate/) | 原生、确定性的 DEFLATE/gzip。 |
| [`purrdf-zstd`](./crates/zstd/) | 原生 Zstandard 帧与确定性的 FastCOVER 字典训练。 |
| [`purrdf-yaml`](./crates/yaml/) | 无损的 YAML 文档模型与 RDF 覆盖。 |
| [`purrdf-ed25519`](./crates/ed25519/) | Ed25519 签名与严格验证。 |
| [`purrdf-stack`](./crates/stack/) | 原生/WASM 栈准入。 |
| [`purrdf-wasm`](./crates/rdf-wasm/) | 面向 JavaScript 的引擎与 ESM 绑定。 |
| [`purrdf-testkit`](./crates/testkit/) | 已发布的开发库：黄金文件、性质与基准测试框架。 |
| [`purrdf-alloc-probe`](./crates/alloc-probe/) | 已发布的开发库：分配度量。 |
| [`purrdf-conformance-kit`](./crates/conformance-kit/) | 已发布的开发库：一致性评分。 |

这 40 个 crate 发布到 crates.io。C ABI、CLI、Python 扩展、一致性测试框架与基准工具位于
同一个工作区，但不作为 Cargo 包发布。共享的第一方词法、编解码、哈希与签名基础层缩小了
外部依赖面；每项共享工作都有唯一一个强制执行的归属。没有任何语义性的 Cargo feature，
因此安装时的 feature 选择无法改变载体的行为。

## 证据与性能

[一致性记分板](./docs/CONFORMANCE.md) 区分官方套件、第一方语料、经批准的分歧与未经
测试的边界。SPARQL 求值有 **1393 个通过**，0 例入台账。
SHACL 在随库固化的 W3C SHACL 1.0 测试套件上 **129/129 通过**，0 例入账，
在随库固化的 W3C SHACL 1.2 测试套件上 **800/800 通过**。
SHACL EARL 涵盖 591 个经批准的测试；独立的 SPARQL Rules 根目录在其独立固定的工作草案
之下贡献 209 个测试。

以下度量涵盖所列出的语料。它们并不断言每一种可能的输入或每一种语言接口都已被
检验：

| 引擎 | 套件 | 结果 |
| --- | --- | --- |
| ShEx 2.1 验证 | shexTest v2.1.0（`vectors/shexTest/`） | **1,105 / 1,105** 尝试，0 xfail |
| ShEx 模式 / 负例语法 / 结构 | shexTest v2.1.0 | **425/425 · 99/99 · 14/14** |
| SHACL | W3C data-shapes（`vectors/shacl/`） | **129 / 129** 通过 · 0 例入账 |
| SHACL 1.2 | W3C `shacl12-test-suite`（`vectors/shacl12/`） | **800 / 800** 通过 · 0 例入账的预期失败，0 例与规范相矛盾；1 个未列入清单的随库固化文件单独评分 |
| SHACL（第一方冻结语料） | `crates/shapes/corpus/` | **73 / 73** |
| SHACL Rules | DASH + 第一方（`vectors/shacl/af/rules/`） | **20 / 20** |
| 语法编解码器 | W3C rdf-tests 往返 | **264 / 264** |
| JSON-LD 1.1 上下文透镜 | W3C JSON-LD 1.1 REC toRDF + 压缩（`crates/rdf/tests/fixtures/jsonld-w3c-rec/`） | **73 / 73** 适用的 toRDF · **13 / 13** 精确压缩 |
| SPARQL 1.0/1.1/1.2 | 完整的 W3C `sparql10`（data-r2）+ `sparql11` + `sparql12` + 第一方，经由 `purrdf-sparql-conformance` | **1393** 通过 · 0 例入台账 |
| SPARQL CDT（SEP-0009） | 随库固化的 `awslabs/SPARQL-CDTs`（`vectors/sparql-cdt/`） | **658 / 658**，0 例入账——词法空间分歧见 [`docs/CONFORMANCE.md`](./docs/CONFORMANCE.md) |
| SPARQL 执行 governor | 第一方冻结语料（`vectors/sparql-governors/`） | **57 / 57**，0 例入账 |
| 蕴涵（SPARQL 蕴涵机制） | W3C `sparql11` `entailment/` 组 | **70 / 70**，0 例入账 |
| 蕴涵（OWL 2 DL 相容性） | 随库固化的 W3C OWL 2 套件 | **262 / 262** 一致，0 例入台账，0 例未入台账 |
| 蕴涵（OWL 2 RL，W3C 蕴涵测试） | 随库固化的 W3C OWL 2 蕴涵套件 | **50 / 50** 一致，0 例入账，0 例未入台账——负例通道 **23 / 23**（未发现不可靠之处），正例通道 **27 / 27** |
| RDFC-1.0 | W3C 规范化夹具 | **65** 个向量（64 个求值 + 1 个负例），绿 |
| RDF 1.2 规范化 profile（`purrdf-rdfc12` 第 3 版） | 第一方向量（`vectors/rdf12-canon/`） | **12 / 12** |
| GTS | 冻结的跨语言向量（`vectors/`） | **39 / 39** 逐字节折叠为其已提交的期望值，0 例入账。修订版 `cd123c2` 的上游语料含 52 个向量，引擎与其中 51 个一致；`13c-self-describing-triple-term` 是唯一长期存在的分歧，68 个 `sigalg-*` 向量不在声明之内 |

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
- [迁移到 5.0](./docs/MIGRATION-5.0.md)、[迁移到 4.0](./docs/MIGRATION-4.0.md)、[迁移到 3.0](./docs/MIGRATION-3.0.md)与[变更日志](./CHANGELOG.md)。
- [GTS](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md)、[PURREMB](./docs/PURREMB.md)、
  [RDF 1.2 规范化](./docs/RDF12-CANON-PROFILE.md)、
  [空节点的规范标号](./docs/design/purrdf-canonical-labelling.md)、
  [存储契约](./crates/rdf-core/STORAGE.md)与
  [发布流程](./docs/RELEASE.md)。

```sh
make metadata      # regenerate and verify projections and license bundles
make check         # formatting, clippy, build, tests and hygiene
make bench         # report-only microbenchmarks
make scale-corpus  # deterministic corpus generation
make lubm          # comparison lane; native Rust generator, pinned external ontology/queries
make watdiv        # comparison lane; frozen network dataset
```

使用者需要 stable Rust **1.98** 或更新版本。贡献者使用 `rust-toolchain.toml` 中声明的
nightly 分析工具链；源码不使用任何 nightly 独有特性。CI 单独强制执行 stable MSRV，并为
WASM 构建每一个已发布的 crate。发布工件使用 stable Rust 构建。

Cargo 套件、Python 分发包与 npm 包共享同一个协调一致的版本，并遵循语义化版本。C ABI
单独编号，其版本为 **0.20.0**，带有签名检查，并可在运行时经由 `purrdf_abi_version` 读取。
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
