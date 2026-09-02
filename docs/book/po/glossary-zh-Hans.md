<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# zh-Hans glossary — a gate input, not a page

This file is read by `scripts/check-i18n-glossary.py` (part of `make check`
and CI). It settles one rendering per load-bearing term and, per term, the
renderings that are wrong **for that term**; the gate refuses a `msgstr` that
uses one of them **when its `msgid` carries the term**. A wrong or
inconsistent rendering of a load-bearing term is worse than English with a
gloss, and the gate is what keeps ten translators from producing four words
for one concept. Where this table and §5.4 of
`docs/design/i18n-zh-assessment.md` differ (row 32), this table supersedes it.

Policy, settled by the house page (<https://blackcatinformatics.ca/zh/> and
its `/zh/purrdf/` page):

1. Specification names and acronyms stay English: `PurRDF`, `GMEOW`, `GTS`,
   RDF 1.2, SPARQL, SHACL, ShEx, OWL 2 DL, IRI, JSON-LD, Turtle. Every IRI,
   prefixed name, keyword, diagnostic code, crate/module/API name, CLI flag,
   file path, spec section number (`SEP-0009`, `RDFC-1.0`) and profile
   identifier (`purrdf-rdfc12`) is invariant and appears in backticks where
   the English does. Rows marked **K** below are *enforced*: when a `msgid`
   carries one of the row's Anchor tokens, the `msgstr` must carry it
   verbatim.
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
English, gloss on first use (and enforced, see policy 1).

How the gate reads the table:

* **Anchor** — the English word(s) whose presence in a `msgid` makes the row
  apply, `、`-separated, matched case-insensitively at a word start and as a
  prefix (`entail` covers *entails* and *entailment*; `canonical` covers
  *canonicalize* and *canonicalization*). A rejection is tested against a
  `msgstr` only when its row is anchored in the `msgstr`'s `msgid`: 标准化
  is refused as a rendering of *canonicalization* and untouched as the
  rendering of *standardized* (the English book says "no standardized
  spelling exists"); 蕴含 is refused for *entailment* and untouched as the
  ordinary verb *implies*. A row whose Anchor is `—` is **global**: its
  rejections apply wherever they appear, and its Note must say why (only the
  zh-Hant-register words qualify). A tracked `zh-Hans` Markdown file has no
  `msgid`, so it is checked against the global rows only; the pour into the
  catalogue is where the table is fully enforced.
* **Rejected** — entries separated by `、`; a plain entry is a substring; an
  entry written `/…/` is a Python regular expression, used where a wrong
  rendering would otherwise match across a word boundary (账本 inside
  台账本身) or inside a right one (知识图 inside 知识图谱). Code spans and
  fenced blocks in a `msgstr` are never matched (a page may write
  「不要写 `蕴含`」).
* **K rows** — every Anchor token is an invariant: if the `msgid` carries it
  (case-sensitively, at a word start), the `msgstr` must contain it verbatim.
  研究物件 for *Research Object* or 吉猫协议 for *GMEOW* is refused by this
  arm even though no Rejected entry names it.

The gate's `--self-test` executes, for every rejection, the refused form
under an anchored `msgid`, the row's own rendering under the same `msgid`,
the rejected word in its *other* sense under an unrelated `msgid` (which
must pass), and the rejected word inside a code span (which must pass); for
every K token, a translation that drops it (refused) and one that keeps it
(passes). A rejection without an ordinary-prose neighbour in the script is a
self-test failure, so the table cannot grow a refusal that is proven only one
way.

| # | Term | Anchor | Rendering | Basis | Rejected | Note |
|---:|---|---|---|---|---|---|
| 1 | triple | triple | 三元组 | E, H | — | universal in 知识图谱 literature |
| 2 | quad | quad | 四元组 | E | — | |
| 3 | named graph | — | 命名图 | E | 具名图 | GLOBAL: 具名图 is the zh-Hant rendering, wrong anywhere in a zh-Hans book |
| 4 | dataset | — | 数据集 | E, H | 资料集 | GLOBAL: 资料集 is the zh-Hant rendering, wrong anywhere in a zh-Hans book |
| 5 | blank node | blank node | 空节点 | E | 空白节点 | 空白节点 is fine for an empty DOM node elsewhere |
| 6 | literal | literal | 字面量 | E | 字面值 | 字面值 is fine for a constant's literal value elsewhere |
| 7 | datatype | — | 数据类型 | E | 资料类型 | GLOBAL: 资料类型 is the zh-Hant rendering, wrong anywhere in a zh-Hans book |
| 8 | language tag | language tag | 语言标签 | E | 语言标记 | |
| 9 | IRI | IRI | IRI | K | — | invariant; gloss 国际化资源标识符 once beside it, never instead of it |
| 10 | ontology | ontology | 本体 | E, H | 本体论 | 本体论 is the philosophical sense |
| 11 | knowledge graph | knowledge graph | 知识图谱 | E | `/知识图(?!谱)/` | the audience's own term for the field |
| 12 | reasoning / inference | reasoning、inference | 推理 | E, H | — | house: 「以推理为核心」 |
| 13 | entailment | entail | 蕴涵 | E (logic) | 蕴含 | 蕴含 also circulates for entailment; this book uses 蕴涵 only. As the ordinary verb "implies" it is untouched |
| 14 | entailment regime | entailment regime | 蕴涵机制 | C | — | keep "(entailment regime)" on first use; no W3C precedent |
| 15 | materialization | materialization、materialize | 物化 | E | 实体化 | database/KG usage |
| 16 | closure (inference) | closure | 推理闭包 | E, qualified | — | qualify on first use on a page; bare 闭包 is NOT gated, because 传递闭包 (transitive closure), 宿主闭包 (a host closure, the programming sense) and the verb phrase 求闭包 (compute the closure) are all correct and all contain it |
| 17 | canonicalization | canonical | 规范化 | H | 标准化 | not 标准化 for canonicalization; `RDFC-1.0` is invariant. 标准化 for "standardized" is untouched |
| 18 | canonicalization profile | profile | 规范化 profile | K + H | — | "profile" has no settled rendering (子集/概貌/配置 all circulate); keep English; `purrdf-rdfc12` is invariant |
| 19 | codec / serialization | codec、serialization、serialize | 编解码器 / 序列化 | H, E | — | |
| 20 | round-trip | round-trip | 往返（转换） | E | — | |
| 21 | determinism | determinism、deterministic | 确定性 | E | 决定性 | 决定性 is "decisive" elsewhere |
| 22 | provenance | provenance | 溯源 | H | 出处 | house: 声明溯源; not 出处/来源 (来源 is not gated: it is the ordinary word for a data source) |
| 23 | triple term (RDF 1.2) | triple term | 三元组项 | H | 三元组术语、三元组词项 | adopted by the house `/zh/purrdf/` page; W3C has none |
| 24 | reifier / `rdf:reifies` | reifier、reifies | 具体化节点 / `rdf:reifies` | C | — | 具体化 (reification) is established; the noun for the subject is coined; the IRI never changes |
| 25 | statement layer / annotation | statement layer、annotation | 陈述层 / 注解 | C / E | — | 陈述 (statement) is established; 注解 for the `{| |}` annotation syntax |
| 26 | base direction (`--ltr`/`--rtl`) | base direction | 基础方向 | C, with precedent | 基本方向 | W3C i18n articles in Chinese use 基础方向 for HTML/CSS base direction; the flags are invariant |
| 27 | property function | property function | 属性函数 | E (Jena usage) | — | Chinese Jena material uses 属性函数 |
| 28 | composite datatype (`SEP-0009`) | composite datatype | 复合数据类型 | E | 组合数据类型 | natural DB rendering; the SEP number stays |
| 29 | governor / budget | governor | governor（执行调控器）/ 预算 | K / E | — | no settled rendering for "governor" in this sense; 预算 is established for resource budgets |
| 30 | ledger (`xfail` ledger) | ledger | 台账（预期失败台账） | E, qualified | `/(?<!台)账本/` | 台账 is the engineering register word; 账本 reads as blockchain (the lookbehind spares 台账本身, "the ledger itself"). Usage, not gated: in a COUNT, 台账 may not follow a bare numeral — 「0 台账」 is "zero ledgers"; write 「0 例入账」, 「台账为空」, 「5 例入台账」. (A numeral before 台账 is also a section number or a table reference, so no regex can tell a count from those) |
| 31 | walk vs. path | walk | 游走 vs. 路径 | E | — | the pair maps cleanly (随机游走 is standard); keep the distinction the code makes |
| 32 | out-of-core (outside `purrdf-core`) | out-of-core | 核心之外 / 核心 crate 之外 | C | 核外 | The English is a PUN: "out-of-core" means outside the `purrdf-core` crate — capabilities that arrive as sibling crates through the extension seams — not the CS sense. 核外 / 核外计算 is exactly that CS sense (data larger than RAM, streamed from disk), so 「核外 SPARQL 扩展」 beside 「内存倒排索引」 two paragraphs later reads as a contradiction that the English does not have. Supersedes §5.4 row 32 of the assessment. 核外 elsewhere (核外电子) is untouched |
| 33 | interned / interner | intern | 驻留 / 驻留表 | E (Python community) | — | |
| 34 | fixpoint, semi-naive | fixpoint、semi-naive | 不动点, 半朴素 | E | — | |
| 35 | embedding, kNN | embedding、kNN | 嵌入（向量嵌入）, k 近邻 | E | — | the audience's core vocabulary |
| 36 | slice | slice | 切片 | E | — | |
| 37 | shape, focus node | shape、focus node | 形状, 焦点节点 | E | — | SHACL Chinese usage |
| 38 | quad template | quad template | 四元组模板 | C | — | a first-party extension that SPARQL 1.2 does not define. `check-spec-attribution.py` treats 四元组模板 and `CONSTRUCT 模板` as anchors and accepts the Chinese disclaimers listed in its `DISCLAIMERS` tuple — 「并非 SPARQL 1.2 特性」, 「SPARQL 1.2 并未定义」, 「第一方扩展」, 「PurRDF 的扩展」, 「不属于 SPARQL 1.2」, 「没有四元组模板」 among them; an exact wording is required, as for the English list |
| 39 | PurRDF | PurRDF | PurRDF | K | — | the brand is never translated or lower-cased in prose (`docs/BRAND.md`); `check-brand-casing.py` enforces the casing inside CJK text, this row enforces survival |
| 40 | Research Object (RO-Crate) | Research Object | Research Object（RO） | K | `/研究对象(?![（(]Research Object)/` | a proper noun; 研究对象 means "the object of study". Keep English, or gloss it as 研究对象（Research Object） / 研究对象（Research Object，RO） — the bare word is refused where the msgid carries the term |
| 41 | realization (DL, of an individual) | realization | 实现（realization） on first use / 实例归类 | C | — | bare 实现 collides with 实现 = "implementation", the far commoner sense in this book; gloss it or use 实例归类. Not gated: 实现 in the implementation sense is everywhere |
| 42 | surface (API surface) | surface | 接口 | E | `/表面(?!上\|看来\|来看)/` | technical, not literary: the English "surface" is the API surface. 表面上 / 表面看来 / 从表面来看 ("ostensibly") are spared; 表面 in other senses (表面张力) is untouched by anchoring |
| 43 | seam (extension seam) | seam | 扩展点 | E | — | 接缝 is acceptable but 扩展点 reads naturally to the audience |
| 44 | mint (an IRI, a witness, a blank node) | mint | 生成 | E | 铸造 | technical, not literary: 铸造 ("cast, coin") is the English project idiom carried over; as bronze casting it is untouched |
| 45 | reach (a host, a surface) | reach | 到达 | E | 抵达 | technical, not literary: 抵达 is the literary "arrive" |
| 46 | dropped loudly | loudly | 显式报错丢弃 | E | 大声 | "loudly" is idiom: the drop is reported explicitly; 大声地丢弃 is nonsense in Chinese |
| 47 | full-text search | full-text | 全文检索 | E | 全文搜索 | the audience's core vocabulary |
| 48 | validation report | validation report | 验证报告 | E | 校验报告 | SHACL Chinese usage; bare 校验 is not gated (校验和 is a checksum) |
| 49 | specification and product names | GMEOW、GTS、RDF、SPARQL、SHACL、ShEx、OWL、JSON-LD、Turtle、TriG、N-Triples、N-Quads、RDFC、PostgreSQL、Rust、Python、WebAssembly、JavaScript、TypeScript | as written | K | — | invariant: each survives verbatim, case-sensitively, into the msgstr of any msgid that carries it (吉猫协议 for GMEOW or 波斯特格雷 for PostgreSQL is refused) |

Add a row when a translation coins or settles a term; give it an Anchor,
add a **Rejected** entry only for a rendering that is wrong *for that term*,
and add its ordinary-prose neighbour to the gate's self-test. The gate reads
this table by its header row, so keep the seven columns.
