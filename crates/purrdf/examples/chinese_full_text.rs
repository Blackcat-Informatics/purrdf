// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compare grapheme fallback with a caller-supplied Chinese lexicon.
//!
//! The full-text index and its search relation are Rust-host facilities.

use std::sync::Arc;

use purrdf::sparql::{ExtensionEnv, NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions};
use purrdf::text::{
    Analyzer, AnalyzerProfile, GraphSelector, Segmentation, TextIndex, TextIndexConfig,
    TextSearchRelation, segment::Dictionary,
};
use purrdf::{SparqlRequest, SparqlResult, TermValue, parse_dataset};

const PHRASE: &str = "南京市长江大桥";
const WORDS: [&str; 5] = ["南京市", "长江大桥", "附近", "长江", "北岸"];
const CORPUS: &str = r#"
@prefix ex: <https://example.org/> .
ex:bridge ex:note "南京市长江大桥"@zh .
ex:nearby ex:note "南京市长江大桥附近"@zh .
ex:north-bank ex:note "南京市长江北岸"@zh .
"#;
const QUERY: &str = r#"
SELECT ?doc ?rank ?matched WHERE {
    ?doc <https://example.org/search> ( "南京市长江大桥" ?score ?rank ?language ?matched )
}
ORDER BY ?rank
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dataset = parse_dataset(CORPUS.as_bytes(), "text/turtle", None)?;
    let lexicon = Dictionary::new(WORDS.map(str::to_owned))?;
    let dictionary = Analyzer::with_profile(
        AnalyzerProfile::standard().with_segmentation(Segmentation::Dictionary(Arc::new(lexicon))),
    )?;
    let graphemes: &[(u32, &str)] = &[
        (0, "南"),
        (1, "京"),
        (2, "市"),
        (3, "长"),
        (4, "江"),
        (5, "大"),
        (6, "桥"),
    ];
    let words: &[(u32, &str)] = &[(0, "南京市"), (1, "长江大桥")];
    for (name, analyzer, expected_tokens, matched) in [
        (
            "grapheme fallback",
            Analyzer::empty_lexicon(),
            graphemes,
            [7, 7, 5],
        ),
        ("caller lexicon", dictionary, words, [2, 2, 1]),
    ] {
        let mut tokens = Vec::new();
        analyzer.analyze(PHRASE, &mut tokens)?;
        let positioned: Vec<_> = tokens
            .iter()
            .map(|t| (t.position, t.text.as_ref()))
            .collect();
        assert_eq!(positioned, expected_tokens);
        println!("{name}: {positioned:?}");
        let config = TextIndexConfig::new(
            vec![TermValue::iri("https://example.org/note")],
            GraphSelector::Default,
            analyzer,
        )?;
        let index = TextIndex::from_dataset(&*dataset, &config)?;
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(
            "https://example.org/search".to_owned(),
            Arc::new(TextSearchRelation::new(Arc::new(index))),
        );
        let env = ExtensionEnv::over_relations(registry)?;
        let result = NativeSparqlEngine::new().query_with_options_view(
            &*dataset,
            SparqlRequest {
                query: QUERY,
                base_iri: None,
                substitutions: &[],
            },
            &QueryOptions::new().with_env(&env),
        )?;
        let SparqlResult::Solutions {
            variables, rows, ..
        } = result
        else {
            return Err("expected ranked SELECT rows".into());
        };
        assert_eq!(variables, ["doc", "rank", "matched"]);
        let expected_rows = ["bridge", "nearby", "north-bank"]
            .map(|document| TermValue::iri(format!("https://example.org/{document}")));
        let integer = purrdf::xsd::XsdDatatype::Integer.iri();
        let expected_rows: Vec<_> = expected_rows
            .into_iter()
            .zip(matched)
            .enumerate()
            .map(|(rank, (document, count))| {
                vec![
                    Some(document),
                    Some(TermValue::typed_literal((rank + 1).to_string(), integer)),
                    Some(TermValue::typed_literal(count.to_string(), integer)),
                ]
            })
            .collect();
        assert_eq!(rows, expected_rows);
        for row in rows {
            let [
                Some(TermValue::Iri(document)),
                Some(TermValue::Literal {
                    lexical_form: rank, ..
                }),
                Some(TermValue::Literal {
                    lexical_form: matched,
                    ..
                }),
            ] = row.as_slice()
            else {
                return Err("expected a document IRI, integer rank and match count".into());
            };
            println!("{rank}: {document} ({matched} matched terms)");
        }
    }
    Ok(())
}
