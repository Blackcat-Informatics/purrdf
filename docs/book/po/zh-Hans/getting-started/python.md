<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->
<!--
zh-Hans 译稿（第一阶段）。与 docs/book/src/getting-started/python.md 逐段对应，
待 mdbook-i18n-helpers 的 .po 工作流落地后倒入 msgstr。代码块与英文原文逐字节相同。
-->

# 入门：Python

Python 包封装的是同一个原生 Rust 引擎，而非重新实现，因此解析、序列化、SPARQL
与验证的行为与 Rust、JavaScript 和 C 表面完全一致。

```sh
pip install purrdf
```

**中国大陆镜像。** 从中国大陆访问 PyPI 时常有延迟或间歇性不可达。开发者通常改用
清华大学 TUNA、中国科学技术大学 USTC 或阿里云的 PyPI 镜像，例如
`pip install -i https://pypi.tuna.tsinghua.edu.cn/simple purrdf`。wheel 会自动同步到
各镜像，PurRDF 这一侧无需任何操作；镜像的配置属于读者自己的环境，本文不代为脚本化。

## 解析

```python
import purrdf

quads = purrdf.parse(
    '<https://example.org/alice> <http://xmlns.com/foaf/0.1/name> "Alice" .',
    purrdf.RdfFormat.TURTLE,
)
```

## 验证：SHACL 与 ShEx

原生验证引擎以顶层子模块的形式暴露，与 Rust 的 `purrdf` 门面 crate（umbrella
crate）保持同构——绝不直接经由内部的 `purrdf_native` 扩展模块：

```python
from purrdf import shacl, shex

report = shacl.validate(shapes_ttl=my_shapes, data_nt=my_data)
print(report["conforms"])

results = shex.validate(my_schema_shexc, my_data_ttl,
                        [("https://example.org/alice", "https://example.org/PersonShape")])
print(results[0]["conformant"])
```

SHACL 结果字典保留稳定的键 `focus`、`path`、`value`、`severity`、`component`、
`source_shape` 与 `message`。两个引擎各自覆盖的范围见 [SHACL](../validation/shacl.md)
与 [ShEx](../validation/shex.md)。

## 蕴涵

`purrdf.entail` 在某一 SPARQL 蕴涵机制（entailment regime）下对数据集求闭包。它不是
`purrdf.shapes.entail`——后者应用的是*形状*图所声明的 SHACL-AF `sh:rule`；这里不接受
任何形状，只使用该蕴涵机制自身规范中的规则表。

```python
import purrdf
from purrdf import entail

dataset = purrdf.RdfDataset(my_turtle, purrdf.RdfFormat.TURTLE)
closure, report = entail.materialize(dataset, "rdfs", "")
print(closure.to_nquads())
print(report)          # what fired, what did not, boundaries, budget, contract hash
```

推理报告是第二个返回值，且永远不可省略——这与 Rust、WebAssembly 和 C 表面所执行的
纪律一致。`entail.materialize_nt(text, regime)` 是其文本进、文本出的孪生接口，供手握
N-Triples/N-Quads 文档的调用方使用。

覆盖率是可测量的，而非口头断言：`entail.rules(regime)` 是规范用以定义该蕴涵机制的
规则表，`entail.implemented_rules(regime)` 则是其中实际会触发的子集。`"owl-direct"` 与
`"rif"` 在此返回 `[]`——二者都没有自己的规范规则表，一个通过 tableau 判定，另一个在
调用方自己的规则下蕴涵——而不是抛出错误。完整说明见 [蕴涵](../entailment.md)，逐条规则的
表格见 [规则清单](../entailment-rules.md)。

## rdflib 兼容

本包附带一个 rdflib 兼容层：

```python
from purrdf.compat.rdflib import Graph
```

若需要逐字不改的 `import rdflib`，可选装一个 extra：

```sh
pip install purrdf[rdflib]
```

这会拉入独立发行的 `purrdf-rdflib` 分发包，其顶层 `rdflib` 包重新导出兼容层的表面，
于是现有第三方代码中的 `import rdflib` / `from rdflib.namespace import RDF` 便透明地
运行在 `purrdf` 之上。**注意：**该影子包占用了 `rdflib` 这一导入名，绝不可与真正的
[`rdflib`](https://pypi.org/project/rdflib/) 同时安装——二者无法共存于同一环境。
它被刻意做成独立分发包（从不打进主 `purrdf` wheel），正是为了让需要真实 rdflib 的环境
直接不装它即可。

兼容层在 CI 中以 rdflib 7.6 自带的、随库固化（vendored）的测试套件加上第一方的差分
一致性套件把关——细节以及已知的、已入台账的分歧见 [rdflib 兼容](../interop/rdflib.md)。

## GTS 关系型导出

Python 包还为分析流水线附带了 GTS 的关系型导出：

```python
from purrdf import gts_to_sqlite, gts_to_duckdb, gts_to_parquet
```

它们把一个 [GTS 容器](../gts.md) 投影为 SQLite、DuckDB 或 Parquet 表。

## 图、表格与研究对象归档

`purrdf.project(data, format=..., profile=..., config=...)` 返回规范的 USTAR 字节与
结构化的损失记录。`purrdf.lift(archive, profile=..., config=...)` 则为十个双向 profile
重建 RDF。每个宿主都使用同一套严格配置与同一条确定性的 Rust 代码路径；各 profile 及
完整示例见 [图、表格与研究对象投影](../concepts/projections.md)。

## 下一步

- [rdflib 兼容](../interop/rdflib.md)——直接替换方案的深入说明。
- [验证](../validation/shacl.md)——在 Python 中使用 SHACL 与 ShEx。
- [蕴涵](../entailment.md)——各蕴涵机制、推理报告，以及各自触发的规则。
- [GTS 图传输](../gts.md)——导出功能所读取的容器格式。
- [图、表格与研究对象投影](../concepts/projections.md)——LPG、CSVW、OBO、SKOS 与
  五种研究对象载体。
