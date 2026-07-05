// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The **permissive RDF 1.2 ingestion protocol** (purrdf P6): the neutral
//! event seam that an RDF *source* (a parser, a GTS reader, a frozen-dataset
//! replayer) uses to push a dataset *into* a *sink* (an IR builder, a serializer)
//! WITHOUT either side knowing the other's concrete types.
//!
//! This crate has **zero dependencies** on purpose. It is the contract that both the
//! IR engine (`purrdf-core`) and the GTS container (`purrdf-gts`) depend ON — so a
//! pure parse→serialize path that touches only these traits stays under the
//! workspace `MIT OR Apache-2.0` license. Its value types ([`EventTerm`],
//! [`EventQuad`], …) are therefore **self-contained**:
//! it carries its OWN [`EventTermId`] term ids, not the engine's dataset-local
//! `TermId`.
//!
//! # The dual
//!
//! [`RdfEventSink`] is the INGESTION direction: an external, possibly out-of-order
//! event stream folds *into* something. Its dual lives in `purrdf-core` as
//! `RdfDatasetVisitor` — the infallible OUTPUT visitor that walks an *already-frozen*
//! dataset *out* as events. Ingestion is fallible (forward references, cancellation,
//! and unresolved-at-finish are all real outcomes); the output visitor is not.
//!
//! # Protocol semantics
//!
//! These are the rules a [`RdfEventSink`] implementer MUST honor and an
//! [`RdfEventSource`] MAY rely on:
//!
//! * **Forward references are allowed.** A [`quad`](RdfEventSink::quad) /
//!   [`reifier`](RdfEventSink::reifier) / [`annotation`](RdfEventSink::annotation)
//!   MAY reference an [`EventTermId`] whose [`term`](RdfEventSink::term) declaration
//!   has not yet arrived. References are resolved at [`finish`](RdfEventSink::finish),
//!   never eagerly. A source that happens to declare every term before referencing it
//!   advertises this via [`RdfEventSource::declares_before_reference`].
//! * **At most one declaration per id per drive.** An [`EventTermId`] is unique
//!   across the WHOLE drive: declaring the same id twice anywhere (in any scope, open
//!   or default) is an [`EventError::RedeclaredId`] — there is no last-writer-wins.
//! * **`EventTermId` is drive-global; `ScopeId` namespaces blank-node labels only.**
//!   The id space is global to one ingestion drive — every reference ([`EventQuad`],
//!   [`EventTriple`], `reifier`, `annotation`) carries a bare [`EventTermId`] that
//!   resolves against that single global space, so a buffered row never needs a scope
//!   to disambiguate which declaration it names. A [`ScopeId`] scopes blank-node
//!   *label* identity ONLY: the same blank label in two different scopes names two
//!   different nodes (mirroring per-segment blank scope in GTS), yet each still gets
//!   its own globally-unique [`EventTermId`]. [`close_scope`](RdfEventSink::close_scope)
//!   seals a scope so no NEW blank may be declared under it; already-declared ids stay
//!   referenceable by their global [`EventTermId`]. Declaring a blank under a sealed
//!   or never-opened scope is [`EventError::ClosedScope`].
//! * **Unresolved at finish is a hard error.** Any [`EventTermId`] still undeclared
//!   when [`finish`](RdfEventSink::finish) runs is [`EventError::Unresolved`] — never
//!   a silent drop or degraded fallback (no-optionality doctrine).
//! * **Cancellation must not freeze.** A sink MAY return
//!   [`ControlFlow::Break`] from any event to cancel
//!   the drive; the source stops immediately and the sink's partial state MUST NOT be
//!   frozen into a result.
//! * **Triple-term nesting is depth-bounded.** Reified [`EventTriple`] terms may
//!   nest, but a sink MUST bound resolution depth by [`MAX_TERM_NESTING_DEPTH`] (16),
//!   hard-failing with [`EventError::NestingDepthExceeded`] rather than recursing
//!   without bound.
//! * **Ill-typed literals are preserved, never auto-rejected.** A [`EventTerm::Literal`]
//!   with a malformed lexical form for its datatype is carried through and MAY be
//!   flagged downstream; the protocol never rejects it at ingestion.

#![forbid(unsafe_code)]

use core::ops::ControlFlow;

/// Depth bound for resolving nested reified-triple terms, mirroring
/// `MAX_GTS_TERM_NESTING_DEPTH` in the IR engine. A cyclic or absurdly nested triple
/// term hard-fails ([`EventError::NestingDepthExceeded`]) rather than recursing
/// without bound.
pub const MAX_TERM_NESTING_DEPTH: usize = 16;

/// A **blank-node label namespace**, local to one ingestion drive. A [`ScopeId`]
/// does NOT scope [`EventTermId`]s (those are drive-global); it scopes blank-node
/// *label* identity ONLY, so the same blank label in different scopes names different
/// nodes. The default scope is [`ScopeId::DEFAULT`]; a source opens further scopes
/// (e.g. one per GTS segment) via [`RdfEventSink::open_scope`] and seals them with
/// [`RdfEventSink::close_scope`] (after which no new blank may be declared under
/// that scope, though already-declared ids remain referenceable globally).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ScopeId(pub u32);

impl ScopeId {
    /// The default/global scope, always open from the start of a drive.
    pub const DEFAULT: Self = Self(0);
}

impl Default for ScopeId {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A **protocol-local, drive-global** term id. Ids are minted by the *source* and are
/// meaningful within one ingestion drive, where they form a SINGLE global id space:
/// every reference carries a bare `EventTermId` that resolves against that one space
/// regardless of scope (a [`ScopeId`] namespaces blank-node *labels* only, never the
/// id space). Ids MAY be forward-referenced (used by a quad before their
/// [`term`](RdfEventSink::term) declaration arrives) and are resolved to the sink's
/// own identity at [`finish`](RdfEventSink::finish). Each id may be declared at most
/// once per drive — redeclaring it anywhere is [`EventError::RedeclaredId`].
///
/// This is deliberately NOT the IR engine's dataset-local `TermId`: the protocol owns
/// its own id space so neither side leaks identity into the other.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EventTermId(pub u32);

/// RDF 1.2 base direction for directional language-tagged literals. Mirrors the IR
/// engine's `RdfTextDirection` by value so this crate stays dependency-free.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum TextDirection {
    /// Left-to-right.
    Ltr,
    /// Right-to-left.
    Rtl,
}

/// A **reified statement** — a triple (s, p, o) of [`EventTermId`]s, NOT a quad.
///
/// This is the RDF 1.2 quoted-triple payload: it carries no graph slot. Its
/// components are themselves [`EventTermId`]s, so a triple term MAY nest another
/// triple term in any position (depth-bounded by [`MAX_TERM_NESTING_DEPTH`]).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EventTriple {
    /// Subject term id.
    pub s: EventTermId,
    /// Predicate term id.
    pub p: EventTermId,
    /// Object term id.
    pub o: EventTermId,
}

/// The *value* of one declared term, borrowed for the duration of the
/// [`term`](RdfEventSink::term) call. Self-contained: an [`EventTerm::Literal`]
/// carries its datatype as a borrowed IRI string (by value), never an id.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventTerm<'a> {
    /// An IRI, by its full string.
    Iri(&'a str),
    /// A blank node, identified by `(label, scope)`. The same label in different
    /// scopes names different nodes.
    Blank {
        /// The blank-node label (without the `_:` prefix).
        label: &'a str,
        /// The scope this blank node belongs to.
        scope: ScopeId,
    },
    /// A literal. The datatype is the **expanded IRI by value** (e.g.
    /// `http://www.w3.org/2001/XMLSchema#string`); an ill-typed lexical form is
    /// preserved verbatim, never rejected here.
    Literal {
        /// The lexical form, byte-for-byte as authored.
        lexical: &'a str,
        /// The datatype IRI, by value.
        datatype: &'a str,
        /// The (optional) language tag.
        language: Option<&'a str>,
        /// The (optional) RDF 1.2 base direction.
        direction: Option<TextDirection>,
    },
    /// A reified triple term (RDF 1.2 quoted triple).
    Triple(EventTriple),
}

/// One quad row: an (s, p, o) statement plus an optional graph name. `g == None`
/// names the default graph.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EventQuad {
    /// Subject term id.
    pub s: EventTermId,
    /// Predicate term id.
    pub p: EventTermId,
    /// Object term id.
    pub o: EventTermId,
    /// Optional graph-name term id; `None` is the default graph.
    pub g: Option<EventTermId>,
}

/// A droppable diagnostic source-location hint. A sink MAY ignore it entirely; it
/// exists only to carry parser positions through to downstream diagnostics.
///
/// Kept deliberately simple (byte offset + 1-based line/column). The whole hint is
/// optional: a source that has no position information simply never calls
/// [`RdfEventSink::location`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct SourceSpan {
    /// Byte offset from the start of the input.
    pub byte_offset: usize,
    /// 1-based line number.
    pub line: u32,
    /// 1-based column number.
    pub column: u32,
}

impl SourceSpan {
    /// A span at a byte offset with line/column position.
    pub fn new(byte_offset: usize, line: u32, column: u32) -> Self {
        Self {
            byte_offset,
            line,
            column,
        }
    }
}

/// The **concrete** ingestion error type. Deliberately not generic: the seam stays
/// object-safe and the error space is fixed by the protocol (see the module docs).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EventError {
    /// An [`EventTermId`] was declared more than once in one drive. Ids are
    /// drive-global, so a redeclaration anywhere — in any scope — is an error.
    RedeclaredId {
        /// The id that was redeclared.
        id: EventTermId,
        /// The scope the FIRST (winning) declaration was recorded under.
        scope: ScopeId,
    },
    /// A blank declaration named a scope that is not currently open — either it was
    /// sealed by [`close_scope`](RdfEventSink::close_scope) or it was never opened.
    ClosedScope {
        /// The closed (or never-opened) scope that was referenced.
        scope: ScopeId,
    },
    /// An [`EventTermId`] was referenced but never declared by the time
    /// [`finish`](RdfEventSink::finish) ran.
    Unresolved {
        /// The id that was never declared.
        id: EventTermId,
    },
    /// Reified-triple-term nesting exceeded [`MAX_TERM_NESTING_DEPTH`].
    NestingDepthExceeded {
        /// The id at which the depth bound was crossed.
        id: EventTermId,
    },
    /// A reified-triple term (directly or transitively) references its own id, so
    /// resolution re-enters an id already in progress. Distinct from
    /// [`Unresolved`](Self::Unresolved) (a genuinely never-declared id) and from
    /// [`NestingDepthExceeded`](Self::NestingDepthExceeded) (a bounded but acyclic
    /// chain): a cycle is a structural error, not a missing declaration.
    CyclicTerm {
        /// The id at which the cycle was detected (the id already being resolved).
        id: EventTermId,
    },
    /// Any other protocol failure, carried as a message (e.g. a sink-specific freeze
    /// failure surfaced through the seam).
    Message(String),
}

impl EventError {
    /// Construct a generic message error.
    pub fn message(msg: impl Into<String>) -> Self {
        Self::Message(msg.into())
    }
}

impl core::fmt::Display for EventError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RedeclaredId { id, scope } => {
                write!(f, "event term id {} redeclared in scope {}", id.0, scope.0)
            }
            Self::ClosedScope { scope } => {
                write!(f, "reference to closed scope {}", scope.0)
            }
            Self::Unresolved { id } => {
                write!(f, "event term id {} referenced but never declared", id.0)
            }
            Self::NestingDepthExceeded { id } => write!(
                f,
                "triple-term nesting depth limit ({MAX_TERM_NESTING_DEPTH}) exceeded resolving event term id {}",
                id.0
            ),
            Self::CyclicTerm { id } => write!(
                f,
                "cyclic triple term: event term id {} (directly or transitively) references itself",
                id.0
            ),
            Self::Message(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for EventError {}

/// The permissive ingestion **sink**: a receiver of an RDF 1.2 event stream that an
/// arbitrary [`RdfEventSource`] drives *into* it.
///
/// # Object safety
///
/// This trait is **object-safe**: it has no generic methods and uses the concrete
/// [`EventError`], so a `&mut dyn RdfEventSink` is a first-class value (used by the
/// erased [`RdfEventSource::drive_erased`] path and by registries that store sinks
/// behind a `dyn`). The compile-time guard below fails the build if object safety
/// ever regresses.
///
/// # Defaults
///
/// Every method has a default so a sink implements only what it cares about: the
/// droppable hints ([`prefix`](Self::prefix) / [`base`](Self::base) /
/// [`location`](Self::location)) default to a no-op `Continue`, and
/// [`quads`](Self::quads) defaults to looping over [`quad`](Self::quad) honoring
/// [`ControlFlow::Break`].
pub trait RdfEventSink {
    /// Declare a term and its value. [`EventTermId`]s are drive-global, so the same id
    /// MUST NOT be declared twice anywhere in one drive (→
    /// [`EventError::RedeclaredId`]). A blank-node declaration's [`ScopeId`] must be
    /// currently open (→ [`EventError::ClosedScope`] otherwise). Declarations MAY
    /// arrive after the quads that reference them (forward references).
    fn term(&mut self, id: EventTermId, term: EventTerm<'_>)
    -> Result<ControlFlow<()>, EventError>;

    /// A quad row. Any of its positions MAY be a not-yet-declared [`EventTermId`]
    /// (resolved at [`finish`](Self::finish)).
    fn quad(&mut self, q: EventQuad) -> Result<ControlFlow<()>, EventError>;

    /// A batch of quad rows. Defaults to looping over [`quad`](Self::quad), stopping
    /// early (and reporting [`ControlFlow::Break`]) if any quad cancels.
    fn quads(&mut self, qs: &[EventQuad]) -> Result<ControlFlow<()>, EventError> {
        for &q in qs {
            if self.quad(q)? == ControlFlow::Break(()) {
                return Ok(ControlFlow::Break(()));
            }
        }
        Ok(ControlFlow::Continue(()))
    }

    /// Bind a `reifier` resource to a reified `triple` term (C0.4: many reifiers MAY
    /// bind one triple). Both the reifier id and the triple's component ids MAY be
    /// forward references.
    fn reifier(
        &mut self,
        reifier: EventTermId,
        triple: EventTriple,
    ) -> Result<ControlFlow<()>, EventError>;

    /// A `(reifier, predicate, object)` statement annotation. All three ids MAY be
    /// forward references.
    fn annotation(
        &mut self,
        reifier: EventTermId,
        p: EventTermId,
        o: EventTermId,
    ) -> Result<ControlFlow<()>, EventError>;

    /// Open a fresh blank-node label namespace, returning its [`ScopeId`]. Blank-node
    /// *label* identity is namespaced per scope (see [`close_scope`](Self::close_scope));
    /// the drive-global [`EventTermId`] space is unaffected.
    fn open_scope(&mut self) -> Result<ScopeId, EventError>;

    /// Seal a scope: no NEW blank may be declared under it afterwards (→
    /// [`EventError::ClosedScope`]). Already-declared ids remain referenceable by
    /// their drive-global [`EventTermId`]. Closing [`ScopeId::DEFAULT`], a scope that
    /// was never opened, or an already-closed scope is itself an
    /// [`EventError::ClosedScope`] — `close_scope` is a real lifecycle op, not a
    /// silent no-op.
    fn close_scope(&mut self, scope: ScopeId) -> Result<ControlFlow<()>, EventError>;

    /// Droppable hint: a namespace prefix mapping (`prefix:` → IRI). Defaults to a
    /// no-op; carrying it is optional.
    fn prefix(&mut self, prefix: &str, iri: &str) -> Result<ControlFlow<()>, EventError> {
        let _ = (prefix, iri);
        Ok(ControlFlow::Continue(()))
    }

    /// Droppable hint: the document base IRI. Defaults to a no-op.
    fn base(&mut self, iri: &str) -> Result<ControlFlow<()>, EventError> {
        let _ = iri;
        Ok(ControlFlow::Continue(()))
    }

    /// Droppable hint: a source-location span for the next event. Defaults to a
    /// no-op.
    fn location(&mut self, span: SourceSpan) -> Result<ControlFlow<()>, EventError> {
        let _ = span;
        Ok(ControlFlow::Continue(()))
    }

    /// Resolve every forward reference and finalize. Any [`EventTermId`] still
    /// undeclared here is [`EventError::Unresolved`] (hard fail, no silent drop).
    ///
    /// Takes `&mut self` (not `self`) so the trait stays object-safe — a `&mut dyn
    /// RdfEventSink` can be finished. A sink that produces an owned result exposes it
    /// through its own concrete API after `finish` returns `Ok`.
    fn finish(&mut self) -> Result<(), EventError>;
}

// Object-safety guard: this fails to compile if `RdfEventSink` ever gains a
// generic method, a `self`-by-value method, or an associated-type-bound that breaks
// `&mut dyn RdfEventSink`. It is the load-bearing P6 invariant.
const _: fn(&mut dyn RdfEventSink) = |_| {};

/// The permissive ingestion **source**: something that can drive an RDF 1.2 event
/// stream into any [`RdfEventSink`].
pub trait RdfEventSource {
    /// Drive the full event stream into `sink`. The `?Sized` bound lets this take
    /// either a concrete sink (zero-cost, monomorphized) or a `dyn RdfEventSink`.
    ///
    /// A source MUST stop immediately if any event returns
    /// [`ControlFlow::Break`], and MUST call
    /// [`finish`](RdfEventSink::finish) exactly once at the end of a non-cancelled
    /// drive.
    fn drive<S: RdfEventSink + ?Sized>(&self, sink: &mut S) -> Result<(), EventError>;

    /// Drive into a type-erased sink. Defaults to [`drive`](Self::drive) (which
    /// already accepts `?Sized`), provided as a named entry point for registries that
    /// only hold a `&mut dyn RdfEventSink`.
    fn drive_erased(&self, sink: &mut dyn RdfEventSink) -> Result<(), EventError> {
        self.drive(sink)
    }

    /// Capability hint: does this source declare every term BEFORE any reference to
    /// it? A sink MAY use this to skip its forward-reference buffering. Defaults to
    /// `false` (assume forward references are possible).
    fn declares_before_reference(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trivial in-memory sink used to exercise the default methods and the
    /// object-safe path within this dependency-free crate.
    #[derive(Default)]
    struct CountingSink {
        terms: usize,
        quads: usize,
        reifiers: usize,
        annotations: usize,
        next_scope: u32,
        finished: bool,
        break_after_quads: Option<usize>,
    }

    impl RdfEventSink for CountingSink {
        fn term(
            &mut self,
            _id: EventTermId,
            _term: EventTerm<'_>,
        ) -> Result<ControlFlow<()>, EventError> {
            self.terms += 1;
            Ok(ControlFlow::Continue(()))
        }

        fn quad(&mut self, _q: EventQuad) -> Result<ControlFlow<()>, EventError> {
            self.quads += 1;
            if let Some(limit) = self.break_after_quads
                && self.quads >= limit
            {
                return Ok(ControlFlow::Break(()));
            }
            Ok(ControlFlow::Continue(()))
        }

        fn reifier(
            &mut self,
            _reifier: EventTermId,
            _triple: EventTriple,
        ) -> Result<ControlFlow<()>, EventError> {
            self.reifiers += 1;
            Ok(ControlFlow::Continue(()))
        }

        fn annotation(
            &mut self,
            _reifier: EventTermId,
            _p: EventTermId,
            _o: EventTermId,
        ) -> Result<ControlFlow<()>, EventError> {
            self.annotations += 1;
            Ok(ControlFlow::Continue(()))
        }

        fn open_scope(&mut self) -> Result<ScopeId, EventError> {
            self.next_scope += 1;
            Ok(ScopeId(self.next_scope))
        }

        fn close_scope(&mut self, _scope: ScopeId) -> Result<ControlFlow<()>, EventError> {
            Ok(ControlFlow::Continue(()))
        }

        fn finish(&mut self) -> Result<(), EventError> {
            self.finished = true;
            Ok(())
        }
    }

    #[test]
    fn default_quads_loops_and_honors_break() {
        let mut sink = CountingSink {
            break_after_quads: Some(2),
            ..CountingSink::default()
        };
        let id = EventTermId(0);
        let q = EventQuad {
            s: id,
            p: id,
            o: id,
            g: None,
        };
        let flow = sink.quads(&[q, q, q, q]).expect("quads ok");
        assert_eq!(flow, ControlFlow::Break(()));
        assert_eq!(sink.quads, 2, "stopped at the break, did not run all four");
    }

    #[test]
    fn droppable_hints_default_to_noop_continue() {
        let mut sink = CountingSink::default();
        assert_eq!(
            sink.prefix("ex", "http://example.org/").unwrap(),
            ControlFlow::Continue(())
        );
        assert_eq!(
            sink.base("http://example.org/").unwrap(),
            ControlFlow::Continue(())
        );
        assert_eq!(
            sink.location(SourceSpan::new(0, 1, 1)).unwrap(),
            ControlFlow::Continue(())
        );
    }

    #[test]
    fn sink_is_usable_as_dyn() {
        // The object-safety guard proves this at compile time; this exercises it.
        let mut sink = CountingSink::default();
        let dynamic: &mut dyn RdfEventSink = &mut sink;
        let _ = dynamic
            .term(EventTermId(0), EventTerm::Iri("http://example.org/s"))
            .expect("term");
        dynamic.finish().expect("finish");
        assert!(sink.finished);
        assert_eq!(sink.terms, 1);
    }

    #[test]
    fn error_display_is_descriptive() {
        let cases = [
            EventError::RedeclaredId {
                id: EventTermId(3),
                scope: ScopeId(1),
            },
            EventError::ClosedScope { scope: ScopeId(2) },
            EventError::Unresolved { id: EventTermId(7) },
            EventError::NestingDepthExceeded { id: EventTermId(9) },
            EventError::CyclicTerm {
                id: EventTermId(11),
            },
            EventError::message("boom"),
        ];
        for case in cases {
            assert!(!case.to_string().is_empty());
        }
        // It really is a std::error::Error.
        fn assert_error<E: std::error::Error>(_: &E) {}
        assert_error(&EventError::message("x"));
    }
}
