// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Positive source revisions retain native state and independently checkable proofs.

use std::cell::Cell;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use purrdf_core::term_fixture::{BudgetedView, ProbeFault, build_page, intern_value, iri};
use purrdf_core::{
    CanonicalizationControl, DatasetStateDigest, DatasetView, FallibleDatasetView, GraphMatch,
    QuadIds, RDFC_V2_CORPUS_POLICY, RdfDataset, RdfDatasetBuilder, TermId, TermRef, TermValue,
    ViewOperationStatus,
};
use purrdf_entail::reasoner::{
    DlAxiom, PositiveUpdateRefusal, PositiveWorld, PositiveWorldError, PositiveWorldLimits,
    Question, Reasoner, SchemaPreparationBudget, ServiceProof, Verdict,
};
use purrdf_iri::vocab::{owl, rdf, rdfs};

fn control() -> CanonicalizationControl<'static> {
    CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None)
}

fn limits() -> PositiveWorldLimits {
    PositiveWorldLimits {
        history_steps: 64,
        source_terms: 4096,
        source_rows: 4096,
        question_bytes: 1_048_576,
        metadata_bytes: 16_777_216,
        metadata_work: 1_000_000,
    }
}

/// Repeat physical rows or refuse real workspace requests while preserving the
/// complete resident statement families. These are provider-contract boundaries.
struct Source {
    data: Arc<RdfDataset>,
    repeat: bool,
    refuse_workspace: bool,
    failure: Cell<Option<&'static str>>,
    stop_after_row: Option<Arc<Stop>>,
}

impl Source {
    fn new(data: Arc<RdfDataset>) -> Self {
        Self {
            data,
            repeat: false,
            refuse_workspace: false,
            failure: Cell::new(None),
            stop_after_row: None,
        }
    }
}

impl DatasetView for Source {
    type Id = TermId;
    type ReadError = ProbeFault;
    type TermGuard<'a>
        = TermRef<'a, TermId>
    where
        Self: 'a;
    type ProbePlan = ();
    fn read_error(&self) -> Option<ProbeFault> {
        self.failure.get().map(ProbeFault)
    }
    fn resolve(&self, id: TermId) -> Result<Self::TermGuard<'_>, ProbeFault> {
        Ok(self.data.as_ref().resolve(id))
    }
    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, ProbeFault> {
        Ok(self.data.as_ref().term_id_by_value(value))
    }
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.data
            .quads()
            .flat_map(|q| std::iter::once(q).chain(self.repeat.then_some(q)))
            .inspect(|_| {
                if let Some(stop) = &self.stop_after_row {
                    stop.0.store(true, Ordering::Relaxed);
                }
            })
    }
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.data.reifier_quads()
    }
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.data.annotation_quads()
    }
    fn named_graphs(&self) -> impl Iterator<Item = TermId> + '_ {
        self.data.named_graphs()
    }
    fn capabilities(&self) -> purrdf_core::RdfStoreCapabilities {
        self.data.capabilities()
    }
    fn term_count(&self) -> u64 {
        self.data.term_count()
    }
    fn probe_plan(&self, _s: bool, _p: bool, _o: bool, _g: GraphMatch) {}
    fn quads_for_pattern_with_plan(
        &self,
        _plan: &(),
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.quads_for_pattern(s, p, o, g)
    }
    fn reserve_workspace(
        &self,
        _bytes: u64,
    ) -> Result<impl purrdf_core::WorkspaceReservation<Error = ProbeFault> + '_, ProbeFault> {
        if self.refuse_workspace {
            self.failure.set(Some("source workspace refused"));
            return Err(ProbeFault("source workspace refused"));
        }
        Ok(purrdf_core::NoopReservation::<ProbeFault>::default())
    }
    fn reserve_owned_workspace(
        &self,
        _bytes: u64,
    ) -> Option<Result<purrdf_core::OwnedWorkspaceReservation<ProbeFault>, ProbeFault>> {
        // This provider's data and caller-selected metadata are resident.
        Some(Ok(purrdf_core::NoopReservation::owned()))
    }
}

impl FallibleDatasetView for Source {
    type Error = ProbeFault;
    type Evidence = u64;
    fn operation_status(&self) -> ViewOperationStatus<ProbeFault, u64> {
        self.read_error()
            .map_or(ViewOperationStatus::Ready { evidence: 917 }, |error| {
                ViewOperationStatus::Failed {
                    error,
                    evidence: 917,
                }
            })
    }
}

#[derive(Debug)]
struct Stop(AtomicBool);
impl purrdf_datalog::StopSignal for Stop {
    fn stopped(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// The physical-row contract permits duplicate visits, but RDF identities and
/// native assertion provenance remain set identities through every revision.
#[test]
fn repeated_source_rows_roundtrip_and_normalized_same_set_is_unchanged() {
    let data = build_page(&[(iri("a"), TermValue::iri(rdf::TYPE), iri("A"))]).unwrap();
    let mut repeated = Source::new(data.clone());
    repeated.repeat = true;
    let single = Source::new(data);
    assert_eq!(
        DatasetStateDigest::from_view(&repeated, &mut control()).unwrap(),
        DatasetStateDigest::from_view(&single, &mut control()).unwrap()
    );
    let world = PositiveWorld::with_proofs(repeated, limits(), &mut control()).unwrap();
    assert_eq!(world.work().interpreted_default_rows, 1);
    assert_eq!(world.work().copied_terms, 6);
    let answer = world.consistency().unwrap();
    let proof = ServiceProof::decode_positive(&answer.proof().unwrap().encode(), limits()).unwrap();
    let context = proof
        .prepare_positive_context(
            &single,
            &Question::Consistency,
            limits(),
            SchemaPreparationBudget::default(),
            None,
            &mut control(),
        )
        .unwrap();
    proof
        .verify_prepared(&Question::Consistency, Some(answer.certificate()), &context)
        .unwrap();
    let world = world.advance(single, &mut control()).unwrap();
    assert_eq!(world.work().interpreted_default_rows, 0);
    assert_eq!(world.work().copied_terms, 0);
    assert!(!world.work().identity_recomputed);
}

/// Source checkpoints, metadata admission and cancellation must refuse complete
/// operations with their original causes and without publishing a successor.
#[test]
fn update_and_consumer_refusals_retain_source_evidence_and_explicit_limits() {
    let data = build_page(&[(iri("a"), TermValue::iri(rdf::TYPE), iri("A"))]).unwrap();
    let world = PositiveWorld::new(BudgetedView::new(data.clone(), usize::MAX), limits()).unwrap();
    assert!(
        matches!(PositiveWorld::with_proofs(BudgetedView::new(data.clone(), 1), limits(), &mut control()), Err(PositiveWorldError::Drain(failure)) if failure.evidence == 0 && failure.error == ProbeFault("the probe view exhausted its row budget"))
    );
    assert!(
        matches!(world.advance(BudgetedView::new(data.clone(), 0), &mut control()), Err(PositiveWorldError::Drain(failure)) if failure.evidence == 0 && failure.error == ProbeFault("the probe view exhausted its row budget"))
    );
    let mut small = limits();
    small.metadata_bytes = 0;
    assert!(
        matches!(PositiveWorld::new(data.clone(), small), Err(PositiveWorldError::Entail(purrdf_entail::EntailError::PositiveUpdate(refusal))) if matches!(*refusal, PositiveUpdateRefusal::RetentionLimit { resource: "metadata bytes", .. }))
    );
    let world = PositiveWorld::with_proofs(data.clone(), limits(), &mut control()).unwrap();
    let proof = world.consistency().unwrap().proof().unwrap().clone();
    let mut refused = Source::new(data.clone());
    refused.refuse_workspace = true;
    assert!(
        matches!(proof.prepare_positive_context(&refused, &Question::Consistency, limits(), SchemaPreparationBudget::default(), None, &mut control()), Err(PositiveWorldError::State(purrdf_core::DatasetStateError::Operation(failure))) if failure.evidence == 917 && failure.error == ProbeFault("source workspace refused"))
    );
    let mut no_work = limits();
    no_work.metadata_work = 0;
    assert!(matches!(
        proof.prepare_positive_context(
            &data,
            &Question::Consistency,
            no_work,
            SchemaPreparationBudget::default(),
            None,
            &mut control()
        ),
        Err(PositiveWorldError::Limit {
            resource: "metadata work",
            ..
        })
    ));
    let mut no_metadata = limits();
    no_metadata.metadata_bytes = 0;
    assert!(matches!(
        proof.prepare_positive_context(
            &data,
            &Question::Consistency,
            no_metadata,
            SchemaPreparationBudget::default(),
            None,
            &mut control()
        ),
        Err(PositiveWorldError::Limit {
            resource: "metadata bytes",
            ..
        })
    ));
    let mut short_history = limits();
    short_history.history_steps = 2;
    let short = PositiveWorld::with_proofs(data.clone(), short_history, &mut control())
        .unwrap()
        .prepare(&Question::Consistency)
        .unwrap();
    assert!(
        matches!(short.prepare(&Question::Consistency), Err(purrdf_entail::EntailError::PositiveUpdate(refusal)) if matches!(*refusal, PositiveUpdateRefusal::RetentionLimit { resource: "history steps", .. }))
    );
    let stop = Arc::new(Stop(AtomicBool::new(false)));
    let prepared = PositiveWorld::with_stop(
        data.clone(),
        limits(),
        SchemaPreparationBudget::default(),
        stop.clone(),
    )
    .unwrap();
    let world = PositiveWorld::with_stop(
        data.clone(),
        limits(),
        SchemaPreparationBudget::default(),
        stop.clone(),
    )
    .unwrap();
    stop.0.store(true, Ordering::Relaxed);
    assert!(matches!(
        prepared.prepare(&Question::Consistency),
        Err(purrdf_entail::EntailError::Stopped)
    ));
    assert!(matches!(
        world.advance(data.clone(), &mut control()),
        Err(PositiveWorldError::Entail(
            purrdf_entail::EntailError::Stopped
        ))
    ));
    assert!(matches!(
        proof.prepare_positive_context(
            &data,
            &Question::Consistency,
            limits(),
            SchemaPreparationBudget::default(),
            Some(stop),
            &mut control()
        ),
        Err(PositiveWorldError::Entail(
            purrdf_entail::EntailError::Stopped
        ))
    ));
    let checking = Arc::new(Stop(AtomicBool::new(false)));
    let checked_context = proof
        .prepare_positive_context(
            &data,
            &Question::Consistency,
            limits(),
            SchemaPreparationBudget::default(),
            Some(checking.clone()),
            &mut control(),
        )
        .unwrap();
    checking.0.store(true, Ordering::Relaxed);
    assert!(matches!(
        proof.verify_prepared(&Question::Consistency, None, &checked_context),
        Err(purrdf_entail::DlProofError::Stopped)
    ));
    assert!(matches!(
        proof.verify_prepared_causal(&Question::Consistency, None, &checked_context),
        Err(purrdf_entail::DlProofError::Stopped)
    ));
    let during = Arc::new(Stop(AtomicBool::new(false)));
    let world = PositiveWorld::with_stop(
        Source::new(data.clone()),
        limits(),
        SchemaPreparationBudget::default(),
        during.clone(),
    )
    .unwrap();
    let mut next = Source::new(data.clone());
    next.stop_after_row = Some(during);
    assert!(matches!(
        world.advance(next, &mut control()),
        Err(PositiveWorldError::Entail(
            purrdf_entail::EntailError::Stopped
        ))
    ));
    let schema_change = build_page(&[
        (iri("a"), TermValue::iri(rdf::TYPE), iri("A")),
        (iri("A"), TermValue::iri(rdfs::SUB_CLASS_OF), iri("B")),
    ])
    .unwrap();
    assert!(
        matches!(PositiveWorld::new(data, limits()).unwrap().advance(schema_change, &mut control()), Err(PositiveWorldError::Entail(purrdf_entail::EntailError::PositiveUpdate(refusal))) if matches!(*refusal, PositiveUpdateRefusal::FrozenSchema { .. }))
    );
}

/// Every delta retains the native datatype/key/equality/nominal owners and can
/// produce independently checked causal support after provider IDs reorder.
#[test]
fn positive_batches_match_cold_literal_key_equality_and_causal_owners() {
    let mut rows = vec![
        (iri("C"), TermValue::iri(rdfs::SUB_CLASS_OF), iri("B")),
        (iri("C"), TermValue::iri(owl::HAS_KEY), iri("key-list")),
        (iri("key-list"), TermValue::iri(rdf::FIRST), iri("p")),
        (
            iri("key-list"),
            TermValue::iri(rdf::REST),
            TermValue::iri(rdf::NIL),
        ),
        (
            iri("p"),
            TermValue::iri(rdf::TYPE),
            TermValue::iri(owl::DATATYPE_PROPERTY),
        ),
        (
            iri("q"),
            TermValue::iri(rdf::TYPE),
            TermValue::iri(owl::DATATYPE_PROPERTY),
        ),
        (
            iri("q"),
            TermValue::iri(rdf::TYPE),
            TermValue::iri(owl::FUNCTIONAL_PROPERTY),
        ),
        (
            iri("N"),
            TermValue::iri(owl::EQUIVALENT_CLASS),
            iri("enumeration"),
        ),
        (
            iri("enumeration"),
            TermValue::iri(owl::ONE_OF),
            iri("nominal-list"),
        ),
        (iri("nominal-list"), TermValue::iri(rdf::FIRST), iri("a")),
        (
            iri("nominal-list"),
            TermValue::iri(rdf::REST),
            TermValue::iri(rdf::NIL),
        ),
        (iri("a"), TermValue::iri(rdf::TYPE), iri("C")),
        (
            iri("a"),
            iri("p"),
            TermValue::typed_literal("01", purrdf_xsd::datatype::XSD_INTEGER),
        ),
        (
            iri("a"),
            iri("q"),
            TermValue::typed_literal("01", purrdf_xsd::datatype::XSD_INTEGER),
        ),
        (iri("b"), TermValue::iri(rdf::TYPE), iri("C")),
        (
            iri("b"),
            iri("p"),
            TermValue::typed_literal("2", purrdf_xsd::datatype::XSD_INTEGER),
        ),
    ];
    let initial = build_page(&rows).unwrap();
    let mut world = PositiveWorld::with_proofs(initial, limits(), &mut control()).unwrap();
    world = world.class_satisfiability(&iri("later")).unwrap().0;
    let batches = [
        vec![
            (
                iri("b"),
                iri("p"),
                TermValue::typed_literal("1.0", purrdf_xsd::datatype::XSD_DECIMAL),
            ),
            (
                iri("b"),
                iri("q"),
                TermValue::typed_literal("1.0", purrdf_xsd::datatype::XSD_DECIMAL),
            ),
            (iri("later"), TermValue::iri(rdf::TYPE), iri("C")),
            (iri("later"), iri("related"), iri("a")),
        ],
        vec![(iri("later"), TermValue::iri(owl::SAME_AS), iri("b"))],
        vec![(
            iri("disconnected"),
            TermValue::iri(owl::DIFFERENT_FROM),
            iri("a"),
        )],
    ];
    for (ordinal, batch) in batches.into_iter().enumerate() {
        let delta = batch.len();
        rows.extend(batch);
        let mut builder = RdfDatasetBuilder::new();
        builder
            .intern_iri(&format!("http://example.org/unused-{ordinal}"))
            .unwrap();
        for (s, p, o) in rows.iter().rev() {
            let s = intern_value(&mut builder, s).unwrap();
            let p = intern_value(&mut builder, p).unwrap();
            let o = intern_value(&mut builder, o).unwrap();
            builder.push_quad(s, p, o, None).unwrap();
        }
        let latest = builder.freeze().unwrap();
        world = world.advance(latest.clone(), &mut control()).unwrap();
        assert_eq!(
            world.work().interpreted_default_rows,
            u64::try_from(delta).unwrap()
        );
        let axiom = if ordinal == 0 {
            DlAxiom::SameIndividual {
                left: iri("a"),
                right: iri("b"),
            }
        } else {
            DlAxiom::ClassAssertion {
                individual: iri("later"),
                class: iri("N"),
            }
        };
        let mut cold = Reasoner::new(&latest).unwrap();
        let expected = cold.entails(&axiom).unwrap();
        let (next, answer) = world.entails(&axiom).unwrap();
        world = next;
        assert_eq!(*expected.answer(), Verdict::True);
        assert_eq!(answer.answer(), expected.answer());
        let proof = answer.proof().unwrap();
        let question = Question::AxiomEntailment {
            axiom: Box::new(axiom),
        };
        let context = proof
            .prepare_positive_context(
                &latest,
                &question,
                limits(),
                SchemaPreparationBudget::default(),
                None,
                &mut control(),
            )
            .unwrap();
        proof
            .verify_prepared(&question, Some(answer.certificate()), &context)
            .unwrap();
        let causal = proof
            .verify_prepared_causal(&question, Some(answer.certificate()), &context)
            .unwrap();
        assert!(causal.refutations().len() > 0);
    }
}

fn layers(base: &RdfDataset, mask: u8) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(base).unwrap();
    let s = intern_value(&mut builder, &iri("note")).unwrap();
    let p = intern_value(&mut builder, &iri("says")).unwrap();
    let o = intern_value(&mut builder, &iri("object")).unwrap();
    let graph = intern_value(&mut builder, &iri("named")).unwrap();
    builder.intern_iri(rdf::REIFIES).unwrap();
    let empty = intern_value(&mut builder, &iri("empty")).unwrap();
    if mask & 1 != 0 {
        builder.push_quad(s, p, o, Some(graph)).unwrap();
    }
    let quoted = builder.intern_triple(s, p, o).unwrap();
    let reifier = intern_value(&mut builder, &iri("reifier")).unwrap();
    if mask & 2 != 0 {
        builder.push_reifier(reifier, quoted).unwrap();
    }
    if mask & 4 != 0 {
        builder.push_annotation(reifier, p, o).unwrap();
    }
    if mask & 8 != 0 {
        builder.declare_named_graph(empty).unwrap();
    }
    builder.freeze().unwrap()
}

/// Complete world identity includes every source family and empty declarations;
/// these ignored semantic families must also retain exact append-only aliases.
#[test]
fn all_source_families_and_empty_graphs_bind_proofs_and_refuse_removal() {
    use purrdf_entail::reasoner::positive::PositiveSourceLayer;
    let base = build_page(&[(iri("a"), TermValue::iri(rdf::TYPE), iri("A"))]).unwrap();
    let mut world = PositiveWorld::with_proofs(base.clone(), limits(), &mut control()).unwrap();
    let original = world.consistency().unwrap().proof().unwrap().clone();
    let empty = layers(&base, 8);
    world = world.advance(empty.clone(), &mut control()).unwrap();
    assert_eq!(world.work().interpreted_default_rows, 0);
    assert!(
        original
            .prepare_positive_context(
                &empty,
                &Question::Consistency,
                limits(),
                SchemaPreparationBudget::default(),
                None,
                &mut control()
            )
            .is_err()
    );
    let empty_answer = world.consistency().unwrap();
    let empty_context = empty_answer
        .proof()
        .unwrap()
        .prepare_positive_context(
            &empty,
            &Question::Consistency,
            limits(),
            SchemaPreparationBudget::default(),
            None,
            &mut control(),
        )
        .unwrap();
    let encoded = original.encode();
    let header = 8 + b"purrdf-dl-positive-service-proof-v1".len();
    let native_length =
        usize::try_from(purrdf_core::bytes::read_u64_le(&encoded, header).unwrap()).unwrap();
    let stripped = ServiceProof::decode(&encoded[header + 8..header + 8 + native_length]).unwrap();
    assert!(
        stripped
            .verify_prepared(&Question::Consistency, None, &empty_context)
            .is_err()
    );
    assert!(
        stripped
            .verify_prepared_causal(&Question::Consistency, None, &empty_context)
            .is_err()
    );
    let all = layers(&base, 15);
    world = world.advance(all.clone(), &mut control()).unwrap();
    assert_eq!(world.work().interpreted_default_rows, 0);
    let answer = world.consistency().unwrap();
    let proof = answer.proof().unwrap();
    let context = proof
        .prepare_positive_context(
            &all,
            &Question::Consistency,
            limits(),
            SchemaPreparationBudget::default(),
            None,
            &mut control(),
        )
        .unwrap();
    proof
        .verify_prepared(&Question::Consistency, Some(answer.certificate()), &context)
        .unwrap();
    for (bit, family) in [
        (1, PositiveSourceLayer::Ordinary),
        (2, PositiveSourceLayer::Reifier),
        (4, PositiveSourceLayer::Annotation),
    ] {
        let world = PositiveWorld::new(all.clone(), limits()).unwrap();
        assert!(
            matches!(world.advance(layers(&base, 15 ^ bit), &mut control()), Err(PositiveWorldError::Entail(purrdf_entail::EntailError::PositiveUpdate(refusal))) if matches!(*refusal, PositiveUpdateRefusal::RemovedStatement { layer, .. } if layer == family))
        );
    }
    let world = PositiveWorld::new(all, limits()).unwrap();
    assert!(
        matches!(world.advance(layers(&base, 7), &mut control()), Err(PositiveWorldError::Entail(purrdf_entail::EntailError::PositiveUpdate(refusal))) if matches!(*refusal, PositiveUpdateRefusal::MissingTerm { .. } | PositiveUpdateRefusal::RemovedGraph { .. }))
    );
}

/// Ignored named-graph literals cannot become data assertions through old query
/// nominals when a later ordinary assertion is decoded.
#[test]
fn query_nominal_promoted_only_in_named_graph_does_not_contaminate_the_abox() {
    let rows = vec![
        (iri("A"), TermValue::iri(rdfs::SUB_CLASS_OF), iri("B")),
        (iri("first"), TermValue::iri(rdf::TYPE), iri("A")),
    ];
    let initial = build_page(&rows).unwrap();
    let invalid = TermValue::typed_literal("not-an-integer", purrdf_xsd::datatype::XSD_INTEGER);
    let world = PositiveWorld::new(initial.clone(), limits())
        .unwrap()
        .prepare(&Question::AxiomEntailment {
            axiom: Box::new(DlAxiom::DifferentIndividuals {
                left: iri("first"),
                right: invalid.clone(),
            }),
        })
        .unwrap();
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(&initial).unwrap();
    let subject = intern_value(&mut builder, &iri("note")).unwrap();
    let predicate = intern_value(&mut builder, &iri("says")).unwrap();
    let object = intern_value(&mut builder, &invalid).unwrap();
    let graph = intern_value(&mut builder, &iri("ignored")).unwrap();
    builder
        .push_quad(subject, predicate, object, Some(graph))
        .unwrap();
    let later = intern_value(&mut builder, &iri("later")).unwrap();
    let ty = intern_value(&mut builder, &TermValue::iri(rdf::TYPE)).unwrap();
    let class = intern_value(&mut builder, &iri("A")).unwrap();
    builder.push_quad(later, ty, class, None).unwrap();
    let latest = builder.freeze().unwrap();
    let world = world.advance(latest.clone(), &mut control()).unwrap();
    let cold = Reasoner::new(&latest).unwrap();
    assert_eq!(*cold.consistency().answer(), Verdict::True);
    assert_eq!(*world.consistency().unwrap().answer(), Verdict::True);
}

/// Positive admission requires an unambiguous functional expression in either
/// source order; legal multiple named inverse-property axioms remain admitted.
#[test]
fn ambiguous_functional_expression_refuses_both_source_orders() {
    for (first, second) in [(iri("B"), iri("C")), (iri("C"), iri("B"))] {
        let rows = vec![
            (
                iri("A"),
                TermValue::iri(rdfs::SUB_CLASS_OF),
                iri("expression"),
            ),
            (
                iri("expression"),
                TermValue::iri(owl::COMPLEMENT_OF),
                first.clone(),
            ),
            (
                iri("expression"),
                TermValue::iri(owl::COMPLEMENT_OF),
                second.clone(),
            ),
        ];
        let error = PositiveWorld::new(build_page(&rows).unwrap(), limits())
            .expect_err("a retained positive schema must have stable operands");
        let PositiveWorldError::Entail(purrdf_entail::EntailError::OwlSyntax(error)) = error else {
            panic!("expected the original native ambiguity cause");
        };
        let purrdf_entail::OwlSyntaxError::AmbiguousOperand {
            node,
            predicate,
            operands,
        } = *error
        else {
            panic!("expected both original competing operands");
        };
        assert_eq!(node, iri("expression"));
        assert_eq!(predicate, TermValue::iri(owl::COMPLEMENT_OF));
        assert!(operands.contains(&first) && operands.contains(&second));
    }
    let legal = build_page(&[
        (iri("p"), TermValue::iri(owl::INVERSE_OF), iri("q")),
        (iri("p"), TermValue::iri(owl::INVERSE_OF), iri("r")),
    ])
    .unwrap();
    assert_eq!(
        *PositiveWorld::new(legal, limits())
            .unwrap()
            .consistency()
            .unwrap()
            .answer(),
        Verdict::True
    );
}

/// An unrelated prior query must not make the next world's honest proof depend
/// on the producer's private interning history.
#[test]
fn latest_source_consumer_checks_a_delta_proof_after_a_query_symbol_hole() {
    let mut rows = vec![
        (iri("A"), TermValue::iri(rdfs::SUB_CLASS_OF), iri("B")),
        (iri("first"), TermValue::iri(rdf::TYPE), iri("A")),
    ];
    let initial = build_page(&rows).unwrap();
    let mut world = PositiveWorld::with_proofs(initial, limits(), &mut control()).unwrap();
    let initial_proof = world.consistency().unwrap().proof().unwrap().clone();
    world = world.class_satisfiability(&iri("query-only")).unwrap().0;
    rows.push((iri("later"), TermValue::iri(rdf::TYPE), iri("A")));
    let latest = build_page(&rows).unwrap();
    world = world.advance(latest.clone(), &mut control()).unwrap();
    assert_eq!(world.work().interpreted_default_rows, 1);
    assert_eq!(world.work().copied_terms, 1);
    let axiom = DlAxiom::ClassAssertion {
        individual: iri("later"),
        class: iri("B"),
    };
    let (next, answer) = world.entails(&axiom).unwrap();
    world = next;
    assert_eq!(*answer.answer(), Verdict::True);
    let proof = answer.proof().unwrap().clone();
    let question = Question::AxiomEntailment {
        axiom: Box::new(axiom),
    };
    let mut consumer = Reasoner::with_proofs(&latest, &mut control()).unwrap();
    consumer.prepare(&question).unwrap();
    let context = consumer.proof_context().unwrap();
    assert!(
        initial_proof
            .verify(
                &latest,
                &Question::Consistency,
                None,
                &context,
                &mut control()
            )
            .is_err()
    );
    assert!(proof.verify_prepared(&question, None, &context).is_err());
    assert!(
        initial_proof
            .prepare_positive_context(
                &latest,
                &Question::Consistency,
                limits(),
                SchemaPreparationBudget::default(),
                None,
                &mut control()
            )
            .is_err()
    );
    let bytes = proof.encode();
    assert!(ServiceProof::decode(&bytes).is_err());
    let decoded = ServiceProof::decode_positive(&bytes, limits()).unwrap();
    assert_eq!(decoded, proof);
    let aligned = decoded
        .prepare_positive_context(
            &latest,
            &question,
            limits(),
            SchemaPreparationBudget::default(),
            None,
            &mut control(),
        )
        .unwrap();
    decoded.verify_prepared(&question, None, &aligned).unwrap();
    let packed = purrdf_core::PackBuilder::build_bytes(&latest, &mut control()).unwrap();
    let packed = purrdf_core::PackView::from_bytes(&packed).unwrap();
    let packed_context = decoded
        .prepare_positive_context(
            &packed,
            &question,
            limits(),
            SchemaPreparationBudget::default(),
            None,
            &mut control(),
        )
        .unwrap();
    decoded
        .verify_prepared(&question, None, &packed_context)
        .unwrap();
    let unchanged = world.advance(latest, &mut control()).unwrap();
    assert_eq!(unchanged.work().interpreted_default_rows, 0);
    assert_eq!(unchanged.work().copied_terms, 0);
    assert!(!unchanged.work().identity_recomputed);
}
