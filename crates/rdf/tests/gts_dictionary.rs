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
    import_gts_events_with_dictionary, import_gts_indexed_term, parse_dataset, serialize_dataset,
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

type IndexedIdentity = Box<
    dyn FnMut(
        &std::io::Cursor<Vec<u8>>,
    ) -> std::io::Result<purrdf_gts::reader::indexed::AccessBinding>,
>;
type IndexedFiles<'a> = purrdf_gts::reader::indexed::IndexedDictionary<
    std::io::Cursor<Vec<u8>>,
    IndexedIdentity,
    &'a mut storage::Files,
>;
fn indexed_files(bytes: Vec<u8>, files: &mut storage::Files) -> IndexedFiles<'_> {
    use purrdf_gts::reader::indexed::{
        AccessBinding, AccessOwner, DecodeCapabilities, IndexedCatalog, IndexedDictionary,
        IndexedInput, LookupPolicy,
    };
    let capabilities = DecodeCapabilities::new(std::sync::Arc::new(|_: &str| None));
    let policy = std::sync::Arc::new(LookupPolicy {
        allow_segments: true,
        expected_head: None,
        limits: files.limits,
    });
    let binding = AccessBinding {
        source: AccessOwner::from_owner(std::sync::Arc::new(())),
        generation: 0,
        length: bytes.len() as u64,
        capabilities: capabilities.owner().clone(),
        capability_generation: 0,
        policy: policy.owner(),
    };
    let current: IndexedIdentity = Box::new(move |_| Ok(binding.clone()));
    let input = IndexedInput::new(std::io::Cursor::new(bytes), current, files.limits)
        .expect("bound immutable source");
    IndexedDictionary::new(
        IndexedCatalog::new(input, capabilities, policy).expect("same actual decoder and policy"),
        files,
    )
    .expect("native file-backed dictionary")
}
fn indexed_fixture(snapshot: bool) -> Vec<u8> {
    use purrdf_lex::cbor::Value;
    let original = fixture::source(1, snapshot);
    let (items, torn) = purrdf_gts::wire::iter_items(&original);
    assert!(torn.is_none());
    let mut writer = Writer::new("rdf12");
    for (_, frame) in items.iter().skip(1) {
        let Value::Map(entries) = frame else {
            panic!("shared fixture frame")
        };
        let kind = purrdf_gts::wire::map_get(entries, "t")
            .and_then(Value::as_text)
            .expect("shared frame kind");
        writer.add_frame(
            kind,
            purrdf_gts::wire::map_get(entries, "d").cloned(),
            None,
            None,
            None,
        );
    }
    // Resolve both indirect and nested spellings against the shared fixture's
    // existing statement-layer binding, including its snapshot-local rebasing.
    writer.add_terms(&[
        Term {
            kind: purrdf_gts::model::TermKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(8),
            triple: None,
        },
        Term {
            kind: purrdf_gts::model::TermKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: Some((1, 2, 10)),
        },
    ]);
    writer.add_index_with_mmr();
    writer.into_bytes()
}

#[test]
fn indexed_real_consumer_matches_scoped_rdf_terms_and_complete_export() {
    let directory = purrdf_testkit::temp_dir!().expect("indexed native storage");
    let segments = [indexed_fixture(false), indexed_fixture(true)];
    let bytes = segments.concat();
    let expected = import_gts_events(&bytes).expect("existing full RDF importer");
    let mut files = storage::Files::new(directory.path());
    let mut indexed = indexed_files(bytes, &mut files);
    assert_eq!(indexed.work().read_bytes, 0);
    for segment in [0, 1] {
        indexed
            .prepare_reference_access(segment)
            .expect("explicit preparation for the fixture's legacy indirect terms");
    }
    let mut assembled = purrdf_rdf::RdfDatasetBuilder::new();
    for (segment, source) in segments.iter().enumerate() {
        let raw = purrdf_gts::reader::read(source, false, None);
        assert!(raw.diagnostics.is_empty(), "{:?}", raw.diagnostics);
        let mut ids = Vec::new();
        for term in 0..raw.terms.len() {
            let key = TermKey {
                segment: segment as u64,
                term,
            };
            let (owner, id) =
                import_gts_indexed_term(&mut indexed, key).expect("actual selective RDF consumer");
            assert_eq!(
                owner.quad_count(),
                0,
                "selected term operation imports no unrelated rows"
            );
            let value = owner.to_owned_term(id);
            ids.push(
                assembled
                    .intern_owned_term(&value)
                    .expect("collect exact selected term"),
            );
            let cold = indexed.work();
            let (warm_owner, warm_id) = import_gts_indexed_term(&mut indexed, key)
                .expect("fresh destination owner over warm source");
            assert_eq!(warm_owner.term_value(warm_id), owner.term_value(id));
            assert_eq!(indexed.work().read_bytes, cold.read_bytes);
            assert_eq!(indexed.work().decoded_payloads, cold.decoded_payloads);
        }
        for (s, p, o, graph) in raw.quads {
            assembled
                .push_quad(ids[s], ids[p], ids[o], graph.map(|g| ids[g]))
                .expect("selected quad terms");
        }
        for (reifier, (s, p, o), graph) in raw.reifiers {
            let triple = assembled
                .intern_triple(ids[s], ids[p], ids[o])
                .expect("selected reification components");
            assembled
                .push_reifier_in_graph(ids[reifier], triple, graph.map(|g| ids[g]))
                .expect("selected reifier terms");
        }
        for (reifier, predicate, object, graph) in raw.annotations {
            assembled
                .push_annotation_in_graph(
                    ids[reifier],
                    ids[predicate],
                    ids[object],
                    graph.map(|g| ids[g]),
                )
                .expect("selected annotation terms");
        }
    }
    let actual = assembled
        .freeze()
        .expect("complete selected-result export owner");
    assert_eq!(render(&actual), render(&expected.dataset));
    assert_eq!(
        actual.owned_reifiers().count(),
        expected.dataset.owned_reifiers().count()
    );
    assert_eq!(
        actual.owned_annotations().count(),
        expected.dataset.owned_annotations().count()
    );
    eprintln!(
        "indexed RDF source work {:?}; settled reader metadata {:?}",
        indexed.work(),
        indexed.metadata()
    );
}

#[test]
fn indexed_rdf_refuses_combined_metadata_budget_and_unindexed_input_explicitly() {
    use purrdf_gts::reader::indexed::{CatalogError, IndexedError};
    let directory = purrdf_testkit::temp_dir!().expect("indexed refusal storage");
    let bytes = indexed_fixture(false);
    let mut probe_files = storage::Files::new(&directory.path().join("probe"));
    let mut probe = indexed_files(bytes.clone(), &mut probe_files);
    probe
        .prepare_reference_access(0)
        .expect("explicit reference preparation");
    for term in [0, 1, 10] {
        probe
            .get(TermKey { segment: 0, term })
            .expect("admit exactly the three raw blocks");
    }
    let settled = probe.metadata().entries;
    import_gts_indexed_term(
        &mut probe,
        TermKey {
            segment: 0,
            term: 11,
        },
    )
    .expect("unbounded observation of combined live metadata");
    drop(probe);
    let combined_peak = probe_files.peak_metadata_entries;
    assert!(
        combined_peak > settled + 1,
        "fixture must require consumer metadata in addition to raw evidence"
    );
    let mut files = storage::Files::new(&directory.path().join("bounded"));
    files.limits.metadata_entries = combined_peak - 1;
    let mut indexed = indexed_files(bytes, &mut files);
    indexed
        .prepare_reference_access(0)
        .expect("explicit reference preparation fits the raw evidence budget");
    for term in [0, 1, 10] {
        indexed
            .get(TermKey { segment: 0, term })
            .expect("raw evidence fits the single combined budget");
    }
    let failure = import_gts_indexed_term(
        &mut indexed,
        TermKey {
            segment: 0,
            term: 11,
        },
    )
    .expect_err("pending closure and raw evidence share one ceiling");
    assert!(source_chain_has(
        &failure,
        |cause| matches!(cause.downcast_ref::<IndexedError>(), Some(IndexedError::Dictionary(DictionaryError::Limit { resource: "indexed-resolution-metadata" | "indexed-reifier-metadata" | "indexed-metadata-entries", required, limit })) if required > limit)
    ));
    let after = indexed.metadata().entries;
    drop(indexed);
    let committed_binding = files
        .first_binding(TermKey {
            segment: 0,
            term: 8,
        })
        .expect("inspect surviving proven raw binding");
    assert_eq!(
        after,
        settled + usize::from(committed_binding.is_some()),
        "failed consumer destroys active resolution and binding workspace while preserving completed raw evidence"
    );
    let mut files = storage::Files::new(&directory.path().join("forward-only"));
    let bytes = fixture::source(1, false);
    import_gts_events(&bytes)
        .expect("the existing explicit forward-only reader still accepts this input");
    let mut indexed = indexed_files(bytes, &mut files);
    let failure = import_gts_indexed_term(
        &mut indexed,
        TermKey {
            segment: 0,
            term: 0,
        },
    )
    .expect_err("unindexed input cannot silently select the full reader");
    assert!(source_chain_has(&failure, |cause| matches!(
        cause.downcast_ref::<IndexedError>(),
        Some(IndexedError::Catalog(CatalogError::IndexUnavailable(0)))
    )));
}

#[test]
fn indexed_rdf_keeps_missing_corrupt_dependency_and_warm_diagnostic_refusals() {
    use purrdf_gts::reader::indexed::IndexedError;
    let directory = purrdf_testkit::temp_dir!().expect("indexed error ownership");
    let mut writer = Writer::new("rdf12");
    writer.add_terms(&[Term::iri("urn:datatype")]);
    writer.add_terms(&[Term {
        kind: purrdf_gts::model::TermKind::Literal,
        value: Some("typed".into()),
        datatype: Some(0),
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    }]);
    writer.add_index();
    let bytes = writer.into_bytes();
    let missing = TermKey {
        segment: 0,
        term: 999,
    };
    let mut files = storage::Files::new(&directory.path().join("missing"));
    let mut indexed = indexed_files(bytes.clone(), &mut files);
    let failure = import_gts_indexed_term(&mut indexed, missing)
        .expect_err("missing original term cannot resolve partially");
    assert!(source_chain_has(
        &failure,
        |cause| matches!(cause.downcast_ref::<IndexedError>(), Some(IndexedError::Dictionary(DictionaryError::Missing(key))) if *key == missing)
    ));
    drop(indexed);
    let mut corrupt = bytes;
    let offset = corrupt
        .windows(b"urn:datatype".len())
        .position(|window| window == b"urn:datatype")
        .expect("dependency lexical bytes");
    corrupt[offset] = b'x';
    let mut files = storage::Files::new(&directory.path().join("corrupt-dependency"));
    let mut indexed = indexed_files(corrupt, &mut files);
    let failure = import_gts_indexed_term(
        &mut indexed,
        TermKey {
            segment: 0,
            term: 1,
        },
    )
    .expect_err("selected literal cannot hide corrupt datatype frame");
    assert!(source_chain_has(&failure, |cause| {
        match cause.downcast_ref::<IndexedError>() {
            Some(IndexedError::Native(native)) => {
                purrdf_testkit::error_capture::Capture::read(&***native).code
                    == "gts-frame-self-hash"
            }
            _ => false,
        }
    }));
    drop(indexed);
    let mut writer = Writer::new("rdf12");
    writer.add_terms(&[Term::lang_literal("broken", "invalid_tag")]);
    writer.add_index();
    let bytes = writer.into_bytes();
    let expected = import_gts_events(&bytes)
        .expect_err("original importer rejects the same lexical diagnostic");
    let mut files = storage::Files::new(&directory.path().join("warm-diagnostic"));
    let mut indexed = indexed_files(bytes, &mut files);
    for _ in 0..2 {
        let failure = import_gts_indexed_term(
            &mut indexed,
            TermKey {
                segment: 0,
                term: 0,
            },
        )
        .expect_err("warm raw descriptor cannot erase original RDF fold refusal");
        assert_eq!(failure.code, expected.code);
    }
}

#[test]
fn explicit_reference_preparation_keeps_unrelated_reifier_terms_out_of_raw_admission() {
    use purrdf_gts::reader::indexed::IndexedError;
    let directory = purrdf_testkit::temp_dir!().expect("independent reifier source blocks");
    let mut writer = Writer::new("rdf12");
    writer.add_terms(&[
        Term::iri("urn:s"),
        Term::iri("urn:p"),
        Term::iri("urn:o"),
        Term::iri("urn:r1"),
        Term {
            kind: purrdf_gts::model::TermKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(3),
            triple: None,
        },
    ]);
    writer.add_terms(&[
        Term::iri("urn:other-s"),
        Term::iri("urn:other-p"),
        Term::lang_literal("unrelated lexical defect", "invalid_tag"),
        Term::iri("urn:r2"),
    ]);
    writer.add_reifies(&[(3, (0, 1, 2), None)]);
    writer.add_reifies(&[(8, (5, 6, 7), None)]);
    writer.add_index();
    let bytes = writer.into_bytes();
    assert!(
        import_gts_events(&bytes).is_err(),
        "full import owns the unrelated malformed term too"
    );
    let mut files = storage::Files::new(directory.path());
    let mut indexed = indexed_files(bytes, &mut files);
    let key = TermKey {
        segment: 0,
        term: 4,
    };
    let failure = import_gts_indexed_term(&mut indexed, key)
        .expect_err("indirect lookup must not silently prepare all references");
    assert!(source_chain_has(&failure, |cause| matches!(
        cause.downcast_ref::<IndexedError>(),
        Some(IndexedError::ReferencePreparationRequired(0))
    )));
    indexed
        .prepare_reference_access(0)
        .expect("caller explicitly authenticates current-wire reference contributors");
    let prepared = indexed.work();
    indexed
        .prepare_reference_access(0)
        .expect("same-bound preparation proof is reusable");
    assert_eq!(indexed.work().verified_frames, prepared.verified_frames);
    assert_eq!(indexed.work().decoded_payloads, prepared.decoded_payloads);
    let (owner, id) = import_gts_indexed_term(&mut indexed, key)
        .expect("independent R2 row requires metadata, not unrelated raw terms");
    assert_eq!(
        owner.term_count(),
        4,
        "only selected triple and its three IRI children are resolved"
    );
    assert!(matches!(
        owner.resolve(id),
        purrdf_rdf::TermRef::Triple { .. }
    ));
    assert!(
        indexed.work().decoded_payloads - prepared.decoded_payloads <= 2,
        "only binding payloads may be decoded after already-admitted R1 raw terms; R2 term block stays encoded"
    );
    let cold = indexed.work();
    let (warm_owner, warm_id) = import_gts_indexed_term(&mut indexed, key)
        .expect("same-source prepared warm indirect lookup");
    assert_eq!(warm_owner.term_value(warm_id), owner.term_value(id));
    assert_eq!(indexed.work().read_bytes, cold.read_bytes);
    assert_eq!(indexed.work().decoded_payloads, cold.decoded_payloads);
    drop(indexed);
    assert!(
        files
            .get(TermKey {
                segment: 0,
                term: 5
            })
            .expect("inspect unrelated raw key")
            .is_none()
    );
    assert!(
        files
            .get(TermKey {
                segment: 0,
                term: 7
            })
            .expect("inspect unrelated malformed raw key")
            .is_none()
    );
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
    // Byte-exact stored output shares the resident RDF language identity fold.
    let mut writer = Writer::new("rdf12");
    let mut directional = Term::lang_literal("directional", "FR");
    directional.direction = Some("rtl".to_owned());
    writer.add_terms(&[
        Term::iri("https://example.org/s"),
        Term::iri("https://example.org/p"),
        Term::lang_literal("ordinary", "FR"),
        directional,
    ]);
    writer.add_quads(&[(0, 1, 2, None), (0, 1, 3, None)]);
    let bytes = writer.into_bytes();
    let resident = import_gts_events(&bytes).unwrap();
    let mut files = storage::Files::new(&root.join("language-fold"));
    let replay = files.replay();
    let mut state = storage::Resolution::new(&root.join("language-fold-state"));
    let mut output = WriterDrain(Vec::new());
    let mut ceiling = Ceiling(128 * 1024);
    let mut memory = Memory::new(&mut ceiling);
    export_gts_rdf_nquads_with_dictionary(
        &bytes,
        &mut files,
        &mut state,
        replay,
        &mut output,
        &mut memory,
    )
    .unwrap();
    assert_eq!(output.0, b"<https://example.org/s> <https://example.org/p> \"ordinary\"@fr .\n<https://example.org/s> <https://example.org/p> \"directional\"@fr--rtl .\n");
    let reparsed = parse_dataset(&output.0, "application/n-quads", None).unwrap();
    assert_eq!(render(&reparsed), render(&resident.dataset));
    assert_eq!(
        memory.admitted_bytes(),
        0,
        "folded language scratch is refunded after destruction"
    );
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
    // Frame-admission neighbours use their own source namespace, independent
    // of the deliberately corrupted record exercised above.
    let mut files = storage::Files::new(&root.join("frame-limits"));
    files.limits.encoded_frame_bytes = 16;
    let bytes = fixture::source(1, false);
    let replay = files.replay();
    let mut resolution = storage::Resolution::new(&root.join("resolution"));
    let error = import_gts_events_with_dictionary(&bytes, &mut files, &mut resolution, replay)
        .expect_err("the encoded source window must refuse its own ceiling");
    assert!(source_chain_has(
        &error,
        |cause| matches!(cause.downcast_ref::<DictionaryError>(), Some(DictionaryError::Limit { resource: "encoded-frame-bytes", limit: 16, required }) if *required > 16)
    ));
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

    // Destination reuse is permitted only for the exact admitted source prefix.
    for mode in [
        "matching",
        "term",
        "binding",
        "extra-binding",
        "lost-term",
        "lost-binding",
    ] {
        let term = Term::iri("https://example.org/prefix");
        let mut resident = storage::Resident::default();
        resident.put(first, &term).unwrap();
        if mode != "extra-binding" {
            resident.bind_first(first, (0, 0, 0)).unwrap();
        }
        let bytes = resident.retained_bytes();
        let mut external = storage::Files::new(&root.join(format!("prefix-{mode}")));
        if !mode.starts_with("lost-") {
            external
                .put(
                    first,
                    &if mode == "term" {
                        Term::iri("https://example.org/other")
                    } else {
                        term.clone()
                    },
                )
                .unwrap();
            external
                .bind_first(
                    first,
                    if mode == "binding" {
                        (1, 1, 1)
                    } else {
                        (0, 0, 0)
                    },
                )
                .unwrap();
        }
        external.discard_writes = match mode {
            "lost-term" => Some("term"),
            "lost-binding" => Some("binding"),
            _ => None,
        };
        let mut coordinator = SpillingDictionary::new(resident, external, SpillPolicy::AtBytes(0));
        let result = coordinator.get(first);
        if mode == "matching" {
            assert_eq!(result.unwrap().unwrap().value, term.value);
            assert_eq!(coordinator.transitions(), 1);
        } else {
            assert!(
                matches!(
                    result,
                    Err(DictionaryError::Corrupt(_) | DictionaryError::Missing(_))
                ),
                "{mode}"
            );
            assert_eq!(coordinator.residence(), DictionaryResidence::Failed);
            assert_eq!(
                coordinator.retained_resident_bytes(),
                bytes,
                "{mode}: no refund before verified commit"
            );
            assert!(coordinator.commit().is_err());
        }
        let (mut resident, external) = coordinator.into_parts();
        assert_eq!(external.commits, usize::from(mode == "matching"), "{mode}");
        if mode != "matching" {
            assert_eq!(resident.get(first).unwrap().unwrap().value, term.value);
        }
    }
    // A newly introduced source key cannot import an unrelated future binding,
    // even when its raw term matches and insertion would trigger migration.
    for future_binding in [false, true] {
        let mut resident = storage::Resident::default();
        resident
            .put(first, &Term::iri("https://example.org/first"))
            .unwrap();
        let retained = resident.retained_bytes();
        let next = TermKey {
            segment: 0,
            term: 1,
        };
        let term = Term::literal("x".repeat(2048), None);
        let mut external = storage::Files::new(&root.join(format!("future-{future_binding}")));
        external.put(next, &term).unwrap();
        if future_binding {
            external.bind_first(next, (1, 1, 1)).unwrap();
        }
        let mut coordinator =
            SpillingDictionary::new(resident, external, SpillPolicy::AtBytes(retained + 1));
        let result = coordinator.introduce(next, &term);
        if future_binding {
            assert!(matches!(result, Err(DictionaryError::Corrupt(_))));
            assert_eq!(coordinator.residence(), DictionaryResidence::Failed);
            assert_eq!(coordinator.retained_resident_bytes(), retained);
        } else {
            assert_eq!(result.unwrap().value, term.value);
        }
        let (_, external) = coordinator.into_parts();
        assert_eq!(external.commits, usize::from(!future_binding));
    }
    // Generic idempotent persistence remains distinct from fresh introduction.
    let mut generic = SpillingDictionary::new(
        storage::Resident::default(),
        storage::Files::new(&root.join("idempotent")),
        SpillPolicy::External,
    );
    let term = Term::iri("https://example.org/same");
    generic.put(first, &term).unwrap();
    generic.bind_first(first, (0, 0, 0)).unwrap();
    generic.put(first, &term).unwrap();
    generic.bind_first(first, (1, 1, 1)).unwrap();
    assert_eq!(generic.first_binding(first).unwrap(), Some((0, 0, 0)));
    assert!(matches!(
        generic.introduce(first, &term),
        Err(DictionaryError::Corrupt(_))
    ));
    assert_eq!(generic.residence(), DictionaryResidence::Failed);
    let (_, mut reopened) = generic.into_parts();
    assert_eq!(reopened.get(first).unwrap().unwrap().value, term.value);
    assert_eq!(reopened.first_binding(first).unwrap(), Some((0, 0, 0)));
    // An initially over-limit prefix must not commit before detecting a
    // conflicting incoming key in the destination.
    let mut resident = storage::Resident::default();
    resident.put(first, &term).unwrap();
    let next = TermKey {
        segment: 0,
        term: 1,
    };
    let mut external = storage::Files::new(&root.join("initial-conflict"));
    external
        .put(next, &Term::iri("https://example.org/old"))
        .unwrap();
    let mut coordinator = SpillingDictionary::new(resident, external, SpillPolicy::AtBytes(0));
    assert!(matches!(
        coordinator.put(next, &Term::iri("https://example.org/new")),
        Err(DictionaryError::Corrupt(_))
    ));
    let (mut resident, external) = coordinator.into_parts();
    assert!(resident.get(first).unwrap().is_some());
    assert_eq!(external.commits, 0);
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
    for (malformed, invalid_subject) in [(false, false), (true, false), (false, true)] {
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
                Term::triple_term(if invalid_subject {
                    (4, 0, 3)
                } else {
                    (1, 0, 4)
                }),
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
        check_event_order(
            &bytes,
            &root.join(format!("events-{malformed}-{invalid_subject}")),
        );
        let mut raw = storage::Files::new(&root.join(format!("raw-{malformed}-{invalid_subject}")));
        let replay = raw.replay();
        let mut resolution =
            storage::Resolution::new(&root.join(format!("state-{malformed}-{invalid_subject}")));
        let mut output = WriterDrain(Vec::new());
        let mut ceiling = Ceiling(128 * 1024);
        let mut memory = Memory::new(&mut ceiling);
        let stored = export_gts_rdf_nquads_with_dictionary(
            &bytes,
            &mut raw,
            &mut resolution,
            replay,
            &mut output,
            &mut memory,
        );
        if invalid_subject || malformed {
            let resident = resident.expect_err("native triple-component position refusal");
            let stored = stored.expect_err("streamed triple-component position refusal");
            if invalid_subject {
                assert_eq!(resident.code, "rdf-ir-triple-subject");
            }
            assert_eq!(stored.code, resident.code);
            let retained = stored
                .retained_bytes()
                .expect("native retained diagnostic layout");
            assert_eq!(
                memory.admitted_bytes(),
                retained,
                "only the returned original-account diagnostic remains charged"
            );
            drop(stored);
            memory
                .release_bytes(retained)
                .expect("refund after diagnostic destruction");
            assert_eq!(memory.admitted_bytes(), 0);
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
