// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//! Host-only judged relevance measurement; production scores remain exact integers.
use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_text::{
    Analyzer, AnalyzerProfile, Fixed, GraphSelector, HanCharacterIndex, PartitionKey, TextIndex,
    TextIndexConfig, rank_partition,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    path::Path,
};
const PREDICATE: &str = "https://example.org/relevance/text";
const USAGE: &str =
    "usage: text_relevance ARTIFACT_DIRECTORY [DOCUMENTS QUERIES QRELS [judged-pool]]";

struct Arguments<'a> {
    artifacts: &'a str,
    external: Option<[&'a str; 3]>,
    judged_pool: bool,
}
fn arguments(values: &[String]) -> Result<Arguments<'_>, String> {
    let Some((artifacts, tail)) = values.split_first() else {
        return Err(USAGE.to_owned());
    };
    let (external, judged_pool) = match tail {
        [] => (None, false),
        [documents, queries, judgments] => (
            Some([documents.as_str(), queries.as_str(), judgments.as_str()]),
            false,
        ),
        [documents, queries, judgments, mode] if mode == "judged-pool" => (
            Some([documents.as_str(), queries.as_str(), judgments.as_str()]),
            true,
        ),
        _ => return Err(USAGE.to_owned()),
    };
    Ok(Arguments {
        artifacts,
        external,
        judged_pool,
    })
}

/// Validated bounded grades keep gain arithmetic independent of external integers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Grade {
    None,
    Marginal,
    Supporting,
    Direct,
}
impl Grade {
    fn parse(value: &str) -> Result<Self, String> {
        match value.parse::<u32>() {
            Ok(0) => Ok(Self::None),
            Ok(1) => Ok(Self::Marginal),
            Ok(2) => Ok(Self::Supporting),
            Ok(3) => Ok(Self::Direct),
            _ => Err(format!("grade {value:?} must be an integer in 0..=3")),
        }
    }
    const fn gain(self) -> f64 {
        match self {
            Self::None => 0.0,
            Self::Marginal => 1.0,
            Self::Supporting => 3.0,
            Self::Direct => 7.0,
        }
    }
}
struct Query<'a> {
    id: &'a str,
    stratum: &'a str,
    text: &'a str,
}
struct Inputs<'a> {
    documents: Vec<(&'a str, &'a str)>,
    queries: Vec<Query<'a>>,
    judgments: BTreeMap<String, BTreeMap<String, Grade>>,
}
fn identifier(id: &str, kind: &str) -> Result<(), String> {
    if id.is_empty() || id.chars().any(char::is_whitespace) {
        return Err(format!(
            "{kind} identifier must be nonempty and contain no whitespace"
        ));
    }
    Ok(())
}

/// Validate the complete evaluation universe before building or scoring an index.
fn inputs<'a>(documents: &'a str, queries: &'a str, judgments: &str) -> Result<Inputs<'a>, String> {
    let mut document_ids = BTreeSet::new();
    let mut parsed_documents = Vec::new();
    for line in rows(documents) {
        let (id, text) = line
            .split_once('\t')
            .ok_or("document requires ID and text")?;
        identifier(id, "document")?;
        if !document_ids.insert(id) {
            return Err(format!("duplicate document identifier {id:?}"));
        }
        parsed_documents.push((id, text));
    }
    if parsed_documents.is_empty() {
        return Err("evaluation requires at least one document".to_owned());
    }
    let mut query_ids = BTreeSet::new();
    let mut parsed_queries = Vec::new();
    for line in rows(queries) {
        let fields = line.split('\t').collect::<Vec<_>>();
        let (id, stratum, text) = match fields.as_slice() {
            [id, text] => (*id, "public", *text),
            [id, stratum, text] => (*id, *stratum, *text),
            _ => return Err("query requires ID, optional stratum, text".to_owned()),
        };
        identifier(id, "query")?;
        identifier(stratum, "stratum")?;
        if stratum == "all" {
            return Err("query stratum 'all' is reserved for aggregate metrics".to_owned());
        }
        if text.trim().is_empty() {
            return Err(format!("query {id:?} has no probe text"));
        }
        if !query_ids.insert(id) {
            return Err(format!("duplicate query identifier {id:?}"));
        }
        parsed_queries.push(Query { id, stratum, text });
    }
    if parsed_queries.is_empty() {
        return Err("evaluation requires at least one query".to_owned());
    }
    let mut labels: BTreeMap<String, BTreeMap<String, Grade>> = BTreeMap::new();
    for line in rows(judgments) {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let [query, iteration, document, grade] = fields.as_slice() else {
            return Err("qrel requires query Q0 document grade".to_owned());
        };
        if !matches!(*iteration, "Q0" | "0") {
            return Err(format!("qrel iteration {iteration:?} must be Q0 or 0"));
        }
        if !query_ids.contains(query) {
            return Err(format!("qrel names unknown query {query:?}"));
        }
        if !document_ids.contains(document) {
            return Err(format!("qrel names unknown document {document:?}"));
        }
        let grade = Grade::parse(grade)?;
        let prior = labels
            .entry((*query).to_owned())
            .or_default()
            .insert((*document).to_owned(), grade);
        if prior.is_some_and(|prior| prior != grade) {
            return Err(format!(
                "contradictory qrels for query {query:?}, document {document:?}"
            ));
        }
    }
    for query in &parsed_queries {
        if !labels.contains_key(query.id) {
            return Err(format!("query {:?} lacks judgments", query.id));
        }
    }
    Ok(Inputs {
        documents: parsed_documents,
        queries: parsed_queries,
        judgments: labels,
    })
}

fn rows(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}
fn rank(
    index: &TextIndex,
    query: &str,
    names: &BTreeMap<TermValue, String>,
    judged: Option<&BTreeMap<String, Grade>>,
) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(rank_partition(
        index,
        &PartitionKey::new(None, None),
        &index.query_terms(query)?,
        None,
    )?
    .into_iter()
    .filter_map(|row| {
        let id = &names[index.document(row.document).unwrap().subject()];
        judged
            .is_none_or(|pool| pool.contains_key(id))
            .then(|| id.clone())
    })
    .collect())
}
fn fused(left: &[String], right: &[String]) -> Result<Vec<String>, Box<dyn Error>> {
    let mut totals: BTreeMap<String, Fixed> = BTreeMap::new();
    for ranking in [left, right] {
        for (at, id) in ranking.iter().enumerate() {
            let score = purrdf_retrieval::contribution(Fixed::ONE, at as u64 + 1, 60)?;
            let prior = totals.get(id).copied().unwrap_or(Fixed::ZERO);
            totals.insert(id.clone(), prior.checked_add(score)?);
        }
    }
    let mut sorted = totals.into_iter().collect::<Vec<_>>();
    sorted.sort_unstable_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    Ok(sorted.into_iter().map(|(id, _)| id).collect())
}
// Host evaluation ratios and logarithmic discounts are not retrieval scores.
#[allow(clippy::float_arithmetic, clippy::cast_precision_loss)]
fn metrics(ranking: &[String], grades: &BTreeMap<String, Grade>) -> [f64; 6] {
    let relevant = grades
        .values()
        .filter(|&&grade| grade != Grade::None)
        .count();
    let hits = ranking
        .iter()
        .take(3)
        .filter(|id| grades.get(*id).is_some_and(|&grade| grade != Grade::None))
        .count();
    let p = hits as f64 / 3.0;
    let r = hits as f64 / relevant.max(1) as f64;
    let f = if p + r == 0.0 {
        0.0
    } else {
        2.0 * p * r / (p + r)
    };
    let rr = ranking
        .iter()
        .position(|id| grades.get(id).is_some_and(|&grade| grade != Grade::None))
        .map_or(0.0, |at| 1.0 / (at + 1) as f64);
    let dcg = |grades: &[Grade]| {
        grades
            .iter()
            .take(3)
            .enumerate()
            .map(|(at, &grade)| grade.gain() / ((at + 2) as f64).log2())
            .sum::<f64>()
    };
    let mut ideal = grades.values().copied().collect::<Vec<_>>();
    ideal.sort_unstable_by(|left, right| right.cmp(left));
    let actual = ranking
        .iter()
        .take(3)
        .map(|id| grades.get(id).copied().unwrap_or(Grade::None))
        .collect::<Vec<_>>();
    let denominator = dcg(&ideal);
    [
        p,
        r,
        f,
        f64::from(hits > 0),
        rr,
        if denominator == 0.0 {
            0.0
        } else {
            dcg(&actual) / denominator
        },
    ]
}
#[allow(clippy::float_arithmetic, clippy::cast_precision_loss)]
fn main() -> Result<(), Box<dyn Error>> {
    let values = std::env::args().skip(1).collect::<Vec<_>>();
    let arguments = arguments(&values)?;
    let (documents, queries, judgments) =
        if let Some([documents, queries, judgments]) = arguments.external {
            (
                std::fs::read_to_string(documents)?,
                std::fs::read_to_string(queries)?,
                std::fs::read_to_string(judgments)?,
            )
        } else {
            (
                include_str!("../tests/relevance/documents.tsv").to_owned(),
                include_str!("../tests/relevance/queries.tsv").to_owned(),
                include_str!("../tests/relevance/judgments.tsv").to_owned(),
            )
        };
    let inputs = inputs(&documents, &queries, &judgments)?;
    let directory = Path::new(arguments.artifacts);
    let mut artifacts = std::fs::read_dir(directory)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "cbor"))
        .map(std::fs::read)
        .collect::<Result<Vec<_>, _>>()?;
    artifacts.sort();
    let bytes = artifacts.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let analyzer = Analyzer::resolve(AnalyzerProfile::standard(), &bytes)?;
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri(PREDICATE);
    let mut names = BTreeMap::new();
    for (at, &(id, text)) in inputs.documents.iter().enumerate() {
        let iri = format!("https://example.org/relevance/doc/{at:08}");
        names.insert(TermValue::iri(&iri), id.to_owned());
        let subject = builder.intern_iri(&iri);
        let literal = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(subject, predicate, literal, None);
    }
    let dataset = builder.freeze()?;
    let config = TextIndexConfig::new(
        vec![TermValue::iri(PREDICATE)],
        GraphSelector::Any,
        analyzer.clone(),
    )?;
    let lexical = TextIndex::from_dataset(&*dataset, &config)?;
    let character = HanCharacterIndex::from_dataset(&*dataset, &config)?;
    let mut totals: BTreeMap<(String, &str), (usize, [f64; 6])> = BTreeMap::new();
    for query in &inputs.queries {
        let grades = &inputs.judgments[query.id];
        let pool = arguments.judged_pool.then_some(grades);
        let left = rank(&lexical, query.text, &names, pool)?;
        let right = rank(character.index(), query.text, &names, pool)?;
        let fusion = fused(&left, &right)?;
        for (lane, ranking) in [("lexical", left), ("han", right), ("fusion", fusion)] {
            let values = metrics(&ranking, grades);
            for group in [query.stratum, "all"] {
                let total = totals.entry((group.to_owned(), lane)).or_default();
                total.0 += 1;
                for (sum, value) in total.1.iter_mut().zip(values) {
                    *sum += value;
                }
            }
        }
    }
    println!("stratum\tlane\tqueries\tP@3\tR@3\tF1@3\tHit@3\tMRR\tnDCG@3");
    for ((stratum, lane), (count, values)) in totals {
        println!(
            "{stratum}\t{lane}\t{count}\t{}",
            values
                .into_iter()
                .map(|value| format!("{:.6}", value / count as f64))
                .collect::<Vec<_>>()
                .join("\t")
        );
    }
    if arguments.external.is_none() {
        let mut predicted = 0;
        let mut gold = 0;
        let mut correct = 0;
        for line in rows(include_str!("../tests/relevance/segmentation.tsv")) {
            let words = line.split('|').collect::<Vec<_>>();
            let text = words.concat();
            let mut at = 0;
            let expected = words
                .iter()
                .map(|word| {
                    let range = (at, at + word.len());
                    at += word.len();
                    range
                })
                .collect::<BTreeSet<_>>();
            let actual = analyzer
                .projections(&text)?
                .lexical
                .into_iter()
                .map(|term| (term.range.start, term.range.end))
                .collect::<BTreeSet<_>>();
            gold += expected.len();
            predicted += actual.len();
            correct += expected.intersection(&actual).count();
        }
        let p = correct as f64 / predicted as f64;
        let r = correct as f64 / gold as f64;
        println!(
            "segmentation\tP={p:.6}\tR={r:.6}\tF1={:.6}\texact_words={correct}/{predicted}/{gold}",
            2.0 * p * r / (p + r)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Grade, arguments, inputs, metrics};
    use std::collections::BTreeMap;

    const DOCUMENTS: &str = "d1\tFirst document\nd2\tSecond document\n";
    const QUERIES: &str = "q1\tdomain\tFirst\n";
    const JUDGMENTS: &str = "q1 Q0 d1 3\nq1 Q0 d2 0\n";

    #[test]
    fn arguments_admit_only_complete_modes() {
        for (values, external, judged_pool) in [
            (vec!["artifacts"], false, false),
            (vec!["artifacts", "docs", "queries", "qrels"], true, false),
            (
                vec!["artifacts", "docs", "queries", "qrels", "judged-pool"],
                true,
                true,
            ),
        ] {
            let values = values.into_iter().map(str::to_owned).collect::<Vec<_>>();
            let parsed = arguments(&values).unwrap();
            assert_eq!(parsed.artifacts, "artifacts");
            assert_eq!(parsed.external.is_some(), external);
            assert_eq!(parsed.judged_pool, judged_pool);
        }
        for values in [
            vec![],
            vec!["artifacts", "docs"],
            vec!["artifacts", "docs", "queries"],
            vec!["artifacts", "docs", "queries", "qrels", "unknown-mode"],
            vec![
                "artifacts",
                "docs",
                "queries",
                "qrels",
                "judged-pool",
                "extra",
            ],
        ] {
            let values = values.into_iter().map(str::to_owned).collect::<Vec<_>>();
            assert!(arguments(&values).is_err(), "{values:?}");
        }
    }

    #[test]
    fn duplicate_identities_and_contradictory_judgments_are_refused() {
        assert!(
            inputs(
                &format!("{DOCUMENTS}d1\tFirst document\n"),
                QUERIES,
                JUDGMENTS
            )
            .is_err()
        );
        assert!(inputs(DOCUMENTS, "q1\tFirst\nq1\tSecond\n", JUDGMENTS).is_err());
        assert!(inputs(DOCUMENTS, QUERIES, "q1 Q0 d1 3\nq1 Q0 d1 0\n").is_err());
        let single = inputs(DOCUMENTS, QUERIES, JUDGMENTS).unwrap();
        let repeated = inputs(DOCUMENTS, QUERIES, &format!("{JUDGMENTS}q1 0 d1 3\n")).unwrap();
        assert_eq!(single.judgments, repeated.judgments);
    }

    #[test]
    fn every_reference_resolves_before_scoring() {
        for judgments in [
            "q2 Q0 d1 3\n",
            "q1 Q0 d3 3\n",
            "q1 Q1 d1 3\n",
            "q1 Q0 d1\n",
            "q1 Q0 d1 3 extra\n",
            "",
        ] {
            assert!(
                inputs(DOCUMENTS, QUERIES, judgments).is_err(),
                "{judgments:?}"
            );
        }
        assert!(inputs(DOCUMENTS, "q1\tFirst\nq2\tSecond\n", JUDGMENTS).is_err());
        let zero = inputs(DOCUMENTS, QUERIES, "q1 Q0 d1 0\n").unwrap();
        assert_eq!(zero.judgments["q1"]["d1"], Grade::None);
    }

    #[test]
    fn malformed_corpora_and_aggregate_name_collision_are_refused() {
        for documents in ["", "d1", "\tFirst", "d 1\tFirst"] {
            assert!(
                inputs(documents, QUERIES, JUDGMENTS).is_err(),
                "{documents:?}"
            );
        }
        for queries in [
            "",
            "q1",
            "\tFirst",
            "q 1\tFirst",
            "q1\t",
            "q1\t \tFirst",
            "q1\tall\tFirst",
            "q1\tdomain\t \tunexpected",
        ] {
            assert!(
                inputs(DOCUMENTS, queries, JUDGMENTS).is_err(),
                "{queries:?}"
            );
        }
        let parsed = inputs(DOCUMENTS, "q1\tFirst\n", JUDGMENTS).unwrap();
        assert_eq!(parsed.queries[0].stratum, "public");
        // An empty RDF literal is a valid candidate; it need not produce tokens.
        assert!(inputs("d1\t\nd2\tSecond", QUERIES, JUDGMENTS).is_ok());
    }

    #[test]
    fn externally_supplied_grades_cannot_overflow_gain() {
        for value in [
            "-1",
            "4",
            "4294967295",
            "9999999999999999999999999999",
            "1.5",
        ] {
            assert!(Grade::parse(value).is_err(), "{value}");
            assert!(inputs(DOCUMENTS, QUERIES, &format!("q1 Q0 d1 {value}\n")).is_err());
        }
        for (value, expected) in [("0", 0.0), ("1", 1.0), ("2", 3.0), ("3", 7.0)] {
            assert_eq!(Grade::parse(value).unwrap().gain(), expected);
        }
    }

    #[test]
    #[allow(clippy::float_arithmetic)] // Hand-calculated metric oracle, not production scoring.
    fn bounded_grades_preserve_the_metric_law() {
        let grades = BTreeMap::from([
            ("direct".to_owned(), Grade::Direct),
            ("irrelevant".to_owned(), Grade::None),
            ("supporting".to_owned(), Grade::Supporting),
        ]);
        let ranking = ["direct", "irrelevant", "supporting"].map(str::to_owned);
        let actual = metrics(&ranking, &grades);
        let expected = [
            2.0 / 3.0,
            1.0,
            0.8,
            1.0,
            1.0,
            8.5 / (7.0 + 3.0 / 3.0_f64.log2()),
        ];
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-12);
        }
        assert_eq!(metrics(&[], &grades), [0.0; 6]);
    }
}
