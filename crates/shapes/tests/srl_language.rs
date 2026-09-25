// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SPARQL 1.2 RL text, end to end through the public API (`purrdf_shapes::srl`): the
//! grammar's corners the W3C suite does not reach, the evaluation semantics each lowering
//! decision must honour, and imports.
//!
//! Every refusal is paired with a valid neighbour whose observable differs.

use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::srl::{self, InferOptions, SrlError};
use purrdf_shapes::term::{Literal, NamedNode, Term};

const EX: &str = "http://example.org/";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

fn iri(local: &str) -> Term {
    Term::NamedNode(NamedNode::from(format!("{EX}{local}").as_str()))
}

fn int(value: i64) -> Term {
    Term::Literal(Literal::new_typed_literal(
        value.to_string(),
        NamedNode::from(XSD_INTEGER),
    ))
}

fn data(ttl: &str) -> Arc<RdfDataset> {
    purrdf::parse_dataset(
        format!("@prefix : <{EX}> .\n{ttl}").as_bytes(),
        "text/turtle",
        None,
    )
    .expect("data parses")
}

fn rules(text: &str) -> String {
    format!("PREFIX : <{EX}>\n{text}")
}

/// Parse, check and infer; the inferred triples.
fn infer(rule_text: &str, ttl: &str) -> Result<Vec<[Term; 3]>, SrlError> {
    let document = srl::parse_and_check(&rules(rule_text), None)?;
    srl::infer(&document, &data(ttl), &InferOptions::default())
        .map(|inference| inference.inferred().to_vec())
}

fn syntax_error(text: &str) -> String {
    match srl::parse(&rules(text), None) {
        Err(SrlError::Syntax { message, .. }) => message,
        other => panic!("expected a syntax error for {text:?}, got {other:?}"),
    }
}

// ── grammar ───────────────────────────────────────────────────────────────────────

/// `':='`, `'<<('` and `')>>'` are single terminals: their two halves must touch.
#[test]
fn multi_character_terminals_must_be_contiguous() {
    srl::parse(&rules("RULE { :s :p ?x } WHERE { SET(?x := 1) }"), None).expect(":= parses");
    assert!(syntax_error("RULE { :s :p ?x } WHERE { SET(?x : = 1) }").contains(":="));
    srl::parse(&rules("RULE {} WHERE { ?s :p <<( :a :b :c )>> }"), None).expect("<<( parses");
    assert!(
        syntax_error("RULE {} WHERE { ?s :p <<( :a :b :c ) >> }").contains(")>>"),
        "a split `)>>` is not the terminal"
    );
}

/// Production [2] read as written: after a declaration that follows a rule or data
/// block, one rule or data block may follow before the next declaration.
#[test]
fn rule_or_data_block_production_is_read_as_written() {
    srl::parse(
        &rules("RULE {} WHERE {} RULE {} WHERE {} PREFIX a: <http://example.org/a#> RULE {} WHERE {} PREFIX b: <http://example.org/b#> DATA {}"),
        None,
    )
    .expect("RuleOrData+ ( Prologue1 RuleOrData? )* parses");
    let message = syntax_error(
        "RULE {} WHERE {} PREFIX a: <http://example.org/a#> RULE {} WHERE {} RULE {} WHERE {}",
    );
    assert!(message.contains("[2]"), "{message}");
}

/// SRL's `BuiltInCall` is its own list: SPARQL-only built-ins are refused, their SRL
/// neighbours accepted.
#[test]
fn builtins_are_srls_list() {
    for refused in [
        "COALESCE(?x, 1)",
        "BOUND(?x)",
        "RAND()",
        "MD5(?x)",
        "EXISTS { ?x :p ?y }",
    ] {
        let message = syntax_error(&format!("RULE {{}} WHERE {{ ?x :p ?y FILTER({refused}) }}"));
        assert!(
            message.contains("not a SPARQL 1.2 RL built-in"),
            "{refused}: {message}"
        );
    }
    for accepted in [
        "IF(?x, 1, 2)",
        "sameTerm(?x, ?y)",
        "CONCAT()",
        "CONCAT(?x, \"a\")",
        "BNODE()",
        "BNODE(\"a\")",
        "SUBSTR(?x, 1)",
        "SUBSTR(?x, 1, 2)",
        "?y NOT IN (1, 2)",
        "<<( \"lit\" :p ?y )>>",
        ":f(?x)",
    ] {
        srl::parse(
            &rules(&format!(
                "RULE {{}} WHERE {{ ?x :p ?y FILTER({accepted}) }}"
            )),
            None,
        )
        .unwrap_or_else(|e| panic!("{accepted}: {e}"));
    }
    assert!(syntax_error("RULE {} WHERE { ?x :p ?y FILTER(STR()) }").contains("STR"));
    assert!(syntax_error("RULE {} WHERE { ?x :p ?y FILTER(NOW(?x)) }").contains("NOW"));
}

/// A property path is sequences and inverses; an annotation names one triple, so it
/// may follow a one-step path but not a longer one.
#[test]
fn paths_expand_and_annotations_need_one_triple() {
    let document = srl::parse(&rules("RULE {} WHERE { ?x ^:p/:q ?y }"), None).expect("parses");
    assert_eq!(
        document.rules()[0].rule().body.len(),
        2,
        "two expanded patterns"
    );
    assert!(syntax_error("RULE {} WHERE { ?x :p* ?y }").contains("path"));
    assert!(syntax_error("RULE {} WHERE { ?x :p/:q ?y {| :r :z |} }").contains("ONE triple"));
    let document = srl::parse(&rules("RULE {} WHERE { ?x ^:p ?y {| :r :z |} }"), None)
        .expect("a one-step inverse path is one triple");
    assert_eq!(document.rules()[0].rule().body.len(), 3);
}

/// A language tag's direction is `ltr` or `rtl`; `VERSION` labels are recorded.
#[test]
fn lang_dir_and_version() {
    srl::parse(&rules("DATA { :s :p \"a\"@en--rtl }"), None).expect("rtl parses");
    assert!(syntax_error("DATA { :s :p \"a\"@en--up }").contains("--up"));
    let document = srl::parse(&format!("VERSION \"1.2\" {}", rules("DATA {}")), None).expect("ok");
    assert_eq!(document.versions(), ["1.2"]);
    assert!(syntax_error("VERSION \"\"\"1.2\"\"\" DATA {}").contains("VERSION"));
}

// ── evaluation semantics ──────────────────────────────────────────────────────────

/// A negation element sees only the variables of the elements BEFORE it: a variable a
/// later pattern binds is a local variable inside it.
#[test]
fn a_negation_sees_only_earlier_variables() {
    let before = infer(
        "RULE { ?s :r :z } WHERE { NOT { ?s :q ?o } ?s :p ?o }",
        ":a :p 1 . :b :q 2 .",
    )
    .expect("evaluates");
    assert!(
        before.is_empty(),
        "some ?s :q ?o exists, so NOT blocks: {before:?}"
    );
    let after = infer(
        "RULE { ?s :r :z } WHERE { ?s :p ?o NOT { ?s :q ?o } }",
        ":a :p 1 . :b :q 2 .",
    )
    .expect("evaluates");
    assert_eq!(after, [[iri("a"), iri("r"), iri("z")]]);
}

/// An assignment's variable may be used by a later pattern: the pattern must match the
/// assigned value.
#[test]
fn an_assigned_variable_joins_a_later_pattern() {
    let inferred = infer(
        "RULE { :x :found ?p } WHERE { SET(?x := 1) :s ?p ?x }",
        ":s :p 1 . :s :q 2 .",
    )
    .expect("evaluates");
    assert_eq!(inferred, [[iri("x"), iri("found"), iri("p")]]);
}

/// `BNODE()` in an assignment is fresh per solution, and fresh against the base graph.
#[test]
fn an_assigned_blank_node_is_fresh_per_solution() {
    let inferred = infer(
        "RULE { ?b :of ?s } WHERE { ?s :p ?o SET(?b := BNODE()) }",
        ":a :p 1 . :c :p 2 . _:bnode1 :p 3 .",
    )
    .expect("evaluates");
    assert_eq!(inferred.len(), 3, "{inferred:?}");
    let mut blanks: Vec<&Term> = inferred.iter().map(|[b, _, _]| b).collect();
    blanks.sort_by_key(ToString::to_string);
    blanks.dedup();
    assert_eq!(
        blanks.len(),
        3,
        "one fresh blank node per solution: {inferred:?}"
    );
    assert!(blanks.iter().all(|b| matches!(b, Term::BlankNode(_))));
    assert!(
        !inferred.iter().any(|[b, _, _]| b.to_string() == "_:bnode1"),
        "{inferred:?}"
    );
}

/// A data block's blank node is not the base graph's blank node of the same label.
#[test]
fn data_block_blank_nodes_are_standardized_apart() {
    let rule = "RULE { ?s :both :yes } WHERE { ?s :p :o . ?s :q :o }";
    let apart = infer(&format!("DATA {{ _:b :q :o }} {rule}"), "_:b :p :o .").expect("ok");
    assert!(
        !apart.iter().any(|[_, p, _]| *p == iri("both")),
        "{apart:?}"
    );
    let together = infer(&format!("DATA {{ :n :q :o }} {rule}"), ":n :p :o .").expect("ok");
    assert!(together.contains(&[iri("n"), iri("both"), iri("yes")]));
}

/// A head instantiated into something that is not an RDF triple is refused by name;
/// the neighbour whose binding is an IRI evaluates.
#[test]
fn a_non_rdf_head_instantiation_is_refused() {
    let rule = "RULE { ?o :inverse ?s } WHERE { ?s :p ?o }";
    let error = infer(rule, ":a :p \"lit\" .").expect_err("literal subject");
    assert!(
        matches!(&error, SrlError::Evaluation { message } if message.contains("not an RDF triple")),
        "{error}"
    );
    assert_eq!(
        infer(rule, ":a :p :b .").expect("ok"),
        [[iri("b"), iri("inverse"), iri("a")]]
    );
}

/// Dependencies unify through triple terms: a negation over a triple term the head can
/// never build is no dependency, and the neighbour that can build it is refused.
#[test]
fn triple_term_dependencies_are_exact() {
    let head = "RULE { ?x :p <<( ?a :b :c )>> } WHERE { ?x :q ?a . ";
    let inferred = infer(
        &format!("{head} NOT {{ ?x :p <<( :x :y :z )>> }} }}"),
        ":s :q :a .",
    )
    .expect("stratifiable");
    assert_eq!(inferred.len(), 1, "{inferred:?}");
    let error = srl::parse_and_check(
        &rules(&format!("{head} NOT {{ ?x :p <<( :x :b :c )>> }} }}")),
        None,
    )
    .expect_err("a closed self-dependency");
    assert!(matches!(error, SrlError::Stratification { .. }), "{error}");
}

/// `RULE {} WHERE { … }` generates nothing, and a neighbouring rule still runs.
#[test]
fn an_empty_head_generates_nothing() {
    let inferred = infer(
        "RULE {} WHERE { ?s :p ?o } RULE { ?s :r ?o } WHERE { ?s :p ?o }",
        ":a :p :b .",
    )
    .expect("evaluates");
    assert_eq!(inferred, [[iri("a"), iri("r"), iri("b")]]);
}

/// Stages are typed: a well-formedness error is not a syntax error.
#[test]
fn stages_are_typed() {
    let bad = rules("RULE { ?s :p ?o } WHERE { FILTER(?o < 50) ?s :p ?o }");
    let document = srl::parse(&bad, None).expect("the grammar accepts it");
    assert!(matches!(
        document.check_well_formed(),
        Err(SrlError::WellFormedness { .. })
    ));
    assert!(matches!(
        srl::parse_and_check(&bad, None),
        Err(SrlError::WellFormedness { .. })
    ));
    let strata = srl::parse(
        &rules("RULE { ?s :q :z } WHERE { ?s :p :o NOT { ?s :r :o } } RULE { ?s :r :o } WHERE { ?s :t :o }"),
        None,
    )
    .expect("parses")
    .stratify()
    .expect("stratifiable");
    assert_eq!(strata.len(), 2);
    assert_eq!(strata[0].general, [1]);
    assert_eq!(strata[1].general, [0]);
}

// ── imports ───────────────────────────────────────────────────────────────────────

/// Imports resolve through the caller's resolver, each once, their data merged apart;
/// an unresolved or declined import is an import error.
#[test]
fn imports_resolve_through_the_callers_resolver() {
    let main = format!(
        "PREFIX : <{EX}>\nIMPORTS <{EX}lib>\nDATA {{ _:b :p :o }}\nRULE {{ ?s :tagged :yes }} WHERE {{ ?s :p :o }}"
    );
    let lib = format!(
        "PREFIX : <{EX}>\nIMPORTS <{EX}main>\nDATA {{ _:b :p :o }}\nRULE {{ ?s :seen :yes }} WHERE {{ ?s :tagged :yes }}"
    );
    let document = srl::parse_and_check(&main, Some(&format!("{EX}main"))).expect("parses");
    assert_eq!(document.imports().len(), 1);
    let unresolved = srl::infer(&document, &data(""), &InferOptions::default())
        .expect_err("imports are not resolved");
    assert!(
        matches!(unresolved, SrlError::Import { .. }),
        "{unresolved}"
    );

    let mut reads = 0usize;
    let mut resolver = |iri: &str| -> Result<String, String> {
        reads += 1;
        if iri == format!("{EX}lib") {
            Ok(lib.clone())
        } else {
            Err(format!("no document at {iri}"))
        }
    };
    let resolved = document.resolve_imports(&mut resolver).expect("resolves");
    assert_eq!(
        reads, 1,
        "the import of the importer's own location is not read"
    );
    assert_eq!(resolved.rules().len(), 2);
    let inferred = srl::infer(&resolved, &data(""), &InferOptions::default())
        .expect("evaluates")
        .inferred()
        .to_vec();
    let seen = inferred
        .iter()
        .filter(|[_, p, _]| *p == iri("seen"))
        .count();
    assert_eq!(
        seen, 2,
        "two data-block blank nodes, standardized apart: {inferred:?}"
    );

    let mut declining = |_: &str| -> Result<String, String> { Err("not supported".to_owned()) };
    assert!(matches!(
        document.resolve_imports(&mut declining),
        Err(SrlError::Import { .. })
    ));
    let mut broken = |_: &str| -> Result<String, String> { Ok("RULE".to_owned()) };
    assert!(matches!(
        document.resolve_imports(&mut broken),
        Err(SrlError::Import { .. })
    ));
}

#[test]
fn numeric_and_boolean_terms() {
    let inferred = infer(
        "DATA { :s :n -1, +2, true } RULE { :s :sum ?t } WHERE { :s :n ?a . :s :n ?b FILTER(?a < ?b) SET(?t := ?a + ?b) }",
        "",
    )
    .expect("evaluates");
    assert!(
        inferred.contains(&[iri("s"), iri("sum"), int(1)]),
        "{inferred:?}"
    );
}
