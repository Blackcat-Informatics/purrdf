<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->
<!--
zh-Hans 译稿（第一阶段）。与 docs/book/src/concepts/rdf12.md 逐段对应，
待 mdbook-i18n-helpers 的 .po 工作流落地后倒入 msgstr。代码块与英文原文逐字节相同。
-->

# RDF 1.2 特性

PurRDF 以 RDF 1.2 为先：[RDF 1.2 Concepts](https://www.w3.org/TR/rdf12-concepts/)
在 RDF 1.1 之上新增的特性都是核心数据模型的一部分，贯穿 IR、编解码器、SPARQL、
验证、各语言绑定与 GTS 传输。

## 三元组项

RDF 1.2 允许三元组本身作为**宾语位置**上的一个词项——即 RDF-star 的「引用三元组」，
在 SPARQL 1.2 语法中写作 `<<( s p o )>>`。在 IR 中，三元组项（triple term）像任何
其他词项一样被驻留并获得一个 `TermId`，因此它能与其余一切（模式、结果、序列化）组合。

- 具备 star 能力（star-capable）的编解码器（Turtle、TriG、N-Triples、N-Quads、
  JSON-LD star）可往返三元组项；投影到不具 star 能力的格式时会发生什么，见
  [编解码器与确定性](codecs.md)。
- SPARQL 1.2 的引用三元组语法由 `purrdf-sparql-algebra` 解析并原生求值——W3C
  SPARQL 1.2 的三元组项支持在一致性测试框架中通过
  （[一致性与测试](../project/conformance.md)）。
- 在 JavaScript 中，`DataFactory.quotedTriple(...)` 产出同一种词项
  （[JavaScript 中的 RDF/JS](../interop/rdfjs.md)）。

## 具体化节点与注解

RDF 1.2 用**具体化节点**（reifier）取代了旧式具体化：它是为三元组的某一*出现*命名的
词项（`rdf:reifies`），使得无需引入额外断言即可为一条陈述附加元数据。在
PurRDF 中，具体化节点绑定与注解存放在数据集上专门的**侧表**（side table）中，而不是混入
四元组表。

具体化节点绑定与注解在每一种具 star 能力的编解码器往返中都得以保留；投影到不具 star
能力的格式时，它们会被*显式*丢弃并报告，实际丢弃的数量交给损失台账（参见
[切片、映射与溯源](../slices.md)）。对具体化陈述进行验证的 SHACL 支持——草案中的
`sh:reifierShape` / `sh:reificationRequired` 接口——见 [SHACL](../validation/shacl.md)。

## 基础方向字面量

RDF 1.2 新增了 `rdf:dirLangString`：一种带语言标签、同时携带基础方向（`ltr` 或
`rtl`）的字面量，用于正确处理双向文本。它们在 IR 和每一个绑定中都是一等公民：

```js
const rtl = f.directionalLiteral("مرحبا", "ar", "rtl");
```

方向在经由具 star 能力的编解码器的序列化往返中得以保留——
[JavaScript 快速入门](../getting-started/javascript.md) 演示了 N-Quads 的往返。

## RDF 1.2 是一个完整的目标，而不是草案借口

PurRDF 把 RDF 1.2 / SPARQL 1.2 规范当作完整的、可实现的目标。凡是某项特性有范围限定
的地方（例如，SHACL 1.2 的具体化节点形状支持是一项有范围限定的工作草案特性，而非完整的
SHACL 1.2 一致性），其范围都被明确陈述并由测试把关——绝不留作静默的部分实现。逐特性的
实时状态是
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md)
中的一致性矩阵。

## 每项特性出现在哪里

| 特性 | IR | 编解码器 | SPARQL | SHACL | RDF/JS | GTS |
| --- | --- | --- | --- | --- | --- | --- |
| 三元组项（宾语位置） | 驻留词项 | 具 star 能力的格式 | `<<( s p o )>>` | 经由路径/值 | `quotedTriple` | 按规范映射 |
| 具体化节点 / 注解 | 侧表 | 具 star 能力的格式 | 具体化节点支持 | `sh:reifierShape`（草案） | — | `rdf:reifies` 映射 |
| 基础方向字面量 | 字面量种类 | 可往返 | 可匹配/可产出 | 值节点 | `directionalLiteral` | 可承载 |

三元组项与 `rdf:reifies` 的 GTS 映射在
[GTS 规范](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/GTS-SPEC.md)
中形式化，该规范将其 RDF 1.2 基底固定在 2026 年 4 月 7 日的 W3C 候选推荐快照上。
