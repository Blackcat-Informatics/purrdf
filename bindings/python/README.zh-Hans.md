<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# PurRDF for Python（简体中文）

> 本文是 [`README.md`](./README.md) 的简体中文译本。PyPI 只渲染英文 README；本译本与
> 仓库和《PurRDF 之书》同行。代码块、标识符与数字与英文原文逐字相同。

<p>
  <a href="https://pypi.org/project/purrdf/"><img src="https://img.shields.io/pypi/v/purrdf.svg?label=PyPI" alt="PyPI"></a>
  <a href="https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSING.md"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0"></a>
  <a href="https://pypi.org/project/purrdf/"><img src="https://img.shields.io/pypi/pyversions/purrdf.svg" alt="Python versions"></a>
</p>

PurRDF 是一个从零实现、依赖极简的 [RDF 1.2](https://www.w3.org/TR/rdf12-concepts/)
引擎——解析器与序列化器、SPARQL、SHACL、ShEx、RDFC-1.0 规范化，以及 GTS 图传输
容器——以 Rust 编写，并原样带入 Python、JavaScript 与 C。`purrdf` 包是这同一个引擎
的 Python 表面：在每种语言中都是同样的、字节级一致的语义，包括大多数现有库尚未承载
的三元组项、具体化节点（reifier）与基础方向字面量。

## 安装

```sh
pip install purrdf
```

需要 Python 3.13+。wheel 内置原生扩展；不需要 Rust 工具链。

**中国大陆镜像。** 从中国大陆访问 PyPI 时常有延迟或间歇性不可达。开发者通常改用
清华大学 TUNA、中国科学技术大学 USTC 或阿里云的 PyPI 镜像，例如
`pip install -i https://pypi.tuna.tsinghua.edu.cn/simple purrdf`。wheel 会自动同步到
各镜像；镜像的配置属于读者自己的环境，本文不代为脚本化。

## 解析 RDF

```python
import purrdf

quads = purrdf.parse(
    '<https://example.org/alice> <http://xmlns.com/foaf/0.1/name> "Alice" .',
    purrdf.RdfFormat.TURTLE,
)
```

`purrdf.parse` 接受 Turtle、TriG、N-Triples、N-Quads、TriX 与 HexTuples
（`purrdf.RdfFormat`）；JSON-LD 与 RDF/XML 经由专门的
`purrdf.from_json_ld` / `purrdf.to_json_ld` 与 `purrdf.from_rdf_xml` /
`purrdf.to_rdf_xml` 转换器。所有编解码器均为第一方实现，输出字节级确定。

带配置的 JSON-LD 与 YAML-LD 使用同一份严格的、带版本号的选项文档。序列化多个数据集时，
可编译一个可复用的上下文：

```python
import json
import purrdf

options = json.dumps({
    "version": 1,
    "mode": "context",
    "prefixes": {"ex": "https://example.org/", "schema": "https://schema.org/"},
})
context = purrdf.CompiledJsonLdContext(options)
jsonld = purrdf.serialize_jsonld(
    nquads,
    format=purrdf.RdfFormat.N_QUADS,
    output_format="jsonld",
    context=context,
)
```

`expanded`、`context` 与确定性的数据集 IRI `derived` 三种模式都是显式的。PurRDF 从不
推断调用方的词汇表，也从不拉取远程上下文。

## 投影图、表格与研究对象载体

`purrdf.project` 与 `purrdf.lift` 是对其他每个表面所用的同一个 Rust 投影引擎的薄调用。
配置是必填的严格 JSON：PurRDF 不提供任何词汇表、身份 IRI 或资源上限的默认值。

```python
import json
import purrdf

config = json.dumps({
    "profile": "lpg-csv",
    "config": {
        "rdf_type": "https://example.org/type",
        "scope": {"mode": "all"},
        "limits": {
            "max_artifacts": 16,
            "max_artifact_bytes": 1_000_000,
            "max_total_bytes": 4_000_000,
            "max_archive_bytes": 5_000_000,
            "max_term_depth": 16,
        },
        "execution_limits": {
            "max_input_records": 1_000,
            "max_model_records": 1_000,
            "max_nodes": 1_000,
            "max_edges": 1_000,
        },
    },
})
package = purrdf.project(
    "@prefix ex: <https://example.org/> . ex:alice ex:knows ex:bob .",
    format=purrdf.RdfFormat.TURTLE,
    profile="lpg-csv",
    config=config,
)
lifted = purrdf.lift(package.archive, profile="lpg-csv", config=config)
assert lifted.dataset.quad_count() == 1
print([(loss.code, loss.location) for loss in package.losses])
```

投影 profile 有 `lpg-csv`、`neo4j-csv`、`open-cypher`、`graphml`、
`csvw-exact`、`csvw-terms`、`okf-terms`、`obo-graphs`、`skos`、`croissant-1.1`、
`ro-crate-1.3`、`datacite-4.6`、`dcat-3`、`dcat-rdf`、`void` 与
`frictionless-data-package-1`。精选的 CSVW/OKF terms、OBO Graphs、SKOS、原生
DCAT RDF 与 VoID 是刻意设计为只写的、入台账的视图。返回的归档是规范的、确定性的
USTAR 字节，每个结果都携带其总会计算出的结构化损失记录。研究对象的上下文、词汇表、
身份与 profile 全部是调用方必须提供的配置。

原生的 RDF 数据集描述使用同一个调用。完整的 JSON 指明输出语法，以及一个映射式/CONSTRUCT
式的 DCAT 源，或者 VoID 的源图、角色词汇表、数据集前缀注册表与资源上限：

```python
from pathlib import Path
import purrdf

source = Path("void-source.trig").read_text()
void_config = Path("void.json").read_text()
description = purrdf.project(
    source,
    format=purrdf.RdfFormat.TRIG,
    profile="void",
    config=void_config,
)
Path("void.tar").write_bytes(description.archive)
```

可移植的 `void-source.trig`、`void.json` 与 `dcat-rdf.json` 示例位于
`crates/rdf/tests/fixtures/dataset-description/`。

随附式的 RO-Crate 打包使用同一个调用，把 `assets=` 设为一个仅含载荷的规范 USTAR 归档，
并配置 `packaging: "attached"`。结果包含精确的载荷、确定性的元数据与自包含的预览；
缺失、无主、保留或尺寸不一致的成员会抛出 `ValueError`。参见可运行的、会生成文件的
[`projection_roundtrip.py`](https://github.com/Blackcat-Informatics/purrdf/blob/main/bindings/python/examples/projection_roundtrip.py)
示例。

对于大型 LPG 载体，`purrdf.project_artifacts(...)` 会调用一个事务式的工件回调，带有
包/工件开始、有界分块、工件结束、提交与中止事件。可选的进度回调会收到不可变的
`ProjectionProgress` 快照；回调中的异常会中止该包并原样返回。这条路径保留所选的规范
LPG 模型，但不保留完整的工件体或 USTAR 字节。参见可运行的原子目录式
[`projection_stream.py`](https://github.com/Blackcat-Informatics/purrdf/blob/main/bindings/python/examples/projection_stream.py)
示例。

## 用 SHACL 验证

SHACL 引擎位于 `purrdf.shapes`（与 Rust crate 同名；`purrdf.shacl` 是向后兼容的别名）：

```python
from purrdf import shapes

report = shapes.validate(shapes_ttl=my_shapes, data_nt=my_data)
print(report["conforms"])
```

完整的 SHACL Core、SHACL-SPARQL 约束/目标，以及经由 `shapes.entail(...)` 的 SHACL-AF
`sh:rule` 蕴涵。可复用的已解析形状为 `shapes.Shapes(shapes_ttl).validate_nt(data_nt)`。

## 用 ShEx 验证

```python
from purrdf import shex

results = shex.validate(
    my_schema_shexc,
    my_data_ttl,
    [("https://example.org/alice", "https://example.org/PersonShape")],
)
print(all(entry["conformant"] for entry in results))
```

ShEx 2.1 验证器通过官方 shexTest 套件中 1,105/1,105 个尝试的验证测试（见仓库的
`docs/CONFORMANCE.md`）。

## 蕴涵机制

SPARQL 蕴涵机制（entailment regime）位于 `purrdf.entail`（与 `purrdf-entail` Rust
crate 同名）。它在某一蕴涵机制自身规范的规则表下对数据集求闭包，完全不接受形状——
不要与 `purrdf.shapes.entail(...)` 混淆，后者应用的是*形状*图所声明的 SHACL-AF
`sh:rule`。

```python
import purrdf
from purrdf import entail

dataset = purrdf.RdfDataset(my_turtle, purrdf.RdfFormat.TURTLE)
closure, report = entail.materialize(dataset, "rdfs", "")
print(closure.to_nquads())
print(report)
```

对于手握文档而非已解析数据集的调用方，`entail.materialize_nt(text, regime, program)`
接受 N-Triples/N-Quads 并返回 `(canonical_nquads, report)`。二者都接受以普通字符串
（`"simple"`、`"rdf"`、`"rdfs"`、`"owl-rl"`、`"owl-direct"`、`"rif"`、`"d"`）或
`entail.Regime.RDFS` 给出的蕴涵机制。

**全部七种蕴涵机制都会求闭包；没有一种被拒绝。**第三个参数是该蕴涵机制自身的规则
文档。六种机制不需要，因此传 `""`——而传入非空值会抛出异常，而不是被静默丢弃。
`"rif"` 是例外：它在*调用方*的规则下蕴涵，而 PurRDF 并不声明这些规则，因此其
`program` 是一份规范性的 RIF-in-XML 文档：

```python
closure, report = entail.materialize(dataset, "rif", my_rif_xml)
```

`"owl-direct"` 同样不接受 program，而这是一项声明而非疏漏：它的额外输入是*查询*的
类表达式，而这一表面是对数据集求闭包而非回答查询——因此它所运行的是与查询无关的
tableau 增强（分类、实现、蕴涵的角色断言，以及 tableau 对本体自身命名词项所判定的
`owl:sameAs` 同一性）。

**推理报告是第二个返回值，且永远不可省略。**它是一份字节稳定的渲染，说明哪些规则触发
了、触发了多少次，哪些规范规则*没有*触发，本次运行把哪些构造留在了边界处，消耗了求值
器固定上限中的多少，以及所运行演算的契约哈希——这样，一个在不同规则集下铸造的缓存
闭包就可以被拒绝，而不是被信任。

规则表可以直接读取，因此覆盖率是你能测量的东西，而不是凭信念接受的东西：

```python
defined = entail.rules("owl-rl")             # 78 — OWL 2 Profiles §4.3 Tables 4–9
fired = entail.implemented_rules("owl-rl")   # 78
missing = [rule for rule in entail.rules("rdfs") if rule not in entail.implemented_rules("rdfs")]
# [] — RDFS fires 18 of its 18 rules; the gap is legitimately empty
added = entail.extensions("owl-rl")          # ['ext-eq-diff-sym']
```

`extensions(regime)` 是第三份、与前两者不相交的清单：本构建会触发、但**没有任何规范
表格陈述**的规则。`owl-rl` 有一条——`ext-eq-diff-sym`，即 `owl:differentFrom` 的
对称性，可靠且形态与 `prp-symp` 完全一样——其余蕴涵机制一条也没有。它在任何蕴涵机制
的 `rules()` 与 `implemented_rules()` 中都不出现，因此上面的 78 不受其影响：那两个
是关于规范的陈述，而触发一条表格中省略的可靠规则并不改变表格所说的内容。这是一个独立
的问题，而不是通过物化一个数据集再读推理报告的 `extension` 行才能得知的东西——尽管
报告说的是同一件事，且二者不可能分歧。

`rdfD1`、`rdfD1a`、`rdfs14` 与 `rdfs14a` 在那个已触发的集合中，且每一条都对一个
*新鲜的*空节点作结论。受限的 chase 铸造一个以前沿寻址的 Skolem 见证并在其下求闭包，
因此这些规则确实运行了——但当闭包被物化回去时，每一条提及见证的结论都被扣留，因为
SPARQL 蕴涵机制从作用域图中取答案，而一个铸造出来的空节点不在其中。报告以一行
`boundary surrogate` 而不是一条缺失的规则来陈述这一点，并且 `completeness` 读作
`exact-within-boundaries` 而不是 `exact`。

**78 / 78 是规则表覆盖率，而规则表覆盖率不是蕴涵一致性。**二者分开度量，只陈述前者
正是推理报告存在的目的所要防止的过度声称：在这份随库固化的 W3C OWL 2 RL 蕴涵测试
语料上，本 chase 达到了 27 个已发布正例蕴涵中的 27 个，并在 23 个负例中的 23 个上与
W3C 一致——其中 3 个被*反驳*（判定为非蕴涵），20 个被*承认*（闭包已算出并观察到不含
该非结论）。负例数字应读作「未发现不可靠之处」，而永远不是「证明了 23 个非蕴涵」。
两个数字都是真的。完整记分板、带类型的分歧台账以及其余每个套件都在
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md)。

以下情况抛出 `ValueError`：未知的蕴涵机制拼写（消息会列出可接受的集合）；`program`
与蕴涵机制不匹配——除 `"rif"` 之外的任何机制收到非空值，或 `"rif"` 无法把它解析为
规范性的 RIF-in-XML 文档；以及求值上限耗尽。上限耗尽是一次拒绝，绝不会把截断的闭包
当作完整闭包交回。是 `"owl-direct"` 或 `"rif"` 本身并不构成拒绝：二者都会物化。

## 描述逻辑推理服务

物化是 chase。OWL 2 直接语义*推理机*是同一模块上的第二条通道——一个 SHOIQ(D)
hypertableau——它的每一项服务都在 `purrdf.entail` 上。每项服务接受一份 N-Triples
（或 N-Quads）文档并以元组形式返回 `(answer, certificate)`，因此调用方是解包证据，
而不是可以选择不要它：

| 服务 | 调用 | 答案 |
| --- | --- | --- |
| 一致性 | `entail.consistency(data)` | `consistency true` / `false` / `unknown`——`unknown` 表示 tableau 达到了步数上限，且绝不会被折叠为 `false` |
| 分类 | `entail.classify(data)` | `equivalent`、`subclass`（传递闭包）、`direct`（其约简）与 `unsatisfiable` 各行 |
| 实现 | `entail.realize(data)` | 命名个体的 `type` 行，随后是最具体的 `direct-type` 行 |
| 实例检索 | `entail.instances(data, class_)` | `instance <term>` 行；`class_` 是**一个** N-Triples 词项，含尖括号 |
| 公理蕴涵 | `entail.entails(data, axiom)` | `entails true` / `false` / `unknown`，随后是该公理被*读入*后的形态，以便看出其谓词选择了哪一类 |
| Profile 认证 | `entail.profile(data)` | `certified <profile>` 行，最严格者在前（`EL`、`QL`、`RL`、`DL`、`Full`） |
| 模块抽取 | `entail.extract_module(data, signature, method)` | 局部性模块，以规范 N-Quads 给出；`method` 为 `"bot"`、`"top"` 或 `"star"` |
| 论证 | `entail.justify(data, axiom)` | 仍能蕴涵 `axiom` 的本体最小子集，以规范 N-Quads 给出 |
| 证明 | `entail.explain_conclusion(data, regime, conclusion)` | `asserted`、`steps`，以及推导所引用的每条规则各一行 `rule` |

### 证明项：产出需选装，消费有检查器

上面的每个调用都**不记录**任何东西并返回二元组。`entail.prove` 是选装项：它记录某项
服务所做的 tableau 运行——代价是每次运行的完成图——并返回一个三元组，第三个元素是一份
`purrdf-dl-proof 1` 文档。答案与证书在两种情况下字节完全相同，因为记录是推理机对自身
的观察，而不是它所读取的一个开关。

| 服务 | 调用 | 答案 |
| --- | --- | --- |
| 证明 | `entail.prove(data, service, argument, step_cap, work_cap)` | `(answer, certificate, proof)`；proof 是一个 `purrdf-dl-proof 1` 块，其头部由证明项派生，`body` 各行是该项自身的规范字节的小写十六进制 |
| 检查证明 | `entail.check_proof(data, service, argument, answer, certificate, proof)` | `purrdf-dl-proof-check 1` 报告——它检查过的摘要与输入身份、它重放的运行，以及 `attested`/`trusted`/`unattested` 计数与检查所依赖的、与产出方共享的组件 |
| 服务集合 | `entail.proof_services()` | 证明项可以针对的七项服务，因此该集合是可测量的而非写死在这里 |

`service` 是那七项之一；`argument` 是该问题在该服务语法下的自身输入——
`consistency`/`classify`/`realize` 为 `""`（非空值会抛出而不是被丢弃），
`class-satisfiability`/`instances` 为**一个** N-Triples 词项，`entails` 为**一条**
三元组，`extract-module` 为一行 `method <bot|top|star>` 后接每行一个词项。
`entail.Reasoner(data, proofs=True)` 是会话级的选装，`session.prove(service, argument)`
是在只解析一次的文档上进行的同一调用。

`check_proof` 中没有任何东西信任产出方：本体从 `data` 解析，问题从 `service` 与
`argument` 重新推导，声称从 `answer` 自身的语法中读回，检查上下文来自该调用自己执行
的反向映射。针对不同本体、不同问题或不同答案的证明会被拒绝——读作
`availability not-recorded` 的文档同样会被拒绝，因为一个无人要求记录的答案绝不能被
呈现为已验证的答案。

下面三项服务属于 **chase** 通道而非 tableau，其证书是 `purrdf-reasoning-report 4`
块而非 DL 块。请注意名称的碰撞，且两个名称都是对的：`entail.entails` 向 tableau 询问
OWL 2 RDF 映射中的一条*公理*，而 `entail.graph_entails` 向蕴涵机制的*规则表*询问一个
前提是否蕴涵一个结论*图*。

| 服务 | 调用 | 答案 |
| --- | --- | --- |
| 确定答案 | `entail.certain_answers(regime, data, pattern, imports)` | `mechanism`，每个投影变量一行 `var`，每个确定答案一行 `row`，以及行集可能不完备的每个原因一行 `limit` |
| 图蕴涵 | `entail.graph_entails(regime, premise, conclusion, imports)` | `mechanism <name>`，随后是 `entailment entailed` / `not-entailed` / `undecided`——三种裁决，绝不是两种 |
| 已验证蕴涵 | `entail.verify_entailment(regime, premise, conclusion, imports)` | 上述内容加上 `warrant present`/`absent` 与 `verified true`/`false`/`not-applicable` |

`pattern` 是在任意位置（**谓词**位置也包括在内）带 `?name` 的 N-Triples；其中的空节点
是非区分变量，受匹配约束但不投影，这正是 SPARQL 对查询空节点的定义。RDF 1.2 三元组项
内部的变量是普通变量——它绑定，也是一列——并且同一个*名字*无论写在哪里都是同一个
*变量*，因此 `?x <ex:p> <<( ?x <ex:q> <ex:r> )>>` 就是它字面上的那个连接。一行是知识库
在其下*蕴涵*该模式的一个代换——在每个模型中都为真，而不只是出现在某一个闭包中。唯一
不接受变量的槽位是字面量的**数据类型**：`"5"^^?d` 要求在一个存放 IRI 而非词项的位置上
绑定，会抛出指明它的 `ValueError`。

谓词变量像任何其他变量一样投影，而在 `OWL_RL` 下它还会渲染一行 `limit`：它遍历整个
谓词词汇表，于是既遍历了规则表中没有任何规则作结论的模式谓词，也遍历了表格之外的
机制所判定的构造，而行所取自的闭包两者都不包含。

**不含** `?name` 的模式是一个结论*图*，因此 `certain_answers` 与 `graph_entails` 问的是
同一个问题，并通过同一次折叠作答：mechanism 是实际抵达它的那一个，关系没有列——
`yes` 是一行裸 `row`，`no` 则一行也没有。当有东西需要投影时，规则表之外的五种机制不会
运行，因为对它们中任何一个所判定的内容做投影变量是另一个问题；「本该需要其中某一个」
会以一行指明该通道的 `limit` 到达，绝不会是一个穷尽式的空答案。

`mechanism` 说明七种机制中*哪一种*得出了裁决。`strict-table` 是该蕴涵机制自身的
规则表，运行一次；`refutation`、`freeze`、`comprehension`、`reflexivity` 与
`data-range` 各自存在，是因为那张表对这种形态的结论*不作判定*，而它们没有一个给表格
增加规则。`composite` 是两种或更多上述机制在同一结论上的折叠——每一种消耗掉它读取的
三元组并把其余的交给下一个——它被如此拼写，而不是用任一成员的名字，因为那样会表示
单一机制就已足够。

`entailment not-entailed` 是一个**证明**——过程对这个前提是完备的，因此映射的缺失就是
蕴涵的缺失——而 `undecided` 是不完备的过程有权改说的话。把后者读作前者，会把本库的
一个局限变成关于你的本体的一个错误陈述。

### `imports`——前提自称并非全部的那些文档

`imports` 是一个有序的 `(ontology_iri, document)` 对序列，其中 `document` 是与前提
完全一样的 N-Quads（或 N-Triples）文本。带有 `owl:imports` 的本体声明其公理是自身的
*加上*它所命名的那些文档的，因此只在前提上作答会回答另一个问题——那些文档就放在这里，
而你的 `owl:imports` 三元组保持在你写下它的位置不动。

**PurRDF 不拉取任何东西。**你没有提供的本体 IRI 会抛出指明该文档的 `ValueError`，
绝不会有网络访问，也绝不会有静默为空的导入。`[]` 是普通的*什么也不导入*情形；该参数
是必填而非默认的，并在全部四个宿主上处于同一位置，因此同一种调用形态在 Python、
JavaScript、C 与 Rust 中都可用。解析是传递到不动点的，因此所提供文档自身的
`owl:imports` 同样会被跟进。

```python
from purrdf import entail

premise = (
    "<https://example.org/o>"
    " <http://www.w3.org/2002/07/owl#imports> <https://example.org/schema> .\n"
    "<https://example.org/socrates>"
    " <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://example.org/Man> .\n"
)
schema = (
    "<https://example.org/Man>"
    " <http://www.w3.org/2000/01/rdf-schema#subClassOf> <https://example.org/Mortal> .\n"
)
conclusion = (
    "<https://example.org/socrates>"
    " <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://example.org/Mortal> .\n"
)

answer, _ = entail.graph_entails(
    "owl-rl", premise, conclusion, [("https://example.org/schema", schema)]
)
assert "entailment entailed" in answer

# The same call with nothing supplied refuses BY NAME rather than reasoning over a
# premise that is missing the axioms it told you about.
try:
    entail.graph_entails("owl-rl", premise, conclusion, [])
except ValueError as refusal:
    assert "https://example.org/schema" in str(refusal)
```

```python
from purrdf import entail

ontology = (
    "<https://example.org/Cat>"
    " <http://www.w3.org/2000/01/rdf-schema#subClassOf>"
    " <https://example.org/Animal> .\n"
    "<https://example.org/felix>"
    " <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
    " <https://example.org/Cat> .\n"
)

answer, certificate = entail.consistency(ontology)
assert answer.strip() == "consistency true"
assert certificate.startswith("purrdf-dl-certificate 1")
assert "completeness decided" in certificate

answer, _ = entail.instances(ontology, "<https://example.org/Animal>")
assert answer.strip() == "instance <https://example.org/felix>"

answer, _ = entail.profile(ontology)
assert answer.splitlines()[0] == "certified EL"
```

证书才是要点，而且每一种证据各有不同的证书。`consistency`、`classify`、`realize`、
`instances` 与 `entails` 渲染一个 `purrdf-dl-certificate 1` 块，携带 DL 通道自身的
完备性——`decided`、`decided-within-boundaries`（某条公理从未成为 DL 子句，每个此类
构造都被点名）或 `budget-exhausted`。这与 chase 报告的概念*不同*——后者是两张规则表
相减——并且渲染在不同的横幅之下，因此二者不可能被互相误解析。`profile` 完全不报告
搜索——它纯粹是语法性的——因此它渲染一个以 `one-directional true` 结尾的
`purrdf-owl-profile-certificate 1` 块：认证证明成员资格，违反并不证明非成员资格。
`extract_module` 渲染 `purrdf-module-extraction 1`，其 `conservative` 行说明模块是最小
的还是一个可靠的超集。`justify` 渲染 `purrdf-justification 1`，`explain_conclusion`
渲染 `purrdf-chase-proof 1`；二者都重新检查自己的答案而不是复述它们——`sufficient`
与 `minimal` 在论证本身以及每个少一条公理的子集上重新判定，而证明的 `derived-*` 各行
是检查器从证明项重新推导出的内容，而不是证明所声称的内容。

tableau 不执行推导步骤，因此 `justify` 是一个*论证*，并刻意不称为证明；
`explain_conclusion` 是 chase 通道上真正具有推导性的那一个。它们是不同种类的东西，
而不是同一事物的两种拼写，这正是没有单一 `explain` 的原因。

除 `completeness` 之外，`purrdf-dl-certificate 1` 块还携带八个搜索成本计数器——
调用方用来区分「完成了的判定」与「仅仅停下了的判定」所需的数字：

| 行 | 计数内容 |
| --- | --- |
| `steps` | 已用的轮数，对照每次判定的轮数上限 |
| `budget` | 该判定所在的轮数上限（知识库自身派生的上限，或收窄它的 `step_cap`） |
| `work` | 已用的匹配、扫描、闭包与克隆工作量，对照工作量上限 |
| `work-budget` | 该判定所在的工作量上限（派生的，或收窄它的 `work_cap`） |
| `decisions` | 本次运行做了多少次子判定 |
| `peak-nodes` | 某次判定构建的最大完成图 |
| `disjunctions` | tableau 的分情形规则触发了多少次 |
| `peak-depth` | 该规则的分支栈达到了多深 |

`step_cap` 与 `work_cap`（默认均为 `0`，表示「不收窄」）出现在上面每一项 DL 服务以及
`Reasoner` 的构造函数上；每一个只能**收紧**知识库自身派生的上限，绝不能放宽，而被
收窄进上限的运行会回答 `unknown`，而不是一个它实际上并未判定的 `false`。

这里没有任何东西重新实现推理机：每个入口点都经由 WebAssembly 与 C 宿主所调用的同一个
共享边界，对照同一份已提交的黄金向量工件检查，因此四个宿主对同一输入返回字节完全相同
的结果。

## SPARQL 属性函数

**属性函数**（property function）是从谓词位置调用的、由宿主提供的关系。与扩展函数
不同，它是一个行源，因此一次调用可以发出零行、一行或多行。`Store.query` /
`Store.query_governed` / `Store.update` / `Store.update_governed`（以及
`MutableDataset` 上同样的四个）把关系当作数据接受，只为本次调用注册：

```python
import purrdf

EX = "http://example.org/"
store = purrdf.Store()

rows = store.query(
    f"SELECT ?person ?team WHERE {{ ?person <{EX}rel/memberOf> ?team }}",
    relations={
        f"{EX}rel/memberOf": (
            1,  # subject-side arity
            1,  # object-side arity
            [
                [purrdf.NamedNode(f"{EX}ada"), purrdf.NamedNode(f"{EX}alpha")],
                [purrdf.NamedNode(f"{EX}chen"), purrdf.NamedNode(f"{EX}beta")],
            ],
        )
    },
)
```

表也可以写成 RDF 而非 Python——存储自身默认图中的一个由 `rdf:List` 组成的
`rdf:List`，每个内层列表一行——并以其头节点命名：

```python
store.query(query, relations_from_graph={f"{EX}rel/memberOf": (purrdf.NamedNode(f"{EX}memberTable"), 1, 1)})
```

已注册的 IRI 在谓词位置上被**精确**识别，因此抵达它不需要命名空间声明。传入
`property_fn_namespaces=[f"{EX}rel/"]` 则要求更严格的读法：该前缀之下的每个谓词都
成为一次调用，而*未注册*的谓词是硬错误，不会变成一个悄悄匹配不到任何东西的三元组
模式。重复的 IRI、参差不齐的表、断裂的列表或指向空无的头节点，会在其提供之处抛出
`ValueError`。

第三种写法根本不是表。`path_relations` 在存储自身的边上注册一次**路径见证**（path
witness）遍历：一次调用读作 `?start <iri> ( ?end ?pathId ?len ?step ?node ?edge )`，
每一跳发出一行，`?edge` 绑定到作为 RDF 1.2 三元组项的被遍历陈述，因此 `GROUP BY ?pathId`
配合 `ORDER BY ?step` 就能在查询内部重组一整条游走。规格中的每个字段都是必填的——
PurRDF 不发明关系 IRI，也不发明遍历包络：

```python
store.query(
    "SELECT ?end ?step ?node WHERE { <http://example.org/a> "
    "<http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) } "
    "ORDER BY ?len ?step",
    path_relations={
        "http://example.org/pf#walk": (
            [(purrdf.NamedNode("http://example.org/p"), "forward")],  # steps
            1, 4,          # min_hops, max_hops
            1024, 100000,  # max_paths_per_seed, max_expansions_per_invocation
            "walk",        # "walk" (every simple-prefix witness) | "shortest"
        )
    },
)
```

注册是按调用进行的且不携带任何可调用对象，因此整个求值仍在释放 GIL 的状态下运行。
在 Rust 侧是任意宿主闭包的那些属性函数——全文索引、GeoSPARQL 关系、嵌入 k 近邻
关系——不跨越这一边界；只有这三种数据形态的注册跨越它。

## 基础 IRI

拼写了相对 IRI 的文档需要一个基础（base）。每个解析入口点都接受可选的 `base=` 关键字
参数（`purrdf.parse(text, format, base=...)`、`RdfDataset(text, format, base=...)`、
各 `Store` 加载器，以及 JSON-LD 与 RDF/XML 转换器），`shapes.validate` /
`shapes.entail` 则为形状文档接受 `shapes_base=`。文档内的指令（`@base`、`BASE`、
`xml:base`、`@context.@base`）优先于关键字参数。二者都不在作用域内时，相对引用会抛出
代码为 `iri-relative-no-base` 的 `ValueError`——对于以字符串形式交来的文本，PurRDF
没有检索 IRI，也不会杜撰一个。N-Triples 与 N-Quads 按语法不允许相对引用，因此不需要
基础。

## rdflib 兼容层

本包附带一个基于原生引擎的 rdflib 形态 API：

```python
from purrdf.compat.rdflib import Graph, URIRef

g = Graph()
g.parse(data=my_ntriples, format="nt")
print(len(g), g.serialize(format="turtle"))
```

若需要逐字不改的 `import rdflib`，安装可选装的 extra：

```sh
pip install purrdf[rdflib]
```

这会拉入独立发行的
[`purrdf-rdflib`](https://github.com/Blackcat-Informatics/purrdf/tree/main/bindings/python-rdflib-shadow)
分发包，其顶层 `rdflib` 包重新导出兼容层的表面，于是现有第三方代码中的
`import rdflib` / `from rdflib.namespace import RDF` 便透明地运行在 `purrdf` 之上。
**注意：**该影子包占用了 `rdflib` 这一导入名，绝不可与真正的
[`rdflib`](https://pypi.org/project/rdflib/) 同时安装——二者无法共存于同一环境。
它被刻意做成独立分发包（从不打进主 `purrdf` wheel），正是为了让需要真实 rdflib 的环境
直接不装它即可。

## GTS 图传输与关系型导出

GTS 是 PurRDF 面向 RDF 1.2 图的单文件、内容寻址、仅追加的容器。从四元组构建一个，
然后直接导出到关系型存储：

```python
import purrdf

gts_bytes = purrdf.gts_from_quads(my_nquads_bytes, format=purrdf.RdfFormat.N_QUADS)

purrdf.gts_to_sqlite(gts_bytes, "graph.db")
purrdf.gts_to_duckdb(gts_bytes, "graph.duckdb")
files = purrdf.gts_to_parquet(gts_bytes, "out/")
```

同样的入口点也归组在 `purrdf.gts` 之下以便发现。

## 进一步了解

- 仓库：<https://github.com/Blackcat-Informatics/purrdf>
- 项目主页：<https://blackcatinformatics.ca/purrdf/>
- GTS 规范、一致性矩阵与完整文档位于仓库的
  [`docs/`](https://github.com/Blackcat-Informatics/purrdf/tree/main/docs) 之下。

以 MIT OR Apache-2.0 双许可发布，由使用者任选其一。
