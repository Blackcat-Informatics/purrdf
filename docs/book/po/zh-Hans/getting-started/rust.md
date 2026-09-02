<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->
<!--
zh-Hans 译稿（第一阶段）。与 docs/book/src/getting-started/rust.md 逐段对应，
待 mdbook-i18n-helpers 的 .po 工作流落地后倒入 msgstr。代码块与英文原文逐字节相同。
-->

# 入门：Rust

Rust 下游只需要一个依赖：门面 crate（umbrella crate）
[`purrdf`](https://crates.io/crates/purrdf)。它在根部重新导出 RDF 1.2 实现接口，并把
其余每个已发布的 crate 挂在一个稳定的模块之下（`purrdf::sparql`、`purrdf::shapes`、
`purrdf::shex`、`purrdf::gts`、`purrdf::entail`、`purrdf::validate`、`purrdf::slice`、
`purrdf::iri`、`purrdf::xsd`、`purrdf::events`）——凡是消费者有理由导入的内容，都可直接
从 `purrdf` 访问，绝不需要伸手进子 crate。

```sh
cargo add purrdf
```

MSRV 为 Rust **1.96**（仅限 stable 工具链；按政策，工作区不含任何 nightly 特性）。

**译注：中国大陆镜像。**从中国大陆访问 crates.io 时常有延迟或间歇性不可达。常用的 crates.io
镜像有清华大学 TUNA（`mirrors.tuna.tsinghua.edu.cn/crates.io-index`）、中国科学技术大学 USTC
（`mirrors.ustc.edu.cn/crates.io-index`）与 `rsproxy.cn`，通过 `~/.cargo/config.toml` 中的源替换
（`[source]`）启用。具体配置请参考各镜像站的说明，本文不再重复。

## 构建、冻结、序列化、解析

```rust,ignore
use purrdf::{parse_dataset, serialize_dataset, RdfDatasetBuilder, RdfLiteral, SerializeGraph};

// Build a dataset in interned TermId space.
let mut b = RdfDatasetBuilder::new();
let alice = b.intern_iri("https://example.org/alice");
let knows = b.intern_iri("http://xmlns.com/foaf/0.1/knows");
let bob = b.intern_iri("https://example.org/bob");
let name = b.intern_iri("http://xmlns.com/foaf/0.1/name");
let hi = b.intern_literal(RdfLiteral::simple("Alice"));
b.push_quad(alice, knows, bob, None);
b.push_quad(alice, name, hi, None);
let ds = b.freeze().expect("freeze");

// Serialize to any native codec and parse back, losslessly.
let ttl = serialize_dataset(&ds, "text/turtle", SerializeGraph::Dataset).unwrap();
let back = parse_dataset(&ttl, "text/turtle", None).unwrap();
assert_eq!(back.quad_count(), 2);
```

构建器→冻结的两段式划分是这套 API 的核心：先在可变的 `RdfDatasetBuilder` 上驻留
（intern）词项、推入四元组，再 `freeze()` 成不可变、已建索引的 `RdfDataset`，所有引擎
（SPARQL、SHACL、ShEx、蕴涵）都在其上求值。参见 [驻留数据集 IR](../concepts/interned-dataset.md)。

## 直接解析文本

```rust,ignore
let turtle = r#"
    @prefix ex: <https://example.org/> .
    ex:cat ex:says "meow" .
"#;
let dataset = purrdf::parse_dataset(turtle.as_bytes(), "text/turtle", None)
    .expect("valid Turtle");
assert_eq!(dataset.quad_count(), 1);
```

畸形输入会得到一个带类型的 `RdfDiagnostic`，在编解码器能够给出的情况下附带源位置——
绝不会是静默的部分解析。

## 访问其他引擎

每个引擎都挂在同一个门面之下。例如，零依赖的 IRI 叶 crate 与 ShEx 模式层：

```rust,ignore
let iri = purrdf::iri::parse("https://example.org/cat").expect("valid IRI");
assert_eq!(iri.as_str(), "https://example.org/cat");

let schema = purrdf::shex::parse_shexc(
    "PREFIX ex: <https://example.org/>\nex:Cat { ex:says . }",
    None,
).expect("valid ShExC");
```

## 何时改为依赖子 crate

大多数应用止步于 `purrdf` 即可。各子 crate
（`purrdf-core`、`purrdf-rdf`、`purrdf-columnar`、`purrdf-sparql-algebra`、
`purrdf-sparql-eval`、`purrdf-sparql-results`、`purrdf-cdt`、`purrdf-shapes`、
`purrdf-shex`、`purrdf-gts`、`purrdf-datalog`、`purrdf-entail`、`purrdf-geo`、
`purrdf-text`、`purrdf-validate`、`purrdf-slice`、`purrdf-iri`、`purrdf-xsd`、
`purrdf-events`、`purrdf-wasm`）是为只想要恰好一个引擎的消费者准备的——例如，
一个只需要解析 IRI 的工具可以单独依赖零依赖的 `purrdf-iri`。crate 一览见
[仓库 README](https://github.com/Blackcat-Informatics/purrdf#crate-map)。

每个发布 crate 都能干净地构建到 `wasm32-unknown-unknown`，因此同一条 Rust 代码路径
在原生宿主与 wasm 宿主中都可用。

## 下一步

- [驻留数据集 IR](../concepts/interned-dataset.md)——IR 如何工作，以及它为何快。
- [图、表格与 Research Object（RO）投影](../concepts/projections.md)——经由门面 crate 的确定性 LPG、
  CSVW、OBO Graphs、SKOS 与 Research Object 载体。
- [SPARQL：查询](../sparql/querying.md)——在冻结数据集上运行查询。
- [docs.rs/purrdf](https://docs.rs/purrdf)——完整的 API 参考。
