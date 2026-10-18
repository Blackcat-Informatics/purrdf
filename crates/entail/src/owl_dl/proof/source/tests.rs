// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact source views at the checked public replay boundary.

use super::*;
use crate::reasoner::schema_tests::Fixture;
use crate::reasoner::{Reasoner, Verdict};
use purrdf_alloc_probe::CurrentThreadWindow;
use purrdf_iri::vocab::{owl, rdf, rdfs};

/// Independent preparation for a tiny authored input; no corpus factory is involved.
fn checked_fixture(input: &purrdf_core::RdfDataset) -> (DlProof, DlProofContext<'static>) {
    let producer = Reasoner::with_proofs(input).unwrap();
    let result = producer.consistency();
    assert_eq!(*result.answer(), Verdict::False);
    let proof = result
        .proof()
        .unwrap()
        .runs()
        .last()
        .unwrap()
        .proof()
        .unwrap()
        .clone();
    let consumer = Reasoner::with_proofs(input).unwrap();
    (proof, consumer.proof_context().unwrap())
}

/// A real derived clash; the consumer prepares the original input independently.
fn derived() -> (DlProof, DlProofContext<'static>) {
    derived_for(None)
}

/// Retain either a named or scoped-blank source inhabitant through real replay.
fn derived_for(individual: Option<TermValue>) -> (DlProof, DlProofContext<'static>) {
    let mut fixture = Fixture::default();
    let class = fixture.name("A");
    let filler = fixture.name("F");
    let property = fixture.name("p");
    fixture.bounds(class, property, filler, 2, true, false);
    fixture.bounds(class, property, filler, 1, false, false);
    let edge = fixture.name("edge");
    let individual = match individual {
        Some(value) => crate::interner::intern_into(&mut fixture.builder, &value).unwrap(),
        None => fixture.name("i"),
    };
    let other = fixture.name("other");
    fixture.edge(edge, rdfs::RANGE, class);
    fixture
        .builder
        .push_quad(other, edge, individual, None)
        .unwrap();
    let input = fixture.freeze();
    checked_fixture(&input)
}

#[test]
fn checked_schema_causal_support_retains_role_and_qualifier_licenses() {
    use crate::owl_dl::graph::changes::Journal;
    let mut f = Fixture::default();
    let class = f.name("A");
    let lower = f.name("F");
    let upper = f.name("G");
    let p = f.name("p");
    let q = f.name("q");
    let r = f.name("r");
    let inverse = f.blank();
    f.edge(p, rdfs::SUB_PROPERTY_OF, q);
    f.edge(q, owl::INVERSE_OF, r);
    f.edge(inverse, owl::INVERSE_OF, r);
    f.edge(lower, rdfs::SUB_CLASS_OF, upper);
    f.bounds(class, p, lower, 2, true, false);
    f.bounds(class, inverse, upper, 1, false, false);
    f.instance("i", class);
    let (proof, ctx) = checked_fixture(&f.freeze());
    let index = proof
        .clashes
        .iter()
        .position(|step| step.schema.is_some())
        .unwrap();
    let (report, journal, schema) = proof
        .replay_clash_recording(index, &ctx, Journal::default())
        .unwrap();
    assert!(report.current_supported());
    let schema = schema.unwrap();
    assert_ne!(schema.bounds().qualifier_derivation(), []);
    assert_eq!(schema.bounds().roles.len(), 2);
    let selected = journal
        .as_ref()
        .unwrap()
        .normalized_slice_with_schema(&ctx.kb, Some(schema))
        .unwrap();
    let inputs = selected.inputs().collect::<Result<Vec<_>, _>>().unwrap();
    for predicate in [rdfs::SUB_PROPERTY_OF, owl::INVERSE_OF] {
        assert!(inputs.iter().any(|input| matches!(input.source,
            crate::owl_dl::input_source::Source::Anchor(crate::owl_dl::AxiomSource::Triple(_, p, _))
            if matches!(ctx.kb.interner.value(p), TermValue::Iri(iri) if iri == predicate))));
    }
    assert_ne!(selected.syntax_roots().unwrap(), []);
}

#[test]
fn checked_schema_recording_refuses_other_owner_tampering_legacy_and_storage_failure() {
    use crate::owl_dl::graph::changes::{Journal, slice::Error};
    use purrdf_lex::allocation::StorageError;
    let (proof, ctx) = derived();
    let index = proof
        .clashes
        .iter()
        .position(|step| step.schema.is_some())
        .unwrap();
    let (_, journal, schema) = proof
        .replay_clash_recording(index, &ctx, Journal::default())
        .unwrap();
    let (_, other) = derived();
    assert!(matches!(
        journal
            .unwrap()
            .normalized_slice_with_schema(&other.kb, schema),
        Err(Error::UnexpectedSchema)
    ));
    assert!(
        proof
            .replay_clash_recording(index, &other, Journal::default())
            .is_ok(),
        "the producer proof is checked against an independently prepared consumer"
    );
    let wrong = Reasoner::with_proofs(&Fixture::default().freeze())
        .unwrap()
        .proof_context()
        .unwrap();
    assert!(
        proof
            .replay_clash_recording(index, &wrong, Journal::default())
            .is_err()
    );
    let mut tampered = proof.clone();
    tampered.clashes[index]
        .schema
        .as_mut()
        .unwrap()
        .bounds
        .lower = u32::MAX;
    assert!(
        tampered
            .replay_clash_recording(index, &ctx, Journal::default())
            .is_err()
    );
    let mut legacy = proof.clone();
    legacy.clashes[index].current = None;
    assert!(
        legacy
            .replay_clash_recording(index, &ctx, Journal::default())
            .is_err()
    );
    assert!(matches!(
        proof.replay_clash_recording(
            index,
            &ctx,
            Journal::with_injected_refusal(StorageError::AllocationFailed)
        ),
        Err(DlProofError::Storage(StorageError::AllocationFailed))
    ));
}

#[test]
fn checked_schema_slices_unused_but_valid_implication_steps() {
    use crate::owl_dl::{
        bounds::{SchemaRule, SchemaStep},
        graph::changes::Journal,
    };
    let mut f = Fixture::default();
    let class = f.name("A");
    let filler = f.name("F");
    let property = f.name("p");
    let extra = f.name("Unused");
    f.edge(class, rdfs::SUB_CLASS_OF, extra);
    f.bounds(class, property, filler, 2, true, false);
    f.bounds(class, property, filler, 1, false, false);
    f.instance("i", class);
    let (mut proof, ctx) = checked_fixture(&f.freeze());
    let index = proof
        .clashes
        .iter()
        .position(|step| step.schema.is_some())
        .unwrap();
    let inclusion = ctx.kb.tbox.iter().position(|&(_, sup)| matches!(ctx.kb.table.concept(sup),
        Concept::Named(term)
        if matches!(ctx.kb.interner.value(*term), TermValue::Iri(iri) if iri == "https://example.org/Unused"))).unwrap();
    let bounds = &mut proof.clashes[index].schema.as_mut().unwrap().bounds;
    let after_assumption = bounds
        .class_steps
        .iter()
        .position(|step| step.rule == SchemaRule::Assume)
        .unwrap()
        + 1;
    bounds.class_steps.insert(
        after_assumption,
        SchemaStep {
            concept: ctx.kb.tbox[inclusion].1,
            rule: SchemaRule::Inclusion(inclusion),
        },
    );
    let (_, journal, schema) = proof
        .replay_clash_recording(index, &ctx, Journal::default())
        .unwrap();
    let selected = journal
        .unwrap()
        .normalized_slice_with_schema(&ctx.kb, schema)
        .unwrap();
    let extra_root = ctx.kb.syntax.tbox_root(inclusion).unwrap().root().unwrap();
    assert!(
        !selected
            .syntax_roots()
            .unwrap()
            .contains(&(crate::owl_dl::syntax::Space::Source, extra_root))
    );
    assert!(
        selected.schema_reads.len() < selected.schema.unwrap().bounds().class_derivation().len()
    );
}

/// A checked application prefix is not a causal slice: retain this counterexample.
#[test]
fn checked_prefix_contains_an_unrelated_application_before_the_clash() {
    use crate::owl_dl::graph::changes::{Journal, Mutation, Origin, ReadOwner};
    let mut f = Fixture::default();
    let unrelated = f.name("Unrelated");
    let derived = f.name("UnrelatedDerived");
    f.edge(unrelated, rdfs::SUB_CLASS_OF, derived);
    f.instance("earlier", unrelated);
    let class = f.name("A");
    let filler = f.name("F");
    let property = f.name("p");
    f.bounds(class, property, filler, 2, true, false);
    f.bounds(class, property, filler, 1, false, false);
    let edge = f.name("edge");
    let from = f.name("from");
    let inhabitant = f.name("inhabitant");
    f.edge(edge, rdfs::RANGE, class);
    f.builder.push_quad(from, edge, inhabitant, None).unwrap();
    let (proof, ctx) = checked_fixture(&f.freeze());
    let index = proof
        .clashes
        .iter()
        .position(|step| step.schema.is_some())
        .unwrap();
    let step = &proof.clashes[index];
    let prefix =
        crate::owl_dl::support::prefix(&proof.support, step.current.as_ref().unwrap().latest, None)
            .unwrap();
    let unrelated_step = prefix.iter().position(|application| matches!(ctx.clause_tbox_origin(application.clause).unwrap(),
        Some(ResolvedAxiomSource::Triple { subject: ResolvedTerm::Source(TermValue::Iri(subject)), .. })
        if subject == "https://example.org/Unrelated")).unwrap();
    let checked = proof.resolve_clash(index, &ctx).unwrap();
    assert!(checked.replay().current_supported());
    assert!(checked.witness().all(|fact| matches!(fact,
        ResolvedFact::Concept(ResolvedNode::Individual(ResolvedTerm::Source(TermValue::Iri(individual))), _)
        if individual == "https://example.org/inhabitant")));
    let (report, mutations, schema) = proof
        .replay_clash_recording(index, &ctx, Journal::default())
        .unwrap();
    assert!(report.current_supported());
    let journal = mutations.unwrap();
    let earlier = ctx
        .kb
        .interner
        .id_of_iri("https://example.org/earlier")
        .unwrap();
    let earlier_node = journal
        .events()
        .iter()
        .find_map(|event| match event.mutation {
            Mutation::Node {
                node,
                named: Some(name),
                ..
            } if name == earlier => Some(node),
            _ => None,
        })
        .unwrap();
    assert!(
        journal
            .events()
            .iter()
            .any(|event| event.cause.origin == Origin::Application
                && event.cause.index == unrelated_step
                && matches!(event.mutation, Mutation::Label { node, .. } if node == earlier_node))
    );
    assert!(
        journal
            .events()
            .iter()
            .any(|event| event.cause.origin == Origin::PreparedRole)
    );
    assert!(journal.reads().iter().any(|read| matches!(read.owner,
        ReadOwner::Application { index, .. } if index == unrelated_step)));
    let selected = journal.normalized_slice(&ctx.kb).unwrap();
    assert!(
        matches!(
            selected.syntax_roots(),
            Err(crate::owl_dl::graph::changes::slice::Error::MissingSchema(
                _
            ))
        ),
        "a multi-license bound guard is not an empty syntax success"
    );
    let selected = journal
        .normalized_slice_with_schema(&ctx.kb, schema)
        .unwrap();
    let roots = selected.syntax_roots().unwrap();
    assert_ne!(roots, []);
    let bounds = selected.schema.as_ref().unwrap().bounds();
    assert!(std::ptr::eq(
        bounds,
        &raw const step.schema.as_ref().unwrap().bounds
    ));
    assert_eq!(
        bounds
            .class_derivation()
            .iter()
            .filter_map(crate::owl_dl::bounds::SchemaStep::inclusion)
            .count(),
        2
    );
    for wanted in [
        owl::MIN_QUALIFIED_CARDINALITY,
        owl::MAX_QUALIFIED_CARDINALITY,
    ] {
        assert!(roots.iter().any(|&(space, root)| {
            assert_eq!(space, crate::owl_dl::syntax::Space::Source);
            ctx.kb.syntax.selected(&[root], None).unwrap().iter().any(|&node| {
                ctx.kb.syntax.nodes[node].reads.iter().any(|read| matches!(read,
                    crate::owl_dl::syntax::Read::Triple(_, predicate, _)
                    if matches!(ctx.kb.interner.value(*predicate), TermValue::Iri(iri) if iri == wanted)))
            })
        }));
    }
    let input_sources = selected.inputs().collect::<Result<Vec<_>, _>>().unwrap();
    assert!(
        input_sources
            .iter()
            .any(|record| matches!(record.input, crate::owl_dl::input_source::Input::Role(_)))
    );
    assert!(input_sources.iter().all(|record| match record.source {
        crate::owl_dl::input_source::Source::Anchor(crate::owl_dl::AxiomSource::Triple(
            subject,
            _,
            _,
        )) => subject != earlier,
        _ => true,
    }));
    assert!(selected.owners.contains(&ReadOwner::Clash {
        clause: step.clause
    }));
    assert!(!selected.owners.iter().any(|owner| matches!(owner,
        ReadOwner::Application { index, .. } if *index == unrelated_step)));
    assert!(
        !selected
            .causes
            .iter()
            .any(|cause| cause.origin == Origin::Application && cause.index == unrelated_step)
    );
    assert!(
        !selected
            .causes
            .iter()
            .any(|cause| cause.origin == Origin::TermDenotation && cause.index == earlier as usize)
    );
    assert!(
        selected
            .causes
            .iter()
            .any(|cause| cause.origin == Origin::PreparedRole)
    );
    // Native dependency selection is now checked; original RDF/schema licenses
    // and all remaining path/effect families still block the public operation.
}

/// Real RDF property chains retain checked native paths through independent replay.
#[test]
fn checked_universal_role_clauses_retain_their_original_compiler_license() {
    use crate::owl_dl::{graph::changes::Journal, roles::RoleClauseKind};
    let mut f = Fixture::default();
    let class = f.name("A");
    let filler = f.name("F");
    let disjoint = f.name("D");
    f.edge(filler, owl::DISJOINT_WITH, disjoint);
    let first = f.name("first");
    let second = f.name("second");
    let chained = f.name("chained");
    let list = f.list(&[first, second]);
    f.edge(chained, owl::PROPERTY_CHAIN_AXIOM, list);
    let restriction = f.blank();
    f.edge(class, rdfs::SUB_CLASS_OF, restriction);
    f.edge(restriction, owl::ON_PROPERTY, chained);
    f.edge(restriction, owl::ALL_VALUES_FROM, filler);
    let from = f.instance("from", class);
    let middle = f.name("middle");
    let target = f.instance("target", disjoint);
    f.builder.push_quad(from, first, middle, None).unwrap();
    f.builder.push_quad(middle, second, target, None).unwrap();
    let (mut proof, ctx) = checked_fixture(&f.freeze());
    let index = proof
        .clashes
        .iter()
        .position(|step| step.current.is_some())
        .unwrap();
    // The ordinary direct universal clause wins before the state route. Reuse
    // its actual accepted native path to supply the alternative legal prefix;
    // no second role search or change to production scheduling is needed.
    let (mut applications, tail, path, obligation_index) = {
        let current = proof.clashes[index].current.as_ref().unwrap();
        let prefix = crate::owl_dl::support::prefix(&proof.support, current.latest, None).unwrap();
        let direct = prefix
            .iter()
            .position(|application| {
                matches!(ctx.clauses.clause(application.clause, &ctx.kb.schema).body.first(),
                Some(crate::owl_dl::clause::BodyAtom::Concept { concept, .. })
                if matches!(ctx.kb.table.decomp(*concept), crate::owl_dl::concept::Decomp::All(..)))
            })
            .unwrap();
        let crate::owl_dl::clause::BodyAtom::Concept { concept, .. } = ctx
            .clauses
            .clause(prefix[direct].clause, &ctx.kb.schema)
            .body[0]
        else {
            unreachable!()
        };
        let program = ctx.kb.role_program.as_ref().unwrap();
        let obligation_index = program
            .obligations
            .iter()
            .position(|obligation| obligation.source == concept)
            .unwrap();
        let (_, journal, schema) = proof
            .replay_clash_recording(index, &ctx, Journal::default())
            .unwrap();
        let selected = journal
            .unwrap()
            .normalized_slice_with_schema(&ctx.kb, schema)
            .unwrap();
        (
            prefix[..direct]
                .iter()
                .map(|application| (**application).clone())
                .collect::<Vec<_>>(),
            prefix[direct + 1..]
                .iter()
                .map(|application| (**application).clone())
                .collect::<Vec<_>>(),
            selected.paths.into_iter().rev().collect::<Vec<_>>(),
            obligation_index,
        )
    };
    let program = ctx.kb.role_program.as_ref().unwrap();
    let obligation = &program.obligations[obligation_index];
    let machine = &program.machines[obligation.machine];
    let clause = |kind| {
        (0..ctx.clauses.count())
            .find(|&index| {
                ctx.clauses.role_source(index, program)
                    == Some(crate::owl_dl::roles::RoleClause {
                        obligation: obligation_index,
                        kind,
                    })
            })
            .unwrap()
    };
    use crate::owl_dl::{
        hyper::Ground,
        support::{Application, Link},
    };
    let from = path.first().unwrap().from.0;
    let to = path.last().unwrap().to.0;
    applications.push(Application {
        clause: clause(RoleClauseKind::Enter),
        frame: vec![from],
        branch: None,
        head: vec![Ground::Concept(from, obligation.states[machine.initial])],
    });
    for step in &path {
        let transition = &machine.transitions[step.transition];
        let frame = if transition.letter.is_some() {
            vec![step.from.0, step.to.0]
        } else {
            vec![step.from.0]
        };
        applications.push(Application {
            clause: clause(RoleClauseKind::Transition(step.transition)),
            frame,
            branch: None,
            head: vec![Ground::Concept(step.to.0, obligation.states[transition.to])],
        });
    }
    applications.push(Application {
        clause: clause(RoleClauseKind::Accept),
        frame: vec![to],
        branch: None,
        head: vec![Ground::Concept(to, obligation.filler)],
    });
    applications.extend(tail);
    proof.support.clear();
    for application in applications {
        let previous = proof.support.len().checked_sub(1);
        proof.support.push(Link {
            previous,
            application,
        });
    }
    proof.clashes[index].current.as_mut().unwrap().latest = proof.support.len().checked_sub(1);
    assert!(
        proof
            .resolve_clash(index, &ctx)
            .unwrap()
            .replay()
            .current_supported()
    );
    let (_, journal, schema) = proof
        .replay_clash_recording(index, &ctx, Journal::default())
        .unwrap();
    let slice = journal
        .unwrap()
        .normalized_slice_with_schema(&ctx.kb, schema)
        .unwrap();
    assert!(
        slice
            .role_clauses
            .iter()
            .any(|source| source.kind == RoleClauseKind::Enter)
    );
    assert!(
        slice
            .role_clauses
            .iter()
            .any(|source| source.kind == RoleClauseKind::Accept)
    );
    assert!(
        slice
            .role_clauses
            .iter()
            .filter(|source| matches!(source.kind, RoleClauseKind::Transition(_)))
            .count()
            >= 2
    );
    let inputs = slice.role_inputs().unwrap();
    assert_eq!(inputs.len(), 1);
    assert!(matches!(
        inputs[0],
        crate::owl_dl::roles::source::Input::Chain(_)
    ));
    assert_ne!(slice.syntax_roots().unwrap(), []);
    let program = ctx.kb.role_program.as_ref().unwrap();
    let window = CurrentThreadWindow::open();
    let located = (0..ctx.clauses.count())
        .filter_map(|index| ctx.clauses.role_source(index, program))
        .count();
    let measured = window.close();
    assert!(located > 0);
    assert_eq!(
        measured.allocations, 0,
        "clause source lookup borrows the original obligation directory"
    );
}

/// Real RDF property chains retain checked native paths through independent replay.
#[test]
fn checked_clash_retains_selected_compiled_role_path() {
    use crate::owl_dl::graph::changes::{Journal, PathKind};
    let mut f = Fixture::default();
    let class = f.name("A");
    let filler = f.name("F");
    let counted = f.name("counted");
    f.bounds(class, counted, filler, 2, true, false);
    f.bounds(class, counted, filler, 1, false, false);
    let first = f.name("first");
    let second = f.name("second");
    let leaf = f.name("leaf");
    let bridge = f.name("bridge");
    let alias_a = f.name("roleAliasA");
    let alias_b = f.name("roleAliasB");
    f.edge(leaf, rdfs::SUB_PROPERTY_OF, bridge);
    f.edge(bridge, rdfs::SUB_PROPERTY_OF, first);
    f.edge(first, rdfs::SUB_PROPERTY_OF, alias_a);
    f.edge(alias_a, rdfs::SUB_PROPERTY_OF, alias_b);
    f.edge(alias_b, rdfs::SUB_PROPERTY_OF, first);
    let chained = f.name("chained");
    let list = f.list(&[first, second]);
    f.edge(chained, owl::PROPERTY_CHAIN_AXIOM, list);
    let unused = f.name("unusedChain");
    let unused_first = f.name("unusedFirst");
    let unused_second = f.name("unusedSecond");
    f.edge(unused_first, rdfs::SUB_PROPERTY_OF, unused_second);
    f.edge(unused_second, rdfs::SUB_PROPERTY_OF, unused_first);
    let unused_list = f.list(&[unused_first, unused_second]);
    f.edge(unused, owl::PROPERTY_CHAIN_AXIOM, unused_list);
    f.edge(chained, rdfs::RANGE, class);
    let from = f.name("from");
    let middle = f.name("middle");
    let target = f.name("target");
    f.builder.push_quad(from, leaf, middle, None).unwrap();
    f.builder.push_quad(middle, second, target, None).unwrap();
    let (proof, ctx) = checked_fixture(&f.freeze());
    let index = proof
        .clashes
        .iter()
        .position(|step| step.schema.is_some())
        .unwrap();
    let checked = proof.resolve_clash(index, &ctx).unwrap();
    assert!(checked.replay().current_supported());
    let step = &proof.clashes[index];
    let current = step.current.as_ref().unwrap();
    let prefix = crate::owl_dl::support::prefix(&proof.support, current.latest, None).unwrap();
    let observed = crate::owl_dl::hyper::replay_observing_support(
        &ctx.kb,
        &crate::owl_dl::graph::Assumptions::of_kb(),
        &prefix,
        step.clause,
        &current.frame,
        &proof.branches,
        Journal::default(),
    )
    .unwrap();
    assert_eq!(observed.frame, step.frame);
    assert_eq!(observed.facts, step.witness);
    let slice = observed
        .mutations
        .unwrap()
        .normalized_slice(&ctx.kb)
        .unwrap();
    assert!(
        slice
            .paths
            .iter()
            .any(|step| matches!(step.kind, PathKind::Edge { .. }))
    );
    for path in &slice.paths {
        let machine = ctx
            .kb
            .role_program
            .as_ref()
            .unwrap()
            .machine(path.role)
            .unwrap();
        let transition = &machine.transitions[path.transition];
        assert_eq!((transition.from, transition.to), (path.from.1, path.to.1));
    }
    let inputs = slice.role_inputs().unwrap();
    assert_eq!(
        inputs.len(),
        6,
        "one chain, two inclusions and one three-edge component certificate"
    );
    let program = ctx.kb.role_program.as_ref().unwrap();
    let chains: Vec<_> = inputs
        .iter()
        .filter(|input| matches!(input, crate::owl_dl::roles::source::Input::Chain(_)))
        .collect();
    assert_eq!(chains.len(), 1, "the unused chain is not a causal source");
    let source = program.source.sources.get(*chains[0]).unwrap();
    assert!(
        matches!(ctx.kb.interner.value(source.anchor.0), TermValue::Iri(iri) if iri == "https://example.org/chained")
    );
    assert!(
        matches!(ctx.kb.interner.value(source.anchor.1), TermValue::Iri(iri) if iri == owl::PROPERTY_CHAIN_AXIOM)
    );
    let reads = &program.source.sources.reads[source.reads.clone()];
    assert_eq!(reads.len(), 4);
    for predicate in [rdf::FIRST, rdf::REST] {
        assert_eq!(reads.iter().filter(|&&(_, p, _)| matches!(ctx.kb.interner.value(p), TermValue::Iri(iri) if iri == predicate)).count(), 2);
    }
}

/// Transitive source support reaches the actual clash through independent replay.
#[test]
fn checked_clash_retains_selected_transitive_role_path() {
    use crate::owl_dl::graph::changes::{Journal, Origin};
    let mut f = Fixture::default();
    let class = f.name("A");
    let filler = f.name("C");
    let disjoint = f.name("D");
    f.edge(filler, owl::DISJOINT_WITH, disjoint);
    let role = f.name("r");
    let transitive = f.builder.intern_iri(owl::TRANSITIVE_PROPERTY).unwrap();
    f.edge(role, rdf::TYPE, transitive);
    let subrole = f.name("s");
    let inverse = f.name("inverse");
    f.edge(subrole, rdfs::SUB_PROPERTY_OF, role);
    f.edge(role, owl::INVERSE_OF, inverse);
    let restriction = f.blank();
    f.edge(class, rdfs::SUB_CLASS_OF, restriction);
    f.edge(restriction, owl::ON_PROPERTY, role);
    f.edge(restriction, owl::ALL_VALUES_FROM, filler);
    let from = f.instance("from", class);
    let middle = f.name("middle");
    let target = f.instance("target", disjoint);
    f.builder.push_quad(from, subrole, middle, None).unwrap();
    f.builder.push_quad(target, inverse, middle, None).unwrap();
    let (proof, ctx) = checked_fixture(&f.freeze());
    assert!(ctx.kb.role_program.is_none());
    let index = proof
        .clashes
        .iter()
        .position(|step| step.current.is_some())
        .unwrap();
    assert!(
        proof
            .resolve_clash(index, &ctx)
            .unwrap()
            .replay()
            .current_supported()
    );
    let step = &proof.clashes[index];
    let current = step.current.as_ref().unwrap();
    let prefix = crate::owl_dl::support::prefix(&proof.support, current.latest, None).unwrap();
    let observed = crate::owl_dl::hyper::replay_observing_support(
        &ctx.kb,
        &crate::owl_dl::graph::Assumptions::of_kb(),
        &prefix,
        step.clause,
        &current.frame,
        &proof.branches,
        Journal::default(),
    )
    .unwrap();
    assert_eq!(observed.frame, step.frame);
    assert_eq!(observed.facts, step.witness);
    let slice = observed
        .mutations
        .unwrap()
        .normalized_slice(&ctx.kb)
        .unwrap();
    assert!(
        slice
            .transitive_nodes
            .iter()
            .filter(|node| matches!(
                node.path,
                crate::owl_dl::graph::changes::reach::Path::Step(_)
            ))
            .count()
            >= 2
    );
    let selected_inputs = slice.inputs().collect::<Result<Vec<_>, _>>().unwrap();
    assert!(selected_inputs.iter().any(|record| matches!(
        record.input,
        crate::owl_dl::input_source::Input::Transitive(_)
    )));
    let roots = slice.syntax_roots().unwrap();
    let source_roots: Vec<_> = roots
        .iter()
        .filter_map(|&(space, root)| {
            (space == crate::owl_dl::syntax::Space::Source).then_some(root)
        })
        .collect();
    let nodes = ctx
        .kb
        .syntax
        .selected(&source_roots, ctx.kb.stop.as_deref())
        .unwrap();
    assert!(nodes.iter().any(|&id| ctx.kb.syntax.nodes[id].reads.iter().any(|read| match read {
        crate::owl_dl::syntax::Read::Triple(_, predicate, _) => matches!(ctx.kb.interner.value(*predicate), TermValue::Iri(iri) if iri == owl::ALL_VALUES_FROM),
        crate::owl_dl::syntax::Read::Child(_) => false,
    })));
    assert!(selected_inputs.iter().any(|record| matches!(
        record.input,
        crate::owl_dl::input_source::Input::Transitive(_)
    ) && record.cause.is_none()));
    assert!(selected_inputs.iter().any(|record| matches!(
        record.input,
        crate::owl_dl::input_source::Input::SubRole { .. }
    )));
    assert!(selected_inputs.iter().any(|record| matches!(
        record.input,
        crate::owl_dl::input_source::Input::Inverse(..)
    )));
    for index in 0..2 {
        assert!(
            slice
                .causes
                .iter()
                .any(|cause| cause.origin == Origin::PreparedRole && cause.index == index)
        );
    }
}

/// Authored equality binds to its real triple; key inference is not that triple.
#[test]
fn checked_input_sources_distinguish_authored_and_key_derived_equality() {
    use crate::owl_dl::graph::changes::{Journal, slice::Error};
    use crate::owl_dl::input_source::{Input, Source};
    for derived_key in [false, true] {
        let mut f = Fixture::default();
        let a = f.name("A");
        let b = f.name("B");
        f.edge(a, owl::DISJOINT_WITH, b);
        let left = f.instance("left", a);
        let right = f.instance("right", b);
        if derived_key {
            let keyed = f.name("Keyed");
            let property = f.name("key");
            let list = f.list(&[property]);
            f.edge(keyed, owl::HAS_KEY, list);
            f.edge(left, rdf::TYPE, keyed);
            f.edge(right, rdf::TYPE, keyed);
            let value = f.name("shared");
            f.builder.push_quad(left, property, value, None).unwrap();
            f.builder.push_quad(right, property, value, None).unwrap();
        } else {
            f.edge(left, owl::SAME_AS, right);
        }
        let (proof, ctx) = checked_fixture(&f.freeze());
        let index = proof
            .clashes
            .iter()
            .position(|step| step.current.is_some())
            .unwrap();
        assert!(
            proof
                .resolve_clash(index, &ctx)
                .unwrap()
                .replay()
                .current_supported()
        );
        let step = &proof.clashes[index];
        let current = step.current.as_ref().unwrap();
        let prefix = crate::owl_dl::support::prefix(&proof.support, current.latest, None).unwrap();
        let observed = crate::owl_dl::hyper::replay_observing_support(
            &ctx.kb,
            &crate::owl_dl::graph::Assumptions::of_kb(),
            &prefix,
            step.clause,
            &current.frame,
            &proof.branches,
            Journal::default(),
        )
        .unwrap();
        let slice = observed.mutations.unwrap().normalized_slice(&ctx.kb);
        if derived_key {
            assert!(matches!(slice, Err(Error::MissingEffect)));
            assert!(matches!(
                ctx.kb.inputs.get(Input::Equality(0)),
                Some(Source::KeyInference { key: 0, .. })
            ));
        } else {
            let slice = slice.unwrap();
            let inputs = slice.inputs().collect::<Result<Vec<_>, _>>().unwrap();
            assert!(inputs.iter().any(|record| match (record.input, record.source) {
                (Input::Equality(_), Source::Anchor(crate::owl_dl::AxiomSource::Triple(_, predicate, _))) => matches!(ctx.kb.interner.value(predicate), TermValue::Iri(iri) if iri == owl::SAME_AS),
                _ => false,
            }));
        }
    }
}

/// A failed native recording keeps its original refusal and cannot publish an empty trace.
#[test]
fn native_mutations_preserve_original_storage_refusal_before_malformed_reporting() {
    use crate::owl_dl::graph::changes::Journal;
    use purrdf_lex::allocation::StorageError;
    let (proof, ctx) = derived();
    let index = proof
        .clashes
        .iter()
        .position(|step| step.current.is_some())
        .unwrap();
    let step = &proof.clashes[index];
    let prefix =
        crate::owl_dl::support::prefix(&proof.support, step.current.as_ref().unwrap().latest, None)
            .unwrap();
    let journal = Journal::with_injected_refusal(StorageError::AllocationFailed);
    assert!(matches!(
        crate::owl_dl::hyper::replay_observing_support(
            &ctx.kb,
            &crate::owl_dl::graph::Assumptions::of_kb(),
            &prefix,
            step.clause,
            &[usize::MAX],
            &proof.branches,
            journal,
        ),
        Err(DlProofError::Storage(StorageError::AllocationFailed))
    ));
    assert!(
        proof
            .resolve_clash(index, &ctx)
            .unwrap()
            .replay()
            .current_supported()
    );
}

#[test]
fn legacy_structural_success_is_not_checked_source_support() {
    let mut fixture = Fixture::default();
    let a = fixture.name("A");
    let b = fixture.name("B");
    fixture.edge(a, owl::DISJOINT_WITH, b);
    let individual = fixture.instance("i", a);
    fixture.edge(individual, rdf::TYPE, b);
    let (mut proof, context) = checked_fixture(&fixture.freeze());
    for step in &mut proof.clashes {
        assert!(step.schema.is_none());
        step.current = None;
    }
    assert!(!proof.replay_clash(0, &context).unwrap().current_supported());
    assert!(matches!(
        proof.resolve_clash(0, &context),
        Err(DlProofError::Malformed { .. })
    ));
}

#[test]
fn actual_role_clash_resolves_both_endpoints_and_property() {
    let mut fixture = Fixture::default();
    let property = fixture.name("p");
    let asymmetric = fixture
        .builder
        .intern_iri(owl::ASYMMETRIC_PROPERTY)
        .unwrap();
    fixture.edge(property, rdf::TYPE, asymmetric);
    let a = fixture.name("a");
    let b = fixture.name("b");
    fixture.builder.push_quad(a, property, b, None).unwrap();
    fixture.builder.push_quad(b, property, a, None).unwrap();
    let (proof, context) = checked_fixture(&fixture.freeze());
    let index = proof
        .clashes
        .iter()
        .position(|step| {
            step.witness
                .iter()
                .any(|f| matches!(f, ProofFact::Edge { .. }))
        })
        .unwrap();
    let checked = proof.resolve_clash(index, &context).unwrap();
    assert!(checked.schema().is_none());
    let mut edges = 0;
    for fact in checked.witness() {
        if let ResolvedFact::Edge(
            ResolvedNode::Individual(ResolvedTerm::Source(from)),
            r,
            ResolvedNode::Individual(ResolvedTerm::Source(to)),
        ) = fact
        {
            edges += 1;
            assert_ne!(from, to);
            assert_eq!(
                r.property,
                ResolvedTerm::Source(&TermValue::Iri("https://example.org/p".to_owned()))
            );
            assert!(!r.inverse);
        }
    }
    assert_eq!(edges, 2);
}

#[test]
fn actual_nominal_clash_resolves_denotation_without_renaming() {
    let mut fixture = Fixture::default();
    let class = fixture.name("A");
    let individual = fixture.instance("i", class);
    let nominal = fixture.blank();
    let list = fixture.list(&[individual]);
    fixture.edge(nominal, owl::ONE_OF, list);
    fixture.edge(class, owl::COMPLEMENT_OF, nominal);
    let (proof, context) = checked_fixture(&fixture.freeze());
    let index = proof
        .clashes
        .iter()
        .position(|step| {
            step.witness
                .iter()
                .any(|f| matches!(f, ProofFact::Denotes { .. }))
        })
        .unwrap();
    let checked = proof.resolve_clash(index, &context).unwrap();
    assert!(checked.witness().any(|f| matches!(f,
        ResolvedFact::Denotes(ResolvedNode::Individual(ResolvedTerm::Source(node)), ResolvedTerm::Source(individual))
        if node == individual)));
}

#[test]
fn resolution_preserves_the_consumers_typed_stop() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    #[derive(Debug)]
    struct Stop(AtomicBool);
    impl purrdf_datalog::StopSignal for Stop {
        fn stopped(&self) -> bool {
            self.0.load(Ordering::Relaxed)
        }
    }
    let input = crate::reasoner::schema_tests::shared_schema(1, 1, true);
    let (proof, _) = checked_fixture(&input);
    let stop = Arc::new(Stop(AtomicBool::new(false)));
    let consumer = Reasoner::with_proofs_and_stop(
        &input,
        crate::reasoner::SchemaPreparationBudget::default(),
        stop.clone(),
    )
    .unwrap();
    let context = consumer.borrowed_proof_context().unwrap();
    let (_, journal, schema) = proof
        .replay_clash_recording(
            0,
            &context,
            crate::owl_dl::graph::changes::Journal::default(),
        )
        .unwrap();
    stop.0.store(true, Ordering::Relaxed);
    assert!(matches!(
        proof.replay_clash_recording(
            0,
            &context,
            crate::owl_dl::graph::changes::Journal::default()
        ),
        Err(DlProofError::Stopped)
    ));
    assert!(matches!(
        journal
            .unwrap()
            .normalized_slice_with_schema(&context.kb, schema),
        Err(crate::owl_dl::graph::changes::slice::Error::Stopped)
    ));
    assert!(matches!(
        proof.resolve_clash(0, &context),
        Err(DlProofError::Stopped)
    ));
}

#[test]
fn checked_clash_resolves_actual_derived_source_membership_without_copying() {
    let (proof, context) = derived();
    let index = proof
        .clashes
        .iter()
        .position(|step| step.schema.is_some())
        .unwrap();
    let checked = proof.resolve_clash(index, &context).unwrap();
    assert!(checked.replay().current_supported());
    let mut count = 0;
    for fact in checked.witness() {
        count += 1;
        let ResolvedFact::Concept(
            ResolvedNode::Individual(ResolvedTerm::Source(individual)),
            class,
        ) = fact
        else {
            panic!("schema clash must retain its source inhabitant and class");
        };
        assert_eq!(
            individual,
            &TermValue::Iri("https://example.org/i".to_owned())
        );
        let ConceptKind::Named(ResolvedTerm::Source(class)) = class.kind() else {
            panic!("schema clash must resolve its native class");
        };
        assert_eq!(class, &TermValue::Iri("https://example.org/A".to_owned()));
    }
    assert!(count > 0);
    // The strings above are test expectations, so measure the iterator separately.
    let window = CurrentThreadWindow::open();
    assert_eq!(checked.witness().count(), count);
    assert_eq!(window.sample().allocations, 0);
}

#[test]
fn source_resolution_refuses_wrong_source_and_absent_current_support() {
    let (mut proof, context) = derived();
    let wrong = Reasoner::with_proofs(&Fixture::default().freeze())
        .unwrap()
        .proof_context()
        .unwrap();
    assert!(proof.resolve_clash(0, &wrong).is_err());
    assert!(proof.resolve_clash(proof.clashes.len(), &context).is_err());
    for step in &mut proof.clashes {
        step.current = None;
        step.schema = None;
    }
    for index in 0..proof.clashes.len() {
        assert!(proof.resolve_clash(index, &context).is_err());
    }
}

#[test]
fn checked_source_blank_scope_and_schema_bounds_survive_resolution() {
    let original = TermValue::Blank {
        label: "inhabitant".to_owned(),
        scope: purrdf_core::BlankScope(71),
    };
    let (proof, context) = derived_for(Some(original.clone()));
    let index = proof
        .clashes
        .iter()
        .position(|s| s.schema.is_some())
        .unwrap();
    let checked = proof.resolve_clash(index, &context).unwrap();
    assert_eq!(checked.replay().source_node(), Some(&original));
    assert!(checked.witness().any(|fact| matches!(fact,
        ResolvedFact::Concept(ResolvedNode::Individual(ResolvedTerm::Source(t)), _) if t == &original)));
    let schema = checked.schema().unwrap();
    assert!(matches!(schema.class().kind(), ConceptKind::Named(_)));
    assert!(matches!(schema.lower().kind(), ConceptKind::Min(2, _, _)));
    let SchemaUpper::Restriction(upper) = schema.upper() else {
        panic!("actual maximum restriction")
    };
    assert!(matches!(upper.kind(), ConceptKind::Max(1, _, _)));
    assert!(matches!(schema.qualifier(), QualifierKind::Identity));
    assert_eq!(schema.qualifier_derivation().len(), 0);
    let steps: Vec<_> = schema.class_derivation().collect();
    assert!(
        steps
            .iter()
            .any(|s| matches!(s.reason, SchemaReason::Assume))
    );
    assert_eq!(
        steps
            .iter()
            .filter(|s| matches!(s.reason, SchemaReason::Inclusion { .. }))
            .count(),
        2
    );
    for s in steps {
        if let SchemaReason::Inclusion { sub, sup, origin } = s.reason {
            assert!(matches!(sub.kind(), ConceptKind::Named(_)));
            assert!(matches!(
                sup.kind(),
                ConceptKind::Min(_, _, _) | ConceptKind::Max(_, _, _)
            ));
            let ResolvedAxiomSource::Triple {
                predicate: ResolvedTerm::Source(predicate),
                object: ResolvedTerm::Source(object),
                ..
            } = origin
            else {
                panic!("authored inclusion must retain its original axiom anchor");
            };
            assert_eq!(predicate, &TermValue::Iri(rdfs::SUB_CLASS_OF.to_owned()));
            assert!(matches!(object, TermValue::Blank { .. }));
        }
    }
}

#[test]
fn exact_literal_metadata_is_borrowed_without_reinterpretation() {
    let mut kb = Kb::empty();
    let literal = TermValue::Literal {
        lexical_form: "مرحبا".to_owned(),
        datatype: rdf::DIR_LANG_STRING.to_owned(),
        language: Some("ar".to_owned()),
        direction: Some(purrdf_core::RdfTextDirection::Rtl),
    };
    let id = kb.interner.intern(literal.clone());
    kb.source_terms = u32::try_from(kb.interner.len()).unwrap();
    let ResolvedTerm::Source(value) = term(&kb, id) else {
        panic!("source literal")
    };
    assert_eq!(value, &literal);
    assert!(std::ptr::eq(value, kb.interner.value(id)));
}

#[test]
fn every_native_constructor_has_a_borrowed_structural_view() {
    let mut kb = Kb::empty();
    let name = kb.interner.intern_iri("https://example.org/A");
    let property = kb.interner.intern_iri("https://example.org/p");
    kb.source_terms = u32::try_from(kb.interner.len()).unwrap();
    let filler = kb.table.intern(Concept::Named(name));
    let range = kb.data_ranges.intern(DataRange::Any);
    let named = || Box::new(Concept::Named(name));
    let concepts = [
        Concept::Top,
        Concept::Bottom,
        Concept::Named(name),
        Concept::Not(named()),
        Concept::And(vec![Concept::Named(name), Concept::Top]),
        Concept::Or(vec![Concept::Named(name), Concept::Bottom]),
        Concept::Some(Role::Named(property), named()),
        Concept::All(Role::Inv(property), named()),
        Concept::Min(2, Role::Named(property), named()),
        Concept::Max(1, Role::Inv(property), named()),
        Concept::Nominal(vec![name]),
        Concept::Data(range),
        Concept::SelfRestriction(Role::Inv(property)),
        Concept::RoleState {
            machine: 3,
            state: 7,
            filler,
            negated: true,
        },
    ];
    let mut seen = Vec::new();
    for concept in &concepts {
        let view = ResolvedConcept { kb: &kb, concept };
        let tag = match view.kind() {
            ConceptKind::Top => 0,
            ConceptKind::Bottom => 1,
            ConceptKind::Named(t) => {
                assert!(matches!(t, ResolvedTerm::Source(_)));
                2
            }
            ConceptKind::Not(c) => {
                assert!(matches!(c.kind(), ConceptKind::Named(_)));
                3
            }
            ConceptKind::And(mut cs) => {
                assert_eq!(cs.len(), 2);
                assert!(matches!(cs.next().unwrap().kind(), ConceptKind::Named(_)));
                4
            }
            ConceptKind::Or(cs) => {
                assert_eq!(cs.count(), 2);
                5
            }
            ConceptKind::Some(r, c) => {
                assert!(!r.inverse);
                assert!(matches!(c.kind(), ConceptKind::Named(_)));
                6
            }
            ConceptKind::All(r, _) => {
                assert!(r.inverse);
                7
            }
            ConceptKind::Min(n, _, _) => {
                assert_eq!(n, 2);
                8
            }
            ConceptKind::Max(n, r, _) => {
                assert_eq!(n, 1);
                assert!(r.inverse);
                9
            }
            ConceptKind::Nominal(mut ids) => {
                assert_eq!(ids.len(), 1);
                assert_eq!(ids.next(), Some(term(&kb, name)));
                assert!(ids.next().is_none());
                10
            }
            ConceptKind::Data(r) => {
                assert!(std::ptr::eq(r, kb.data_ranges.range(range)));
                11
            }
            ConceptKind::SelfRestriction(r) => {
                assert!(r.inverse);
                12
            }
            ConceptKind::RoleState {
                machine,
                state,
                filler,
                negated,
            } => {
                assert_eq!((machine, state, negated), (3, 7, true));
                assert!(matches!(filler.kind(), ConceptKind::Named(_)));
                13
            }
        };
        seen.push(tag);
    }
    assert_eq!(seen, (0..14).collect::<Vec<_>>());
}

#[test]
fn query_symbols_and_recursive_reserved_roots_never_gain_source_identity() {
    let mut kb = Kb::empty();
    let source = kb.interner.intern_iri("https://example.org/source");
    let property = kb.interner.intern_iri("https://example.org/p");
    kb.source_terms = u32::try_from(kb.interner.len()).unwrap();
    let query = kb.interner.intern_iri("https://example.org/query");
    let filler = kb.table.intern(Concept::Top);
    let nested = NodeRef::Reserved(Box::new(super::super::ReservedRef {
        origin: NodeRef::Individual(query),
        role: super::super::ProofRole::of(Role::Inv(property)),
        filler,
        index: 2,
    }));
    let root = NodeRef::Reserved(Box::new(super::super::ReservedRef {
        origin: nested,
        role: super::super::ProofRole::of(Role::Named(property)),
        filler,
        index: 1,
    }));
    assert!(matches!(
        node(&kb, &NodeRef::Individual(source)),
        ResolvedNode::Individual(ResolvedTerm::Source(_))
    ));
    assert!(matches!(
        node(&kb, &NodeRef::Anonymous(5)),
        ResolvedNode::Anonymous(5)
    ));
    let ResolvedNode::Reserved(root) = node(&kb, &root) else {
        panic!("reserved root")
    };
    assert_eq!(root.index(), 1);
    assert!(!root.role().inverse);
    assert!(matches!(root.filler().kind(), ConceptKind::Top));
    let ResolvedNode::Reserved(parent) = root.origin() else {
        panic!("recursive origin")
    };
    assert_eq!(parent.index(), 2);
    assert!(parent.role().inverse);
    assert!(matches!(
        parent.origin(),
        ResolvedNode::Individual(ResolvedTerm::QuerySymbol(_))
    ));
}
