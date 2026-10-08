// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native/wasm execution of connected and genuinely independent production rules.

use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause};
use purrdf_datalog::guard::{Guard, GuardCall, GuardEvaluator};
use purrdf_datalog::seminaive::{EvalOptions, evaluate_guarded};
use purrdf_datalog::seminaive::{compile, evaluate};
use purrdf_datalog::store::RelationStore;

struct Distinct;

impl GuardEvaluator for Distinct {
    fn evaluate(&self, call: &GuardCall<'_>) -> Result<Vec<Vec<String>>, String> {
        assert_eq!(call.inputs.len(), 2);
        Ok(if call.inputs[0] == call.inputs[1] {
            Vec::new()
        } else {
            vec![Vec::new()]
        })
    }
}

fn later_partial_delta_keeps_both_productive_anchors() {
    let atom = |s: &str, p: &str, o: &str| {
        ClauseAtom::positive(
            ClauseTerm::var(s),
            format!("https://example.org/{p}"),
            ClauseTerm::var(o),
        )
    };
    let program = compile(vec![
        DlClause::datalog(atom("?x", "p", "?y"), vec![atom("?x", "a", "?y")]),
        DlClause::datalog(atom("?y", "q", "?z"), vec![atom("?y", "b", "?z")]),
        DlClause::datalog(
            atom("?x", "r", "?z"),
            vec![atom("?x", "p", "?y"), atom("?y", "q", "?z")],
        ),
    ])
    .expect("partial-delta fixture compiles");
    let mut input = RelationStore::new();
    for (s, p, o) in [
        ("a", "p", "b"),
        ("b", "q", "c"),
        ("d", "a", "b"),
        ("b", "b", "e"),
    ] {
        input.insert(
            &format!("<https://example.org/{s}>"),
            &format!("<https://example.org/{p}>"),
            &format!("<https://example.org/{o}>"),
            RelationStore::DEFAULT_GRAPH,
        );
    }
    let evaluation = evaluate(&program, input).expect("later productive anchors complete");
    let proofs: Vec<_> = evaluation
        .derivations()
        .iter()
        .filter(|proof| proof.rule() == 2)
        .collect();
    assert_eq!(proofs.len(), 4);
    for (s, o) in [("a", "c"), ("d", "c"), ("a", "e"), ("d", "e")] {
        let proof = proofs
            .iter()
            .find(|proof| {
                proof.fact().subject == format!("<https://example.org/{s}>")
                    && proof.fact().object == format!("<https://example.org/{o}>")
            })
            .expect("all old/new combinations survive");
        assert_eq!(proof.sources().len(), 2);
        assert_eq!(proof.sources()[0].predicate, "<https://example.org/p>");
        assert_eq!(proof.sources()[1].predicate, "<https://example.org/q>");
        assert_eq!(proof.sources()[0].subject, proof.fact().subject);
        assert_eq!(proof.sources()[0].object, proof.sources()[1].subject);
        assert_eq!(proof.sources()[1].object, proof.fact().object);
    }
}

fn original_guarded_pair_has_exact_connected_work_and_authored_proofs() {
    const NS: &str = "https://example.invalid/k#";
    let atom = |s: ClauseTerm, p: &str, o: ClauseTerm| ClauseAtom::positive(s, p, o);
    let kind = purrdf_core::vocab::rdf::TYPE;
    let target = format!("{NS}target");
    let rule = DlClause::datalog(
        atom(
            ClauseTerm::var("?v"),
            &format!("{NS}hasTwoSpools"),
            ClauseTerm::literal(format!(
                "\"true\"^^<{}>",
                purrdf_core::datatype::XSD_BOOLEAN
            )),
        ),
        vec![
            atom(
                ClauseTerm::var("?v"),
                kind,
                ClauseTerm::iri(format!("{NS}Vault")),
            ),
            atom(ClauseTerm::var("?v"), &target, ClauseTerm::var("?s1")),
            atom(ClauseTerm::var("?v"), &target, ClauseTerm::var("?s2")),
            atom(
                ClauseTerm::var("?s1"),
                kind,
                ClauseTerm::iri(format!("{NS}Spool")),
            ),
            atom(
                ClauseTerm::var("?s2"),
                kind,
                ClauseTerm::iri(format!("{NS}Spool")),
            ),
        ],
    )
    .with_guards(vec![Guard::filter(
        "?s1 != ?s2",
        vec!["?s1".into(), "?s2".into()],
    )]);
    let program = compile(vec![rule]).expect("original guarded pair compiles");
    for two in [false, true] {
        let mut input = RelationStore::new();
        for i in 0..100 {
            let v = format!("<{NS}v{i}>");
            input.insert(
                &v,
                &format!("<{kind}>"),
                &format!("<{NS}Vault>"),
                RelationStore::DEFAULT_GRAPH,
            );
            for suffix in ['a', 'b', 'c'] {
                let s = format!("<{NS}v{i}{suffix}>");
                input.insert(&v, &format!("<{target}>"), &s, RelationStore::DEFAULT_GRAPH);
                let class = if suffix == 'c' || (two && suffix == 'b') {
                    "Spool"
                } else {
                    "Store"
                };
                input.insert(
                    &s,
                    &format!("<{kind}>"),
                    &format!("<{NS}{class}>"),
                    RelationStore::DEFAULT_GRAPH,
                );
            }
        }
        let evaluation =
            evaluate_guarded(&program, input, &Distinct, &EvalOptions::default(), None)
                .expect("unchanged defaults admit the original pair");
        assert_eq!(evaluation.derivations().len(), if two { 100 } else { 0 });
        // The whole frozen store is initially delta, so only the last anchor
        // can have a nonempty OldOnly suffix. Its five connected expansions
        // cost (1+3+1+3+1)N, or (1+3+2+6+4)N with two Spools. The guard's
        // two distinct assignments additionally charge 2N emitted heads.
        assert_eq!(
            evaluation.budget().join_steps(),
            if two { 1800 } else { 900 }
        );
        for proof in evaluation.derivations() {
            let sources = proof.sources();
            assert_eq!(sources.len(), 5);
            assert_eq!(proof.fact().predicate, format!("<{NS}hasTwoSpools>"));
            assert_eq!(
                proof.fact().object,
                format!("\"true\"^^<{}>", purrdf_core::datatype::XSD_BOOLEAN)
            );
            assert_eq!(sources[0].predicate, format!("<{kind}>"));
            assert_eq!(sources[1].predicate, format!("<{target}>"));
            assert_eq!(sources[2].predicate, format!("<{target}>"));
            assert_eq!(sources[3].predicate, format!("<{kind}>"));
            assert_eq!(sources[4].predicate, format!("<{kind}>"));
            let v = &proof.fact().subject;
            let stem = v.strip_suffix('>').expect("IRI subject");
            assert_eq!(sources[0].subject, *v);
            assert_eq!(sources[0].object, format!("<{NS}Vault>"));
            assert_eq!(sources[1].subject, *v);
            assert_eq!(sources[2].subject, *v);
            assert_eq!(sources[1].object, format!("{stem}b>"));
            assert_eq!(sources[2].object, format!("{stem}c>"));
            assert_eq!(sources[3].subject, sources[1].object);
            assert_eq!(sources[4].subject, sources[2].object);
            assert_eq!(sources[3].object, format!("<{NS}Spool>"));
            assert_eq!(sources[4].object, format!("<{NS}Spool>"));
        }
    }
}

fn connected_rule_restores_all_authored_premises() {
    let atom = |s: ClauseTerm, p: &str, o: ClauseTerm| ClauseAtom::positive(s, p, o);
    let a = "https://example.org/a";
    let edge = "https://example.org/edge";
    let b = "https://example.org/b";
    let program = compile(vec![DlClause::datalog(
        atom(
            ClauseTerm::var("?x"),
            "https://example.org/result",
            ClauseTerm::var("?y"),
        ),
        vec![
            atom(
                ClauseTerm::var("?x"),
                a,
                ClauseTerm::iri("https://example.org/yes"),
            ),
            atom(ClauseTerm::var("?x"), edge, ClauseTerm::var("?y")),
            atom(
                ClauseTerm::var("?y"),
                b,
                ClauseTerm::iri("https://example.org/yes"),
            ),
        ],
    )])
    .expect("connected fixture compiles");
    let mut input = RelationStore::new();
    for i in 0..100 {
        let x = format!("<https://example.org/x{i}>");
        let y = format!("<https://example.org/y{i}>");
        input.insert(
            &x,
            &format!("<{a}>"),
            "<https://example.org/yes>",
            RelationStore::DEFAULT_GRAPH,
        );
        input.insert(&x, &format!("<{edge}>"), &y, RelationStore::DEFAULT_GRAPH);
        input.insert(
            &y,
            &format!("<{b}>"),
            "<https://example.org/yes>",
            RelationStore::DEFAULT_GRAPH,
        );
    }
    let evaluation =
        evaluate(&program, input).expect("unchanged target defaults admit linear connected work");
    assert_eq!(evaluation.derivations().len(), 100);
    // All seeds are new in round one. The first two decompositions have empty
    // OldOnly suffixes and are skipped; Full/Full/Delta admits 3N frames.
    // Round two's result delta addresses none of the three body predicates.
    assert_eq!(evaluation.budget().join_steps(), 300);
    for proof in evaluation.derivations() {
        let sources = proof.sources();
        assert_eq!(sources.len(), 3);
        assert_eq!(sources[0].predicate, format!("<{a}>"));
        assert_eq!(sources[1].predicate, format!("<{edge}>"));
        assert_eq!(sources[2].predicate, format!("<{b}>"));
        assert_eq!(sources[0].subject, proof.fact().subject);
        assert_eq!(sources[1].subject, proof.fact().subject);
        assert_eq!(sources[1].object, proof.fact().object);
        assert_eq!(sources[2].subject, proof.fact().object);
    }
}

fn independent_head_projects_without_cartesian_work() {
    let atom = |v: &str, p: &str| {
        ClauseAtom::positive(
            ClauseTerm::var(v),
            p,
            ClauseTerm::iri("https://example.org/yes"),
        )
    };
    let program = compile(vec![DlClause::datalog(
        atom("?x", "https://example.org/result"),
        vec![
            atom("?y", "https://example.org/b"),
            atom("?x", "https://example.org/a"),
        ],
    )])
    .expect("independent fixture compiles");
    let mut input = RelationStore::new();
    for i in 0..100 {
        for p in ["a", "b"] {
            input.insert(
                &format!("<https://example.org/x{i:03}>"),
                &format!("<https://example.org/{p}>"),
                "<https://example.org/yes>",
                RelationStore::DEFAULT_GRAPH,
            );
        }
    }
    let evaluation =
        evaluate(&program, input).expect("unchanged target defaults admit additive factors");
    assert_eq!(evaluation.derivations().len(), 100);
    assert_eq!(evaluation.budget().join_steps(), 300);
    for proof in evaluation.derivations() {
        assert_eq!(proof.sources().len(), 2);
        assert_eq!(proof.sources()[0].subject, "<https://example.org/x000>");
        assert_eq!(proof.sources()[0].predicate, "<https://example.org/b>");
        assert_eq!(proof.sources()[1].subject, proof.fact().subject);
        assert_eq!(proof.sources()[1].predicate, "<https://example.org/a>");
    }
}

purrdf_testkit::harness_main!(
    later_partial_delta_keeps_both_productive_anchors,
    original_guarded_pair_has_exact_connected_work_and_authored_proofs,
    connected_rule_restores_all_authored_premises,
    independent_head_projects_without_cartesian_work
);
