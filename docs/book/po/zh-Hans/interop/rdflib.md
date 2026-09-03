<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->
<!--
zh-Hans 译稿（第一阶段）。与 docs/book/src/interop/rdflib.md 逐段对应，
待 mdbook-i18n-helpers 的 .po 工作流落地后倒入 msgstr。代码块与英文原文逐字节相同。
-->

# rdflib 兼容

Python 的 [rdflib](https://rdflib.readthedocs.io/) 是这个生态中现有的 RDF 库，PurRDF
在两个层级上与之衔接：一个显式的兼容模块，以及一个可选装的直接替换影子包。

## 第一层：显式兼容层

主 `purrdf` wheel 附带一个由原生引擎支撑的 rdflib 兼容层：

```python
from purrdf.compat.rdflib import Graph

g = Graph()
g.parse(data="<https://example.org/a> <https://example.org/b> <https://example.org/c> .",
        format="turtle")
```

对于想在 PurRDF 引擎上使用 rdflib 形态 API 的新代码，这是推荐路径：导入名名副其实，
并且它能与真正安装的 `rdflib` 共存。

## 第二层：`purrdf[rdflib]` 影子分发包

若需要逐字不改的 `import rdflib`，安装可选装的 extra：

```sh
pip install purrdf[rdflib]
```

这会拉入独立发行的 `purrdf-rdflib` 分发包，其顶层 `rdflib` 包重新导出兼容层的 API，
于是现有第三方代码中的 `import rdflib` / `from rdflib.namespace import RDF` 便透明地
运行在 `purrdf` 之上。**注意：**该影子包占用了 `rdflib` 这一导入名，绝不可与真正的
[`rdflib`](https://pypi.org/project/rdflib/) 同时安装——二者无法共存于同一环境。
它被刻意做成独立分发包（从不打进主 `purrdf` wheel），正是为了让需要真实 rdflib 的环境
直接不装它即可。

## 兼容性如何得到保证

兼容层不是「尽力而为」——它作为单一一致性矩阵的一部分在 CI 中把关
（[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md)）：

- **rdflib 直接替换（LSP）门禁**用 rdflib 7.6 **自带的、随库固化（vendored）的测试套件**来跑
  `purrdf` 直接替换包。
- **对等性测试套件（parity suite）**用 `purrdf.compat` 对真实的 rdflib 7.6 运行第一方的差分测试。

二者都使用严格的预期失败台账：每一处已知分歧都带着逐测试的原因列出，一次意外失败会
打断构建，而一处被静默修复的分歧同样会打断构建，直到台账相应缩减为止。台账中的残余
分歧覆盖诸如集合运算下 Graph 子类的身份、`rdf:List`/Collection 的变更、
`Result.bindings` / `SELECT *` 子查询投影、图前缀转发，以及旧式 `ConjunctiveGraph`
语义这类角落——当前的精确清单请查阅台账本身。

## 性能

一个仅供报告的基准测试框架把原生支撑的 `purrdf.compat.rdflib` 直接替换包与真实
rdflib 在解析、序列化、SPARQL 与三元组模式迭代上计时对比（`make bench-python`）。
方法论以及一份有代表性的（依宿主而异的）结果表见
[`docs/BENCHMARKS.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/BENCHMARKS.md)
——数字因宿主而异，因此请在本地复现，而不要相信一个固定的倍数。设计理念见
[性能](../project/performance.md)。

## 相关

- [入门：Python](../getting-started/python.md)
- [一致性与测试](../project/conformance.md)——台账纪律如何运作。
