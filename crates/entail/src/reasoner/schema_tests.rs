// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-built production controls, and the same-kernel uncached measurement.

use std::sync::Arc;
use std::time::Duration;

use purrdf_alloc_probe::CurrentThreadWindow;
use purrdf_core::collections::{ListVocab, build_rdf_list};
use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId, TermValue};
use purrdf_iri::vocab::{owl, rdf, rdfs};
use purrdf_testkit::bench::{Arguments, Bench, BenchmarkId};
use purrdf_xsd::datatype::XSD_NON_NEGATIVE_INTEGER;

use super::{
    Certified, DlCompleteness, Reasoner, SchemaObstruction, SchemaPreparationBudget, ServiceProof,
    Verdict,
};

/// Authored schema fixture. Every dynamic name is explicitly in example.org.
#[derive(Default)]
pub(crate) struct Fixture {
    pub(crate) builder: RdfDatasetBuilder,
    next: usize,
}

impl Fixture {
    pub(crate) fn name(&mut self, local: &str) -> TermId {
        self.builder
            .intern_iri(&format!("https://example.org/{local}"))
    }
    pub(crate) fn blank(&mut self) -> TermId {
        let label = format!("schema{}", self.next);
        self.next += 1;
        self.builder.intern_blank(&label, BlankScope::DEFAULT)
    }
    pub(crate) fn edge(&mut self, subject: TermId, predicate: &str, object: TermId) {
        let predicate = self.builder.intern_iri(predicate);
        self.builder.push_quad(subject, predicate, object, None);
    }
    pub(crate) fn instance(&mut self, local: &str, class: TermId) -> TermId {
        let individual = self.name(local);
        self.edge(individual, rdf::TYPE, class);
        individual
    }
    pub(crate) fn bounds(
        &mut self,
        class: TermId,
        role: TermId,
        filler: TermId,
        count: u32,
        minimum: bool,
        data: bool,
    ) {
        let restriction = self.blank();
        let value = self.builder.intern_literal(RdfLiteral {
            lexical_form: count.to_string(),
            datatype: Some(XSD_NON_NEGATIVE_INTEGER.to_owned()),
            language: None,
            direction: None,
        });
        self.edge(restriction, owl::ON_PROPERTY, role);
        self.edge(
            restriction,
            if data {
                owl::ON_DATA_RANGE
            } else {
                owl::ON_CLASS
            },
            filler,
        );
        self.edge(
            restriction,
            if minimum {
                owl::MIN_QUALIFIED_CARDINALITY
            } else {
                owl::MAX_QUALIFIED_CARDINALITY
            },
            value,
        );
        self.edge(class, rdfs::SUB_CLASS_OF, restriction);
    }
    pub(crate) fn some(&mut self, class: TermId, role: TermId, filler: TermId) {
        let restriction = self.blank();
        self.edge(restriction, owl::ON_PROPERTY, role);
        self.edge(restriction, owl::SOME_VALUES_FROM, filler);
        self.edge(class, rdfs::SUB_CLASS_OF, restriction);
    }
    pub(crate) fn list(&mut self, members: &[TermId]) -> TermId {
        let nil = self.builder.intern_iri(rdf::NIL);
        if members.is_empty() {
            return nil;
        }
        let vocab = ListVocab {
            first: self.builder.intern_iri(rdf::FIRST),
            rest: self.builder.intern_iri(rdf::REST),
            nil,
        };
        let cells: Vec<_> = (0..members.len()).map(|_| self.blank()).collect();
        build_rdf_list(
            members.iter().copied(),
            &vocab,
            |index| cells[index],
            |subject, predicate, object| {
                self.builder.push_quad(subject, predicate, object, None);
            },
        )
    }
    pub(crate) fn freeze(self) -> Arc<RdfDataset> {
        self.builder.freeze().expect("authored fixture")
    }
}

pub(crate) fn term(local: &str) -> TermValue {
    TermValue::Iri(format!("https://example.org/{local}"))
}

pub(crate) fn shared_schema(
    restrictions: usize,
    instances: usize,
    contradictory: bool,
) -> Arc<RdfDataset> {
    let mut fixture = Fixture::default();
    let class = fixture.name("A");
    let filler = fixture.name("F");
    for index in 0..restrictions {
        let role = fixture.name(&format!("p{index}"));
        fixture.bounds(class, role, filler, 1, true, false);
        fixture.bounds(class, role, filler, u32::from(!contradictory), false, false);
    }
    for index in 0..instances {
        fixture.instance(&format!("i{index}"), class);
    }
    fixture.freeze()
}

pub(crate) fn checked(dataset: &RdfDataset, answer: &Certified<Verdict>) -> ServiceProof {
    let proof = answer.proof().expect("opt-in recording");
    let encoded = proof.encode();
    let decoded = ServiceProof::decode(&encoded).expect("native canonical proof");
    assert_eq!(decoded.encode(), encoded);
    let mut checker = Reasoner::with_proofs(dataset).expect("consumer-owned source");
    checker
        .prepare(decoded.question())
        .expect("query preparation");
    let context = checker.proof_context().expect("independent context");
    let replay = decoded
        .verify(
            dataset,
            decoded.question(),
            Some(answer.certificate()),
            &context,
        )
        .expect("every necessary prepared-clash premise is independently checked");
    assert_eq!(replay.checks().unattested(), 0);
    decoded
}

#[test]
fn shared_instances_prepare_once_and_warm_no_clash_calls_do_not_copy_proofs() {
    let mut original = None;
    for instances in [1, 64, 256] {
        let dataset = shared_schema(4, instances, false);
        let reasoner = Reasoner::new(&dataset).unwrap();
        let stats = reasoner.schema_preparation();
        assert_eq!(stats.contradictions, 0);
        assert!(stats.classes > 0 && stats.restrictions >= 8 && stats.allocations > 0);
        assert!(reasoner.schema_bounds(&term("A")).is_none());
        if let Some(before) = original {
            assert_eq!(
                stats, before,
                "ABox width cannot cause another schema preparation"
            );
        }
        original = Some(stats);
        for _ in 0..2 {
            assert_eq!(*reasoner.consistency().answer(), Verdict::True);
            assert_eq!(reasoner.schema_preparation(), stats);
        }
    }
}

#[test]
fn a_class_name_does_not_assert_an_inhabitant_but_the_question_supplies_one() {
    let dataset = shared_schema(2, 0, true);
    let mut reasoner = Reasoner::with_proofs(&dataset).unwrap();
    assert!(reasoner.schema_bounds(&term("A")).is_some());
    assert_eq!(*reasoner.consistency().answer(), Verdict::True);
    let answer = reasoner.class_satisfiability(&term("A")).unwrap();
    assert_eq!(*answer.answer(), Verdict::False);
    let proof = checked(&dataset, &answer);
    assert!(
        proof
            .runs()
            .iter()
            .filter_map(|run| run.proof())
            .flat_map(crate::DlProof::clashes)
            .any(|step| step.schema_evidence().is_some())
    );
    assert_eq!(
        *reasoner
            .class_satisfiability(&term("unconstrained"))
            .unwrap()
            .answer(),
        Verdict::True
    );
}

#[test]
fn asserted_subsumed_equivalent_role_derived_and_existential_membership_are_checked() {
    for derivation in 0..5 {
        let mut fixture = Fixture::default();
        let class = fixture.name("A");
        let filler = fixture.name("F");
        let role = fixture.name("p");
        fixture.bounds(class, role, filler, 2, true, false);
        fixture.bounds(class, role, filler, 1, false, false);
        match derivation {
            0 => {
                fixture.instance("i", class);
            }
            1 | 2 => {
                let sub = fixture.name("B");
                fixture.edge(
                    sub,
                    if derivation == 1 {
                        rdfs::SUB_CLASS_OF
                    } else {
                        owl::EQUIVALENT_CLASS
                    },
                    class,
                );
                fixture.instance("i", sub);
            }
            3 => {
                let object = fixture.name("i");
                let subject = fixture.name("s");
                let edge = fixture.name("edge");
                fixture.edge(edge, rdfs::RANGE, class);
                fixture.builder.push_quad(subject, edge, object, None);
            }
            _ => {
                let source = fixture.name("B");
                let edge = fixture.name("edge");
                fixture.some(source, edge, class);
                fixture.instance("s", source);
            }
        }
        let dataset = fixture.freeze();
        let reasoner = Reasoner::with_proofs(&dataset).unwrap();
        let answer = reasoner.consistency();
        assert_eq!(
            *answer.answer(),
            Verdict::False,
            "actual inhabitance derivation {derivation}"
        );
        let proof = checked(&dataset, &answer);
        let schema = proof
            .runs()
            .iter()
            .filter_map(|run| run.proof())
            .flat_map(crate::DlProof::clashes)
            .find_map(|step| step.schema_evidence())
            .expect("the checked current class closes");
        if derivation == 0 {
            assert_eq!(schema.support_steps(), 0);
        } else if derivation <= 2 {
            // Positive subclass closure prepares B itself. Its CURRENT asserted
            // membership plus the checked B→A source law is sufficient; no ABox
            // implication application needs to be invented for the proof.
            assert_eq!(
                schema.bounds().class(),
                reasoner.schema_bounds(&term("B")).unwrap().class()
            );
        } else {
            assert!(
                schema.support_steps() > 0,
                "role/existential-derived membership needs a checked prefix"
            );
        }
    }
}

#[test]
fn qualified_containment_is_directional_and_has_no_unrelated_transfer() {
    for relation in 0..4 {
        let mut fixture = Fixture::default();
        let class = fixture.name("A");
        let lower = fixture.name("F");
        let upper = fixture.name("G");
        let role = fixture.name("p");
        fixture.bounds(class, role, lower, 2, true, false);
        fixture.bounds(class, role, upper, 1, false, false);
        if relation != 0 {
            fixture.edge(
                if relation == 3 { upper } else { lower },
                if relation == 2 {
                    owl::EQUIVALENT_CLASS
                } else {
                    rdfs::SUB_CLASS_OF
                },
                if relation == 3 { lower } else { upper },
            );
        }
        fixture.instance("i", class);
        let dataset = fixture.freeze();
        let reasoner = Reasoner::with_proofs(&dataset).unwrap();
        let expected = if relation == 1 || relation == 2 {
            Verdict::False
        } else {
            Verdict::True
        };
        assert_eq!(
            reasoner.schema_bounds(&term("A")).is_some(),
            expected == Verdict::False
        );
        let answer = reasoner.consistency();
        assert_eq!(*answer.answer(), expected);
        if expected == Verdict::False {
            checked(&dataset, &answer);
        }
    }
}

#[test]
fn exact_data_spaces_and_finite_extents_are_prepared_without_opaque_transfer() {
    for (datatype, minimum, extent) in [
        (purrdf_xsd::datatype::XSD_BOOLEAN, 3, Some(2)),
        (purrdf_xsd::datatype::XSD_BYTE, 257, Some(256)),
        (purrdf_xsd::datatype::XSD_STRING, 3, None),
        (purrdf_xsd::datatype::XSD_DATE_TIME_STAMP, 3, None),
    ] {
        let mut fixture = Fixture::default();
        let class = fixture.name("A");
        let role = fixture.name("dp");
        let filler = fixture.builder.intern_iri(datatype);
        let data_property = fixture.builder.intern_iri(owl::DATATYPE_PROPERTY);
        fixture.edge(role, rdf::TYPE, data_property);
        fixture.bounds(class, role, filler, minimum, true, true);
        if extent.is_none() {
            fixture.bounds(class, role, filler, minimum - 1, false, true);
        }
        fixture.instance("i", class);
        let dataset = fixture.freeze();
        let reasoner = Reasoner::with_proofs(&dataset).unwrap();
        let evidence = reasoner
            .schema_bounds(&term("A"))
            .expect("exact supported datatype");
        if let Some(extent) = extent {
            assert_eq!(
                evidence.upper_bound(),
                super::SchemaUpperBound::DataExtent(extent)
            );
        }
        let answer = reasoner.consistency();
        assert_eq!(*answer.answer(), Verdict::False);
        checked(&dataset, &answer);
    }
    let mut fixture = Fixture::default();
    let class = fixture.name("A");
    let role = fixture.name("dp");
    let opaque = fixture.name("CustomDatatype");
    let datatype = fixture.builder.intern_iri(rdfs::DATATYPE);
    fixture.edge(opaque, rdf::TYPE, datatype);
    let supported = fixture
        .builder
        .intern_iri(purrdf_xsd::datatype::XSD_INTEGER);
    fixture.bounds(class, role, opaque, 2, true, true);
    fixture.bounds(class, role, supported, 1, false, true);
    let dataset = fixture.freeze();
    let reasoner = Reasoner::new(&dataset).unwrap();
    assert!(
        reasoner.schema_bounds(&term("A")).is_none(),
        "unsupported range cannot license inclusion"
    );
}

#[test]
fn exact_qualifier_and_role_sources_are_part_of_the_bound_not_current_edges() {
    for inverted in [false, true] {
        let mut fixture = Fixture::default();
        let class = fixture.name("A");
        let filler = fixture.name("F");
        let narrow = fixture.name("narrow");
        let broad = fixture.name("broad");
        fixture.edge(narrow, rdfs::SUB_PROPERTY_OF, broad);
        fixture.bounds(
            class,
            if inverted { broad } else { narrow },
            filler,
            2,
            true,
            false,
        );
        fixture.bounds(
            class,
            if inverted { narrow } else { broad },
            filler,
            1,
            false,
            false,
        );
        fixture.instance("i", class);
        let dataset = fixture.freeze();
        let reasoner = Reasoner::with_proofs(&dataset).unwrap();
        let answer = reasoner.consistency();
        assert_eq!(
            *answer.answer(),
            if inverted {
                Verdict::True
            } else {
                Verdict::False
            }
        );
        assert_eq!(reasoner.schema_bounds(&term("A")).is_some(), !inverted);
        if !inverted {
            checked(&dataset, &answer);
        }
    }
}

#[test]
fn nominal_names_do_not_supply_cached_distinctness_or_ignore_current_identification() {
    for merged in [false, true] {
        let mut fixture = Fixture::default();
        let class = fixture.name("A");
        let role = fixture.name("p");
        let left = fixture.name("left");
        let right = fixture.name("right");
        let filler = fixture.blank();
        let members = fixture.list(&[left, right]);
        fixture.edge(filler, owl::ONE_OF, members);
        fixture.bounds(class, role, filler, 2, true, false);
        fixture.bounds(class, role, filler, 2, false, false);
        if merged {
            fixture.edge(left, owl::SAME_AS, right);
        }
        fixture.instance("i", class);
        let dataset = fixture.freeze();
        let reasoner = Reasoner::new(&dataset).unwrap();
        assert!(reasoner.schema_bounds(&term("A")).is_none());
        assert_eq!(
            *reasoner.consistency().answer(),
            if merged {
                Verdict::False
            } else {
                Verdict::True
            }
        );
    }
}

#[test]
fn actual_long_datatype_temporaries_share_the_preparation_owner() {
    let mut fixture = Fixture::default();
    let class = fixture.name("A");
    let role = fixture.name("dp");
    let datatype = fixture
        .builder
        .intern_iri(purrdf_xsd::datatype::XSD_INTEGER);
    let range = fixture.blank();
    fixture.edge(range, owl::ON_DATATYPE, datatype);
    let whole = "9".repeat(1024);
    let mut facets = Vec::new();
    for (predicate, lexical) in [
        (
            purrdf_xsd::datatype::XSD_MIN_INCLUSIVE,
            format!("{whole}.1"),
        ),
        (
            purrdf_xsd::datatype::XSD_MAX_INCLUSIVE,
            format!("1{}0.9", "0".repeat(1023)),
        ),
    ] {
        let facet = fixture.blank();
        let literal = fixture.builder.intern_literal(RdfLiteral {
            lexical_form: lexical,
            datatype: Some(purrdf_xsd::datatype::XSD_DECIMAL.to_owned()),
            language: None,
            direction: None,
        });
        fixture.edge(facet, predicate, literal);
        facets.push(facet);
    }
    let list = fixture.list(&facets);
    fixture.edge(range, owl::WITH_RESTRICTIONS, list);
    fixture.bounds(class, role, range, 2, true, true);
    fixture.instance("i", class);
    let dataset = fixture.freeze();
    let reasoner = Reasoner::with_proofs(&dataset).unwrap();
    assert_eq!(
        reasoner.schema_bounds(&term("A")).unwrap().upper_bound(),
        super::SchemaUpperBound::DataExtent(1)
    );
    let answer = reasoner.consistency();
    assert_eq!(*answer.answer(), Verdict::False);
    checked(&dataset, &answer);
    let window = CurrentThreadWindow::open();
    let mut preparation = crate::owl_dl::bounds::Preparation::default();
    preparation
        .extend_until(
            &reasoner.kb,
            SchemaPreparationBudget::default(),
            &mut || Ok::<(), core::convert::Infallible>(()),
        )
        .unwrap();
    let measured = window.sample();
    assert_eq!(
        measured.retained_bytes,
        i64::try_from(preparation.stats.retained_bytes).unwrap()
    );
    assert!(
        measured.peak_working_bytes <= i64::try_from(preparation.stats.peak_bytes).unwrap(),
        "{measured:?} vs {:?}",
        preparation.stats
    );
    assert_eq!(measured.allocations, preparation.stats.allocations);
    drop(preparation);
    assert_eq!(window.sample().retained_bytes, 0);
}

#[test]
fn nominal_identification_and_distinct_successors_remain_per_individual() {
    for distinct in [false, true] {
        let mut fixture = Fixture::default();
        let class = fixture.name("A");
        let filler = fixture.name("F");
        let role = fixture.name("p");
        fixture.bounds(class, role, filler, 1, false, false);
        let source = fixture.instance("i", class);
        let left = fixture.instance("left", filler);
        let right = fixture.instance("right", filler);
        fixture.builder.push_quad(source, role, left, None);
        fixture.builder.push_quad(source, role, right, None);
        if distinct {
            fixture.edge(left, owl::DIFFERENT_FROM, right);
        } else {
            fixture.edge(left, owl::SAME_AS, right);
        }
        let dataset = fixture.freeze();
        let reasoner = Reasoner::new(&dataset).unwrap();
        assert!(reasoner.schema_bounds(&term("A")).is_none());
        assert_eq!(
            *reasoner.consistency().answer(),
            if distinct {
                Verdict::False
            } else {
                Verdict::True
            }
        );
    }
    let mut fixture = Fixture::default();
    let bad = fixture.name("A");
    let filler = fixture.name("F");
    let role = fixture.name("p");
    fixture.bounds(bad, role, filler, 2, true, false);
    fixture.bounds(bad, role, filler, 1, false, false);
    let good = fixture.name("Good");
    let choice = fixture.blank();
    let list = fixture.list(&[bad, good]);
    fixture.edge(choice, owl::UNION_OF, list);
    fixture.instance("i", choice);
    let dataset = fixture.freeze();
    let reasoner = Reasoner::new(&dataset).unwrap();
    assert_eq!(
        *reasoner.consistency().answer(),
        Verdict::True,
        "one rejected union branch cannot empty the other"
    );
}

#[test]
fn retraction_purge_and_source_replacement_cannot_reuse_a_prepared_proof() {
    let original = shared_schema(1, 1, true);
    let original_reasoner = Reasoner::with_proofs(&original).unwrap();
    let original_answer = original_reasoner.consistency();
    let proof = checked(&original, &original_answer);
    for revised in [
        shared_schema(1, 0, true),
        shared_schema(1, 1, false),
        shared_schema(0, 1, false),
    ] {
        let checker = Reasoner::with_proofs(&revised).unwrap();
        assert_eq!(*checker.consistency().answer(), Verdict::True);
        let context = checker.proof_context().unwrap();
        assert!(
            proof
                .verify(
                    &revised,
                    proof.question(),
                    Some(original_answer.certificate()),
                    &context
                )
                .is_err()
        );
    }
    // The same program owner invalidates at the original mutable TBox home too.
    let dataset = shared_schema(1, 0, false);
    let mut reasoner = Reasoner::new(&dataset).unwrap();
    assert!(reasoner.schema_bounds(&term("A")).is_none());
    let class = reasoner.concept_of(&term("A")).expect("query preparation");
    let class = reasoner.kb.table.concept(class).clone();
    let lower = (0..reasoner.kb.table.len())
        .find_map(|id| match *reasoner.kb.table.decomp(id as u32) {
            crate::owl_dl::concept::Decomp::Min(_, role, filler)
            | crate::owl_dl::concept::Decomp::Some(role, filler) => Some((role, filler)),
            _ => None,
        })
        .unwrap();
    let filler = reasoner.kb.table.concept(lower.1).clone();
    let maximum = crate::owl_dl::concept::Concept::Max(0, lower.0, Box::new(filler));
    reasoner.kb.push_gci(class, maximum);
    reasoner.kb.finalize().expect("fixture preparation");
    assert!(reasoner.schema_bounds(&term("A")).is_some());
}

#[test]
fn refused_entries_are_typed_incomplete_and_sufficient_retry_finishes_the_same_source() {
    let dataset = shared_schema(3, 1, true);
    for budget in [
        SchemaPreparationBudget {
            work: Some(0),
            bytes: None,
        },
        SchemaPreparationBudget {
            work: None,
            bytes: Some(0),
        },
    ] {
        let mut reasoner = Reasoner::with_proofs_and_preparation_budget(&dataset, budget).unwrap();
        assert!(reasoner.schema_preparation().obstruction.is_some());
        assert!(match reasoner.schema_preparation().obstruction.unwrap() {
            SchemaObstruction::Work { .. } => budget.work.is_some(),
            SchemaObstruction::Storage { .. } => budget.bytes.is_some(),
            SchemaObstruction::Allocation => false,
        });
        let answer = reasoner.consistency();
        assert_eq!(*answer.answer(), Verdict::Unknown);
        assert_eq!(
            answer.certificate().completeness(),
            DlCompleteness::BudgetExhausted
        );
        let proof = answer.proof().unwrap();
        assert!(
            proof
                .receipt()
                .and_then(super::StopReceipt::schema_obstruction)
                .is_some()
        );
        assert_eq!(
            ServiceProof::decode(&proof.encode()).unwrap().receipt(),
            proof.receipt()
        );
        reasoner
            .retry_schema_preparation(SchemaPreparationBudget::default())
            .unwrap();
        assert!(reasoner.schema_preparation().obstruction.is_none());
        let completed = reasoner.consistency();
        assert_eq!(*completed.answer(), Verdict::False);
        checked(&dataset, &completed);
    }
}

#[test]
fn stopping_an_existing_owner_does_not_publish_clear_or_detach_pins() {
    #[derive(Debug)]
    struct Stop;
    impl purrdf_datalog::StopSignal for Stop {
        fn stopped(&self) -> bool {
            true
        }
    }
    let dataset = shared_schema(2, 1, true);
    assert!(matches!(
        Reasoner::with_stop(&dataset, Arc::new(Stop)),
        Err(crate::EntailError::Stopped)
    ));
    let window = CurrentThreadWindow::open();
    {
        let mut reasoner = Reasoner::with_preparation_budget(
            &dataset,
            SchemaPreparationBudget {
                work: Some(1),
                bytes: None,
            },
        )
        .unwrap();
        reasoner.kb.stop = Some(Arc::new(Stop));
        assert!(matches!(
            reasoner.retry_schema_preparation(SchemaPreparationBudget::default()),
            Err(crate::EntailError::Stopped)
        ));
        assert_eq!(*reasoner.consistency().answer(), Verdict::Unknown);
    }
    assert_eq!(window.sample().retained_bytes, 0);
    assert_eq!(
        Arc::strong_count(&dataset),
        1,
        "there is no unreturned input/session pin"
    );
}

#[test]
fn cached_and_uncached_native_membership_paths_agree_and_check_the_same_premises() {
    for contradictory in [false, true] {
        let dataset = shared_schema(4, 16, contradictory);
        let prepared = Reasoner::with_proofs(&dataset).unwrap();
        let mut reference = Reasoner::with_proofs(&dataset).unwrap();
        reference.kb.schema_reference = true;
        let actual = prepared.consistency();
        let expected = reference.consistency();
        assert_eq!(actual.answer(), expected.answer());
        assert_eq!(
            actual.proof().unwrap().encode(),
            expected.proof().unwrap().encode(),
            "same class evidence and current-support proof, not merely the verdict"
        );
        assert!(reference.kb.schema_reference_stats.get().classes > 0);
        if contradictory {
            checked(&dataset, &actual);
            checked(&dataset, &expected);
        } else {
            assert_eq!(reference.kb.schema_reference_stats.get().contradictions, 0);
        }
    }
}

/// Report-only, O3 via the normal release profile. The uncached control is inside
/// this native test crate; no public execution mode or alternate algorithm ships.
#[test]
#[ignore = "report-only O3 time/allocation sweeps, run after semantic qualification"]
fn schema_preparation_o3_scaling() {
    assert!(
        !std::hint::black_box(cfg!(debug_assertions)),
        "qualify using the exact workspace release/O3 profile"
    );
    let mut bench = Bench::with_arguments(Arguments {
        quick: true,
        ..Arguments::default()
    })
    .warm_up_time(Duration::from_millis(500))
    .measurement_time(Duration::from_secs(1));
    let mut group = bench.benchmark_group("schema-preparation");
    for restrictions in [1, 8, 32] {
        for instances in [1, 16, 64] {
            let dataset = shared_schema(restrictions, instances, false);
            let inputs = Reasoner::new(&dataset).unwrap();
            let prepare = |kb: &crate::owl_dl::Kb| {
                let mut table = crate::owl_dl::bounds::Preparation::default();
                table
                    .extend_until(kb, SchemaPreparationBudget::default(), &mut || {
                        Ok::<(), core::convert::Infallible>(())
                    })
                    .unwrap();
                table
            };
            let window = CurrentThreadWindow::open();
            let table = prepare(&inputs.kb);
            let measured = window.sample();
            assert_eq!(
                measured.retained_bytes,
                i64::try_from(table.stats.retained_bytes).unwrap()
            );
            assert!(measured.peak_working_bytes <= i64::try_from(table.stats.peak_bytes).unwrap());
            let preparation = table.stats;
            drop(table);
            assert_eq!(window.sample().retained_bytes, 0);
            drop(window);
            println!(
                "cold schema owners restrictions={restrictions} instances={instances} preparation={:?} allocations={} traffic={} peak={} retained={}",
                preparation,
                measured.allocations,
                measured.requested_bytes,
                measured.peak_working_bytes,
                measured.retained_bytes
            );
            group.bench_with_input(
                BenchmarkId::new("cold-preparation", format!("r{restrictions}-n{instances}")),
                &inputs.kb,
                |bencher, kb| bencher.iter(|| std::hint::black_box(prepare(kb))),
            );
            group.bench_with_input(
                BenchmarkId::new("cold-reasoner", format!("r{restrictions}-n{instances}")),
                &dataset,
                |bencher, dataset| {
                    bencher.iter(|| std::hint::black_box(Reasoner::new(dataset).unwrap()));
                },
            );
            for uncached in [false, true] {
                let mut reasoner = Reasoner::new(&dataset).unwrap();
                reasoner.kb.schema_reference = uncached;
                let window = CurrentThreadWindow::open();
                let result = reasoner.consistency();
                assert_eq!(*result.answer(), Verdict::True);
                let measured = window.sample();
                drop(window);
                println!(
                    "schema owners restrictions={restrictions} instances={instances} uncached={uncached} preparation={:?} repeated={:?} allocations={} traffic={} peak={} retained={}",
                    reasoner.schema_preparation(),
                    reasoner.kb.schema_reference_stats.get(),
                    measured.allocations,
                    measured.requested_bytes,
                    measured.peak_working_bytes,
                    measured.retained_bytes
                );
                group.bench_with_input(
                    BenchmarkId::new(
                        if uncached { "uncached" } else { "prepared" },
                        format!("r{restrictions}-n{instances}"),
                    ),
                    &reasoner,
                    |bencher, reasoner| {
                        bencher.iter(|| std::hint::black_box(reasoner.consistency()));
                    },
                );
            }
        }
    }
    group.finish();
    print!("{}", bench.console());
    let outcomes = bench.outcomes();
    assert_eq!(outcomes.len(), 36, "every row of the two-axis sweep ran");
    assert!(
        outcomes
            .iter()
            .all(|outcome| outcome.failure.is_none() && outcome.estimates.is_some()),
        "the standard benchmark measurements and saved estimates must all succeed: {}",
        bench.console(),
    );
}
