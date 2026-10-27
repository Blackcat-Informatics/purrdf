// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One public acceptance corpus shared by the native and portable harness.

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_algebra::GraphPattern;
use purrdf_sparql_eval::agg_fn::WorkspaceAccumulator;
use purrdf_sparql_eval::shapes::spans::{SpanGeometry, longest_first_indices};
use purrdf_sparql_eval::shapes::{
    ShapeEstimate, ShapeOffer, ShapeOperator, ShapeProvider, ShapeRegistry, ShapeRequest,
    ShapeRuntime,
};
use purrdf_sparql_eval::{
    EvalError, EvalOptions, ExtensionEnv, InternedOutcome, InternedRequest, NativeSparqlEngine,
    PropertyFunctionRegistry, QueryOptions, WorkspaceCapability,
};
use purrdf_sparql_results::{ResultProvenance, to_json};
use purrdf_testkit::prop::prelude::*;
use purrdf_text::dictionary::{DictionaryCache, DictionaryEntry};
use purrdf_text::{
    Analyzer, GraphSelector, PhraseOccurrenceRelation, TermOccurrenceRelation, TextIndex,
    TextIndexConfig,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) const NOTE: &str = "https://example.org/note";
pub(crate) const OTHER: &str = "https://example.org/other";
pub(crate) const PHRASE: &str = "https://example.org/phrase";
pub(crate) const OCCURS: &str = "https://example.org/occurs";
const POS: &str = "https://example.org/position";
const SEQUENCE: &str = "https://example.org/sequence";
const TIME: &str = "https://example.org/time";

/// Execute the original property-function seam with either physical corpus law.
fn corpus_query<D, P>(
    dataset: &D,
    index: Arc<TextIndex<P>>,
    query: &str,
) -> purrdf_sparql_eval::RetainedSparqlResult
where
    D: purrdf_core::FallibleDatasetView + Sync,
    P: purrdf_text::CorpusProfile,
{
    let mut relations = PropertyFunctionRegistry::new();
    relations.register(OCCURS, Arc::new(TermOccurrenceRelation::new(index.clone())));
    relations.register(
        PHRASE,
        Arc::new(PhraseOccurrenceRelation::new(index.clone())),
    );
    relations.register(
        "https://example.org/search",
        Arc::new(purrdf_text::TextSearchRelation::new(index)),
    );
    let env = ExtensionEnv::over_relations(relations).unwrap();
    NativeSparqlEngine::new()
        .query_fallible_view(dataset, request(query), QueryOptions::new().with_env(&env))
        .expect("the original physical corpus query completes")
        .result
}

pub(crate) fn graph_free_corpus_keys_ranking_and_query() {
    use purrdf_core::ir::TripleDataset;
    use purrdf_text::{Document, GraphFree, PartitionFilter, PartitionKey, select};

    let fixture = Fixture::text(&[
        ("urn:first".into(), "river river bank".into(), None, false),
        ("urn:second".into(), "river valley".into(), None, false),
        ("urn:third".into(), "bank meadow".into(), None, true),
    ]);
    let queries = [
        "SELECT ?s ?score ?rank ?lang ?matched WHERE { ?s <https://example.org/search> (\"river\" ?score ?rank ?lang ?matched) } ORDER BY ?rank ?s",
        "SELECT ?s ?lang ?position WHERE { ?s <https://example.org/occurs> (\"river\" ?lang ?position) } ORDER BY ?s ?position",
        "SELECT ?s ?lang ?start ?end WHERE { ?s <https://example.org/phrase> (\"river bank\" ?lang ?start ?end) } ORDER BY ?s ?start ?end",
        "SELECT ?s ?rank WHERE { ?s <https://example.org/search> (\"river\" ?score ?rank ?lang ?matched) FILTER NOT EXISTS { ?s <https://example.org/other> ?excluded } } ORDER BY ?rank ?s",
    ];
    let expected =
        queries.map(|query| corpus_query(&*fixture.dataset, fixture.index.clone(), query));
    let (_, phrase_rows) = expected[2].solutions().unwrap();
    assert_eq!(
        phrase_rows,
        vec![vec![
            Some(TermValue::iri("urn:first")),
            Some(TermValue::simple_literal("")),
            Some(TermValue::typed_literal(
                "1",
                purrdf_core::datatype::XSD_INTEGER
            )),
            Some(TermValue::typed_literal(
                "3",
                purrdf_core::datatype::XSD_INTEGER
            )),
        ]]
    );
    let Fixture {
        dataset,
        index: quad,
        ..
    } = fixture;
    let compact = TripleDataset::try_from(Arc::try_unwrap(dataset).unwrap()).unwrap();
    let read = compact.read().unwrap();
    let config = TextIndexConfig::<GraphFree>::for_triples(
        vec![TermValue::iri(NOTE), TermValue::iri(OTHER)],
        Analyzer::empty_lexicon(),
    )
    .unwrap();
    let index = Arc::new(TextIndex::from_dataset(&read, &config).unwrap());
    purrdf_text::verify_binding(&index, &read, &config).unwrap();
    assert_eq!(
        size_of::<PartitionKey<GraphFree>>(),
        size_of::<Option<String>>()
    );
    assert_eq!(
        size_of::<Document>() - size_of::<Document<GraphFree>>(),
        size_of::<Option<TermValue>>()
    );
    assert_eq!(index.document_count(), 3);
    assert_eq!(index.partition_count(), 1);
    assert_eq!(
        index.corpus_profile(),
        "purrdf-text-corpus-subject-language-v1"
    );
    assert_eq!(quad.corpus_profile(), purrdf_text::INDEX_CORPUS_PROFILE_ID);
    assert_ne!(index.fingerprint(), quad.fingerprint());
    assert_ne!(index.source_fingerprint(), quad.source_fingerprint());
    assert_eq!(
        purrdf_hash::hex::encode(&index.fingerprint()),
        "a3b848138dd6e067908437f62a086b82f3ecda0104cee1b0943b41d67b9b239a",
        "the graph-free index identity"
    );
    assert_eq!(
        purrdf_hash::hex::encode(&index.source_fingerprint()),
        "b857dde328bad38b1551da367e6a4991d2b2fa86e6efe1568573b9ca774ff7ed",
        "the graph-free source identity"
    );

    let lexical_config = config.with_surface_index(false);
    let lexical = Arc::new(TextIndex::from_dataset(&read, &lexical_config).unwrap());
    purrdf_text::verify_binding(&lexical, &read, &lexical_config).unwrap();
    assert_ne!(lexical.fingerprint(), index.fingerprint());
    assert_eq!(lexical.source_fingerprint(), index.source_fingerprint());
    assert!(lexical.surface_index().is_none());
    assert!(index.surface_index().is_some());

    assert_eq!(
        select(
            &index,
            &["river".into()],
            &PartitionFilter::unconstrained(),
            None,
            None
        )
        .unwrap(),
        select(
            &quad,
            &["river".into()],
            &PartitionFilter::unconstrained(),
            None,
            None
        )
        .unwrap()
    );
    for (query, expected) in queries.into_iter().zip(expected) {
        let actual = corpus_query(&read, index.clone(), query);
        assert_eq!(actual.solutions(), expected.solutions(), "{query}");
        let without_surface = corpus_query(&read, lexical.clone(), query);
        assert_eq!(without_surface.solutions(), expected.solutions(), "{query}");
    }
}

pub(crate) fn graph_free_rdf12_partitions_and_named_refusal() {
    use purrdf_core::ir::{TripleDatasetBuilder, TripleReadError};
    use purrdf_core::{BlankScope, DatasetView};
    use purrdf_text::{GraphFree, PartitionKey, TextError};

    let mut builder = TripleDatasetBuilder::default();
    let subject = builder.intern_blank("shared", BlankScope(7)).unwrap();
    let predicate = builder.intern_iri(NOTE).unwrap();
    let object = builder.intern_iri("urn:object").unwrap();
    let quoted = builder.intern_triple(subject, predicate, object).unwrap();
    builder.push_reifier(subject, quoted).unwrap();
    for (text, language, annotation) in [
        ("river bank", "en", false),
        ("river valley", "en", true),
        ("fleuve rive", "fr", true),
    ] {
        let literal = builder
            .intern_literal(RdfLiteral::language_tagged(text, language))
            .unwrap();
        if annotation {
            builder
                .push_annotation(subject, predicate, literal)
                .unwrap();
        } else {
            builder.push_triple(subject, predicate, literal).unwrap();
        }
    }
    let dataset = builder.freeze().unwrap();
    let read = dataset.read().unwrap();
    let config = TextIndexConfig::<GraphFree>::for_triples(
        vec![TermValue::iri(NOTE)],
        Analyzer::empty_lexicon(),
    )
    .unwrap();
    let index = TextIndex::from_dataset(&read, &config).unwrap();
    assert_eq!(index.document_count(), 2);
    assert_eq!(index.partition_count(), 2);
    let english = PartitionKey::<GraphFree>::for_language(Some("en".into()));
    assert_eq!(index.document_frequency(&english, "river"), 1);
    assert_eq!(
        index.document(0).unwrap().subject(),
        &TermValue::Blank {
            label: "shared".into(),
            scope: BlankScope(7),
        }
    );
    assert_eq!(index.document(0).unwrap().length(), 4);
    purrdf_text::verify_binding(&index, &read, &config).unwrap();

    let empty = TripleDatasetBuilder::default().freeze().unwrap();
    assert!(purrdf_text::verify_binding(&index, &empty.read().unwrap(), &config).is_err());
    let named = TextIndexConfig::new(
        vec![TermValue::iri(NOTE)],
        GraphSelector::Named(TermValue::iri("urn:missing:graph")),
        Analyzer::empty_lexicon(),
    )
    .unwrap();
    for source in [&dataset, &empty] {
        let read = source.read().unwrap();
        let error = TextIndex::from_dataset(&read, &named).unwrap_err();
        let TextError::SourceRead(retained) = &error else {
            panic!("named graph refusal keeps its native source");
        };
        assert_eq!(
            std::error::Error::source(retained)
                .unwrap()
                .downcast_ref::<TripleReadError>(),
            Some(&TripleReadError::NamedGraphsUnsupported)
        );
        assert_eq!(
            read.read_error(),
            Some(TripleReadError::NamedGraphsUnsupported)
        );
        assert!(TextIndex::from_dataset(&source.read().unwrap(), &config).is_ok());
    }
    let quad = RdfDatasetBuilder::new().freeze().unwrap();
    assert_eq!(
        TextIndex::from_dataset(&*quad, &named)
            .unwrap()
            .document_count(),
        0
    );
    assert!(matches!(
        TextIndex::from_dataset(&*quad, &config),
        Err(TextError::Config(_))
    ));
}

pub(crate) struct Fixture {
    pub(crate) dataset: Arc<RdfDataset>,
    pub(crate) index: Arc<TextIndex>,
    pub(crate) env: ExtensionEnv<'static>,
}
impl Fixture {
    /// `graph` is deliberately hidden by the occurrence relation's row schema.
    pub(crate) fn text(rows: &[(String, String, Option<String>, bool)]) -> Self {
        Self::analyzed(rows, Analyzer::empty_lexicon())
    }
    fn analyzed(rows: &[(String, String, Option<String>, bool)], analyzer: Analyzer) -> Self {
        let mut builder = RdfDatasetBuilder::new();
        for (subject, text, graph, other) in rows {
            let s = builder.intern_iri(subject).unwrap();
            let p = builder
                .intern_iri(if *other { OTHER } else { NOTE })
                .unwrap();
            let o = builder.intern_literal(RdfLiteral::simple(text)).unwrap();
            let g = graph
                .as_ref()
                .map(|graph| builder.intern_iri(graph).unwrap());
            builder.push_quad(s, p, o, g).unwrap();
        }
        let dataset = builder.freeze().expect("valid original RDF fixture");
        Self::over(dataset, analyzer)
    }
    fn over(dataset: Arc<RdfDataset>, analyzer: Analyzer) -> Self {
        let config = TextIndexConfig::new(
            vec![TermValue::iri(NOTE), TermValue::iri(OTHER)],
            GraphSelector::Any,
            analyzer,
        )
        .expect("explicit predicates/analyzer");
        let index = Arc::new(
            TextIndex::from_dataset(&*dataset, &config).expect("the original analysis indexes"),
        );
        let mut relations = PropertyFunctionRegistry::new();
        relations.register(
            PHRASE.to_owned(),
            Arc::new(PhraseOccurrenceRelation::new(index.clone())),
        );
        relations.register(
            OCCURS.to_owned(),
            Arc::new(TermOccurrenceRelation::new(index.clone())),
        );
        Self {
            dataset,
            index,
            env: ExtensionEnv::over_relations(relations).expect("honest native declarations"),
        }
    }
    pub(crate) fn answer(&self, query: &str, disabled: bool) -> SparqlResult {
        engine(disabled)
            .query_with_options_view(
                &*self.dataset,
                request(query),
                QueryOptions::new().with_env(&self.env),
            )
            .unwrap_or_else(|error| panic!("query failed: {error:?}\n{query}"))
    }
    pub(crate) fn equal(&self, query: &str) -> SparqlResult {
        let literal = self.answer(query, true);
        let shaped = self.answer(query, false);
        assert_eq!(
            bytes(&shaped),
            bytes(&literal),
            "literal ordered bag/columns/lexicals\n{query}"
        );
        shaped
    }
    pub(crate) fn report(
        &self,
        query: &str,
        operator: &str,
    ) -> purrdf_sparql_eval::QueryExplanation {
        let report = engine(false)
            .explain_query_with_options(
                &self.dataset,
                query,
                None,
                QueryOptions::new().with_env(&self.env),
            )
            .expect("actual EXPLAIN");
        assert!(
            report
                .shapes()
                .iter()
                .any(|shape| shape.operator == Some(operator)),
            "the production query must select {operator}: {report:?}"
        );
        report
    }
}
pub(crate) fn engine(disabled: bool) -> NativeSparqlEngine {
    NativeSparqlEngine::new().with_eval_options(EvalOptions {
        disable_shapes: disabled,
        force_sequential: true,
        ..EvalOptions::default()
    })
}
pub(crate) fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}
pub(crate) fn bytes(result: &SparqlResult) -> Vec<u8> {
    to_json(result, &ResultProvenance::default(), None)
        .expect("real SPARQL JSON serialization")
        .bytes
}
fn rows(result: &SparqlResult) -> &[Vec<Option<TermValue>>] {
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("SELECT solutions");
    };
    rows
}
pub(crate) fn phrase_query(phrases: &[String]) -> String {
    let values = phrases
        .iter()
        .enumerate()
        .map(|(entry, phrase)| {
            format!(
                "(<https://example.org/entry/{entry}> {} UNDEF)",
                quoted(phrase)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "SELECT ?entry ?phrase ?extra ?doc ?lang ?start ?end WHERE {{ VALUES (?entry ?phrase ?extra) {{ {values} }} ?doc <{PHRASE}> (?phrase ?lang ?start ?end) }}"
    )
}
fn quoted(value: &str) -> String {
    let mut out = String::new();
    purrdf_lex::json_escape::push_string(
        &mut out,
        value,
        purrdf_lex::json_escape::JsonEscapes::ShortForms,
    );
    out
}

/// Shrinking generated cases vary dictionary contents, duplicates and token volume.
pub(crate) fn dictionary_property() {
    prop::Runner::new(Config::with_cases(32), "dictionary literal ordered bags")
        .run(
            &(1usize..9, 1usize..17, 0u64..10_000),
            |(documents, entries, seed)| {
                let words = ["lana", "audley", "stone", "quick", "Café", "STRASSE"];
                let mut random = seed;
                let mut draw = || {
                    let draw = purrdf_hash::mix::splitmix64_next(&mut random);
                    words[(draw % words.len() as u64) as usize]
                };
                let corpus: Vec<_> = (0..documents)
                    .map(|doc| {
                        (
                            format!("https://example.org/doc/{doc}"),
                            (0..11).map(|_| draw()).collect::<Vec<_>>().join(" "),
                            None,
                            false,
                        )
                    })
                    .collect();
                let phrases: Vec<_> = (0..entries)
                    .map(|entry| {
                        (0..=(entry % 3))
                            .map(|_| draw())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect();
                let fixture = Fixture::text(&corpus);
                let query = phrase_query(&phrases);
                prop_assert_eq!(
                    bytes(&fixture.answer(&query, false)),
                    bytes(&fixture.answer(&query, true))
                );
                let report = fixture.report(&query, "dictionary-match");
                let visited: u64 = report
                    .shapes()
                    .iter()
                    .filter(|shape| shape.operator == Some("dictionary-match"))
                    .map(|shape| shape.work.source_visits)
                    .sum();
                prop_assert_eq!(
                    visited,
                    fixture
                        .index
                        .documents()
                        .enumerate()
                        .map(|(ordinal, _)| fixture
                            .index
                            .token_occurrences(u32::try_from(ordinal).unwrap())
                            .expect("stream")
                            .len() as u64)
                        .sum()
                );
                Ok(())
            },
        )
        .unwrap_or_else(|error| panic!("{error}"));
}

/// The existing token relation, not only the new phrase adapter, selects the AC.
pub(crate) fn expanded_dictionary_and_union() {
    let fixture = Fixture::text(&[(
        "https://example.org/doc/a".into(),
        "lana audley lana audley".into(),
        None,
        false,
    )]);
    let body = format!(
        "VALUES (?entry ?phrase ?a ?b) {{ (<https://example.org/a> \"lana audley\" \"lana\" \"audley\") (<https://example.org/b> \"LANA AUDLEY\" \"LANA\" \"AUDLEY\") }} ?doc <{OCCURS}> (?a ?lang ?start) . ?doc <{OCCURS}> (?b ?lang ?finish) . FILTER(?finish = ?start + 1)"
    );
    let query = format!("SELECT * WHERE {{ {body} }}");
    assert_eq!(rows(&fixture.equal(&query)).len(), 4);
    fixture.report(&query, "dictionary-match");
    let union = format!("SELECT * WHERE {{ {{ {body} }} UNION {{ {body} }} }}");
    assert_eq!(rows(&fixture.equal(&union)).len(), 8);
    let report = fixture.report(&union, "dictionary-match");
    assert_eq!(
        report
            .shapes()
            .iter()
            .filter(|shape| shape.operator == Some("dictionary-match"))
            .map(|shape| shape.work.source_visits)
            .sum::<u64>(),
        4,
        "one stream visit for the whole UNION dictionary"
    );
    let single = format!(
        "SELECT * WHERE {{ VALUES (?entry ?phrase ?token) {{ (<https://example.org/a> \"lana\" \"lana\") }} ?doc <{OCCURS}> (?token ?lang ?start) }}"
    );
    assert_eq!(rows(&fixture.equal(&single)).len(), 2);
    fixture.report(&single, "dictionary-match");
    // Bound positions and repeated arguments remain the original unifier's law.
    let bounded = format!(
        "SELECT * WHERE {{ VALUES ?phrase {{ \"lana audley\" }} ?doc <{PHRASE}> (?phrase ?lang 2 ?end) }}"
    );
    assert_eq!(rows(&fixture.equal(&bounded)).len(), 1);
}

/// A real 534-name request and an above-4096 dictionary use the same compiler.
pub(crate) fn dictionary_cache_and_size() {
    let fixture = Fixture::text(&[(
        "https://example.org/doc/a".into(),
        "lana audley".into(),
        None,
        false,
    )]);
    let capability = WorkspaceCapability::resident();
    let cache = DictionaryCache::default();
    for count in [0, 1, 534, 4097] {
        let identities: Vec<_> = (0..count)
            .map(|entry| TermValue::iri(format!("https://example.org/e/{entry}")))
            .collect();
        let phrases: Vec<_> = (0..count)
            .map(|entry| {
                if entry == 0 {
                    "lana audley".to_owned()
                } else {
                    format!("absent name{entry}")
                }
            })
            .collect();
        let entries: Vec<_> = identities
            .iter()
            .zip(&phrases)
            .map(|(identity, phrase)| DictionaryEntry {
                identity,
                phrase,
                payload: &[],
            })
            .collect();
        let first = cache
            .get(&fixture.index, &entries, &capability)
            .expect("all authored rows compile");
        assert_eq!(first.len(), count);
        let second = cache
            .get(&fixture.index, &entries, &capability)
            .expect("repeat native key");
        if count != 0 {
            assert!(
                std::ptr::eq(first.entry(0).unwrap().0, second.entry(0).unwrap().0),
                "cache really shares the original compilation"
            );
        }
        let matches = first
            .scan(fixture.index.clone(), None, &capability)
            .expect("one scan");
        assert_eq!(matches.rows().len(), usize::from(count != 0));
        assert_eq!(matches.work().source_visits, if count == 0 { 0 } else { 2 });
        cache.clear();
        assert_eq!(
            matches.rows().len(),
            usize::from(count != 0),
            "eviction cannot detach live matches"
        );
        if count == 534 || count == 4097 {
            let query = phrase_query(&phrases);
            assert_eq!(
                rows(&fixture.equal(&query)).len(),
                1,
                "ordinary large VALUES consumer"
            );
            fixture.report(&query, "dictionary-match");
        }
    }
    let identity = TermValue::iri("https://example.org/reused");
    let a = [DictionaryEntry {
        identity: &identity,
        phrase: "lana",
        payload: &[],
    }];
    let b = [DictionaryEntry {
        identity: &identity,
        phrase: "audley",
        payload: &[],
    }];
    let first = cache.get(&fixture.index, &a, &capability).unwrap();
    let changed = cache.get(&fixture.index, &b, &capability).unwrap();
    assert!(
        !std::ptr::eq(first.entry(0).unwrap().0, changed.entry(0).unwrap().0),
        "authored dictionary change invalidates compilation"
    );
    let fresh = Fixture::text(&[(
        "https://example.org/doc/new".into(),
        "stone lana".into(),
        None,
        false,
    )]);
    let rebound = cache.get(&fresh.index, &b, &capability).unwrap();
    assert!(
        std::ptr::eq(changed.entry(0).unwrap().0, rebound.entry(0).unwrap().0),
        "same analyzer reuses index-independent automaton"
    );
    assert_eq!(
        rebound
            .scan(fresh.index, None, &capability)
            .unwrap()
            .rows()
            .len(),
        0,
        "new index cannot reuse old occurrence IDs"
    );
}

pub(crate) fn dictionary_projection_and_aliases() {
    // Six physical tokens form eight independent positional derivations.
    // The declared bound must cover the original bag, not only the source size.
    let aliases = Fixture::text(&[
        (
            "https://example.org/aliased".into(),
            "lana audley stone".into(),
            None,
            false,
        ),
        (
            "https://example.org/aliased".into(),
            "lana audley stone".into(),
            Some("https://example.org/alias-graph".into()),
            false,
        ),
    ]);
    let aliased_query = phrase_query(&["lana audley stone".into()]);
    let expected_aliases = aliases.equal(&aliased_query);
    assert_eq!(rows(&expected_aliases).len(), 8);
    let relation = PhraseOccurrenceRelation::new(aliases.index.clone());
    assert!(
        purrdf_sparql_eval::PropertyFunction::rows_per_invocation(
            &relation,
            purrdf_core::binding_pattern::BindingPattern::from_code("fbfff")
        ) >= 8
    );
    for disabled in [false, true] {
        let answer = engine(disabled)
            .query_governed_view(
                &*aliases.dataset,
                request(&aliased_query),
                QueryOptions::new().with_env(&aliases.env),
                &purrdf_sparql_eval::QueryGovernors::METERED,
            )
            .unwrap();
        let purrdf_sparql_eval::GovernedOutcome::Complete { result: answer, .. } = answer else {
            panic!("finite aliased bag fits the original metered governors");
        };
        assert_eq!(bytes(&answer), bytes(&expected_aliases));
    }
    let fixture = Fixture::text(&[
        (
            "https://example.org/doc/a".into(),
            "lana audley".into(),
            None,
            false,
        ),
        (
            "https://example.org/doc/a".into(),
            "lana stone".into(),
            Some("https://example.org/g".into()),
            false,
        ),
        (
            "https://example.org/doc/b".into(),
            "lana".into(),
            None,
            false,
        ),
        (
            "https://example.org/doc/b".into(),
            "audley".into(),
            None,
            true,
        ),
    ]);
    let query = phrase_query(&[
        "lana audley".into(),
        "LÁNA AUDLEY".into(),
        "stone".into(),
        "not present".into(),
    ]);
    let shaped = fixture.equal(&query);
    assert_eq!(
        rows(&shaped).len(),
        7,
        "hidden graph aliases preserve each literal positional derivation"
    );
    fixture.report(&query, "dictionary-match");
    // Predicate filtering retains gaps instead of inventing token adjacency.
    let relation = PhraseOccurrenceRelation::new(fixture.index.clone())
        .on_predicate(&TermValue::iri(NOTE))
        .unwrap();
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(PHRASE, Arc::new(relation));
    let scoped = Fixture {
        env: ExtensionEnv::over_relations(registry).unwrap(),
        ..fixture
    };
    let result = scoped.equal(&phrase_query(&["lana audley".into()]));
    assert_eq!(rows(&result).len(), 2);
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder
        .intern_iri("https://example.org/multilingual")
        .unwrap();
    let predicate = builder.intern_iri(NOTE).unwrap();
    for (language, direction, text) in [
        ("EN", None, "lana audley"),
        ("fr", None, "lana stone"),
        (
            "ar",
            Some(purrdf_core::RdfTextDirection::Rtl),
            "lana audley",
        ),
        ("zh", None, "中文 lana"),
    ] {
        let mut literal = RdfLiteral::language_tagged(text, language);
        literal.direction = direction;
        let object = builder.intern_literal(literal).unwrap();
        builder.push_quad(subject, predicate, object, None).unwrap();
    }
    let tagged = Fixture::over(builder.freeze().unwrap(), Analyzer::empty_lexicon());
    let query = phrase_query(&["lana audley".into(), "stone".into(), "中文".into()]);
    let rows = tagged.equal(&query);
    assert_ne!(
        rows.solutions().unwrap().1,
        &[] as &[Vec<Option<TermValue>>]
    );
    tagged.report(&query, "dictionary-match");
}

/// Independent quadratic geometric reference, not the production selector.
pub(crate) fn longest_first_reference() {
    let mut spans = vec![
        SpanGeometry {
            domain: 0,
            start: 0,
            end: 4,
        },
        SpanGeometry {
            domain: 0,
            start: 2,
            end: 6,
        },
        SpanGeometry {
            domain: 0,
            start: 0,
            end: 4,
        },
        SpanGeometry {
            domain: 0,
            start: 4,
            end: 7,
        },
        SpanGeometry {
            domain: 1,
            start: 2,
            end: 6,
        },
    ];
    for seed in 0..24 {
        for index in 0..12 {
            let start = (seed * 7 + index * 3) % 13;
            spans.push(SpanGeometry {
                domain: index % 2,
                start: start as u64,
                end: (start + index % 5) as u64,
            });
        }
        let mut order: Vec<_> = (0..spans.len()).collect();
        order.sort_by_key(|&index| {
            let span = spans[index];
            (
                span.domain,
                std::cmp::Reverse(span.end - span.start),
                span.start,
                span.end,
            )
        });
        let mut reference = Vec::new();
        for index in order {
            let span = spans[index];
            if !reference.iter().any(|&previous| {
                let prior: SpanGeometry = spans[previous];
                prior.domain == span.domain
                    && prior != span
                    && prior.start < prior.end
                    && span.start < span.end
                    && prior.start.max(span.start) < prior.end.min(span.end)
            }) {
                reference.push(index);
            }
        }
        reference.sort_unstable();
        assert_eq!(
            &*longest_first_indices(&spans, &WorkspaceCapability::resident()).unwrap(),
            &reference
        );
    }
    assert!(
        longest_first_indices(
            &[SpanGeometry {
                domain: 0,
                start: 2,
                end: 1
            }],
            &WorkspaceCapability::resident()
        )
        .is_err()
    );
    let fixture = Fixture::text(&[(
        "https://example.org/doc/a".into(),
        "lana audley stone quick".into(),
        None,
        false,
    )]);
    let query = phrase_query(&[
        "lana audley stone".into(),
        "audley stone quick".into(),
        "quick".into(),
        "lana audley stone".into(),
    ]);
    engine(false)
        .query_interned_view(
            &*fixture.dataset,
            InternedRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::new().with_env(&fixture.env),
            |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else {
                    panic!("SELECT");
                };
                let selected = rows
                    .longest_first_spans(&["doc", "lang"], "start", "end")
                    .unwrap();
                assert_eq!(
                    selected.ordinals(),
                    &[0, 2, 3],
                    "equal spans retain authored duplicates and adjacent quick survives"
                );
                for (&index, row) in selected.ordinals().iter().zip(selected.rows()) {
                    assert!(
                        std::ptr::eq(row, &raw const rows.rows()[index]),
                        "no detached row/evidence copy"
                    );
                }
            },
        )
        .unwrap();
    // Strictly longer containing spans leave both crossing length-three spans.
    // Greedy selection retains only the earlier one. These are different laws.
    assert_eq!(rows(&fixture.equal(&query)).len(), 4);
    let other = ["lana audley stone", "audley stone quick", "quick"]
        .iter()
        .map(|phrase| quoted(phrase))
        .collect::<Vec<_>>()
        .join(" ");
    let body = query
        .strip_suffix(" }")
        .expect("the original query's WHERE closing brace");
    let containment = format!(
        "{body} FILTER NOT EXISTS {{ VALUES ?other {{ {other} }} ?doc <{PHRASE}> (?other ?lang ?s2 ?e2) FILTER((?e2-?s2)>(?end-?start) && ?s2<=?start && ?e2>=?end) }} }}"
    );
    let result = fixture.equal(&containment);
    assert_eq!(
        rows(&result).len(),
        3,
        "both crossing spans and the first span's duplicate survive strict containment"
    );
    assert!(rows(&result).iter().all(|row| !matches!(&row[1], Some(TermValue::Literal { lexical_form, .. }) if lexical_form == "quick")), "containment removes quick, while explicit greedy selection preserves that adjacent span");
}

pub(crate) fn positions(values: &[String]) -> Arc<RdfDataset> {
    positions_with_breakers(values, false)
}
fn positions_with_breakers(values: &[String], breakers: bool) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for (at, value) in values.iter().enumerate() {
        let s = builder
            .intern_iri(&format!("https://example.org/token/{at}"))
            .unwrap();
        let p = builder.intern_iri(POS).unwrap();
        let o = builder
            .intern_literal(RdfLiteral::typed(value, purrdf_xsd::datatype::XSD_INTEGER))
            .unwrap();
        builder.push_quad(s, p, o, None).unwrap();
        let p = builder.intern_iri(SEQUENCE).unwrap();
        let o = builder
            .intern_iri(&format!("https://example.org/sequence/{}", at % 2))
            .unwrap();
        builder.push_quad(s, p, o, None).unwrap();
        if breakers && at % 3 == 1 {
            let p = builder.intern_iri("https://example.org/breaker").unwrap();
            let o = builder
                .intern_literal(RdfLiteral::typed("true", purrdf_xsd::datatype::XSD_BOOLEAN))
                .unwrap();
            builder.push_quad(s, p, o, None).unwrap();
        }
    }
    builder.freeze().unwrap()
}

pub(crate) fn window_barrier_property() {
    prop::Runner::new(Config::with_cases(24), "retained original between barrier")
        .run(&(1usize..15, 0u32..8, any::<bool>()), |(count, width, breakers)| {
            let values: Vec<_> = (0..count).map(|at| (at / 2).to_string()).collect();
            let dataset = positions_with_breakers(&values, breakers);
            let nested = if breakers { "&& NOT EXISTS { ?c <https://example.org/breaker> true }" } else { "" };
            let query = window_query(&width.to_string()).replace(
                &format!("ABS(?j - ?i) <= {width}"),
                &format!("ABS(?j - ?i) <= {width} && NOT EXISTS {{ ?c <{SEQUENCE}> ?seq ; <{POS}> ?k . FILTER(?k > IF(?i < ?j, ?i, ?j) && ?k < IF(?i < ?j, ?j, ?i) {nested}) }}"),
            );
            let literal = engine(true).query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY).unwrap();
            let shaped = engine(false).query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY).unwrap();
            prop_assert_eq!(bytes(&shaped), bytes(&literal));
            let report = engine(false).explain_query(&dataset, &query, None).unwrap();
            prop_assert!(report.shapes().iter().any(|shape| shape.operator == Some("ordered-window")));
            Ok(())
        }).unwrap_or_else(|error| panic!("{error}"));
}

pub(crate) fn dictionary_analyzer_identity() {
    use purrdf_text::{AccentFold, AnalyzerProfile, Segmentation, Stemming};
    let corpus = [(
        "https://example.org/doc/a".into(),
        "sîth STRASSE running relational 中文".into(),
        None,
        false,
    )];
    let cache = DictionaryCache::default();
    let identity = TermValue::iri("https://example.org/entry");
    let entries = [DictionaryEntry {
        identity: &identity,
        phrase: "sith",
        payload: &[],
    }];
    let capability = WorkspaceCapability::resident();
    let mut previous = None;
    for accent in [AccentFold::Preserve, AccentFold::LatinGreekCyrillic] {
        for stemming in [Stemming::None, Stemming::English] {
            let analyzer = Analyzer::with_profile(
                AnalyzerProfile::new(256)
                    .unwrap()
                    .with_segmentation(Segmentation::EmptyLexicon)
                    .with_accent_fold(accent)
                    .with_stemming(stemming),
            )
            .unwrap();
            let fixture = Fixture::analyzed(&corpus, analyzer);
            let compiled = cache.get(&fixture.index, &entries, &capability).unwrap();
            if let Some(prior) = previous.take() {
                let prior: purrdf_text::dictionary::Dictionary = prior;
                assert!(
                    !std::ptr::eq(prior.entry(0).unwrap().0, compiled.entry(0).unwrap().0),
                    "each changed analyzer identity invalidates the same cache"
                );
            }
            previous = Some(compiled);
            let query = phrase_query(&[
                "sith".into(),
                "STRASSE".into(),
                "running relational".into(),
                "中文".into(),
            ]);
            fixture.equal(&query);
            fixture.report(&query, "dictionary-match");
        }
    }
}
pub(crate) fn window_query(bound: &str) -> String {
    format!(
        "SELECT ?a ?b ?i ?j WHERE {{ ?a <{SEQUENCE}> ?seq ; <{POS}> ?i . ?b <{SEQUENCE}> ?seq ; <{POS}> ?j . FILTER(ABS(?j - ?i) <= {bound}) }}"
    )
}
pub(crate) fn window_property() {
    // Four times the input must not materialize sixteen times the subject
    // pairs before their unique native ordinal cells become bound.
    let mut previous = None;
    for count in [64usize, 256] {
        let dataset = positions(
            &(0..count)
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        );
        let query = window_query("0");
        let literal = engine(true)
            .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
            .unwrap();
        let allocation = purrdf_alloc_probe::CurrentThreadWindow::open();
        let shaped = engine(false)
            .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
            .unwrap();
        let measured = allocation.close();
        assert_eq!(bytes(&shaped), bytes(&literal));
        assert_eq!(shaped.solutions().unwrap().1.len(), count);
        let report = engine(false).explain_query(&dataset, &query, None).unwrap();
        let work = report
            .shapes()
            .iter()
            .find(|shape| shape.operator == Some("ordered-window"))
            .expect("native ordered window")
            .work;
        assert_eq!(work.source_visits, (count * 4) as u64);
        if let Some((smaller, transitions)) = previous {
            assert!(
                measured.allocations < smaller * 8,
                "window access precedes subject fanout"
            );
            assert!(
                work.transitions < transitions * 8,
                "window transitions scale before fanout"
            );
        }
        previous = Some((measured.allocations, work.transitions));
    }
    let mut builder = RdfDatasetBuilder::new();
    let sequence = builder
        .intern_iri("https://example.org/shared-sequence")
        .unwrap();
    for subject in ["https://example.org/multi-a", "https://example.org/multi-b"] {
        let subject = builder.intern_iri(subject).unwrap();
        let predicate = builder.intern_iri(SEQUENCE).unwrap();
        builder
            .push_quad(subject, predicate, sequence, None)
            .unwrap();
        for lexical in ["0", "1", "invalid"] {
            let predicate = builder.intern_iri(POS).unwrap();
            let value = builder
                .intern_literal(RdfLiteral::typed(
                    lexical,
                    purrdf_xsd::datatype::XSD_INTEGER,
                ))
                .unwrap();
            builder.push_quad(subject, predicate, value, None).unwrap();
        }
    }
    let ambiguous = builder.freeze().unwrap();
    for width in ["0", "1"] {
        let query = window_query(width);
        let literal = engine(true)
            .query_with_options_view(&*ambiguous, request(&query), QueryOptions::EMPTY)
            .unwrap();
        let shaped = engine(false)
            .query_with_options_view(&*ambiguous, request(&query), QueryOptions::EMPTY)
            .unwrap();
        assert_eq!(
            bytes(&shaped),
            bytes(&literal),
            "ambiguous ordinals retain the original bag and traversal"
        );
    }
    prop::Runner::new(Config::with_cases(32), "exact ordered windows")
        .run(
            &(1usize..19, 0u32..8, 0u64..10_000),
            |(count, bound, seed)| {
                let values: Vec<_> = (0..count)
                    .map(|at| ((i128::from(seed) % 19) - at as i128 / 2).to_string())
                    .collect();
                let dataset = positions(&values);
                let query = window_query(&bound.to_string());
                let literal = engine(true)
                    .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
                    .unwrap();
                let shaped = engine(false)
                    .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
                    .unwrap();
                prop_assert_eq!(bytes(&shaped), bytes(&literal));
                let report = engine(false).explain_query(&dataset, &query, None).unwrap();
                prop_assert!(
                    report
                        .shapes()
                        .iter()
                        .any(|shape| shape.operator == Some("ordered-window"))
                );
                Ok(())
            },
        )
        .unwrap_or_else(|error| panic!("{error}"));
    let dataset = positions(&[
        "100000000000000000000000000000000000000000".into(),
        "100000000000000000000000000000000000000001".into(),
        "invalid".into(),
        "0".into(),
    ]);
    for predicate in [
        "ABS(?j-?i)<=0",
        "ABS(?j-?i)<1",
        "?j-?i<=-1",
        "?i-?j>=1",
        "ABS(?j-?i)<=100000000000000000000000000000000000000001",
    ] {
        let query = window_query("2").replace("ABS(?j - ?i) <= 2", predicate);
        assert_eq!(
            bytes(
                &engine(false)
                    .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
                    .unwrap()
            ),
            bytes(
                &engine(true)
                    .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
                    .unwrap()
            )
        );
    }
}

pub(crate) fn series() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for (index, lexical) in [
        "2024-02-29T23:59:59Z",
        "2024-03-01T00:00:00+00:00",
        "2024-03-01T00:30:00+01:00",
        "-0001-12-31T24:00:00Z",
        "12024-02-29T00:00:00Z",
        "2024-03-01T00:00:00",
        "invalid",
    ]
    .iter()
    .enumerate()
    {
        let s = builder
            .intern_iri(&format!("https://example.org/event/{index}"))
            .unwrap();
        let p = builder.intern_iri(TIME).unwrap();
        let o = builder
            .intern_literal(RdfLiteral::typed(
                *lexical,
                purrdf_xsd::datatype::XSD_DATE_TIME,
            ))
            .unwrap();
        builder.push_quad(s, p, o, None).unwrap();
        let p = builder.intern_iri(POS).unwrap();
        let o = builder
            .intern_literal(RdfLiteral::typed(
                index.to_string(),
                purrdf_xsd::datatype::XSD_INTEGER,
            ))
            .unwrap();
        builder.push_quad(s, p, o, None).unwrap();
    }
    builder.freeze().unwrap()
}
pub(crate) fn typed_series_property() {
    let dataset = series();
    for target in [
        purrdf_xsd::datatype::XSD_DATE,
        purrdf_xsd::datatype::XSD_G_YEAR_MONTH,
    ] {
        for aggregate in [
            "COUNT(?s)",
            "SUM(?n)",
            "AVG(?n)",
            "MIN(?t)",
            "MAX(?t)",
            "SAMPLE(?t)",
            "GROUP_CONCAT(STR(?t);separator=\"|\")",
            "COUNT(DISTINCT ?s)",
            "FOLD(?n)",
            "FOLD(?n ORDER BY DESC(?n))",
            "FOLD(DISTINCT ?n)",
        ] {
            let query = format!(
                "SELECT ?bucket ({aggregate} AS ?value) WHERE {{ ?s <{TIME}> ?t ; <{POS}> ?n . BIND(<{target}>(?t) AS ?bucket) }} GROUP BY ?bucket"
            );
            let expected = engine(true)
                .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
                .unwrap();
            let actual = engine(false)
                .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
                .unwrap();
            assert_eq!(
                bytes(&actual),
                bytes(&expected),
                "typed key and native aggregate order: {query}"
            );
            let report = engine(false).explain_query(&dataset, &query, None).unwrap();
            assert!(
                report
                    .shapes()
                    .iter()
                    .any(|shape| shape.operator == Some("typed-bucketed-series")),
                "real GROUP selected: {report:?}"
            );
        }
    }
    prop::Runner::new(Config::with_cases(24), "typed bucket ordered bags")
        .run(&(1usize..35, 0u64..10_000), |(count, seed)| {
            let values = (0..count).map(|at| {
                let month = 1 + (seed as usize + at / 3) % 12;
                let day = 1 + at % 28;
                format!("(\"2024-{month:02}-{day:02}T12:00:00{}\"^^<{}> {})", if at % 2 == 0 { "Z" } else { "+02:00" }, purrdf_xsd::datatype::XSD_DATE_TIME_STAMP, at % 7)
            }).collect::<Vec<_>>().join(" ");
            let query = format!("SELECT ?bucket (SUM(?n) AS ?total) (SAMPLE(?n) AS ?first) (FOLD(?n) AS ?ordered) WHERE {{ VALUES (?t ?n) {{ {values} }} BIND(<{}>(?t) AS ?bucket) }} GROUP BY ?bucket", purrdf_xsd::datatype::XSD_G_YEAR_MONTH);
            let empty = RdfDatasetBuilder::new().freeze().unwrap();
            let literal = engine(true).query_with_options_view(&*empty, request(&query), QueryOptions::EMPTY).unwrap();
            let shaped = engine(false).query_with_options_view(&*empty, request(&query), QueryOptions::EMPTY).unwrap();
            prop_assert_eq!(bytes(&shaped), bytes(&literal));
            let report = engine(false).explain_query(&empty, &query, None).unwrap();
            prop_assert!(report.shapes().iter().any(|shape| shape.operator == Some("typed-bucketed-series")));
            Ok(())
        }).unwrap_or_else(|error| panic!("{error}"));
}

struct OrderedDigest;
// An inline order witness: `shift` makes concatenating partial folds exact.
// The hash of each authored term still comes from the one native hash home.
struct DigestAccumulator {
    value: u64,
    shift: u64,
}
impl purrdf_sparql_eval::AggregateAccumulator for DigestAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        let Some(TermValue::Literal { lexical_form, .. }) = args.first() else {
            return Err(EvalError::config("the fixture digest requires a literal"));
        };
        self.value = self
            .value
            .wrapping_mul(131)
            .wrapping_add(purrdf_hash::fnv::fnv1a64(lexical_form.as_bytes()));
        self.shift = self.shift.wrapping_mul(131);
        Ok(())
    }
    fn step_admitted(
        &mut self,
        args: &[TermValue],
        _: &WorkspaceCapability,
    ) -> Result<(), EvalError> {
        self.step(args)
    }
    fn combine(
        &mut self,
        other: Box<dyn purrdf_sparql_eval::AggregateAccumulator>,
    ) -> Result<(), EvalError> {
        let other = other
            .into_any()
            .downcast::<Self>()
            .map_err(|_| EvalError::config("the fixture checksum requires its original state"))?;
        self.value = self
            .value
            .wrapping_mul(other.shift)
            .wrapping_add(other.value);
        self.shift = self.shift.wrapping_mul(other.shift);
        Ok(())
    }
    fn combine_admitted(
        &mut self,
        other: Box<dyn purrdf_sparql_eval::AggregateAccumulator>,
        _: &WorkspaceCapability,
    ) -> Result<(), EvalError> {
        self.combine(other)
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }
    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(Some(TermValue::typed_literal(
            self.value.to_string(),
            purrdf_xsd::datatype::XSD_INTEGER,
        )))
    }
    fn finish_admitted(
        self: Box<Self>,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<purrdf_sparql_eval::WorkspaceTerm>, EvalError> {
        workspace
            .literal(self.value, purrdf_xsd::datatype::XSD_INTEGER)
            .map(Some)
    }
}
impl purrdf_sparql_eval::CustomAggregate for OrderedDigest {
    fn arity(&self) -> purrdf_sparql_eval::Arity {
        purrdf_sparql_eval::Arity::Exact(1)
    }
    fn volatility(&self) -> purrdf_sparql_eval::Volatility {
        purrdf_sparql_eval::Volatility::Stable
    }
    fn algebraic_class(&self) -> purrdf_sparql_eval::AlgebraicClass {
        purrdf_sparql_eval::AlgebraicClass::OrderDependent
    }
    fn state_bound(&self) -> u64 {
        0
    }
    fn init(&self, _: &[(String, TermValue)]) -> Box<dyn purrdf_sparql_eval::AggregateAccumulator> {
        Box::new(DigestAccumulator { value: 0, shift: 1 })
    }
    fn init_admitted(
        &self,
        _: &[(String, TermValue)],
        _: purrdf_xsd::exact::DivisionPolicy,
        workspace: &WorkspaceCapability,
    ) -> Result<WorkspaceAccumulator, EvalError> {
        WorkspaceAccumulator::new(DigestAccumulator { value: 0, shift: 1 }, workspace)
    }
    fn exact_numeric_cost_admitted(
        &self,
        _: &[Vec<TermValue>],
        _: &[(String, TermValue)],
        _: purrdf_xsd::exact::DivisionPolicy,
        _: &WorkspaceCapability,
    ) -> Result<purrdf_xsd::exact::Cost, EvalError> {
        Ok(purrdf_xsd::exact::Cost::ZERO)
    }
}

pub(crate) fn bucket_custom_and_parallel_order() {
    let dataset = series();
    let mut aggregates = purrdf_sparql_eval::AggregateRegistry::default();
    aggregates.register(
        "https://example.org/ordered-digest",
        Arc::new(OrderedDigest),
    );
    let env = ExtensionEnv::over_aggregates(aggregates).unwrap();
    let query = format!(
        "SELECT ?bucket (AGG(<https://example.org/ordered-digest>, ?t) AS ?order) (SUM(?n/3) AS ?rounded) (AVG(?n/3) AS ?mean) WHERE {{ ?s <{TIME}> ?t ; <{POS}> ?n . BIND(<{}>(?t) AS ?bucket) }} GROUP BY ?bucket",
        purrdf_xsd::datatype::XSD_G_YEAR_MONTH
    );
    let options = QueryOptions::new().with_env(&env);
    let reference = engine(true)
        .query_with_options_view(&*dataset, request(&query), options)
        .unwrap();
    for sequential in [false, true] {
        let native = NativeSparqlEngine::new().with_eval_options(EvalOptions {
            force_sequential: sequential,
            ..EvalOptions::default()
        });
        let found = native
            .query_with_options_view(&*dataset, request(&query), options)
            .unwrap();
        assert_eq!(
            bytes(&found),
            bytes(&reference),
            "global aggregate step and each group's rounding/order law"
        );
    }
    let report = engine(false)
        .explain_query_with_options(&dataset, &query, None, options)
        .unwrap();
    assert!(
        report
            .shapes()
            .iter()
            .any(|shape| shape.operator == Some("typed-bucketed-series"))
    );
    // Contextual LATERAL drives GROUP through its original row-ordered fold.
    let lateral = format!(
        "SELECT * WHERE {{ VALUES ?period {{ 2 3 }} LATERAL {{ SELECT ?bucket (FOLD(?n) AS ?order) (AVG(?n/3) AS ?mean) WHERE {{ ?s <{TIME}> ?t ; <{POS}> ?n FILTER(?n>=?period) BIND(<{}>(?t) AS ?bucket) }} GROUP BY ?bucket }} }}",
        purrdf_xsd::datatype::XSD_DATE
    );
    assert_eq!(
        bytes(
            &engine(false)
                .query_with_options_view(&*dataset, request(&lateral), QueryOptions::EMPTY)
                .unwrap()
        ),
        bytes(
            &engine(true)
                .query_with_options_view(&*dataset, request(&lateral), QueryOptions::EMPTY)
                .unwrap()
        )
    );
}

struct InvalidShape(u8);
impl ShapeProvider for InvalidShape {
    fn offer(
        &self,
        request: ShapeRequest<'_>,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<ShapeOffer>, EvalError> {
        if !matches!(request.pattern, GraphPattern::Values { .. }) {
            return Ok(None);
        }
        ShapeOffer::new_admitted(
            "invalid-fixture",
            Self(self.0),
            ShapeEstimate::default(),
            workspace,
        )
        .map(Some)
    }
}
impl ShapeOperator for InvalidShape {
    fn execute(&self, runtime: &mut dyn ShapeRuntime) -> Result<(), EvalError> {
        if self.0 == 0 {
            runtime.input(&[99])?;
        } else {
            let input = runtime.input(&[])?.unwrap();
            let mut bindings = purrdf_sparql_eval::AdmittedVec::new(runtime.workspace());
            bindings.push(purrdf_sparql_eval::shapes::ShapeBinding {
                variable: purrdf_sparql_algebra::Variable::new("invented"),
                value: runtime
                    .workspace()
                    .literal(1, purrdf_xsd::datatype::XSD_INTEGER)?,
            })?;
            runtime.emit_with(&[input.row(0).unwrap()], bindings)?;
        }
        Ok(())
    }
}
pub(crate) fn host_refusal_and_governor_boundary() {
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    for invalid in [0, 1] {
        let mut registry = ShapeRegistry::new();
        registry.register("invalid-fixture", Arc::new(InvalidShape(invalid)));
        let env = ExtensionEnv::over_relations(PropertyFunctionRegistry::new())
            .unwrap()
            .with_shapes(registry);
        let result = engine(false).query_with_options_view(
            &*dataset,
            request("SELECT * WHERE { VALUES ?x { 1 } }"),
            QueryOptions::new().with_env(&env),
        );
        assert!(
            result.is_err(),
            "a selected host's physical contract failure must propagate without literal retry"
        );
    }
    let fixture = Fixture::text(&[(
        "https://example.org/doc/a".into(),
        "lana audley".into(),
        None,
        false,
    )]);
    let positions = positions(&["1".into(), "2".into()]);
    for (dataset, query, options) in [
        (
            &*fixture.dataset,
            phrase_query(&["lana audley".into()]),
            QueryOptions::new().with_env(&fixture.env),
        ),
        (&*positions, window_query("2"), QueryOptions::EMPTY),
    ] {
        let outcome = engine(false)
            .query_governed_view(
                dataset,
                request(&query),
                options,
                &purrdf_sparql_eval::QueryGovernors::UNBOUNDED.with_fuel(0),
            )
            .unwrap();
        assert!(
            matches!(
                outcome,
                purrdf_sparql_eval::GovernedOutcome::BudgetExhausted(_)
            ),
            "no physical strategy may invent success after zero fuel"
        );
    }
}

pub(crate) fn dictionary_long_phrase_and_determinism() {
    let phrase = std::iter::repeat_n("lana", 1025)
        .collect::<Vec<_>>()
        .join(" ");
    let fixture = Fixture::text(&[(
        "https://example.org/doc/a".into(),
        phrase.clone(),
        None,
        false,
    )]);
    let entry = TermValue::iri("https://example.org/long-entry");
    let dictionary = DictionaryCache::default()
        .get(
            &fixture.index,
            &[DictionaryEntry {
                identity: &entry,
                phrase: &phrase,
                payload: &[],
            }],
            &WorkspaceCapability::resident(),
        )
        .unwrap();
    let matches = dictionary
        .scan(fixture.index, None, &WorkspaceCapability::resident())
        .unwrap();
    assert_eq!(matches.rows().len(), 1);
    assert_eq!((matches.rows()[0].start, matches.rows()[0].end), (0, 1025));
    assert_eq!(
        matches.work().source_visits,
        1025,
        "arbitrarily long representable phrases keep the one-pass kernel"
    );
    let corpus = [
        (
            "https://example.org/doc/b".into(),
            "lana stone".into(),
            None,
            false,
        ),
        (
            "https://example.org/doc/a".into(),
            "LANA audley".into(),
            None,
            false,
        ),
    ];
    let original = Fixture::text(&corpus);
    let reversed = Fixture::text(&corpus.into_iter().rev().collect::<Vec<_>>());
    let query = phrase_query(&["lana".into(), "lana audley".into()]);
    assert_eq!(
        bytes(&original.equal(&query)),
        bytes(&reversed.equal(&query)),
        "build order does not enter dictionary or occurrence identities"
    );
}

pub(crate) fn segmented(dataset: &RdfDataset) -> purrdf_core::SegmentedImage {
    let mut builder = purrdf_core::SegmentedBuilder::new(
        purrdf_core::SegmentedBuildLimits::new(4096, 500_000, 4096, 512, 4).unwrap(),
    )
    .unwrap();
    for quad in dataset.quads() {
        let mut values = vec![
            dataset.term_value(quad.s),
            dataset.term_value(quad.p),
            dataset.term_value(quad.o),
        ];
        if let Some(graph) = quad.g {
            values.push(dataset.term_value(graph));
        }
        let mut ids = Vec::new();
        builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
        builder
            .push_quad(purrdf_core::QuadIds {
                s: ids[0],
                p: ids[1],
                o: ids[2],
                g: quad.g.map(|_| ids[3]),
            })
            .unwrap();
    }
    builder.seal().unwrap()
}

pub(crate) fn bounded_shapes_keep_original_owners() {
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_core::{DatasetView, SegmentedReadLimits};
    const CEILING: u64 = 32_000_000;
    let fixture = Fixture::text(&[(
        "https://example.org/doc/a".into(),
        "lana audley lana audley".into(),
        None,
        false,
    )]);
    let image = segmented(&fixture.dataset);
    let query = phrase_query(&["lana audley".into(), "lana".into()]);
    let reference = fixture.answer(&query, true);
    let expected = reference.solutions().unwrap().1;
    for route in 0..4 {
        let mut relations = PropertyFunctionRegistry::new();
        let phrase = PhraseOccurrenceRelation::new(fixture.index.clone());
        phrase.retain_dictionary(false);
        relations.register(PHRASE, Arc::new(phrase));
        let env = ExtensionEnv::over_relations(relations).unwrap();
        let options = QueryOptions::new().with_env(&env);
        let native = engine(false);
        let prepared = native
            .prepare_query_with_options(&query, None, options)
            .unwrap();
        let source = image
            .open_session(SegmentedReadLimits::new(CEILING, 2, 16_384, 20_000_000, 8))
            .unwrap();
        let owner_balance = |source: &purrdf_core::SegmentedSession| {
            let receipt = source.evidence();
            receipt.live_bytes() - receipt.unpinned_cache_bytes() - receipt.bookkeeping_bytes()
        };
        let baseline = owner_balance(&source);
        let window = CurrentThreadWindow::open();
        let result = match route {
            0 => {
                native
                    .query_fallible_view(&source, request(&query), options)
                    .unwrap()
                    .result
            }
            1 => {
                native
                    .query_prepared_fallible_view(&source, &prepared, &[], options)
                    .unwrap()
                    .result
            }
            2 => {
                native
                    .query_governed_fallible_view(
                        &source,
                        request(&query),
                        options,
                        &purrdf_sparql_eval::QueryGovernors::METERED,
                    )
                    .unwrap()
                    .result
            }
            _ => {
                native
                    .query_prepared_governed_fallible_view(
                        &source,
                        &prepared,
                        &[],
                        options,
                        &purrdf_sparql_eval::QueryGovernors::METERED,
                    )
                    .unwrap()
                    .result
            }
        };
        let measured = window.close();
        assert!(
            u64::try_from(measured.peak_working_bytes).unwrap() <= source.evidence().peak_bytes(),
            "physical peak remains covered by ORIGINAL pre-admission"
        );
        assert!(source.evidence().peak_bytes() <= CEILING);
        assert_eq!(result.solutions().unwrap().1, expected);
        let live = source.evidence().live_bytes();
        let window = CurrentThreadWindow::open();
        let clone = result.clone();
        drop(result);
        assert_eq!(
            window.close().allocations,
            0,
            "retained result clone is shallow"
        );
        assert_eq!(
            source.evidence().live_bytes(),
            live,
            "cloned matches keep the original grants"
        );
        drop(prepared);
        drop(native);
        drop(env);
        assert!(owner_balance(&source) > baseline);
        assert_eq!(clone.solutions().unwrap().1, expected);
        drop(clone);
        assert_eq!(
            owner_balance(&source),
            baseline,
            "payloads die before original grant release"
        );
    }
    // Admit the source's own control first; only the query's additional owner
    // is denied. Session open requires a cache slot and the header's complete
    // Merkle proof; zero capacities refuse the source before reaching the query.
    let header_capacity = image
        .receipt()
        .block_count()
        .next_power_of_two()
        .trailing_zeros()
        + 1;
    let source_control = image
        .open_session(SegmentedReadLimits::new(
            CEILING,
            1,
            header_capacity,
            20_000_000,
            1,
        ))
        .unwrap();
    let control = source_control.evidence().peak_bytes();
    drop(source_control);
    let denied = image
        .open_session(SegmentedReadLimits::new(
            control,
            1,
            header_capacity,
            20_000_000,
            1,
        ))
        .unwrap();
    let header_requests = denied.evidence().request_count();
    let failure = engine(false).query_fallible_view(
        &denied,
        request(&query),
        QueryOptions::new().with_env(&fixture.env),
    );
    assert!(
        matches!(
            &failure,
            Err(purrdf_sparql_eval::FallibleSparqlError::Operational {
                error: purrdf_core::SegmentedError::Residency { limit, .. },
                ..
            }) if *limit == control
        ),
        "query owner retains its exact original residency refusal: {failure:?}"
    );
    assert_eq!(
        denied.evidence().request_count(),
        header_requests,
        "initial owner refusal precedes all query source reads"
    );
    // A source without the probe-order certificate retains original query support.
    let positions = positions(&["1".into(), "2".into(), "2".into()]);
    let source = segmented(&positions)
        .open_session(SegmentedReadLimits::new(CEILING, 2, 16_384, 20_000_000, 8))
        .unwrap();
    assert!(!source.certifies_probe_order());
    let query = window_query("1");
    let shaped = engine(false)
        .query_fallible_view(&source, request(&query), QueryOptions::EMPTY)
        .unwrap();
    let other = segmented(&positions)
        .open_session(SegmentedReadLimits::new(CEILING, 2, 16_384, 20_000_000, 8))
        .unwrap();
    let literal = engine(true)
        .query_fallible_view(&other, request(&query), QueryOptions::EMPTY)
        .unwrap();
    assert_eq!(
        shaped.result.solutions().unwrap(),
        literal.result.solutions().unwrap(),
        "uncertified order declines optimization and runs the original store path"
    );
}

struct CompileProbe {
    index: Arc<TextIndex>,
    cache: Arc<DictionaryCache>,
    entries: Arc<[(TermValue, String)]>,
    entered: Arc<AtomicUsize>,
}
impl ShapeProvider for CompileProbe {
    fn offer(
        &self,
        request: ShapeRequest<'_>,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<ShapeOffer>, EvalError> {
        if !matches!(request.pattern, GraphPattern::Values { .. }) {
            return Ok(None);
        }
        self.entered.fetch_add(1, Ordering::Relaxed);
        let mut entries =
            purrdf_sparql_eval::AdmittedVec::with_capacity(self.entries.len(), workspace)?;
        for (identity, phrase) in &*self.entries {
            entries.push(DictionaryEntry {
                identity,
                phrase,
                payload: &[],
            })?;
        }
        self.entered.store(2, Ordering::Relaxed);
        let compiled = self.cache.get(&self.index, &entries, workspace)?;
        assert_eq!(compiled.len(), self.entries.len());
        ShapeOffer::new_admitted(
            "dictionary-refusal-probe",
            CopyRows(self.entered.clone()),
            ShapeEstimate::default(),
            workspace,
        )
        .map(Some)
    }
}
pub(crate) fn cache_refusal_keeps_previous_owner() {
    use purrdf_core::SegmentedReadLimits;
    let fixture = Fixture::text(&[(
        "https://example.org/doc/a".into(),
        "lana audley".into(),
        None,
        false,
    )]);
    let image = segmented(&fixture.dataset);
    let cache = Arc::new(DictionaryCache::default());
    let original_id = TermValue::iri("https://example.org/original");
    let original = [DictionaryEntry {
        identity: &original_id,
        phrase: "lana",
        payload: &[],
    }];
    let before = cache
        .get(&fixture.index, &original, &WorkspaceCapability::resident())
        .unwrap();
    let entries: Arc<[(TermValue, String)]> = (0..4_097)
        .map(|index| {
            (
                TermValue::iri(format!("https://example.org/replacement/{index}")),
                format!("absent phrase{index}"),
            )
        })
        .collect();
    let entered = Arc::new(AtomicUsize::new(0));
    let mut shapes = ShapeRegistry::new();
    shapes.register(
        "dictionary-refusal-probe",
        Arc::new(CompileProbe {
            index: fixture.index.clone(),
            cache: cache.clone(),
            entries,
            entered: entered.clone(),
        }),
    );
    let env = ExtensionEnv::over_relations(PropertyFunctionRegistry::new())
        .unwrap()
        .with_shapes(shapes);
    let source = image
        .open_session(SegmentedReadLimits::new(
            1_000_000, 2, 16_384, 20_000_000, 8,
        ))
        .unwrap();
    let failure = engine(false).query_fallible_view(
        &source,
        request("SELECT ?x WHERE { VALUES ?x { 1 } }"),
        QueryOptions::new().with_env(&env),
    );
    assert!(
        matches!(
            &failure,
            Err(purrdf_sparql_eval::FallibleSparqlError::Operational {
                error: purrdf_core::SegmentedError::Residency {
                    limit: 1_000_000,
                    ..
                },
                ..
            })
        ),
        "native replacement retains its exact residency refusal: {failure:?}"
    );
    assert_eq!(
        entered.load(Ordering::Relaxed),
        2,
        "the actual native compilation owner was reached after request metadata admission"
    );
    assert!(matches!(
        purrdf_core::FallibleDatasetView::operation_status(&source),
        purrdf_core::ViewOperationStatus::Ready { .. }
    ));
    let after = cache
        .get(&fixture.index, &original, &WorkspaceCapability::resident())
        .unwrap();
    assert!(
        std::ptr::eq(before.entry(0).unwrap().0, after.entry(0).unwrap().0),
        "failed build did not evict or replace the original successful owner"
    );
    assert_eq!(
        before
            .scan(fixture.index, None, &WorkspaceCapability::resident())
            .unwrap()
            .rows()
            .len(),
        1
    );
    // Retry the same real producer with a sufficient account, then destroy the
    // returned result and evict the cache only AFTER its payload is no longer used.
    let healthy = image
        .open_session(SegmentedReadLimits::new(
            32_000_000, 2, 16_384, 20_000_000, 8,
        ))
        .unwrap();
    let baseline = healthy.evidence().live_bytes()
        - healthy.evidence().unpinned_cache_bytes()
        - healthy.evidence().bookkeeping_bytes();
    let native = engine(false);
    let answer = native
        .query_fallible_view(
            &healthy,
            request("SELECT ?x WHERE { VALUES ?x { 1 } }"),
            QueryOptions::new().with_env(&env),
        )
        .unwrap();
    assert_eq!(answer.result.solutions().unwrap().1.len(), 1);
    drop(answer);
    drop(native);
    assert!(
        healthy.evidence().live_bytes()
            - healthy.evidence().unpinned_cache_bytes()
            - healthy.evidence().bookkeeping_bytes()
            > baseline,
        "cache retains its ORIGINAL native compilation grant"
    );
    cache.clear();
    assert_eq!(
        healthy.evidence().live_bytes()
            - healthy.evidence().unpinned_cache_bytes()
            - healthy.evidence().bookkeeping_bytes(),
        baseline
    );
}

struct Passthrough {
    offered: Arc<AtomicUsize>,
    selected: Arc<AtomicUsize>,
    decline: bool,
}
struct CopyRows(Arc<AtomicUsize>);
impl ShapeProvider for Passthrough {
    fn offer(
        &self,
        request: ShapeRequest<'_>,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<ShapeOffer>, EvalError> {
        if !matches!(request.pattern, GraphPattern::Values { .. }) {
            return Ok(None);
        }
        self.offered.fetch_add(1, Ordering::Relaxed);
        if self.decline {
            return Ok(None);
        }
        ShapeOffer::new_admitted(
            "fixture-store-order",
            CopyRows(self.selected.clone()),
            ShapeEstimate::default(),
            workspace,
        )
        .map(Some)
    }
}
impl ShapeOperator for CopyRows {
    fn execute(&self, runtime: &mut dyn ShapeRuntime) -> Result<(), EvalError> {
        self.0.fetch_add(1, Ordering::Relaxed);
        if let Some(input) = runtime.input(&[])? {
            for ordinal in 0..input.len() {
                let row = input.row(ordinal).expect("original input ordinal");
                assert!(runtime.emit(&[row])?);
            }
        }
        Ok(())
    }
}
pub(crate) fn registered_host_and_decline() {
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let query = "SELECT ?value WHERE { VALUES ?value { 2 1 2 UNDEF } }";
    let expected = engine(true)
        .query_with_options_view(&*dataset, request(query), QueryOptions::EMPTY)
        .unwrap();
    for decline in [false, true] {
        let offered = Arc::new(AtomicUsize::new(0));
        let selected = Arc::new(AtomicUsize::new(0));
        let mut shapes = ShapeRegistry::new();
        shapes.register(
            "fixture-host",
            Arc::new(Passthrough {
                offered: offered.clone(),
                selected: selected.clone(),
                decline,
            }),
        );
        let env = ExtensionEnv::over_relations(PropertyFunctionRegistry::new())
            .unwrap()
            .with_shapes(shapes);
        let answer = engine(false)
            .query_with_options_view(
                &*dataset,
                request(query),
                QueryOptions::new().with_env(&env),
            )
            .unwrap();
        assert_eq!(bytes(&answer), bytes(&expected));
        assert_eq!(
            offered.load(Ordering::Relaxed),
            1,
            "one consultation per original subtree"
        );
        assert_eq!(selected.load(Ordering::Relaxed), usize::from(!decline));
    }
}

pub(crate) fn prepared_and_query_forms() {
    let fixture = Fixture::text(&[(
        "https://example.org/doc/a".into(),
        "lana audley".into(),
        None,
        false,
    )]);
    let query = phrase_query(&["lana audley".into(), "lana".into()]);
    let native = engine(false);
    let options = QueryOptions::new().with_env(&fixture.env);
    let prepared = native
        .prepare_query_with_options(&query, None, options)
        .unwrap();
    assert_eq!(
        bytes(
            &native
                .query_prepared(&fixture.dataset, &prepared, &[], options)
                .unwrap()
        ),
        bytes(&fixture.answer(&query, true))
    );
    for head in [
        "ASK",
        "CONSTRUCT { ?doc <https://example.org/matched> ?phrase }",
        "DESCRIBE ?doc",
    ] {
        let form = query.replacen(
            "SELECT ?entry ?phrase ?extra ?doc ?lang ?start ?end",
            head,
            1,
        );
        let shaped = fixture.answer(&form, false);
        let literal = fixture.answer(&form, true);
        match (shaped, literal) {
            (SparqlResult::Boolean(a), SparqlResult::Boolean(b)) => assert_eq!(a, b),
            (SparqlResult::Graph(a), SparqlResult::Graph(b)) => assert_eq!(
                purrdf_core::ir::canon::try_canonicalize(&a).unwrap().nquads,
                purrdf_core::ir::canon::try_canonicalize(&b).unwrap().nquads
            ),
            (a, b) => panic!("same original form: {a:?}/{b:?}"),
        }
    }
    // Changing a substitution changes the actual native compilation key.
    let parameterized =
        format!("SELECT ?doc ?start WHERE {{ ?doc <{PHRASE}> (?needle ?lang ?start ?end) }}");
    let declared = ["needle"];
    let mut prepared = native
        .prepare_execution(&parameterized, None, &declared, options)
        .unwrap();
    let literal = engine(true);
    let mut reference = literal
        .prepare_execution(&parameterized, None, &declared, options)
        .unwrap();
    let collect = |outcome: InternedOutcome<'_, '_, RdfDataset>| {
        let InternedOutcome::Solutions(rows) = outcome else {
            panic!("prepared SELECT rows");
        };
        rows.rows()
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| cell.map(|cell| rows.value_of(cell)))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    for phrase in ["lana", "lana audley", "stone"] {
        prepared.bind(0, TermValue::simple_literal(phrase)).unwrap();
        reference
            .bind(0, TermValue::simple_literal(phrase))
            .unwrap();
        let actual = native
            .execute(&mut prepared, &*fixture.dataset, options, collect)
            .unwrap();
        let expected = literal
            .execute(&mut reference, &*fixture.dataset, options, collect)
            .unwrap();
        assert_eq!(actual, expected);
    }
}

struct DeclaredClaims {
    record: purrdf_core::evidence::EvidenceRecord,
    complete: purrdf_core::evidence::SearchCompleteness,
    closed: bool,
    fail: bool,
}
impl DeclaredClaims {
    fn new(
        information: purrdf_core::evidence::Information,
        complete: purrdf_core::evidence::SearchCompleteness,
        closed: bool,
    ) -> Self {
        use purrdf_core::evidence::{EvidenceRecord, Frame, InformationMap};
        use purrdf_lex::allocation::{Memory, Resident};
        Self {
            record: EvidenceRecord::from_parts(
                &InformationMap::uniform(information),
                complete,
                &Frame::actual(),
                &[],
                &[],
                &mut Memory::new(&mut Resident),
            )
            .unwrap(),
            complete,
            closed,
            fail: false,
        }
    }
}
impl purrdf_sparql_eval::evidence_exec::EvidenceSource<purrdf_core::TermId> for DeclaredClaims {
    fn claim(
        &self,
        _: &purrdf_core::QuadIds<purrdf_core::TermId>,
    ) -> Result<Option<&purrdf_core::evidence::EvidenceRecord>, EvalError> {
        if self.fail {
            return Err(EvalError::config("the original claim declaration refused"));
        }
        Ok(Some(&self.record))
    }
    fn scope(
        &self,
        _: purrdf_core::GraphMatch<purrdf_core::TermId>,
    ) -> Result<purrdf_sparql_eval::evidence_exec::EvidenceScope, EvalError> {
        Ok(purrdf_sparql_eval::evidence_exec::EvidenceScope {
            completeness: self.complete,
            closure_declared: self.closed,
            missing_reason: None,
        })
    }
    fn reason(
        &self,
        _: purrdf_core::evidence::ReasonId,
    ) -> Result<Option<purrdf_core::evidence::ReasonDescription<'_>>, EvalError> {
        Ok(None)
    }
}

#[derive(Debug)]
struct EvidenceAnswer {
    variables: Vec<String>,
    rows: Vec<(Vec<Option<TermValue>>, purrdf_hash::hex::Digest32, bool)>,
    search: purrdf_sparql_eval::RowEvidence,
    search_identity: purrdf_hash::hex::Digest32,
    existential: purrdf_sparql_eval::RowEvidence,
    existential_identity: purrdf_hash::hex::Digest32,
    existential_undecided: bool,
}
// RowEvidence deliberately has no global numeric-ID equality. Keep the original
// handle separately and compare canonical records across independent executions.
fn evidence_answer(
    dataset: &RdfDataset,
    query: &str,
    options: QueryOptions<'_, '_>,
    claims: &dyn purrdf_sparql_eval::evidence_exec::EvidenceSource<purrdf_core::TermId>,
    disabled: bool,
) -> EvidenceAnswer {
    engine(disabled)
        .query_interned_evidence_view(
            dataset,
            InternedRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            options,
            claims,
            |outcome| {
                let InternedOutcome::Solutions(answer) = outcome else {
                    panic!("original SELECT rows");
                };
                let existential = answer
                    .existential_evidence()
                    .expect("original query-owned existential declaration");
                EvidenceAnswer {
                    variables: answer
                        .variables()
                        .iter()
                        .map(|variable| variable.as_str().to_owned())
                        .collect(),
                    rows: answer
                        .rows()
                        .iter()
                        .map(|row| {
                            (
                                row.iter()
                                    .map(|cell| cell.map(|cell| answer.value_of(cell)))
                                    .collect(),
                                row.evidence()
                                    .with_record(purrdf_core::evidence::EvidenceRecord::identity)
                                    .unwrap(),
                                row.evidence().is_undecided(),
                            )
                        })
                        .collect(),
                    search: answer.search_evidence().clone(),
                    search_identity: answer
                        .search_evidence()
                        .with_record(purrdf_core::evidence::EvidenceRecord::identity)
                        .unwrap(),
                    existential_identity: existential
                        .with_record(purrdf_core::evidence::EvidenceRecord::identity)
                        .unwrap(),
                    existential_undecided: existential.is_undecided(),
                    existential,
                }
            },
        )
        .unwrap()
}
fn equal_evidence(
    dataset: &RdfDataset,
    query: &str,
    options: QueryOptions<'_, '_>,
    claims: &dyn purrdf_sparql_eval::evidence_exec::EvidenceSource<purrdf_core::TermId>,
) -> EvidenceAnswer {
    let literal = evidence_answer(dataset, query, options, claims, true);
    let shaped = evidence_answer(dataset, query, options, claims, false);
    assert_eq!(
        shaped.variables, literal.variables,
        "original evidence column order"
    );
    assert_eq!(
        shaped.rows, literal.rows,
        "original bindings, canonical proof records and membership: {query}"
    );
    assert_eq!(
        shaped.search_identity, literal.search_identity,
        "whole reached search, including empty output: {query}"
    );
    assert_eq!(
        (shaped.existential_identity, shaped.existential_undecided),
        (literal.existential_identity, literal.existential_undecided),
        "original existential publication and closure: {query}"
    );
    shaped
}

#[derive(Clone, Copy)]
enum GapSite {
    FalseWindowPair,
    Adjacency,
    Barrier,
}
struct ExpressionGaps {
    claims: DeclaredClaims,
    site: GapSite,
    reason: purrdf_core::evidence::ReasonId,
    cause: TermValue,
    concern: TermValue,
    reached: AtomicUsize,
    declared: AtomicUsize,
    fail: bool,
}
impl ExpressionGaps {
    fn new(site: GapSite) -> Self {
        use purrdf_core::evidence::{Information, ReasonDescription, SearchCompleteness};
        use purrdf_lex::allocation::{Memory, Resident};
        let cause = TermValue::iri("https://example.org/original-expression-gap");
        let concern = TermValue::Triple {
            s: purrdf_core::TermBox::new(TermValue::iri("https://example.org/expression")),
            p: purrdf_core::TermBox::new(TermValue::iri("https://example.org/requires")),
            o: purrdf_core::TermBox::new(TermValue::iri(
                "https://example.org/required-expression-support",
            )),
        };
        let reason = ReasonDescription {
            cause: &cause,
            concern: &concern,
            constraint: None,
            vault: None,
            watermark: None,
        }
        .identity(&mut Memory::new(&mut Resident))
        .unwrap();
        Self {
            claims: DeclaredClaims::new(Information::Supported, SearchCompleteness::Complete, true),
            site,
            reason,
            cause,
            concern,
            reached: AtomicUsize::new(0),
            declared: AtomicUsize::new(0),
            fail: false,
        }
    }
}
impl purrdf_sparql_eval::evidence_exec::EvidenceSource<purrdf_core::TermId> for ExpressionGaps {
    fn claim(
        &self,
        quad: &purrdf_core::QuadIds<purrdf_core::TermId>,
    ) -> Result<Option<&purrdf_core::evidence::EvidenceRecord>, EvalError> {
        purrdf_sparql_eval::evidence_exec::EvidenceSource::claim(&self.claims, quad)
    }
    fn scope(
        &self,
        graph: purrdf_core::GraphMatch<purrdf_core::TermId>,
    ) -> Result<purrdf_sparql_eval::evidence_exec::EvidenceScope, EvalError> {
        purrdf_sparql_eval::evidence_exec::EvidenceSource::scope(&self.claims, graph)
    }
    fn reason(
        &self,
        reason: purrdf_core::evidence::ReasonId,
    ) -> Result<Option<purrdf_core::evidence::ReasonDescription<'_>>, EvalError> {
        Ok(
            (reason == self.reason).then_some(purrdf_core::evidence::ReasonDescription {
                cause: &self.cause,
                concern: &self.concern,
                constraint: None,
                vault: None,
                watermark: None,
            }),
        )
    }
    fn expression_gap(
        &self,
        expression: &purrdf_sparql_algebra::Expression,
        schema: &[purrdf_sparql_algebra::Variable],
        row: &[Option<purrdf_sparql_eval::SolutionTerm<purrdf_core::TermId>>],
        _: purrdf_core::GraphMatch<purrdf_core::TermId>,
    ) -> Result<Option<purrdf_core::evidence::ReasonId>, EvalError> {
        use purrdf_sparql_algebra::Expression;
        self.reached.fetch_add(1, Ordering::Relaxed);
        let column = |name: &str| schema.iter().position(|variable| variable.as_str() == name);
        let selected = match self.site {
            GapSite::FalseWindowPair => {
                matches!(expression, Expression::LessOrEqual(..))
                    && match (column("a"), column("b")) {
                        (Some(a), Some(b)) => {
                            row[a].is_some() && row[b].is_some() && row[a] != row[b]
                        }
                        _ => false,
                    }
            }
            GapSite::Adjacency => {
                matches!(expression, Expression::Equal(..))
                    && column("start").is_some()
                    && column("finish").is_some()
            }
            GapSite::Barrier => matches!(expression, Expression::And(_)) && column("k").is_some(),
        };
        if !selected {
            return Ok(None);
        }
        self.declared.fetch_add(1, Ordering::Relaxed);
        if self.fail {
            return Err(EvalError::WorkspaceUnpriced(
                "the original false-row expression declaration refused",
            ));
        }
        Ok(Some(self.reason))
    }
}

pub(crate) fn false_expression_gaps_keep_original_traversal() {
    let dataset = positions(&["0".into(), "1".into(), "3".into(), "4".into()]);
    let query = window_query("0");
    let standard = engine(false)
        .query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY)
        .unwrap();
    assert_eq!(
        rows(&standard).len(),
        4,
        "ordinary exact window has only its four diagonal rows"
    );
    let report = engine(false).explain_query(&dataset, &query, None).unwrap();
    assert!(
        report
            .shapes()
            .iter()
            .any(|shape| shape.operator == Some("ordered-window")),
        "standard execution keeps the indexed production path"
    );
    let source = ExpressionGaps::new(GapSite::FalseWindowPair);
    let literal = evidence_answer(&dataset, &query, QueryOptions::EMPTY, &source, true);
    let reached = source.reached.swap(0, Ordering::Relaxed);
    let declared = source.declared.swap(0, Ordering::Relaxed);
    let shaped = evidence_answer(&dataset, &query, QueryOptions::EMPTY, &source, false);
    assert_eq!(shaped.rows, literal.rows);
    assert_eq!(
        shaped.rows.len(),
        8,
        "four false off-diagonal pairs are retained beside four ordinary diagonal pairs"
    );
    assert_eq!(
        shaped
            .rows
            .iter()
            .filter(|(_, _, undecided)| *undecided)
            .count(),
        4
    );
    assert_eq!(
        (
            source.reached.load(Ordering::Relaxed),
            source.declared.load(Ordering::Relaxed)
        ),
        (reached, declared),
        "physical access must reach every original expression declaration"
    );
    assert_eq!(declared, 4);
    assert_eq!(shaped.search_identity, literal.search_identity);
    assert_eq!(
        (shaped.existential_identity, shaped.existential_undecided),
        (literal.existential_identity, literal.existential_undecided)
    );
    let denied = ExpressionGaps {
        fail: true,
        ..source
    };
    for disabled in [true, false] {
        let failure = engine(disabled)
            .query_interned_evidence_view(
                &*dataset,
                InternedRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &denied,
                |_| (),
            )
            .unwrap_err();
        assert!(
            failure
                .message
                .contains("the original false-row expression declaration refused"),
            "a known false comparison cannot swallow its original refusal: {failure:?}"
        );
    }

    let fixture = Fixture::text(&[(
        "https://example.org/doc/a".into(),
        "lana audley lana audley".into(),
        None,
        false,
    )]);
    let adjacency = format!(
        "SELECT * WHERE {{ VALUES (?entry ?phrase ?a ?b) {{ (<https://example.org/entry> \"lana audley\" \"lana\" \"audley\") }} ?doc <{OCCURS}> (?a ?lang ?start) . ?doc <{OCCURS}> (?b ?lang ?finish) . FILTER(?finish = ?start + 1) }}"
    );
    let gaps = ExpressionGaps::new(GapSite::Adjacency);
    let answer = equal_evidence(
        &fixture.dataset,
        &adjacency,
        QueryOptions::new().with_env(&fixture.env),
        &gaps,
    );
    assert_eq!(
        answer.rows.len(),
        4,
        "both positional mismatches retain original missing-expression support"
    );
    assert!(answer.rows.iter().all(|(_, _, undecided)| *undecided));

    let barrier = query.replace("ABS(?j - ?i) <= 0", &format!("ABS(?j - ?i) <= 0 && NOT EXISTS {{ ?c <{SEQUENCE}> ?seq ; <{POS}> ?k . FILTER(?k > IF(?i < ?j, ?i, ?j) && ?k < IF(?i < ?j, ?j, ?i)) }}"));
    let gaps = ExpressionGaps::new(GapSite::Barrier);
    let answer = equal_evidence(&dataset, &barrier, QueryOptions::EMPTY, &gaps);
    assert_ne!(answer.rows, []);
    assert!(
        answer.rows.iter().all(|(_, _, undecided)| *undecided),
        "false inner comparisons still publish undecided existential support before native negation"
    );
    assert!(gaps.declared.load(Ordering::Relaxed) > 0);
    assert_eq!(
        answer
            .existential
            .with_record(purrdf_core::evidence::EvidenceRecord::identity)
            .unwrap(),
        answer.existential_identity,
        "original existential owner survives the query and selected physical context"
    );
}

struct EvidencePassthrough(Arc<AtomicUsize>);
impl ShapeProvider for EvidencePassthrough {
    fn offer(
        &self,
        request: ShapeRequest<'_>,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<ShapeOffer>, EvalError> {
        if !matches!(request.pattern, GraphPattern::Bgp { .. }) {
            return Ok(None);
        }
        ShapeOffer::new_admitted(
            "original-evidence-host",
            CopyRows(self.0.clone()),
            ShapeEstimate::default(),
            workspace,
        )
        .map(Some)
    }
}
pub(crate) fn original_evidence_and_empty_barriers() {
    use purrdf_core::evidence::{Information, SearchCompleteness};
    let fixture = Fixture::text(&[(
        "https://example.org/doc/a".into(),
        "lana audley lana".into(),
        None,
        false,
    )]);
    let query = phrase_query(&["lana".into(), "lana audley".into()])
        .replace("WHERE {", &format!("WHERE {{ ?doc <{NOTE}> ?authored ."));
    let positions = positions(&["0".into(), "1".into(), "3".into(), "4".into()]);
    let calendar = series();
    let group = format!(
        "SELECT ?bucket (COUNT(?s) AS ?n) (SUM(?n) AS ?total) (FOLD(?n) AS ?order) WHERE {{ ?s <{TIME}> ?t ; <{POS}> ?n . BIND(<{}>(?t) AS ?bucket) }} GROUP BY ?bucket",
        purrdf_xsd::datatype::XSD_DATE
    );
    for information in [
        Information::Both,
        Information::Opposed,
        Information::Neither,
    ] {
        for (complete, closed) in [
            (SearchCompleteness::Complete, true),
            (SearchCompleteness::Complete, false),
            (SearchCompleteness::Undetermined, false),
        ] {
            let claims = DeclaredClaims::new(information, complete, closed);
            let dictionary = equal_evidence(
                &fixture.dataset,
                &query,
                QueryOptions::new().with_env(&fixture.env),
                &claims,
            );
            assert_ne!(dictionary.rows, []);
            assert!(dictionary.rows.iter().all(|(_, identity, _)| *identity
                != purrdf_core::evidence::EvidenceRecord::default().identity()));
            equal_evidence(&positions, &window_query("2"), QueryOptions::EMPTY, &claims);
            equal_evidence(&calendar, &group, QueryOptions::EMPTY, &claims);
            // Empty and nonempty intervals both retain the original reached
            // source scope. ClosedAbsence is decided by the EXISTS owner.
            for bound in ["0", "2", "10"] {
                let query = window_query(bound).replace(&format!("ABS(?j - ?i) <= {bound}"), &format!("ABS(?j - ?i) <= {bound} && NOT EXISTS {{ ?c <{SEQUENCE}> ?seq ; <{POS}> ?k . FILTER(?k > IF(?i < ?j, ?i, ?j) && ?k < IF(?i < ?j, ?j, ?i)) }}"));
                let answer = equal_evidence(&positions, &query, QueryOptions::EMPTY, &claims);
                if !closed {
                    assert!(
                        answer.rows.iter().all(|(_, _, undecided)| *undecided),
                        "an indexed empty interval is not a closure declaration"
                    );
                }
            }
            let empty = format!("SELECT ?a WHERE {{ ?a <{POS}> ?i FILTER(?i < -100) }}");
            let retained = equal_evidence(&positions, &empty, QueryOptions::EMPTY, &claims);
            assert_eq!(retained.rows, []);
            assert_eq!(
                retained
                    .search
                    .with_record(purrdf_core::evidence::EvidenceRecord::identity)
                    .unwrap(),
                retained.search_identity,
                "search metadata survives query owner destruction"
            );
        }
    }
    let claims = DeclaredClaims::new(Information::Both, SearchCompleteness::Complete, true);
    let mut registry = ShapeRegistry::new();
    registry.register(
        "original-evidence-host",
        Arc::new(EvidencePassthrough(Arc::new(AtomicUsize::new(0)))),
    );
    let env = ExtensionEnv::over_relations(PropertyFunctionRegistry::new())
        .unwrap()
        .with_shapes(registry);
    let host_query = format!("SELECT ?s ?o WHERE {{ ?s <{NOTE}> ?o }}");
    equal_evidence(
        &fixture.dataset,
        &host_query,
        QueryOptions::new().with_env(&env),
        &claims,
    );
    // The explicit geometric consumer returns the exact original borrowed row
    // and bag search handle; it never extracts and rebuilds a record.
    engine(false)
        .query_interned_evidence_view(
            &*fixture.dataset,
            InternedRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::new().with_env(&fixture.env),
            &claims,
            |outcome| {
                let InternedOutcome::Solutions(rows) = outcome else {
                    panic!("SELECT");
                };
                let selected = rows
                    .longest_first_spans(&["doc", "lang"], "start", "end")
                    .unwrap();
                assert!(std::ptr::eq(
                    selected.search_evidence(),
                    rows.search_evidence()
                ));
                for (&index, row) in selected.ordinals().iter().zip(selected.rows()) {
                    assert!(std::ptr::eq(row.evidence(), rows.rows()[index].evidence()));
                }
            },
        )
        .unwrap();
    let denied = DeclaredClaims {
        fail: true,
        ..claims
    };
    for disabled in [true, false] {
        let failure = engine(disabled)
            .query_interned_evidence_view(
                &*positions,
                InternedRequest {
                    query: &window_query("2"),
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &denied,
                |_| (),
            )
            .unwrap_err();
        assert!(
            failure
                .message
                .contains("the original claim declaration refused"),
            "physical selection preserves the reached declaration failure: {failure:?}"
        );
    }
}

pub(crate) fn scoped_streams_and_update_where() {
    let mut builder = RdfDatasetBuilder::new();
    let position = builder.intern_iri(POS).unwrap();
    let sequence = builder.intern_iri(SEQUENCE).unwrap();
    let reifies = builder
        .intern_iri(purrdf_core::vocab::rdf::REIFIES)
        .unwrap();
    let graphs = [
        None,
        Some(builder.intern_iri("https://example.org/g1").unwrap()),
        Some(builder.intern_iri("https://example.org/g2").unwrap()),
    ];
    for (index, graph) in graphs.iter().enumerate() {
        let s = builder
            .intern_iri(&format!("https://example.org/token/{index}"))
            .unwrap();
        let number = builder
            .intern_literal(RdfLiteral::typed(
                index.to_string(),
                purrdf_xsd::datatype::XSD_INTEGER,
            ))
            .unwrap();
        let group = builder
            .intern_iri("https://example.org/common-sequence")
            .unwrap();
        builder.push_quad(s, position, number, *graph).unwrap();
        builder.push_quad(s, sequence, group, *graph).unwrap();
        if graph.is_some() {
            let first = builder
                .intern_iri("https://example.org/token/shared")
                .unwrap();
            builder.push_quad(first, position, number, *graph).unwrap();
            builder.push_quad(first, sequence, group, *graph).unwrap();
        }
    }
    // The position and sequence are native virtual annotation rows, not quads.
    for index in 0..3 {
        let subject = builder
            .intern_iri(&format!("https://example.org/statement/{index}"))
            .unwrap();
        let predicate = builder.intern_iri(NOTE).unwrap();
        let object = builder
            .intern_literal(RdfLiteral::simple("original"))
            .unwrap();
        let triple = builder.intern_triple(subject, predicate, object).unwrap();
        let reifier = builder
            .intern_iri(&format!("https://example.org/reifier/{index}"))
            .unwrap();
        builder.push_reifier(reifier, triple).unwrap();
        let number = builder
            .intern_literal(RdfLiteral::typed(
                index.to_string(),
                purrdf_xsd::datatype::XSD_INTEGER,
            ))
            .unwrap();
        let group = builder
            .intern_iri("https://example.org/virtual-sequence")
            .unwrap();
        builder.push_annotation(reifier, position, number).unwrap();
        builder.push_annotation(reifier, sequence, group).unwrap();
    }
    let _ = reifies;
    let dataset = builder.freeze().unwrap();
    let ordinary = window_query("2");
    for query in [ordinary.clone(), ordinary.replace("WHERE", "FROM <https://example.org/g1> FROM <https://example.org/g2> WHERE"), ordinary.replace("WHERE {", "FROM NAMED <https://example.org/g1> FROM NAMED <https://example.org/g2> WHERE { GRAPH ?g {") ] {
        let query = if query.contains("GRAPH ?g") { format!("{query} }}") } else { query };
        let literal = engine(true).query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY).unwrap();
        let shaped = engine(false).query_with_options_view(&*dataset, request(&query), QueryOptions::EMPTY).unwrap();
        assert_eq!(bytes(&shaped), bytes(&literal), "actual default/named/merged/virtual native probe order: {query}");
        let report = engine(false).explain_query(&dataset, &query, None).unwrap();
        assert!(report.shapes().iter().any(|shape| shape.operator == Some("ordered-window")));
    }
    let fixture = Fixture::text(&[(
        "https://example.org/doc/a".into(),
        "lana audley".into(),
        None,
        false,
    )]);
    let original = phrase_query(&["lana".into(), "lana audley".into()]);
    let body = original.split_once(" WHERE ").unwrap().1;
    let update = format!("INSERT {{ ?doc <https://example.org/match> ?entry }} WHERE {body}");
    let mut literal = fixture.dataset.clone();
    let mut shaped = fixture.dataset.clone();
    let options = QueryOptions::new().with_env(&fixture.env);
    engine(true)
        .update_with_options(&mut literal, request(&update), options)
        .unwrap();
    engine(false)
        .update_with_options(&mut shaped, request(&update), options)
        .unwrap();
    assert_eq!(
        purrdf_core::ir::canon::try_canonicalize(&literal)
            .unwrap()
            .nquads,
        purrdf_core::ir::canon::try_canonicalize(&shaped)
            .unwrap()
            .nquads,
        "UPDATE WHERE uses the same native physical dispatch and original graph writer"
    );
}
