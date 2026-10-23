<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>


[English](README.md) | 简体中文
# `purrdf-text`——针对 RDF 字面量的确定性全文检索

[![crates.io](https://img.shields.io/crates/v/purrdf-text.svg)](https://crates.io/crates/purrdf-text)
[![docs.rs](https://docs.rs/purrdf-text/badge.svg)](https://docs.rs/purrdf-text)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-text` 是 PurRDF 工具包的内存全文索引。它从冻结数据集中读取 RDF 1.2 字面量，按明确解析的 Unicode／词典配置分析文本，并以精确 BM25F 分数回答排序查询。[分析与辅助检索](../../docs/design/text-analysis.md)说明限定作用域的变音符号、英语词干提取、五套完整的基线词典、emoji、HTML 字符引用、来源证据、位置子串以及独立的汉字检索结果生成器。

## 由调用方提供的 IRI

PurRDF 不生成词汇表。此 crate 通过求值器的属性函数扩展接口向 SPARQL 暴露排序检索；查询用于调用该接口的谓词 IRI **由调用方配置**，既没有默认命名空间，也没有虚构的回退值。配置中没有指定 IRI 时，返回类型化错误，而不是猜测。待索引字面量所属的谓词也由调用方指定。

## 精确运算，各目标架构答案一致

排序使用小数点后十二位的十进制定点整数运算。评分和排序不使用浮点运算：crate 根部声明 `#![deny(clippy::float_arithmetic)]`。

这是正确性保证，而非风格选择。BM25 需要自然对数，不同的 libm `ln` 实现可以在最低有效位相差一个单位。只要出现一次这样的差异，两份分数极接近的文档就可能交换顺序，使原生构建与同一引擎的 WebAssembly 构建对相同数据、相同查询返回不同的行序。此处的对数由定长整数级数计算，迭代次数固定，绝不以收敛测试决定；所以每种目标架构上的结果都仅由输入决定。

原生 Rust 测试以独立预期固定**排序**行为：行序以及每个分数的十进制词法形式。测试还断言两个独立构建的索引通过属性函数扩展接口查询后，**序列化**答案的字节完全一致。`make wasm` 另外构建发布 crate；实际目标架构路径与宿主接口上的 WASM 执行见 [WASM 测试归属](../../docs/WASM_TESTING.md)。

## 带版本的 BM25F，在分区内排序

`TextIndexConfig::with_surface_index(false)` 建立仅含 BM25F 的索引，省略子串和语音倒排表。
其他情况下，构建会包含 surface index。`TextIndex::surface_index()` 返回
`Option<&SurfaceIndex>`，让调用方显式处理其缺失。此选择不改变精确排序分数、
文档顺序、来源身份或独立的汉字检索。索引代次使用 `purrdf-text/index/v4` 原像，
其中的 surface-presence 字节区分被省略的索引和空索引。两种模式的索引指纹都会
有意改变；来源指纹和序列化排序结果仍保留原有编码。

`TextIndexConfig::<GraphFree>::for_triples` 选择按主体和语言组织的文档，以及仅按语言划分的分区。
其 `purrdf-text/triples/index/v1` 身份包含相同的 surface-presence 字段，并省略所有图坐标和选择器。
独立的 `purrdf-text/triples/source/v1` 身份覆盖来源三元组，不因省略 surface index 而改变。

排序统一使用含字段的 BM25F 路径，单字段情况也一样。不可变的 `RankingProfile` 将调用方指定的命名字段、每个字段的权重与长度归一化、完整的谓词路由、精确运算、查询聚合与字段总体统计方式绑定到同一指纹。`PreparedQuery` 则根据语料统计单独携带该查询的认证分数上界。`k1 = 1.2` 固定不变。`RankingProfile::single_field()` 显式把每个选定谓词路由到同一字段，权重为一，`b = 0.75`。

`TextIndex::from_dataset_with_ranking` 按显式配置建立索引；`with_ranking_profile` 利用保留的谓词级词元事实对现有索引重新加权或重映射，无须再次切词。分析器身份与排序身份分开。纯函数式 `PreparedCorpus` 和 `PreparedQuery` API 将同一评分实现提供给其他存储，并将缓存的 IDF 绑定到经验证的语料统计与不可变配置。

`index.with_delta(&next_dataset, &added, &removed)` 消耗原索引，并依据有效的新增与移除语句返回更新后的索引。
输入使用与索引相同的载体配置。断言层与注解层的字面量行都计入，同一个字面量同时出现在两个层中的情况也计入。
逆转所给增量必须恢复索引的来源指纹；缺少新增语句或增量不完整都会产生类型化拒绝。
下一数据集也会为非字面量变更刷新谓词覆盖信息。

只重新分析发生变更的文档，按其完整键区分：`GraphLanguage` 为 `(graph, subject, language)`，
`GraphFree` 为 `(subject, language)`。一个变更文档的全部下一状态字面量行一起分析，
因此留在该文档中的字面量也会重新分析。其他文档保留其词元与来源投影缓冲区，复用其 gram 位置和发音代码。
规范 ID、统计、指纹与答案都与重新完整构建一致。更新后 ID 可能改变，必须与其索引代次一起保存。
来源收集、排序、哈希与有序表合并仍会遍历保留的数据，总工作量不是次线性的。
若要保留旧快照，应先克隆再进行取得所有权的更新；显式克隆会复制缓冲区，不会隐式共享它们。

对于稀疏字段，选择 `RankingProfile::with_field_populations()`，并向 `PreparedCorpus::with_field_populations` 提供携带各字段的文档数量。长度于是相对于携带该字段的文档的平均长度归一化；IDF 仍按整个语料计算。选择此配置时，`TextIndex` 会按分区推导这些数量。该模式会改变排序身份；既有稠密配置的指纹与分数保持不变。

排序法则不设置全局分数上限，也不为宿主的并列排序键预留位。每个 `PreparedQuery` 都根据实际语料与 IDF 计算该查询的精确分数上界和所需位宽；分数携带这一证书，宿主自行选择键的表示。[排序契约](RANKING.md)给出上界证明、运算顺序、独立算术参考以及词汇表搜索的归属决策。

语料统计在 `GraphLanguage` 中按 `(graph, language)` 分区计算，在 `GraphFree` 中仅按语言分区计算，因此分数只相对于一个语料有意义。`?rank` 是文档在**自己分区内**从 1 开始的位置，绝非全局名次；跨分区比较名次等于对不同语料计算的分数排序。行按 `(partition key ASC, rank ASC)` 输出。需要一份统一排序列表的查询，在 `GraphLanguage` 中应绑定 `?lang` 并限制图，在 `GraphFree` 中只需绑定 `?lang`。单分区索引同样给出一份排序列表。

Rust 字段因此命名为 `Scored::partition_rank`：名次的*数值*并不记录产生它的语料。一个含三种语言的索引会给出三个名次为 1 的行，对其执行 `LIMIT 10` 取得的是按分区优先、再按分区内名次排列的前十行，而不是全局最优的十份文档。

在同一分区内，顺序是 `(score DESC, document id ASC)`。文档 ID 仅在按完整文档键排序后分配：`GraphLanguage` 为 `(graph, subject, language)`，`GraphFree` 为 `(subject, language)`。因此 ID 升序就是规范顺序升序，两个独立构建的索引也有可复现的并列顺序。ID 互不相同，所以该顺序是严格全序。

分数以固定宽度的 `xsd:decimal` 送达使用者。因此两行可能报告相同的 `?score`，却带有不同的 `?rank`：它们的精确分数可能相同，也可能只是舍入后相同。在同一分区内，需要重现索引顺序时应写 `ORDER BY ?rank`；`ORDER BY DESC(?score)` 可能产生不同顺序。

## PurRDF 的组成部分

此 crate 属于 [PurRDF](https://github.com/Blackcat-Informatics/purrdf) 工作区。PurRDF 是 RDF 1.2 工具包，包含原生编解码器、SPARQL、SHACL、ShEx、蕴涵以及 GTS 图传输，并提供 Python、WebAssembly 与 C 接口（GTS 容器本身可通过 Python 和 C 使用，但不在 wasm 包中）。大多数应用应依赖总入口 [`purrdf`](https://crates.io/crates/purrdf)；仅需独立使用索引及其算术实现时，才直接依赖 `purrdf-text`。

整个工作区刻意不提供 Cargo feature 标志。MSRV 遵循工作区的 `rust-version`（目前为 1.98；该最低版本承诺面向稳定版 Rust 的使用者）。

## 许可证

可自行选择以下任一许可证：

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)
