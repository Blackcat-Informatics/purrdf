<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->
<!--
zh-Hans 译稿（第一阶段）。与 docs/book/src/concepts/codecs.md 逐段对应，
待 mdbook-i18n-helpers 的 .po 工作流落地后倒入 msgstr。代码块与英文原文逐字节相同。
-->

# 编解码器与确定性

PurRDF 为九种格式提供**第一方**的解析器与序列化器——没有任何包装的第三方编解码器：

| 格式 | 媒体类型 | 具 star 能力 |
| --- | --- | --- |
| Turtle | `text/turtle` | 是 |
| TriG | `application/trig` | 是 |
| N-Triples | `application/n-triples` | 是 |
| N-Quads | `application/n-quads` | 是 |
| RDF/XML | `application/rdf+xml` | 否 |
| TriX | `application/trix` | 否 |
| HexTuples | `application/x-hextuples` | 否 |
| JSON-LD (star) | `application/ld+json` | 是 |
| YAML-LD | `application/ld+yaml` | 是 |

它们位于内核之上一层的 [`purrdf-rdf`](https://docs.rs/purrdf-rdf) 中，并可经由门面
crate（umbrella crate）抵达：

```rust,ignore
use purrdf::{parse_dataset, serialize_dataset, SerializeGraph};

let turtle = br#"
    @prefix ex: <https://example.org/> .
    ex:cat ex:says "meow" .
"#;

// Parse into the frozen, value-interned RDF 1.2 dataset IR.
let ds = parse_dataset(turtle, "text/turtle", None).expect("valid Turtle");
assert_eq!(ds.quad_count(), 1);

// Serialize back out through any native codec — byte-deterministic output.
let nq = serialize_dataset(&ds, "application/n-quads", SerializeGraph::Dataset)
    .expect("serializes");
```

## Open Knowledge Format 包

原生的 OKF 编解码器把由调用方指定 profile 的 RDF 1.2 数据集映射为面向智能体的、带
YAML frontmatter 的 Markdown 文件，并经由 RDF 事件接缝把它们提升回来。OKF 是一个
内存中的包（bundle）API，而不是又一种媒体类型：文件如何存放由调用方决定，因此同一份
代码保持确定性且 wasm 干净。

`OkfConfig::new` 要求给出词汇表命名空间、文档基础 IRI 以及可识别的 frontmatter 键。
没有内置的本体或命名空间。用 `lift_okf_bundle` 驱动一个 `RdfEventSink`，或用
`write_okf_bundle`（由 `OkfWriter`——一个 `RdfDatasetVisitor`——支撑）投影一个冻结的
数据集。两个方向都总是返回一份损失台账。无损的 profile 得到空台账；写出时，命名图、
非 profile/OWL 行，以及无关的具体化节点或注解行都会被逐一明确指出。

## 字节级确定性

每个序列化器都是**字节级确定的**：同一数据集总是产生相同的字节，在每个平台、每种
语言绑定中皆然。这是一条硬性的工作区不变量，而非尽力而为——任何输出路径都不允许
依赖迭代顺序、时间或随机数（哈希器采用固定密钥的 `ahash` 正是为此），并且黄金文件
测试把发出的字节固定下来。

确定性是让工具包其余部分得以组合的基础：[GTS](../gts.md) 与
[切片目录](../slices.md) 中的内容寻址、评审时可 diff 的序列化结果，以及可逐字节比较的
跨语言一致性向量。

## 诊断，而非部分解析

畸形输入会得到一个带类型的 `RdfDiagnostic`，在编解码器能够给出的情况下附带源位置——
绝不会是静默的部分解析。解析可选择记录一张源位置区间表以获得更丰富的诊断。诊断在
内核中保持结构化（不含 SARIF）；面向编辑器与 CI 时，用
[`purrdf-validate`](https://docs.rs/purrdf-validate) 把它们渲染为字节级确定的
SARIF 2.1.0（参见 [SHACL](../validation/shacl.md#sarif-output)）。

## 有损投影必须出声

RDF 1.2 的陈述级数据（三元组项、具体化节点绑定、注解）在每一次具 star 能力的往返中
都得以保留。序列化到不具 star 能力的投影时，这一层会被*大声地*丢弃：实际丢弃的数量
交给机器可读的损失台账
（[`generated/rdf-loss-matrix.json`](https://github.com/Blackcat-Informatics/purrdf/blob/main/generated/rdf-loss-matrix.json)），
而不是凭空消失。同一纪律也适用于 SPARQL 结果边界（[结果格式](../sparql/results.md)）
与 RDF↔GTS 边界。

## 简洁打包编解码器

在上述文本编解码器之外，`purrdf-core` 还附带一个用途不同的**二进制**编解码器：面向
大规模参考包的整数据集只读、直接查询压缩形态的编码，而不是带媒体类型的交换格式。
`PackBuilder::build_bytes(&dataset)` 把一个自包含、字节级确定的 pack——一个值字典、
按图分区的简洁位图三元组，以及 RDF 1.2 侧表（具体化节点绑定、陈述注解）——写入一个
`Vec<u8>`。`PackView::from_bytes(&[u8])` 在借来的切片上零拷贝地打开它，并直接对
打包后的字节回答模式查询，无需先解压或物化。

当一个数据集已不再变化、需要分发、归档或在每次加载都重新解析文本已嫌太慢的规模上
提供服务时，就该用 pack：RDF 1.2（命名图、引用三元组、具体化节点、注解）得到完整
支持，并且 `verify_pack` 会从 pack 自身解码出的内容独立地重新计算数据集的 RDFC-1.0
摘要——这是一个**经认证的只读投影**，而不只是一个压缩文件。库本身从不对 pack 做内存
映射（每个已发布的 crate 都保持 `wasm32-unknown-unknown` 干净）；想要一个持久的、
大于堆的层级的原生消费者自行 `mmap` 文件，再把得到的借用切片交给
`PackView::from_bytes`。完整契约见
[后端契约](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/design/purrdf-backend-contract.md)
的「Pack backend」一节。

## 确定性的嵌入伴随件

`.purremb` 是嵌入（embedding）投影在某一个精确的 `.purrpck` 之上的、面向 mmap 的
伴随件（companion）。它不修改 pack，也不改变 RDF 的规范同一性。其已排序的节目录把
有限的稠密 `f32` 或 `f64` 矩阵绑定到源 pack 的精确 SHA-256、一个独立验证的 RDFC
摘要、完整的模型与处理契约、稳定的目标集，以及逐节和整件的完整性证据。
`EmbeddingBuilder` 接受无序的行；`EmbeddingStreamWriter` 接受规范顺序的行并以有界的
矩阵工作内存运行；二者产出相同的规范字节。

两类主体是一等公民。大型文本集合使用「语料—文档—分块」层级：UTF-8 文本留在外部，而
目标记录保留内容摘要、逻辑身份、字节与 Unicode 标量坐标、分块契约，以及按族限定的
token 区间。RDF 数据使用一个 RDF 1.2 模型，覆盖数据集、默认图与命名图、陈述、具体化
节点绑定、注解、方向字面量、空节点与递归的三元组项。源本地的 pack 序数只是经过验证的
查找提示，从不作为身份。

Matryoshka（套娃式）族只存储最宽的那个稠密矩阵。每个声明的前导前缀都是一个独立的
`VectorSpaceId` 与 `ProjectionId`，因此粗粒度的前缀不可能被静默地与完整空间相比较或
相替代。原始前缀行是零拷贝的跨步视图；确定性的 L2 前缀按需计算。近似索引仍是不透明、
可重建的派生工件，绑定到恰好一个精确的前缀投影。它们从不取代权威矩阵。

构造遵循同一条证据路径。首先，通过构建或独立验证精确的源 pack 取得一个
`CertifiedPurrpckSource`；任意的摘要声称无法构造出这个类型。对于语料，派生
`CorpusTarget`、`DocumentTarget::from_content` 与 `TextChunkTarget::from_document`
记录，添加必需的层级关系，并为每一个进入族矩阵的文档或分块添加一个 `TokenSpan`。对于
RDF，从那个已验证的 RDF 1.2 数据集派生数据集、图、陈述、具体化节点、注解与词项目标。
RDF-star 三元组项使用 `RdfTermTarget::Triple`；它们不进入单独的身份体系。

一个 `EmbeddingFamilyContract` 定义完整的生成流水线。Matryoshka 契约列出其允许的前导
维度，而其 `MatrixInput` 只在最宽维度上携带行，并为每个声明的空间携带一个
`ProjectionSpec`。消费者通过 `effective_matrix` 解析出精确的
`(TargetSetId, VectorSpaceId)`，并且在比较来自独立输入的行之前必须调用
`require_compatible_vector_spaces`。

大型集合在工件边界处分片：每个 `.purremb` 各自命名其精确的源 pack 与本地目标集，而
相等的族契约保留相同的 `FamilyId` 与 `VectorSpaceId`。语料清单与
`ExternalBinding::from_bytes` 绑定外部文本或其他精确工件；
`ExternalBinding::from_purrpck` 则加入独立认证的 RDF 证据。绑定携带调用方提供的角色
与媒体类型。PurRDF 不为它们发明策略或本体词汇表。

`EmbeddingView::from_bytes` 借用任何稳定的字节切片，无论它由堆拥有、由调用方内存
映射，还是位于 WebAssembly 线性内存中。结构性打开、完整工件验证、精确源验证与经认证
的源验证是几种明确的证据状态，而不是访问门禁。对文件做 mmap 的调用方必须在视图或
常驻的验证证书存在期间保持底层字节不可变。

嵌入与 ANN 结构是敏感的派生内容：模型反演、成员推断、相似度探测、摘要字典攻击以及
索引结构都可能泄露源数据的性质。容器哈希能检测损坏与过期的附着；它们不认证作者、
不加密内容，也不授予访问权限。参见逐字节精确的
[PURREMB v1 规范](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/PURREMB.md)。

## 列式 Parquet 编解码器

`purrdf::columnar` 暴露双向的 SQL/DataFrame 交换路径。它把任意 `DatasetView` 加上一个
内容寻址的 blob 存储映射为五个标准 Parquet 文件（`terms`、`quads`、`reifiers`、
`annotations` 与 `blobs`），并在不依赖 Arrow 或通用 Parquet 运行时的情况下把这一
精确 profile 读回。该映射保留 RDF 1.2 三元组项、具体化节点、注解、图作用域、方向
字面量、空节点作用域，以及显式为空的命名图。

这些文件是字节级确定的，可被 DuckDB 等引擎读取。每个字段以及刻意收窄的 Parquet
profile 见
[规范性的列式模式](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/COLUMNAR.md)。

## 一致性

编解码器由 W3C `rdf-tests` 语法语料把关，该语料随库固化并冻结在仓库内——撰写本文时，
N-Quads、N-Triples、RDF/XML、TriG 与 Turtle 共 250/250 个往返用例。实时记分板是
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md)。

## 相关

- [规范化与 Diff](canonicalization.md)——当需要的是*规范*序列化而不只是确定性
  序列化时。
- [驻留数据集 IR](interned-dataset.md)——文本编解码器解析出的目标，以及 pack
  编解码器与 `RdfDataset` 一同实现的 `DatasetView` 读取接缝。
