<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->


[English](README.md) | 简体中文
# purrdf-cdt

为 PurRDF 的 RDF 1.2 查询栈提供原生 SPARQL **复合数据类型**（SEP-0009：`cdt:List` 和 `cdt:Map`）。

`purrdf-cdt` 是一个**封闭的叶子 crate**：运行时仅依赖 `purrdf-events`（其中的 `TextDirection` 是唯一的 RDF 1.2 基础方向类型，`purrdf_cdt::TextDirection` 会重新导出它）、`purrdf-iri`（绝对 IRI 验证）、`purrdf-xsd`（XSD 值空间）、`purrdf-lex`（项语法以及字面量和 IRI 的规范转义器）和 `purrdf-hash`（`UCHAR` 转义所用的大写十六进制数字）。它与 `purrdf-core` 在任何方向上都没有依赖关系，因此内核以后可以依赖它而不形成循环。它使用 `#![no_std]` 加 `alloc`，与工作区中的其他发布 crate 一样可构建到 `wasm32-unknown-unknown`。

提供以下能力：

* `parse_list` / `parse_map` / `parse_cdt`：SEP-0009 词法语法；扫描过程是**迭代式**的，受元素数量（`MAX_ELEMENTS`，计数在全部嵌套层级间共享；嵌套容器及其中的成员分别计数）和总字节数（`MAX_LEXICAL_BYTES`）两项已公布资源上限约束。没有递归下降，因此恶意词法形式会产生带字节偏移量的类型化错误，而不是无法捕获的栈溢出终止。
* `CdtValue::canonical_lexical`：PurRDF 为自身计算的值选用的字节确定性规范拼写（SEP-0009 未定义这种拼写）。
* `list_equal` / `map_equal` / `list_less_than` / `map_less_than`：规范定义的偏比较，错误会向调用方传播。
* `total_value_cmp`：可证明满足传递性的**句法**全序，用于 `ORDER BY`、映射键顺序和呈现顺序。
* `parse_literal`：覆盖 XSD 与 CDT 值空间的封闭 `LiteralValue` 枚举，保留「成功解析／不属于此值空间／字面量词法形式不符合其数据类型」三种状态。

PurRDF 对 SEP-0009 词法空间有两项明示的扩展（不生成新的 IRI，数据类型仍为 `cdt:List` / `cdt:Map`）：作为元素的 RDF 1.2 **三元组项** `<<( s p o )>>`，以及**带方向**的语言标签字面量 `"lex"@lang--ltr` / `"lex"@lang--rtl`。仅当实际项需要它们时才会输出这些形式；SEP-0009 无法表达相应的值。

## 许可证

可自行选择以下任一许可证：

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)
