// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SPARQL 1.2 RL text, end to end through the public API (`purrdf_shapes::srl`): the
//! grammar's corners the W3C suite does not reach, the evaluation semantics each lowering
//! decision must honour, and imports.
//!
//! Every refusal is paired with a valid neighbour whose observable differs.

#[path = "support/terms.rs"]
mod terms;

use std::fmt::Write as _;
use std::sync::Arc;
use terms::example_org as iri;
use terms::integer as int;

use purrdf::RdfDataset;
use purrdf_shapes::srl::{self, InferOptions, SrlError};
use purrdf_shapes::term::{NamedNode, Term};

const EX: &str = "http://example.org/";

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

/// Production [2]'s intended reading: after the initial prologue, declarations and
/// rule or data blocks interleave freely. Two rule blocks and a data block after a
/// declaration that follows a rule all parse, and every rule runs — each one's own
/// inferred triple appears, over the base data and over the data block's triple.
#[test]
fn declarations_and_rule_blocks_interleave_freely() {
    let inferred = infer(
        "RULE { ?x :r1 :yes } WHERE { ?x :p ?y }
         PREFIX a: <http://example.org/a#>
         RULE { ?x a:r2 :yes } WHERE { ?x :p ?y }
         RULE { ?x :r3 :yes } WHERE { ?x :p ?y }
         DATA { :d :p :e }",
        ":s :p :o .",
    )
    .expect("RULE PREFIX RULE RULE DATA parses, checks and runs");
    let yes = iri("yes");
    let r2 = Term::NamedNode(NamedNode::from("http://example.org/a#r2"));
    for subject in ["s", "d"] {
        for predicate in [iri("r1"), r2.clone(), iri("r3")] {
            let triple = [iri(subject), predicate, yes.clone()];
            assert!(
                inferred.contains(&triple),
                "{triple:?} missing from {inferred:?}"
            );
        }
    }
    // The literal reading's own shape still parses.
    srl::parse(
        &rules("RULE {} WHERE {} RULE {} WHERE {} PREFIX a: <http://example.org/a#> RULE {} WHERE {} PREFIX b: <http://example.org/b#> DATA {}"),
        None,
    )
    .expect("RuleOrData+ ( Prologue1 RuleOrData? )* parses");
}

/// The neighbours the interleaving does not admit: a declaration is not a rule
/// element, and a rule needs its `WHERE` body.
#[test]
fn a_declaration_inside_a_rule_or_a_bodiless_rule_is_refused() {
    let message =
        syntax_error("RULE { ?x :r :yes } WHERE { PREFIX a: <http://example.org/a#> ?x :p ?y }");
    assert!(message.contains("found `PREFIX`"), "{message}");
    let message = syntax_error(
        "RULE { ?x :r :yes } PREFIX a: <http://example.org/a#> RULE { ?x :r :yes } WHERE { ?x :p ?y }",
    );
    assert!(message.contains("WHERE"), "{message}");
    let message = syntax_error("RULE { ?x :r :yes }");
    assert!(message.contains("WHERE"), "{message}");
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

/// §7.1: "The version announcement SHOULD be made early in the document", read as a
/// must: a first `VERSION` after a `RULE` or `DATA` block is refused at the syntax stage.
/// The neighbours load: the same announcement before the block, a later second
/// `VERSION` after an early first one (each directive applies to what follows it), and
/// a document that announces no version at all — and the early one's rule is observed
/// inferring.
#[test]
fn the_version_announcement_is_made_early() {
    for late in [
        "DATA { :a :p :o }\nVERSION \"1.2\"",
        "RULE { ?x :q ?y } WHERE { ?x :p ?y }\nVERSION \"1.2\"\nDATA { :a :p :o }",
    ] {
        let message = syntax_error(late);
        assert!(message.contains("SHOULD be made early"), "{message}");
    }
    let early = srl::parse(
        &rules("VERSION \"1.2\"\nRULE { ?x :q ?y } WHERE { ?x :p ?y }\nDATA { :a :p :o }"),
        None,
    )
    .expect("an early announcement parses");
    assert_eq!(early.versions(), ["1.2"]);
    let again = srl::parse(
        &rules("VERSION \"1.2\"\nDATA { :a :p :o }\nVERSION \"1.2\"\nDATA { :b :p :o }"),
        None,
    )
    .expect("a second announcement after an early one parses");
    assert_eq!(again.versions(), ["1.2", "1.2"]);
    srl::parse(&rules("DATA { :a :p :o }"), None).expect("no announcement parses");
    let inferred = infer(
        "VERSION \"1.2\"\nRULE { ?x :q ?y } WHERE { ?x :p ?y }",
        "<http://example.org/a> <http://example.org/p> <http://example.org/o> .",
    )
    .expect("the early announcement's rule runs");
    assert_eq!(inferred.len(), 1, "{inferred:?}");
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

// ── the check-only entry point ────────────────────────────────────────────────────

/// `srl::check` answers each conformance question up to its level and evaluates nothing:
/// a self-negating rule is syntactically valid and well formed but not stratifiable, so it
/// passes the first two levels and is refused at the third, by the stratification stage —
/// while its stratifiable neighbour, differing only in the negated predicate, passes all
/// three and names its strata.
#[test]
fn check_answers_each_level_and_refuses_by_stage() {
    use srl::CheckLevel;
    let cyclic = rules("RULE { ?x :p :z } WHERE { ?x :q :o NOT { ?x :p :z } }");
    let acyclic = rules("RULE { ?x :p :z } WHERE { ?x :q :o NOT { ?x :r :z } }");
    for level in [CheckLevel::Syntax, CheckLevel::WellFormed] {
        let checked = srl::check(&cyclic, None, &[], level).expect("below stratification");
        assert_eq!(checked.level(), level);
        assert!(checked.strata().is_none(), "{level} does not stratify");
    }
    let refused = srl::check(&cyclic, None, &[], CheckLevel::Stratified)
        .expect_err("a closed self-dependency");
    assert!(
        matches!(refused, SrlError::Stratification { .. }),
        "{refused}"
    );

    let checked = srl::check(&acyclic, None, &[], CheckLevel::default()).expect("stratifiable");
    assert_eq!(checked.level(), CheckLevel::Stratified);
    assert_eq!(checked.strata().map(<[_]>::len), Some(1));
    assert_eq!(
        checked.summary(),
        "SPARQL 1.2 RL rule set is well formed and stratified (level stratified): 1 rule, \
         0 data triples, 0 imported rule sets, 1 stratum, no VERSION"
    );

    // An ill-formed rule is refused at `well-formed` and above, by that stage; the grammar
    // alone accepts it.
    let ill = rules("RULE { ?s :p ?o } WHERE { FILTER(?o < 50) ?s :p ?o }");
    srl::check(&ill, None, &[], CheckLevel::Syntax).expect("the grammar accepts it");
    for level in [CheckLevel::WellFormed, CheckLevel::Stratified] {
        let error = srl::check(&ill, None, &[], level).expect_err("ill formed");
        assert!(matches!(error, SrlError::WellFormedness { .. }), "{error}");
    }
    // Not a document at all: refused by the grammar at every level.
    for level in CheckLevel::ALL {
        let error = srl::check(&rules("RULE {"), None, &[], level).expect_err("not SRL");
        assert!(matches!(error, SrlError::Syntax { .. }), "{error}");
    }
    for level in CheckLevel::ALL {
        assert_eq!(CheckLevel::from_name(level.name()), Some(level));
    }
    assert_eq!(CheckLevel::from_name("stratify"), None);
}

/// `srl::check` resolves the `IMPORTS` closure from the table exactly as a rules run does,
/// holds an imported rule to §4.2 like the importer's own, and names what it read.
#[test]
fn check_resolves_imports_from_the_table() {
    use srl::CheckLevel;
    let main = format!(
        "PREFIX : <{EX}>\nVERSION \"1.2\"\nIMPORTS <{EX}lib>\nDATA {{ :a :p :o }}\n\
         RULE {{ ?s :tagged :yes }} WHERE {{ ?s :p :o }}"
    );
    let lib = format!("PREFIX : <{EX}>\nRULE {{ ?s :seen :yes }} WHERE {{ ?s :tagged :yes }}");
    let lib_iri = format!("{EX}lib");

    let unresolved = srl::check(&main, None, &[], CheckLevel::Stratified)
        .expect_err("the empty table resolves nothing");
    assert!(
        matches!(unresolved, SrlError::Import { .. }),
        "{unresolved}"
    );

    let checked =
        srl::check(&main, None, &[(&lib_iri, &lib)], CheckLevel::Stratified).expect("resolved");
    assert_eq!(checked.imported(), std::slice::from_ref(&lib_iri));
    assert_eq!(checked.document().rules().len(), 2);
    assert_eq!(checked.document().imports(), &[] as &[NamedNode]);
    assert_eq!(
        checked.summary(),
        "SPARQL 1.2 RL rule set is well formed and stratified (level stratified): 2 rules, \
         1 data triple, 1 imported rule set, 1 stratum, VERSION \"1.2\""
    );
    // The checked document is the one a rules run evaluates.
    let inferred = srl::infer(checked.document(), &data(""), &InferOptions::default())
        .expect("evaluates")
        .inferred()
        .to_vec();
    assert!(
        inferred.contains(&[iri("a"), iri("seen"), iri("yes")]),
        "{inferred:?}"
    );

    // An ill-formed IMPORTED rule is refused at `well-formed`, naming its document.
    let bad_lib = format!("PREFIX : <{EX}>\nRULE {{ ?s :seen ?c }} WHERE {{ ?s :tagged :yes }}");
    let error = srl::check(&main, None, &[(&lib_iri, &bad_lib)], CheckLevel::WellFormed)
        .expect_err("the imported rule is ill formed");
    assert!(
        matches!(&error, SrlError::WellFormedness { rule, .. } if rule.contains(&lib_iri)),
        "{error}"
    );
    // A table entry the closure never reaches is refused as unused.
    let other = format!("{EX}other");
    let error = srl::check(
        &main,
        None,
        &[(&lib_iri, &lib), (&other, &lib)],
        CheckLevel::Syntax,
    )
    .expect_err("unused entry");
    assert!(
        matches!(error, SrlError::UnreachedImports { .. }),
        "{error}"
    );
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

// ── rule-evaluation limits ────────────────────────────────────────────────────────

/// A general rule that nests the triple term it matched inside a new one every round
/// never terminates. Under a caller's round limit it is refused with the typed
/// [`SrlError::LimitExceeded`], naming the limit, the numbers and the rule by its
/// document position; under the defaults it is refused too, by name, never as
/// "divergent".
#[test]
fn a_rule_nesting_triple_terms_every_round_is_refused_at_a_limit() {
    let document = srl::parse_and_check(
        &rules("RULE :nest { ?x :p <<( ?x :p ?y )>> } WHERE { ?x :p ?y }"),
        None,
    )
    .expect("checks");
    let fixed = srl::infer(
        &document,
        &data(":a :p :b ."),
        &InferOptions::default().with_max_term_generating_rounds(8),
    )
    .expect_err("past the stated limit");
    let SrlError::LimitExceeded(limit) = &fixed else {
        panic!("expected a typed limit refusal, got {fixed}");
    };
    assert_eq!(
        limit.limit(),
        purrdf_shapes::RuleLimit::TermGeneratingRounds
    );
    assert_eq!(limit.observed(), 9);
    assert_eq!(limit.permitted(), 8);
    assert!(limit.stated());
    assert_eq!(limit.rules().len(), 1);
    assert!(
        limit.rules()[0].starts_with(&format!("rule <{EX}nest>")),
        "{limit}"
    );
    assert!(
        fixed
            .to_string()
            .ends_with("raise the limit with InferOptions::with_max_term_generating_rounds"),
        "{fixed}"
    );
    let default = infer(
        "RULE :nest { ?x :p <<( ?x :p ?y )>> } WHERE { ?x :p ?y }",
        ":a :p :b .",
    )
    .expect_err("the nesting never ends");
    assert!(!default.to_string().contains("diverge"), "{default}");
    assert!(default.to_string().contains("exceeded the"), "{default}");
}

/// The neighbour: a recursive rule over a long chain infers no term the store did not
/// hold — every `:connected` object is a chain node — so no term-generating round is
/// counted, and the closure completes in full.
#[test]
fn a_closure_deeper_than_the_floor_completes() {
    const LENGTH: usize = 300;
    let mut ttl = String::new();
    for index in 0..LENGTH {
        writeln!(ttl, ":n{index} :link :n{} .", index + 1).expect("write to String");
    }
    let inferred = infer(
        "RULE { ?x :connected ?y } WHERE { ?x :link ?y }
         RULE { ?x :connected ?z } WHERE { ?x :link ?y . ?y :connected ?z }",
        &ttl,
    )
    .expect("a closure over a long chain completes");
    assert_eq!(inferred.len(), LENGTH * (LENGTH + 1) / 2);
    assert!(inferred.contains(&[iri("n0"), iri("connected"), iri(&format!("n{LENGTH}"))]));
}
