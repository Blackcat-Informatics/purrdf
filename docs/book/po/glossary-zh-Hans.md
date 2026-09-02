<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# zh-Hans glossary — a gate input, not a page

This file is read by `scripts/check-i18n-glossary.py` (part of `make check`
and CI). Every translated unit — each `msgstr` in `po/zh-Hans.po` and every
line of every tracked Markdown file with `zh-Hans` in its path
(`README.zh-Hans.md`, `docs/book/po/zh-Hans/**`) — is rejected if it uses a rendering listed in the
**Rejected** column: a wrong or inconsistent rendering of a load-bearing term
is worse than English with a gloss, and the gate is what keeps ten translators
from producing four words for one concept.

Policy, settled by the house page (<https://blackcatinformatics.ca/zh/> and
its `/zh/purrdf/` page):

1. Specification names and acronyms stay English: `PurRDF`, `GMEOW`, `GTS`,
   RDF 1.2, SPARQL, SHACL, ShEx, OWL 2 DL, IRI, JSON-LD, Turtle. Every IRI,
   prefixed name, keyword, diagnostic code, crate/module/API name, CLI flag,
   file path, spec section number (`SEP-0009`, `RDFC-1.0`) and profile
   identifier (`purrdf-rdfc12`) is invariant and appears in backticks where
   the English does.
2. Concepts with a settled mainland knowledge-graph (知识图谱) rendering use
   it.
3. A coined term (**C**) carries the English term in parentheses on its first
   use on a page — 「蕴涵机制（entailment regime）」 — and stands alone
   afterwards on that page.
4. Where no rendering is confident (**K**), the English word stays inside the
   Chinese sentence, as the house page does with `LLM` and `DevOps`, glossed
   once.
5. Typography: full-width CJK punctuation (。，：；「」), a half-width space
   between a Latin run and a CJK run, code and identifiers in backticks.

**Basis** legend — **E**: established mainland rendering; **H**: fixed by the
house page; **C**: no established rendering, PurRDF coins one; **K**: keep
English, gloss on first use.

**Rejected** column syntax — entries separated by `、`; a plain entry is a
substring the gate refuses anywhere in a translated unit; an entry written
`/…/` is a Python regular expression, used where a wrong rendering would
otherwise match across a word boundary (账本 inside 台账本身) or inside a
right one (知识图 inside 知识图谱). Only list a rejection that is wrong
*wherever it appears*: 来源 is not listed against *provenance* because it is
the ordinary word for a data source, bare 闭包 is not listed against
*closure* because 传递闭包, 宿主闭包 and 求闭包 are all correct, and a gate that
refuses ordinary prose is a gate that gets switched off. Both of those were
found by running the gate over the first translated drafts, not guessed.

| # | Term | Rendering | Basis | Rejected | Note |
|---:|---|---|---|---|---|
| 1 | triple | 三元组 | E, H | — | universal in 知识图谱 literature |
| 2 | quad | 四元组 | E | — | |
| 3 | named graph | 命名图 | E | 具名图 | 具名图 is the zh-Hant rendering |
| 4 | dataset | 数据集 | E, H | 资料集 | 资料集 is the zh-Hant rendering |
| 5 | blank node | 空节点 | E | 空白节点 | |
| 6 | literal | 字面量 | E | 字面值 | |
| 7 | datatype | 数据类型 | E | 资料类型 | |
| 8 | language tag | 语言标签 | E | 语言标记 | |
| 9 | IRI | IRI | K | — | gloss 国际化资源标识符 once; never translate in running text |
| 10 | ontology | 本体 | E, H | 本体论 | 本体论 is the philosophical sense |
| 11 | knowledge graph | 知识图谱 | E | `/知识图(?!谱)/` | the audience's own term for the field |
| 12 | reasoning / inference | 推理 | E, H | — | house: 「以推理为核心」 |
| 13 | entailment | 蕴涵 | E (logic) | 蕴含 | 蕴含 also circulates; this book uses 蕴涵 only |
| 14 | entailment regime | 蕴涵机制 | C | — | keep "(entailment regime)" on first use; no W3C precedent |
| 15 | materialization | 物化 | E | 实体化 | database/KG usage |
| 16 | closure (inference) | 推理闭包 | E, qualified | — | qualify on first use on a page; bare 闭包 is NOT gated, because 传递闭包 (transitive closure), 宿主闭包 (a host closure, the programming sense) and the verb phrase 求闭包 (compute the closure) are all correct and all contain it |
| 17 | canonicalization | 规范化 | H | 标准化 | not 标准化; `RDFC-1.0` is invariant |
| 18 | canonicalization profile | 规范化 profile | K + H | — | "profile" has no settled rendering (子集/概貌/配置 all circulate); keep English; `purrdf-rdfc12` is invariant |
| 19 | codec / serialization | 编解码器 / 序列化 | H, E | — | |
| 20 | round-trip | 往返（转换） | E | — | |
| 21 | determinism | 确定性 | E | 决定性 | |
| 22 | provenance | 溯源 | H | 出处 | house: 声明溯源; not 出处/来源 (来源 is not gated, see above) |
| 23 | triple term (RDF 1.2) | 三元组项 | H | 三元组术语、三元组词项 | adopted by the house `/zh/purrdf/` page; W3C has none |
| 24 | reifier / `rdf:reifies` | 具体化节点 / `rdf:reifies` | C | — | 具体化 (reification) is established; the noun for the subject is coined; the IRI never changes |
| 25 | statement layer / annotation | 陈述层 / 注解 | C / E | — | 陈述 (statement) is established; 注解 for the `{| |}` annotation syntax |
| 26 | base direction (`--ltr`/`--rtl`) | 基础方向 | C, with precedent | 基本方向 | W3C i18n articles in Chinese use 基础方向 for HTML/CSS base direction; the flags are invariant |
| 27 | property function | 属性函数 | E (Jena usage) | — | Chinese Jena material uses 属性函数 |
| 28 | composite datatype (`SEP-0009`) | 复合数据类型 | E | 组合数据类型 | natural DB rendering; the SEP number stays |
| 29 | governor / budget | governor（执行调控器）/ 预算 | K / E | — | no settled rendering for "governor" in this sense; 预算 is established for resource budgets |
| 30 | ledgered (`xfail` ledger) | 台账（预期失败台账） | E, qualified | `/(?<!台)账本/` | 台账 is the engineering register word; 账本 reads as blockchain. The lookbehind spares 台账本身 ("the ledger itself"), where 账本 straddles two words |
| 31 | walk vs. path | 游走 vs. 路径 | E | — | the pair maps cleanly (随机游走 is standard); keep the distinction the code makes |
| 32 | out-of-core | 核外 | E (HPC) | — | 核外计算 is standard |
| 33 | interned / interner | 驻留 / 驻留表 | E (Python community) | — | |
| 34 | fixpoint, semi-naive | 不动点, 半朴素 | E | — | |
| 35 | embedding, kNN, full-text search | 嵌入（向量嵌入）, k 近邻, 全文检索 | E | 全文搜索 | the audience's core vocabulary |
| 36 | slice | 切片 | E | — | |
| 37 | shape, focus node, validation report | 形状, 焦点节点, 验证报告 | E | 校验报告 | SHACL Chinese usage; bare 校验 is not gated (校验和 is a checksum) |
| 38 | quad template | 四元组模板 | C | — | a first-party extension that SPARQL 1.2 does not define; `check-spec-attribution.py` recognises this rendering as an anchor and the disclaimers 「并非 SPARQL 1.2 特性」/「SPARQL 1.2 并未定义」/「第一方扩展」 |
| 39 | PurRDF | PurRDF | H | — | the brand is never translated or lower-cased in prose (`docs/BRAND.md`); `check-brand-casing.py` enforces it inside CJK text too |

Add a row when a translation coins or settles a term; add a **Rejected**
entry only for a rendering that is wrong wherever it appears. The gate reads
this table by its header row, so keep the six columns.
