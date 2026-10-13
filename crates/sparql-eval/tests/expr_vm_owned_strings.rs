// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Real-entry string and nested-term ownership fixtures.

mod support;

#[global_allocator]
static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

thread_local! {
    static TEXT_WINDOW: std::cell::RefCell<Option<purrdf_alloc_probe::CurrentThreadWindow>> = const { std::cell::RefCell::new(None) };
}

struct TextRelease(std::sync::Arc<std::sync::atomic::AtomicI64>);
impl Drop for TextRelease {
    fn drop(&mut self) {
        let retained = TEXT_WINDOW.with(|window| {
            window
                .borrow()
                .as_ref()
                .expect("active text window")
                .sample()
                .retained_bytes
        });
        self.0.store(retained, std::sync::atomic::Ordering::Relaxed);
    }
}

#[test]
fn shallow_authored_leaf_clone_keeps_original_text_until_last_owner() {
    use purrdf_lex::allocation::SharedText;
    use purrdf_sparql_algebra::{Expression, Literal, NamedNode, Variable};
    use std::sync::atomic::{AtomicI64, Ordering};
    let observed = std::sync::Arc::new(AtomicI64::new(i64::MIN));
    let datatype = NamedNode::new_unchecked(purrdf_xsd::datatype::XSD_STRING);
    for kind in 0..3 {
        observed.store(i64::MIN, Ordering::Relaxed);
        TEXT_WINDOW.with(|window| {
            *window.borrow_mut() = Some(purrdf_alloc_probe::CurrentThreadWindow::open());
        });
        let text = "x".repeat(32_768);
        let native = SharedText::try_from_admitted(text, TextRelease(observed.clone())).unwrap();
        let leaf = match kind {
            0 => Expression::Variable(Variable::from_admitted(native)),
            1 => Expression::NamedNode(NamedNode::from_admitted(native)),
            _ => Expression::Literal(Literal::from_admitted(native, datatype.clone(), None, None)),
        };
        let before = TEXT_WINDOW.with(|window| window.borrow().as_ref().unwrap().sample());
        let kept = leaf.clone();
        drop(leaf);
        let after = TEXT_WINDOW.with(|window| window.borrow().as_ref().unwrap().sample());
        assert_eq!(
            after.allocations, before.allocations,
            "ordinary lexical clones allocate nothing"
        );
        assert_eq!(after.retained_bytes, before.retained_bytes);
        assert_eq!(observed.load(Ordering::Relaxed), i64::MIN);
        drop(kept);
        assert_eq!(
            observed.load(Ordering::Relaxed),
            0,
            "header and original String precede grant destruction"
        );
        let measured = TEXT_WINDOW.with(|window| window.borrow_mut().take().unwrap().close());
        assert_eq!(measured.retained_bytes, 0);
    }
}

use purrdf_core::{SparqlRequest, TermValue};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryGovernors, QueryOptions, RetainedSparqlResult};
use support::segmented::{CEILING, fixture, open_with_evidence};

fn literal(
    value: Option<&TermValue>,
    lexical: &str,
    language: Option<&str>,
    direction: Option<purrdf_core::RdfTextDirection>,
) {
    let Some(TermValue::Literal {
        lexical_form,
        language: actual_language,
        direction: actual_direction,
        ..
    }) = value
    else {
        panic!("expected literal fixture cell");
    };
    assert_eq!(lexical_form, lexical);
    assert_eq!(actual_language.as_deref(), language);
    assert_eq!(*actual_direction, direction);
}

fn bounded_matrix(query: &str, mut check: impl FnMut(&RetainedSparqlResult)) {
    let (image, _) = fixture();
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(query, None).unwrap();
    let request = || SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    };
    for entry in 0..4 {
        let source = open_with_evidence(&image, CEILING, 2, 16_384);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = match entry {
            0 => {
                engine
                    .query_fallible_view(&source, request(), QueryOptions::EMPTY)
                    .unwrap()
                    .result
            }
            1 => {
                engine
                    .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
                    .unwrap()
                    .result
            }
            2 => {
                engine
                    .query_governed_fallible_view(
                        &source,
                        request(),
                        QueryOptions::EMPTY,
                        &QueryGovernors::METERED,
                    )
                    .unwrap()
                    .result
            }
            _ => {
                engine
                    .query_prepared_governed_fallible_view(
                        &source,
                        &prepared,
                        &[],
                        QueryOptions::EMPTY,
                        &QueryGovernors::METERED,
                    )
                    .unwrap()
                    .result
            }
        };
        let measured = window.close();
        assert!(
            u64::try_from(measured.peak_working_bytes).unwrap() <= source.evidence().peak_bytes(),
            "entry {entry}: actual peak {} exceeds admitted peak {}",
            measured.peak_working_bytes,
            source.evidence().peak_bytes(),
        );
        let live = source.evidence().live_bytes();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let kept = result.clone();
        drop(result);
        let measured = window.close();
        assert_eq!(
            measured.allocations, 0,
            "retained result clones share original owners"
        );
        assert_eq!(source.evidence().live_bytes(), live);
        check(&kept);
    }
}

#[test]
fn composite_fold_and_unfold_keep_original_owners_on_every_entry() {
    let prefix = "PREFIX cdt: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/> ";
    let query = format!(
        "{prefix} SELECT (FOLD(DISTINCT ?value ORDER BY ?sort) AS ?list) WHERE {{ VALUES (?value ?sort) {{ (3 0) (1 1) (3 -1) (2 2) (UNDEF 3) }} }}"
    );
    bounded_matrix(&query, |result| {
        let (_, rows) = result.solutions().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0][0],
            Some(TermValue::typed_literal(
                format!(
                    "[\"3\"^^<{0}>,\"1\"^^<{0}>,\"2\"^^<{0}>,null]",
                    purrdf_xsd::datatype::XSD_INTEGER
                ),
                purrdf_cdt::CDT_LIST
            ))
        );
    });
    let query = format!(
        "{prefix} SELECT (FOLD(?key, ?value ORDER BY ?sort) AS ?map) WHERE {{ VALUES (?key ?value ?sort) {{ ('k' 1 0) ('k' 2 1) ('z' UNDEF 2) }} }}"
    );
    bounded_matrix(&query, |result| {
        let (_, rows) = result.solutions().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0][0],
            Some(TermValue::typed_literal(
                format!(
                    "{{\"k\"^^<{0}>:\"2\"^^<{1}>,\"z\"^^<{0}>:null}}",
                    purrdf_xsd::datatype::XSD_STRING,
                    purrdf_xsd::datatype::XSD_INTEGER
                ),
                purrdf_cdt::CDT_MAP
            ))
        );
    });
    let query = format!(
        "{prefix} SELECT ?item ?index WHERE {{ BIND(cdt:List(3,1,2) AS ?list) UNFOLD(?list AS ?item, ?index) }}"
    );
    bounded_matrix(&query, |result| {
        let (_, rows) = result.solutions().unwrap();
        let expected = [3u64, 1, 2];
        assert_eq!(rows.len(), expected.len());
        for (position, (row, value)) in rows.iter().zip(expected).enumerate() {
            assert_eq!(row[0], Some(TermValue::integer(value)));
            assert_eq!(
                row[1],
                Some(TermValue::integer(u64::try_from(position + 1).unwrap()))
            );
        }
    });
}

#[test]
fn string_views_preserve_plain_tagged_directional_type_errors_and_duplicate_bags() {
    let query = r#"SELECT ?id (STR(?v) AS ?s) (LANG(?v) AS ?l)
        (CONTAINS(STR(?v), "é") AS ?str_contains) (CONTAINS(?v, "é") AS ?contains)
        (CONCAT(?v, ?v) AS ?joined) (SUBSTR(?v, 2, 1) AS ?part)
        (LANGMATCHES(LANG(?v), "fr") AS ?matched)
        WHERE { VALUES (?id ?v) {
          (0 "aé"@fr--rtl) (1 "aé"@fr--rtl) (1 "aé"@fr--rtl) (2 "xy") (3 12)
        } } ORDER BY ?id"#;
    bounded_matrix(query, |result| {
        let (_, rows) = result.solutions().unwrap();
        assert_eq!(rows.len(), 5);
        let rtl = Some(purrdf_core::RdfTextDirection::Rtl);
        for (row, id) in rows[..3].iter().zip(["0", "1", "1"]) {
            literal(row[0].as_ref(), id, None, None);
            literal(row[1].as_ref(), "aé", None, None);
            literal(row[2].as_ref(), "fr", None, None);
            literal(row[3].as_ref(), "true", None, None);
            literal(row[4].as_ref(), "true", None, None);
            literal(row[5].as_ref(), "aéaé", Some("fr"), rtl);
            literal(row[6].as_ref(), "é", Some("fr"), rtl);
            literal(row[7].as_ref(), "true", None, None);
        }
        literal(rows[3][0].as_ref(), "2", None, None);
        literal(rows[3][1].as_ref(), "xy", None, None);
        literal(rows[3][2].as_ref(), "", None, None);
        literal(rows[3][3].as_ref(), "false", None, None);
        literal(rows[3][4].as_ref(), "false", None, None);
        literal(rows[3][5].as_ref(), "xyxy", None, None);
        literal(rows[3][6].as_ref(), "y", None, None);
        literal(rows[3][7].as_ref(), "false", None, None);
        literal(rows[4][0].as_ref(), "3", None, None);
        literal(rows[4][1].as_ref(), "12", None, None);
        literal(rows[4][2].as_ref(), "", None, None);
        literal(rows[4][3].as_ref(), "false", None, None);
        assert!(rows[4][4].is_none());
        assert!(rows[4][5].is_none());
        assert!(rows[4][6].is_none());
        literal(rows[4][7].as_ref(), "false", None, None);
    });
}

#[test]
fn nested_constructor_value_owners_survive_until_component_interning() {
    let query = r#"SELECT (STR(SUBJECT(OBJECT(?v))) AS ?inner_subject)
        (STR(PREDICATE(?v)) AS ?outer_predicate) (STR(OBJECT(OBJECT(?v))) AS ?text)
        WHERE { BIND(TRIPLE(<http://example.org/outer>, <http://example.org/p>,
          TRIPLE(<http://example.org/inner>, <http://example.org/q>, "é")) AS ?v) }"#;
    bounded_matrix(query, |result| {
        let (_, rows) = result.solutions().unwrap();
        assert_eq!(rows.len(), 1);
        literal(rows[0][0].as_ref(), "http://example.org/inner", None, None);
        literal(rows[0][1].as_ref(), "http://example.org/p", None, None);
        literal(rows[0][2].as_ref(), "é", None, None);
    });
}

#[test]
fn scalar_producers_preserve_native_values_and_original_output_owners() {
    let query = format!(
        r#"BASE <http://example.org/base/>
        PREFIX xsd: <{}>
        SELECT (DATATYPE(?d) AS ?kind) (IRI("../value") AS ?iri)
          (ENCODE_FOR_URI("é /") AS ?encoded) (MD5("abc") AS ?digest)
          (STRLANG("é", "FR") AS ?tagged) (STRLANGDIR("é", "FR", "rtl") AS ?directed)
          (YEAR(?d) AS ?year) (MONTH(?d) AS ?month) (DAY(?d) AS ?day)
          (TZ(?d) AS ?tz) (TIMEZONE(?d) AS ?zone)
          (STR(ADJUST(?d, "")) AS ?local)
          (BNODE("same") = BNODE("same") AS ?same) (STRUUID() AS ?uuid)
        WHERE {{ VALUES ?d {{ "2024-02-29T12:34:56+05:30"^^xsd:dateTime }} }}"#,
        purrdf_xsd::XSD_NS,
    );
    bounded_matrix(&query, |result| {
        let (_, rows) = result.solutions().unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(
            row[0],
            Some(TermValue::iri(purrdf_xsd::datatype::XSD_DATE_TIME))
        );
        assert_eq!(row[1], Some(TermValue::iri("http://example.org/value")));
        literal(row[2].as_ref(), "%C3%A9%20%2F", None, None);
        literal(
            row[3].as_ref(),
            "900150983cd24fb0d6963f7d28e17f72",
            None,
            None,
        );
        literal(row[4].as_ref(), "é", Some("fr"), None);
        literal(
            row[5].as_ref(),
            "é",
            Some("fr"),
            Some(purrdf_core::RdfTextDirection::Rtl),
        );
        literal(row[6].as_ref(), "2024", None, None);
        literal(row[7].as_ref(), "2", None, None);
        literal(row[8].as_ref(), "29", None, None);
        literal(row[9].as_ref(), "+05:30", None, None);
        literal(row[10].as_ref(), "PT5H30M", None, None);
        literal(row[11].as_ref(), "2024-02-29T12:34:56", None, None);
        literal(row[12].as_ref(), "true", None, None);
        let Some(TermValue::Literal {
            lexical_form: uuid, ..
        }) = &row[13]
        else {
            panic!("STRUUID returns a literal");
        };
        assert_eq!(uuid.len(), 36);
        assert_eq!(uuid.as_bytes()[14], b'4');
        assert!(matches!(uuid.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
        for (index, byte) in uuid.bytes().enumerate() {
            if matches!(index, 8 | 13 | 18 | 23) {
                assert_eq!(byte, b'-');
            } else {
                assert!(byte.is_ascii_hexdigit());
            }
        }
    });
}

#[test]
fn complete_unselected_regex_entries_preserve_bags_unicode_and_final_newline_law() {
    let query = r#"SELECT ?id (REGEX(?text, "^$", "m") AS ?final)
        (REGEX(?text, "\\p{Lu}", "i") AS ?fold)
        (REGEX(?text, "\\i") AS ?name)
        (REGEX(?text, "\\&\\~") AS ?liberal)
        (REGEX(?text, "(a)\\1") AS ?bad)
        (REPLACE(?text, "", "|") AS ?expanded)
        WHERE { VALUES (?id ?text) {
            (0 "a\n") (1 "é") (1 "é") (2 "ı") (3 "&~") (4 "") (5 "꟎")
        } } ORDER BY ?id"#;
    bounded_matrix(query, |result| {
        let (_, rows) = result.solutions().unwrap();
        assert_eq!(rows.len(), 7);
        let expected = [
            ("0", true, true, true, false, "|a|\n|"),
            ("1", false, true, true, false, "|é|"),
            ("1", false, true, true, false, "|é|"),
            ("2", false, false, true, false, "|ı|"),
            ("3", false, false, false, true, "|&|~|"),
            ("4", true, false, false, false, "|"),
            // U+A7CE is Cn in compatibility's Unicode 16, while the shared
            // XML NameStartChar production admits its scalar range.
            ("5", false, false, true, false, "|꟎|"),
        ];
        for (row, (id, final_line, fold, name, liberal, expanded)) in rows.iter().zip(expected) {
            literal(row[0].as_ref(), id, None, None);
            for (column, value) in [final_line, fold, name, liberal].into_iter().enumerate() {
                literal(
                    row[1 + column].as_ref(),
                    if value { "true" } else { "false" },
                    None,
                    None,
                );
            }
            assert!(
                row[5].is_none(),
                "unsupported backreference remains an expression error"
            );
            literal(row[6].as_ref(), expanded, None, None);
        }
    });
}

#[test]
fn complete_unselected_replace_entries_keep_empty_utf8_q_and_lexical_errors() {
    let query = r#"SELECT (REPLACE("aé", "", "$1", "q") AS ?verbatim)
        (REPLACE("aba", "a*", "X") AS ?suppressed)
        (REPLACE("aé", "", "\\$") AS ?escaped)
        (REPLACE("no", "z", "$") AS ?invalid)
        (REPLACE("aba", "(a*)*", "$1|") AS ?capture)
        WHERE {}"#;
    let old = purrdf_core::xsd_regex::compile("(a*)*", "").unwrap();
    let captured = old.replace_all("aba", "$1|").unwrap();
    bounded_matrix(query, |result| {
        let (_, rows) = result.solutions().unwrap();
        assert_eq!(rows.len(), 1);
        literal(rows[0][0].as_ref(), "$1a$1é$1", None, None);
        literal(rows[0][1].as_ref(), "XbX", None, None);
        literal(rows[0][2].as_ref(), "$a$é$", None, None);
        assert!(
            rows[0][3].is_none(),
            "invalid template is checked before an unmatched search"
        );
        literal(rows[0][4].as_ref(), captured.as_ref(), None, None);
    });
}

#[test]
fn complete_unselected_regex_reaches_nonselect_entries_through_the_native_owner() {
    bounded_matrix(r#"ASK { FILTER(REGEX("a\n", "^$", "m")) }"#, |result| {
        assert_eq!(result.boolean(), Some(true));
    });
    bounded_matrix(
        r#"CONSTRUCT { <http://example.org/s> <http://example.org/p> ?text }
        WHERE { BIND(REPLACE("aé", "", "|") AS ?text) }"#,
        |result| {
            let graph = result.graph().unwrap();
            assert_eq!(graph.quad_count(), 1);
            let value = graph.term_value(graph.quads().next().unwrap().o);
            assert!(
                matches!(value, TermValue::Literal { lexical_form, .. } if lexical_form == "|a|é|")
            );
        },
    );
    bounded_matrix(
        r#"DESCRIBE <https://example.org/target>
        WHERE { FILTER(REGEX("a\n", "^$", "m")) }"#,
        |result| {
            assert!(result.graph().unwrap().quad_count() > 0);
        },
    );
}
