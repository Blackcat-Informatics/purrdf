// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Target-agnostic GTS segment decode + two-phase resolution core.
//!
//! Folding GTS events into an id-carrying model has two hazards that any
//! consumer must handle identically:
//!
//! 1. **Per-segment scope.** `reader::read` folds every segment into one
//!    append-order term table, which destroys per-segment blank-node scope (the
//!    same `_:b1` label in two segments names two *different* nodes). Only the
//!    [`StreamingSink`] callbacks preserve segment identity — each carries a
//!    `segment_index`.
//! 2. **Event-order independence.** `writer::Writer::deterministic` emits frames
//!    in the order `terms → quads → reifies → annot`. A quoted-triple `Term`
//!    written in the older indirect spelling therefore arrives (in the `terms`
//!    frame) carrying only its `reifier` id — the `reifies` frame that binds
//!    that reifier to the triple's `(s, p, o)` arrives **later**. A single-pass
//!    fold that resolves such a term the instant its `term` event fires cannot
//!    succeed. (A self-describing triple term, which states its own components
//!    in `Term::triple`, has no such dependency; its components are still
//!    resolved through the same buffered pass because they are ordinary term
//!    ids.)
//!
//! [`SegmentResolver`] owns that hazard-handling once, generic over an emit
//! target ([`ResolvedSink`]). It buffers each segment's raw term / quad /
//! reifier / annotation rows, then runs a **two-phase** resolution:
//!
//! 1. **Streaming phase** ([`StreamingSink`] callbacks): record RAW per-segment
//!    descriptors keyed by `(segment_index, gts_id)`; forward every non-RDF
//!    event straight through to the target.
//! 2. **Resolution phase** (per-segment flush, see below): resolve each gts term
//!    to a target id — non-triple terms intern directly; triple terms resolve
//!    their now-complete `(s, p, o)` iteratively, inner-first, with continuations
//!    threaded through the buffered terms — then push the raw quad / reifier /
//!    annotation rows through the per-segment remap.
//!
//! # Memory model — resolve per-segment at close
//!
//! GTS term ids are segment-local and no resolution ever crosses a segment
//! (reifier bindings, datatype ids, and triple components are all
//! within-segment), and the streaming reader delivers every event of segment
//! *N* before any event of segment *N+1*. [`SegmentResolver`] therefore FLUSHES
//! a completed segment the moment an incoming event carries a `segment_index`
//! greater than the current one: it resolves that segment's buffered terms in
//! sorted `gts_id` order, pushes its quads / reifiers / annotations in insertion
//! order, and drops that segment's raw buffers. [`SegmentResolver::finish`]
//! flushes the final segment. Because `(seg, id)` keys sort segment-major, this
//! yields the SAME interner allocation order (hence identical frozen term
//! order) and the SAME row push order as a whole-file fold that resolved all
//! buffered terms in one global sorted pass — while bounding buffered memory to
//! a single segment.
//!
//! Per the no-optionality / hard-fail doctrine, a *genuinely* dangling term id
//! or reifier binding — one still unresolved after ALL of a segment's events are
//! seen — is an [`Err`], never a silent skip.

use purrdf_lex::cbor::Value;

use std::sync::Arc;

use crate::dictionary::{DictionaryError, TermKey, TermLease};
use crate::model::{
    Diagnostic, OpaqueNode, Quad, Signature, StreamableInfo, Suppression, Term, TermKind, Triple3,
};
use crate::reader::{BlobPayload, BlobRefusal, FrameContext, StreamingSink};

mod rows;
use rows::Rows;
mod storage;
pub use storage::{
    PendingResolution, ResidentResolutionStorage, ResolutionCounts, ResolutionStorage,
    ResolutionTerm,
};

/// The emit target [`SegmentResolver`] drives once GTS segment-local ids have
/// been resolved to the target's own id space.
///
/// The resolver owns GTS-frame decode and per-segment two-phase resolution; the
/// target owns interning, row storage, and the mapping of the four structured
/// decode failures onto its own error type (usually enriched with location
/// context). Non-RDF events pass through with no-op defaults so a target that
/// only cares about triples need not implement them.
pub trait ResolvedSink {
    /// Caller-owned replay storage for transformed RDF frames, when supported.
    fn frame_scratch(
        &mut self,
    ) -> Option<std::io::Result<Box<dyn purrdf_lex::cbor::ReplayStorage>>> {
        None
    }
    /// The target's resolved-term identifier. `Copy` so the resolver can cache
    /// it in its per-segment remap without cloning.
    type Id: Copy;
    /// The target's error type. Streaming callbacks latch the first one.
    type Error;

    /// Intern a resolved IRI term, returning its target id.
    fn intern_iri(
        &mut self,
        segment_index: u64,
        gts_id: usize,
        iri: &str,
    ) -> Result<Self::Id, Self::Error>;

    /// Intern a resolved blank-node term (already scoped by `segment_index`),
    /// returning its target id.
    fn intern_blank(
        &mut self,
        segment_index: u64,
        gts_id: usize,
        label: &str,
    ) -> Result<Self::Id, Self::Error>;

    /// Intern a resolved literal term, returning its target id. The datatype (if
    /// any) is already resolved to a target id; the target performs its own
    /// datatype-must-be-IRI check and base-direction parsing.
    fn intern_literal(
        &mut self,
        segment_index: u64,
        gts_id: usize,
        lexical: String,
        datatype: Option<Self::Id>,
        lang: Option<String>,
        direction: Option<String>,
    ) -> Result<Self::Id, Self::Error>;

    /// Intern borrowed stored literal fields while retaining their original account.
    /// Targets using external dictionaries implement this callback. The default
    /// refuses explicitly rather than making an unadmitted lexical copy.
    fn intern_stored_literal(
        &mut self,
        segment_index: u64,
        gts_id: usize,
        _literal: TermLease,
        _datatype: Option<Self::Id>,
    ) -> Result<Self::Id, Self::Error> {
        Err(self.err_dangling_term(segment_index, gts_id, "unsupported stored literal callback"))
    }

    /// Intern a resolved quoted-triple term from its resolved components.
    fn intern_triple(
        &mut self,
        segment_index: u64,
        gts_id: usize,
        s: Self::Id,
        p: Self::Id,
        o: Self::Id,
    ) -> Result<Self::Id, Self::Error>;

    /// Push a resolved quad row (`g` is `None` for the default graph).
    fn push_quad(
        &mut self,
        segment_index: u64,
        s: Self::Id,
        p: Self::Id,
        o: Self::Id,
        g: Option<Self::Id>,
    ) -> Result<(), Self::Error>;

    /// Push a resolved reifier row binding `reifier` to the triple `(s, p, o)`
    /// in graph `g`.
    fn push_reifier(
        &mut self,
        segment_index: u64,
        reifier: Self::Id,
        s: Self::Id,
        p: Self::Id,
        o: Self::Id,
        g: Option<Self::Id>,
    ) -> Result<(), Self::Error>;

    /// Push a resolved annotation row `(reifier, predicate, object)` in graph
    /// `g`.
    fn push_annotation(
        &mut self,
        segment_index: u64,
        reifier: Self::Id,
        p: Self::Id,
        o: Self::Id,
        g: Option<Self::Id>,
    ) -> Result<(), Self::Error>;

    /// Build the error for a gts id that no `term` event ever introduced —
    /// a genuinely dangling reference. `role` names the referencing position.
    fn err_dangling_term(&self, segment_index: u64, gts_id: usize, role: &str) -> Self::Error;

    /// Legacy structural-depth error factory, retained for source compatibility.
    /// Iterative resolution no longer calls it.
    fn err_nesting_limit(&self, segment_index: u64, gts_id: usize) -> Self::Error;

    /// Build the error for a quoted-triple term that states neither its own
    /// `(s, p, o)` nor a reifier id to resolve them through.
    fn err_unbound_triple(&self, segment_index: u64, gts_id: usize) -> Self::Error;

    /// Build the error for a triple term whose reifier no `reifies` event ever
    /// bound.
    fn err_missing_reifier(&self, segment_index: u64, reifier: usize) -> Self::Error;

    /// Per-frame provenance passthrough (default no-op).
    fn frame(&mut self, _ctx: FrameContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Inline blob digest + declared metadata passthrough (default no-op).
    fn blob(
        &mut self,
        _segment_index: u64,
        _digest: &str,
        _meta: Option<&Value>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Original encoded blob bound passthrough (default unbounded).
    fn blob_encoded_limit(&self) -> Option<usize> {
        None
    }

    /// Eager blob decoding bound passthrough (default unbounded).
    fn blob_decode_limit(&self) -> Option<usize> {
        None
    }

    /// Enclosing (non-blob) frame decode bound (default: unbounded).
    fn frame_decode_limit(&self) -> Option<usize> {
        None
    }

    /// Borrowed blob payload passthrough (default no-op).
    fn blob_payload(&mut self, _payload: BlobPayload<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Refused blob passthrough (default no-op).
    fn blob_refused(&mut self, _refusal: BlobRefusal<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Opaque frame passthrough (default no-op).
    fn opaque(&mut self, _segment_index: u64, _node: &OpaqueNode) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Signature observation passthrough (default no-op).
    fn signature(&mut self, _segment_index: u64, _sig: &Signature) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Suppression directive passthrough (default no-op).
    fn suppression(
        &mut self,
        _segment_index: u64,
        _suppression: &Suppression,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Completed segment-head passthrough (default no-op).
    fn segment_head(&mut self, _segment_index: u64, _head: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Completed streamable-layout passthrough (default no-op).
    fn streamable_layout(
        &mut self,
        _segment_index: u64,
        _info: &StreamableInfo,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Reader diagnostic passthrough (default no-op).
    fn diagnostic(&mut self, _diag: &Diagnostic) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// A [`StreamingSink`] that drives a [`ResolvedSink`], owning GTS-frame decode
/// and per-segment two-phase resolution exactly once (see the module docs).
///
/// The `StreamingSink` callbacks return `()`; a decode or target error is
/// therefore latched (first-wins) and surfaced through [`Self::take_error`],
/// while [`Self::finish`] returns any error raised flushing the final segment.
pub struct SegmentResolver<
    S: ResolvedSink,
    D: ResolutionStorage<S::Id> = ResidentResolutionStorage<<S as ResolvedSink>::Id>,
> {
    /// The emit target resolved rows are pushed into.
    sink: S,
    /// Pending continuations, completed mappings and first reifier bindings.
    storage: D,
    /// Preserve a backend's original cause at the consumer's error boundary.
    map_storage: fn(D::Error) -> S::Error,
    /// Error transport selected with caller-owned numeric row replay.
    map_rows: Option<fn(std::io::Error) -> S::Error>,
    /// Accepted quad identifiers, compactly encoded until their phase runs.
    raw_quads: Rows<3>,
    /// Accepted reifier identifiers, preserving every binding in arrival order.
    raw_reifiers: Rows<4>,
    /// Accepted annotation identifiers, preserving their optional graph.
    raw_annotations: Rows<3>,
    /// The segment currently being buffered; a higher incoming index flushes it.
    current_segment: Option<u64>,
    /// First latched error. Streaming callbacks are infallible, so a failure is
    /// parked here and surfaced after the reader returns.
    error: Option<S::Error>,
    failed: bool,
}

impl<S: ResolvedSink, D: ResolutionStorage<S::Id>> std::fmt::Debug for SegmentResolver<S, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SegmentResolver")
            .field("buffered_terms", &self.storage.counts().pending)
            .field("buffered_quads", &self.raw_quads.len())
            .field("buffered_reifiers", &self.raw_reifiers.len())
            .field("buffered_annotations", &self.raw_annotations.len())
            .field("buffered_remaps", &self.storage.counts().resolved)
            .field("buffered_reifier_bindings", &self.storage.counts().reifiers)
            .field("current_segment", &self.current_segment)
            .field("has_error", &self.error.is_some())
            .field("failed", &self.failed)
            .finish_non_exhaustive()
    }
}

impl<S: ResolvedSink> SegmentResolver<S> {
    /// Wrap an emit target in a fresh resolver.
    pub fn new(sink: S) -> Self {
        Self::with_storage(
            sink,
            ResidentResolutionStorage::default(),
            |error| match error {},
        )
    }
}

impl<S: ResolvedSink, D: ResolutionStorage<S::Id>> SegmentResolver<S, D> {
    /// Wrap a consumer with explicit caller-owned resolution storage.
    ///
    /// The backend preserves source keys, admitted term leases and original
    /// consumer identifiers across eviction. `map_storage` retains its native
    /// failure at the consumer's existing error boundary. There is no resident
    /// fallback. Numeric row replay remains a separate storage resource.
    pub fn with_storage(sink: S, storage: D, map_storage: fn(D::Error) -> S::Error) -> Self {
        Self {
            sink,
            storage,
            map_storage,
            map_rows: None,
            raw_quads: Rows::default(),
            raw_reifiers: Rows::default(),
            raw_annotations: Rows::default(),
            current_segment: None,
            error: None,
            failed: false,
        }
    }

    /// Supply separate caller-owned replay storage for all accepted row tables.
    ///
    /// Select this before delivering any events. Source acceptance and numeric
    /// row encoding remain shared with the resident path; only residency changes.
    ///
    /// # Errors
    /// Returns original storage failures through `map_error`, or refuses an
    /// attempt to replace storage after the drive has started.
    pub fn with_row_storage(
        mut self,
        quads: Box<dyn purrdf_lex::cbor::ReplayStorage>,
        reifiers: Box<dyn purrdf_lex::cbor::ReplayStorage>,
        annotations: Box<dyn purrdf_lex::cbor::ReplayStorage>,
        map_error: fn(std::io::Error) -> S::Error,
    ) -> Result<Self, S::Error> {
        if self.current_segment.is_some() || self.failed {
            return Err(map_error(purrdf_hash::error::io_error(
                std::io::ErrorKind::InvalidInput,
                DictionaryError::Corrupt("row storage must be selected before the drive"),
            )));
        }
        self.raw_quads = Rows::external(quads).map_err(map_error)?;
        self.raw_reifiers = Rows::external(reifiers).map_err(map_error)?;
        self.raw_annotations = Rows::external(annotations).map_err(map_error)?;
        self.map_rows = Some(map_error);
        Ok(self)
    }

    fn row_failure(&self, error: std::io::Error) -> S::Error {
        // Resident replay contains only its own immutable numeric bytes and
        // cannot produce an I/O failure. External replay installs this mapper.
        (self
            .map_rows
            .expect("fallible row replay requires its original error mapper"))(error)
    }

    /// Borrow the emit target.
    pub fn sink(&self) -> &S {
        &self.sink
    }

    /// Mutably borrow the emit target.
    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }

    /// Consume the resolver, returning the emit target.
    pub fn into_sink(self) -> S {
        self.sink
    }

    /// Recover the consumer and its caller-owned persistent resolution storage.
    pub fn into_parts(self) -> (S, D) {
        (self.sink, self.storage)
    }

    /// Take the first latched streaming error, if any.
    pub fn take_error(&mut self) -> Option<S::Error> {
        self.error.take()
    }

    /// Count of the per-segment reifier→triple binding map currently
    /// retained (see the `reifier_bindings` field doc). This is populated as
    /// `reifier` events for the CURRENTLY OPEN segment stream in, and is `0`
    /// immediately after every `resolve_buffered` flush — so reading
    /// it right after a segment has flushed (e.g. on the first event of the
    /// next segment, or after [`Self::finish`]) is bounded by a single
    /// segment's own reifier cardinality, never by the sum across every
    /// segment already flushed. Exposed for tests asserting the
    /// bounded-memory contract (GTS-SPEC §7.7) from outside the crate.
    pub fn buffered_reifier_binding_count(&self) -> usize {
        self.storage.counts().reifiers
    }

    /// Count of the per-segment gts-id → target-id remap memo currently
    /// retained (see the `remaps` field doc). Same bound as
    /// [`Self::buffered_reifier_binding_count`]: `0` immediately after every
    /// segment flush.
    pub fn buffered_remap_count(&self) -> usize {
        self.storage.counts().resolved
    }

    /// Flush the final buffered segment, resolving its terms and pushing its
    /// rows. Call once after [`crate::reader::read_to_sink`] returns (and after
    /// checking [`Self::take_error`]).
    pub fn finish(&mut self) -> Result<(), S::Error> {
        if self.failed {
            return Err(self.error.take().unwrap_or_else(|| {
                self.sink.err_dangling_term(
                    self.current_segment.unwrap_or_default(),
                    0,
                    "previously failed resolution",
                )
            }));
        }
        self.failed = true;
        self.resolve_buffered()?;
        if let Err(error) = self.raw_quads.finish() {
            return Err(self.row_failure(error));
        }
        if let Err(error) = self.raw_reifiers.finish() {
            return Err(self.row_failure(error));
        }
        self.raw_annotations
            .finish()
            .map_err(|error| self.row_failure(error))?;
        self.failed = false;
        Ok(())
    }

    /// Record the first latched error; later errors do not overwrite it.
    fn fail(&mut self, error: S::Error) {
        self.failed = true;
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    fn buffer_term(&mut self, segment: u64, term_id: usize, term: ResolutionTerm) {
        if self.failed {
            return;
        }
        self.advance_segment(segment);
        if self.failed {
            return;
        }
        let pending = PendingResolution {
            term: Some(term),
            parent: None,
            next: 0,
            active: false,
        };
        if let Err(error) = self.storage.set_pending(
            TermKey {
                segment,
                term: term_id,
            },
            pending,
        ) {
            self.fail((self.map_storage)(error));
        }
    }

    /// Note an incoming buffering event's segment. When it advances past the
    /// segment currently buffered, flush the completed segment (latching any
    /// error) before accepting the new one.
    fn advance_segment(&mut self, segment_index: u64) {
        match self.current_segment {
            Some(current) if segment_index > current => {
                if let Err(error) = self.resolve_buffered() {
                    self.fail(error);
                }
                self.current_segment = Some(segment_index);
            }
            None => self.current_segment = Some(segment_index),
            Some(_) => {}
        }
    }

    /// Phase 2 for the currently buffered segment: resolve every buffered term
    /// (sorted `(segment_index, gts_id)`, which is `gts_id`-sorted within the
    /// single buffered segment) then push its quad / reifier / annotation rows
    /// in insertion order, draining the raw buffers.
    fn resolve_buffered(&mut self) -> Result<(), S::Error> {
        // Resolve every introduced term (idempotent through `remaps`). Iterate
        // in a deterministic order so the target's interner allocation order —
        // and thus any frozen term order — is reproducible for a fixed stream.
        if let Some(segment_index) = self.current_segment {
            let mut previous = None;
            while let Some(gts_id) = self
                .storage
                .next_term(segment_index, previous)
                .map_err(self.map_storage)?
            {
                self.resolve_term(segment_index, gts_id, "term")?;
                previous = Some(gts_id);
            }
        }

        let mut quads = std::mem::take(&mut self.raw_quads).into_iter();
        let quad_result = (|| {
            for row in quads.by_ref() {
                let (segment, [s, p, o], g) = row.map_err(|error| self.row_failure(error))?;
                let s = self.resolve_term(segment, s, "quad subject")?;
                let p = self.resolve_term(segment, p, "quad predicate")?;
                let o = self.resolve_term(segment, o, "quad object")?;
                let g = g
                    .map(|id| self.resolve_term(segment, id, "quad graph name"))
                    .transpose()?;
                self.sink.push_quad(segment, s, p, o, g)?;
            }
            Ok(())
        })();
        self.raw_quads = quads.into_empty(quad_result.is_ok());
        quad_result?;

        let mut reifiers = std::mem::take(&mut self.raw_reifiers).into_iter();
        let reifier_result = (|| {
            for row in reifiers.by_ref() {
                let (segment, [reifier, s, p, o], g) =
                    row.map_err(|error| self.row_failure(error))?;
                let reifier = self.resolve_term(segment, reifier, "reifier")?;
                let s = self.resolve_term(segment, s, "reified subject")?;
                let p = self.resolve_term(segment, p, "reified predicate")?;
                let o = self.resolve_term(segment, o, "reified object")?;
                let g = g
                    .map(|id| self.resolve_term(segment, id, "reifier graph name"))
                    .transpose()?;
                self.sink.push_reifier(segment, reifier, s, p, o, g)?;
            }
            Ok(())
        })();
        self.raw_reifiers = reifiers.into_empty(reifier_result.is_ok());
        reifier_result?;

        let mut annotations = std::mem::take(&mut self.raw_annotations).into_iter();
        let annotation_result = (|| {
            for row in annotations.by_ref() {
                let (segment, [r, p, v], g) = row.map_err(|error| self.row_failure(error))?;
                let r = self.resolve_term(segment, r, "annotation reifier")?;
                let p = self.resolve_term(segment, p, "annotation predicate")?;
                let v = self.resolve_term(segment, v, "annotation object")?;
                let g = g
                    .map(|id| self.resolve_term(segment, id, "annotation graph name"))
                    .transpose()?;
                self.sink.push_annotation(segment, r, p, v, g)?;
            }
            Ok(())
        })();
        self.raw_annotations = annotations.into_empty(annotation_result.is_ok());
        annotation_result?;

        // Bounded-memory streaming fold: both maps are segment-local — no
        // resolution ever crosses a segment boundary (reifier bindings, and
        // the term-id remap memo, are only ever read for the segment that is
        // currently buffered) — so drop them here rather than retaining them
        // for the lifetime of the resolver. Without this, both grow O(total
        // reifiers/terms across ALL segments) instead of O(one segment).
        if let Some(segment) = self.current_segment {
            self.storage
                .complete_segment(segment)
                .map_err(self.map_storage)?;
        }

        Ok(())
    }

    /// Resolve a GTS segment-local term id to its target id, interning it (and
    /// any nested quoted triples) on demand. The buffered records carry the DFS
    /// continuation; completed dependencies are memoized through `remaps`.
    /// An active-path revisit retains the original dangling-reference error and
    /// incoming edge role. A failed continuation stays active, so retrying finish
    /// cannot silently omit the term whose callback failed.
    fn resolve_term(
        &mut self,
        segment_index: u64,
        gts_id: usize,
        role: &str,
    ) -> Result<S::Id, S::Error> {
        let key = |term| TermKey {
            segment: segment_index,
            term,
        };
        if let Some(id) = self
            .storage
            .resolved(key(gts_id))
            .map_err(self.map_storage)?
        {
            return Ok(id);
        }
        let Some(mut root) = self
            .storage
            .pending(key(gts_id))
            .map_err(self.map_storage)?
        else {
            return Err(self.sink.err_dangling_term(segment_index, gts_id, role));
        };
        if root.active {
            return Err(self.sink.err_dangling_term(segment_index, gts_id, role));
        }
        root.active = true;
        self.storage
            .set_pending(key(gts_id), root)
            .map_err(self.map_storage)?;
        let mut current = gts_id;
        loop {
            let Some(mut held) = self
                .storage
                .pending(key(current))
                .map_err(self.map_storage)?
            else {
                return Err(self.sink.err_dangling_term(segment_index, current, role));
            };
            let Some(term) = held.term.as_deref() else {
                return Err(self.sink.err_dangling_term(segment_index, current, role));
            };
            let dependency = match term.kind {
                TermKind::Literal if held.next == 0 => {
                    term.datatype.map(|id| (id, "literal datatype"))
                }
                TermKind::Triple if held.next < 3 => {
                    let (s, p, o) = self.triple_components(segment_index, current, term)?;
                    Some(match held.next {
                        0 => (s, "reified subject"),
                        1 => (p, "reified predicate"),
                        _ => (o, "reified object"),
                    })
                }
                _ => None,
            };
            if let Some((child, child_role)) = dependency {
                held.next += 1;
                self.storage
                    .set_pending(key(current), held)
                    .map_err(self.map_storage)?;
                if self
                    .storage
                    .resolved(key(child))
                    .map_err(self.map_storage)?
                    .is_some()
                {
                    continue;
                }
                let Some(mut next) = self.storage.pending(key(child)).map_err(self.map_storage)?
                else {
                    return Err(self
                        .sink
                        .err_dangling_term(segment_index, child, child_role));
                };
                if next.active {
                    return Err(self
                        .sink
                        .err_dangling_term(segment_index, child, child_role));
                }
                next.active = true;
                next.parent = Some(current);
                self.storage
                    .set_pending(key(child), next)
                    .map_err(self.map_storage)?;
                current = child;
                continue;
            }
            let parent = held.parent;
            let term = held.term.take().expect("current term was inspected above");
            // Persist the consumed continuation before invoking a consumer.
            // A failed callback can never be retried as a silently omitted term.
            self.storage
                .set_pending(key(current), held)
                .map_err(self.map_storage)?;
            let resolved = self.intern_raw_term(segment_index, current, term)?;
            self.storage
                .set_resolved(key(current), resolved)
                .map_err(self.map_storage)?;
            self.storage
                .remove_pending(key(current))
                .map_err(self.map_storage)?;
            if let Some(parent) = parent {
                current = parent;
            } else {
                return Ok(resolved);
            }
        }
    }

    /// Intern one completed term while retaining its original payload owner.
    fn intern_raw_term(
        &mut self,
        segment_index: u64,
        gts_id: usize,
        term: ResolutionTerm,
    ) -> Result<S::Id, S::Error> {
        let our_id = match term.kind {
            TermKind::Iri => {
                // An absent or empty value is the target's `iri-missing-value`
                // failure, raised from its own `intern_iri`.
                let iri = term.value.as_deref().unwrap_or_default();
                self.sink.intern_iri(segment_index, gts_id, iri)?
            }
            // Per-segment scope isolation: the target scopes by `segment_index`
            // so the SAME blank label in different segments interns distinctly.
            TermKind::Bnode => {
                let generated;
                let label = match term.value.as_deref() {
                    Some(label) => label,
                    None => {
                        generated = format!("gts_bnode_{segment_index}_{gts_id}");
                        &generated
                    }
                };
                self.sink.intern_blank(segment_index, gts_id, label)?
            }
            TermKind::Literal => {
                let datatype = match term.datatype {
                    Some(dt_gts_id) => Some(self.resolved_dependency(
                        segment_index,
                        dt_gts_id,
                        "literal datatype",
                    )?),
                    None => None,
                };
                match term {
                    ResolutionTerm::Stored(term) => {
                        self.sink
                            .intern_stored_literal(segment_index, gts_id, term, datatype)?
                    }
                    ResolutionTerm::Resident(term) => {
                        let term = Arc::unwrap_or_clone(term);
                        self.sink.intern_literal(
                            segment_index,
                            gts_id,
                            term.value.unwrap_or_default(),
                            datatype,
                            term.lang,
                            term.direction,
                        )?
                    }
                }
            }
            // A self-describing quoted triple (wire `tt`) carries its own
            // components, so it needs no reifier at all; only a term written in
            // the legacy indirect spelling has to go through a reifier id — and
            // that id may now bind several triples, in which case its FIRST
            // binding is the one such a term can mean (the reader flags the
            // ambiguity as `ConflictingReifier`).
            TermKind::Triple => {
                let (s, p, o) = self.triple_components(segment_index, gts_id, &term)?;
                let (s, p, o) = (
                    self.resolved_dependency(segment_index, s, "reified subject")?,
                    self.resolved_dependency(segment_index, p, "reified predicate")?,
                    self.resolved_dependency(segment_index, o, "reified object")?,
                );
                self.sink.intern_triple(segment_index, gts_id, s, p, o)?
            }
        };
        Ok(our_id)
    }

    /// Locate components without resolving them. Explicit components take
    /// precedence; the first legacy reifier binding is a lookup, not a term edge.
    fn triple_components(
        &mut self,
        segment_index: u64,
        gts_id: usize,
        term: &Term,
    ) -> Result<Triple3, S::Error> {
        match (term.triple, term.reifier) {
            (Some(components), _) => Ok(components),
            (None, Some(reifier)) => self
                .storage
                .first_binding(TermKey {
                    segment: segment_index,
                    term: reifier,
                })
                .map_err(self.map_storage)?
                .ok_or_else(|| self.sink.err_missing_reifier(segment_index, reifier)),
            (None, None) => Err(self.sink.err_unbound_triple(segment_index, gts_id)),
        }
    }

    fn resolved_dependency(
        &mut self,
        segment: u64,
        term: usize,
        role: &str,
    ) -> Result<S::Id, S::Error> {
        self.storage
            .resolved(TermKey { segment, term })
            .map_err(self.map_storage)?
            .ok_or_else(|| self.sink.err_dangling_term(segment, term, role))
    }
}

impl<S: ResolvedSink, D: ResolutionStorage<S::Id>> StreamingSink for SegmentResolver<S, D> {
    fn frame_scratch(
        &mut self,
    ) -> Option<std::io::Result<Box<dyn purrdf_lex::cbor::ReplayStorage>>> {
        if self.failed {
            return Some(Err(std::io::Error::other("segment sink already failed")));
        }
        self.sink.frame_scratch()
    }
    fn frame(&mut self, ctx: FrameContext<'_>) {
        if self.failed {
            return;
        }
        if let Err(error) = self.sink.frame(ctx) {
            self.fail(error);
        }
    }

    fn term(&mut self, segment_index: u64, term_id: usize, term: Arc<Term>) {
        self.buffer_term(segment_index, term_id, ResolutionTerm::Resident(term));
    }

    fn stored_term(
        &mut self,
        segment_index: u64,
        term_id: usize,
        term: TermLease,
    ) -> Result<(), DictionaryError> {
        self.buffer_term(segment_index, term_id, ResolutionTerm::Stored(term));
        // Consumer/backend failures retain their original S::Error in the
        // existing first-failure slot, checked before finishing resolution.
        Ok(())
    }

    fn quad(&mut self, segment_index: u64, quad: Quad) {
        if self.failed {
            return;
        }
        self.advance_segment(segment_index);
        if self.failed {
            return;
        }
        let (s, p, o, graph) = quad;
        if let Err(error) = self.raw_quads.push(segment_index, [s, p, o], graph) {
            self.fail(self.row_failure(error));
        }
    }

    fn reifier(&mut self, segment_index: u64, reifier: crate::model::ReifierRow) {
        if self.failed {
            return;
        }
        self.advance_segment(segment_index);
        if self.failed {
            return;
        }
        // Row-array `(reifier_id, (s, p, o), graph?)`. Record the reifier → (s,
        // p, o) binding so a Triple term (any order) can resolve its components,
        // and stash the row (with its named graph) so the reifier resource binds
        // the resolved triple at segment close.
        let (reifier_id, triple, graph) = reifier;
        // One reifier id may bind several triples (`rdf:reifies` is not
        // functional). Every row is stashed below and pushed as its own
        // statement-layer binding; this memo only serves the LEGACY indirect
        // triple-term spelling, which can name a single triple — so it keeps
        // the FIRST binding, exactly as `Graph::reifier` does on the folded
        // path.
        if let Err(error) = self.storage.bind_first(
            TermKey {
                segment: segment_index,
                term: reifier_id,
            },
            triple,
        ) {
            self.fail((self.map_storage)(error));
            return;
        }
        let (s, p, o) = triple;
        if let Err(error) = self
            .raw_reifiers
            .push(segment_index, [reifier_id, s, p, o], graph)
        {
            self.fail(self.row_failure(error));
        }
    }

    fn annotation(&mut self, segment_index: u64, annotation: crate::model::AnnotationRow) {
        if self.failed {
            return;
        }
        self.advance_segment(segment_index);
        if self.failed {
            return;
        }
        // Row-array `(reifier, predicate, value, graph?)`.
        let (reifier, predicate, value, graph) = annotation;
        if let Err(error) =
            self.raw_annotations
                .push(segment_index, [reifier, predicate, value], graph)
        {
            self.fail(self.row_failure(error));
        }
    }

    fn suppression(&mut self, segment_index: u64, suppression: &Suppression) {
        if self.failed {
            return;
        }
        if let Err(error) = self.sink.suppression(segment_index, suppression) {
            self.fail(error);
        }
    }

    fn blob(&mut self, segment_index: u64, digest: &str, meta: Option<&Value>) {
        if self.failed {
            return;
        }
        if let Err(error) = self.sink.blob(segment_index, digest, meta) {
            self.fail(error);
        }
    }

    fn blob_encoded_limit(&self) -> Option<usize> {
        self.sink.blob_encoded_limit()
    }

    fn blob_decode_limit(&self) -> Option<usize> {
        self.sink.blob_decode_limit()
    }

    fn frame_decode_limit(&self) -> Option<usize> {
        self.sink.frame_decode_limit()
    }

    fn blob_payload(&mut self, payload: BlobPayload<'_>) {
        if self.failed {
            return;
        }
        if let Err(error) = self.sink.blob_payload(payload) {
            self.fail(error);
        }
    }

    fn blob_refused(&mut self, refusal: BlobRefusal<'_>) {
        if self.failed {
            return;
        }
        if let Err(error) = self.sink.blob_refused(refusal) {
            self.fail(error);
        }
    }

    fn opaque(&mut self, segment_index: u64, opaque: &OpaqueNode) {
        if self.failed {
            return;
        }
        if let Err(error) = self.sink.opaque(segment_index, opaque) {
            self.fail(error);
        }
    }

    fn signature(&mut self, segment_index: u64, signature: &Signature) {
        if self.failed {
            return;
        }
        if let Err(error) = self.sink.signature(segment_index, signature) {
            self.fail(error);
        }
    }

    fn diagnostic(&mut self, diagnostic: &Diagnostic) {
        if self.failed {
            return;
        }
        if let Err(error) = self.sink.diagnostic(diagnostic) {
            self.fail(error);
        }
    }

    fn segment_head(&mut self, segment_index: u64, head: &[u8]) {
        if self.failed {
            return;
        }
        if let Err(error) = self.sink.segment_head(segment_index, head) {
            self.fail(error);
        }
    }

    fn streamable_layout(&mut self, segment_index: u64, info: &StreamableInfo) {
        if self.failed {
            return;
        }
        if let Err(error) = self.sink.streamable_layout(segment_index, info) {
            self.fail(error);
        }
    }
}
