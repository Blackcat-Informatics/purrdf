// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Bridge a GTS file onto the permissive RDF 1.2 ingestion protocol
//! ([`purrdf_events::RdfEventSink`]).
//!
//! [`stream_events`] drives a GTS file through the shared two-phase decode core
//! ([`crate::segment_decode::SegmentResolver`]) into a [`GtsEventSink`] — an
//! [`RdfEventSink`] that also receives per-frame GTS provenance and the
//! GTS-specific frames (blobs, opaque nodes, signatures, suppressions) that have
//! no place in the neutral RDF event vocabulary.
//!
//! It rides the bounded-memory streaming fold (GTS-SPEC §7.7 "Streaming fold and
//! bounded memory"): the reader delivers one frame at a time and the decode core
//! buffers only the current segment's descriptors, so an external index builder
//! (posting lists, a ClaimId→byte-offset map) runs in a single pass without ever
//! materializing the union dataset.
//!
//! # Id and scope mapping
//!
//! The decode core resolves each segment's terms in a deterministic
//! `(segment_index, gts_id)` order (segment-major, `gts_id`-sorted, inner-triple
//! first), calling the sink's `intern_*` methods exactly once per term. The
//! bridge mints a fresh, monotonically increasing [`EventTermId`] on each intern
//! — so id assignment is a pure function of file order, reproducible across runs.
//! Because every id is declared exactly once, the protocol never sees an
//! [`purrdf_events::EventError::RedeclaredId`].
//!
//! Per-segment blank-node scope (the same `_:b0` in two segments names two
//! different nodes) is preserved by opening a fresh [`ScopeId`] on the first
//! intern of each segment and closing the previous one, mirroring GTS segment
//! isolation onto the protocol's scope namespacing.
//!
//! # Faithful failures
//!
//! The RDF event protocol carries no graph slot on a reifier binding or a
//! statement annotation ([`EventTriple`] is a bare triple). A GTS
//! *graph-scoped* reifier or annotation therefore cannot be represented, and the
//! bridge raises a hard [`purrdf_events::EventError`] rather than silently
//! dropping the graph — honoring the no-swallow doctrine.
//!
//! The raw GTS reader ([`crate::reader`]) is a deliberately permissive
//! Baseline Reader (GTS-SPEC §7.4/§7.5, GTS-CONFORMANCE.md §6): a row whose
//! subject/predicate/object/graph-name/datatype/reifier names a segment-local
//! term id no `term` event ever introduced is diagnosed (`PositionConstraint`
//! or `ForwardReference`) and DROPPED, and folding continues so a damaged or
//! partially-authored file still yields its recoverable content. That
//! degrade-and-continue policy is right for the raw byte reader, but the RDF
//! event protocol promises every id it ever declares is usable — a row that
//! silently vanishes is indistinguishable from a fact that was never
//! asserted. [`EventEmitter::diagnostic`] therefore escalates exactly those
//! two diagnostic codes to a hard [`purrdf_events::EventError`] (the same
//! "genuinely dangling term id" failure [`stream_events`] documents), while
//! every other reader diagnostic (frame damage, chain breaks, unknown codecs,
//! …) still passes through to [`GtsEventSink::gts_diagnostic`] unharmed —
//! that class of degradation is not a dangling reference and stays governed
//! by the permissive-read contract (see `claimid_offset_index_single_pass` in
//! `tests/event_bridge.rs`).

use core::ops::ControlFlow;
use std::collections::HashMap;

use ciborium::value::Value;
use purrdf_events::{
    EventError, EventQuad, EventTerm, EventTermId, EventTriple, RdfEventSink, ScopeId,
    TextDirection,
};

use crate::model::{ByteRange, Diagnostic, OpaqueNode, Signature, StreamableInfo, Suppression};
use crate::reader::{FrameContext, ReadOptions, StreamingReadResult, read_to_sink_with_options};
use crate::segment_decode::{ResolvedSink, SegmentResolver};

/// Well-known `xsd:string` datatype IRI implied by a plain literal (RDF §7.1).
///
/// This is RDF's own datatype IRI, mirrored from
/// `crates/rdf/src/native_codecs/hextuples.rs`; it is NOT fabricated PurRDF
/// vocabulary.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
/// Well-known `rdf:langString` datatype IRI implied by a language tag (RDF §7.1).
///
/// RDF's own datatype IRI (see [`XSD_STRING`]), not fabricated vocabulary.
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// A GTS-aware event sink: an [`RdfEventSink`] that also receives per-frame GTS
/// provenance and the GTS-specific frames that have no neutral RDF event.
///
/// Every GTS hook defaults to a no-op `Continue`, so a consumer that only cares
/// about RDF terms and quads implements just [`RdfEventSink`] and inherits inert
/// GTS hooks. Each provenance-bearing hook receives the [`FrameContext`] cached
/// from the most recently streamed frame — `None` before the first frame has
/// been announced, since the reader fires a frame's provenance *after* that
/// frame's rows.
pub trait GtsEventSink: RdfEventSink {
    /// Per-frame byte/identity provenance, announced once per frame.
    fn frame(&mut self, _ctx: FrameContext<'_>) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }

    /// An inline blob's content digest and declared public metadata.
    fn blob(
        &mut self,
        _ctx: Option<FrameContext<'_>>,
        _digest: &str,
        _meta: Option<&Value>,
    ) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }

    /// An opaque frame (unknown codec, missing key, or damaged payload).
    fn opaque(
        &mut self,
        _ctx: Option<FrameContext<'_>>,
        _node: &OpaqueNode,
    ) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }

    /// A signature observation on a frame.
    fn signature(
        &mut self,
        _ctx: Option<FrameContext<'_>>,
        _sig: &Signature,
    ) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }

    /// A suppression directive.
    fn suppression(
        &mut self,
        _ctx: Option<FrameContext<'_>>,
        _suppression: &Suppression,
    ) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }

    /// A reader diagnostic. Frame-scoped diagnostics carry the cached
    /// [`FrameContext`]; file-level diagnostics carry `None`.
    fn gts_diagnostic(
        &mut self,
        _ctx: Option<FrameContext<'_>>,
        _diagnostic: &Diagnostic,
    ) -> Result<ControlFlow<()>, EventError> {
        Ok(ControlFlow::Continue(()))
    }
}

/// An owned snapshot of the most recently streamed [`FrameContext`], kept so the
/// provenance-bearing [`GtsEventSink`] hooks can borrow it after the reader's
/// borrowed context has expired.
struct CachedFrame {
    segment_index: usize,
    frame_index: usize,
    content_id: Vec<u8>,
    range: ByteRange,
    frame_type: String,
    valid: bool,
}

impl CachedFrame {
    /// Reconstruct a borrowed [`FrameContext`] over this cached snapshot.
    fn as_context(&self) -> FrameContext<'_> {
        FrameContext {
            segment_index: self.segment_index,
            frame_index: self.frame_index,
            content_id: &self.content_id,
            range: self.range.clone(),
            frame_type: &self.frame_type,
            valid: self.valid,
        }
    }
}

/// A [`ResolvedSink`] that folds resolved GTS terms and rows onto a
/// [`GtsEventSink`], minting a fresh [`EventTermId`] per term and rotating a
/// [`ScopeId`] per segment.
///
/// Constructible through [`EventEmitter::new`] so callers that cannot go through
/// [`stream_events`] — chiefly tests exercising a decode failure the GTS
/// [`crate::writer::Writer`] refuses to author — can drive it behind a
/// [`SegmentResolver`] directly.
pub struct EventEmitter<'s> {
    /// The bridged consumer.
    sink: &'s mut dyn GtsEventSink,
    /// The next [`EventTermId`] to mint; incremented on every intern.
    next_id: u32,
    /// Resolved IRI text by minted id, retained so a literal's datatype id can be
    /// resolved back to its IRI string for [`EventTerm::Literal`].
    iri_map: HashMap<EventTermId, String>,
    /// The segment whose scope is currently open, if any.
    current_segment: Option<usize>,
    /// The scope opened for [`Self::current_segment`].
    current_scope: ScopeId,
    /// The most recently streamed frame's provenance.
    cached: Option<CachedFrame>,
    /// Set once any sink call requests [`ControlFlow::Break`]; further sink
    /// emission is suppressed so a cancelled drive is never finalized.
    cancelled: bool,
}

impl std::fmt::Debug for EventEmitter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventEmitter")
            .field("next_id", &self.next_id)
            .field("interned_iris", &self.iri_map.len())
            .field("current_segment", &self.current_segment)
            .field("current_scope", &self.current_scope)
            .field("cancelled", &self.cancelled)
            .finish_non_exhaustive()
    }
}

impl<'s> EventEmitter<'s> {
    /// Wrap a [`GtsEventSink`] in a fresh emitter.
    pub fn new(sink: &'s mut dyn GtsEventSink) -> Self {
        Self {
            sink,
            next_id: 0,
            iri_map: HashMap::new(),
            current_segment: None,
            current_scope: ScopeId::DEFAULT,
            cached: None,
            cancelled: false,
        }
    }

    /// Mint the next monotonically increasing [`EventTermId`].
    fn mint(&mut self) -> EventTermId {
        let id = EventTermId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Count of resolved IRI strings currently retained in `iri_map` (see the
    /// field doc). `iri_map` is cleared on every genuine segment transition
    /// ([`Self::ensure_scope`]), so reading this right after a segment has
    /// been fully resolved reflects only THAT segment's own interned-IRI
    /// count, never the cumulative total across every segment seen so far.
    /// Exposed for tests asserting the bounded-memory contract (GTS-SPEC
    /// §7.7) from outside the crate.
    pub fn interned_iri_count(&self) -> usize {
        self.iri_map.len()
    }

    /// Ensure the scope for `segment_index` is the open one, closing the previous
    /// segment's scope and opening a fresh one on a segment change. A no-op once
    /// cancelled (beyond recording the segment).
    fn ensure_scope(&mut self, segment_index: usize) -> Result<(), EventError> {
        if self.current_segment == Some(segment_index) {
            return Ok(());
        }
        // Genuine segment change: the shared decode core resolves and flushes
        // one segment's terms in full (inner-triple first) before opening the
        // next, so `iri_map` can never hold an IRI a later literal in THIS new
        // segment still needs — every datatype IRI a literal references is
        // (re-)interned within its own segment before that literal is emitted.
        // Dropping the map here bounds `EventEmitter`'s retained memory to one
        // segment's worth of IRIs (R8) instead of the whole file's.
        self.iri_map.clear();
        if self.cancelled {
            self.current_segment = Some(segment_index);
            return Ok(());
        }
        if self.current_segment.is_some()
            && self.sink.close_scope(self.current_scope)? == ControlFlow::Break(())
        {
            self.cancelled = true;
            self.current_segment = Some(segment_index);
            return Ok(());
        }
        self.current_scope = self.sink.open_scope()?;
        self.current_segment = Some(segment_index);
        Ok(())
    }

    /// Emit a term declaration, latching cancellation on a `Break`.
    fn emit_term(&mut self, id: EventTermId, term: EventTerm<'_>) -> Result<(), EventError> {
        if self.cancelled {
            return Ok(());
        }
        if self.sink.term(id, term)? == ControlFlow::Break(()) {
            self.cancelled = true;
        }
        Ok(())
    }

    /// Close the scope left open by the final segment, if any. Called once by
    /// [`stream_events`] before finalizing a non-cancelled drive.
    fn close_open_scope(&mut self) -> Result<(), EventError> {
        if self.cancelled {
            return Ok(());
        }
        if self.current_segment.is_some() {
            if self.sink.close_scope(self.current_scope)? == ControlFlow::Break(()) {
                self.cancelled = true;
            }
            self.current_segment = None;
        }
        Ok(())
    }

    /// Resolve a literal's datatype id (or absence) to its expanded datatype IRI.
    ///
    /// A `Some` datatype id that never resolved to an IRI is a hard error (a
    /// literal cannot be typed by a non-IRI). Absence defaults per RDF §7.1:
    /// `rdf:langString` with a language tag, else `xsd:string`.
    fn datatype_iri(
        &self,
        datatype: Option<EventTermId>,
        lang: Option<&str>,
    ) -> Result<String, EventError> {
        match datatype {
            Some(dt) => {
                self.iri_map.get(&dt).cloned().ok_or_else(|| {
                    EventError::message("GTS literal datatype must resolve to an IRI")
                })
            }
            None if lang.is_some() => Ok(RDF_LANG_STRING.to_owned()),
            None => Ok(XSD_STRING.to_owned()),
        }
    }
}

/// Parse a GTS literal base-direction token into a [`TextDirection`].
///
/// Mirrors `crates/rdf/src/gts_resolve.rs::parse_gts_direction`: `None` is
/// legitimate absence, an unrecognized token is a hard error, and RDF 1.2 admits
/// a base direction only on a non-empty language-tagged string.
fn parse_direction(
    direction: Option<&str>,
    lang: Option<&str>,
) -> Result<Option<TextDirection>, EventError> {
    let parsed = match direction {
        None => return Ok(None),
        Some("ltr") => TextDirection::Ltr,
        Some("rtl") => TextDirection::Rtl,
        Some(other) => {
            return Err(EventError::message(format!(
                "unrecognized GTS literal base direction {other:?}"
            )));
        }
    };
    if lang.is_none_or(str::is_empty) {
        return Err(EventError::message(
            "an RDF 1.2 literal base direction requires a non-empty language tag",
        ));
    }
    Ok(Some(parsed))
}

impl ResolvedSink for EventEmitter<'_> {
    type Id = EventTermId;
    type Error = EventError;

    fn intern_iri(
        &mut self,
        segment_index: usize,
        _gts_id: usize,
        iri: &str,
    ) -> Result<Self::Id, Self::Error> {
        self.ensure_scope(segment_index)?;
        let id = self.mint();
        self.iri_map.insert(id, iri.to_owned());
        self.emit_term(id, EventTerm::Iri(iri))?;
        Ok(id)
    }

    fn intern_blank(
        &mut self,
        segment_index: usize,
        _gts_id: usize,
        label: &str,
    ) -> Result<Self::Id, Self::Error> {
        self.ensure_scope(segment_index)?;
        let id = self.mint();
        let scope = self.current_scope;
        self.emit_term(id, EventTerm::Blank { label, scope })?;
        Ok(id)
    }

    fn intern_literal(
        &mut self,
        segment_index: usize,
        _gts_id: usize,
        lexical: String,
        datatype: Option<Self::Id>,
        lang: Option<String>,
        direction: Option<String>,
    ) -> Result<Self::Id, Self::Error> {
        self.ensure_scope(segment_index)?;
        // Clone the datatype IRI into a local so the `sink.term` call below does
        // not hold a borrow of `self.iri_map` while it borrows `self.sink`.
        let datatype_iri = self.datatype_iri(datatype, lang.as_deref())?;
        let direction = parse_direction(direction.as_deref(), lang.as_deref())?;
        let id = self.mint();
        self.emit_term(
            id,
            EventTerm::Literal {
                lexical: &lexical,
                datatype: &datatype_iri,
                language: lang.as_deref(),
                direction,
            },
        )?;
        Ok(id)
    }

    fn intern_triple(
        &mut self,
        segment_index: usize,
        _gts_id: usize,
        s: Self::Id,
        p: Self::Id,
        o: Self::Id,
    ) -> Result<Self::Id, Self::Error> {
        self.ensure_scope(segment_index)?;
        let id = self.mint();
        self.emit_term(id, EventTerm::Triple(EventTriple { s, p, o }))?;
        Ok(id)
    }

    fn push_quad(
        &mut self,
        segment_index: usize,
        s: Self::Id,
        p: Self::Id,
        o: Self::Id,
        g: Option<Self::Id>,
    ) -> Result<(), Self::Error> {
        self.ensure_scope(segment_index)?;
        if self.cancelled {
            return Ok(());
        }
        if self.sink.quad(EventQuad { s, p, o, g })? == ControlFlow::Break(()) {
            self.cancelled = true;
        }
        Ok(())
    }

    fn push_reifier(
        &mut self,
        segment_index: usize,
        reifier: Self::Id,
        s: Self::Id,
        p: Self::Id,
        o: Self::Id,
        g: Option<Self::Id>,
    ) -> Result<(), Self::Error> {
        self.ensure_scope(segment_index)?;
        if self.cancelled {
            return Ok(());
        }
        if g.is_some() {
            return Err(EventError::message(
                "graph-scoped reifier cannot be represented on the RdfEventSink protocol (no graph slot)",
            ));
        }
        if self.sink.reifier(reifier, EventTriple { s, p, o })? == ControlFlow::Break(()) {
            self.cancelled = true;
        }
        Ok(())
    }

    fn push_annotation(
        &mut self,
        segment_index: usize,
        reifier: Self::Id,
        p: Self::Id,
        o: Self::Id,
        g: Option<Self::Id>,
    ) -> Result<(), Self::Error> {
        self.ensure_scope(segment_index)?;
        if self.cancelled {
            return Ok(());
        }
        if g.is_some() {
            return Err(EventError::message(
                "graph-scoped annotation cannot be represented on the RdfEventSink protocol (no graph slot)",
            ));
        }
        if self.sink.annotation(reifier, p, o)? == ControlFlow::Break(()) {
            self.cancelled = true;
        }
        Ok(())
    }

    fn err_dangling_term(&self, segment_index: usize, gts_id: usize, role: &str) -> Self::Error {
        EventError::message(format!(
            "GTS dangling term reference: segment {segment_index} {role} names undeclared term id {gts_id}"
        ))
    }

    fn err_nesting_limit(&self, segment_index: usize, gts_id: usize) -> Self::Error {
        EventError::message(format!(
            "GTS quoted-triple nesting limit exceeded resolving segment {segment_index} term id {gts_id}"
        ))
    }

    fn err_unbound_triple(&self, segment_index: usize, gts_id: usize) -> Self::Error {
        EventError::message(format!(
            "GTS triple term has no reifier binding: segment {segment_index} term id {gts_id}"
        ))
    }

    fn err_missing_reifier(&self, segment_index: usize, reifier: usize) -> Self::Error {
        EventError::message(format!(
            "GTS triple term references missing reifier {reifier} in segment {segment_index}"
        ))
    }

    fn frame(&mut self, ctx: FrameContext<'_>) -> Result<(), Self::Error> {
        self.cached = Some(CachedFrame {
            segment_index: ctx.segment_index,
            frame_index: ctx.frame_index,
            content_id: ctx.content_id.to_vec(),
            range: ctx.range.clone(),
            frame_type: ctx.frame_type.to_owned(),
            valid: ctx.valid,
        });
        if self.cancelled {
            return Ok(());
        }
        if self.sink.frame(ctx)? == ControlFlow::Break(()) {
            self.cancelled = true;
        }
        Ok(())
    }

    fn blob(
        &mut self,
        _segment_index: usize,
        digest: &str,
        meta: Option<&Value>,
    ) -> Result<(), Self::Error> {
        if self.cancelled {
            return Ok(());
        }
        let ctx = self.cached.as_ref().map(CachedFrame::as_context);
        if self.sink.blob(ctx, digest, meta)? == ControlFlow::Break(()) {
            self.cancelled = true;
        }
        Ok(())
    }

    fn opaque(&mut self, _segment_index: usize, node: &OpaqueNode) -> Result<(), Self::Error> {
        if self.cancelled {
            return Ok(());
        }
        let ctx = self.cached.as_ref().map(CachedFrame::as_context);
        if self.sink.opaque(ctx, node)? == ControlFlow::Break(()) {
            self.cancelled = true;
        }
        Ok(())
    }

    fn signature(&mut self, _segment_index: usize, sig: &Signature) -> Result<(), Self::Error> {
        if self.cancelled {
            return Ok(());
        }
        let ctx = self.cached.as_ref().map(CachedFrame::as_context);
        if self.sink.signature(ctx, sig)? == ControlFlow::Break(()) {
            self.cancelled = true;
        }
        Ok(())
    }

    fn suppression(
        &mut self,
        _segment_index: usize,
        suppression: &Suppression,
    ) -> Result<(), Self::Error> {
        if self.cancelled {
            return Ok(());
        }
        let ctx = self.cached.as_ref().map(CachedFrame::as_context);
        if self.sink.suppression(ctx, suppression)? == ControlFlow::Break(()) {
            self.cancelled = true;
        }
        Ok(())
    }

    fn diagnostic(&mut self, diag: &Diagnostic) -> Result<(), Self::Error> {
        if self.cancelled {
            return Ok(());
        }
        // `PositionConstraint` / `ForwardReference` mean the RAW GTS reader
        // (§7.4/§7.5) just silently DROPPED a row — a quad/reifier/annotation
        // whose subject/predicate/object/graph-name/dt/rf named a segment-local
        // term id no `term` event ever introduced — rather than resolving it.
        // GTS's Baseline Reader is deliberately permissive about this (it
        // degrades and keeps folding survivors, per GTS-CONFORMANCE.md §6), but
        // the RDF event protocol is stricter: every id it ever declares MUST be
        // usable, and a row silently vanishing from the stream is exactly the
        // "genuinely dangling term id" this module's docs already promise is an
        // `Err`, never a silent skip. Escalate here rather than passing it
        // through as an informational `gts_diagnostic` — otherwise the drive
        // would finish "successfully" having quietly dropped the row, which is
        // indistinguishable from that row never having been asserted.
        if matches!(
            diag.code.as_str(),
            "PositionConstraint" | "ForwardReference"
        ) {
            return Err(EventError::message(format!(
                "GTS dangling term reference ({}): {}",
                diag.code, diag.detail
            )));
        }
        // A frame-scoped diagnostic carries the cached frame provenance; a
        // file-level diagnostic (e.g. `EmptyFile`) carries none.
        let ctx = diag
            .frame_index
            .and(self.cached.as_ref())
            .map(CachedFrame::as_context);
        if self.sink.gts_diagnostic(ctx, diag)? == ControlFlow::Break(()) {
            self.cancelled = true;
        }
        Ok(())
    }

    fn streamable_layout(
        &mut self,
        _segment_index: usize,
        _info: &StreamableInfo,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Drive a GTS file through the shared decode core into a [`GtsEventSink`].
///
/// Terms resolve in deterministic file order (segment-major, `gts_id`-sorted,
/// inner-triple first); each mints a fresh [`EventTermId`], and each segment
/// opens its own blank-node [`ScopeId`]. GTS-specific frames (blobs, opaque
/// nodes, signatures, suppressions) and per-frame provenance reach the sink's
/// GTS hooks; diagnostics reach [`GtsEventSink::gts_diagnostic`] and are also
/// returned in the [`StreamingReadResult`].
///
/// A structural decode failure (a genuinely dangling term id, a graph-scoped
/// reifier the protocol cannot carry, …) is returned as [`Err`] rather than
/// swallowed. When any sink call returns [`ControlFlow::Break`] the drive stops
/// emitting and, crucially, [`RdfEventSink::finish`] is NOT called — a cancelled
/// drive is never finalized.
///
/// # Errors
///
/// Returns the first [`EventError`] raised while resolving or emitting the
/// stream, or by finalizing the sink.
pub fn stream_events(
    data: &[u8],
    options: ReadOptions<'_>,
    sink: &mut dyn GtsEventSink,
) -> Result<StreamingReadResult, EventError> {
    let mut resolver = SegmentResolver::new(EventEmitter::new(sink));
    let read_result = read_to_sink_with_options(data, options, &mut resolver);
    if let Some(error) = resolver.take_error() {
        return Err(error);
    }
    resolver.finish()?;
    let mut emitter = resolver.into_sink();
    emitter.close_open_scope()?;
    if !emitter.cancelled {
        emitter.sink.finish()?;
    }
    Ok(read_result)
}
