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
//! §5.1.3 is implemented in ONE place, `purrdf-cli`, which is the one surface that has a
//! retrieval IRI: a filesystem input's RFC-8089 `file://` IRI, derived from the
//! canonicalized path and applied only when no base of higher precedence was given.
//! Keeping it there is what preserves byte determinism for every other surface (a base
//! invented from the local filesystem would differ per machine and leak local paths into
//! published RDF) while still answering §5.1.3 where the retrieval IRI genuinely exists.
//! Nothing filesystem-shaped crosses into this crate or into `purrdf-rdf`.
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
use crate::parse::{Iri, parse};

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
    /// workspace. There is deliberately **no** "the reference is already absolute"
    /// fast path: `Iri::resolve` already dot-normalizes and revalidates an absolute
    /// reference, and a second entry point that skipped that would be free to
    /// diverge, which is precisely the defect this layer exists to delete.
    pub fn resolve(&self, reference: &str) -> Result<Iri> {
        self.0.resolve(reference)
    }

    /// Rebind this base from a `@base` / `BASE` / `xml:base` directive.
    ///
    /// The directive itself may be relative, in which case it is resolved against
    /// the base currently in force (Turtle §6.1, RFC-3986 §5.1.1) — so a chain of
    /// directives composes left to right.
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
    /// # Ok::<(), purrdf_iri::IriError>(())
    /// ```
    pub fn rebind(&self, directive: &str) -> Result<Self> {
        Self::try_from(self.resolve(directive)?)
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

/// Where the base IRI currently in force came from.
///
/// Carried so a diagnostic can say *"resolved against the `@base` on line 3, which
/// itself resolved against the caller-supplied base"* rather than merely quoting a
/// string with no provenance.
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
        if let Some(scoped) = self.current() {
            return scoped.iri().resolve(reference);
        }
        // No base in scope. The EMPTY reference is the same-document reference, so
        // it is relative by definition — `parse` would reject it as merely empty,
        // which would misreport the actual problem.
        if reference.is_empty() {
            return Err(IriError::NoBase {
                reference: String::new(),
            });
        }
        // Parse FIRST: a malformed reference is a syntax error on its own terms and
        // must not be reported as "no base" (which would send the user off to add a
        // `@base` that cannot help).
        let iri = parse(reference)?;
        if iri.has_scheme() {
            Ok(iri)
        } else {
            Err(IriError::NoBase {
                reference: reference.to_owned(),
            })
        }
    }

    /// Resolve `reference` for a grammar whose syntax admits **no relative
    /// reference at all** (N-Triples, N-Quads, TriX, `HexTuples`).
    ///
    /// A relative reference is [`IriError::NotAbsoluteByGrammar`] regardless of
    /// whether a base is in scope: the base is never applied, because applying it
    /// would accept a document the grammar rejects. This is deliberately a
    /// different error from [`IriError::NoBase`] — supplying a base cannot fix it.
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
            });
        }
        let iri = parse(reference)?;
        if iri.has_scheme() {
            Ok(iri)
        } else {
            Err(IriError::NotAbsoluteByGrammar {
                reference: reference.to_owned(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
