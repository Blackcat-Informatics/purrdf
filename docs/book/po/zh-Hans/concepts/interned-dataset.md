<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->
<!--
zh-Hans 译稿（第一阶段）。与 docs/book/src/concepts/interned-dataset.md 逐段对应，
待 mdbook-i18n-helpers 的 .po 工作流落地后倒入 msgstr。代码块与英文原文逐字节相同。
-->

# 驻留数据集 IR

PurRDF 中的一切都在同一个中间表示（IR）上求值：一个不可变的、按值驻留（interned）的
RDF 1.2 数据集，由被隔离保护的
[`purrdf-core`](https://docs.rs/purrdf-core) 内核所拥有。

## 词项只驻留一次

每个词项——IRI、空节点、字面量、三元组项——都在一个字符串存储区（string arena）中
**只存一次**，并由一个可复制的 `TermId`（经 niche 优化的 `NonZeroU32`）寻址。四元组
是四个 `TermId` 构成的行。于是词项相等只是一次整数比较，四元组保持固定的小尺寸，而一个
在一百万条四元组中出现的词项，其字节恰好只付一次代价。

热点映射使用固定密钥的 `ahash`——确定性的哈希是
[字节级确定性纪律](codecs.md) 的一部分，而不只是速度上的选择。

## 构建器 → 冻结

IR 有严格的两阶段生命周期：

```rust,ignore
use purrdf_core::{RdfDatasetBuilder, RdfLiteral};

// Intern terms once; quads are rows of copyable TermIds.
let mut b = RdfDatasetBuilder::new();
let cat = b.intern_iri("https://example.org/cat");
let says = b.intern_iri("https://example.org/says");
let meow = b.intern_literal(RdfLiteral::simple("meow"));
b.push_quad(cat, says, meow, None);

// Freeze into the immutable, indexed dataset the engines evaluate over.
let ds = b.freeze().expect("well-formed dataset");
assert_eq!(ds.quad_count(), 1);
```

`RdfDatasetBuilder` 是可变的摄入阶段：驻留词项、推入四元组、附加具体化节点
（reifier）与注解。`freeze()` 校验结构并产出一个不可变的 `RdfDataset`：四元组行存放在
`Box<[QuadRow]>` 表中，带有惰性的序数置换索引（每条四元组每个轴大约 4 字节）。冻结后
的数据集就是 SPARQL、SHACL、ShEx 与蕴涵引擎共同读取的对象，读取途径是无分配的
`DatasetView` trait。

冻结也正是让并发变简单的原因：冻结后的数据集不可变，因此可以在多个线程之间共享并读取
（C ABI 暴露的正是这样一个 `Send + Sync` 句柄）。

## 写时复制的变更

「不可变」并不意味着「静态」。变更通过冻结基底之上的写时复制（copy-on-write）增量进行：
编辑累积在一个轻量的覆盖层中，结果再冻结成一个新数据集，既不复制未触及的基底行，也不
重新驻留共享的词项。SPARQL UPDATE 与 C ABI 的可变 `PurrdfGraph` 句柄都走这条路径。

## 内核中还有什么

除 IR 本身之外，`purrdf-core` 还拥有：

- **`DatasetView`**——每个引擎据以求值的静态读取 trait。
- **结构化诊断**——带源位置的、有类型的 `RdfDiagnostic`（刻意不含 SARIF；SARIF 的
  边界在 [`purrdf-validate`](https://docs.rs/purrdf-validate)）。
- **规范化**（RDF 1.1 子集上的 RDFC-1.0；具体化节点与注解之上的 `purrdf-rdfc12`
  profile）、数据集 diff 与同构判定——参见
  [规范化与 Diff](canonicalization.md)。
- **存储与引擎扩展点（seam）**——狭窄的解析器入口、序列化器出口以及 `SparqlEngine` trait，由
  兄弟 crate 中的适配器实现。
- **溯源与损失台账**——一个通用的溯源附件（sidecar）与机器可读的 RDF↔GTS 损失矩阵，
  外加原生的 FnO 与 SSSOM 编解码器
  （参见 [切片、映射与溯源](../slices.md)）。

文本编解码器*不在*内核中——解析与序列化位于上一层的
[`purrdf-rdf`](https://docs.rs/purrdf-rdf)。这一划分让内核保持小巧，其不变量可在
crate 边界上强制执行：没有 oxigraph，没有 PyO3（一道卫生门禁断言依赖树），
`wasm32` 干净，且 IR 层不含文件 IO。

## 为何如此设计

布局由测量而非断言决定：criterion 基准
`crates/rdf-core/benches/ir_layout.rs` 在分配次数、内存高水位与端到端延迟上比较
结构数组、数组结构与谓词邻接三种布局——最终采用的布局就是胜出的那个。参见
[性能](../project/performance.md)。
