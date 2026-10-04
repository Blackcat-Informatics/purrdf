// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Chinese technical retrieval with explicit first-party relevance judgments.
//!
//! This is a small acceptance corpus for knowledge graphs, symbolic reasoning,
//! RDF and memetics. Its supplied vocabulary is fixture data, not a general
//! Chinese lexicon or evidence of unrestricted linguistic coverage. Relevance
//! labels express the topic each query requests; partial lexical matches are
//! deliberate distractors. Precision and recall are checked as integer ratios.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTextDirection, TermValue};
use purrdf_sparql_eval::PropertyFunctionRegistry;
use purrdf_text::{
    Analyzer, AnalyzerProfile, GraphSelector, PartitionKey, Segmentation, Stemming,
    TermOccurrenceRelation, TextIndex, TextIndexConfig, TextSearchRelation, rank_partition,
    segment::Dictionary,
};

#[path = "support/sparql.rs"]
mod sparql;

const NOTE: &str = "https://example.org/technical/text";
const OCCURS: &str = "https://example.org/technical/occurs";
const SEARCH: &str = "https://example.org/technical/search";

// Equal costs make fully covered compound terms win over a longer sequence of
// components. Specific ambiguity tests below provide unequal lexical costs.
const TECHNICAL_WORDS: &[&str] = &[
    "知识图谱",
    "符号推理",
    "规则",
    "逻辑",
    "证明链",
    "RDF1.2",
    "数据模型",
    "SPARQL",
    "查询",
    "检索",
    "语义",
    "校验",
    "断言",
    "三元组",
    "命名图",
    "来源",
    "证据",
    "模因传播",
    "社会叙事",
    "影响",
    "因果关系",
    "企业",
    "治理",
    "工业政策",
    "统计学习",
    "大模型",
    "向量检索",
    "嵌入",
    "音乐推荐",
    "语义搜索",
    "查询计划",
    "中文",
    "全文检索",
    "词典",
    "消歧",
    "术语",
    "边界",
    "数据",
    "关系",
    "用户行为",
    "信息系统",
    "推理游戏",
    "叙事",
    "谜题",
    "自然语言",
    "群体行为",
    "通过",
    "组织",
    "使用",
    "存储",
    "支持",
    "关联",
    "进行",
    "生成",
    "维护",
    "保留",
    "展示",
    "解释",
    "中的",
    "与",
    "并",
    "的",
];

const DOCUMENTS: &[&str] = &[
    // 00–01: symbolic reasoning over a knowledge graph.
    "知识图谱通过符号推理组织规则与证明链",
    "符号推理使用逻辑规则校验知识图谱",
    // 02–03: RDF data model and SPARQL querying.
    "知识图谱存储RDF1.2数据模型并支持SPARQL查询",
    "SPARQL查询通过查询计划检索RDF1.2数据模型",
    // 04–05: memetic propagation in social narratives.
    "模因传播通过社会叙事影响群体行为",
    "社会叙事中的模因传播关联因果关系",
    // 06–09: neighboring technical topics, deliberately irrelevant to those queries.
    "向量检索使用嵌入进行语义搜索",
    "统计学习使用大模型生成自然语言",
    "音乐推荐使用用户行为与向量检索",
    "工业政策治理企业数据与信息系统",
    // 10–11: text segmentation and assertion provenance.
    "中文全文检索通过词典消歧维护术语边界",
    "命名图保留三元组断言的来源与证据",
    // 12–13: graph and narrative mentions without symbolic reasoning or memetics.
    "知识图谱展示企业关系与数据来源",
    "推理游戏通过叙事解释谜题",
];

fn analyzer(entries: Vec<(String, u32)>) -> Analyzer {
    let dictionary = Dictionary::with_costs(entries).expect("valid technical vocabulary");
    Analyzer::with_profile(
        AnalyzerProfile::new(256)
            .expect("valid scalar bound")
            .with_segmentation(Segmentation::Dictionary(Arc::new(dictionary))),
    )
    .expect("the vocabulary remains valid under the selected normalization law")
}

fn technical_entries() -> Vec<(String, u32)> {
    TECHNICAL_WORDS
        .iter()
        .map(|word| ((*word).to_owned(), 1))
        .collect()
}

fn corpus(
    documents: &[&str],
    analyzer: Analyzer,
    reverse_insertion: bool,
) -> (Arc<RdfDataset>, Arc<TextIndex>) {
    let mut rows: Vec<_> = documents.iter().enumerate().collect();
    if reverse_insertion {
        rows.reverse();
    }
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri(NOTE);
    for (id, text) in rows {
        let subject = builder.intern_iri(&format!("https://example.org/technical/{id:02}"));
        let literal = builder.intern_literal(RdfLiteral::simple(*text));
        builder.push_quad(subject, predicate, literal, None);
    }
    let dataset = builder.freeze().expect("valid technical RDF corpus");
    let config = TextIndexConfig::new(
        vec![TermValue::iri(NOTE)],
        GraphSelector::Any,
        Analyzer::empty_lexicon(),
    )
    .expect("explicit text predicate")
    .with_analyzer(analyzer);
    let index = TextIndex::from_dataset(&*dataset, &config).expect("technical corpus indexes");
    (dataset, Arc::new(index))
}

#[test]
fn ranked_chinese_technical_queries_meet_explicit_precision_and_recall() {
    let analysis = analyzer(technical_entries());
    let (dataset, index) = corpus(DOCUMENTS, analysis.clone(), false);
    assert_eq!(index.analyzer(), &analysis);
    for (id, document) in index.documents().enumerate() {
        assert_eq!(
            document.subject(),
            &TermValue::iri(format!("https://example.org/technical/{id:02}")),
            "judgments below identify documents by canonical subject order"
        );
    }
    let queries: &[(&str, &[u32])] = &[
        ("知识图谱符号推理", &[0, 1]),
        ("RDF1.2数据模型SPARQL查询", &[2, 3]),
        ("模因传播社会叙事", &[4, 5]),
        ("中文全文检索", &[10]),
        ("命名图断言证据", &[11]),
    ];
    let partition = PartitionKey::new(None, None);
    let mut hit_total = 0;
    let mut cutoff_total = 0;
    let mut relevant_total = 0;
    for &(query, relevant) in queries {
        let terms = index.analyzer().terms(query).expect("valid text analysis");
        let rows = rank_partition(&index, &partition, &terms, Some(relevant.len() as u64))
            .expect("exact integer BM25 ranking");
        let hits = rows
            .iter()
            .filter(|row| relevant.contains(&row.document))
            .count();
        assert_eq!(rows.len(), relevant.len(), "{query:?}: top-k result count");
        // Precision@k = relevant retrieved / k; recall@k = relevant retrieved /
        // all judged relevant. Equal numerators/denominators mean exactly one.
        assert_eq!(
            (hits, relevant.len()),
            (relevant.len(), relevant.len()),
            "{query:?}: precision@k"
        );
        assert_eq!(
            hits * 10_000 / relevant.len(),
            10_000,
            "{query:?}: recall@k"
        );
        let mut returned: Vec<_> = rows.iter().map(|row| row.document).collect();
        returned.sort_unstable();
        assert_eq!(returned, relevant, "{query:?}: exact relevant document IDs");
        hit_total += hits;
        cutoff_total += rows.len();
        relevant_total += relevant.len();
    }
    assert_eq!((hit_total, cutoff_total, relevant_total), (8, 8, 8));

    // The SPARQL relation must apply this stored dictionary, including the
    // mixed-script terms. An independently constructed empty-lexicon analyzer
    // would emit individual Han graphemes and miss these declared words.
    let mut relations = PropertyFunctionRegistry::new();
    relations.register(SEARCH.to_owned(), Arc::new(TextSearchRelation::new(index)));
    assert_eq!(
        sparql::answer(
            &dataset,
            &relations,
            &format!(
                "SELECT ?doc WHERE {{ ?doc <{SEARCH}> \
                 ( \"RDF1.2数据模型SPARQL查询\" ?score ?rank ?lang ?matched ) \
                 FILTER(?matched = 4) }} ORDER BY ?doc"
            ),
        ),
        vec![
            vec!["<https://example.org/technical/02>".to_owned()],
            vec!["<https://example.org/technical/03>".to_owned()],
        ]
    );
}

#[test]
fn dictionary_and_rdf_insertion_order_preserve_ranked_answers_and_identities() {
    let entries = technical_entries();
    let mut reversed = entries.clone();
    reversed.reverse();
    // Identical normalized duplicate entries do not make a new analysis law.
    reversed.push(("ＲＤＦ１．２".to_owned(), 1));
    let first = analyzer(entries);
    let second = analyzer(reversed);
    assert_eq!(first.fingerprint(), second.fingerprint());
    let (_, left) = corpus(DOCUMENTS, first, false);
    let (_, right) = corpus(DOCUMENTS, second, true);
    assert_eq!(left.fingerprint(), right.fingerprint());
    assert_eq!(left.source_fingerprint(), right.source_fingerprint());
    for query in [
        "知识图谱符号推理",
        "RDF1.2数据模型SPARQL查询",
        "模因传播社会叙事",
    ] {
        let partition = PartitionKey::new(None, None);
        assert_eq!(
            rank_partition(
                &left,
                &partition,
                &left.analyzer().terms(query).expect("valid text analysis"),
                None
            ),
            rank_partition(
                &right,
                &partition,
                &right.analyzer().terms(query).expect("valid text analysis"),
                None
            ),
            "{query:?}: rank, integer score and match count are all reproducible"
        );
    }
}

#[test]
fn lexical_costs_resolve_real_chinese_word_ambiguity() {
    let life = analyzer(vec![
        ("研究".to_owned(), 1),
        ("研究生".to_owned(), 10),
        ("生命".to_owned(), 1),
        ("命".to_owned(), 1),
        ("起源".to_owned(), 1),
    ]);
    let student = analyzer(vec![
        ("研究".to_owned(), 1),
        ("研究生".to_owned(), 0),
        ("生命".to_owned(), 1),
        ("命".to_owned(), 0),
        ("起源".to_owned(), 1),
    ]);
    assert_eq!(
        life.terms("研究生命起源").expect("valid text analysis"),
        ["研究", "生命", "起源"]
    );
    assert_eq!(
        student.terms("研究生命起源").expect("valid text analysis"),
        ["研究生", "命", "起源"]
    );
    assert_ne!(life.fingerprint(), student.fingerprint());
    let (_, life_index) = corpus(&["研究生命起源"], life, false);
    let (_, student_index) = corpus(&["研究生命起源"], student, false);
    assert_eq!(
        life_index.source_fingerprint(),
        student_index.source_fingerprint()
    );
    assert_ne!(life_index.fingerprint(), student_index.fingerprint());
    assert_eq!(life_index.term_frequency(0, "生命"), 1);
    assert_eq!(life_index.term_frequency(0, "研究生"), 0);
    assert_eq!(student_index.term_frequency(0, "生命"), 0);
    assert_eq!(student_index.term_frequency(0, "研究生"), 1);
}

#[test]
fn chinese_phrase_adjacency_is_a_real_sparql_occurrence_join() {
    let (dataset, index) = corpus(
        &[
            "知识图谱符号推理",
            "知识图谱使用符号推理",
            "符号推理知识图谱",
        ],
        analyzer(technical_entries()),
        false,
    );
    let mut relations = PropertyFunctionRegistry::new();
    relations.register(
        OCCURS.to_owned(),
        Arc::new(TermOccurrenceRelation::new(index)),
    );
    assert_eq!(
        sparql::answer(
            &dataset,
            &relations,
            &format!(
                "SELECT ?doc WHERE {{ \
                 ?doc <{OCCURS}> ( \"知识图谱\" ?lang ?first ) . \
                 ?doc <{OCCURS}> ( \"符号推理\" ?lang ?second ) . \
                 FILTER(?second = ?first + 1) }}"
            ),
        ),
        vec![vec!["<https://example.org/technical/00>".to_owned()]],
        "intervening terms and reversed order must both reject the phrase"
    );
}

#[test]
fn supplementary_han_and_unknown_units_remain_searchable() {
    let mut entries = technical_entries();
    entries.extend([("𠮷田".to_owned(), 1), ("研究".to_owned(), 1)]);
    let analysis = analyzer(entries);
    let text = "𠮷田研究𠀀\u{31350}知识图谱";
    assert_eq!(
        analysis.terms(text).expect("valid text analysis"),
        ["𠮷田", "研究", "𠀀", "\u{31350}", "知识图谱"]
    );
    let (_, index) = corpus(&[text, "吉田研究知识图谱"], analysis, false);
    let partition = PartitionKey::new(None, None);
    for query in ["𠮷田", "𠀀", "\u{31350}"] {
        let matches = rank_partition(
            &index,
            &partition,
            &index.analyzer().terms(query).expect("valid text analysis"),
            None,
        )
        .expect("supplementary-plane query");
        assert_eq!(
            matches.iter().map(|row| row.document).collect::<Vec<_>>(),
            [0]
        );
    }
    // Exact substring matching crosses dictionary and unknown-unit boundaries;
    // its evidence gives byte ranges, including four-byte Han scalars.
    let fragment = "𠀀\u{31350}";
    let evidence = index
        .surface_index()
        .substring(fragment)
        .expect("one fragment");
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].term.documents(), [0]);
    assert_eq!(evidence[0].ranges.len(), 1);
    assert_eq!(
        &evidence[0].term.text()[evidence[0].ranges[0].clone()],
        fragment
    );
    assert!(
        index.surface_index().phonetic("𠮷田").is_err(),
        "Latin phonetic codes are not Chinese pronunciation evidence"
    );
}

#[test]
fn mixed_identifiers_and_declared_english_stemming_keep_script_boundaries() {
    let mut entries = technical_entries();
    entries.push(("running知识图谱".to_owned(), 1));
    let dictionary = Dictionary::with_costs(entries).expect("mixed-script term admitted");
    let analysis = Analyzer::with_profile(
        AnalyzerProfile::new(256)
            .expect("valid bound")
            .with_stemming(Stemming::English)
            .with_segmentation(Segmentation::Dictionary(Arc::new(dictionary))),
    )
    .expect("declared English analysis with explicit mixed-script vocabulary");
    assert_eq!(
        analysis
            .terms("ＲＤＦ１．２数据模型SPARQL查询 running running知识图谱")
            .expect("valid text analysis"),
        [
            "rdf1.2",
            "数据模型",
            "sparql",
            "查询",
            "run",
            "running知识图谱"
        ]
    );
}

#[test]
fn rdf12_annotation_identity_survives_search_normalization() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/technical/claim");
    let predicate = builder.intern_iri("https://example.org/technical/uses");
    let object = builder.intern_iri("https://example.org/technical/model");
    builder.push_quad(subject, predicate, object, None);
    let triple = builder.intern_triple(subject, predicate, object);
    let note = builder.intern_iri(NOTE);
    let originals = ["RDF1.2数据模型", "ＲＤＦ１．２数据模型"];
    let mut literals = Vec::new();
    for (number, lexical) in originals.into_iter().enumerate() {
        let reifier = builder.intern_iri(&format!("https://example.org/technical/reifier{number}"));
        let literal = builder.intern_literal(RdfLiteral {
            lexical_form: lexical.to_owned(),
            datatype: None,
            language: Some("zh".to_owned()),
            direction: Some(RdfTextDirection::Ltr),
        });
        literals.push(literal);
        builder.push_reifier(reifier, triple);
        builder.push_annotation(reifier, note, literal);
    }
    let dataset = builder
        .freeze()
        .expect("RDF 1.2 annotations and directional literals");
    let before: Vec<_> = literals.iter().map(|&id| dataset.term_value(id)).collect();
    assert_ne!(
        before[0], before[1],
        "normalization does not define RDF identity"
    );
    let triple_before = dataset.term_value(triple);
    let index = TextIndex::from_dataset(
        &*dataset,
        &TextIndexConfig::new(
            vec![TermValue::iri(NOTE)],
            GraphSelector::Any,
            Analyzer::empty_lexicon(),
        )
        .expect("explicit annotation predicate")
        .with_analyzer(analyzer(technical_entries())),
    )
    .expect("annotation text indexes through the Chinese law");
    let partition = PartitionKey::new(None, Some("zh".to_owned()));
    assert_eq!(index.document_count(), 2);
    assert_eq!(index.document_frequency(&partition, "rdf1.2"), 2);
    assert_eq!(index.document_frequency(&partition, "数据模型"), 2);
    let matches = rank_partition(
        &index,
        &partition,
        &index
            .analyzer()
            .terms("ｒｄｆ１．２数据模型")
            .expect("valid text analysis"),
        None,
    )
    .expect("both source spellings are retrieved");
    assert_eq!(
        matches.iter().map(|row| row.document).collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(matches[0].score, matches[1].score);
    assert_eq!(
        literals
            .iter()
            .map(|&id| dataset.term_value(id))
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(dataset.term_value(triple), triple_before);
}
