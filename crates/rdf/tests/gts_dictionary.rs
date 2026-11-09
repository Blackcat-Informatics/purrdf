// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Real native eviction, reopening and production import/export qualification.

#[path = "support/gts_import.rs"]
mod fixture;
#[path = "support/gts_dictionary.rs"]
mod storage;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::sink::WriterDrain;
use purrdf_gts::dictionary::{
    DictionaryError, DictionaryResidence, ResidentTermStorage, SpillPolicy, SpillingDictionary,
    TermKey, TermStorage,
};
use purrdf_gts::model::Term;
use purrdf_gts::segment_decode::{PendingResolution, ResolutionStorage, ResolutionTerm};
use purrdf_gts::writer::Writer;
use purrdf_lex::allocation::{Ceiling, Memory};
use purrdf_rdf::{
    SerializeGraph, export_gts_rdf_nquads_with_dictionary, import_gts_events,
    import_gts_events_with_dictionary, parse_dataset, serialize_dataset,
};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[derive(Default)]
struct Events(Vec<String>);
impl purrdf_gts::reader::StreamingSink for Events {
    fn term(&mut self, segment: u64, id: usize, term: std::sync::Arc<Term>) {
        self.0.push(format!("term:{segment}:{id}:{term:?}"));
    }
    fn stored_term(
        &mut self,
        segment: u64,
        id: usize,
        term: purrdf_gts::dictionary::TermLease,
    ) -> Result<(), DictionaryError> {
        self.0.push(format!("term:{segment}:{id}:{:?}", *term));
        Ok(())
    }
    fn frame(&mut self, frame: purrdf_gts::reader::FrameContext<'_>) {
        self.0.push(format!("frame:{frame:?}"));
    }
    fn quad(&mut self, segment: u64, value: purrdf_gts::model::Quad) {
        self.0.push(format!("quad:{segment}:{value:?}"));
    }
    fn reifier(&mut self, segment: u64, value: purrdf_gts::model::ReifierRow) {
        self.0.push(format!("reifier:{segment}:{value:?}"));
    }
    fn annotation(&mut self, segment: u64, value: purrdf_gts::model::AnnotationRow) {
        self.0.push(format!("annotation:{segment}:{value:?}"));
    }
    fn blob(&mut self, segment: u64, digest: &str, meta: Option<&purrdf_lex::cbor::Value>) {
        self.0.push(format!("blob:{segment}:{digest}:{meta:?}"));
    }
    fn suppression(&mut self, segment: u64, value: &purrdf_gts::model::Suppression) {
        self.0.push(format!("suppression:{segment}:{value:?}"));
    }
    fn signature(&mut self, segment: u64, value: &purrdf_gts::model::Signature) {
        self.0.push(format!("signature:{segment}:{value:?}"));
    }
    fn opaque(&mut self, segment: u64, value: &purrdf_gts::model::OpaqueNode) {
        self.0.push(format!("opaque:{segment}:{value:?}"));
    }
    fn diagnostic(&mut self, value: &purrdf_gts::model::Diagnostic) {
        self.0.push(format!(
            "diagnostic:{}:{}:{:?}",
            value.code, value.detail, value.frame_index
        ));
    }
    fn segment_head(&mut self, segment: u64, head: &[u8]) {
        self.0.push(format!("head:{segment}:{head:?}"));
    }
    fn streamable_layout(&mut self, segment: u64, info: &purrdf_gts::model::StreamableInfo) {
        self.0.push(format!("layout:{segment}:{info:?}"));
    }
}

fn check_event_order(bytes: &[u8], root: &std::path::Path) {
    let mut expected = Events::default();
    let resident = purrdf_gts::reader::read_to_sink(bytes, true, None, &mut expected);
    let mut stored = Events::default();
    let mut files = storage::Files::new(root);
    let result = purrdf_gts::reader::read_to_sink_with_dictionary(
        bytes,
        purrdf_gts::reader::ReadOptions::new(true, None),
        &mut files,
        &mut stored,
    )
    .expect("stored reader events");
    assert_eq!(stored.0, expected.0, "exact source event and sidecar order");
    assert_eq!(result.segment_heads, resident.segment_heads);
    assert_eq!(result.segment_profiles, resident.segment_profiles);
    assert_eq!(result.segment_meta, resident.segment_meta);
    assert_eq!(result.torn, resident.torn);
}

fn render(dataset: &purrdf_rdf::ir::RdfDataset) -> Vec<u8> {
    let bytes = serialize_dataset(dataset, "application/n-quads", SerializeGraph::Dataset)
        .expect("render RDF 1.2");
    let text = String::from_utf8(bytes).expect("UTF-8 RDF text");
    let mut lines: Vec<_> = text.lines().collect();
    lines.sort_unstable();
    lines.join("\n").into_bytes()
}

#[test]
fn native_eviction_reopening_and_actual_import_export_preserve_complete_rdf() {
    let directory = purrdf_testkit::temp_dir!().expect("native qualification directory");
    let root = directory.path();
    let mut expected = None;
    for snapshot in [false, true] {
        let bytes = fixture::source(1, snapshot);
        check_event_order(&bytes, &root.join(format!("events-{snapshot}")));
        let (resident, _) = fixture::observe(&bytes);
        let mut raw = storage::Files::new(&root.join(format!("raw-{snapshot}")));
        let replay = raw.replay();
        let mut resolution = storage::Resolution::new(&root.join(format!("import-{snapshot}")));
        let imported = import_gts_events_with_dictionary(&bytes, &mut raw, &mut resolution, replay)
            .expect("native file-backed importer");
        fixture::check_content(&imported, &mut expected);
        assert_eq!(render(&imported.dataset), render(&resident.dataset));
        assert_eq!(imported.envelope, resident.envelope);
        assert_eq!(imported.dataset.term_count(), resident.dataset.term_count());
        for index in 0..resident.dataset.term_count() {
            let id =
                purrdf_core::TermId::from_index(u32::try_from(index).expect("fixture term index"))
                    .expect("fixture term identity");
            assert_eq!(
                imported.dataset.resolve(id),
                resident.dataset.resolve(id),
                "consumer allocation order at term {index}"
            );
        }
        assert_eq!(
            imported.dataset.owned_quads().collect::<Vec<_>>(),
            resident.dataset.owned_quads().collect::<Vec<_>>()
        );
        assert_eq!(
            imported.dataset.owned_reifiers().collect::<Vec<_>>(),
            resident.dataset.owned_reifiers().collect::<Vec<_>>()
        );
        assert_eq!(
            imported.dataset.owned_annotations().collect::<Vec<_>>(),
            resident.dataset.owned_annotations().collect::<Vec<_>>()
        );
        assert!(raw.reads > 9, "validation must reload evicted entries");
        drop(raw);
        let mut raw = storage::Files::new(&root.join(format!("raw-{snapshot}")));
        let key = TermKey {
            segment: 0,
            term: 1,
        };
        let lease = raw
            .get(key)
            .expect("reload persisted term")
            .expect("term survives reopen");
        drop(raw);
        assert!(
            lease
                .value
                .as_deref()
                .is_some_and(|value| value.starts_with("https://"))
        );

        let mut raw = storage::Files::new(&root.join(format!("text-{snapshot}")));
        let replay = raw.replay();
        let mut resolution = storage::Resolution::new(&root.join(format!("export-{snapshot}")));
        let mut output = WriterDrain(Vec::new());
        let mut ceiling = Ceiling(128 * 1024);
        let result = export_gts_rdf_nquads_with_dictionary(
            &bytes,
            &mut raw,
            &mut resolution,
            replay,
            &mut output,
            &mut Memory::new(&mut ceiling),
        )
        .expect("stored textual export");
        assert_eq!(result.written as usize, output.0.len());
        assert_eq!(result.envelope, resident.envelope);
        let text = std::str::from_utf8(&output.0).expect("UTF-8 projection");
        let rows: Vec<_> = text.lines().collect();
        assert_eq!(rows.len(), 5, "three quads, one reifier and one annotation");
        assert!(rows[0].contains("<<("));
        assert!(rows[1].contains("\"directional\"@en--rtl"));
        assert!(rows[2].contains("\"typed\"^^"));
        assert!(rows[3].contains("#reifies>"));
        assert!(rows[4].contains("\"directional\"@en--rtl"));
        let parsed = parse_dataset(&output.0, "application/n-quads", None)
            .expect("reparse exported RDF 1.2");
        assert_eq!(render(&parsed), render(&resident.dataset));
        let late = raw.reads + raw.writes - 1;
        let mut failed = storage::Files::new(&root.join(format!("late-{snapshot}")));
        failed.cancel_after = Some(late);
        let replay = failed.replay();
        let mut resolution = storage::Resolution::new(&root.join(format!("late-state-{snapshot}")));
        let mut memory = Memory::new(&mut ceiling);
        let error = export_gts_rdf_nquads_with_dictionary(
            &bytes,
            &mut failed,
            &mut resolution,
            replay,
            &mut WriterDrain(Vec::new()),
            &mut memory,
        )
        .expect_err("late storage cancellation cannot report complete output");
        assert!(has_original_refusal(&error));
        assert_eq!(
            memory.admitted_bytes(),
            0,
            "failed output releases its own scratch"
        );
    }
}

fn has_original_refusal(error: &(dyn std::error::Error + 'static)) -> bool {
    source_chain_has(error, |error| {
        error
            .downcast_ref::<purrdf_testkit::error_capture::CallerRefusal>()
            .is_some_and(|cause| cause.limit == u128::MAX)
    })
}

fn source_chain_has(
    error: &(dyn std::error::Error + 'static),
    predicate: impl Fn(&(dyn std::error::Error + 'static)) -> bool,
) -> bool {
    let mut cause = Some(error);
    while let Some(error) = cause {
        if predicate(error) {
            return true;
        }
        cause = error.source();
    }
    false
}

#[test]
fn text_refusal_refunds_partial_blank_label_scratch_before_returning() {
    let directory = purrdf_testkit::temp_dir!().expect("refusal directory");
    let mut writer = Writer::new("rdf12");
    let mut anonymous = Term::blank("");
    anonymous.value = None;
    writer.add_terms(&[
        anonymous,
        Term::iri("https://example.org/p"),
        Term::iri("https://example.org/o"),
    ]);
    writer.add_quads(&[(0, 1, 2, None)]);
    let bytes = writer.into_bytes();
    for extra in [1, 32, 64 * 1024] {
        let mut files = storage::Files::new(&directory.path().join(format!("raw-{extra}")));
        let replay = files.replay();
        let mut state = storage::Resolution::new(&directory.path().join(format!("state-{extra}")));
        let mut ceiling = Ceiling(purrdf_core::sink::DRAIN_BUFFER_BYTES + extra);
        let mut memory = Memory::new(&mut ceiling);
        let result = export_gts_rdf_nquads_with_dictionary(
            &bytes,
            &mut files,
            &mut state,
            replay,
            &mut WriterDrain(Vec::new()),
            &mut memory,
        );
        assert_eq!(
            result.is_ok(),
            extra == 64 * 1024,
            "adjacent admitted neighbour"
        );
        assert_eq!(
            memory.admitted_bytes(),
            0,
            "destroyed partial label cannot retain a charge"
        );
    }
}

#[test]
fn native_records_refuse_corruption_cancellation_and_frame_limits() {
    let directory = purrdf_testkit::temp_dir!().expect("native qualification directory");
    let root = directory.path();
    let mut files = storage::Files::new(root);
    let key = TermKey {
        segment: 0,
        term: 0,
    };
    files
        .put(key, &Term::iri("https://example.org/pinned"))
        .expect("persist with interrupted short writes");
    let lease = files.get(key).expect("read").expect("present");
    files.corrupt(key);
    assert!(matches!(files.get(key), Err(DictionaryError::Corrupt(_))));
    assert_eq!(lease.value.as_deref(), Some("https://example.org/pinned"));
    files.cancel_after = Some(0);
    let error = files.get(key).expect_err("original cancellation");
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(&error);
    let mut original = false;
    while let Some(error) = cause {
        original |= error
            .downcast_ref::<purrdf_testkit::error_capture::CallerRefusal>()
            .is_some_and(|cause| cause.limit == u128::MAX);
        cause = error.source();
    }
    assert!(original);
    files.cancel_after = None;
    files.limits.encoded_frame_bytes = 16;
    let bytes = fixture::source(1, false);
    let replay = files.replay();
    let mut resolution = storage::Resolution::new(&root.join("resolution"));
    assert!(
        import_gts_events_with_dictionary(&bytes, &mut files, &mut resolution, replay).is_err()
    );
    files.limits.encoded_frame_bytes = 8192;
    let mut writer = Writer::new("rdf12");
    writer.add_frame(
        "terms",
        None,
        Some(purrdf_lex::cbor::encode(&purrdf_lex::cbor::Value::Array(
            vec![purrdf_gts::writer::term_to_wire(&Term::iri(
                "https://example.org/a-decoded-term",
            ))],
        ))),
        Some(&["gzip".to_owned()]),
        None,
    );
    let bytes = writer.into_bytes();
    for limit in [16, 8192] {
        files.limits.decoded_frame_bytes = limit;
        let replay = files.replay();
        let mut resolution = storage::Resolution::new(&root.join(format!("decoded-{limit}")));
        let result = import_gts_events_with_dictionary(&bytes, &mut files, &mut resolution, replay);
        if limit == 16 {
            let error =
                result.expect_err("decoded window is enforced independently of encoded size");
            assert!(source_chain_has(&error, |error| matches!(
                error.downcast_ref::<purrdf_gts::codec::CodecError>(),
                Some(purrdf_gts::codec::CodecError::Limit { limit: 16 })
            )));
        } else {
            result.expect("adjacent admitted decompression window");
        }
    }
}

#[test]
fn native_spill_commits_before_refund_and_never_falls_back_after_failure() {
    let directory = purrdf_testkit::temp_dir!().expect("native spill directory");
    let root = directory.path();
    let mut unused = storage::Resident::default();
    unused.reject_limits = true;
    let mut direct = SpillingDictionary::new(
        unused,
        storage::Files::new(&root.join("direct")),
        SpillPolicy::External,
    );
    direct
        .limits()
        .expect("direct external operation ignores unused resident policy");
    direct
        .put(
            TermKey {
                segment: 0,
                term: 0,
            },
            &Term::iri("https://example.org/direct"),
        )
        .expect("direct external insertion");
    assert_eq!(direct.residence(), DictionaryResidence::External);
    assert_eq!(direct.transitions(), 0);
    let key = TermKey {
        segment: 0,
        term: 0,
    };
    let mut occupied = storage::Resident::default();
    occupied
        .put(key, &Term::iri("https://example.org/existing"))
        .expect("seed resident identity");
    let mut invalid = SpillingDictionary::new(
        occupied,
        storage::Files::new(&root.join("invalid-direct")),
        SpillPolicy::External,
    );
    assert!(
        invalid.get(key).is_err(),
        "direct selection cannot silently lose existing resident keys"
    );
    let mut occupied = storage::Resident::default();
    occupied
        .put(key, &Term::iri("https://example.org/existing"))
        .expect("seed over-limit neighbour");
    let mut admitted = SpillingDictionary::new(
        occupied,
        storage::Files::new(&root.join("initial-spill")),
        SpillPolicy::AtBytes(0),
    );
    assert!(
        admitted
            .get(key)
            .expect("initial admission migrates before lookup")
            .is_some()
    );
    assert_eq!(admitted.transitions(), 1);
    let resident = storage::Resident::default();
    let fixed = resident.retained_bytes();
    let mut spill = SpillingDictionary::new(
        resident,
        storage::Files::new(&root.join("success")),
        SpillPolicy::AtBytes(fixed + 1024),
    );
    let first = TermKey {
        segment: 0,
        term: 0,
    };
    spill
        .put(first, &Term::iri("https://example.org/pinned"))
        .expect("resident neighbour");
    assert_eq!(spill.residence(), DictionaryResidence::Resident);
    let pin = spill
        .get(first)
        .expect("resident lookup")
        .expect("resident term");
    spill
        .bind_first(first, (0, 0, 0))
        .expect("resident first binding");
    spill
        .put(
            TermKey {
                segment: 0,
                term: 1,
            },
            &Term::literal("x".repeat(2048), None),
        )
        .expect("committed native migration");
    assert_eq!(spill.residence(), DictionaryResidence::External);
    assert_eq!(spill.transitions(), 1);
    assert_eq!(
        spill.retained_resident_bytes(),
        fixed + pin.retained_bytes(),
        "eviction cannot refund an escaped original pin"
    );
    assert_eq!(
        spill.first_binding(first).expect("binding reload"),
        Some((0, 0, 0))
    );
    let (_, external) = spill.into_parts();
    drop(external);
    let mut reopened = storage::Files::new(&root.join("success"));
    assert_eq!(
        reopened
            .get(first)
            .expect("reopen")
            .expect("migrated term")
            .value,
        pin.value
    );
    let path = root.join("reopened-resolution");
    let mut state = storage::Resolution::<TermKey>::new(&path);
    state
        .set_pending(
            first,
            PendingResolution {
                term: Some(ResolutionTerm::Stored(pin.clone())),
                parent: None,
                next: 0,
                active: false,
            },
        )
        .expect("persist pending identity");
    drop(state);
    let mut state = storage::Resolution::<TermKey>::new(&path);
    assert_eq!(state.counts().pending, 1);
    assert_eq!(
        state
            .pending(first)
            .expect("reload pending")
            .expect("pending present")
            .term
            .expect("pinned raw payload")
            .value,
        pin.value
    );
    state
        .set_resolved(first, first)
        .expect("persist target identity");
    state
        .remove_pending(first)
        .expect("remove durable completed continuation");
    state.complete_segment(0).expect("commit completed mapping");
    drop(state);
    let mut state = storage::Resolution::<TermKey>::new(&path);
    assert_eq!(
        state.resolved(first).expect("reopen target mapping"),
        Some(first)
    );
    assert_eq!(
        state.counts().pending + state.counts().resolved,
        0,
        "completed state is not live workspace"
    );
    let mut external = storage::Files::new(&root.join("failure"));
    external.cancel_after = Some(0);
    let mut failed = SpillingDictionary::new(
        storage::Resident::default(),
        external,
        SpillPolicy::AtBytes(fixed),
    );
    assert!(
        failed
            .put(first, &Term::iri("https://example.org/refused"))
            .is_err()
    );
    assert_eq!(failed.residence(), DictionaryResidence::Failed);
    assert!(failed.get(first).is_err());
    assert!(failed.commit().is_err());
}

#[test]
fn scoped_nested_and_composite_terms_preserve_identity_and_native_refusal() {
    let directory = purrdf_testkit::temp_dir!().expect("nested native directory");
    let root = directory.path();
    for malformed in [false, true] {
        let mut bytes = Vec::new();
        for _ in 0..2 {
            let mut writer = Writer::new("rdf12");
            let mut language = Term::lang_literal("ordinary text", "en");
            // Language determines the effective datatype even when a raw dt
            // names a composite datatype.
            language.datatype = Some(2);
            let mut indirect = Term::triple_term((1, 0, 3));
            indirect.triple = None;
            indirect.reifier = Some(1);
            writer.add_terms(&[
                Term::iri("https://example.org/p"),
                Term::blank("b"),
                Term::iri("http://w3id.org/awslabs/neptune/SPARQL-CDTs/List"),
                Term::literal(if malformed { "[" } else { "[_:b]" }, Some(2)),
                Term::triple_term((1, 0, 3)),
                Term::triple_term((1, 0, 4)),
                indirect,
                language,
            ]);
            writer.add_quads(&[
                (1, 0, 3, None),
                (1, 0, 5, None),
                (1, 0, 6, None),
                (1, 0, 7, None),
            ]);
            writer.add_reifies(&[(1, (1, 0, 3), None)]);
            writer.add_blob(b"original blob sidecar", None, None);
            bytes.extend_from_slice(&writer.into_bytes());
        }
        let resident = import_gts_events(&bytes);
        check_event_order(&bytes, &root.join(format!("events-{malformed}")));
        let mut raw = storage::Files::new(&root.join(format!("raw-{malformed}")));
        let replay = raw.replay();
        let mut resolution = storage::Resolution::new(&root.join(format!("state-{malformed}")));
        let mut output = WriterDrain(Vec::new());
        let mut ceiling = Ceiling(128 * 1024);
        let stored = export_gts_rdf_nquads_with_dictionary(
            &bytes,
            &mut raw,
            &mut resolution,
            replay,
            &mut output,
            &mut Memory::new(&mut ceiling),
        );
        if malformed {
            assert!(resident.is_err(), "native composite refusal");
            assert!(stored.is_err(), "stored composite refusal");
        } else {
            stored.expect("scoped nested export");
            let resident = resident.expect("native scoped terms");
            let parsed =
                parse_dataset(&output.0, "application/n-quads", None).expect("nested output");
            assert_eq!(render(&parsed), render(&resident.dataset));
        }
    }
}

#[test]
fn growing_irregular_terms_keep_text_workspace_bounded_outside_reported_metadata() {
    let directory = purrdf_testkit::temp_dir!().expect("native qualification directory");
    let root = directory.path();
    for count in [64usize, 257, 1025] {
        let mut writer = Writer::new("rdf12");
        writer.add_terms(&[Term::iri("https://example.org/p")]);
        for index in 0..count {
            let text = format!(
                "https://example.org/{index}/{}",
                "x".repeat(((index * 37) % 173) * 19)
            );
            writer.add_terms(&[Term::iri(text)]);
            writer.add_quads(&[(index + 1, 0, index + 1, None)]);
        }
        let bytes = writer.into_bytes();
        let expected = import_gts_events(&bytes).expect("resident comparison");
        let mut raw = storage::Files::new(&root.join(format!("raw-{count}")));
        let replay = raw.replay();
        let mut resolution = storage::Resolution::new(&root.join(format!("state-{count}")));
        let output_path = root.join(format!("output-{count}.nq"));
        let mut output =
            WriterDrain(std::fs::File::create(&output_path).expect("caller output file"));
        let mut ceiling = Ceiling(128 * 1024);
        let window = CurrentThreadWindow::open();
        let exported = export_gts_rdf_nquads_with_dictionary(
            &bytes,
            &mut raw,
            &mut resolution,
            replay,
            &mut output,
            &mut Memory::new(&mut ceiling),
        )
        .expect("growing file-backed export");
        let measured = window.close();
        drop(output);
        let output_bytes =
            std::fs::read(output_path).expect("read complete file outside measurement");
        let parsed =
            parse_dataset(&output_bytes, "application/n-quads", None).expect("complete export");
        assert_eq!(render(&parsed), render(&expected.dataset));
        // This whole-call bound includes observed reader metadata; it is a
        // regression budget for these three corpus sizes, not an extrapolated
        // universal bound or a subtraction of estimated metadata bytes.
        assert!(measured.peak_working_bytes <= 512 * 1024, "{measured:?}");
        assert_eq!(raw.metadata.frame_ids, count * 2 + 1);
        assert_eq!(raw.metadata.provenance_subjects, 0);
        println!(
            "terms={count} source={} output={} metadata={:?} reads={} writes={} allocations={} requested={} retained={} peak={}",
            bytes.len(),
            exported.written,
            raw.metadata,
            raw.reads,
            raw.writes,
            measured.allocations,
            measured.requested_bytes,
            measured.retained_bytes,
            measured.peak_working_bytes
        );
    }
}
