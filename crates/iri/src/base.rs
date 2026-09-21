// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Base-aware IRI reference resolution: the in-scope base stack every codec
//! shares.
//!
//! RDF grammars split into two families, and conflating them is how relative
//! IRIs leak into a store:
//!
//! * Grammars that **admit relative references** (Turtle, TriG, N3, RDF/XML,
//!   JSON-LD, SPARQL) resolve them against the base in force — [`BaseScope::resolve`].
//! * Grammars whose syntax admits **no relative reference at all** (N-Triples,
//!   N-Quads, TriX, `HexTuples`) require an absolute IRI regardless of whether a
//!   base happens to be available — [`BaseScope::resolve_absolute_only`].
//!
//! # Only relative references are resolved
//!
//! Both families agree on what an **absolute** reference means: it is the IRI, taken
//! lexical-verbatim, whether or not a base happens to be in scope. Every grammar here
//! resolves *relative* IRIs against the base (Turtle §6.1, SPARQL §4.1.1, RDF/XML
//! `xml:base`, JSON-LD IRI expansion) and says nothing about resolving an absolute
//! one; putting one through RFC-3986 §5.2.2 anyway would apply `remove_dot_segments`
//! to it, which is §6.2.2.3 *syntax-based normalization* — forbidden by RDF Concepts
//! §3.2 ("Further normalization MUST NOT be performed") and pinned against by the
//! W3C JSON-LD REC vectors, which require `<http://a/bb/ccc/../d;p?q>` to survive
//! intact. Resolving it under a base and not without one would mean identical
//! document bytes denoting two different graphs with two different RDFC-1.0 digests,
//! so the rule lives in ONE place (this module's private `Reference` classifier) that
//! all three entry points share.
//!
//! # Where the base comes from (RFC-3986 §5.1)
//!
//! The precedence is the spec's, in the spec's order: an in-document base directive
//! (§5.1.1 — Turtle `@base`, SPARQL `BASE`, `xml:base`, JSON-LD `@context.@base`); else
//! the base the caller supplied through the API or `--base` (§5.1.2); else the
//! document's **retrieval IRI** (§5.1.3); else the reference cannot be resolved and the
//! resolution is a hard [`IriError::NoBase`] (§5.1.4) — never a silently interned
//! relative IRI, and never a fabricated default.
//!
//! `purrdf-iri` implements the first two steps and the §5.1.4 failure, and that is all
//! any library surface can implement: this crate — and with it every Rust library,
//! wasm, C-ABI and Python entry point — is handed BYTES, so it has no retrieval IRI to
//! fall back to and §5.1.3 is vacuous there. Those surfaces therefore hard-fail exactly
//! where §5.1.4 says to.
//!
//! §5.1.3 is implemented in ONE place, `purrdf_slice::retrieval`: a filesystem input's
//! RFC-8089 `file://` IRI, derived from the canonicalized path (with the Windows
//! extended-length and UNC translation that derivation requires) and applied only when no
//! base of higher precedence was given. `purrdf-slice` is the only *library* crate in the
//! workspace that opens files, and the two other surfaces that have a retrieval IRI —
//! `purrdf-shapes`' shape-union loader and `purrdf-cli` — consume it rather than
//! re-deriving one. Keeping it in a single filesystem-facing crate is what preserves byte
//! determinism for every other surface (a base invented from the local filesystem would
//! differ per machine and leak local paths into published RDF) while still answering
//! §5.1.3 where the retrieval IRI genuinely exists. Nothing filesystem-shaped crosses into
//! this crate or into `purrdf-rdf`, which is what keeps both wasm32-clean.
//!
//! [`BaseIri`] carries the "is absolute" invariant in the type, so the check
//! happens once at construction instead of at every resolution site, and
//! [`BaseScope`] replaces the hand-rolled base stacks that `xml:base` (per
//! element), JSON-LD `@base` (per context frame), and Turtle `@base` (rebinding
//! relative to the previous base) each grew independently.
//!
//! # Examples
//!
//! ```rust
//! use purrdf_iri::{BaseIri, BaseOrigin, BaseScope, IriError};
//!
//! // A document with a caller-supplied base resolves relative references.
//! let base = BaseIri::parse("http://example.org/dir/doc.ttl")?;
//! let mut scope = BaseScope::rooted(base, BaseOrigin::Caller);
//! assert_eq!(scope.resolve("")?.as_str(), "http://example.org/dir/doc.ttl");
//! assert_eq!(scope.resolve("other")?.as_str(), "http://example.org/dir/other");
//!
//! // `@base <sub/>` rebinds relative to the base already in force.
//! scope.rebind("sub/", BaseOrigin::Directive { line: 3, column: 1 })?;
//! assert_eq!(scope.resolve("x")?.as_str(), "http://example.org/dir/sub/x");
//!
//! // With no base in scope at all, a relative reference is a hard error.
//! let empty = BaseScope::empty();
//! assert!(matches!(empty.resolve(""), Err(IriError::NoBase { .. })));
//! # Ok::<(), IriError>(())
//! ```

use crate::error::{IriError, Result};
use crate::parse::{Iri, IriForm, classify, parse};

/// An [`Iri`] that is guaranteed to be **absolute** (to have a scheme).
///
/// The RFC-3986 §5.1 "base must be absolute" precondition is checked exactly once,
/// at construction, and thereafter carried by the type — so the resolution sites
/// downstream cannot forget it and cannot re-derive a different answer.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::{BaseIri, IriError};
///
/// let base = BaseIri::parse("http://example.org/a/b/c")?;
/// assert_eq!(base.as_str(), "http://example.org/a/b/c");
/// assert_eq!(base.resolve("../d")?.as_str(), "http://example.org/a/d");
///
/// // A scheme-less string is not a base.
/// assert!(matches!(
///     BaseIri::parse("/a/b/c"),
///     Err(IriError::NonAbsoluteBase(_))
/// ));
/// # Ok::<(), IriError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseIri(Iri);

impl BaseIri {
    /// Parse `s` as an absolute IRI to be used as a base.
    ///
    /// Uses the RFC-3987 IRI grammar (not the ASCII-only RFC-3986 URI subset), so
    /// a base carrying non-ASCII code points — which Turtle produces after `UCHAR`
    /// decoding — is accepted verbatim.
    pub fn parse(s: &str) -> Result<Self> {
        Self::try_from(parse(s)?)
    }

    /// Borrow the underlying validated [`Iri`].
    #[must_use]
    pub fn as_iri(&self) -> &Iri {
        &self.0
    }

    /// The base IRI text, verbatim.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Resolve `reference` against this base (RFC-3986 §5.2, strict), returning a
    /// new absolute [`Iri`].
    ///
    /// This delegates to [`Iri::resolve`] — the single resolution algorithm in the
    /// workspace — and is the faithful RFC-3986 answer, including §5.2.2's
    /// `remove_dot_segments(R.path)` for a reference that carries its own scheme.
    /// There is deliberately **no** second copy of that algorithm anywhere.
    ///
    /// Whether a reference is resolved *at all* is a separate question, and not this
    /// method's to answer: the RDF grammars resolve **relative** references only, so
    /// [`BaseScope::resolve`] and [`BaseScope::resolve_absolute_only`] decide it once,
    /// for every grammar family, through the private `Reference` classifier this module
    /// keeps as that rule's single owner. An absolute reference never reaches here from
    /// that layer.
    pub fn resolve(&self, reference: &str) -> Result<Iri> {
        self.0.resolve(reference)
    }

    /// Rebind this base from a `@base` / `BASE` / `xml:base` directive.
    ///
    /// The directive itself may be relative, in which case it is resolved against
    /// the base currently in force (Turtle §6.1, RFC-3986 §5.1.1) — so a chain of
    /// directives composes left to right. An **absolute** directive simply replaces
    /// the base, lexical-verbatim: it is not a reference to be resolved, and
    /// [`BaseScope::rebind`] on an empty scope has no choice but to take it verbatim,
    /// so taking it any other way here would make the same directive establish two
    /// different bases depending on what preceded it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_iri::BaseIri;
    ///
    /// let root = BaseIri::parse("http://example.org/a/b/c")?;
    /// let once = root.rebind("d/")?;
    /// assert_eq!(once.as_str(), "http://example.org/a/b/d/");
    /// assert_eq!(once.rebind("e/")?.as_str(), "http://example.org/a/b/d/e/");
    ///
    /// // An absolute directive replaces the base exactly as written.
    /// assert_eq!(
    ///     once.rebind("http://example.org/x/./y/")?.as_str(),
    ///     "http://example.org/x/./y/"
    /// );
    /// # Ok::<(), purrdf_iri::IriError>(())
    /// ```
    pub fn rebind(&self, directive: &str) -> Result<Self> {
        // `@base <>` is the same-document reference (RFC-3986 §4.4): it re-establishes
        // the base in force, minus any fragment.
        if directive.is_empty() {
            return Self::try_from(self.resolve("")?);
        }
        match Reference::parse(directive)? {
            Reference::Absolute(iri) => Self::try_from(iri),
            Reference::Relative(iri) => Self::try_from(self.0.resolve_iri(&iri)?),
        }
    }

    /// Spell `target` as a reference relative to this base — the exact inverse of
    /// [`resolve`](Self::resolve), so a serializer can emit `<>` or `<foo>` under a
    /// `@base` instead of a fully-expanded IRI.
    ///
    /// # `None` is semantic, not optionality
    ///
    /// `None` means **no relative spelling of `target` exists against this base** —
    /// the scheme differs, the authority differs, or `target` is not in the base's
    /// dot-normalized image (so no reference could round-trip to it byte for byte).
    /// It never means "failed" or "unavailable": the caller's correct response is to
    /// emit the absolute IRI, not to report an error.
    ///
    /// This is the **second** such carve-out in this crate, alongside
    /// [`expand_curie`](crate::expand_curie), whose `None` is likewise the semantic
    /// "not a CURIE / undeclared prefix" signal rather than a degraded failure. Both
    /// are documented exceptions to the crate's `no-optionality` hard-fail doctrine,
    /// and the list is exactly these two.
    ///
    /// Whenever this returns `Some(rel)`, `self.resolve(&rel)` reproduces `target`
    /// verbatim; that round trip is asserted over the whole RFC-3986 §5.4 table.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_iri::{BaseIri, parse};
    ///
    /// let base = BaseIri::parse("http://example.org/dir/doc.ttl")?;
    ///
    /// // The base itself is the empty reference — this is Turtle's `<>`.
    /// assert_eq!(
    ///     base.relativize(&parse("http://example.org/dir/doc.ttl")?),
    ///     Some(String::new())
    /// );
    /// assert_eq!(
    ///     base.relativize(&parse("http://example.org/dir/other")?),
    ///     Some("other".to_owned())
    /// );
    ///
    /// // A different authority has no relative spelling.
    /// assert_eq!(base.relativize(&parse("http://other.example/x")?), None);
    /// # Ok::<(), purrdf_iri::IriError>(())
    /// ```
    #[must_use]
    pub fn relativize(&self, target: &Iri) -> Option<String> {
        let base = &self.0;
        // A relative reference can never change the scheme, and can only change the
        // authority via a network-path reference (`//host/...`), which is not
        // shorter than the absolute form and would still need the base's scheme.
        if target.scheme() != base.scheme() || target.authority() != base.authority() {
            return None;
        }

        let candidate = relative_spelling(base, target);
        // Structural construction is the implementation; this round trip is the
        // contract. A target outside the base's dot-normalized image (e.g. a path
        // still containing `..`) cannot be spelled relatively at all — say so with
        // `None` rather than emit a reference that resolves somewhere else.
        let back = self.resolve(&candidate).ok()?;
        (back.as_str() == target.as_str()).then_some(candidate)
    }
}

impl TryFrom<Iri> for BaseIri {
    type Error = IriError;

    fn try_from(iri: Iri) -> Result<Self> {
        if iri.has_scheme() {
            Ok(Self(iri))
        } else {
            Err(IriError::NonAbsoluteBase(iri.as_str().to_owned()))
        }
    }
}

impl core::fmt::Display for BaseIri {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// Build the relative spelling of `target` against `base` from RFC-3986 component
/// structure (never by prefix-stripping the two strings).
///
/// The caller has already established that scheme and authority agree.
fn relative_spelling(base: &Iri, target: &Iri) -> String {
    let same_path = base.path() == target.path();
    let same_query = base.query() == target.query();

    // A reference with an EMPTY path inherits the base path, and inherits the base
    // query too unless it carries its own. That covers the same-document cases; any
    // other shape needs a real path, including "same path but the base has a query
    // the target lacks", which an empty path could not express.
    let mut out = if same_path && (same_query || target.query().is_some()) {
        String::new()
    } else {
        let mut path = relative_path(base.path(), target.path());
        if path.starts_with("//") {
            // Would re-parse as a network-path reference (a new authority).
            path.insert_str(0, "/.");
        } else if !path.starts_with('/') && first_segment_has_colon(&path) {
            // RFC-3986 §4.2 `path-noscheme`: a relative reference's first segment
            // may not contain ':' or it would be read as a scheme.
            path.insert_str(0, "./");
        }
        path
    };

    // With an empty path the query is inherited, so emit one only when it differs.
    // With a non-empty path nothing is inherited, so emit whatever the target has.
    let emit_query = if out.is_empty() { !same_query } else { true };
    if emit_query && let Some(q) = target.query() {
        out.push('?');
        out.push_str(q);
    }
    // The fragment is never inherited: it is taken from the reference verbatim.
    if let Some(f) = target.fragment() {
        out.push('#');
        out.push_str(f);
    }
    out
}

/// The relative path from `base_path` to `target_path`, as `../`-prefixed segments.
///
/// Both paths are compared segment-wise. The base's final segment is its "document"
/// and is not part of its directory, exactly mirroring the RFC-3986 §5.2.3 merge
/// step that resolution will apply in the other direction.
fn relative_path(base_path: &str, target_path: &str) -> String {
    // `split` always yields at least one element, so both slices are non-empty.
    let base_segs: Vec<&str> = base_path.split('/').collect();
    let target_segs: Vec<&str> = target_path.split('/').collect();
    let base_dir = &base_segs[..base_segs.len() - 1];
    let (target_dir, target_file) = target_segs.split_at(target_segs.len() - 1);
    let target_file = target_file[0];

    let common = base_dir
        .iter()
        .zip(target_dir.iter())
        .take_while(|(b, t)| b == t)
        .count();

    let mut out = String::with_capacity(target_path.len());
    for _ in common..base_dir.len() {
        out.push_str("../");
    }
    for seg in &target_dir[common..] {
        out.push_str(seg);
        out.push('/');
    }
    out.push_str(target_file);
    if out.is_empty() {
        // Same directory, empty document segment: the empty string would be a
        // same-document reference (inheriting the query), so spell it explicitly.
        out.push_str("./");
    }
    out
}

/// `true` iff the first path segment contains a `:` (RFC-3986 §4.2 `path-noscheme`).
fn first_segment_has_colon(path: &str) -> bool {
    path.split('/').next().is_some_and(|seg| seg.contains(':'))
}

/// A parsed, non-empty IRI reference, classified by whether the RDF grammars
/// resolve it at all.
///
/// This is the **single owner** of that rule for the base layer:
/// [`BaseScope::resolve`], [`BaseScope::resolve_absolute_only`] and
/// [`BaseIri::rebind`] all route through it, so the same string cannot be answered
/// three different ways.
enum Reference {
    /// The reference already **is** an IRI (it has a scheme).
    ///
    /// Every RDF grammar in scope here — Turtle/TriG/N3 §6.1, SPARQL §4.1.1,
    /// RDF/XML `xml:base`, JSON-LD IRI expansion — says *relative* IRIs are resolved
    /// against the base. An absolute one is not a reference to be resolved; it is
    /// the IRI, and it is carried lexical-verbatim. Running it through RFC-3986
    /// §5.2.2 anyway would apply `remove_dot_segments` to it, which is RFC-3986
    /// §6.2.2.3 *syntax-based normalization* — exactly what RDF Concepts §3.2
    /// forbids ("Further normalization MUST NOT be performed"), and what the pinned
    /// W3C JSON-LD REC vectors 0122/0123 pin by expecting
    /// `<http://a/bb/ccc/../d;p?q>` to survive intact.
    Absolute(Iri),
    /// The reference has no scheme and needs the base in force (RFC-3986 §5.2).
    Relative(Iri),
}

impl Reference {
    /// Parse and classify a **non-empty** reference.
    ///
    /// Parsing happens FIRST so that a malformed reference is reported as the syntax
    /// error it is, rather than as a base-related failure that would send the author
    /// off to add a `@base` that cannot help.
    fn parse(reference: &str) -> Result<Self> {
        let iri = parse(reference)?;
        if iri.has_scheme() {
            Ok(Self::Absolute(iri))
        } else {
            Ok(Self::Relative(iri))
        }
    }
}

/// Where the base IRI currently in force came from.
///
/// [`Display`](core::fmt::Display) renders it as the noun phrase a diagnostic reads
/// it as — *"the `@base` at line 3 column 1"*, *"the caller-supplied base"*, *"the
/// enclosing scope's base"* — which is how it reaches users, through
/// [`BaseInScope`] on the base-related [`IriError`] variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseOrigin {
    /// Supplied by the caller through the API (or the CLI), not by the document.
    Caller,
    /// Established by an in-document directive (`@base`, `BASE`, `xml:base`) at
    /// this 1-based source position.
    Directive {
        /// 1-based line of the directive.
        line: u32,
        /// 1-based column of the directive.
        column: u32,
    },
    /// Inherited from an enclosing lexical scope — an outer XML element, or an
    /// outer JSON-LD context frame.
    Enclosing,
}

impl core::fmt::Display for BaseOrigin {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Caller => f.write_str("the caller-supplied base"),
            Self::Directive { line, column } => {
                write!(f, "the `@base` at line {line} column {column}")
            }
            Self::Enclosing => f.write_str("the enclosing scope's base"),
        }
    }
}

/// The base-IRI situation at the point a resolution failed.
///
/// This is what makes [`BaseOrigin`] observable: it rides on the base-related
/// [`IriError`] variants and is rendered by their
/// [`Display`](core::fmt::Display), so a diagnostic says *which* base was in force
/// and where it came from. Because every consumer in the workspace already prints
/// `{err}`, that reaches every surface — Rust, wasm, the C ABI, Python and the CLI —
/// with no consumer edit at all.
///
/// It is an enum rather than an `Option<ScopedBase>` on purpose: "no base was in
/// scope" is a first-class RFC-3986 §5.1.4 answer with its own rendering, not the
/// absence of one, and this crate's `Option`-returning surfaces are a closed list of
/// two ([`expand_curie`](crate::expand_curie) and [`BaseIri::relativize`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaseInScope {
    /// No base IRI was in scope at all — RFC-3986 §5.1.4.
    Absent,
    /// A base was in force, with this provenance.
    InForce {
        /// The base IRI in force, verbatim.
        iri: String,
        /// How it came to be in force.
        origin: BaseOrigin,
    },
}

impl BaseInScope {
    /// The state of the innermost base of `scope`.
    #[must_use]
    pub fn of(scope: &BaseScope) -> Self {
        match scope.current() {
            None => Self::Absent,
            Some(scoped) => Self::InForce {
                iri: scoped.iri().as_str().to_owned(),
                origin: scoped.origin(),
            },
        }
    }
}

impl core::fmt::Display for BaseInScope {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Absent => f.write_str("no base IRI is in scope"),
            Self::InForce { iri, origin } => {
                write!(f, "{origin}, <{iri}>, is in scope")
            }
        }
    }
}

/// A base IRI together with the provenance of how it came to be in force.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedBase {
    iri: BaseIri,
    origin: BaseOrigin,
}

impl ScopedBase {
    /// Pair a base with its origin.
    #[must_use]
    pub fn new(iri: BaseIri, origin: BaseOrigin) -> Self {
        Self { iri, origin }
    }

    /// The base IRI in force.
    #[must_use]
    pub fn iri(&self) -> &BaseIri {
        &self.iri
    }

    /// Where this base came from.
    #[must_use]
    pub fn origin(&self) -> BaseOrigin {
        self.origin
    }
}

/// The stack of base IRIs in scope while parsing a document.
///
/// An **empty** stack means *no base is in scope*, which is a first-class state and
/// not an error until a relative reference actually needs one. Base scoping is
/// genuinely stacked in three of this workspace's codecs — `xml:base` per element,
/// JSON-LD `@base` per context frame, and Turtle `@base` rebinding relative to the
/// previous base — so it is one type here instead of three hand-rolled ones.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::{BaseIri, BaseOrigin, BaseScope};
///
/// // `xml:base` nesting: push on element entry, pop on exit.
/// let mut scope = BaseScope::rooted(BaseIri::parse("http://example.org/a/")?, BaseOrigin::Caller);
/// scope.push(BaseIri::parse("http://example.org/a/inner/")?, BaseOrigin::Enclosing);
/// assert_eq!(scope.resolve("x")?.as_str(), "http://example.org/a/inner/x");
/// scope.pop();
/// assert_eq!(scope.resolve("x")?.as_str(), "http://example.org/a/x");
/// # Ok::<(), purrdf_iri::IriError>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BaseScope(Vec<ScopedBase>);

impl BaseScope {
    /// A scope with no base at all. Relative references will hard-fail with
    /// [`IriError::NoBase`] until one is supplied.
    #[must_use]
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    /// A scope rooted at `base`.
    #[must_use]
    pub fn rooted(base: BaseIri, origin: BaseOrigin) -> Self {
        Self(vec![ScopedBase::new(base, origin)])
    }

    /// The base currently in force, or `None` when no base is in scope.
    #[must_use]
    pub fn current(&self) -> Option<&ScopedBase> {
        self.0.last()
    }

    /// `true` iff no base is in scope.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many bases are stacked (nesting depth).
    #[must_use]
    pub fn depth(&self) -> usize {
        self.0.len()
    }

    /// Enter a nested lexical scope with `base` in force.
    pub fn push(&mut self, base: BaseIri, origin: BaseOrigin) {
        self.0.push(ScopedBase::new(base, origin));
    }

    /// Leave the innermost lexical scope, restoring the enclosing base. Popping an
    /// already-empty scope is a no-op.
    pub fn pop(&mut self) {
        self.0.pop();
    }

    /// Apply a `@base` / `BASE` / `xml:base` directive **in place**, replacing the
    /// base currently in force rather than nesting a new one.
    ///
    /// A rebinding directive may be relative, in which case it resolves against the
    /// base already in force. With an empty scope there is nothing to resolve
    /// against, so the directive must itself be absolute — a relative one is
    /// [`IriError::NonAbsoluteBase`].
    pub fn rebind(&mut self, directive: &str, origin: BaseOrigin) -> Result<()> {
        let rebound = match self.0.last() {
            Some(top) => top.iri().rebind(directive)?,
            None => BaseIri::parse(directive)?,
        };
        let scoped = ScopedBase::new(rebound, origin);
        match self.0.last_mut() {
            Some(top) => *top = scoped,
            None => self.0.push(scoped),
        }
        Ok(())
    }

    /// Resolve `reference` for a grammar that **admits relative references**
    /// (Turtle, TriG, N3, RDF/XML, JSON-LD, SPARQL).
    ///
    /// An absolute reference resolves normally. A relative reference with no base in
    /// scope is [`IriError::NoBase`] (RFC-3986 §5.1.4) — never a silently-interned
    /// relative IRI, and never a base this layer invented for it. A retrieval IRI
    /// reaches the scope only by having been PUSHED into it by a surface that has one.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_iri::{BaseScope, IriError};
    ///
    /// let scope = BaseScope::empty();
    /// // An absolute reference needs no base.
    /// assert_eq!(scope.resolve("http://example.org/x")?.as_str(), "http://example.org/x");
    /// // A relative one names itself in the error.
    /// let err = scope.resolve("foo").unwrap_err();
    /// assert_eq!(err.diagnostic_code(), "iri-relative-no-base");
    /// assert!(format!("{err}").contains("\"foo\""));
    /// # Ok::<(), IriError>(())
    /// ```
    pub fn resolve(&self, reference: &str) -> Result<Iri> {
        // The EMPTY reference is the same-document reference (RFC-3986 §4.4), so it
        // is relative by definition — and `parse` would reject it as merely empty,
        // which would misreport the actual problem.
        if reference.is_empty() {
            return match self.current() {
                Some(scoped) => scoped.iri().resolve(""),
                None => Err(IriError::NoBase {
                    reference: String::new(),
                }),
            };
        }
        match Reference::parse(reference)? {
            // Verbatim, whether or not a base is in scope. Which of these two
            // branches a document takes must never change the IRI it denotes:
            // resolving an absolute reference through §5.2.2 with a base but not
            // without one is what made `<http://a/b/../c>` intern as
            // `http://a/c` in one and `http://a/b/../c` in the other — identical
            // bytes, two graphs, two RDFC-1.0 digests.
            Reference::Absolute(iri) => Ok(iri),
            Reference::Relative(iri) => match self.current() {
                Some(scoped) => scoped.iri().as_iri().resolve_iri(&iri),
                None => Err(IriError::NoBase {
                    reference: reference.to_owned(),
                }),
            },
        }
    }

    /// [`resolve`](Self::resolve)'s verdict for a caller that needs only the
    /// ACCEPTANCE, not the resolved [`Iri`].
    ///
    /// Accepts exactly what `resolve` accepts and fails with exactly the error
    /// `resolve` fails with — an **absolute** reference simply skips building the
    /// owned `Iri` that `resolve` hands back and this caller would drop unread. That
    /// is sound because `resolve` carries an absolute reference lexical-verbatim: the
    /// value it returns is the `reference` string itself, unchanged, so a caller that
    /// does not want the value needs nothing beyond the grammar check that classifying
    /// the reference already performed. Every other shape — the empty same-document
    /// reference, and a relative reference with or without a base — falls through to
    /// `resolve` itself rather than to a second transcription of it.
    ///
    /// The store-once term tables of an RDF dataset are the motivating caller: they
    /// validate each DISTINCT IRI exactly once and keep the string in their own
    /// arena, so the parsed `Iri` is pure waste — one allocation per distinct IRI in
    /// a document, a pack dictionary, or a restored dataset.
    ///
    /// # Errors
    ///
    /// Whatever [`resolve`](Self::resolve) returns for the same reference and scope.
    pub fn check(&self, reference: &str) -> Result<()> {
        if !reference.is_empty() && classify(reference)? == IriForm::Absolute {
            return Ok(());
        }
        self.resolve(reference).map(drop)
    }

    /// Resolve `reference` for a grammar whose syntax admits **no relative
    /// reference at all** (N-Triples, N-Quads, TriX, `HexTuples`).
    ///
    /// A relative reference is [`IriError::NotAbsoluteByGrammar`] regardless of
    /// whether a base is in scope: the base is never applied, because applying it
    /// would accept a document the grammar rejects. This is deliberately a
    /// different error from [`IriError::NoBase`] — supplying a base cannot fix it.
    ///
    /// An **absolute** reference is carried lexical-verbatim, which is the identical
    /// answer [`resolve`](Self::resolve) gives it: these grammars differ from the
    /// others in what they *reject*, never in what an accepted IRI denotes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_iri::{BaseIri, BaseOrigin, BaseScope};
    ///
    /// let scope = BaseScope::rooted(BaseIri::parse("http://example.org/")?, BaseOrigin::Caller);
    /// // Even WITH a base in scope, the grammar forbids the relative form.
    /// let err = scope.resolve_absolute_only("foo").unwrap_err();
    /// assert_eq!(err.diagnostic_code(), "iri-not-absolute-by-grammar");
    /// # Ok::<(), purrdf_iri::IriError>(())
    /// ```
    pub fn resolve_absolute_only(&self, reference: &str) -> Result<Iri> {
        if reference.is_empty() {
            return Err(IriError::NotAbsoluteByGrammar {
                reference: String::new(),
                base: BaseInScope::of(self),
            });
        }
        match Reference::parse(reference)? {
            Reference::Absolute(iri) => Ok(iri),
            Reference::Relative(_) => Err(IriError::NotAbsoluteByGrammar {
                reference: reference.to_owned(),
                // The base is not applied, but it IS in scope, and saying so is what
                // stops a caller who passed `--base` concluding it was dropped.
                base: BaseInScope::of(self),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::test_runner::{Config, TestRunner};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn base_requires_a_scheme() {
        assert!(matches!(
            BaseIri::parse("/a/b"),
            Err(IriError::NonAbsoluteBase(_))
        ));
        assert!(BaseIri::parse("http://example.org/a").is_ok());
    }

    #[test]
    fn rebind_on_empty_scope_requires_absolute() {
        let mut scope = BaseScope::empty();
        assert!(matches!(
            scope.rebind("sub/", BaseOrigin::Caller),
            Err(IriError::NonAbsoluteBase(_))
        ));
        assert!(scope.is_empty());
        scope
            .rebind("http://example.org/a/", BaseOrigin::Caller)
            .expect("absolute directive roots the scope");
        assert_eq!(scope.depth(), 1);
    }

    #[test]
    fn rebind_replaces_the_top_rather_than_nesting() {
        let mut scope = BaseScope::rooted(
            BaseIri::parse("http://example.org/a/b").unwrap(),
            BaseOrigin::Caller,
        );
        scope
            .rebind("c/", BaseOrigin::Directive { line: 2, column: 1 })
            .expect("relative directive rebinds");
        assert_eq!(scope.depth(), 1);
        let current = scope.current().expect("a base is in force");
        assert_eq!(current.iri().as_str(), "http://example.org/a/c/");
        assert_eq!(
            current.origin(),
            BaseOrigin::Directive { line: 2, column: 1 }
        );
    }

    #[test]
    fn push_and_pop_restore_the_enclosing_base() {
        let mut scope = BaseScope::rooted(
            BaseIri::parse("http://example.org/a/").unwrap(),
            BaseOrigin::Caller,
        );
        scope.push(
            BaseIri::parse("http://example.org/a/inner/").unwrap(),
            BaseOrigin::Enclosing,
        );
        assert_eq!(scope.depth(), 2);
        scope.pop();
        assert_eq!(scope.current().unwrap().origin(), BaseOrigin::Caller);
        scope.pop();
        assert!(scope.is_empty());
        // Popping an empty scope is a no-op, not a panic.
        scope.pop();
        assert!(scope.is_empty());
    }

    /// `check` is `resolve`'s verdict, so the two must agree on EVERY reference —
    /// acceptance, refusal, and which refusal.
    ///
    /// `check` short-circuits the absolute arm to skip building an `Iri` nobody
    /// reads, and a short circuit is exactly where an accept-everything or a
    /// refuse-everything hides: either one still passes any test that only exercises
    /// its own half. So both halves are executed here against the SAME references,
    /// under a scope that has a base and one that does not, and the verdicts are
    /// compared rather than asserted independently.
    #[test]
    fn check_agrees_with_resolve_on_every_reference() {
        let rooted = BaseScope::rooted(
            BaseIri::parse("http://example.org/a/b").unwrap(),
            BaseOrigin::Caller,
        );
        let empty = BaseScope::empty();

        for reference in [
            // Absolute — the arm `check` short-circuits.
            "http://example.org/x",
            "https://example.org/x?q=1#f",
            "urn:example:x",
            "file:///tmp/x",
            "http://example.org/a/bb/ccc/../d;p?q",
            // Relative — resolved with a base, refused without one.
            "foo",
            "../d",
            "/d",
            "?q",
            "#frag",
            // The same-document reference, which is relative by definition.
            "",
            // Not well formed at all: a syntax error in either scope, and NOT a
            // base-related one.
            "http://example.org/<bad>",
            "http://exa mple.org/x",
            "1nvalid:x",
        ] {
            for (name, scope) in [("rooted", &rooted), ("empty", &empty)] {
                let resolved = scope.resolve(reference);
                let checked = scope.check(reference);
                assert_eq!(
                    resolved.is_ok(),
                    checked.is_ok(),
                    "{name}: check and resolve disagree on accepting {reference:?}",
                );
                if let (Err(expected), Err(actual)) = (&resolved, &checked) {
                    assert_eq!(
                        expected.diagnostic_code(),
                        actual.diagnostic_code(),
                        "{name}: check and resolve refuse {reference:?} for different reasons",
                    );
                }
            }
        }

        // The counts prove the loop above was not vacuous in either direction: an
        // empty scope accepts exactly the absolute references and refuses the rest.
        let absolute = ["http://example.org/x", "urn:example:x"];
        let relative = ["foo", "", "../d"];
        assert!(absolute.iter().all(|r| empty.check(r).is_ok()));
        assert!(relative.iter().all(|r| empty.check(r).is_err()));
        assert!(relative.iter().all(|r| rooted.check(r).is_ok()));
    }

    /// A generated-reference strategy for [`check_agrees_with_resolve_over_generated_references`].
    ///
    /// The 14 references above were hand-chosen to hit each branch once; this
    /// generator exists to hit the same branches from thousands of different
    /// angles instead of the one angle a human thought of. A strategy that just
    /// throws `".*"` at the parser would overwhelmingly produce early syntax
    /// errors that `resolve` and `check` both refuse identically for a boring
    /// reason, which is a property test that never exercises the interesting
    /// disagreement surface (an absolute reference `check` fast-paths past
    /// `resolve`'s classification). So each arm below targets one shape the
    /// grammar treats specially — absolute IRIs (hierarchical and opaque),
    /// scheme-relative and rooted-relative forms, dot-segment relative paths,
    /// the query-only/fragment-only/empty same-document forms, percent-encodings
    /// (both well-formed and truncated/invalid), IPv6 literal authorities
    /// (valid and malformed), non-ASCII IRI characters, and the gen-delims/
    /// sub-delims/control-character boundary — with the absolute and rooted arms
    /// weighted heavily enough that a real fraction of generated cases resolve
    /// successfully rather than refuse.
    fn generated_reference() -> impl Strategy<Value = String> {
        let scheme = prop::sample::select(vec!["http", "https", "ftp", "urn", "mailto", "tag"]);
        let host = "[a-z][a-z0-9-]{0,10}(\\.[a-z][a-z0-9-]{0,10}){0,2}";
        let seg = "[a-zA-Z0-9._~!$&'()*+,;=-]{0,8}";
        let segs = prop::collection::vec(seg, 0..4);
        let part = "[a-zA-Z0-9._~!$&'()*+,;=:@/?-]{0,10}";
        let dot_form = prop::sample::select(vec![
            ".",
            "..",
            "./",
            "../",
            "a/./b",
            "a/../b",
            "./a",
            "../a",
            "a/b/../../c",
            "..%2fa",
            "a/..",
            "a/.",
        ]);
        let ipv6 = prop::sample::select(vec![
            "//[::1]/x",
            "//[2001:db8::1]:8080/x",
            "//[::1",
            "//[fe80::1%25eth0]/x",
            "//[::1]:port/x",
            "//[gggg::1]/x",
        ]);
        let non_ascii = prop::sample::select(vec!["café", "北京", "🎉", "naïve", "Ω", "e\u{0301}"]);
        let boundary_char = prop::sample::select(vec![
            ' ', '<', '>', '"', '{', '}', '|', '\\', '^', '`', '\u{0}', '\u{7f}',
        ]);
        let percent_bad = prop::sample::select(vec!["%zz", "%a", "%", "%1", "%gg", "%-1"]);

        prop_oneof![
            // Absolute, hierarchical IRIs with an authority, optional query and
            // fragment, and occasional dot segments that must survive verbatim
            // (RDF Concepts §3.2) rather than being resolved away.
            6 => (
                scheme.clone(),
                host,
                segs.clone(),
                prop::option::of(part),
                prop::option::of(part)
            )
                .prop_map(|(s, h, segs, q, f)| {
                    let mut out = format!("{s}://{h}");
                    for seg in &segs {
                        out.push('/');
                        out.push_str(seg);
                    }
                    if let Some(q) = q {
                        out.push('?');
                        out.push_str(&q);
                    }
                    if let Some(f) = f {
                        out.push('#');
                        out.push_str(&f);
                    }
                    out
                }),
            // Absolute, opaque (no-authority) IRIs: `scheme:opaque-part`.
            3 => (scheme.clone(), seg).prop_map(|(s, o)| format!("{s}:{o}")),
            // Network-path reference: `//host/path...` — relative, has no scheme.
            4 => (
                "[a-z][a-z0-9-]{0,10}(\\.[a-z][a-z0-9-]{0,10}){0,2}",
                segs.clone()
            )
                .prop_map(|(h, segs)| {
                    let mut out = format!("//{h}");
                    for seg in &segs {
                        out.push('/');
                        out.push_str(seg);
                    }
                    out
                }),
            // Rooted-relative: `/path...`.
            4 => segs.clone().prop_map(|segs| {
                let mut out = String::new();
                for seg in &segs {
                    out.push('/');
                    out.push_str(seg);
                }
                if out.is_empty() {
                    out.push('/');
                }
                out
            }),
            // Dot-segment-only relative-path references.
            3 => dot_form.prop_map(str::to_owned),
            // The empty, same-document reference.
            1 => Just(String::new()),
            // Fragment-only and query-only same-document references.
            2 => part.prop_map(|f| format!("#{f}")),
            2 => part.prop_map(|q| format!("?{q}")),
            // Percent-encoding stress: a well-formed octet or a malformed one,
            // spliced into a path segment.
            3 => (prop::bool::ANY, seg).prop_map(|(valid, s)| {
                let pct = if valid { "%41" } else { "%zz" };
                format!("/{s}{pct}")
            }),
            2 => percent_bad.prop_map(|p| format!("/{p}")),
            // IPv6 literal authorities, valid and malformed.
            3 => ipv6.prop_map(str::to_owned),
            // Non-ASCII / IRI characters (RFC-3987 `ucschar`), which the RFC-3986
            // URI grammar rejects and the RFC-3987 IRI grammar admits.
            3 => (non_ascii, segs.clone()).prop_map(|(n, segs)| {
                let mut out = format!("/{n}");
                for seg in &segs {
                    out.push('/');
                    out.push_str(seg);
                }
                out
            }),
            // Boundary/delimiter and control characters the grammar treats
            // specially, spliced into an otherwise plausible path.
            3 => (boundary_char, seg).prop_map(|(c, s)| format!("/{s}{c}{s}")),
            // A fully-unstructured fallback so nothing the structured arms above
            // happen to miss is systematically excluded.
            2 => ".{0,30}",
        ]
    }

    /// [`check_agrees_with_resolve_on_every_reference`] proves agreement on 14
    /// hand-chosen references. This proves the same agreement — acceptance AND,
    /// on refusal, [`IriError::diagnostic_code`] — across thousands of GENERATED
    /// ones drawn from [`generated_reference`], under the same two scopes.
    ///
    /// The accepted/refused counts are asserted non-trivially non-zero at the
    /// end for the same reason the hand-written test counts its fixed cases: a
    /// generator that only ever produces syntax garbage would make this
    /// property vacuously true without ever reaching the accept path `check`
    /// fast-paths past `resolve`.
    #[test]
    fn check_agrees_with_resolve_over_generated_references() {
        let rooted = BaseScope::rooted(
            BaseIri::parse("http://example.org/a/b").unwrap(),
            BaseOrigin::Caller,
        );
        let empty = BaseScope::empty();

        let accepted = AtomicUsize::new(0);
        let rejected = AtomicUsize::new(0);

        let mut runner = TestRunner::new(Config {
            cases: 4096,
            failure_persistence: None,
            ..Config::default()
        });
        runner
            .run(&generated_reference(), |reference| {
                for (name, scope) in [("rooted", &rooted), ("empty", &empty)] {
                    let resolved = scope.resolve(&reference);
                    let checked = scope.check(&reference);
                    if resolved.is_ok() {
                        accepted.fetch_add(1, Ordering::Relaxed);
                    } else {
                        rejected.fetch_add(1, Ordering::Relaxed);
                    }
                    if resolved.is_ok() != checked.is_ok() {
                        return Err(TestCaseError::fail(format!(
                            "{name}: check and resolve disagree on accepting {reference:?}: \
                             resolve_ok={} check_ok={}",
                            resolved.is_ok(),
                            checked.is_ok()
                        )));
                    }
                    if let (Err(expected), Err(actual)) = (&resolved, &checked)
                        && expected.diagnostic_code() != actual.diagnostic_code()
                    {
                        return Err(TestCaseError::fail(format!(
                            "{name}: check and resolve refuse {reference:?} for different \
                             reasons: resolve={} check={}",
                            expected.diagnostic_code(),
                            actual.diagnostic_code()
                        )));
                    }
                }
                Ok(())
            })
            .unwrap_or_else(|error| panic!("{error}"));

        let accepted = accepted.load(Ordering::Relaxed);
        let rejected = rejected.load(Ordering::Relaxed);
        assert!(
            accepted > 100,
            "generator produced too few accepted references ({accepted}) to prove agreement \
             on the accept path check short-circuits"
        );
        assert!(
            rejected > 100,
            "generator produced too few rejected references ({rejected}) to prove agreement \
             on the refuse path, including which diagnostic_code() is reported"
        );
    }
}
