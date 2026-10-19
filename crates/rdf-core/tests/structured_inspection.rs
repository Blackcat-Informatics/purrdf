// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Inspect original graph payloads and actual native refusals without rendering.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_hash::error::ErrorValue;
use purrdf_testkit::error_capture::Capture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn structured_graph_fixture() -> std::sync::Arc<purrdf_core::RdfDataset> {
    use purrdf_core::{
        BlankScope, ContentIdScheme, RdfDatasetBuilder, RdfLocation, RdfTextDirection, TermBox,
        TermFactory, TermValue,
    };
    let mut builder = RdfDatasetBuilder::with_content_addressing(
        ContentIdScheme::new("urn:content:").unwrap(),
        Some("https://example.org/derivation".into()),
    );
    let subject = TermValue::Blank {
        label: "original".into(),
        scope: BlankScope(37),
    };
    let predicate = TermValue::iri("https://example.org/p");
    let value = TermValue::Literal {
        lexical_form: "方向".into(),
        datatype: purrdf_iri::vocab::rdf::DIR_LANG_STRING.into(),
        language: Some("zh".into()),
        direction: Some(RdfTextDirection::Rtl),
    };
    let triple = TermValue::Triple {
        s: TermBox::new(subject.clone()),
        p: TermBox::new(predicate.clone()),
        o: TermBox::new(value.clone()),
    };
    let s = builder.intern_value(&subject).unwrap();
    let p = builder.intern_value(&predicate).unwrap();
    let o = builder.intern_value(&value).unwrap();
    let t = builder.intern_value(&triple).unwrap();
    let r = builder.intern_iri("https://example.org/reifier").unwrap();
    let second = builder.intern_iri("https://example.org/second").unwrap();
    let g = builder.intern_iri("https://example.org/graph").unwrap();
    let empty = builder.intern_iri("https://example.org/empty").unwrap();
    let handle = builder.push_quad_with_handle(s, p, o, None).unwrap();
    builder.attach_location(
        handle,
        RdfLocation {
            path: Some("source.ttl".into()),
            line: Some(u64::MAX),
            ..RdfLocation::default()
        },
    );
    builder.push_quad(second, p, o, None).unwrap();
    builder.push_quad(s, p, o, Some(g)).unwrap();
    builder.push_reifier_in_graph(r, t, Some(g)).unwrap();
    builder.push_annotation_in_graph(r, p, o, Some(g)).unwrap();
    builder.declare_named_graph(empty).unwrap();
    builder.freeze().unwrap()
}

fn graph_result_inspection_borrows_all_rdf_streams_and_private_dictionary_fields() {
    use purrdf_core::SparqlResult;
    let data = structured_graph_fixture();
    let result = SparqlResult::Graph(data.clone().into());
    let graph = Capture::read(&result).child("graph");
    let events = Vec::with_capacity(256);
    let window = CurrentThreadWindow::open();
    let exact = Capture::read_with_buffer(data.as_ref(), events);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(graph.code, "rdf-dataset");
    assert_eq!(exact.records("quads").len(), 3);
    assert_eq!(exact.records("reifiers").len(), 1);
    assert_eq!(exact.records("annotations").len(), 1);
    assert!(
        matches!(exact.records("locations")[0].child("location").field("line"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    let ErrorValue::Record(dictionary) = exact.field("dictionary") else {
        panic!("the original dictionary is retained")
    };
    let events = Vec::with_capacity(256);
    let window = CurrentThreadWindow::open();
    let dictionary = Capture::read_with_buffer(dictionary, events);
    assert_eq!(window.close().allocations, 0);
    assert!(matches!(
        dictionary.child("content_id_scheme").field("prefix"),
        ErrorValue::Text("urn:content:")
    ));
    assert!(matches!(
        dictionary.field("derivation_predicate_iri"),
        ErrorValue::Iri("https://example.org/derivation")
    ));
    let terms = dictionary.records("terms");
    assert_eq!(terms.len(), data.term_count());
    let literal = terms
        .iter()
        .find(|term| term.child("term").code == "rdf-interned-literal")
        .unwrap()
        .child("term");
    assert!(matches!(
        literal.field("lexical_form"),
        ErrorValue::Text("方向")
    ));
    assert!(matches!(literal.field("language"), ErrorValue::Text("zh")));
    assert!(matches!(
        literal.field("direction"),
        ErrorValue::Code("rtl")
    ));
    assert!(matches!(
        terms
            .iter()
            .find(|term| term.child("term").code == "rdf-interned-blank")
            .unwrap()
            .child("term")
            .field("scope"),
        ErrorValue::Unsigned(37)
    ));
    assert!(
        terms
            .iter()
            .any(|term| term.child("term").code == "rdf-interned-triple")
    );
    assert!(matches!(
        exact.child("capabilities").field("named_graphs"),
        ErrorValue::Boolean(true)
    ));
}

fn native_evidence_alignment_and_answer_state_failures_remain_structured() {
    use purrdf_core::evidence::{
        AnswerAccumulator, AnswerRequest, AnswerTier, EvidenceColumn, EvidenceId,
        EvidenceWorkspace, SearchCompleteness, StandpointKey,
    };
    use purrdf_lex::allocation::Resident;
    let mut owner = EvidenceWorkspace::with_admission(Resident);
    let mut column = EvidenceColumn::uniform_default(0);
    let original = owner
        .set_row(&mut column, usize::MAX, EvidenceId::default())
        .unwrap_err();
    let captured = Capture::read(&original);
    assert_eq!(captured.code, "evidence-workspace-column");
    assert_eq!(
        captured.child("cause").code,
        "evidence-column-row-out-of-bounds"
    );
    let error = owner.canonical_index().unwrap_err();
    assert_eq!(Capture::read(&error).code, "evidence-workspace-aborted");
    let mut answer = AnswerAccumulator::with_admission(
        AnswerRequest {
            tier: AnswerTier::Summary,
            question: None,
            standpoint: StandpointKey::Unscoped,
            input_completeness: SearchCompleteness::Complete,
            closure_declared: false,
        },
        Resident,
    );
    let error = answer.status().unwrap_err();
    assert_eq!(Capture::read(&error).code, "answer-evidence-not-finished");
    let error = answer.finish(false).unwrap_err();
    assert_eq!(
        Capture::read(&error).code,
        "answer-evidence-enumeration-required"
    );
    answer.finish(true).unwrap();
    let error = answer.finish(true).unwrap_err();
    assert_eq!(
        Capture::read(&error).code,
        "answer-evidence-already-finished"
    );
}

fn xpath_actual_refusals_keep_profile_offsets_and_full_width_resource_counts() {
    use purrdf_core::xsd_regex::xpath::{self, Limits, Profile, Resource};
    let error = xpath::compile(Profile::Xpath31, "a", "z", Limits::default()).unwrap_err();
    let cause = Capture::read(&error);
    assert_eq!(cause.code, "xpath-invalid-flags");
    assert!(matches!(cause.field("flag"), ErrorValue::Character('z')));
    assert!(matches!(cause.field("offset"), ErrorValue::ByteCount(0)));
    assert_eq!(cause.child("profile").code, "xpath-profile-3.1-2017-03-21");
    let refusal = Limits::default()
        .with(Resource::PatternBytes, 0)
        .admit(Resource::PatternBytes, u128::MAX)
        .unwrap_err();
    let capture = Capture::read(&refusal);
    assert!(matches!(
        capture.field("required"),
        ErrorValue::Unsigned(u128::MAX)
    ));
    assert!(matches!(capture.field("limit"), ErrorValue::Unsigned(0)));
    let source = xpath::compile(
        Profile::Xpath20,
        "a",
        "",
        Limits::default().with(Resource::PatternBytes, 0),
    )
    .unwrap_err();
    assert_eq!(
        Capture::read(&source).child("cause").child("resource").code,
        "xpath-resource-pattern-bytes"
    );
}

fn graph_refusal_keeps_original_invalid_edge_without_rendering() {
    use purrdf_lex::allocation::{Memory, Resident};
    let adjacency = [vec![usize::MAX]];
    let mut admission = Resident;
    let error =
        purrdf_core::graph::tarjan_scc_with_memory(&adjacency, &mut Memory::new(&mut admission))
            .unwrap_err();
    let events = Vec::with_capacity(2);
    let window = CurrentThreadWindow::open();
    let fields = Capture::read_with_buffer(&error, events);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(fields.code, "graph-invalid-edge");
    assert!(matches!(fields.field("from"), ErrorValue::Unsigned(0)));
    assert!(
        matches!(fields.field("to"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
    );
}

fn event_ingestion_retains_original_freeze_causes_and_cancellation() {
    use core::ops::ControlFlow;
    use purrdf_core::DatasetSink;
    use purrdf_events::{EventError, EventQuad, EventTerm, EventTermId, RdfEventSink};
    let mut cancelled = DatasetSink::new();
    cancelled.mark_cancelled();
    let error = cancelled.finish().unwrap_err();
    assert_eq!(error, EventError::Cancelled);
    assert_eq!(Capture::read(&error).code, "event-cancelled");
    assert!(cancelled.into_dataset().is_none());

    for invalid in [true, false] {
        let mut sink = DatasetSink::new();
        let s = EventTermId(10);
        let p = EventTermId(20);
        let o = EventTermId(30);
        assert_eq!(
            sink.term(s, EventTerm::Iri("https://example.org/s"))
                .unwrap(),
            ControlFlow::Continue(())
        );
        let predicate = if invalid {
            EventTerm::Literal {
                lexical: "not-a-predicate",
                datatype: purrdf_xsd::datatype::XSD_STRING,
                language: None,
                direction: None,
            }
        } else {
            EventTerm::Iri("https://example.org/p")
        };
        assert_eq!(sink.term(p, predicate).unwrap(), ControlFlow::Continue(()));
        assert_eq!(
            sink.term(o, EventTerm::Iri("https://example.org/o"))
                .unwrap(),
            ControlFlow::Continue(())
        );
        assert_eq!(
            sink.quad(EventQuad { s, p, o, g: None }).unwrap(),
            ControlFlow::Continue(())
        );
        if invalid {
            let error = sink.finish().unwrap_err();
            assert!(
                sink.into_dataset().is_none(),
                "refused data must never freeze"
            );
            let original = std::error::Error::source(&error).unwrap().source().unwrap();
            assert!(original.is::<purrdf_core::RdfDiagnostic>());
            let fields = Capture::read(&error);
            assert_eq!(fields.code, "event-native");
            let freeze = fields.child("cause");
            assert_eq!(freeze.code, "event-ingest-freeze");
            let diagnostic = freeze.child("cause");
            assert_eq!(diagnostic.code, "rdf-diagnostic");
            assert!(matches!(
                diagnostic.field("code"),
                ErrorValue::Code("rdf-ir-predicate-not-iri")
            ));
        } else {
            sink.finish().unwrap();
            assert_eq!(sink.into_dataset().unwrap().quad_count(), 1);
        }
    }
}

fn byte_cover_refusals_keep_original_ranges_and_nested_digest_finding() {
    use purrdf_core::cover::{ByteCover, CoverBuilder};

    let cover = ByteCover::identity(b"abc");
    for end in [4, u64::MAX] {
        let error = cover.reference(1..end).unwrap_err();
        let captured = Capture::read(&error);
        assert_eq!(captured.code, "cover-emit-range");
        assert!(matches!(captured.field("start"), ErrorValue::ByteCount(1)));
        assert!(
            matches!(captured.field("end"), ErrorValue::ByteCount(value) if value == u128::from(end))
        );
        assert!(matches!(
            captured.field("byte_length"),
            ErrorValue::ByteCount(3)
        ));
    }
    let reference = cover.reference(1..3).unwrap();
    assert_eq!(reference.resolve_source(b"abc").unwrap(), b"bc");
    let error = reference.resolve_source(b"abd").unwrap_err();
    let captured = Capture::read(&error);
    assert_eq!(captured.code, "cover-emit-cover");
    let cause = captured.child("cause");
    assert_eq!(cause.code, "cover-digest-mismatch");
    let expected = purrdf_core::ContentDigest::of(b"abc").to_hex();
    let actual = purrdf_core::ContentDigest::of(b"abd").to_hex();
    assert!(matches!(cause.field("expected"), ErrorValue::Text(value) if value == expected));
    assert!(matches!(cause.field("reconstructed"), ErrorValue::Text(value) if value == actual));
    let error = CoverBuilder::new(b"abc").finish().unwrap_err();
    let captured = Capture::read(&error).child("cause");
    assert_eq!(captured.code, "cover-uncovered-range");
    assert!(matches!(
        captured.field("byte_start"),
        ErrorValue::ByteCount(0)
    ));
    assert!(matches!(
        captured.field("byte_end"),
        ErrorValue::ByteCount(3)
    ));
    assert_eq!(cover.reconstruct().unwrap(), b"abc");
}

fn embedding_attachment_failures_keep_original_pack_causes_after_input_drop() {
    use purrdf_core::{
        CertifiedPurrpckSource, ContentDigest, EmbeddingError, EmbeddingSourceError,
        ExternalBinding, ExternalBindingContract, ExternalScope, PackError, RdfDatasetBuilder,
    };
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let (_, mut bytes) = CertifiedPurrpckSource::from_dataset(&dataset).unwrap();
    bytes[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    let contract = ExternalBindingContract {
        role: "https://example.org/role/source".into(),
        media_type: "application/vnd.purrdf.pack".into(),
        stable_identifier: None,
        revision: None,
        policy_reference: None,
    };
    let scope = ExternalScope::Source(ContentDigest::of(b"owner"));
    let source = CertifiedPurrpckSource::certify(&bytes).unwrap_err();
    let external = ExternalBinding::from_purrpck(scope, &bytes, &contract).unwrap_err();
    drop(bytes);
    drop(dataset);
    drop(contract);
    for (error, code) in [
        (source, "purremb-invalid-source-pack"),
        (external, "purremb-invalid-external-pack"),
    ] {
        let window = CurrentThreadWindow::open();
        let retained = error.clone();
        assert_eq!(window.close().allocations, 0);
        drop(error);
        let owner = match &retained {
            EmbeddingError::InvalidSourcePack(cause)
            | EmbeddingError::InvalidExternalPack(cause) => cause,
            other => panic!("unexpected attachment refusal: {other:?}"),
        };
        assert!(matches!(
            owner.as_ref(),
            EmbeddingSourceError::Pack(PackError::UnsupportedVersion(u32::MAX))
        ));
        let source = std::error::Error::source(&retained).unwrap();
        let native = source.source().unwrap();
        assert_eq!(
            native.downcast_ref::<PackError>(),
            Some(&PackError::UnsupportedVersion(u32::MAX))
        );
        let buffer = Vec::with_capacity(32);
        let window = CurrentThreadWindow::open();
        let captured = Capture::read_with_buffer(&retained, buffer);
        assert_eq!(window.close().allocations, 0);
        assert_eq!(captured.code, code);
        let native = captured.child("cause").child("cause");
        assert_eq!(native.code, "pack-unsupported-version");
        assert!(matches!(
            native.field("version"),
            ErrorValue::Unsigned(value) if value == u128::from(u32::MAX)
        ));
    }
}

purrdf_testkit::harness_main!(
    embedding_attachment_failures_keep_original_pack_causes_after_input_drop,
    byte_cover_refusals_keep_original_ranges_and_nested_digest_finding,
    event_ingestion_retains_original_freeze_causes_and_cancellation,
    graph_result_inspection_borrows_all_rdf_streams_and_private_dictionary_fields,
    native_evidence_alignment_and_answer_state_failures_remain_structured,
    xpath_actual_refusals_keep_profile_offsets_and_full_width_resource_counts,
    graph_refusal_keeps_original_invalid_edge_without_rendering,
);
