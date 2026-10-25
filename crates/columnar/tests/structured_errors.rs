// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Physical Parquet reader refusals through the portable public API.

use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Weak};

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_columnar::{
    ColumnarError, ColumnarWriteError, Compression, ParquetFiles, ValueBodyReason, project, read,
    write,
};
use purrdf_core::{
    ContentDigest, ContentStore, ContentStoreError, DatasetView, GraphMatch, QuadIds, RdfDataset,
    RdfDatasetBuilder, RdfStoreCapabilities, TermGuard, TermId, TermRef, TermValue,
};
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};
use purrdf_testkit::error_capture::Capture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Retain physical bounds after input disposal, including full-width counts.
fn original_parquet_bounds_survive_file_disposal_without_narrowing() {
    let files = ParquetFiles::from_array(std::array::from_fn(|_| Vec::new()));
    let error = read(&files).unwrap_err();
    drop(files);
    let copy = error.clone();
    drop(error);
    let fields = Capture::read(&copy);
    assert_eq!(fields.code, "columnar-truncated");
    assert!(matches!(
        fields.field("context"),
        ErrorValue::Code("Parquet file")
    ));
    assert!(matches!(fields.field("needed"), ErrorValue::ByteCount(12)));
    assert!(matches!(
        fields.field("available"),
        ErrorValue::ByteCount(0)
    ));
    let error = ColumnarError::LimitExceeded {
        context: "original wire count",
        value: u64::MAX,
        maximum: 7,
    };
    let fields = Capture::read(&error);
    assert!(
        matches!(fields.field("value"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(matches!(fields.field("maximum"), ErrorValue::Unsigned(7)));
}

#[derive(Debug, Default)]
struct Counts {
    clones: AtomicUsize,
    drops: AtomicUsize,
    pins: AtomicUsize,
    released: AtomicUsize,
}

#[derive(Debug)]
struct Owner {
    identity: u128,
    counts: Arc<Counts>,
}

#[derive(Debug)]
enum Refusal {
    Read {
        phase: &'static str,
        observed: u128,
        owner: Arc<Owner>,
    },
}

impl Clone for Refusal {
    /// Count provider-issued clones separately from error transport.
    fn clone(&self) -> Self {
        let Self::Read {
            phase,
            observed,
            owner,
        } = self;
        owner.counts.clones.fetch_add(1, Ordering::Relaxed);
        Self::Read {
            phase,
            observed: *observed,
            owner: Arc::clone(owner),
        }
    }
}

impl Drop for Refusal {
    /// Count the release of each provider or returned error owner.
    fn drop(&mut self) {
        let Self::Read { owner, .. } = self;
        owner.counts.drops.fetch_add(1, Ordering::Relaxed);
    }
}

impl std::fmt::Display for Refusal {
    /// Reject any attempt to replace the original structured cause with text.
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        panic!("source transport and inspection must not render this refusal")
    }
}

impl std::error::Error for Refusal {}

impl StructuredError for Refusal {
    purrdf_hash::error_visit! {
        Read { phase, observed, owner } => "columnar-test-read" {
            "phase" => Code(phase),
            "observed" => ByteCount(*observed),
            "identity" => Unsigned(owner.identity),
        },
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Initial,
    Completed,
    Resolution,
    UnlatchedResolution,
    NestedResolution,
    CompletedWithCodec,
    ProjectionDatatype,
}

impl Phase {
    /// The original source's stable reason, retained without presentation.
    const fn code(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Completed => "completed",
            Self::Resolution => "resolution",
            Self::UnlatchedResolution => "unlatched-resolution",
            Self::NestedResolution => "nested-resolution",
            Self::CompletedWithCodec => "completed-with-codec",
            Self::ProjectionDatatype => "projection-datatype",
        }
    }
}

struct FaultView {
    inner: Arc<RdfDataset>,
    phase: Phase,
    failed: Cell<bool>,
    refusal: Refusal,
    returned: Cell<usize>,
    reads: Cell<usize>,
    resolutions: Cell<usize>,
    counts: Arc<Counts>,
}

struct Pin<'a> {
    value: TermRef<'a>,
    counts: &'a Counts,
}

impl TermGuard<TermId> for Pin<'_> {
    /// Lend the original term under this pin's lifetime.
    fn term(&self) -> TermRef<'_> {
        self.value
    }
}

impl Drop for Pin<'_> {
    /// Record that the resolver released its original source pin.
    fn drop(&mut self) {
        assert_ne!(self.counts.pins.fetch_sub(1, Ordering::Relaxed), 0);
        self.counts.released.fetch_add(1, Ordering::Relaxed);
    }
}

impl FaultView {
    /// Return the provider's own clone, counted separately from error transport.
    fn failure(&self) -> Refusal {
        self.returned.set(self.returned.get() + 1);
        self.refusal.clone()
    }
}

impl DatasetView for FaultView {
    type Id = TermId;
    type ReadError = Refusal;
    type TermGuard<'a>
        = Pin<'a>
    where
        Self: 'a;
    type ProbePlan = <RdfDataset as DatasetView>::ProbePlan;

    /// Report the provider's sticky failure without releasing its owner.
    fn read_error(&self) -> Option<Self::ReadError> {
        self.failed.get().then(|| self.failure())
    }

    /// Trip completed-read cases while yielding the original native rows.
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.reads.set(self.reads.get() + 1);
        if matches!(self.phase, Phase::Completed | Phase::CompletedWithCodec) {
            self.failed.set(true);
        }
        self.inner.quads()
    }

    /// Pin a native term or refuse at the selected original resolution depth.
    fn resolve(&self, id: TermId) -> Result<Self::TermGuard<'_>, Self::ReadError> {
        self.resolutions.set(self.resolutions.get() + 1);
        let value = match self.phase {
            Phase::Resolution | Phase::UnlatchedResolution | Phase::NestedResolution
                if self.phase != Phase::NestedResolution || self.resolutions.get() == 3 =>
            {
                if self.phase != Phase::UnlatchedResolution {
                    self.failed.set(true);
                }
                return Err(self.failure());
            }
            // The completed-read refusal must still precede an inner codec
            // refusal: this literal's datatype deliberately refers to itself.
            Phase::CompletedWithCodec => TermRef::Literal {
                lexical: "original input",
                datatype: id,
                language: None,
                direction: None,
            },
            Phase::ProjectionDatatype
                if self.inner.term_id_by_iri("https://example.org/source") == Some(id) =>
            {
                TermRef::Literal {
                    lexical: "original input",
                    datatype: self
                        .inner
                        .term_id_by_blank("original datatype", purrdf_core::BlankScope(27))
                        .unwrap(),
                    language: None,
                    direction: None,
                }
            }
            _ => self.inner.resolve(id).map_err(|never| match never {})?,
        };
        self.counts.pins.fetch_add(1, Ordering::Relaxed);
        Ok(Pin {
            value,
            counts: &self.counts,
        })
    }

    /// Delegate identity lookup to the source's native dictionary.
    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, Self::ReadError> {
        self.inner
            .term_id_by_value(value)
            .map_err(|never| match never {})
    }

    /// Preserve the original backend capabilities.
    fn capabilities(&self) -> RdfStoreCapabilities {
        self.inner.capabilities()
    }

    /// Reuse the original native probe planner.
    fn probe_plan(&self, s: bool, p: bool, o: bool, g: GraphMatch) -> Self::ProbePlan {
        self.inner.probe_plan(s, p, o, g)
    }

    /// Execute the native plan without remapping source rows or ids.
    fn quads_for_pattern_with_plan(
        &self,
        plan: &Self::ProbePlan,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.quads_for_pattern_with_plan(plan, s, p, o, g)
    }

    /// Retain the original source dictionary cardinality.
    fn term_count(&self) -> u64 {
        self.inner.term_count()
    }
}

/// Build a real native source, injecting only the selected operational refusal.
fn fault_view(phase: Phase) -> (FaultView, Weak<Owner>, Arc<Counts>) {
    let counts = Arc::new(Counts::default());
    let owner = Arc::new(Owner {
        identity: u128::MAX - 91,
        counts: Arc::clone(&counts),
    });
    let weak = Arc::downgrade(&owner);
    let mut builder = RdfDatasetBuilder::new();
    if matches!(
        phase,
        Phase::Resolution
            | Phase::UnlatchedResolution
            | Phase::NestedResolution
            | Phase::CompletedWithCodec
            | Phase::ProjectionDatatype
    ) {
        let term = builder.intern_iri("https://example.org/source").unwrap();
        let object = if phase == Phase::NestedResolution {
            let leaf = builder.intern_iri("https://example.org/dependent").unwrap();
            builder.intern_triple(term, term, leaf).unwrap()
        } else if phase == Phase::ProjectionDatatype {
            builder
                .intern_blank("original datatype", purrdf_core::BlankScope(27))
                .unwrap()
        } else {
            term
        };
        builder.push_quad(term, term, object, None).unwrap();
    }
    (
        FaultView {
            inner: builder.freeze().unwrap(),
            phase,
            failed: Cell::new(phase == Phase::Initial),
            refusal: Refusal::Read {
                phase: phase.code(),
                observed: u128::MAX - 17,
                owner,
            },
            returned: Cell::new(0),
            reads: Cell::new(0),
            resolutions: Cell::new(0),
            counts: Arc::clone(&counts),
        },
        weak,
        counts,
    )
}

#[derive(Default)]
struct Inspect<'error> {
    variants: [Option<&'static str>; 2],
    count: usize,
    phase: Option<&'error str>,
    observed: Option<u128>,
    identity: Option<u128>,
}

impl<'error> ErrorVisitor<'error> for Inspect<'error> {
    /// Retain each stable variant identity without allocating.
    fn variant(&mut self, code: &'static str) {
        self.variants[self.count] = Some(code);
        self.count += 1;
    }

    /// Borrow primitives and recursively inspect the original stable cause.
    fn value(&mut self, name: Option<&'static str>, value: ErrorValue<'error>) {
        match (name, value) {
            (Some("cause"), ErrorValue::Cause(cause)) => cause.visit(self),
            (Some("phase"), ErrorValue::Code(phase)) => self.phase = Some(phase),
            (Some("observed"), ErrorValue::ByteCount(observed)) => self.observed = Some(observed),
            (Some("identity"), ErrorValue::Unsigned(identity)) => self.identity = Some(identity),
            _ => panic!("unexpected source field or primitive kind"),
        }
    }

    /// Refuse a list substituted for the source's primitive fields.
    fn begin_list(&mut self, _: Option<&'static str>, _: usize) {
        panic!("source fields are not a list")
    }

    /// Refuse an unexpected list terminator.
    fn end_list(&mut self) {
        panic!("source fields are not a list")
    }

    /// Require the original cause rather than a temporary compound adapter.
    fn begin_record(&mut self, _: Option<&'static str>, _: &'static str) {
        panic!("the original source is a stable borrowed cause")
    }

    /// Refuse an unexpected temporary-record terminator.
    fn end_record(&mut self) {
        panic!("the original source is a stable borrowed cause")
    }
}

/// Exercise original failure precedence, pin release and error-owner lifetime.
fn public_projection_and_write_preserve_source_owners_and_fields() {
    for phase in [
        Phase::Initial,
        Phase::Completed,
        Phase::Resolution,
        Phase::UnlatchedResolution,
        Phase::NestedResolution,
        Phase::CompletedWithCodec,
    ] {
        for compression in [
            None,
            Some(Compression::Uncompressed),
            Some(Compression::Zstd),
        ] {
            let (view, weak, counts) = fault_view(phase);
            let blobs = ContentStore::new();
            let error = match compression {
                None => project(&view, &blobs).unwrap_err(),
                Some(compression) => write(&view, &blobs, compression).unwrap_err(),
            };
            let native_returns = view.returned.get();
            assert_eq!(
                native_returns,
                if matches!(phase, Phase::Resolution | Phase::NestedResolution) {
                    2
                } else {
                    1
                }
            );
            assert_eq!(counts.clones.load(Ordering::Relaxed), native_returns);
            if phase == Phase::Initial {
                assert_eq!(view.reads.get(), 0);
            }
            assert_eq!(
                view.resolutions.get(),
                match phase {
                    Phase::NestedResolution => 3,
                    Phase::Resolution | Phase::UnlatchedResolution | Phase::CompletedWithCodec => 1,
                    Phase::Initial | Phase::Completed | Phase::ProjectionDatatype => 0,
                }
            );
            assert_eq!(counts.pins.load(Ordering::Relaxed), 0);
            assert_eq!(
                counts.released.load(Ordering::Relaxed),
                match phase {
                    Phase::NestedResolution => 2,
                    Phase::CompletedWithCodec => 1,
                    Phase::Initial
                    | Phase::Completed
                    | Phase::Resolution
                    | Phase::UnlatchedResolution => 0,
                    Phase::ProjectionDatatype => 0,
                }
            );
            drop(view);
            assert_eq!(weak.strong_count(), 1);

            let ColumnarWriteError::SourceRead(cause) = &error else {
                panic!("the original operational refusal must precede any codec refusal");
            };
            let source = std::error::Error::source(&error)
                .unwrap()
                .downcast_ref::<Refusal>()
                .unwrap();
            assert!(std::ptr::eq(source, cause));

            let mut inspected = Inspect::default();
            let window = CurrentThreadWindow::open();
            error.visit(&mut inspected);
            let measured = window.close();
            assert_eq!(measured.allocations, 0);
            assert_eq!(measured.requested_bytes, 0);
            assert_eq!(measured.retained_bytes, 0);
            assert_eq!(measured.peak_working_bytes, 0);
            assert_eq!(
                inspected.variants,
                [Some("columnar-source-read"), Some("columnar-test-read")]
            );
            assert_eq!(inspected.phase, Some(phase.code()));
            assert_eq!(inspected.observed, Some(u128::MAX - 17));
            assert_eq!(inspected.identity, Some(u128::MAX - 91));
            assert_eq!(counts.clones.load(Ordering::Relaxed), native_returns);

            let captured = Capture::read(&error);
            assert_eq!(captured.child("cause").code, "columnar-test-read");
            drop(captured);
            drop(error);
            assert_eq!(weak.strong_count(), 0);
            assert_eq!(counts.drops.load(Ordering::Relaxed), native_returns + 1);
        }
    }

    for compression in [
        None,
        Some(Compression::Uncompressed),
        Some(Compression::Zstd),
    ] {
        let (view, owner, counts) = fault_view(Phase::ProjectionDatatype);
        let blobs = ContentStore::new();
        // The unchanged native source is valid; only the borrowed adapter's
        // datatype view is malformed, and no operational read error masks it.
        project(&*view.inner, &blobs).unwrap();
        let error = match compression {
            None => project(&view, &blobs).unwrap_err(),
            Some(compression) => write(&view, &blobs, compression).unwrap_err(),
        };
        assert_eq!(counts.pins.load(Ordering::Relaxed), 0);
        assert_eq!(counts.clones.load(Ordering::Relaxed), 0);
        drop(view);
        assert_eq!(owner.strong_count(), 0);
        let ColumnarWriteError::Codec(codec) = &error else {
            panic!("a pure grammar refusal must retain its codec reason");
        };
        inspect_without_allocation(codec);
        assert_eq!(
            error.to_string(),
            "columnar literal datatype is malformed: datatype id does not resolve to an IRI"
        );
        let reason = Capture::read(codec).child("reason");
        assert_eq!(reason.code, "columnar-projection-datatype-not-iri");
        let actual = reason.child("actual");
        assert_eq!(actual.code, "rdf-term-blank");
        assert!(matches!(
            actual.field("label"),
            ErrorValue::Text("original datatype")
        ));
        assert!(matches!(actual.field("scope"), ErrorValue::Unsigned(27)));
        let original = std::error::Error::source(codec)
            .unwrap()
            .downcast_ref::<purrdf_columnar::ProjectionReason>()
            .unwrap();
        let ColumnarError::Projection { reason, .. } = codec else {
            panic!("the codec exposes its native projection reason");
        };
        assert!(std::ptr::eq(original, reason));
    }
}

/// Preserve native codec messages and the human-selected source prefix.
fn human_presentation_preserves_codec_and_source_messages() {
    let codec = ColumnarError::Truncated {
        context: "original table",
        needed: 19,
        available: 7,
    };
    let expected = codec.to_string();
    let wrapped = ColumnarWriteError::<std::fmt::Error>::from(codec);
    assert_eq!(wrapped.to_string(), expected);
    let source = ColumnarWriteError::SourceRead(std::fmt::Error);
    assert_eq!(
        source.to_string(),
        format!("columnar source read failed: {}", std::fmt::Error)
    );
}

/// Replace one uncompressed data value while retaining every physical boundary.
fn replace_unique(bytes: &mut [u8], original: &[u8], replacement: &[u8]) {
    assert_eq!(original.len(), replacement.len());
    let positions: Vec<_> = bytes
        .windows(original.len())
        .enumerate()
        .filter_map(|(index, value)| (value == original).then_some(index))
        .collect();
    assert_eq!(positions.len(), 1, "fixture must identify one data value");
    let start = positions[0];
    bytes[start..start + original.len()].copy_from_slice(replacement);
}

#[derive(Default)]
struct NativeInspection {
    fields: usize,
    records: usize,
    bytes: usize,
}

impl<'error> ErrorVisitor<'error> for NativeInspection {
    /// Count original identities throughout the complete borrowed tree.
    fn variant(&mut self, _: &'static str) {
        self.records += 1;
    }
    /// Follow native causes and digest records without rendering or allocation.
    fn value(&mut self, _: Option<&'static str>, value: ErrorValue<'error>) {
        self.fields += 1;
        match value {
            ErrorValue::Cause(value) | ErrorValue::Record(value) | ErrorValue::Term(value) => {
                value.visit(self);
            }
            ErrorValue::Bytes(bytes) => self.bytes += bytes.len(),
            _ => {}
        }
    }
    /// Lists retain their original entries in the same borrowed traversal.
    fn begin_list(&mut self, _: Option<&'static str>, _: usize) {}
    /// End the current borrowed list.
    fn end_list(&mut self) {}
    /// Count compound views supplied directly by the native producer.
    fn begin_record(&mut self, _: Option<&'static str>, _: &'static str) {
        self.records += 1;
    }
    /// End the current native compound view.
    fn end_record(&mut self) {}
}

/// Inspect every nested native field with all four allocation quantities zero.
fn inspect_without_allocation(error: &ColumnarError) -> NativeInspection {
    let mut visitor = NativeInspection::default();
    let window = CurrentThreadWindow::open();
    error.visit(&mut visitor);
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
    assert_eq!(measured.retained_bytes, 0);
    assert_eq!(measured.peak_working_bytes, 0);
    assert!(visitor.fields > 0);
    visitor
}

/// Read an original SHA-256 identity through the root digest's byte record.
fn assert_content_digest(record: &Capture<'_>, expected: ContentDigest) {
    assert_eq!(record.code, "content-digest-sha256");
    let digest = record.child("digest");
    assert_eq!(digest.code, "digest32");
    assert!(
        matches!(digest.field("bytes"), ErrorValue::Bytes(bytes) if bytes == expected.as_bytes())
    );
}

/// Actual blob verification keeps both identities and the nonzero row after disposal.
fn public_blob_read_preserves_original_integrity_cause() {
    let first = b"original first payload";
    let second = b"original other payload";
    let mut blobs = ContentStore::new();
    let a = blobs.insert(first.to_vec());
    let b = blobs.insert(second.to_vec());
    let (stored, payload) = if a > b { (a, first) } else { (b, second) };
    let mut changed = payload.to_vec();
    changed[0] ^= 1;
    let mut array = write(
        &RdfDatasetBuilder::new().freeze().unwrap(),
        &blobs,
        Compression::Uncompressed,
    )
    .unwrap()
    .files
    .into_array();
    replace_unique(&mut array[4], payload, &changed);
    let files = ParquetFiles::from_array(array);
    let error = read(&files).unwrap_err();
    drop(files);
    drop(blobs);
    let ColumnarError::BlobContent { row, cause } = &error else {
        panic!("expected original blob cause: {error}");
    };
    assert_eq!(*row, 1);
    assert_eq!(
        error.to_string(),
        format!("columnar blobs.bytes is malformed: row 1: {cause}")
    );
    assert!(std::ptr::eq(
        std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<ContentStoreError>()
            .unwrap(),
        cause
    ));
    assert_eq!(inspect_without_allocation(&error).bytes, 64);
    let captured = Capture::read(&error);
    assert!(matches!(captured.field("row"), ErrorValue::Unsigned(1)));
    let cause = captured.child("cause");
    assert_eq!(cause.code, "content-digest-mismatch");
    assert_content_digest(&cause.child("stored"), stored);
    assert_content_digest(&cause.child("actual"), ContentDigest::of(&changed));

    // The public reader returns rejected decoded digest cells, independently of
    // the original files and without rendering or copying a diagnostic string.
    let mut blobs = ContentStore::new();
    let digest = blobs.insert(first.to_vec()).to_hex();
    for (replacement, code, message) in [
        (vec![0xff; 64], "columnar-digest-utf8", "row 0 is not UTF-8"),
        (
            vec![b'g'; 64],
            "columnar-digest-shape",
            "row 0 is not a 64-digit SHA-256 hex value",
        ),
        (
            digest.to_ascii_uppercase().into_bytes(),
            "columnar-digest-case",
            "row 0 is not canonical lowercase hexadecimal",
        ),
    ] {
        let mut array = write(
            &RdfDatasetBuilder::new().freeze().unwrap(),
            &blobs,
            Compression::Uncompressed,
        )
        .unwrap()
        .files
        .into_array();
        replace_unique(&mut array[4], digest.as_bytes(), &replacement);
        let files = ParquetFiles::from_array(array);
        let error = read(&files).unwrap_err();
        drop(files);
        assert_eq!(inspect_without_allocation(&error).bytes, 64);
        let reason = Capture::read(&error).child("reason");
        assert_eq!(reason.code, code);
        assert!(matches!(reason.field("row"), ErrorValue::Unsigned(0)));
        assert!(matches!(reason.field("bytes"), ErrorValue::Bytes(bytes) if bytes == replacement));
        assert_eq!(
            error.to_string(),
            format!("columnar blobs.digest is malformed: {message}")
        );
        if code == "columnar-digest-utf8" {
            let cause = reason.child("cause");
            assert_eq!(cause.code, "std-utf8");
            assert!(matches!(
                cause.field("valid_up_to"),
                ErrorValue::ByteCount(0)
            ));
            assert!(matches!(cause.field("error_len"), ErrorValue::ByteCount(1)));
        }
    }
}

/// Actual kernel validation survives the public reader with its complete diagnostic.
fn public_reconstruction_preserves_original_diagnostic() {
    let mut builder = RdfDatasetBuilder::new();
    let id = builder.intern_iri("urn:original").unwrap();
    builder.push_quad(id, id, id, None).unwrap();
    let mut array = write(
        &builder.freeze().unwrap(),
        &ContentStore::new(),
        Compression::Uncompressed,
    )
    .unwrap()
    .files
    .into_array();
    replace_unique(&mut array[0], b"urn:original", b"0rn:original");
    let files = ParquetFiles::from_array(array);
    let error = read(&files).unwrap_err();
    drop(files);
    let ColumnarError::Reconstruction(cause) = &error else {
        panic!("expected original reconstruction diagnostic: {error}");
    };
    assert!(std::ptr::eq(
        std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<purrdf_core::RdfDiagnostic>()
            .unwrap(),
        cause
    ));
    inspect_without_allocation(&error);
    let mut native = RdfDatasetBuilder::new();
    let id = native.intern_iri("0rn:original").unwrap();
    native.push_quad(id, id, id, None).unwrap();
    let expected = native.freeze().unwrap_err();
    assert_eq!(cause, &expected);
    let captured = Capture::read(&error).child("cause");
    assert_eq!(captured.code, Capture::read(&expected).code);
    assert_eq!(
        error.to_string(),
        format!("columnar RDF reconstruction is malformed: {expected}")
    );

    // Valid native output remains readable before its authored metadata key is
    // corrupted. The native reason must retain that original decoded text.
    let mut array = write(
        &RdfDatasetBuilder::new().freeze().unwrap(),
        &ContentStore::new(),
        Compression::Uncompressed,
    )
    .unwrap()
    .files
    .into_array();
    read(&ParquetFiles::from_array(array.clone())).unwrap();
    replace_unique(
        &mut array[0],
        b"purrdf.columnar.schema-version",
        b"xurrdf.columnar.schema-version",
    );
    let files = ParquetFiles::from_array(array);
    let error = read(&files).unwrap_err();
    drop(files);
    inspect_without_allocation(&error);
    let reason = Capture::read(&error).child("reason");
    assert_eq!(reason.code, "columnar-parquet-metadata");
    let pairs = reason.child("profile").records("observed");
    assert!(pairs.iter().any(|pair| matches!(
        pair.field("key"),
        ErrorValue::Text("xurrdf.columnar.schema-version")
    )));
    let source = std::error::Error::source(&error).unwrap();
    assert!(source.is::<purrdf_columnar::MetadataReason>());

    // Two authored key/value structs carry the same original metadata key.
    let footer = [
        0x59, 0x2c, 0x18, 1, b'k', 0x18, 1, b'a', 0, 0x18, 1, b'k', 0x18, 1, b'b', 0, 0,
    ];
    let files = compact_footer_files(&footer);
    let error = read(&files).unwrap_err();
    drop(files);
    inspect_without_allocation(&error);
    let reason = Capture::read(&error).child("reason");
    assert_eq!(reason.code, "columnar-parquet-duplicate-key");
    assert!(matches!(reason.field("key"), ErrorValue::Text("k")));
    assert_eq!(
        error.to_string(),
        "columnar file key/value metadata is malformed: duplicate key k"
    );
}

/// Native wrappers move causes without allocating and keep diagnostic attachments.
fn native_cause_transport_preserves_storage_and_full_width_rows() {
    let mut store = ContentStore::new();
    let cause = store
        .insert_checked(ContentDigest::from_raw([0xff; 32]), b"original".to_vec())
        .unwrap_err();
    let global_cause = cause.clone();
    let counts = Arc::new(Counts::default());
    let owner = Arc::new(Owner {
        identity: u128::MAX - 91,
        counts: Arc::clone(&counts),
    });
    let weak = Arc::downgrade(&owner);
    let refusal = Refusal::Read {
        phase: "original-attachment",
        observed: u128::MAX - 17,
        owner,
    };
    let mut resident = purrdf_lex::allocation::Resident;
    let mut memory = purrdf_lex::allocation::Memory::new(&mut resident);
    let native: Box<dyn purrdf_hash::error::NativeError> = memory.boxed(refusal).unwrap();
    let native = purrdf_lex::allocation::SharedOwned::try_from_admitted(
        native,
        purrdf_lex::allocation::Resident,
    )
    .unwrap();
    let diagnostic =
        purrdf_core::RdfDiagnostic::from(purrdf_lex::allocation::StorageError::AdmissionFailed)
            .with_detail("original diagnostic detail")
            .with_location(
                purrdf_core::RdfLocation::logical("original location").with_gts_quad(u64::MAX),
            )
            .with_native_cause(native, &mut memory)
            .unwrap();
    let window = CurrentThreadWindow::open();
    let global = ColumnarError::ContentStore(global_cause);
    let row = ColumnarError::BlobContent {
        row: usize::MAX,
        cause,
    };
    let reconstruction = ColumnarError::Reconstruction(diagnostic);
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
    assert_eq!(measured.retained_bytes, 0);
    assert_eq!(measured.peak_working_bytes, 0);
    for error in [&global, &row, &reconstruction] {
        inspect_without_allocation(error);
    }
    let ColumnarError::ContentStore(cause) = &global else {
        unreachable!()
    };
    assert!(std::ptr::eq(
        std::error::Error::source(&global)
            .unwrap()
            .downcast_ref::<ContentStoreError>()
            .unwrap(),
        cause
    ));
    assert_eq!(
        global.to_string(),
        format!("columnar content store is malformed: {cause}")
    );
    assert!(
        matches!(Capture::read(&row).field("row"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
    );
    let ColumnarError::Reconstruction(cause) = &reconstruction else {
        unreachable!()
    };
    assert_eq!(cause.detail.as_deref(), Some("original diagnostic detail"));
    assert_eq!(
        cause.location.as_ref().unwrap().gts_quad_index,
        Some(u64::MAX)
    );
    assert_eq!(
        cause.storage_error(),
        Some(purrdf_lex::allocation::StorageError::AdmissionFailed)
    );
    let captured = Capture::read(&reconstruction)
        .child("cause")
        .child("attachments")
        .child("native_cause");
    assert!(
        matches!(captured.field("observed"), ErrorValue::ByteCount(value) if value == u128::MAX - 17)
    );
    assert!(
        matches!(captured.field("identity"), ErrorValue::Unsigned(value) if value == u128::MAX - 91)
    );
    assert_eq!(weak.strong_count(), 1);
    assert_eq!(counts.clones.load(Ordering::Relaxed), 0);
    assert_eq!(
        reconstruction.to_string(),
        "columnar storage: native workspace admission refused"
    );
    assert!(std::ptr::eq(
        std::error::Error::source(&reconstruction)
            .unwrap()
            .downcast_ref::<purrdf_core::RdfDiagnostic>()
            .unwrap(),
        cause
    ));
    drop(reconstruction);
    assert_eq!(weak.strong_count(), 0);
    assert_eq!(counts.drops.load(Ordering::Relaxed), 1);
}

/// Envelope an authored footer so refusals traverse the actual public reader.
fn compact_footer_files(footer: &[u8]) -> ParquetFiles {
    let mut file = b"PAR1".to_vec();
    file.extend_from_slice(footer);
    file.extend_from_slice(&u32::try_from(footer.len()).unwrap().to_le_bytes());
    file.extend_from_slice(b"PAR1");
    let mut files = std::array::from_fn(|_| Vec::new());
    files[0] = file;
    ParquetFiles::from_array(files)
}

/// Public grammar failures retain their original cause after all bytes are gone.
fn public_compact_causes_survive_input_disposal() {
    use purrdf_core::ir::pack::bits::{write_varint, zigzag_encode};

    let mut wide = vec![0x15]; // version: i32
    write_varint(&mut wide, zigzag_encode(i64::MAX));
    for (footer, code, context) in [
        (wide, "columnar-integer-range", "compact i32"),
        (vec![0x15, 0x80], "columnar-varint", "compact i32"),
        // Schema list, one struct, name field, invalid second UTF-8 byte.
        (
            vec![0x29, 0x1c, 0x48, 2, b'a', 0xff],
            "columnar-utf8",
            "compact string",
        ),
    ] {
        let files = compact_footer_files(&footer);
        let error = read(&files).unwrap_err();
        drop(files);
        drop(footer);
        let copy = error.clone();
        drop(error);
        assert!(inspect_without_allocation(&copy).records >= 3);
        let outer = Capture::read(&copy);
        assert_eq!(outer.code, "columnar-malformed");
        assert!(matches!(outer.field("context"), ErrorValue::Code(value) if value == context));
        let reason = outer.child("reason");
        assert_eq!(reason.code, code);
        let cause = reason.child("cause");
        let source = std::error::Error::source(&copy).unwrap();
        let original = source.source().unwrap();
        match code {
            "columnar-integer-range" => {
                assert!(
                    matches!(reason.field("value"), ErrorValue::Signed(value) if value == i128::from(i64::MAX))
                );
                assert_eq!(cause.code, "integer-conversion-range");
                assert!(original.is::<std::num::TryFromIntError>());
                assert_eq!(
                    copy.to_string(),
                    "columnar compact i32 is malformed: value exceeds i32"
                );
            }
            "columnar-varint" => {
                assert!(matches!(reason.field("offset"), ErrorValue::ByteCount(1)));
                assert_eq!(cause.code, "pack-bits-truncated");
                assert!(matches!(cause.field("needed"), ErrorValue::ByteCount(3)));
                assert!(matches!(cause.field("found"), ErrorValue::ByteCount(2)));
                assert!(original.is::<purrdf_core::ir::pack::bits::PackBitsError>());
            }
            "columnar-utf8" => {
                assert_eq!(cause.code, "std-utf8");
                assert!(matches!(
                    cause.field("valid_up_to"),
                    ErrorValue::ByteCount(1)
                ));
                assert!(matches!(cause.field("error_len"), ErrorValue::ByteCount(1)));
                let utf8 = original.downcast_ref::<std::str::Utf8Error>().unwrap();
                assert_eq!((utf8.valid_up_to(), utf8.error_len()), (1, Some(1)));
                assert_eq!(
                    copy.to_string(),
                    "columnar compact string is malformed: invalid UTF-8"
                );
            }
            _ => unreachable!("the fixture names its retained native cause"),
        }
    }
}

/// Early grammar refusals and nested skips allocate no diagnostic strings.
fn public_compact_first_fault_and_unknown_skips_do_not_allocate() {
    use purrdf_core::ir::pack::bits::write_varint;

    let mut overflow = vec![0x68]; // unknown binary field
    write_varint(&mut overflow, u64::MAX);
    let cases = [
        (vec![0x10, 0x15, 0x80], "columnar-stop-delta"),
        (vec![0x18], "columnar-field-type"),
        (vec![0x69, 0x10], "columnar-stop-element"),
        (vec![0x6b, 1, 0x03], "columnar-stop-value"),
        (
            overflow,
            if usize::BITS == 64 {
                "columnar-byte-range"
            } else {
                "columnar-binary-length"
            },
        ),
    ];
    for (footer, code) in cases {
        let files = compact_footer_files(&footer);
        let window = CurrentThreadWindow::open();
        let error = read(&files).unwrap_err();
        let copy = error.clone();
        let mut visitor = NativeInspection::default();
        copy.visit(&mut visitor);
        let measured = window.close();
        assert_eq!((measured.allocations, measured.requested_bytes), (0, 0));
        drop(files);
        drop(footer);
        drop(error);
        let reason = Capture::read(&copy).child("reason");
        assert_eq!(reason.code, code);
        match code {
            "columnar-stop-delta" => {
                assert!(matches!(reason.field("header"), ErrorValue::Unsigned(16)));
            }
            "columnar-field-type" => {
                assert!(matches!(reason.field("id"), ErrorValue::Signed(1)));
                assert!(matches!(reason.field("actual"), ErrorValue::Unsigned(8)));
                assert!(matches!(reason.field("expected"), ErrorValue::Unsigned(5)));
            }
            "columnar-byte-range" => {
                assert!(matches!(reason.field("offset"), ErrorValue::ByteCount(11)));
                assert!(
                    matches!(reason.field("length"), ErrorValue::ByteCount(value) if value == u128::from(u64::MAX))
                );
            }
            "columnar-binary-length" => assert!(
                matches!(reason.field("value"), ErrorValue::ByteCount(value) if value == u128::from(u64::MAX))
            ),
            _ => {}
        }
    }

    // A complete healthy file remains valid with a nested unknown struct whose
    // binary payload is skipped as bytes, without imposing string validation.
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let files = write(
        &dataset,
        &ContentStore::default(),
        Compression::Uncompressed,
    )
    .unwrap();
    let mut array = files.files.into_array();
    let file = &mut array[0];
    let old_length = u32::from_le_bytes(file[file.len() - 8..file.len() - 4].try_into().unwrap());
    let footer_end = file.len() - 8;
    assert_eq!(file[footer_end - 1], 0);
    // An explicit field id 100, struct, binary field 1, one byte, end struct/root.
    let extra = [0x0c, 0xc8, 0x01, 0x18, 1, 0xff, 0];
    file.splice(footer_end - 1..footer_end - 1, extra);
    let length_at = file.len() - 8;
    let new_length = old_length + u32::try_from(extra.len()).unwrap();
    file[length_at..length_at + 4].copy_from_slice(&new_length.to_le_bytes());
    assert_eq!(
        read(&ParquetFiles::from_array(array))
            .unwrap()
            .dataset
            .quad_count(),
        0
    );
}

/// Public page decoding retains the original signed prefix after file disposal.
fn public_plain_value_refusals_survive_file_disposal() {
    let payload = b"original value-body payload";
    let mut blobs = ContentStore::new();
    blobs.insert(payload.to_vec());
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let files = write(&dataset, &blobs, Compression::Uncompressed)
        .unwrap()
        .files;
    assert_eq!(read(&files).unwrap().blobs.len(), 1);
    let mut original = (payload.len() as i32).to_le_bytes().to_vec();
    original.extend_from_slice(payload);
    for length in [-1_i32, i32::MIN] {
        let mut array = files.clone().into_array();
        let mut replacement = length.to_le_bytes().to_vec();
        replacement.extend_from_slice(payload);
        replace_unique(&mut array[4], &original, &replacement);
        let changed = ParquetFiles::from_array(array);
        let error = read(&changed).unwrap_err();
        drop(changed);
        let copy = error.clone();
        drop(error);
        assert!(matches!(
            copy,
            ColumnarError::ValueBody {
                context: "PLAIN BYTE_ARRAY length",
                reason: ValueBodyReason::NegativeLength { length: actual },
            } if actual == length
        ));
        let native = std::error::Error::source(&copy)
            .unwrap()
            .downcast_ref::<ValueBodyReason>()
            .unwrap();
        let ColumnarError::ValueBody { reason, .. } = &copy else {
            unreachable!("the original value-body variant was checked above");
        };
        assert!(std::ptr::eq(native, reason));
        inspect_without_allocation(&copy);
        let fields = Capture::read(&copy).child("reason");
        assert_eq!(fields.code, "columnar-negative-length");
        assert!(
            matches!(fields.field("length"), ErrorValue::Signed(actual) if actual == i128::from(length))
        );
    }
}

purrdf_testkit::harness_main!(
    original_parquet_bounds_survive_file_disposal_without_narrowing,
    public_projection_and_write_preserve_source_owners_and_fields,
    human_presentation_preserves_codec_and_source_messages,
    public_blob_read_preserves_original_integrity_cause,
    public_reconstruction_preserves_original_diagnostic,
    native_cause_transport_preserves_storage_and_full_width_rows,
    public_compact_causes_survive_input_disposal,
    public_compact_first_fault_and_unknown_skips_do_not_allocate,
    public_plain_value_refusals_survive_file_disposal,
);
