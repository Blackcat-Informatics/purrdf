// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Sourced admission through actual compiler, scheduled, chase and cache doors.

use purrdf_datalog::admission::{
    AdmissionRefusal, DeclarationLayer, DeclarationSource, DerivationStep, NeverDeriveDeclarations,
};
use purrdf_datalog::cache::{PlanCache, PlanIdentity};
use purrdf_datalog::chase::{ChaseError, chase_with_declarations};
use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct};
use purrdf_datalog::guard::{Guard, GuardReads, Negation, NoGuards};
use purrdf_datalog::plan::Parsed;
use purrdf_datalog::schedule::{
    Layer, NoHooks, Schedule, compile_scheduled_with_declarations, evaluate_scheduled,
};
use purrdf_datalog::seminaive::{
    EvalError, EvalOptions, compile, compile_with_declarations, evaluate,
};
use purrdf_datalog::store::RelationStore;

const INTENT: &str = "https://example.org/deceptiveIntentClaim";
const FLAGGED: &str = "https://example.org/flaggedProjection";
const STRUCTURAL: &str = "https://example.org/structuralWitness";

fn atom(predicate: &str) -> ClauseAtom {
    ClauseAtom::positive(ClauseTerm::var("?x"), predicate, ClauseTerm::var("?y"))
}

fn rule(head: &str, body: &str) -> DlClause {
    DlClause::datalog(atom(head), vec![atom(body)])
}

fn declarations() -> NeverDeriveDeclarations {
    let mut declarations = NeverDeriveDeclarations::default();
    declarations.declare(
        INTENT.to_owned(),
        DeclarationSource {
            layer: DeclarationLayer::Ontology,
            identity: "https://example.org/assessor-obligation".to_owned(),
        },
    );
    declarations
}

fn direct_and_transitive_refusals_name_authored_paths() {
    let declarations = declarations();
    for (rules, expected) in [
        (
            vec![rule(INTENT, FLAGGED)],
            vec![DerivationStep {
                rule: 0,
                predicate: INTENT.to_owned(),
            }],
        ),
        (
            vec![rule(STRUCTURAL, FLAGGED), rule(INTENT, STRUCTURAL)],
            vec![
                DerivationStep {
                    rule: 0,
                    predicate: STRUCTURAL.to_owned(),
                },
                DerivationStep {
                    rule: 1,
                    predicate: INTENT.to_owned(),
                },
            ],
        ),
    ] {
        let reference = declarations
            .admit(&rules)
            .expect_err("protected production is refused");
        let AdmissionRefusal::ProtectedHead {
            predicate,
            chain,
            sources,
        } = &reference
        else {
            panic!("{reference:?}");
        };
        assert_eq!(predicate, INTENT);
        assert_eq!(chain, &expected);
        assert_eq!(sources.len(), 1);
        for _ in 0..4 {
            assert_eq!(declarations.admit(&rules).unwrap_err(), reference);
            assert_eq!(
                compile_with_declarations(rules.clone(), &declarations).unwrap_err(),
                EvalError::NeverDerive(reference.clone())
            );
        }
    }
}

fn unrelated_deception_structure_executes_and_asserted_intent_stays_readable() {
    let declarations = declarations();
    let program = compile_with_declarations(
        vec![
            DlClause::datalog(
                ClauseAtom::positive(
                    ClauseTerm::var("?event"),
                    STRUCTURAL,
                    ClauseTerm::var("?projected"),
                ),
                vec![
                    ClauseAtom::positive(
                        ClauseTerm::var("?event"),
                        "https://example.org/eventType",
                        ClauseTerm::iri("https://example.org/eventTypeDeception"),
                    ),
                    ClauseAtom::positive(
                        ClauseTerm::var("?event"),
                        "https://example.org/heldStandpoint",
                        ClauseTerm::var("?held"),
                    ),
                    ClauseAtom::positive(
                        ClauseTerm::var("?event"),
                        "https://example.org/projectedStandpoint",
                        ClauseTerm::var("?projected"),
                    ),
                ],
            ),
            rule("https://example.org/assessorRead", INTENT),
        ],
        &declarations,
    )
    .unwrap();
    let mut input = RelationStore::new();
    for (predicate, object) in [
        (
            "https://example.org/eventType",
            "<https://example.org/eventTypeDeception>",
        ),
        (
            "https://example.org/heldStandpoint",
            "<https://example.org/private-standpoint>",
        ),
        (
            "https://example.org/projectedStandpoint",
            "<https://example.org/statement>",
        ),
    ] {
        input.insert(
            "<https://example.org/alice>",
            &format!("<{predicate}>"),
            object,
            RelationStore::DEFAULT_GRAPH,
        );
    }
    for predicate in [FLAGGED, INTENT] {
        input.insert(
            "<https://example.org/alice>",
            &format!("<{predicate}>"),
            "<https://example.org/statement>",
            RelationStore::DEFAULT_GRAPH,
        );
    }
    let outcome = evaluate(&program, input).unwrap();
    for predicate in [
        FLAGGED,
        INTENT,
        STRUCTURAL,
        "https://example.org/assessorRead",
    ] {
        assert!(outcome.facts().contains(
            "<https://example.org/alice>",
            &format!("<{predicate}>"),
            "<https://example.org/statement>",
            RelationStore::DEFAULT_GRAPH
        ));
    }
    assert_eq!(outcome.derivations().len(), 2);
    assert!(
        outcome
            .derivations()
            .iter()
            .all(|derivation| derivation.fact().predicate != format!("<{INTENT}>"))
    );
}

fn every_authority_survives_union_and_reused_program_admission() {
    let mut policy = declarations();
    for layer in [DeclarationLayer::StoreProfile, DeclarationLayer::RuleSet] {
        policy.declare(
            INTENT.to_owned(),
            DeclarationSource {
                layer,
                identity: "https://example.org/profile".to_owned(),
            },
        );
    }
    policy.extend(&NeverDeriveDeclarations::default());
    assert_eq!(policy.iter().next().unwrap().1.len(), 3);
    let safe = vec![rule(STRUCTURAL, FLAGGED)];
    let program = compile_with_declarations(safe.clone(), &policy)
        .unwrap()
        .admit_declarations(&NeverDeriveDeclarations::default())
        .unwrap();
    assert_eq!(program.declarations(), Some(&policy));
    let legacy = compile(vec![rule(INTENT, FLAGGED)]).unwrap();
    assert!(
        matches!(legacy.admit_declarations(&policy), Err(AdmissionRefusal::ProtectedHead { sources, .. }) if sources.len() == 3)
    );
    let parsed = Parsed::with_declarations(safe, &policy)
        .unwrap()
        .stratify()
        .unwrap()
        .plan()
        .into_executable();
    assert_eq!(parsed.declarations(), Some(&policy));
}

fn existential_conjunctive_disjunctive_heads_cannot_bypass_admission() {
    let policy = declarations();
    let heads = [
        (
            vec![HeadDisjunct::new(vec![ClauseAtom::positive(
                ClauseTerm::var("?x"),
                INTENT,
                ClauseTerm::var("?fresh"),
            )])],
            vec!["?fresh".to_owned()],
        ),
        (
            vec![HeadDisjunct::new(vec![atom(STRUCTURAL), atom(INTENT)])],
            vec![],
        ),
        (
            vec![
                HeadDisjunct::new(vec![atom(STRUCTURAL)]),
                HeadDisjunct::new(vec![atom(INTENT)]),
            ],
            vec![],
        ),
    ];
    for (head, existentials) in heads {
        let rules = vec![DlClause::new(head, existentials, vec![atom(FLAGGED)])];
        assert!(matches!(
            compile_with_declarations(rules.clone(), &policy),
            Err(EvalError::NeverDerive(
                AdmissionRefusal::ProtectedHead { .. }
            ))
        ));
        assert!(matches!(
            Parsed::with_declarations(rules.clone(), &policy),
            Err(EvalError::NeverDerive(
                AdmissionRefusal::ProtectedHead { .. }
            ))
        ));
        assert!(matches!(
            compile_scheduled_with_declarations(
                rules.clone(),
                Schedule::new(vec![Layer::new(vec![vec![0]], vec![])]),
                &policy
            ),
            Err(EvalError::NeverDerive(
                AdmissionRefusal::ProtectedHead { .. }
            ))
        ));
        assert!(matches!(
            chase_with_declarations(
                &rules,
                RelationStore::new(),
                &EvalOptions::default(),
                None,
                &policy
            ),
            Err(ChaseError::NeverDerive(
                AdmissionRefusal::ProtectedHead { .. }
            ))
        ));
    }
}

fn wildcard_absent_protected_symbol_is_explicitly_undecidable() {
    let head = ClauseAtom::quad(
        ClauseTerm::var("?x"),
        ClauseTerm::var("?predicate"),
        ClauseTerm::var("?y"),
        ClauseTerm::DefaultGraph,
    );
    let rules = vec![DlClause::datalog(head.clone(), vec![head])];
    assert!(matches!(
        declarations().admit(&rules),
        Err(AdmissionRefusal::UndecidableHead { rule: 0, .. })
    ));
    assert!(NeverDeriveDeclarations::default().admit(&rules).is_ok());
    // The mixed bundle must refuse without a diagnostic-path panic.
    let mut mixed = rules;
    mixed.push(rule(INTENT, FLAGGED));
    assert!(matches!(
        declarations().admit(&mixed),
        Err(AdmissionRefusal::UndecidableHead { rule: 0, .. })
    ));
}

fn cached_acceptance_and_refusal_bind_all_policy_sources() {
    let rules = vec![rule(INTENT, FLAGGED)];
    let empty = NeverDeriveDeclarations::default();
    let policy = declarations();
    let mut cache = PlanCache::new(8);
    let free = cache.get_or_compile("consumer", rules.clone());
    assert!(free.plan().is_ok());
    let protected = cache.get_or_compile_with_declarations("consumer", rules.clone(), &policy);
    assert!(!protected.cache_hit());
    assert!(matches!(protected.plan(), Err(EvalError::NeverDerive(_))));
    let warm = cache.get_or_compile_with_declarations("consumer", rules.clone(), &policy);
    assert!(warm.cache_hit());
    assert_eq!(warm.plan_builds(), 0);
    let mut additional = policy.clone();
    additional.declare(
        INTENT.to_owned(),
        DeclarationSource {
            layer: DeclarationLayer::RuleSet,
            identity: "https://example.org/rule-author".to_owned(),
        },
    );
    assert!(
        !cache
            .get_or_compile_with_declarations("consumer", rules.clone(), &additional)
            .cache_hit()
    );
    assert_eq!(
        PlanIdentity::new("consumer", &rules),
        PlanIdentity::with_declarations("consumer", &rules, &empty)
    );
    assert_ne!(
        PlanIdentity::with_declarations("consumer", &rules, &policy),
        PlanIdentity::with_declarations("consumer", &rules, &additional)
    );
}

fn cyclic_dependencies_and_empty_heads_have_total_decisions() {
    let policy = declarations();
    assert!(
        policy
            .admit(&[rule(STRUCTURAL, FLAGGED), rule(FLAGGED, STRUCTURAL)])
            .is_ok()
    );
    assert!(matches!(
        policy.admit(&[rule(INTENT, STRUCTURAL), rule(STRUCTURAL, INTENT)]),
        Err(AdmissionRefusal::ProtectedHead { .. })
    ));
    assert!(
        policy
            .admit(&[DlClause::inconsistency(vec![atom(INTENT)])])
            .is_ok()
    );
}

fn scheduled_and_chase_safe_programs_really_produce_structural_witnesses() {
    let policy = declarations();
    let rules = vec![rule(STRUCTURAL, FLAGGED)];
    let schedule = Schedule::new(vec![Layer::new(vec![vec![0]], vec![])]);
    let program = compile_scheduled_with_declarations(rules.clone(), schedule, &policy)
        .unwrap()
        .admit_declarations(&NeverDeriveDeclarations::default())
        .unwrap();
    assert_eq!(program.declarations(), Some(&policy));
    let mut input = RelationStore::new();
    input.insert(
        "<https://example.org/alice>",
        &format!("<{FLAGGED}>"),
        "<https://example.org/statement>",
        RelationStore::DEFAULT_GRAPH,
    );
    let scheduled = evaluate_scheduled(
        &program,
        input.clone(),
        &NoGuards,
        &mut NoHooks,
        &EvalOptions::default(),
        None,
    )
    .unwrap();
    let chased =
        chase_with_declarations(&rules, input, &EvalOptions::default(), None, &policy).unwrap();
    assert_eq!(
        scheduled.facts().facts_sorted(),
        chased.facts().facts_sorted()
    );
    assert_eq!(scheduled.derivations().len(), 1);
    assert_eq!(chased.derivations().len(), 1);
    assert!(scheduled.facts().contains(
        "<https://example.org/alice>",
        &format!("<{STRUCTURAL}>"),
        "<https://example.org/statement>",
        RelationStore::DEFAULT_GRAPH
    ));
}

fn negative_reads_and_opaque_guards_do_not_exempt_protected_heads() {
    let policy = declarations();
    let safe =
        rule(STRUCTURAL, FLAGGED).with_negations(vec![Negation::new(vec![atom(INTENT)], vec![])]);
    assert!(policy.admit(&[safe]).is_ok());
    let guarded = rule(INTENT, FLAGGED).with_guards(vec![Guard::new(
        "https://example.org/opaquePredicateDecision",
        vec!["?x".to_owned()],
        vec![],
        GuardReads::Bindings,
    )]);
    assert!(matches!(
        policy.admit(&[guarded]),
        Err(AdmissionRefusal::ProtectedHead { .. })
    ));
    let certificate = policy.admit(&[rule(STRUCTURAL, FLAGGED)]).unwrap();
    assert_eq!(
        certificate
            .extend(
                &[rule(FLAGGED, STRUCTURAL)],
                &NeverDeriveDeclarations::default()
            )
            .unwrap_err(),
        AdmissionRefusal::ProgramChanged
    );
}

purrdf_testkit::harness_main!(
    direct_and_transitive_refusals_name_authored_paths,
    unrelated_deception_structure_executes_and_asserted_intent_stays_readable,
    every_authority_survives_union_and_reused_program_admission,
    existential_conjunctive_disjunctive_heads_cannot_bypass_admission,
    wildcard_absent_protected_symbol_is_explicitly_undecidable,
    cached_acceptance_and_refusal_bind_all_policy_sources,
    cyclic_dependencies_and_empty_heads_have_total_decisions,
    scheduled_and_chase_safe_programs_really_produce_structural_witnesses,
    negative_reads_and_opaque_guards_do_not_exempt_protected_heads
);
