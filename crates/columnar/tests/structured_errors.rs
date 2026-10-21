// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Physical Parquet reader refusals through the portable public API.

use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Weak};

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_columnar::{
    ColumnarError, ColumnarWriteError, Compression, ParquetFiles, project, read, write,
};
use purrdf_core::{
    ContentStore, DatasetView, GraphMatch, QuadIds, RdfDataset, RdfDatasetBuilder,
    RdfStoreCapabilities, TermGuard, TermId, TermRef, TermValue,
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
    ) {
        let term = builder.intern_iri("https://example.org/source").unwrap();
        let object = if phase == Phase::NestedResolution {
            let leaf = builder.intern_iri("https://example.org/dependent").unwrap();
            builder.intern_triple(term, term, leaf).unwrap()
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
                    Phase::Initial | Phase::Completed => 0,
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

purrdf_testkit::harness_main!(
    original_parquet_bounds_survive_file_disposal_without_narrowing,
    public_projection_and_write_preserve_source_owners_and_fields,
    human_presentation_preserves_codec_and_source_messages,
);
