// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The permissive-ingestion bridge (purrdf P6, #840): wire the immutable IR to the
//! dependency-free `purrdf-events` protocol.
//!
//! Two pieces live here:
//!
//! * [`DatasetSink`] — an [`RdfEventSink`] that folds
//!   an external event stream into a frozen [`RdfDataset`]. It is **two-phase**, the
//!   same shape as the proven [`super::import_sink`] GTS importer: the streaming
//!   phase buffers raw [`EventTermId`] → term
//!   declarations and raw quad / reifier / annotation rows verbatim (resolving
//!   nothing, so a forward reference is fine), and [`finish`](DatasetSink::finish)
//!   resolves every id into a fresh [`RdfDatasetBuilder`] inner-first (depth-bounded
//!   by [`purrdf_events::MAX_TERM_NESTING_DEPTH`]) and HARD-fails on any id still
//!   undeclared.
//!
//! * [`FrozenDatasetSource`] — an
//!   [`RdfEventSource`] that replays an
//!   already-frozen `&RdfDataset` *into* any sink: it declares a `term` event for
//!   every term in [`TermId`] order (so it declares-before-reference), then the quad
//!   / reifier / annotation events. This is the in-repo source that lets P6 be tested
//!   end-to-end without the cross-repo GTS source (deferred, purrdf-gts#249).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use core::ops::ControlFlow;

use purrdf_events::{
    EventError, EventQuad, EventTerm, EventTermId, EventTriple, RdfEventSink, RdfEventSource,
    ScopeId, TextDirection, MAX_TERM_NESTING_DEPTH,
};

use super::builder::RdfDatasetBuilder;
use super::dataset::{RdfDataset, TermRef};
use super::term::{BlankScope, TermId};
use crate::{RdfLiteral, RdfTextDirection};

/// A buffered term declaration, owned so it survives until phase-2 resolution. The
/// borrowed [`EventTerm`] strings are copied into owned form on receipt, because the
/// source's borrow does not outlive the `term` call.
#[derive(Clone, Debug)]
enum RawTerm {
    Iri(String),
    Blank {
        label: String,
        scope: ScopeId,
    },
    Literal {
        lexical: String,
        datatype: String,
        language: Option<String>,
        direction: Option<TextDirection>,
    },
    Triple(EventTriple),
}

impl RawTerm {
    /// Copy a borrowed [`EventTerm`] into an owned [`RawTerm`].
    fn from_event(term: EventTerm<'_>) -> Self {
        match term {
            EventTerm::Iri(iri) => RawTerm::Iri(iri.to_owned()),
            EventTerm::Blank { label, scope } => RawTerm::Blank {
                label: label.to_owned(),
                scope,
            },
            EventTerm::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => RawTerm::Literal {
                lexical: lexical.to_owned(),
                datatype: datatype.to_owned(),
                language: language.map(str::to_owned),
                direction,
            },
            EventTerm::Triple(triple) => RawTerm::Triple(triple),
        }
    }
}

/// Map the protocol's [`TextDirection`] onto the IR's [`RdfTextDirection`].
fn map_direction(direction: Option<TextDirection>) -> Option<RdfTextDirection> {
    direction.map(|d| match d {
        TextDirection::Ltr => RdfTextDirection::Ltr,
        TextDirection::Rtl => RdfTextDirection::Rtl,
    })
}

/// An [`RdfEventSink`] that folds a permissive ingestion event stream into a frozen
/// [`RdfDataset`], tolerant of forward references (two-phase; see the module docs).
pub struct DatasetSink {
    /// RAW term declarations recorded during the streaming phase, keyed by
    /// [`EventTermId`]. A triple term stashes its component ids verbatim; resolution
    /// (which may follow a forward reference) is deferred to [`finish`](Self::finish).
    raw_terms: HashMap<EventTermId, RawTerm>,
    /// The scope each [`EventTermId`] was declared under, for the redeclaration check
    /// and the closed-scope guard.
    declared_in: HashMap<EventTermId, ScopeId>,
    /// Phase-2 memo: EventTermId → the interned [`TermId`]. A successfully resolved id
    /// is recorded here so later references hit the memo instead of re-resolving.
    remaps: HashMap<EventTermId, TermId>,
    /// Phase-2 in-progress guard: the set of [`EventTermId`]s currently mid-resolution
    /// (their nested components are still being resolved). Re-entering an id already in
    /// this set is a cyclic triple term ([`EventError::CyclicTerm`]) — distinct from a
    /// genuinely never-declared id ([`EventError::Unresolved`]). A raw term is removed
    /// from `raw_terms` only AFTER its components resolve, so a self/transitive cycle
    /// trips this guard rather than reading as a (removed-therefore-)missing term.
    resolving: HashSet<EventTermId>,
    /// RAW quad rows, resolved in phase 2.
    raw_quads: Vec<EventQuad>,
    /// RAW reifier bindings `(reifier id, triple)`, resolved in phase 2.
    raw_reifiers: Vec<(EventTermId, EventTriple)>,
    /// RAW annotation rows `(reifier, predicate, object)`, resolved in phase 2.
    raw_annotations: Vec<(EventTermId, EventTermId, EventTermId)>,
    /// Open scopes. [`ScopeId::DEFAULT`] is always open; [`open_scope`](Self::open_scope)
    /// adds more, [`close_scope`](Self::close_scope) removes (seals) them. Openness is
    /// determined solely by membership here, so a sealed OR never-opened scope id both
    /// read as "not open".
    open_scopes: Vec<ScopeId>,
    /// The next scope ordinal to mint.
    next_scope: u32,
    /// Set once a cancellation ([`ControlFlow::Break`]) is observed: a cancelled sink
    /// MUST NOT freeze.
    cancelled: bool,
    /// The frozen dataset, available after a successful [`finish`](Self::finish).
    frozen: Option<Arc<RdfDataset>>,
    /// Builder used during phase-2 resolution; an `Option` so `finish` can take it.
    builder: Option<RdfDatasetBuilder>,
}

impl Default for DatasetSink {
    /// The default sink is identical to [`new`](Self::new): the default scope
    /// ([`ScopeId::DEFAULT`]) is open from the start. This is implemented MANUALLY
    /// (rather than `#[derive]`d) because a derived `Default` would leave
    /// `open_scopes` empty, so `ScopeId::DEFAULT` would not be considered open — a
    /// latent bug. Keeping `new`/`default` in lock-step (one delegates to the other)
    /// ensures the two initial states can never diverge.
    fn default() -> Self {
        Self {
            raw_terms: HashMap::new(),
            declared_in: HashMap::new(),
            remaps: HashMap::new(),
            resolving: HashSet::new(),
            raw_quads: Vec::new(),
            raw_reifiers: Vec::new(),
            raw_annotations: Vec::new(),
            // The default scope is open from the start (see the doc comment above).
            open_scopes: vec![ScopeId::DEFAULT],
            next_scope: 0,
            cancelled: false,
            frozen: None,
            builder: None,
        }
    }
}

impl DatasetSink {
    /// A fresh sink with the default scope open. Delegates to [`Default`] so the two
    /// can never diverge.
    pub fn new() -> Self {
        Self::default()
    }

    /// The frozen dataset produced by a successful [`finish`](RdfEventSink::finish).
    /// `None` before `finish` or after a cancelled drive.
    pub fn into_dataset(self) -> Option<Arc<RdfDataset>> {
        self.frozen
    }

    /// Borrow the frozen dataset, if any.
    pub fn dataset(&self) -> Option<&Arc<RdfDataset>> {
        self.frozen.as_ref()
    }

    /// Validate that `scope` is currently open. Returns
    /// [`EventError::ClosedScope`] if the scope is not in `open_scopes` — i.e. it has
    /// either been sealed via [`close_scope`](Self::close_scope) OR was never opened
    /// at all (referencing a closed scope's id is a protocol error per the spec).
    /// [`ScopeId::DEFAULT`] is open by default because `new`/`default` seed it.
    fn ensure_scope_open(&self, scope: ScopeId) -> Result<(), EventError> {
        if !self.open_scopes.contains(&scope) {
            return Err(EventError::ClosedScope { scope });
        }
        Ok(())
    }

    /// Phase-2 primitive: resolve an [`EventTermId`] to its interned [`TermId`],
    /// resolving (and interning) it on demand. Three failure modes are kept distinct:
    /// a never-declared id is [`EventError::Unresolved`]; an over-deep but acyclic
    /// triple-term chain is [`EventError::NestingDepthExceeded`]; and a self/transitive
    /// cycle is [`EventError::CyclicTerm`], caught by the in-progress guard below.
    fn resolve(&mut self, id: EventTermId, depth: usize) -> Result<TermId, EventError> {
        if let Some(&existing) = self.remaps.get(&id) {
            return Ok(existing);
        }
        if depth > MAX_TERM_NESTING_DEPTH {
            return Err(EventError::NestingDepthExceeded { id });
        }
        // In-progress guard: if this id is already being resolved further up the stack,
        // its components reference itself — a cyclic triple term. Report it as such
        // rather than as a missing declaration.
        if !self.resolving.insert(id) {
            return Err(EventError::CyclicTerm { id });
        }
        // BORROW (clone) the raw term, leaving it in `raw_terms` until its components
        // resolve, so a re-entry hits the in-progress guard above (not a missing term).
        // A genuinely never-declared id lands here as `Unresolved`.
        let Some(raw) = self.raw_terms.get(&id).cloned() else {
            self.resolving.remove(&id);
            return Err(EventError::Unresolved { id });
        };
        let resolved = self.intern_raw(raw, depth);
        self.resolving.remove(&id);
        let our_id = resolved?;
        // Resolution succeeded: drop the raw term (it resolves at most once) and memo.
        self.raw_terms.remove(&id);
        self.remaps.insert(id, our_id);
        Ok(our_id)
    }

    /// Intern one already-located raw term, recursing inner-first for triple terms.
    ///
    /// The `builder` borrow is taken *inside* each arm that actually mutates it,
    /// never across the whole `match`. This keeps the borrow structure explicit and
    /// robust: the `Triple` arm needs `&mut self` (to recurse through `resolve`)
    /// before it touches the builder, so a single match-wide `&mut self.builder`
    /// would only compile thanks to NLL dropping it — fragile under refactor.
    fn intern_raw(&mut self, raw: RawTerm, depth: usize) -> Result<TermId, EventError> {
        let our_id = match raw {
            RawTerm::Iri(iri) => self.builder_mut().intern_iri(iri),
            RawTerm::Blank { label, scope } => {
                // Scope 0 (DEFAULT) maps to the IR default scope; a protocol scope `n`
                // maps to IR `BlankScope(n)` so same-label blanks in different scopes
                // intern to DISTINCT ids (mirrors GTS per-segment scope).
                self.builder_mut().intern_blank(label, BlankScope(scope.0))
            }
            RawTerm::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                // Ill-typed lexical forms are preserved verbatim — interning never
                // rejects them (the protocol flags, never auto-rejects).
                let literal = RdfLiteral {
                    lexical_form: lexical,
                    // A language tag forces rdf:langString at intern time (C0.1); an
                    // explicit datatype is otherwise carried through by value.
                    datatype: if language.is_some() {
                        None
                    } else {
                        Some(datatype)
                    },
                    language,
                    direction: map_direction(direction),
                };
                self.builder_mut().intern_literal(literal)
            }
            RawTerm::Triple(EventTriple { s, p, o }) => {
                // Resolve the components first (needs `&mut self`), THEN borrow the
                // builder — the two borrows never overlap.
                let s = self.resolve(s, depth + 1)?;
                let p = self.resolve(p, depth + 1)?;
                let o = self.resolve(o, depth + 1)?;
                self.builder_mut().intern_triple(s, p, o)
            }
        };
        Ok(our_id)
    }

    /// Mutably borrow the phase-2 builder, which is present for the whole of
    /// [`finish`](RdfEventSink::finish).
    fn builder_mut(&mut self) -> &mut RdfDatasetBuilder {
        self.builder
            .as_mut()
            .expect("builder present during resolution")
    }
}

impl RdfEventSink for DatasetSink {
    fn term(
        &mut self,
        id: EventTermId,
        term: EventTerm<'_>,
    ) -> Result<ControlFlow<()>, EventError> {
        // Redeclaration of the same id while its declaring scope is still open is an
        // error (no last-writer-wins). Single lookup: fetch the prior scope and bail.
        if let Some(&scope) = self.declared_in.get(&id) {
            return Err(EventError::RedeclaredId { id, scope });
        }
        // A blank node's scope must be open at declaration time.
        let scope = match &term {
            EventTerm::Blank { scope, .. } => *scope,
            _ => ScopeId::DEFAULT,
        };
        self.ensure_scope_open(scope)?;
        self.declared_in.insert(id, scope);
        self.raw_terms.insert(id, RawTerm::from_event(term));
        Ok(ControlFlow::Continue(()))
    }

    fn quad(&mut self, q: EventQuad) -> Result<ControlFlow<()>, EventError> {
        self.raw_quads.push(q);
        Ok(ControlFlow::Continue(()))
    }

    fn reifier(
        &mut self,
        reifier: EventTermId,
        triple: EventTriple,
    ) -> Result<ControlFlow<()>, EventError> {
        self.raw_reifiers.push((reifier, triple));
        Ok(ControlFlow::Continue(()))
    }

    fn annotation(
        &mut self,
        reifier: EventTermId,
        p: EventTermId,
        o: EventTermId,
    ) -> Result<ControlFlow<()>, EventError> {
        self.raw_annotations.push((reifier, p, o));
        Ok(ControlFlow::Continue(()))
    }

    fn open_scope(&mut self) -> Result<ScopeId, EventError> {
        // Hard-fail on ordinal exhaustion rather than wrapping (no degraded fallback).
        let next = self.next_scope.checked_add(1).ok_or_else(|| {
            EventError::message("scope ordinal space exhausted (u32::MAX scopes opened)")
        })?;
        self.next_scope = next;
        let scope = ScopeId(next);
        self.open_scopes.push(scope);
        Ok(scope)
    }

    fn close_scope(&mut self, scope: ScopeId) -> Result<ControlFlow<()>, EventError> {
        // `close_scope` is a real lifecycle op, not a silent no-op. The default scope
        // cannot be closed, and a scope that was never opened (or is already closed —
        // both read as "not in open_scopes") cannot be closed either: each is a
        // ClosedScope protocol error. A valid close removes the scope from the open
        // set, after which a later blank declaration under it reads as "not open"
        // (see `ensure_scope_open`).
        if scope == ScopeId::DEFAULT || !self.open_scopes.contains(&scope) {
            return Err(EventError::ClosedScope { scope });
        }
        self.open_scopes.retain(|&s| s != scope);
        Ok(ControlFlow::Continue(()))
    }

    fn finish(&mut self) -> Result<(), EventError> {
        // Cancellation guard: a sink that observed a Break MUST NOT freeze. A source
        // honoring the protocol never calls finish after a Break, but defend anyway.
        if self.cancelled {
            return Err(EventError::message(
                "finish called after a cancelled (ControlFlow::Break) drive",
            ));
        }

        self.builder = Some(RdfDatasetBuilder::new());

        // Resolve every declared term (idempotent via `remaps`), in a deterministic
        // id order so the interner's allocation order is reproducible.
        let mut ids: Vec<EventTermId> = self.raw_terms.keys().copied().collect();
        ids.sort_unstable();
        for id in ids {
            self.resolve(id, 0)?;
        }

        // Quads.
        let raw_quads = std::mem::take(&mut self.raw_quads);
        for q in raw_quads {
            let s = self.resolve(q.s, 0)?;
            let p = self.resolve(q.p, 0)?;
            let o = self.resolve(q.o, 0)?;
            let g = match q.g {
                Some(g) => Some(self.resolve(g, 0)?),
                None => None,
            };
            self.builder
                .as_mut()
                .expect("builder present")
                .push_quad(s, p, o, g);
        }

        // Reifier bindings: bind the reifier resource to the interned triple term.
        let raw_reifiers = std::mem::take(&mut self.raw_reifiers);
        for (reifier, EventTriple { s, p, o }) in raw_reifiers {
            let reifier_id = self.resolve(reifier, 0)?;
            let s = self.resolve(s, 0)?;
            let p = self.resolve(p, 0)?;
            let o = self.resolve(o, 0)?;
            let builder = self.builder.as_mut().expect("builder present");
            let triple_term = builder.intern_triple(s, p, o);
            builder.push_reifier(reifier_id, triple_term);
        }

        // Annotations `(reifier, predicate, object)`.
        let raw_annotations = std::mem::take(&mut self.raw_annotations);
        for (r, p, v) in raw_annotations {
            let r = self.resolve(r, 0)?;
            let p = self.resolve(p, 0)?;
            let v = self.resolve(v, 0)?;
            self.builder
                .as_mut()
                .expect("builder present")
                .push_annotation(r, p, v);
        }

        let builder = self.builder.take().expect("builder present");
        let dataset = builder
            .freeze()
            .map_err(|diagnostic| EventError::message(format!("freeze failed: {diagnostic}")))?;
        self.frozen = Some(dataset);
        Ok(())
    }
}

/// A wrapper a source can use to observe cancellation: any sink that wants the
/// "do-not-freeze on Break" guarantee tracks it. [`DatasetSink`] does so via this
/// helper invoked by [`FrozenDatasetSource::drive`]; an external source signals a
/// Break to the sink and the sink records it.
impl DatasetSink {
    /// Mark that a [`ControlFlow::Break`] was observed during the drive so a later
    /// [`finish`](RdfEventSink::finish) refuses to freeze. A driving
    /// [`RdfEventSource`] calls this when it stops early.
    pub fn mark_cancelled(&mut self) {
        self.cancelled = true;
    }
}

/// An [`RdfEventSource`] that replays an already-frozen [`RdfDataset`] *into* any
/// [`RdfEventSink`]: a `term` event per term in [`TermId`] order (declares-before-
/// reference), then quad / reifier / annotation events.
pub struct FrozenDatasetSource<'a> {
    dataset: &'a RdfDataset,
}

impl<'a> FrozenDatasetSource<'a> {
    /// Wrap a frozen dataset as an ingestion source.
    pub fn new(dataset: &'a RdfDataset) -> Self {
        Self { dataset }
    }

    /// Emit one term declaration, translating the IR's [`TermRef`] into the protocol's
    /// [`EventTerm`]. Returns the sink's control flow.
    fn emit_term<S: RdfEventSink + ?Sized>(
        &self,
        sink: &mut S,
        id: TermId,
    ) -> Result<ControlFlow<()>, EventError> {
        let event_id = EventTermId(id.index() as u32);
        match self.dataset.resolve(id) {
            TermRef::Iri(iri) => sink.term(event_id, EventTerm::Iri(iri)),
            TermRef::Blank { label, scope } => sink.term(
                event_id,
                EventTerm::Blank {
                    label,
                    scope: ScopeId(scope.ordinal()),
                },
            ),
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                // The datatype is an interned IRI term in the dataset; resolve it to
                // its IRI string so the protocol carries the datatype by value.
                let datatype_iri = match self.dataset.resolve(datatype) {
                    TermRef::Iri(iri) => iri,
                    _ => {
                        return Err(EventError::message(
                            "literal datatype did not resolve to an IRI",
                        ))
                    }
                };
                sink.term(
                    event_id,
                    EventTerm::Literal {
                        lexical,
                        datatype: datatype_iri,
                        language,
                        direction: direction.map(|d| match d {
                            RdfTextDirection::Ltr => TextDirection::Ltr,
                            RdfTextDirection::Rtl => TextDirection::Rtl,
                        }),
                    },
                )
            }
            TermRef::Triple { s, p, o } => sink.term(
                event_id,
                EventTerm::Triple(EventTriple {
                    s: EventTermId(s.index() as u32),
                    p: EventTermId(p.index() as u32),
                    o: EventTermId(o.index() as u32),
                }),
            ),
        }
    }
}

impl RdfEventSource for FrozenDatasetSource<'_> {
    fn drive<S: RdfEventSink + ?Sized>(&self, sink: &mut S) -> Result<(), EventError> {
        // Open every non-default blank scope this dataset uses BEFORE declaring any
        // blank under it: the sink's `ensure_scope_open` guard rejects a blank whose
        // scope was never opened. `open_scope` mints sequential ordinals (1, 2, …),
        // so opening `max_scope` times yields exactly the ids ScopeId(1..=max_scope),
        // matching the blank ordinals carried by value. ScopeId::DEFAULT (0) is open
        // from the start and is never minted here.
        let max_scope = (0..self.dataset.term_count())
            .filter_map(
                |i| match self.dataset.resolve(TermId::from_index(i as u32)) {
                    TermRef::Blank { scope, .. } => Some(scope.ordinal()),
                    _ => None,
                },
            )
            .max()
            .unwrap_or(0);
        for _ in 0..max_scope {
            sink.open_scope()?;
        }

        // Terms first, in ascending id order — a triple term's components (lower ids)
        // and a literal's datatype are declared before the term referencing them.
        for i in 0..self.dataset.term_count() {
            let id = TermId::from_index(i as u32);
            if let ControlFlow::Break(()) = self.emit_term(sink, id)? {
                return Ok(());
            }
        }
        for quad in self.dataset.quads() {
            let event = EventQuad {
                s: EventTermId(quad.s.index() as u32),
                p: EventTermId(quad.p.index() as u32),
                o: EventTermId(quad.o.index() as u32),
                g: quad.g.map(|g| EventTermId(g.index() as u32)),
            };
            if let ControlFlow::Break(()) = sink.quad(event)? {
                return Ok(());
            }
        }
        for (reifier, triple) in self.dataset.reifiers() {
            // Resolve the bound triple term to its (s, p, o) so the protocol carries a
            // reified statement, not an id-to-id binding.
            let TermRef::Triple { s, p, o } = self.dataset.resolve(triple) else {
                return Err(EventError::message("reifier did not bind a triple term"));
            };
            let event = EventTriple {
                s: EventTermId(s.index() as u32),
                p: EventTermId(p.index() as u32),
                o: EventTermId(o.index() as u32),
            };
            if let ControlFlow::Break(()) =
                sink.reifier(EventTermId(reifier.index() as u32), event)?
            {
                return Ok(());
            }
        }
        for (reifier, p, o) in self.dataset.annotations() {
            if let ControlFlow::Break(()) = sink.annotation(
                EventTermId(reifier.index() as u32),
                EventTermId(p.index() as u32),
                EventTermId(o.index() as u32),
            )? {
                return Ok(());
            }
        }
        sink.finish()
    }

    /// The frozen-IR replay declares every term in id order before referencing it.
    fn declares_before_reference(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::compare::datasets_isomorphic;
    use crate::RdfLiteral;
    use std::collections::HashSet;

    fn iri(b: &mut RdfDatasetBuilder, n: &str) -> TermId {
        b.intern_iri(format!("http://example.org/{n}"))
    }

    /// Declare a term on the sink, asserting it did not cancel — keeps the tests free
    /// of `must_use` `ControlFlow` discards while still proving each event continued.
    fn decl(sink: &mut DatasetSink, id: EventTermId, term: EventTerm<'_>) {
        let flow = sink.term(id, term).expect("term declaration ok");
        assert_eq!(flow, ControlFlow::Continue(()));
    }

    /// Buffer a quad on the sink, asserting it continued.
    fn push(sink: &mut DatasetSink, q: EventQuad) {
        let flow = sink.quad(q).expect("quad buffered");
        assert_eq!(flow, ControlFlow::Continue(()));
    }

    /// Build a non-trivial dataset: IRIs, blanks across two scopes, a typed literal, a
    /// language literal, a named graph, a nested triple term, reifiers + annotations.
    fn build_fixture() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = iri(&mut b, "s");
        let p = iri(&mut b, "p");
        let o = iri(&mut b, "o");
        let g = iri(&mut b, "g");
        let b0 = b.intern_blank("x".to_owned(), BlankScope(0));
        let b1 = b.intern_blank("x".to_owned(), BlankScope(1));
        let typed = b.intern_literal(RdfLiteral::typed(
            "42",
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        let lang = b.intern_literal(RdfLiteral::language_tagged("hi", "EN"));
        // Inner triple term <<s p o>> and outer <<s asserts <<s p o>>>>.
        let inner = b.intern_triple(s, p, o);
        let asserts = iri(&mut b, "asserts");
        let outer = b.intern_triple(s, asserts, inner);
        let r = iri(&mut b, "r");
        let ann_p = iri(&mut b, "annp");

        b.push_quad(s, p, o, None);
        b.push_quad(s, p, b0, None);
        b.push_quad(s, p, b1, Some(g));
        b.push_quad(s, p, typed, None);
        b.push_quad(s, p, lang, None);
        b.push_quad(s, asserts, outer, None);
        b.push_reifier(r, inner);
        b.push_annotation(r, ann_p, o);

        b.freeze().expect("fixture freezes")
    }

    /// Quad/reifier/annotation value triples for an equality oracle that is robust to
    /// term-id renumbering.
    fn quad_values(ds: &RdfDataset) -> HashSet<String> {
        ds.quad_refs().map(|q| format!("{q:?}")).collect()
    }

    #[test]
    fn round_trip_replay_into_sink_equals_original() {
        let original = build_fixture();
        let source = FrozenDatasetSource::new(&original);
        let mut sink = DatasetSink::new();
        source.drive(&mut sink).expect("drive ok");
        let rebuilt = sink.into_dataset().expect("frozen after finish");

        assert!(
            datasets_isomorphic(&original, &rebuilt),
            "replayed dataset is isomorphic to the original"
        );
        assert_eq!(original.quad_count(), rebuilt.quad_count());
        assert_eq!(
            original.reifiers().count(),
            rebuilt.reifiers().count(),
            "reifier count preserved"
        );
        assert_eq!(
            original.annotations().count(),
            rebuilt.annotations().count(),
            "annotation count preserved"
        );
        assert_eq!(
            quad_values(&original),
            quad_values(&rebuilt),
            "quad value sets match"
        );
    }

    #[test]
    fn object_safe_erased_drive_path() {
        let original = build_fixture();
        let source = FrozenDatasetSource::new(&original);
        let mut sink = DatasetSink::new();
        let dynamic: &mut dyn RdfEventSink = &mut sink;
        source.drive_erased(dynamic).expect("erased drive ok");
        let rebuilt = sink.into_dataset().expect("frozen after finish");
        assert!(datasets_isomorphic(&original, &rebuilt));
        assert!(
            source.declares_before_reference(),
            "frozen replay declares-before-reference"
        );
    }

    #[test]
    fn forward_reference_resolves_at_finish() {
        // A quad references id 0/1/2 BEFORE any of them is declared.
        let mut sink = DatasetSink::new();
        let (s, p, o) = (EventTermId(0), EventTermId(1), EventTermId(2));
        push(&mut sink, EventQuad { s, p, o, g: None });
        // Now declare them, out of order.
        decl(&mut sink, o, EventTerm::Iri("http://example.org/o"));
        decl(&mut sink, s, EventTerm::Iri("http://example.org/s"));
        decl(&mut sink, p, EventTerm::Iri("http://example.org/p"));
        sink.finish().expect("forward refs resolve at finish");
        let ds = sink.into_dataset().expect("frozen");
        assert_eq!(ds.quad_count(), 1);
    }

    #[test]
    fn unresolved_id_at_finish_is_error() {
        let mut sink = DatasetSink::new();
        let (s, p, o) = (EventTermId(0), EventTermId(1), EventTermId(2));
        decl(&mut sink, s, EventTerm::Iri("http://example.org/s"));
        decl(&mut sink, p, EventTerm::Iri("http://example.org/p"));
        // o (id 2) is referenced by the quad but NEVER declared.
        push(&mut sink, EventQuad { s, p, o, g: None });
        let err = sink.finish().expect_err("unresolved id must fail");
        assert_eq!(err, EventError::Unresolved { id: o });
        // And nothing was frozen.
        assert!(sink.dataset().is_none(), "no freeze on unresolved id");
    }

    #[test]
    fn redeclaration_in_one_scope_is_error() {
        let mut sink = DatasetSink::new();
        let id = EventTermId(0);
        decl(&mut sink, id, EventTerm::Iri("http://example.org/s"));
        let err = sink
            .term(id, EventTerm::Iri("http://example.org/other"))
            .expect_err("redeclaration must fail");
        assert_eq!(
            err,
            EventError::RedeclaredId {
                id,
                scope: ScopeId::DEFAULT
            }
        );
    }

    #[test]
    fn closed_scope_reference_is_error() {
        let mut sink = DatasetSink::new();
        let scope = sink.open_scope().expect("open scope");
        let _ = sink.close_scope(scope).expect("close scope");
        // Declaring a blank under the now-closed scope is an error.
        let err = sink
            .term(EventTermId(0), EventTerm::Blank { label: "b", scope })
            .expect_err("closed-scope reference must fail");
        assert_eq!(err, EventError::ClosedScope { scope });
    }

    #[test]
    fn never_opened_scope_reference_is_error() {
        // A scope id that was NEVER opened (not in `open_scopes`) is just as invalid
        // as a sealed one: declaring a blank under it is a ClosedScope protocol error.
        let mut sink = DatasetSink::new();
        let scope = ScopeId(7);
        let err = sink
            .term(EventTermId(0), EventTerm::Blank { label: "b", scope })
            .expect_err("never-opened-scope reference must fail");
        assert_eq!(err, EventError::ClosedScope { scope });
        assert!(sink.dataset().is_none(), "no freeze on protocol error");
    }

    #[test]
    fn default_matches_new_initial_state() {
        // `DatasetSink::default()` must produce the SAME initial state as `new()`:
        // the default scope is open, so a blank under it declares fine.
        let mut sink = DatasetSink::default();
        decl(
            &mut sink,
            EventTermId(0),
            EventTerm::Blank {
                label: "b",
                scope: ScopeId::DEFAULT,
            },
        );
        sink.finish()
            .expect("default sink has the default scope open");
        assert!(sink.dataset().is_some(), "default sink freezes");
    }

    /// A source that returns `Break` mid-stream cancels the drive and the sink, when
    /// marked cancelled, refuses to freeze.
    #[test]
    fn cancellation_does_not_freeze() {
        /// A sink that breaks on the first quad, marking itself cancelled.
        #[derive(Default)]
        struct BreakingSink {
            inner: DatasetSink,
        }
        impl RdfEventSink for BreakingSink {
            fn term(
                &mut self,
                id: EventTermId,
                term: EventTerm<'_>,
            ) -> Result<ControlFlow<()>, EventError> {
                self.inner.term(id, term)
            }
            fn quad(&mut self, _q: EventQuad) -> Result<ControlFlow<()>, EventError> {
                // Cancel on the first quad.
                self.inner.mark_cancelled();
                Ok(ControlFlow::Break(()))
            }
            fn reifier(
                &mut self,
                reifier: EventTermId,
                triple: EventTriple,
            ) -> Result<ControlFlow<()>, EventError> {
                self.inner.reifier(reifier, triple)
            }
            fn annotation(
                &mut self,
                reifier: EventTermId,
                p: EventTermId,
                o: EventTermId,
            ) -> Result<ControlFlow<()>, EventError> {
                self.inner.annotation(reifier, p, o)
            }
            fn open_scope(&mut self) -> Result<ScopeId, EventError> {
                self.inner.open_scope()
            }
            fn close_scope(&mut self, scope: ScopeId) -> Result<ControlFlow<()>, EventError> {
                self.inner.close_scope(scope)
            }
            fn finish(&mut self) -> Result<(), EventError> {
                self.inner.finish()
            }
        }

        let original = build_fixture();
        let source = FrozenDatasetSource::new(&original);
        let mut sink = BreakingSink::default();
        // The drive returns Ok (cancellation is a clean stop), but finish was never
        // reached because the source returns on Break.
        source.drive(&mut sink).expect("cancelled drive returns ok");
        assert!(
            sink.inner.dataset().is_none(),
            "a cancelled sink did not freeze"
        );
        // And an explicit finish after cancellation refuses to freeze.
        let err = sink.inner.finish().expect_err("finish after cancel fails");
        assert!(matches!(err, EventError::Message(_)));
    }

    #[test]
    fn cyclic_triple_term_is_error() {
        // A triple term whose object is itself: <<s p T>> where T == the triple's own
        // id. Resolving T re-enters resolve(T, ..) while T is still in progress, so the
        // in-progress guard fires CyclicTerm — NOT Unresolved (the raw term is still
        // present) and NOT NestingDepthExceeded (the cycle is caught before depth 16).
        let mut sink = DatasetSink::new();
        let s = EventTermId(0);
        let p = EventTermId(1);
        let cyclic = EventTermId(2);
        decl(&mut sink, s, EventTerm::Iri("http://example.org/s"));
        decl(&mut sink, p, EventTerm::Iri("http://example.org/p"));
        decl(
            &mut sink,
            cyclic,
            EventTerm::Triple(EventTriple { s, p, o: cyclic }),
        );
        let err = sink.finish().expect_err("cyclic triple term must fail");
        assert_eq!(
            err,
            EventError::CyclicTerm { id: cyclic },
            "a self-referential triple term is a cycle, not Unresolved, got {err:?}"
        );
        assert!(sink.dataset().is_none(), "no freeze on a cyclic term");
    }

    #[test]
    fn close_default_or_unopened_scope_is_error() {
        // close_scope is a real lifecycle op: closing the default scope, a scope that
        // was never opened, or an already-closed scope each fails with ClosedScope.
        let mut sink = DatasetSink::new();
        // (a) the default scope cannot be closed.
        let err = sink
            .close_scope(ScopeId::DEFAULT)
            .expect_err("closing the default scope must fail");
        assert_eq!(
            err,
            EventError::ClosedScope {
                scope: ScopeId::DEFAULT
            }
        );
        // (b) a never-opened scope cannot be closed.
        let unopened = ScopeId(9);
        let err = sink
            .close_scope(unopened)
            .expect_err("closing a never-opened scope must fail");
        assert_eq!(err, EventError::ClosedScope { scope: unopened });
        // (c) double-close: open, close (ok), close again (error).
        let scope = sink.open_scope().expect("open scope");
        let flow = sink.close_scope(scope).expect("first close ok");
        assert_eq!(flow, ControlFlow::Continue(()));
        let err = sink
            .close_scope(scope)
            .expect_err("closing an already-closed scope must fail");
        assert_eq!(err, EventError::ClosedScope { scope });
    }

    #[test]
    fn nesting_bound_is_enforced() {
        // Build a chain of triple terms nested past depth 16, terminating in real
        // IRIs so the ONLY failure is the depth bound (not an unresolved leaf): the
        // outermost triple's object is the next triple, recursing the full chain.
        let mut sink = DatasetSink::new();
        let s = EventTermId(0);
        let p = EventTermId(1);
        let leaf = EventTermId(2);
        decl(&mut sink, s, EventTerm::Iri("http://example.org/s"));
        decl(&mut sink, p, EventTerm::Iri("http://example.org/p"));
        decl(&mut sink, leaf, EventTerm::Iri("http://example.org/leaf"));
        // Chain: triple id (3+k) has object = triple id (3+k+1); the deepest triple's
        // object is the real IRI `leaf`, so every leaf is declared. Depth of the chain
        // exceeds MAX_TERM_NESTING_DEPTH, so resolving the head recurses past the bound.
        let chain = MAX_TERM_NESTING_DEPTH + 4;
        for k in 0..chain {
            let this = EventTermId(3 + k as u32);
            let object = if k + 1 < chain {
                EventTermId(3 + k as u32 + 1)
            } else {
                leaf
            };
            decl(
                &mut sink,
                this,
                EventTerm::Triple(EventTriple { s, p, o: object }),
            );
        }
        let err = sink.finish().expect_err("nesting past 16 must fail");
        assert!(
            matches!(err, EventError::NestingDepthExceeded { .. }),
            "deep nesting hits the depth bound, got {err:?}"
        );
        assert!(sink.dataset().is_none(), "no freeze on depth-bound failure");
    }
}
