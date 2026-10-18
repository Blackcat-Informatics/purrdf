<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# PurRDF 列式模式

[English](COLUMNAR.md) | 简体中文

`purrdf-columnar` 用一组固定的五个 Parquet 文件表示一个 RDF 1.2 数据集和一个内容寻址的 blob 存储。`ParquetFiles` 值始终包含这五个文件；空表也有对应的有效零行文件。

所有表都是扁平的。Parquet 物理类型只有 `INT64` 和 `BYTE_ARRAY`；表示文本的字节数组带有标准 UTF8 注解。必需列每行恰有一个值。可选列的定义层级为 0 时表示 null，为 1 时表示有值。

## 表

### `terms.parquet`

| 列 | 类型 | 基数 | 含义 |
| --- | --- | --- | --- |
| `id` | INT64 | 必需 | 稠密的列式项 ID，从零开始。 |
| `kind` | INT64 | 必需 | `0` 表示 IRI，`1` 表示字面量，`2` 表示空节点，`3` 表示三元组项。 |
| `lex` | UTF8 | 可选 | IRI、字面量的词法形式或空节点标签；三元组项为 null。 |
| `datatype` | INT64 | 可选 | 字面量数据类型 IRI 的项 ID。 |
| `lang` | UTF8 | 可选 | 小写语言标签。 |
| `direction` | INT64 | 可选 | `0` 表示从左到右，`1` 表示从右到左。 |
| `scope` | INT64 | 可选 | 空节点作用域的序号。 |
| `triple_s` | INT64 | 可选 | 三元组项的主体 ID。 |
| `triple_p` | INT64 | 可选 | 三元组项的谓词 ID。 |
| `triple_o` | INT64 | 可选 | 三元组项的客体 ID。 |
| `named_graph` | INT64 | 必需 | 该项是已声明图的名称时为 `1`，否则为 `0`。 |

项字典包含从基础四元组、具体化节点绑定、注解和命名图声明可达的所有项及其递归依赖。行按项值的规范编码排序，而不按后端内部的 ID 排序。这样既保留了显式声明的空命名图，也使输出字节不受 `DatasetView` 实现的 ID 分配方式影响。

### `quads.parquet`

| 列 | 类型 | 基数 | 含义 |
| --- | --- | --- | --- |
| `s` | INT64 | 必需 | 主体项 ID。 |
| `p` | INT64 | 必需 | 谓词项 ID。 |
| `o` | INT64 | 必需 | 客体项 ID。 |
| `g` | INT64 | 可选 | 命名图的项 ID；null 表示默认图。 |

行不重复，并按 `(s, p, o, g)` 排序，默认图排在命名图之前。

### `reifiers.parquet`

| 列 | 类型 | 基数 | 含义 |
| --- | --- | --- | --- |
| `reifier` | INT64 | 必需 | 具体化节点资源的项 ID。 |
| `s` | INT64 | 必需 | 被具体化陈述的主体 ID。 |
| `p` | INT64 | 必需 | 被具体化陈述的谓词 ID。 |
| `o` | INT64 | 必需 | 被具体化陈述的客体 ID。 |
| `g` | INT64 | 可选 | 绑定所属的图；null 表示默认图。 |

行不重复，并按 `(reifier, s, p, o, g)` 排序。

### `annotations.parquet`

| 列 | 类型 | 基数 | 含义 |
| --- | --- | --- | --- |
| `reifier` | INT64 | 必需 | 被注解的具体化节点的项 ID。 |
| `predicate` | INT64 | 必需 | 注解谓词的项 ID。 |
| `value` | INT64 | 必需 | 注解客体的项 ID。 |
| `g` | INT64 | 可选 | 注解所属的图；null 表示默认图。 |

行不重复，并按 `(reifier, predicate, value, g)` 排序。

### `blobs.parquet`

| 列 | 类型 | 基数 | 含义 |
| --- | --- | --- | --- |
| `digest` | UTF8 | 必需 | 小写 SHA-256 摘要，共 64 个十六进制字符。 |
| `bytes` | BYTE_ARRAY | 必需 | 内容寻址的载荷字节。 |

行按原始摘要字节排序。读取器重新计算每个载荷的散列，摘要不匹配时拒绝读取；它不会修复损坏内容，也不会通过更改索引来接受这些内容。

## Parquet profile

每个非空表只含一个行组，每列只含一个 Data Page V2。值采用 PLAIN 编码。可选列的定义层级使用 Parquet RLE 混合编码中的游程长度形式；模式是扁平的，因此没有重复层级。一次写入为所有值体选择同一种编解码器：UNCOMPRESSED 或 ZSTD。Zstd 是运行时选项，由始终存在的纯 Rust 依赖 `structured-zstd` 提供，不由 Cargo feature 控制。

页头和文件元数据使用 Thrift Compact Protocol。文件携带稳定的键值元数据，标明列式模式版本 `1` 及对应的逻辑表。写出器不输出时钟、RNG、文件系统信息或环境中的生产者信息。

读取器针对这一确切的模式和编码 profile。结构偏离、不支持的功能、格式错误的引用、无效的 RDF 位置、解压大小不匹配或尾随数据都会导致拒绝读取。

## 解码安全预算

在分配解码列或进入 Zstd 之前，读取器交叉核对行组、列块和页的大小，并估算表的常驻列槽位及未压缩值字节所需的空间。这个固定 profile 的工作集每表不得超过 256 MiB。由元数据决定的向量最初最多为 65,536 行预留空间；公开读取器一次解码并释放一个物理表。超过上限的输入返回 `ColumnarError::LimitExceeded`；写出器采用同一预算，因此不会输出配套读取器无法接受的文件。
