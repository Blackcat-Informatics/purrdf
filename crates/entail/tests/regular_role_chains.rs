// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual public reasoning over regular role inclusions, not an RDF-list boundary.

use std::sync::Arc;

use purrdf_core::term_fixture::iri;
use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermId, TermValue};
use purrdf_entail::reasoner::{DlAxiom, DlCompleteness, Question, Reasoner, Verdict};
use purrdf_entail::{EntailError, RoleHierarchyError};
use purrdf_iri::vocab::{owl, rdf, rdfs};

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn role_refusals_keep_native_fields_after_the_source_drops() {
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_core::collections::ListErrorKind;
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::Capture;

    let mut ontology = Ontology::default();
    let property = ontology.named("original-property");
    let cell = ontology.blank();
    ontology.relation(property, owl::PROPERTY_CHAIN_AXIOM, cell);
    ontology.relation(cell, rdf::FIRST, property);
    ontology.relation(cell, rdf::REST, cell);
    let dataset = ontology.finish();
    let error = match Reasoner::new(&dataset) {
        Err(EntailError::RoleHierarchy(error)) => error,
        result => panic!("expected native role-list refusal, got {result:?}"),
    };
    drop(dataset);
    let (original, presentation) = match &error {
        RoleHierarchyError::Witness {
            original,
            presentation,
        } => (original, presentation),
        other => panic!("source-bearing refusal lost its witness: {other:?}"),
    };
    let node = match original.as_ref() {
        RoleHierarchyError::MalformedList {
            node,
            kind: ListErrorKind::Cycle,
        } => *node,
        other => panic!("expected original cycle classification, got {other:?}"),
    };
    let buffer = Vec::with_capacity(8);
    let window = CurrentThreadWindow::open();
    let fields = Capture::read_with_buffer(&error, buffer);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(fields.code, "role-hierarchy-witness");
    let native = fields.child("original");
    assert_eq!(native.code, "role-hierarchy-malformed-list");
    assert!(
        matches!(native.field("node"), ErrorValue::Unsigned(value) if value == u128::from(node))
    );
    assert_eq!(native.child("kind").code, "rdf-list-cycle");
    assert!(
        matches!(fields.field("presentation"), ErrorValue::Record(value)
        if std::ptr::addr_eq(value, presentation as &dyn purrdf_hash::error::StructuredError))
    );

    let signed = RoleHierarchyError::OrderConflict {
        lower: u32::MAX,
        lower_inverse: true,
        upper: u32::MAX - 1,
        upper_inverse: false,
    };
    let fields = Capture::read(&signed);
    assert_eq!(fields.code, "role-hierarchy-order-conflict");
    assert!(
        matches!(fields.field("lower"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX))
    );
    assert!(
        matches!(fields.field("upper"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX - 1))
    );
    assert!(matches!(
        fields.field("lower_inverse"),
        ErrorValue::Boolean(true)
    ));
    assert!(matches!(
        fields.field("upper_inverse"),
        ErrorValue::Boolean(false)
    ));

    for (kind, code) in [
        (ListErrorKind::Cycle, "rdf-list-cycle"),
        (ListErrorKind::MissingFirst, "rdf-list-missing-first"),
        (ListErrorKind::MultipleFirst, "rdf-list-multiple-first"),
        (ListErrorKind::MissingRest, "rdf-list-missing-rest"),
        (ListErrorKind::MultipleRest, "rdf-list-multiple-rest"),
        (ListErrorKind::NonEmptyNil, "rdf-list-nonempty-nil"),
    ] {
        assert_eq!(Capture::read(&kind).code, code);
    }
}

fn entailment_inspection_keeps_native_reads_and_the_actual_inconsistent_run() {
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::{CallerRefusal, Capture};

    let read = EntailError::SourceRead(CallerRefusal { limit: u128::MAX });
    let fields = Capture::read(&read);
    assert_eq!(fields.code, "entailment-source-read");
    assert!(matches!(
        fields.child("cause").field("limit"),
        ErrorValue::Unsigned(u128::MAX)
    ));

    let mut ontology = Ontology::default();
    let subject = ontology.named("original-individual");
    let nothing = ontology.standard(owl::NOTHING);
    ontology.relation(subject, rdf::TYPE, nothing);
    let dataset = ontology.finish();
    let error =
        purrdf_entail::materialize(&dataset, purrdf_entail::Materialization::OwlRl).unwrap_err();
    drop(dataset);
    let fields = Capture::read(&error);
    assert_eq!(fields.code, "entailment-inconsistent");
    let run = fields.child("run");
    assert_eq!(run.code, "entailment-inconsistent-run");
    let witness = run.child("witness");
    assert!(matches!(
        witness.field("rule"),
        ErrorValue::Code("cls-nothing2")
    ));
    assert!(matches!(witness.field("graph"), ErrorValue::Absent));
    let premises = witness.records("premises");
    assert_eq!(premises.len(), 1);
    assert_eq!(premises[0].child("subject").code, "rdf-term-iri");
    let original = match &error {
        EntailError::Inconsistent(original) => original,
        other => panic!("original inconsistent run lost: {other:?}"),
    };
    let buffer = Vec::with_capacity(1024);
    let window = CurrentThreadWindow::open();
    let report = Capture::read_with_buffer(original.report(), buffer);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(report.code, "entailment-report");
    assert!(matches!(report.field("regime"), ErrorValue::Code("OWL-RL")));
    assert_eq!(
        report.records("rules_fired").len(),
        original.report().rules_fired().len()
    );
    assert_eq!(
        report.records("boundaries").len(),
        original.report().boundaries().len()
    );
    assert!(
        matches!(report.child("contract_hash").field("digest"), ErrorValue::Bytes(bytes)
        if bytes == original.report().contract_hash().digest())
    );
    assert_eq!(report.child("budget").code, "datalog-budget-report");
    assert_eq!(
        report.child("inconsistency").code,
        "entailment-inconsistency-witness"
    );
    assert!(matches!(report.field("termination"), ErrorValue::Absent));
    assert!(matches!(report.field("mechanism"), ErrorValue::Absent));

    let mut ontology = Ontology::default();
    let subject = ontology.named("original-individual");
    let thing = ontology.standard(owl::THING);
    ontology.relation(subject, rdf::TYPE, thing);
    let dataset = ontology.finish();
    assert!(purrdf_entail::materialize(&dataset, purrdf_entail::Materialization::OwlRl).is_ok());
}

fn import_conflict_inspection_keeps_original_versions_and_default_graph() {
    use purrdf_core::imports::{VersionConflict, VersionConflictKind};
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::Capture;

    let error: EntailError = EntailError::IncompatibleImports(vec![VersionConflict {
        first: None,
        second: Some("https://example.org/imported".into()),
        kind: VersionConflictKind::SameSeries {
            series: "https://example.org/series".into(),
            first_version: "https://example.org/old".into(),
            second_version: "https://example.org/new".into(),
        },
    }]);
    let fields = Capture::read(&error);
    let conflicts = fields.records("conflicts");
    assert_eq!(conflicts.len(), 1);
    assert!(matches!(conflicts[0].field("first"), ErrorValue::Absent));
    assert!(matches!(
        conflicts[0].field("second"),
        ErrorValue::Iri("https://example.org/imported")
    ));
    let kind = conflicts[0].child("kind");
    assert_eq!(kind.code, "imports-same-series");
    assert!(matches!(
        kind.field("series"),
        ErrorValue::Iri("https://example.org/series")
    ));
    assert!(matches!(
        kind.field("first_version"),
        ErrorValue::Iri("https://example.org/old")
    ));
    assert!(matches!(
        kind.field("second_version"),
        ErrorValue::Iri("https://example.org/new")
    ));
    let kind = VersionConflictKind::IncompatibleWith {
        iri: "https://example.org/incompatible".into(),
    };
    let fields = Capture::read(&kind);
    assert_eq!(fields.code, "imports-incompatible-with");
    assert!(matches!(
        fields.field("iri"),
        ErrorValue::Iri("https://example.org/incompatible")
    ));

    let unresolved: EntailError =
        EntailError::UnresolvedImport("https://example.org/missing".into());
    assert!(matches!(
        Capture::read(&unresolved).field("iri"),
        ErrorValue::Iri("https://example.org/missing")
    ));
    let unused: EntailError = EntailError::UnreachedImport {
        iris: vec!["https://example.org/unused".into()],
        unanchored: vec!["https://example.org/unused".into()],
    };
    let fields = Capture::read(&unused);
    let iris: Vec<_> = fields
        .events
        .iter()
        .filter_map(|event| match event {
            purrdf_testkit::error_capture::Event::Value(None, ErrorValue::Iri(iri)) => Some(*iri),
            _ => None,
        })
        .collect();
    assert_eq!(
        iris,
        ["https://example.org/unused", "https://example.org/unused"]
    );
}

fn actual_collection_refusals_keep_original_terms_and_the_ordered_prefix() {
    use purrdf_core::TermFactory;
    use purrdf_core::collections::ListErrorKind;
    use purrdf_entail::{CollectionPhase, Materialization};
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::Capture;

    for phase in [CollectionPhase::OwlRl, CollectionPhase::OwlDirect] {
        let mut ontology = Ontology::default();
        let owner = ontology.named("original-class");
        let head_value = TermValue::Blank {
            label: "original-cell".into(),
            scope: BlankScope(91),
        };
        let member_value = TermValue::Literal {
            lexical_form: "原文".into(),
            datatype: rdf::LANG_STRING.into(),
            language: Some("zh".into()),
            direction: None,
        };
        let head = ontology.builder.intern_value(&head_value).unwrap();
        let member = ontology.builder.intern_value(&member_value).unwrap();
        ontology.relation(owner, owl::INTERSECTION_OF, head);
        ontology.relation(head, rdf::FIRST, member);
        let individual = ontology.named("source-individual");
        ontology.relation(individual, rdf::TYPE, owner);
        let dataset = ontology.finish();
        let error = match phase {
            CollectionPhase::OwlRl => {
                purrdf_entail::materialize(&dataset, Materialization::OwlRl).unwrap_err()
            }
            CollectionPhase::OwlDirect => Reasoner::new(&dataset).unwrap_err(),
        };
        drop(dataset);
        let original = match &error {
            EntailError::Collection(original) => original,
            other => panic!("native collection cause lost: {other:?}"),
        };
        assert_eq!(original.phase, phase);
        assert_eq!(original.head, head_value);
        assert_eq!(original.original.node, head_value);
        assert_eq!(original.original.kind, ListErrorKind::MissingRest);
        assert_eq!(original.original.members, [member_value]);
        let buffer = Vec::with_capacity(4);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let fields = Capture::read_with_buffer(&error, buffer);
        assert_eq!(window.close().allocations, 0);
        let fields = fields.child("cause");
        assert_eq!(fields.child("original").code, "rdf-list-error");
        assert_eq!(fields.child("original").records("members").len(), 1);
        assert!(matches!(
            fields.child("head").field("scope"),
            ErrorValue::Unsigned(91)
        ));
    }
}

fn empty_collection_admission_checks_nil_structure() {
    use purrdf_entail::{CollectionPhase, Materialization};
    for phase in [CollectionPhase::OwlRl, CollectionPhase::OwlDirect] {
        for malformed in [false, true] {
            let mut ontology = Ontology::default();
            let owner = ontology.named("original-class");
            let nil = ontology.standard(rdf::NIL);
            ontology.relation(owner, owl::INTERSECTION_OF, nil);
            let individual = ontology.named("source-individual");
            ontology.relation(individual, rdf::TYPE, owner);
            if malformed {
                ontology.relation(nil, rdf::FIRST, owner);
            }
            let dataset = ontology.finish();
            let result = match phase {
                CollectionPhase::OwlRl => {
                    purrdf_entail::materialize(&dataset, Materialization::OwlRl).map(|_| ())
                }
                CollectionPhase::OwlDirect => Reasoner::new(&dataset).map(|_| ()),
            };
            if malformed {
                assert!(
                    matches!(result, Err(EntailError::Collection(ref error))
                if error.original.kind == purrdf_core::collections::ListErrorKind::NonEmptyNil),
                    "{phase:?} must preserve the native nil fault: {result:?}"
                );
            } else {
                assert!(result.is_ok());
            }
        }
    }
}

fn dataset_diagnostics_retain_original_metadata_and_native_cause_without_rendering() {
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_core::{RdfDiagnostic, RdfLocation};
    use purrdf_entail::DatasetFailurePhase;
    use purrdf_hash::error::{ErrorValue, NativeError};
    use purrdf_lex::allocation::{Memory, Resident, SharedOwned};
    use purrdf_testkit::error_capture::{CallerRefusal, Capture};

    for phase in [
        DatasetFailurePhase::Build,
        DatasetFailurePhase::AugmentedFreeze,
    ] {
        // This control uses unrestricted resident allocation; its grant is unit.
        let cause = SharedOwned::try_from_admitted(
            Box::new(CallerRefusal { limit: u128::MAX }) as Box<dyn NativeError>,
            (),
        )
        .unwrap();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let diagnostic = RdfDiagnostic::error("original-code", "original-message")
            .with_detail("original-detail")
            .with_location(RdfLocation {
                path: Some("original-input.ttl".into()),
                line: Some(u64::MAX),
                ..RdfLocation::default()
            })
            .with_native_cause(cause, &mut memory)
            .unwrap();
        let original_code = diagnostic.code.as_ptr();
        let expected = match phase {
            DatasetFailurePhase::Build => format!("entailment build error: {diagnostic}"),
            DatasetFailurePhase::AugmentedFreeze => {
                format!("entailment build error: freeze augmented dataset: {diagnostic}")
            }
        };
        let buffer = Vec::with_capacity(4);
        let window = CurrentThreadWindow::open();
        let error: EntailError = match phase {
            DatasetFailurePhase::Build => diagnostic.into(),
            DatasetFailurePhase::AugmentedFreeze => EntailError::Dataset {
                phase,
                cause: diagnostic,
            },
        };
        let error = error.with_read_error::<CallerRefusal>();
        let fields = Capture::read_with_buffer(&error, buffer);
        assert_eq!(window.close().allocations, 0);
        assert_eq!(fields.code, "entailment-dataset");
        let original = fields.child("cause");
        assert!(matches!(original.field("code"), ErrorValue::Code(value)
            if value == "original-code" && value.as_ptr() == original_code));
        assert!(matches!(
            original.field("message"),
            ErrorValue::Text("original-message")
        ));
        assert!(matches!(
            original.field("detail"),
            ErrorValue::Text("original-detail")
        ));
        assert!(
            matches!(original.child("location").field("line"), ErrorValue::Unsigned(value)
            if value == u128::from(u64::MAX))
        );
        assert!(matches!(
            original
                .child("attachments")
                .child("native_cause")
                .field("limit"),
            ErrorValue::Unsigned(u128::MAX)
        ));
        let native = std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<RdfDiagnostic>()
            .unwrap();
        let native: &(dyn std::error::Error + 'static) = native.native_cause().unwrap();
        assert_eq!(
            native.downcast_ref::<CallerRefusal>().unwrap().limit,
            u128::MAX
        );
        assert_eq!(error.to_string(), expected);
    }
}

#[derive(Default)]
struct Ontology {
    builder: RdfDatasetBuilder,
    next: usize,
}

fn authored_cardinality_refusals_retain_the_original_operands_and_integer_cause() {
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_core::RdfLiteral;
    use purrdf_entail::OwlSyntaxError;
    use purrdf_testkit::error_capture::Capture;
    use purrdf_xsd::datatype;

    for lexical in ["  not-an-integer\t", "4294967295", "2"] {
        let mut ontology = Ontology::default();
        let class = ontology.named("CardinalityClass");
        let property = ontology.named("cardinality-property");
        let restriction = ontology.blank();
        let bound = ontology
            .builder
            .intern_literal(RdfLiteral::typed(
                lexical,
                datatype::XSD_NON_NEGATIVE_INTEGER,
            ))
            .unwrap();
        let object_property = ontology.standard(owl::OBJECT_PROPERTY);
        let restriction_class = ontology.standard(owl::RESTRICTION);
        ontology.relation(property, rdf::TYPE, object_property);
        ontology.relation(restriction, rdf::TYPE, restriction_class);
        ontology.relation(restriction, owl::ON_PROPERTY, property);
        ontology.relation(restriction, owl::MAX_CARDINALITY, bound);
        ontology.relation(class, rdfs::SUB_CLASS_OF, restriction);
        let dataset = ontology.finish();
        let original = TermValue::typed_literal(lexical, datatype::XSD_NON_NEGATIVE_INTEGER);
        let result = Reasoner::new(&dataset);
        if lexical == "2" {
            assert!(result.is_ok(), "adjacent supported cardinality: {result:?}");
            continue;
        }
        let error = match result {
            Err(EntailError::OwlSyntax(error)) => error,
            other => panic!("actual restriction must retain its native refusal: {other:?}"),
        };
        drop(dataset);
        match error.as_ref() {
            OwlSyntaxError::CardinalityLexical { literal, cause } => {
                assert_eq!(literal, &original);
                assert_eq!(cause.kind(), &std::num::IntErrorKind::InvalidDigit);
                assert!(
                    std::error::Error::source(error.as_ref())
                        .unwrap()
                        .downcast_ref::<std::num::ParseIntError>()
                        .is_some()
                );
            }
            OwlSyntaxError::CardinalityRange {
                literal,
                value,
                maximum,
            } => {
                assert_eq!(literal, &original);
                assert_eq!(*value, u32::MAX);
                assert_eq!(*maximum, u32::MAX - 1);
            }
            other => panic!("wrong native cardinality refusal: {other:?}"),
        }
        let buffer = Vec::with_capacity(8);
        let window = CurrentThreadWindow::open();
        let fields = Capture::read_with_buffer(error.as_ref(), buffer);
        assert_eq!(window.close().allocations, 0);
        assert_eq!(fields.child("literal").code, "rdf-term-literal");
    }
}

fn actual_rif_refusals_keep_original_scope_and_survive_the_document() {
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_entail::{RifComponent, RifIriPhase, RifXmlError, parse_rif_xml};
    use purrdf_testkit::error_capture::Capture;

    let document = format!(
        "<Document xmlns=\"{}\"><directive><Import><location>facts.ttl</location></Import></directive></Document>",
        purrdf_iri::vocab::rif::NS
    );
    let error = match parse_rif_xml(&document, None) {
        Err(EntailError::RifXml(error)) => error,
        other => panic!("relative reference without a base must refuse: {other:?}"),
    };
    assert!(parse_rif_xml(&document, Some("https://example.org/rules/")).is_ok());
    drop(document);
    match error.as_ref() {
        RifXmlError::Iri {
            phase,
            reference,
            base,
            cause,
        } => {
            assert_eq!(*phase, RifIriPhase::Reference);
            assert_eq!(reference, "facts.ttl");
            assert!(base.is_none());
            assert!(matches!(cause, purrdf_iri::IriError::NoBase { .. }));
            assert!(
                std::error::Error::source(error.as_ref())
                    .unwrap()
                    .downcast_ref::<purrdf_iri::IriError>()
                    .is_some()
            );
        }
        other => panic!("lost the original IRI scope: {other:?}"),
    }
    let buffer = Vec::with_capacity(8);
    let window = CurrentThreadWindow::open();
    let fields = Capture::read_with_buffer(error.as_ref(), buffer);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(fields.code, "rif-xml-iri");
    let missing = format!(
        "<Document xmlns=\"{}\"><directive><Import/></directive></Document>",
        purrdf_iri::vocab::rif::NS
    );
    assert!(
        matches!(parse_rif_xml(&missing, None), Err(EntailError::RifXml(error))
        if matches!(error.as_ref(), RifXmlError::Missing(RifComponent::ImportLocation)))
    );
}

impl Ontology {
    fn named(&mut self, name: &str) -> TermId {
        self.builder
            .intern_iri(&format!("http://example.org/{name}"))
            .unwrap()
    }

    fn blank(&mut self) -> TermId {
        let label = format!("syntax{}", self.next);
        self.next += 1;
        self.builder
            .intern_blank(&label, BlankScope::DEFAULT)
            .unwrap()
    }

    fn standard(&mut self, iri: &str) -> TermId {
        self.builder.intern_iri(iri).unwrap()
    }

    fn triple(&mut self, s: TermId, p: TermId, o: TermId) {
        self.builder.push_quad(s, p, o, None).unwrap();
    }

    fn inverse(&mut self, role: TermId) -> TermId {
        let inverse = self.blank();
        let predicate = self.standard(owl::INVERSE_OF);
        self.triple(inverse, predicate, role);
        inverse
    }

    fn chain(&mut self, head: TermId, body: &[TermId]) {
        let first = self.standard(rdf::FIRST);
        let rest = self.standard(rdf::REST);
        let mut tail = self.standard(rdf::NIL);
        for &member in body.iter().rev() {
            let cell = self.blank();
            self.triple(cell, first, member);
            self.triple(cell, rest, tail);
            tail = cell;
        }
        let predicate = self.standard(owl::PROPERTY_CHAIN_AXIOM);
        self.triple(head, predicate, tail);
    }

    fn edge(&mut self, s: &str, role: TermId, o: &str) {
        let subject = self.named(s);
        let object = self.named(o);
        self.triple(subject, role, object);
    }

    fn relation(&mut self, sub: TermId, predicate: &str, sup: TermId) {
        let predicate = self.standard(predicate);
        self.triple(sub, predicate, sup);
    }

    fn restriction(&mut self, role: TermId, predicate: &str, filler: TermId) -> TermId {
        let restriction = self.blank();
        let on = self.standard(owl::ON_PROPERTY);
        self.triple(restriction, on, role);
        let predicate = self.standard(predicate);
        self.triple(restriction, predicate, filler);
        restriction
    }

    fn finish(self) -> Arc<RdfDataset> {
        self.builder.freeze().expect("valid RDF fixture")
    }
}

fn assertion(subject: &str, property: &str, object: &str) -> DlAxiom {
    DlAxiom::ObjectPropertyAssertion {
        subject: iri(subject),
        property: iri(property),
        object: iri(object),
    }
}

fn entailed(reasoner: &mut Reasoner, axiom: &DlAxiom, expected: Verdict) {
    let answer = reasoner.entails(axiom).expect("complete service");
    assert_eq!(*answer.answer(), expected);
    assert_eq!(
        answer.certificate().completeness(),
        DlCompleteness::Decided,
        "{:?}",
        answer.certificate().boundaries()
    );
    assert_eq!(answer.certificate().boundaries(), []);
    assert!(answer.certificate().storage_refusal().is_none());
}

/// These nine generic shapes are the verbatim ordered chain shapes of the nine
/// external donor axioms, with consumer IRIs deliberately replaced by caller data.
fn all_nine_consumer_shapes_are_exact_and_contradictions_are_detected() {
    let chains = [
        (
            "hasSensoryQuantity",
            "hasSensoryObservation",
            "sensoryResult",
        ),
        ("eventLocation", "eventLocation", "containedInLocation"),
        ("wasAssociatedWith", "generatedObservation", "vantage"),
        ("hasReferenceFrame", "isResultOf", "hasReferenceFrame"),
        ("memberOf", "memberOf", "subOrganizationOf"),
        (
            "hasCoordinates",
            "hasCoordinateObservation",
            "coordinateResult",
        ),
        ("hasGeometry", "hasCoordinateObservation", "geometryResult"),
        ("locatedAt", "locatedAt", "containedInLocation"),
        ("periodStart", "hasTemporalMeasurement", "measuredDate"),
    ];
    let mut ontology = Ontology::default();
    for (head, left, right) in chains {
        let head = ontology.named(head);
        let left = ontology.named(left);
        let right = ontology.named(right);
        ontology.chain(head, &[left, right]);
        ontology.edge("a", left, "b");
        ontology.edge("b", right, "c");
        // A second recursive hop, where the authored endpoint permits it.
        ontology.edge("c", right, "d");
    }
    let dataset = ontology.finish();
    let mut reasoner = Reasoner::new(&dataset).expect("the combined regular RBox");
    let consistency = reasoner.consistency();
    assert_eq!(*consistency.answer(), Verdict::True);
    assert_eq!(
        consistency.certificate().completeness(),
        DlCompleteness::Decided
    );
    for (head, _, _) in chains {
        entailed(&mut reasoner, &assertion("a", head, "c"), Verdict::True);
        entailed(&mut reasoner, &assertion("c", head, "a"), Verdict::False);
    }
    for head in ["eventLocation", "memberOf", "locatedAt"] {
        entailed(&mut reasoner, &assertion("a", head, "d"), Verdict::True);
    }
    for (head, left, right) in chains {
        let mut contradicted = Ontology::default();
        let head = contradicted.named(head);
        let left = contradicted.named(left);
        let right = contradicted.named(right);
        contradicted.chain(head, &[left, right]);
        contradicted.edge("a", left, "b");
        contradicted.edge("b", right, "c");
        let negative = contradicted.blank();
        let kind = contradicted.standard(owl::NEGATIVE_PROPERTY_ASSERTION);
        contradicted.relation(negative, rdf::TYPE, kind);
        let a = contradicted.named("a");
        let c = contradicted.named("c");
        contradicted.relation(negative, owl::SOURCE_INDIVIDUAL, a);
        contradicted.relation(negative, owl::ASSERTION_PROPERTY, head);
        contradicted.relation(negative, owl::TARGET_INDIVIDUAL, c);
        let dataset = contradicted.finish();
        let answer = Reasoner::new(&dataset)
            .expect("regular contradicted source")
            .consistency();
        assert_eq!(*answer.answer(), Verdict::False);
        assert_eq!(answer.certificate().completeness(), DlCompleteness::Decided);
    }
}

fn inverse_heads_nested_dependencies_and_simple_equivalence_cycles() {
    let mut o = Ontology::default();
    let p = o.named("p");
    let q = o.named("q");
    let r = o.named("r");
    let alias = o.named("alias");
    let s = o.named("s");
    let inv_r = o.inverse(r);
    o.chain(inv_r, &[p, q]);
    o.relation(r, owl::EQUIVALENT_PROPERTY, alias);
    o.chain(s, &[alias, p]);
    o.edge("a", p, "b");
    o.edge("b", q, "c");
    o.edge("a", p, "d");
    let dataset = o.finish();
    let mut reasoner = Reasoner::new(&dataset).expect("regular inverse hierarchy");
    entailed(&mut reasoner, &assertion("c", "r", "a"), Verdict::True);
    entailed(&mut reasoner, &assertion("c", "alias", "a"), Verdict::True);
    entailed(&mut reasoner, &assertion("c", "s", "d"), Verdict::True);
    entailed(&mut reasoner, &assertion("a", "s", "c"), Verdict::False);
}

fn universal_obligations_clash_after_recursive_paths_and_blocking() {
    for cyclic in [false, true] {
        let mut o = Ontology::default();
        let r = o.named("r");
        let p = o.named("p");
        let c = o.named("C");
        let a = o.named("a");
        let class_a = o.named("A");
        let bottom = o.standard(owl::NOTHING);
        let top = o.standard(owl::THING);
        o.chain(r, &[r, p]);
        if cyclic {
            let some = o.restriction(p, owl::SOME_VALUES_FROM, class_a);
            o.relation(class_a, rdfs::SUB_CLASS_OF, some);
            o.relation(a, rdf::TYPE, class_a);
            // Every r-reachable node must have no p successor. The recursive
            // existential's successor carries A and therefore requires one.
            let no_p = o.restriction(p, owl::ALL_VALUES_FROM, bottom);
            let all = o.restriction(r, owl::ALL_VALUES_FROM, no_p);
            o.relation(a, rdf::TYPE, all);
            let some_r = o.restriction(r, owl::SOME_VALUES_FROM, class_a);
            o.relation(a, rdf::TYPE, some_r);
        } else {
            o.edge("a", r, "b");
            o.edge("b", p, "c");
            o.edge("c", p, "d");
            let all = o.restriction(r, owl::ALL_VALUES_FROM, c);
            o.relation(a, rdf::TYPE, all);
            let not_c = o.blank();
            o.relation(not_c, owl::COMPLEMENT_OF, c);
            let d = o.named("d");
            o.relation(d, rdf::TYPE, not_c);
        }
        // Keep a genuinely populated object domain in both cases.
        o.relation(a, rdf::TYPE, top);
        let dataset = o.finish();
        let answer = Reasoner::new(&dataset)
            .expect("regular input")
            .consistency();
        assert_eq!(*answer.answer(), Verdict::False);
        assert_eq!(answer.certificate().completeness(), DlCompleteness::Decided);
    }
}

fn cyclic_existentials_with_role_obligations_have_a_finite_blocking_closure() {
    let mut o = Ontology::default();
    let r = o.named("r");
    let p = o.named("p");
    let a = o.named("a");
    let class = o.named("A");
    o.chain(r, &[r, p]);
    let some = o.restriction(p, owl::SOME_VALUES_FROM, class);
    o.relation(class, rdfs::SUB_CLASS_OF, some);
    let all = o.restriction(r, owl::ALL_VALUES_FROM, class);
    o.relation(class, rdfs::SUB_CLASS_OF, all);
    o.relation(a, rdf::TYPE, class);
    o.edge("a", r, "b");
    let dataset = o.finish();
    let reasoner = Reasoner::new(&dataset).expect("regular input");
    let answer = reasoner.consistency();
    assert_eq!(*answer.answer(), Verdict::True);
    assert_eq!(answer.certificate().completeness(), DlCompleteness::Decided);
}

fn printed_order_counterexample_and_inverse_endpoint_typo_do_not_pass() {
    for inverse_endpoint in [false, true] {
        let mut o = Ontology::default();
        let a = o.named("aRole");
        let x = o.named("x");
        if inverse_endpoint {
            let inverse = o.inverse(a);
            o.chain(a, &[inverse, x]);
        } else {
            let b = o.named("bRole");
            let c = o.named("cRole");
            let d = o.named("dRole");
            let y = o.named("y");
            o.relation(b, rdfs::SUB_PROPERTY_OF, c);
            o.relation(d, rdfs::SUB_PROPERTY_OF, a);
            o.chain(b, &[a, x]);
            o.chain(d, &[y, c]);
        }
        let dataset = o.finish();
        assert!(matches!(
            Reasoner::new(&dataset),
            Err(EntailError::RoleHierarchy(error)) if matches!(error.classification(),
                RoleHierarchyError::OrderConflict { .. }
                    | RoleHierarchyError::RecursiveDependency { .. })
        ));
    }
}

fn literal_recursive_endpoint_is_not_replaced_by_an_equivalent_name() {
    let mut o = Ontology::default();
    let r = o.named("r");
    let alias = o.named("alias");
    let p = o.named("p");
    o.relation(r, owl::EQUIVALENT_PROPERTY, alias);
    o.chain(r, &[alias, p]);
    let dataset = o.finish();
    assert!(matches!(
        Reasoner::new(&dataset),
        Err(EntailError::RoleHierarchy(_))
    ));
}

fn recorded_chain_consequence_replays_against_the_original_rbox() {
    let mut o = Ontology::default();
    let p = o.named("p");
    let q = o.named("q");
    let r = o.named("r");
    o.chain(r, &[p, q]);
    o.edge("a", p, "b");
    o.edge("b", q, "c");
    let dataset = o.finish();
    let axiom = assertion("a", "r", "c");
    let question = Question::AxiomEntailment {
        axiom: Box::new(axiom.clone()),
    };
    let mut reasoner = Reasoner::with_proofs(&dataset).expect("regular source");
    let answer = reasoner.entails(&axiom).expect("recorded answer");
    assert_eq!(*answer.answer(), Verdict::True);
    let mut consumer = Reasoner::with_proofs(&dataset).expect("independent reverse mapping");
    consumer.prepare(&question).expect("query preparation");
    let context = consumer.proof_context().expect("proof context");
    let proof = answer.proof().expect("actual recorded chain refutation");
    proof
        .verify(&dataset, &question, Some(answer.certificate()), &context)
        .expect("independent original-inclusion replay");

    let mut altered = Ontology::default();
    let p = altered.named("p");
    let q = altered.named("q");
    let r = altered.named("r");
    altered.chain(r, &[q, p]);
    altered.edge("a", p, "b");
    altered.edge("b", q, "c");
    let altered = altered.finish();
    let mut checker = Reasoner::with_proofs(&altered).expect("regular changed RBox");
    checker.prepare(&question).expect("same question");
    let changed_context = checker.proof_context().expect("changed source context");
    assert!(
        proof
            .verify(
                &altered,
                &question,
                Some(answer.certificate()),
                &changed_context
            )
            .is_err(),
        "a proof of p·q must not authenticate a q·p source"
    );
}

fn chain_derived_types_reach_all_query_services_and_materialization() {
    let mut o = Ontology::default();
    let p = o.named("p");
    let q = o.named("q");
    let r = o.named("r");
    let class = o.named("C");
    o.chain(r, &[p, q]);
    o.relation(r, rdfs::RANGE, class);
    o.edge("a", p, "b");
    o.edge("b", q, "c");
    // E forces a p·q path while forbidding every r successor. Only the chain
    // inclusion makes E empty; the source ontology itself remains consistent.
    let empty = o.named("E");
    let thing = o.standard(owl::THING);
    let nothing = o.standard(owl::NOTHING);
    let q_path = o.restriction(q, owl::SOME_VALUES_FROM, thing);
    let p_path = o.restriction(p, owl::SOME_VALUES_FROM, q_path);
    let no_r = o.restriction(r, owl::ALL_VALUES_FROM, nothing);
    o.relation(empty, rdfs::SUB_CLASS_OF, p_path);
    o.relation(empty, rdfs::SUB_CLASS_OF, no_r);
    let dataset = o.finish();
    let mut reasoner = Reasoner::new(&dataset).expect("regular source");
    entailed(
        &mut reasoner,
        &DlAxiom::ClassAssertion {
            individual: iri("c"),
            class: iri("C"),
        },
        Verdict::True,
    );
    let instances = reasoner.instances(&iri("C")).expect("instances");
    assert_eq!(
        instances.certificate().completeness(),
        DlCompleteness::Decided
    );
    assert_eq!(instances.answer().as_slice(), &[iri("c")]);
    let realized = reasoner.realize().expect("realization");
    assert_eq!(
        realized.certificate().completeness(),
        DlCompleteness::Decided
    );
    assert_eq!(
        realized
            .answer()
            .types()
            .iter()
            .filter(|(_, class)| *class == iri("C"))
            .cloned()
            .collect::<Vec<_>>(),
        vec![(iri("c"), iri("C"))]
    );
    assert!(
        realized
            .direct_types()
            .expect("fully decided realization")
            .contains(&(iri("c"), iri("C")))
    );
    let classified = reasoner.classify().expect("classification");
    assert_eq!(
        classified.certificate().completeness(),
        DlCompleteness::Decided
    );
    assert!(classified.answer().unsatisfiable().contains(&iri("E")));
    assert!(
        classified
            .answer()
            .subsumptions()
            .contains(&(iri("E"), iri("C")))
    );
    let empty_answer = reasoner
        .class_satisfiability(&iri("E"))
        .expect("empty class");
    assert_eq!(*empty_answer.answer(), Verdict::False);
    let satisfiable = reasoner
        .class_satisfiability(&iri("C"))
        .expect("satisfiability");
    assert_eq!(*satisfiable.answer(), Verdict::True);
    let pattern = purrdf_entail::QTriple {
        s: purrdf_entail::QNode::Term(iri("a")),
        p: purrdf_entail::QNode::Term(iri("r")),
        o: purrdf_entail::QNode::Term(iri("c")),
    };
    let (augmented, report) = purrdf_entail::materialize_dl_reported(&dataset, &[pattern])
        .expect("actual production query augmentation");
    assert_eq!(report.completeness(), purrdf_entail::Completeness::Exact);
    assert!(augmented.quads().any(|quad| {
        augmented.term_value(quad.s) == iri("a")
            && augmented.term_value(quad.p) == iri("r")
            && augmented.term_value(quad.o) == iri("c")
    }));
}

fn fixed_top_is_universal_and_bottom_chain_clashes() {
    for bottom_head in [false, true] {
        let mut o = Ontology::default();
        let p = o.named("p");
        let q = o.named("q");
        let fixed = o.standard(if bottom_head {
            owl::BOTTOM_OBJECT_PROPERTY
        } else {
            owl::TOP_OBJECT_PROPERTY
        });
        o.chain(fixed, &[p, q]);
        o.edge("a", p, "b");
        o.edge("b", q, "c");
        let dataset = o.finish();
        let mut reasoner = Reasoner::new(&dataset).expect("fixed role syntax");
        let answer = reasoner.consistency();
        assert_eq!(
            *answer.answer(),
            if bottom_head {
                Verdict::False
            } else {
                Verdict::True
            }
        );
        assert_eq!(answer.certificate().completeness(), DlCompleteness::Decided);
        if !bottom_head {
            let axiom = DlAxiom::ObjectPropertyAssertion {
                subject: iri("isolated"),
                property: TermValue::iri(owl::TOP_OBJECT_PROPERTY),
                object: iri("other"),
            };
            entailed(&mut reasoner, &axiom, Verdict::True);
        }
    }
}

fn fixed_top_aliases_without_a_chain_reach_query_only_individuals() {
    for inverse in [false, true] {
        let mut o = Ontology::default();
        let top = o.standard(owl::TOP_OBJECT_PROPERTY);
        let alias = o.named("universalAlias");
        o.relation(
            top,
            if inverse {
                owl::INVERSE_OF
            } else {
                rdfs::SUB_PROPERTY_OF
            },
            alias,
        );
        let dataset = o.finish();
        let mut reasoner = Reasoner::with_proofs(&dataset).expect("fixed top alias");
        let axiom = assertion("newSubject", "universalAlias", "newObject");
        let answer = reasoner.entails(&axiom).expect("universal relation");
        assert_eq!(*answer.answer(), Verdict::True);
        let question = Question::AxiomEntailment {
            axiom: Box::new(axiom),
        };
        let mut consumer = Reasoner::with_proofs(&dataset).expect("source context");
        consumer.prepare(&question).expect("query roots");
        answer
            .proof()
            .expect("recorded universal alias proof")
            .verify(
                &dataset,
                &question,
                Some(answer.certificate()),
                &consumer.proof_context().expect("checking context"),
            )
            .expect("independent fixed-role alias replay");

        let mut invalid = Ontology::default();
        let top = invalid.standard(owl::TOP_OBJECT_PROPERTY);
        let alias = invalid.named("universalAlias");
        invalid.relation(
            top,
            if inverse {
                owl::INVERSE_OF
            } else {
                rdfs::SUB_PROPERTY_OF
            },
            alias,
        );
        let functional = invalid.standard(owl::FUNCTIONAL_PROPERTY);
        invalid.relation(alias, rdf::TYPE, functional);
        assert!(matches!(
            Reasoner::new(&invalid.finish()),
            Err(EntailError::RoleHierarchy(error)) if matches!(error.classification(),
                RoleHierarchyError::NonSimpleProperty { .. })
        ));
    }
}

fn non_simple_characteristics_and_malformed_chain_lists_hard_refuse() {
    for characteristic in [
        owl::FUNCTIONAL_PROPERTY,
        owl::INVERSE_FUNCTIONAL_PROPERTY,
        owl::IRREFLEXIVE_PROPERTY,
        owl::ASYMMETRIC_PROPERTY,
    ] {
        let mut o = Ontology::default();
        let p = o.named("p");
        let q = o.named("q");
        let r = o.named("r");
        o.chain(r, &[p, q]);
        let kind = o.standard(characteristic);
        o.relation(r, rdf::TYPE, kind);
        let dataset = o.finish();
        assert!(matches!(
            Reasoner::new(&dataset),
            Err(EntailError::RoleHierarchy(error)) if matches!(error.classification(),
                RoleHierarchyError::NonSimpleProperty { .. })
        ));
    }
    let mut o = Ontology::default();
    let p = o.named("p");
    let r = o.named("r");
    let cell = o.blank();
    o.relation(r, owl::PROPERTY_CHAIN_AXIOM, cell);
    o.relation(cell, rdf::FIRST, p);
    o.relation(cell, rdf::REST, cell);
    let dataset = o.finish();
    assert!(matches!(
        Reasoner::new(&dataset),
        Err(EntailError::RoleHierarchy(error)) if matches!(error.classification(),
            RoleHierarchyError::MalformedList { .. })
    ));
}

fn role_language_work_exhaustion_is_unknown_never_a_consistency_proof() {
    let mut o = Ontology::default();
    let p = o.named("p");
    let r = o.named("r");
    o.chain(r, &[r, p]);
    o.edge("a", r, "b");
    o.edge("b", p, "c");
    let dataset = o.finish();
    let reasoner = Reasoner::with_proofs(&dataset)
        .expect("regular source")
        .with_work_cap(1);
    let answer = reasoner.consistency();
    assert_eq!(*answer.answer(), Verdict::Unknown);
    assert_eq!(
        answer.certificate().completeness(),
        DlCompleteness::BudgetExhausted
    );
}

fn nominal_merges_and_simple_functionality_preserve_role_obligations() {
    for contradict in [false, true] {
        let mut o = Ontology::default();
        let p = o.named("p");
        let q = o.named("q");
        let r = o.named("r");
        let a = o.named("a");
        let b = o.named("b");
        let c = o.named("c");
        let class = o.named("C");
        o.chain(r, &[p, q]);
        let function = o.standard(owl::FUNCTIONAL_PROPERTY);
        o.relation(p, rdf::TYPE, function);
        let nominal = o.blank();
        let cell = o.blank();
        let nil = o.standard(rdf::NIL);
        o.relation(nominal, owl::ONE_OF, cell);
        o.relation(cell, rdf::FIRST, b);
        o.relation(cell, rdf::REST, nil);
        let some = o.restriction(p, owl::SOME_VALUES_FROM, nominal);
        o.relation(a, rdf::TYPE, some);
        o.edge("a", p, "other");
        o.edge("b", q, "c");
        let all = o.restriction(r, owl::ALL_VALUES_FROM, class);
        o.relation(a, rdf::TYPE, all);
        let asserted = if contradict {
            let complement = o.blank();
            o.relation(complement, owl::COMPLEMENT_OF, class);
            complement
        } else {
            class
        };
        o.relation(c, rdf::TYPE, asserted);
        let dataset = o.finish();
        let answer = Reasoner::new(&dataset)
            .expect("regular source with a simple counted role")
            .consistency();
        assert_eq!(
            *answer.answer(),
            if contradict {
                Verdict::False
            } else {
                Verdict::True
            }
        );
        assert_eq!(answer.certificate().completeness(), DlCompleteness::Decided);
    }
}

fn cancelling_an_existing_regular_role_owner_cannot_publish_a_decision() {
    #[derive(Debug)]
    struct Stop(std::sync::atomic::AtomicBool);
    impl purrdf_datalog::StopSignal for Stop {
        fn stopped(&self) -> bool {
            self.0.load(std::sync::atomic::Ordering::Relaxed)
        }
    }
    let mut o = Ontology::default();
    let p = o.named("p");
    let r = o.named("r");
    o.chain(r, &[r, p]);
    o.edge("a", r, "b");
    o.edge("b", p, "c");
    let dataset = o.finish();
    let stop = Arc::new(Stop(std::sync::atomic::AtomicBool::new(false)));
    let reasoner = Reasoner::with_stop(&dataset, stop.clone()).expect("regular source");
    stop.0.store(true, std::sync::atomic::Ordering::Relaxed);
    let answer = reasoner.consistency();
    assert_eq!(*answer.answer(), Verdict::Unknown);
    assert!(answer.certificate().stopped());
    assert!(answer.certificate().storage_refusal().is_none());
    assert!(answer.proof().is_none());
}

fn top_outside_the_rbox_retains_universal_semantics_and_source_refusal_spelling() {
    let mut o = Ontology::default();
    let p = o.named("p");
    let q = o.named("q");
    let r = o.named("r");
    o.chain(r, &[p, q]);
    o.edge("a", p, "b");
    o.edge("b", q, "c");
    let top = o.standard(owl::TOP_OBJECT_PROPERTY);
    o.edge("a", top, "d");
    let dataset = o.finish();
    let mut reasoner = Reasoner::new(&dataset).expect("top only in ABox, outside role order");
    assert_eq!(*reasoner.consistency().answer(), Verdict::True);
    entailed(&mut reasoner, &assertion("a", "r", "c"), Verdict::True);
    entailed(
        &mut reasoner,
        &DlAxiom::ObjectPropertyAssertion {
            subject: iri("d"),
            property: TermValue::Iri(owl::TOP_OBJECT_PROPERTY.to_owned()),
            object: iri("c"),
        },
        Verdict::True,
    );

    let mut invalid = Ontology::default();
    let r = invalid.named("offendingRole");
    invalid.chain(r, &[r]);
    let dataset = invalid.finish();
    let Err(EntailError::RoleHierarchy(error)) = Reasoner::new(&dataset) else {
        panic!("one-member chain must refuse before publication");
    };
    assert!(matches!(
        error.classification(),
        RoleHierarchyError::ShortChain { .. }
    ));
    assert!(
        error
            .to_string()
            .contains("http://example.org/offendingRole")
    );
    assert!(error.presentation().is_some());
}

/// Restriction-role failures retain typed causes and source arguments after input disposal.
fn class_expression_role_refusals_retain_original_source_terms() {
    use purrdf_lex::diagnostic::DiagnosticValue;
    for recording in [false, true] {
        for literal in [false, true] {
            let expected = if literal {
                "literal-role-source"
            } else {
                "http://example.org/annotationRole"
            };
            let failure = {
                let mut o = Ontology::default();
                let role = if literal {
                    o.builder
                        .intern_literal(purrdf_core::RdfLiteral::simple(expected))
                        .unwrap()
                } else {
                    let role = o.named("annotationRole");
                    let kind = o.standard(owl::ANNOTATION_PROPERTY);
                    o.relation(role, rdf::TYPE, kind);
                    role
                };
                let filler = o.named("Filler");
                let restriction = o.restriction(role, owl::SOME_VALUES_FROM, filler);
                let class = o.named("Class");
                o.relation(class, rdfs::SUB_CLASS_OF, restriction);
                let dataset = o.finish();
                match if recording {
                    Reasoner::with_proofs(&dataset)
                } else {
                    Reasoner::new(&dataset)
                } {
                    Err(error) => error,
                    Ok(_) => panic!("invalid restriction role must refuse before publication"),
                }
            };
            let EntailError::RoleHierarchy(error) = failure else {
                panic!("role refusal must retain its native typed classification");
            };
            assert!(matches!(
                error.classification(),
                RoleHierarchyError::InvalidProperty { .. }
            ));
            let presentation = error
                .presentation()
                .expect("original source must be retained");
            assert_eq!(presentation.message_id(), "entail.role-hierarchy.source");
            let source = presentation
                .parameters()
                .iter()
                .find(|parameter| parameter.name() == "first")
                .expect("source argument");
            assert!(
                matches!(source.value(), DiagnosticValue::Text(value) if value.contains(expected))
            );
            assert!(presentation.english().contains(expected));
            let original = std::error::Error::source(&error)
                .and_then(|source| source.downcast_ref::<RoleHierarchyError>())
                .expect("original typed cause");
            assert!(matches!(
                original,
                RoleHierarchyError::InvalidProperty { .. }
            ));
        }
    }
}

fn stopping_during_regular_role_reads_never_publishes_a_completed_answer() {
    #[derive(Debug)]
    struct Stop {
        polls: std::sync::atomic::AtomicU64,
        fire_at: std::sync::atomic::AtomicU64,
    }
    impl purrdf_datalog::StopSignal for Stop {
        fn stopped(&self) -> bool {
            let poll = self
                .polls
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            poll >= self.fire_at.load(std::sync::atomic::Ordering::Relaxed)
        }
    }
    let mut o = Ontology::default();
    let p = o.named("p");
    let r = o.named("r");
    o.chain(r, &[r, p]);
    for (a, b) in [("a", "b"), ("b", "c"), ("c", "d")] {
        o.edge(a, p, b);
    }
    o.edge("a", r, "b");
    let filler = o.named("C");
    let restriction = o.restriction(r, owl::ALL_VALUES_FROM, filler);
    let a = o.named("a");
    o.relation(a, rdf::TYPE, restriction);
    let dataset = o.finish();
    let stop = Arc::new(Stop {
        polls: std::sync::atomic::AtomicU64::new(0),
        fire_at: std::sync::atomic::AtomicU64::new(u64::MAX),
    });
    let reasoner = Reasoner::with_stop(&dataset, stop.clone()).expect("regular source");
    stop.polls.store(0, std::sync::atomic::Ordering::Relaxed);
    let healthy = reasoner.consistency();
    assert_eq!(*healthy.answer(), Verdict::True);
    let polls = stop.polls.load(std::sync::atomic::Ordering::Relaxed);
    assert!(polls > 3, "must exercise reads after entering a round");
    for fire_at in 1..polls {
        stop.polls.store(0, std::sync::atomic::Ordering::Relaxed);
        stop.fire_at
            .store(fire_at, std::sync::atomic::Ordering::Relaxed);
        let answer = reasoner.consistency();
        assert_eq!(
            *answer.answer(),
            Verdict::Unknown,
            "stop at poll {fire_at}/{polls}"
        );
        assert!(answer.certificate().stopped());
        assert!(answer.proof().is_none());
    }
    stop.fire_at
        .store(u64::MAX, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(*reasoner.consistency().answer(), Verdict::True);
}

purrdf_testkit::harness_main!(
    authored_cardinality_refusals_retain_the_original_operands_and_integer_cause,
    actual_rif_refusals_keep_original_scope_and_survive_the_document,
    dataset_diagnostics_retain_original_metadata_and_native_cause_without_rendering,
    actual_collection_refusals_keep_original_terms_and_the_ordered_prefix,
    empty_collection_admission_checks_nil_structure,
    import_conflict_inspection_keeps_original_versions_and_default_graph,
    entailment_inspection_keeps_native_reads_and_the_actual_inconsistent_run,
    role_refusals_keep_native_fields_after_the_source_drops,
    class_expression_role_refusals_retain_original_source_terms,
    top_outside_the_rbox_retains_universal_semantics_and_source_refusal_spelling,
    stopping_during_regular_role_reads_never_publishes_a_completed_answer,
    all_nine_consumer_shapes_are_exact_and_contradictions_are_detected,
    inverse_heads_nested_dependencies_and_simple_equivalence_cycles,
    universal_obligations_clash_after_recursive_paths_and_blocking,
    cyclic_existentials_with_role_obligations_have_a_finite_blocking_closure,
    printed_order_counterexample_and_inverse_endpoint_typo_do_not_pass,
    literal_recursive_endpoint_is_not_replaced_by_an_equivalent_name,
    recorded_chain_consequence_replays_against_the_original_rbox,
    chain_derived_types_reach_all_query_services_and_materialization,
    fixed_top_is_universal_and_bottom_chain_clashes,
    fixed_top_aliases_without_a_chain_reach_query_only_individuals,
    non_simple_characteristics_and_malformed_chain_lists_hard_refuse,
    role_language_work_exhaustion_is_unknown_never_a_consistency_proof,
    cancelling_an_existing_regular_role_owner_cannot_publish_a_decision,
    nominal_merges_and_simple_functionality_preserve_role_obligations,
);
