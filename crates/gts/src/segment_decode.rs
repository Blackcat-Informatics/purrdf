// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//!    therefore arrives (in the `terms` frame) carrying only its `reifier` id —
//!    the `reifies` frame that binds that reifier to the triple's `(s, p, o)`
//!    arrives **later**. A single-pass fold that resolves a triple term the
//!    instant its `term` event fires cannot succeed.
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
//!    their now-complete `(s, p, o)` recursively, inner-first, depth-bounded by
//!    [`MAX_GTS_TERM_NESTING_DEPTH`] — then push the raw quad / reifier /
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

use ciborium::value::Value;

use crate::model::{
    Diagnostic, OpaqueNode, Quad, Signature, StreamableInfo, Suppression, Term, TermKind, Triple3,
};
use crate::reader::{FrameContext, StreamingSink};

/// A [`std::collections::HashMap`] keyed by the workspace's fixed-key `ahash`
/// policy (`crates/rdf-core/src/hash.rs`'s `FastHasher`) — no runtime RNG
/// seeding, so it stays wasm-clean. Iteration order is unspecified; every
/// order-sensitive read here goes through an explicit sort, never hash order.
type FastMap<K, V> =
    std::collections::HashMap<K, V, core::hash::BuildHasherDefault<ahash::AHasher>>;

/// Depth bound for resolving nested quoted-triple terms. A cyclic or absurdly
/// nested triple term hard-fails rather than recursing without bound. Mirrors
/// the same guard on the folded (`reader::read`) path.
pub const MAX_GTS_TERM_NESTING_DEPTH: usize = 16;

/// The three resolved components `(subject, predicate, object)` a reifier binds,
/// in the target's id space.
type ResolvedComponents<S> = (
    <S as ResolvedSink>::Id,
    <S as ResolvedSink>::Id,
    <S as ResolvedSink>::Id,
);

/// The emit target [`SegmentResolver`] drives once GTS segment-local ids have
/// been resolved to the target's own id space.
///
/// The resolver owns GTS-frame decode and per-segment two-phase resolution; the
/// target owns interning, row storage, and the mapping of the four structured
/// decode failures onto its own error type (usually enriched with location
/// context). Non-RDF events pass through with no-op defaults so a target that
/// only cares about triples need not implement them.
pub trait ResolvedSink {
    /// The target's resolved-term identifier. `Copy` so the resolver can cache
    /// it in its per-segment remap without cloning.
    type Id: Copy;
    /// The target's error type. Streaming callbacks latch the first one.
    type Error;

    /// Intern a resolved IRI term, returning its target id.
    fn intern_iri(
        &mut self,
        segment_index: usize,
        gts_id: usize,
        iri: &str,
    ) -> Result<Self::Id, Self::Error>;

    /// Intern a resolved blank-node term (already scoped by `segment_index`),
    /// returning its target id.
    fn intern_blank(
        &mut self,
        segment_index: usize,
        gts_id: usize,
        label: &str,
    ) -> Result<Self::Id, Self::Error>;

    /// Intern a resolved literal term, returning its target id. The datatype (if
    /// any) is already resolved to a target id; the target performs its own
    /// datatype-must-be-IRI check and base-direction parsing.
    fn intern_literal(
        &mut self,
        segment_index: usize,
        gts_id: usize,
        lexical: String,
        datatype: Option<Self::Id>,
        lang: Option<String>,
        direction: Option<String>,
    ) -> Result<Self::Id, Self::Error>;

    /// Intern a resolved quoted-triple term from its resolved components.
    fn intern_triple(
        &mut self,
        segment_index: usize,
        gts_id: usize,
        s: Self::Id,
        p: Self::Id,
        o: Self::Id,
    ) -> Result<Self::Id, Self::Error>;

    /// Push a resolved quad row (`g` is `None` for the default graph).
    fn push_quad(
        &mut self,
        segment_index: usize,
        s: Self::Id,
        p: Self::Id,
        o: Self::Id,
        g: Option<Self::Id>,
    ) -> Result<(), Self::Error>;

    /// Push a resolved reifier row binding `reifier` to the triple `(s, p, o)`
    /// in graph `g`.
    fn push_reifier(
        &mut self,
        segment_index: usize,
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
        segment_index: usize,
        reifier: Self::Id,
        p: Self::Id,
        o: Self::Id,
        g: Option<Self::Id>,
    ) -> Result<(), Self::Error>;

    /// Build the error for a gts id that no `term` event ever introduced —
    /// a genuinely dangling reference. `role` names the referencing position.
    fn err_dangling_term(&self, segment_index: usize, gts_id: usize, role: &str) -> Self::Error;

    /// Build the error for exceeding [`MAX_GTS_TERM_NESTING_DEPTH`] while
    /// resolving nested quoted triples.
    fn err_nesting_limit(&self, segment_index: usize, gts_id: usize) -> Self::Error;

    /// Build the error for a quoted-triple term that carries no reifier id.
    fn err_unbound_triple(&self, segment_index: usize, gts_id: usize) -> Self::Error;

    /// Build the error for a triple term whose reifier no `reifies` event ever
    /// bound.
    fn err_missing_reifier(&self, segment_index: usize, reifier: usize) -> Self::Error;

    /// Per-frame provenance passthrough (default no-op).
    fn frame(&mut self, _ctx: FrameContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Inline blob digest + declared metadata passthrough (default no-op).
    fn blob(
        &mut self,
        _segment_index: usize,
        _digest: &str,
        _meta: Option<&Value>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Opaque frame passthrough (default no-op).
    fn opaque(&mut self, _segment_index: usize, _node: &OpaqueNode) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Signature observation passthrough (default no-op).
    fn signature(&mut self, _segment_index: usize, _sig: &Signature) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Suppression directive passthrough (default no-op).
    fn suppression(
        &mut self,
        _segment_index: usize,
        _suppression: &Suppression,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Completed segment-head passthrough (default no-op).
    fn segment_head(&mut self, _segment_index: usize, _head: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Completed streamable-layout passthrough (default no-op).
    fn streamable_layout(
        &mut self,
        _segment_index: usize,
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
pub struct SegmentResolver<S: ResolvedSink> {
    /// The emit target resolved rows are pushed into.
    sink: S,
    /// RAW per-segment terms buffered during the streaming phase, keyed by
    /// `(segment_index, gts_id)`, resolved and drained at segment close.
    raw_terms: FastMap<(usize, usize), Term>,
    /// Per-segment memo from `(segment_index, gts_id)` to the target id,
    /// populated as the currently buffered segment's terms resolve (so a term
    /// referenced twice — e.g. as both a quad subject and a reifier subject —
    /// interns once) and cleared at the end of [`Self::resolve_buffered`]:
    /// segment-local ids never cross a segment boundary, so retaining this
    /// past segment close would grow it O(total terms across ALL segments).
    remaps: FastMap<(usize, usize), S::Id>,
    /// Per-segment reifier bindings `(segment_index, reifier) → (s, p, o)` gts
    /// ids, recorded from `reifier` events so a Triple term (any order) can
    /// recover its components.
    reifier_bindings: FastMap<(usize, usize), Triple3>,
    /// RAW quad rows `(segment_index, (s, p, o, g) gts ids)`.
    raw_quads: Vec<(usize, Quad)>,
    /// RAW reifier rows `(segment_index, reifier, (s, p, o), graph?)`.
    raw_reifiers: Vec<(usize, usize, Triple3, Option<usize>)>,
    /// RAW annotation rows `(segment_index, (r, p, v), graph?)`.
    raw_annotations: Vec<(usize, Triple3, Option<usize>)>,
    /// The segment currently being buffered; a higher incoming index flushes it.
    current_segment: Option<usize>,
    /// First latched error. Streaming callbacks are infallible, so a failure is
    /// parked here and surfaced after the reader returns.
    error: Option<S::Error>,
}

impl<S: ResolvedSink> std::fmt::Debug for SegmentResolver<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SegmentResolver")
            .field("buffered_terms", &self.raw_terms.len())
            .field("buffered_quads", &self.raw_quads.len())
            .field("buffered_reifiers", &self.raw_reifiers.len())
            .field("buffered_annotations", &self.raw_annotations.len())
            .field("buffered_remaps", &self.remaps.len())
            .field("buffered_reifier_bindings", &self.reifier_bindings.len())
            .field("current_segment", &self.current_segment)
            .field("has_error", &self.error.is_some())
            .finish_non_exhaustive()
    }
}

impl<S: ResolvedSink> SegmentResolver<S> {
    /// Wrap an emit target in a fresh resolver.
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            raw_terms: FastMap::default(),
            remaps: FastMap::default(),
            reifier_bindings: FastMap::default(),
            raw_quads: Vec::new(),
            raw_reifiers: Vec::new(),
            raw_annotations: Vec::new(),
            current_segment: None,
            error: None,
        }
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
        self.reifier_bindings.len()
    }

    /// Count of the per-segment gts-id → target-id remap memo currently
    /// retained (see the `remaps` field doc). Same bound as
    /// [`Self::buffered_reifier_binding_count`]: `0` immediately after every
    /// segment flush.
    pub fn buffered_remap_count(&self) -> usize {
        self.remaps.len()
    }

    /// Flush the final buffered segment, resolving its terms and pushing its
    /// rows. Call once after [`crate::reader::read_to_sink`] returns (and after
    /// checking [`Self::take_error`]).
    pub fn finish(&mut self) -> Result<(), S::Error> {
        self.resolve_buffered()
    }

    /// Record the first latched error; later errors do not overwrite it.
    fn fail(&mut self, error: S::Error) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    /// Note an incoming buffering event's segment. When it advances past the
    /// segment currently buffered, flush the completed segment (latching any
    /// error) before accepting the new one.
    fn advance_segment(&mut self, segment_index: usize) {
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
        let mut keys: Vec<(usize, usize)> = self.raw_terms.keys().copied().collect();
        keys.sort_unstable();
        for (segment_index, gts_id) in keys {
            self.resolve_term(segment_index, gts_id, "term", 0)?;
        }

        // Quads.
        let raw_quads = std::mem::take(&mut self.raw_quads);
        for (segment_index, (s, p, o, g)) in raw_quads {
            let s = self.resolve_term(segment_index, s, "quad subject", 0)?;
            let p = self.resolve_term(segment_index, p, "quad predicate", 0)?;
            let o = self.resolve_term(segment_index, o, "quad object", 0)?;
            let g = match g {
                Some(g) => Some(self.resolve_term(segment_index, g, "quad graph name", 0)?),
                None => None,
            };
            self.sink.push_quad(segment_index, s, p, o, g)?;
        }

        // Reifier bindings: bind the reifier resource to the triple.
        let raw_reifiers = std::mem::take(&mut self.raw_reifiers);
        for (segment_index, reifier, (s, p, o), graph) in raw_reifiers {
            let reifier_id = self.resolve_term(segment_index, reifier, "reifier", 0)?;
            let s = self.resolve_term(segment_index, s, "reified subject", 0)?;
            let p = self.resolve_term(segment_index, p, "reified predicate", 0)?;
            let o = self.resolve_term(segment_index, o, "reified object", 0)?;
            let g = match graph {
                Some(g) => Some(self.resolve_term(segment_index, g, "reifier graph name", 0)?),
                None => None,
            };
            self.sink
                .push_reifier(segment_index, reifier_id, s, p, o, g)?;
        }

        // Annotations `(reifier, predicate, object, graph?)`.
        let raw_annotations = std::mem::take(&mut self.raw_annotations);
        for (segment_index, (r, p, v), graph) in raw_annotations {
            let r = self.resolve_term(segment_index, r, "annotation reifier", 0)?;
            let p = self.resolve_term(segment_index, p, "annotation predicate", 0)?;
            let v = self.resolve_term(segment_index, v, "annotation object", 0)?;
            let g = match graph {
                Some(g) => Some(self.resolve_term(segment_index, g, "annotation graph name", 0)?),
                None => None,
            };
            self.sink.push_annotation(segment_index, r, p, v, g)?;
        }

        // Bounded-memory streaming fold: both maps are segment-local — no
        // resolution ever crosses a segment boundary (reifier bindings, and
        // the term-id remap memo, are only ever read for the segment that is
        // currently buffered) — so drop them here rather than retaining them
        // for the lifetime of the resolver. Without this, both grow O(total
        // reifiers/terms across ALL segments) instead of O(one segment).
        self.reifier_bindings.clear();
        self.remaps.clear();

        Ok(())
    }

    /// Resolve a GTS segment-local term id to its target id, interning it (and
    /// any nested quoted triples) on demand. Memoized through `remaps` and
    /// depth-bounded against cyclic quoted triples. A gts id no `term` event
    /// introduced is a genuinely dangling reference and hence an [`Err`].
    fn resolve_term(
        &mut self,
        segment_index: usize,
        gts_id: usize,
        role: &str,
        depth: usize,
    ) -> Result<S::Id, S::Error> {
        if let Some(&id) = self.remaps.get(&(segment_index, gts_id)) {
            return Ok(id);
        }
        if depth > MAX_GTS_TERM_NESTING_DEPTH {
            return Err(self.sink.err_nesting_limit(segment_index, gts_id));
        }
        // MOVE the raw term out: it resolves at most once (every later reference
        // hits the `remaps` cache above), so taking ownership lets interning
        // borrow the target mutably without a conflict AND without cloning the
        // term's owned strings. A (pathological) cyclic reference now surfaces as
        // a dangling-term ref — the term is already removed — rather than the
        // nesting-limit; both are hard failures and a GTS Writer never emits
        // cycles.
        let Some(term) = self.raw_terms.remove(&(segment_index, gts_id)) else {
            return Err(self.sink.err_dangling_term(segment_index, gts_id, role));
        };
        let our_id = self.intern_raw_term(segment_index, gts_id, term, depth)?;
        self.remaps.insert((segment_index, gts_id), our_id);
        Ok(our_id)
    }

    /// Intern one already-located raw term, recursing (inner-first) for quoted
    /// triples through their reifier binding. Takes the [`Term`] BY VALUE (it was
    /// just removed from `raw_terms`) and MOVES its owned strings into the target.
    fn intern_raw_term(
        &mut self,
        segment_index: usize,
        gts_id: usize,
        term: Term,
        depth: usize,
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
                let label = term
                    .value
                    .unwrap_or_else(|| format!("gts_bnode_{segment_index}_{gts_id}"));
                self.sink.intern_blank(segment_index, gts_id, &label)?
            }
            TermKind::Literal => {
                let datatype = match term.datatype {
                    Some(dt_gts_id) => Some(self.resolve_term(
                        segment_index,
                        dt_gts_id,
                        "literal datatype",
                        depth + 1,
                    )?),
                    None => None,
                };
                self.sink.intern_literal(
                    segment_index,
                    gts_id,
                    term.value.unwrap_or_default(),
                    datatype,
                    term.lang,
                    term.direction,
                )?
            }
            TermKind::Triple => {
                let Some(reifier_gts_id) = term.reifier else {
                    return Err(self.sink.err_unbound_triple(segment_index, gts_id));
                };
                let (s, p, o) =
                    self.resolve_triple_components(segment_index, reifier_gts_id, depth + 1)?;
                self.sink.intern_triple(segment_index, gts_id, s, p, o)?
            }
        };
        Ok(our_id)
    }

    /// Resolve the `(s, p, o)` of the triple a reifier binds, THROUGH this
    /// segment's terms, depth-bounded against cyclic quoted triples. The reifier
    /// binding MUST exist by phase 2; a missing one is a genuine dangling
    /// reference and hence an [`Err`].
    fn resolve_triple_components(
        &mut self,
        segment_index: usize,
        reifier: usize,
        depth: usize,
    ) -> Result<ResolvedComponents<S>, S::Error> {
        if depth > MAX_GTS_TERM_NESTING_DEPTH {
            return Err(self.sink.err_nesting_limit(segment_index, reifier));
        }
        let Some(&(s, p, o)) = self.reifier_bindings.get(&(segment_index, reifier)) else {
            return Err(self.sink.err_missing_reifier(segment_index, reifier));
        };
        let s = self.resolve_term(segment_index, s, "reified subject", depth)?;
        let p = self.resolve_term(segment_index, p, "reified predicate", depth)?;
        let o = self.resolve_term(segment_index, o, "reified object", depth)?;
        Ok((s, p, o))
    }
}

impl<S: ResolvedSink> StreamingSink for SegmentResolver<S> {
    fn frame(&mut self, ctx: FrameContext<'_>) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self.sink.frame(ctx) {
            self.fail(error);
        }
    }

    fn term(&mut self, segment_index: usize, term_id: usize, term: &Term) {
        if self.error.is_some() {
            return;
        }
        self.advance_segment(segment_index);
        if self.error.is_some() {
            return;
        }
        // Streaming phase: stash the raw term verbatim. A quoted-triple term
        // cannot be resolved yet — its reifier binding may arrive in a later
        // frame — so we defer ALL resolution to segment close.
        self.raw_terms
            .insert((segment_index, term_id), term.clone());
    }

    fn quad(&mut self, segment_index: usize, quad: Quad) {
        if self.error.is_some() {
            return;
        }
        self.advance_segment(segment_index);
        if self.error.is_some() {
            return;
        }
        self.raw_quads.push((segment_index, quad));
    }

    fn reifier(&mut self, segment_index: usize, reifier: crate::model::ReifierRow) {
        if self.error.is_some() {
            return;
        }
        self.advance_segment(segment_index);
        if self.error.is_some() {
            return;
        }
        // Row-array `(reifier_id, (s, p, o), graph?)`. Record the reifier → (s,
        // p, o) binding so a Triple term (any order) can resolve its components,
        // and stash the row (with its named graph) so the reifier resource binds
        // the resolved triple at segment close.
        let (reifier_id, triple, graph) = reifier;
        self.reifier_bindings
            .insert((segment_index, reifier_id), triple);
        self.raw_reifiers
            .push((segment_index, reifier_id, triple, graph));
    }

    fn annotation(&mut self, segment_index: usize, annotation: crate::model::AnnotationRow) {
        if self.error.is_some() {
            return;
        }
        self.advance_segment(segment_index);
        if self.error.is_some() {
            return;
        }
        // Row-array `(reifier, predicate, value, graph?)`.
        let (reifier, predicate, value, graph) = annotation;
        self.raw_annotations
            .push((segment_index, (reifier, predicate, value), graph));
    }

    fn suppression(&mut self, segment_index: usize, suppression: &Suppression) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self.sink.suppression(segment_index, suppression) {
            self.fail(error);
        }
    }

    fn blob(&mut self, segment_index: usize, digest: &str, meta: Option<&Value>) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self.sink.blob(segment_index, digest, meta) {
            self.fail(error);
        }
    }

    fn opaque(&mut self, segment_index: usize, opaque: &OpaqueNode) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self.sink.opaque(segment_index, opaque) {
            self.fail(error);
        }
    }

    fn signature(&mut self, segment_index: usize, signature: &Signature) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self.sink.signature(segment_index, signature) {
            self.fail(error);
        }
    }

    fn diagnostic(&mut self, diagnostic: &Diagnostic) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self.sink.diagnostic(diagnostic) {
            self.fail(error);
        }
    }

    fn segment_head(&mut self, segment_index: usize, head: &[u8]) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self.sink.segment_head(segment_index, head) {
            self.fail(error);
        }
    }

    fn streamable_layout(&mut self, segment_index: usize, info: &StreamableInfo) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self.sink.streamable_layout(segment_index, info) {
            self.fail(error);
        }
    }
}
