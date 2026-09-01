// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Caller-invoked blank-node recourse: RDF 1.2 **skolemization** and
//! **deskolemization** over the frozen IR, plus the shared whole-dataset
//! term-rewrite driver that `canonical_relabel` (see [`super::canon`]) rides on.
//!
//! Serializer egress is total: a blank label illegal in the target syntax's
//! alphabet is escaped into it (see [`crate::blank_label`]), deterministically
//! and injectively, so no dataset fails to serialize. A caller who wants the
//! labels in the document to be of their own choosing — rather than the
//! escape's mechanical rewrite of, say, a JSON-LD `_:has space` — rewrites the
//! dataset first, with one of two explicit, deterministic operations:
//!
//! - [`super::canon::canonical_relabel`]: relabel every blank to its canonical
//!   `c14n{n}` label (ASCII alphanumerics — legal in every alphabet).
//! - [`skolemize`]: replace every blank node with an IRI under the
//!   caller-supplied authority's `/.well-known/genid/` space, per the RDF 1.2
//!   *Replacing Blank Nodes with IRIs* section (the `genid` well-known URI
//!   suffix, RFC 8615). [`deskolemize`] is its exact inverse.
//!
//! Both are NEW-dataset operations, never serializer modes: the input dataset is
//! untouched, and the serializers keep exactly one behavior.
//!
//! # The authority is caller-supplied — PurRDF mints nothing
//!
//! Skolem IRIs live under an authority the CALLER controls (RDF 1.2: "systems
//! wishing to do this SHOULD mint a new, globally unique IRI"). PurRDF is not an
//! ontology and owns no IRI space, so there is no default authority: an empty,
//! whitespace-bearing, or non-IRI-shaped authority is a hard
//! [`SkolemError::InvalidAuthority`], never a fabricated fallback.
//!
//! # The genid segment encoding
//!
//! A skolem IRI is `{authority}/.well-known/genid/{encoded}`, where `encoded`
//! captures the blank node's `(label, scope)` pair injectively and reversibly
//! using only IRI-path-segment-safe bytes (ASCII alphanumerics and `-`; no
//! percent-encoding, so no interaction with IRI normalizers):
//!
//! ```text
//! encoded  ::= "s" scope "-" body
//! scope    ::= "0" | [1-9] [0-9]*     (canonical decimal of the u32 scope ordinal)
//! body     ::= ( verbatim | escape )*
//! verbatim ::= [0-9A-Za-z]            (a label UTF-8 byte that is ASCII alphanumeric)
//! escape   ::= "-" hex hex            (any other label UTF-8 byte)
//! hex      ::= [0-9a-f]               (lowercase only)
//! ```
//!
//! The form is canonical and therefore unambiguous: the scope's digit run is
//! terminated by the mandatory `-` separator; inside `body` every `-` introduces
//! exactly two hex digits, so the byte stream parses left-to-right without
//! backtracking; and the canonicality rules — no leading zero in `scope`,
//! lowercase hex only, an ASCII-alphanumeric byte MUST be verbatim (never
//! escaped), and the decoded bytes must be valid UTF-8 — mean every `(label,
//! scope)` has exactly one encoding and [`deskolemize`] rejects everything
//! outside that image as [`SkolemError::MalformedGenid`] (corrupt input is
//! refused, never silently passed through).
//!
//! # Determinism
//!
//! Both operations are pure functions of `(dataset, authority)`: no RNG, no
//! UUIDs, no clocks. Skolemizing the same dataset under the same authority
//! always yields the same dataset.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::sync::Arc;

use crate::RdfLiteral;

use super::builder::RdfDatasetBuilder;
use super::dataset::{QuadHandle, RdfDataset, TermRef};
use super::term::{BlankScope, TermId};

/// The well-known path (RFC 8615) under which skolem IRIs are minted, including
/// both surrounding separators: a skolem IRI is
/// `{authority}{GENID_WELL_KNOWN_PATH}{encoded}`.
pub const GENID_WELL_KNOWN_PATH: &str = "/.well-known/genid/";

/// Why [`skolemize`] / [`deskolemize`] refused.
///
/// Every variant is a refusal of the OPERATION, not a degraded output: no
/// variant leaves a partially rewritten dataset behind.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SkolemError {
    /// The caller-supplied authority cannot prefix an IRI: empty, whitespace or
    /// control characters, characters the IRI grammar forbids, a `#` or `?`
    /// (either would land the minted genid path inside a fragment or query
    /// string rather than a well-known path), no scheme, or a trailing `/`
    /// (the operation supplies the path separator itself). PurRDF hardcodes no
    /// default authority, so there is no fallback.
    InvalidAuthority {
        /// The authority exactly as supplied.
        authority: Box<str>,
        /// Which rule it violated.
        reason: &'static str,
    },
    /// The [`skolemize`] input already carries an IRI under the caller's
    /// `/.well-known/genid/` space. Accepting it would make skolemization
    /// non-injective — a pre-existing genid IRI and a freshly skolemized blank
    /// would be indistinguishable to [`deskolemize`] — so the dataset is refused.
    ReservedGenid {
        /// The offending IRI, in full.
        iri: Box<str>,
    },
    /// A [`deskolemize`] input IRI under the caller's genid space whose final
    /// segment does not decode under the documented grammar. Corrupt input is a
    /// hard error, never silently passed through.
    MalformedGenid {
        /// The offending IRI, in full.
        iri: Box<str>,
        /// Which grammar rule the segment violated.
        reason: &'static str,
    },
    /// A [`deskolemize`] input carries a genid IRI under the caller's authority
    /// in a position that cannot hold a blank node — a predicate slot (of a
    /// quad, an annotation, or a quoted triple) or a literal's datatype.
    /// [`skolemize`] can never produce such a dataset (blanks are illegal in
    /// those positions), so the input is corrupt and refused.
    GenidInIriOnlyPosition {
        /// The offending IRI, in full.
        iri: Box<str>,
    },
    /// A [`deskolemize`] input genid IRI decodes to a `(label, scope)` the
    /// dataset ALREADY carries as a live blank node. Decoding it would silently
    /// conflate two distinct nodes into one, so the operation refuses instead.
    BlankCollision {
        /// The decoded label.
        label: Box<str>,
        /// The decoded scope.
        scope: BlankScope,
    },
}

impl std::fmt::Display for SkolemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAuthority { authority, reason } => write!(
                f,
                "the skolem authority {authority:?} is unusable: {reason}; PurRDF mints no \
                 default authority, so the caller must supply an absolute IRI prefix"
            ),
            Self::ReservedGenid { iri } => write!(
                f,
                "the dataset already carries <{iri}> under the supplied authority's \
                 {GENID_WELL_KNOWN_PATH} space; skolemizing it would make the operation \
                 non-invertible, so the dataset is refused"
            ),
            Self::MalformedGenid { iri, reason } => write!(
                f,
                "the genid IRI <{iri}> under the supplied authority does not decode under the \
                 skolem segment grammar ({reason}); corrupt input is refused, not passed through"
            ),
            Self::GenidInIriOnlyPosition { iri } => write!(
                f,
                "the genid IRI <{iri}> under the supplied authority appears in a position that \
                 cannot hold a blank node (a predicate or a literal datatype); no skolemization \
                 can have produced it there, so the input is corrupt and refused"
            ),
            Self::BlankCollision { label, scope } => write!(
                f,
                "deskolemizing would mint blank node ({label:?}, scope {}) which the dataset \
                 already carries; decoding would silently conflate two distinct nodes, so the \
                 operation refuses",
                scope.ordinal()
            ),
        }
    }
}

impl std::error::Error for SkolemError {}

/// Validate the caller-supplied authority and return the full genid prefix
/// `{authority}/.well-known/genid/`.
fn genid_prefix(authority: &str) -> Result<String, SkolemError> {
    let refuse = |reason: &'static str| SkolemError::InvalidAuthority {
        authority: authority.into(),
        reason,
    };
    if authority.is_empty() {
        return Err(refuse("it is empty"));
    }
    if authority
        .chars()
        .any(|c| c.is_whitespace() || c.is_control())
    {
        return Err(refuse("it contains whitespace or control characters"));
    }
    if authority
        .chars()
        .any(|c| matches!(c, '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\'))
    {
        return Err(refuse("it contains a character the IRI grammar forbids"));
    }
    if authority.contains('#') {
        return Err(refuse(
            "it contains '#' (the genid path would land inside the fragment, not a well-known \
             path)",
        ));
    }
    if authority.contains('?') {
        return Err(refuse(
            "it contains '?' (the genid path would land inside the query string, not a \
             well-known path)",
        ));
    }
    let Some(colon) = authority.find(':') else {
        return Err(refuse("it has no IRI scheme (no ':')"));
    };
    let scheme = &authority[..colon];
    let mut scheme_chars = scheme.chars();
    let scheme_ok = scheme_chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && scheme_chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if !scheme_ok {
        return Err(refuse("it does not start with a legal IRI scheme"));
    }
    if authority.ends_with('/') {
        return Err(refuse(
            "it ends with '/' (the operation supplies the path separator itself)",
        ));
    }
    Ok(format!("{authority}{GENID_WELL_KNOWN_PATH}"))
}

/// Encode a blank node's `(label, scope)` as the canonical genid path segment
/// (see the module documentation for the grammar and the injectivity argument).
fn encode_blank(label: &str, scope: BlankScope) -> String {
    let mut out = String::with_capacity(label.len() + 8);
    out.push('s');
    let _ = write!(out, "{}", scope.ordinal());
    out.push('-');
    for &byte in label.as_bytes() {
        if byte.is_ascii_alphanumeric() {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "-{byte:02x}");
        }
    }
    out
}

/// Decode a canonical genid path segment back to `(label, scope)`, rejecting
/// (with the violated rule) everything outside [`encode_blank`]'s image.
fn decode_blank(encoded: &str) -> Result<(String, BlankScope), &'static str> {
    let rest = encoded
        .strip_prefix('s')
        .ok_or("the segment does not start with 's'")?;
    let digits_end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let digits = &rest[..digits_end];
    if digits.is_empty() {
        return Err("the scope has no digits");
    }
    if digits.len() > 1 && digits.starts_with('0') {
        return Err("the scope has a leading zero (non-canonical)");
    }
    let scope: u32 = digits
        .parse()
        .map_err(|_| "the scope does not fit in u32")?;
    let body = rest[digits_end..]
        .strip_prefix('-')
        .ok_or("the scope is not followed by the '-' separator")?;

    let bytes = body.as_bytes();
    let mut label_bytes: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'-' => {
                let (Some(&hi), Some(&lo)) = (bytes.get(i + 1), bytes.get(i + 2)) else {
                    return Err("a '-' escape is not followed by two hex digits");
                };
                let (Some(hi), Some(lo)) = (lower_hex_value(hi), lower_hex_value(lo)) else {
                    return Err("a '-' escape carries a non-lowercase-hex digit");
                };
                let byte = hi * 16 + lo;
                if byte.is_ascii_alphanumeric() {
                    return Err("an ASCII-alphanumeric byte is escaped (non-canonical)");
                }
                label_bytes.push(byte);
                i += 3;
            }
            c if c.is_ascii_alphanumeric() => {
                label_bytes.push(c);
                i += 1;
            }
            _ => return Err("the body carries a byte outside the segment alphabet"),
        }
    }
    let label = String::from_utf8(label_bytes)
        .map_err(|_| "the escaped label bytes are not valid UTF-8")?;
    Ok((label, BlankScope(scope)))
}

/// The value of a lowercase hex digit (`[0-9a-f]`), or `None` — uppercase is
/// rejected so every byte has exactly one escape spelling (canonicality).
const fn lower_hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

/// How a whole-dataset rewrite maps the two leaf term kinds a rewrite may
/// touch. [`rebuild_dataset`] drives an implementation over every term surface
/// the dataset model exposes (quads, quoted-triple components, reifier
/// bindings, annotations, named-graph declarations, literal datatypes).
pub(crate) trait TermMapper {
    /// The rewrite's refusal type.
    type Error;

    /// Map one blank node (identified by its source-dataset id and resolved
    /// `(label, scope)`) to a term interned in `builder`.
    fn map_blank(
        &mut self,
        builder: &mut RdfDatasetBuilder,
        id: TermId,
        label: &str,
        scope: BlankScope,
    ) -> Result<TermId, Self::Error>;

    /// Map one IRI to a term interned in `builder`. `iri_only` is `true` when
    /// the IRI sits in a position that cannot hold a blank node (a predicate
    /// slot or a literal's datatype), so a mapper that mints blanks from IRIs
    /// must refuse rather than produce an invalid dataset.
    fn map_iri(
        &mut self,
        builder: &mut RdfDatasetBuilder,
        iri: &str,
        iri_only: bool,
    ) -> Result<TermId, Self::Error>;
}

/// Re-intern the term at `id` into `builder`, routing blanks and IRIs through
/// `mapper` and recursing into quoted-triple components and literal datatypes.
fn reintern<M: TermMapper>(
    ds: &RdfDataset,
    builder: &mut RdfDatasetBuilder,
    id: TermId,
    mapper: &mut M,
    iri_only: bool,
) -> Result<TermId, M::Error> {
    match ds.resolve(id) {
        TermRef::Iri(iri) => mapper.map_iri(builder, iri, iri_only),
        TermRef::Blank { label, scope } => mapper.map_blank(builder, id, label, scope),
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let dt = match ds.resolve(datatype) {
                TermRef::Iri(iri) => iri,
                other => unreachable!("a literal datatype must be an IRI, got {other:?}"),
            };
            // Route the datatype through the mapper so an IRI-rewriting mapper
            // gets to refuse it (`iri_only`); the returned id is the same one
            // `intern_literal` re-derives from the string below.
            let _ = mapper.map_iri(builder, dt, true)?;
            let lexical_form = remap_composite_lexical(ds, builder, lexical, dt, mapper)?;
            Ok(builder.intern_literal(RdfLiteral {
                lexical_form,
                datatype: Some(dt.to_owned()),
                language: language.map(str::to_owned),
                direction,
            }))
        }
        TermRef::Triple { s, p, o } => {
            let s = reintern(ds, builder, s, mapper, false)?;
            let p = reintern(ds, builder, p, mapper, true)?;
            let o = reintern(ds, builder, o, mapper, false)?;
            Ok(builder.intern_triple(s, p, o))
        }
    }
}

/// Rewrite the terms a composite (`cdt:List` / `cdt:Map`) literal EMBEDS in its
/// lexical form through `mapper`, so a whole-dataset term rewrite reaches inside
/// literals instead of stopping at them.
///
/// A blank node written inside a composite literal is a blank node of the graph
/// (see [`crate::cdt_blank`]). A rewrite that renamed only the term-position
/// occurrences would leave the embedded one DANGLING — naming a label the
/// rewritten dataset no longer has — which is exactly what made
/// [`super::compare::datasets_isomorphic`] wrong for such a dataset. Embedded
/// IRIs go through [`TermMapper::map_iri`] for the mirror-image reason: a
/// skolemization that turned an embedded blank into a genid IRI would otherwise
/// not be invertible.
///
/// Any other datatype returns its lexical form unchanged and untouched.
fn remap_composite_lexical<M: TermMapper>(
    ds: &RdfDataset,
    builder: &mut RdfDatasetBuilder,
    lexical: &str,
    datatype: &str,
    mapper: &mut M,
) -> Result<String, M::Error> {
    use std::collections::BTreeMap;

    use crate::blank_label::{LabelAlphabet, decode_blank_label};

    if !crate::cdt_blank::is_cdt_datatype(datatype) {
        return Ok(lexical.to_owned());
    }

    // The mapper is fallible and the rewriter's callbacks are not, so every
    // mapping is resolved UP FRONT and the rewrite is then a pure substitution.
    let mut blanks: BTreeMap<(String, BlankScope), String> = BTreeMap::new();
    for (label, scope) in crate::cdt_blank::cdt_embedded_blanks(lexical, datatype) {
        if blanks.contains_key(&(label.clone(), scope)) {
            continue;
        }
        // `intern_literal` interned every embedded pair, so this resolves; a pair
        // the source dataset does not hold names no node of it and is left alone.
        let Some(id) = ds.term_id_by_blank(&label, scope) else {
            continue;
        };
        let mapped = mapper.map_blank(builder, id, &label, scope)?;
        blanks.insert((label, scope), element_text(builder, mapped));
    }

    let mut iris: BTreeMap<String, String> = BTreeMap::new();
    let mut failure: Option<M::Error> = None;
    let out = crate::cdt_blank::rewrite_cdt_terms(
        lexical,
        datatype,
        &mut |token| {
            let (label, scope) = decode_blank_label(token, LabelAlphabet::BlankNodeLabel);
            blanks.get(&(label.into_owned(), scope)).cloned()
        },
        &mut |iri, iri_only| {
            if failure.is_some() {
                return None;
            }
            if let Some(text) = iris.get(iri) {
                return Some(text.clone());
            }
            match mapper.map_iri(builder, iri, iri_only) {
                Ok(mapped) => {
                    let text = element_text(builder, mapped);
                    iris.insert(iri.to_owned(), text.clone());
                    Some(text)
                }
                Err(err) => {
                    failure = Some(err);
                    None
                }
            }
        },
    );
    if let Some(err) = failure {
        return Err(err);
    }
    let changed = matches!(out, std::borrow::Cow::Owned(_));
    let out = out.into_owned();

    // A mapper that produced a token the composite grammar cannot hold would
    // corrupt the store silently. Refuse loudly instead — the rewrite is a
    // programming error in the mapper, not a property of the input data, and a
    // rewrite that changed nothing cannot have introduced one.
    assert!(
        !changed || purrdf_cdt::parse_cdt_by_iri(&out, datatype).is_ok(),
        "a term rewrite produced an ill-formed composite lexical form: {out}"
    );
    Ok(out)
}

/// The composite-lexical element text a mapped term is written back as.
fn element_text(builder: &RdfDatasetBuilder, id: TermId) -> String {
    use crate::blank_label::{LabelAlphabet, encode_blank_label};

    match builder.resolve(id) {
        TermRef::Iri(iri) => {
            let mut out = String::with_capacity(iri.len() + 2);
            out.push('<');
            write_iriref_escaped(iri, &mut out);
            out.push('>');
            out
        }
        TermRef::Blank { label, scope } => {
            format!(
                "_:{}",
                encode_blank_label(label, scope, LabelAlphabet::BlankNodeLabel)
            )
        }
        other => {
            unreachable!("a term rewrite may only produce an IRI or a blank node, got {other:?}")
        }
    }
}

/// Write an IRI as a composite `IRIREF` body, `\u`-escaping every character the
/// production excludes so the result is always a legal token.
fn write_iriref_escaped(iri: &str, out: &mut String) {
    for ch in iri.chars() {
        match ch {
            // `'` is legal in an `IRIREF`, but a composite literal may be
            // embedded inside a single-quoted string, where a raw `'` would
            // close it. Escaping it costs nothing and is always legal.
            '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\' | '\'' => {
                let _ = write!(out, "\\u{:04X}", ch as u32);
            }
            c if (c as u32) <= 0x20 => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
}

/// Rebuild `ds` as a NEW frozen dataset with every term routed through
/// `mapper`, preserving all statement surfaces: quads (with their source
/// locations), reifier bindings, annotations, and named-graph declarations.
///
/// The NON-SERIALIZED derived side tables survive the rewrite rather than
/// resetting: `ds`'s [`ContentIdScheme`](crate::ContentIdScheme) (and
/// derivation-predicate IRI, if configured) is read back via
/// [`RdfDataset::content_id_scheme`] and threaded onto the fresh builder via
/// [`RdfDatasetBuilder::with_content_addressing`], so `content_id`/`content_ids`
/// on the output are RE-DERIVED from the rewritten IRI bytes — not copied: a
/// term whose bytes a rewrite actually changed (e.g. the caller's own content
/// IRIs, if any) is re-recognized against its NEW bytes, which is the correct
/// semantics, not a stale copy. The derivation-predecessor index
/// (`predecessors`/`predecessor_chain`) needs no separate remapping at all: it
/// is not an independent stored table but a lazy decode of the (already
/// carried-forward, unchanged) annotation rows against the configured
/// derivation predicate, so once the predicate IRI is threaded through it
/// resolves on the output exactly as it did on `ds`. When `ds` never
/// configured content addressing, the output carries none either — no
/// fabricated scheme.
///
/// # Panics
/// Never on a mapper that keeps positional validity (the [`TermMapper::map_iri`]
/// `iri_only` contract): a positionally valid rewrite of a valid dataset
/// re-freezes without a diagnostic.
pub(crate) fn rebuild_dataset<M: TermMapper>(
    ds: &RdfDataset,
    mapper: &mut M,
) -> Result<RdfDataset, M::Error> {
    let mut builder = match ds.content_id_scheme() {
        Some(scheme) => {
            let derivation_predicate = ds.derivation_predicate().map(|id| match ds.resolve(id) {
                TermRef::Iri(iri) => iri.to_owned(),
                other => unreachable!(
                    "the frozen derivation-predicate TermId must resolve to an IRI, got {other:?}"
                ),
            });
            RdfDatasetBuilder::with_content_addressing(scheme.clone(), derivation_predicate)
        }
        None => RdfDatasetBuilder::new(),
    };
    for (index, q) in ds.quads().enumerate() {
        let handle = builder.next_quad_handle();
        let s = reintern(ds, &mut builder, q.s, mapper, false)?;
        let p = reintern(ds, &mut builder, q.p, mapper, true)?;
        let o = reintern(ds, &mut builder, q.o, mapper, false)?;
        let g = match q.g {
            Some(g) => Some(reintern(ds, &mut builder, g, mapper, false)?),
            None => None,
        };
        builder.push_quad(s, p, o, g);
        if let Some(loc) = ds.location_of(QuadHandle::from_index(index as u32)) {
            builder.attach_location(handle, loc.clone());
        }
    }
    for (r, t, g) in ds.reifiers_with_graph() {
        let r = reintern(ds, &mut builder, r, mapper, false)?;
        let t = reintern(ds, &mut builder, t, mapper, false)?;
        let g = match g {
            Some(g) => Some(reintern(ds, &mut builder, g, mapper, false)?),
            None => None,
        };
        builder.push_reifier_in_graph(r, t, g);
    }
    for (r, p, o, g) in ds.annotations_with_graph() {
        let r = reintern(ds, &mut builder, r, mapper, false)?;
        let p = reintern(ds, &mut builder, p, mapper, true)?;
        let o = reintern(ds, &mut builder, o, mapper, false)?;
        let g = match g {
            Some(g) => Some(reintern(ds, &mut builder, g, mapper, false)?),
            None => None,
        };
        builder.push_annotation_in_graph(r, p, o, g);
    }
    for g in ds.named_graphs() {
        let g = reintern(ds, &mut builder, g, mapper, false)?;
        builder.declare_named_graph(g);
    }
    Ok(Arc::try_unwrap(
        builder
            .freeze()
            .expect("a positionally valid term rewrite of a valid dataset is valid"),
    )
    .unwrap_or_else(|arc| arc.owned_snapshot()))
}

/// The [`skolemize`] mapper: every blank becomes a genid IRI; a pre-existing
/// IRI under the same genid space is refused (invertibility).
struct Skolemizer {
    /// `{authority}/.well-known/genid/`.
    prefix: String,
}

impl TermMapper for Skolemizer {
    type Error = SkolemError;

    fn map_blank(
        &mut self,
        builder: &mut RdfDatasetBuilder,
        _id: TermId,
        label: &str,
        scope: BlankScope,
    ) -> Result<TermId, SkolemError> {
        let iri = format!("{}{}", self.prefix, encode_blank(label, scope));
        Ok(builder.intern_iri(&iri))
    }

    fn map_iri(
        &mut self,
        builder: &mut RdfDatasetBuilder,
        iri: &str,
        _iri_only: bool,
    ) -> Result<TermId, SkolemError> {
        if iri.starts_with(&self.prefix) {
            return Err(SkolemError::ReservedGenid { iri: iri.into() });
        }
        Ok(builder.intern_iri(iri))
    }
}

/// The [`deskolemize`] mapper: exactly the IRIs under the caller's genid space
/// decode back to blanks; everything else passes through unchanged.
struct Deskolemizer {
    /// `{authority}/.well-known/genid/`.
    prefix: String,
    /// The `(label, scope)` pairs the dataset already carries as blanks — a
    /// decode landing on one of these is a refused conflation.
    existing: BTreeSet<(Box<str>, BlankScope)>,
}

impl TermMapper for Deskolemizer {
    type Error = SkolemError;

    fn map_blank(
        &mut self,
        builder: &mut RdfDatasetBuilder,
        _id: TermId,
        label: &str,
        scope: BlankScope,
    ) -> Result<TermId, SkolemError> {
        Ok(builder.intern_blank(label, scope))
    }

    fn map_iri(
        &mut self,
        builder: &mut RdfDatasetBuilder,
        iri: &str,
        iri_only: bool,
    ) -> Result<TermId, SkolemError> {
        let Some(segment) = iri.strip_prefix(&self.prefix) else {
            // Not under OUR authority's genid space: untouched — including any
            // other authority's `/.well-known/genid/` IRIs.
            return Ok(builder.intern_iri(iri));
        };
        if iri_only {
            return Err(SkolemError::GenidInIriOnlyPosition { iri: iri.into() });
        }
        let (label, scope) =
            decode_blank(segment).map_err(|reason| SkolemError::MalformedGenid {
                iri: iri.into(),
                reason,
            })?;
        if self.existing.contains(&(label.as_str().into(), scope)) {
            return Err(SkolemError::BlankCollision {
                label: label.into_boxed_str(),
                scope,
            });
        }
        Ok(builder.intern_blank(&label, scope))
    }
}

/// Collect every `(label, scope)` blank the dataset carries in any statement
/// surface (quads, quoted triples, reifiers, annotations, graph declarations).
fn existing_blanks(ds: &RdfDataset) -> BTreeSet<(Box<str>, BlankScope)> {
    fn walk(ds: &RdfDataset, id: TermId, out: &mut BTreeSet<(Box<str>, BlankScope)>) {
        match ds.resolve(id) {
            TermRef::Blank { label, scope } => {
                out.insert((label.into(), scope));
            }
            TermRef::Triple { s, p, o } => {
                walk(ds, s, out);
                walk(ds, p, out);
                walk(ds, o, out);
            }
            TermRef::Iri(_) | TermRef::Literal { .. } => {}
        }
    }
    let mut out = BTreeSet::new();
    for q in ds.quads() {
        walk(ds, q.s, &mut out);
        walk(ds, q.o, &mut out);
        if let Some(g) = q.g {
            walk(ds, g, &mut out);
        }
    }
    for (r, t, g) in ds.reifiers_with_graph() {
        walk(ds, r, &mut out);
        walk(ds, t, &mut out);
        if let Some(g) = g {
            walk(ds, g, &mut out);
        }
    }
    for (r, _p, o, g) in ds.annotations_with_graph() {
        walk(ds, r, &mut out);
        walk(ds, o, &mut out);
        if let Some(g) = g {
            walk(ds, g, &mut out);
        }
    }
    for g in ds.named_graphs() {
        walk(ds, g, &mut out);
    }
    out
}

/// Skolemize `dataset` under the caller-supplied `authority`: return a NEW
/// frozen dataset in which every blank node — in subject, object, and
/// graph-name position, inside quoted-triple terms, as a reifier, in
/// annotations, and in named-graph declarations — is replaced by the IRI
/// `{authority}/.well-known/genid/{encoded}`, per the RDF 1.2 skolemization
/// scheme. `encoded` is the injective, reversible segment encoding of the
/// blank's `(label, scope)` documented at module level, so [`deskolemize`]
/// under the same authority reconstructs the original dataset exactly (labels
/// AND scopes). All other terms, quads, reifiers, annotations, and quad source
/// locations are preserved. The non-serialized derived side tables
/// (`content_ids`, `predecessors`/`predecessor_chain`) survive the rewrite too
/// (the crate-private `rebuild_dataset` helper implements the exact contract):
/// content addressing re-derives from the output's IRI bytes under `dataset`'s own
/// [`ContentIdScheme`](crate::ContentIdScheme), and the predecessor index
/// resolves over the carried-forward annotation table.
///
/// Deterministic: a pure function of `(dataset, authority)` — no RNG, no
/// clocks, no global state.
///
/// # Errors
/// [`SkolemError::InvalidAuthority`] if `authority` is empty, carries
/// whitespace/control or IRI-forbidden characters, carries a `#` or `?`, has
/// no scheme, or ends with `/`; [`SkolemError::ReservedGenid`] if the dataset
/// already carries an IRI under `{authority}/.well-known/genid/` (skolemizing
/// it would make the operation non-invertible).
pub fn skolemize(dataset: &RdfDataset, authority: &str) -> Result<RdfDataset, SkolemError> {
    let prefix = genid_prefix(authority)?;
    rebuild_dataset(dataset, &mut Skolemizer { prefix })
}

/// Deskolemize `dataset` under the caller-supplied `authority`: return a NEW
/// frozen dataset in which exactly the IRIs under
/// `{authority}/.well-known/genid/` are decoded back to the blank nodes
/// (`label`, `scope`) they encode, in every position blanks may occupy. IRIs
/// under any OTHER authority's genid path are untouched. The exact inverse of
/// [`skolemize`] under the same authority. As with [`skolemize`], the
/// non-serialized derived side tables (`content_ids`,
/// `predecessors`/`predecessor_chain`) survive the rewrite (see the
/// crate-private `rebuild_dataset` helper).
///
/// Deterministic: a pure function of `(dataset, authority)`.
///
/// # Errors
/// [`SkolemError::InvalidAuthority`] as for [`skolemize`];
/// [`SkolemError::MalformedGenid`] if a genid IRI under this authority does not
/// decode under the documented grammar (corrupt input is refused, never
/// silently passed through); [`SkolemError::GenidInIriOnlyPosition`] if such an
/// IRI sits in a predicate or datatype slot (a blank node is illegal there, so
/// no skolemization can have produced it); [`SkolemError::BlankCollision`] if a
/// decode lands on a `(label, scope)` the dataset already carries as a blank
/// (decoding would silently conflate two distinct nodes).
pub fn deskolemize(dataset: &RdfDataset, authority: &str) -> Result<RdfDataset, SkolemError> {
    let prefix = genid_prefix(authority)?;
    let existing = existing_blanks(dataset);
    rebuild_dataset(dataset, &mut Deskolemizer { prefix, existing })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::canon::canonicalize;
    use crate::ir::{RdfDatasetBuilder, canonical_relabel};

    const AUTHORITY: &str = "https://example.org";
    const PREFIX: &str = "https://example.org/.well-known/genid/";

    /// A dataset exercising every blank surface with hostile labels: spaces,
    /// control bytes, unicode, non-default scopes, a blank graph name, blanks
    /// inside a quoted-triple term, a blank reifier, and an annotation.
    fn hostile_dataset() -> RdfDataset {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        let bad = b.intern_blank("bad label", BlankScope::DEFAULT);
        let ctl = b.intern_blank("ctl\u{1}\u{7f}byte", BlankScope::DEFAULT);
        let uni = b.intern_blank("日本-λ", BlankScope(3));
        let bg = b.intern_blank("graph blank", BlankScope(2));
        b.push_quad(bad, p, ctl, None);
        b.push_quad(uni, p, o, Some(bg));
        let qs = b.intern_blank("quoted subject", BlankScope::DEFAULT);
        let triple = b.intern_triple(qs, p, o);
        b.push_quad(bad, p, triple, None);
        let r = b.intern_blank("reifier blank", BlankScope(5));
        b.push_reifier(r, triple);
        b.push_annotation(r, p, ctl);
        Arc::try_unwrap(b.freeze().expect("valid")).unwrap_or_else(|arc| arc.owned_snapshot())
    }

    /// Every blank `(label, scope)` reachable in `ds`'s statement surfaces.
    fn blank_set(ds: &RdfDataset) -> BTreeSet<(Box<str>, BlankScope)> {
        existing_blanks(ds)
    }

    // -------------------------------------------------------------------
    // skolemize ∘ deskolemize = identity
    // -------------------------------------------------------------------

    #[test]
    fn skolemize_then_deskolemize_is_the_identity_on_a_hostile_dataset() {
        let ds = hostile_dataset();
        let sk = skolemize(&ds, AUTHORITY).expect("skolemize");
        // No blanks remain anywhere, and every minted IRI is path-safe.
        assert!(
            blank_set(&sk).is_empty(),
            "skolemize must convert ALL blanks"
        );
        for i in 0..sk.term_count() {
            if let TermRef::Iri(iri) = sk.resolve(TermId::from_index(i as u32))
                && let Some(segment) = iri.strip_prefix(PREFIX)
            {
                assert!(
                    segment
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-'),
                    "genid segment must stay IRI-path-safe: {segment:?}"
                );
            }
        }
        let back = deskolemize(&sk, AUTHORITY).expect("deskolemize");
        assert_eq!(
            canonicalize(&back).nquads,
            canonicalize(&ds).nquads,
            "round trip must preserve the dataset up to canonical bytes"
        );
        assert_eq!(
            blank_set(&back),
            blank_set(&ds),
            "round trip must reconstruct the exact (label, scope) pairs"
        );
    }

    #[test]
    fn skolemize_is_deterministic() {
        let ds = hostile_dataset();
        let a = skolemize(&ds, AUTHORITY).expect("skolemize");
        let b = skolemize(&ds, AUTHORITY).expect("skolemize");
        assert_eq!(canonicalize(&a).nquads, canonicalize(&b).nquads);
    }

    // -------------------------------------------------------------------
    // deskolemize scoping and refusals
    // -------------------------------------------------------------------

    #[test]
    fn deskolemize_leaves_foreign_genid_authorities_untouched() {
        let foreign = "https://other.example/.well-known/genid/s0-x";
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(foreign);
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, p, o, None);
        let ds = b.freeze().expect("valid");
        let out = deskolemize(&ds, AUTHORITY).expect("foreign genids are not ours to decode");
        let survived = (0..out.term_count())
            .any(|i| matches!(out.resolve(TermId::from_index(i as u32)), TermRef::Iri(iri) if iri == foreign));
        assert!(
            survived,
            "the foreign genid IRI must pass through unchanged"
        );
        assert!(blank_set(&out).is_empty());
    }

    #[test]
    fn deskolemize_refuses_a_malformed_genid_under_our_authority() {
        for segment in [
            "s01-a",
            "x0-a",
            "s0",
            "s0--4",
            "s0--4A",
            "s0--41",
            "extra/segments",
        ] {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_iri(&format!("{PREFIX}{segment}"));
            let p = b.intern_iri("http://example.org/p");
            let o = b.intern_iri("http://example.org/o");
            b.push_quad(s, p, o, None);
            let ds = b.freeze().expect("valid");
            assert!(
                matches!(
                    deskolemize(&ds, AUTHORITY),
                    Err(SkolemError::MalformedGenid { .. })
                ),
                "segment {segment:?} must be refused as corrupt"
            );
        }
    }

    #[test]
    fn deskolemize_refuses_a_genid_in_predicate_or_datatype_position() {
        // Predicate.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let genid_p = b.intern_iri(&format!("{PREFIX}s0-x"));
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, genid_p, o, None);
        let ds = b.freeze().expect("valid");
        assert!(matches!(
            deskolemize(&ds, AUTHORITY),
            Err(SkolemError::GenidInIriOnlyPosition { .. })
        ));

        // Literal datatype.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let lit = b.intern_literal(RdfLiteral::typed("5", format!("{PREFIX}s0-x")));
        b.push_quad(s, p, lit, None);
        let ds = b.freeze().expect("valid");
        assert!(matches!(
            deskolemize(&ds, AUTHORITY),
            Err(SkolemError::GenidInIriOnlyPosition { .. })
        ));
    }

    #[test]
    fn deskolemize_refuses_a_decode_that_collides_with_a_live_blank() {
        let mut b = RdfDatasetBuilder::new();
        let existing = b.intern_blank("x", BlankScope::DEFAULT);
        let genid = b.intern_iri(&format!("{PREFIX}s0-x"));
        let p = b.intern_iri("http://example.org/p");
        b.push_quad(existing, p, genid, None);
        let ds = b.freeze().expect("valid");
        match deskolemize(&ds, AUTHORITY) {
            Err(SkolemError::BlankCollision { label, scope }) => {
                assert_eq!(&*label, "x");
                assert_eq!(scope, BlankScope::DEFAULT);
            }
            other => panic!("a conflating decode must be refused; got {other:?}"),
        }
    }

    #[test]
    fn skolemize_refuses_a_dataset_already_carrying_our_genid_space() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(&format!("{PREFIX}s0-x"));
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_blank("b", BlankScope::DEFAULT);
        b.push_quad(s, p, o, None);
        let ds = b.freeze().expect("valid");
        assert!(matches!(
            skolemize(&ds, AUTHORITY),
            Err(SkolemError::ReservedGenid { .. })
        ));
    }

    // -------------------------------------------------------------------
    // Authority validation
    // -------------------------------------------------------------------

    #[test]
    fn hostile_authorities_are_refused_by_both_operations() {
        let ds = hostile_dataset();
        for authority in [
            "",
            "   ",
            "no-scheme",
            "1http://x",
            "https://example.org/",
            "http://exa mple.org",
            "http://x<y>",
            "https://example.org#frag",
            "https://example.org?q=1",
        ] {
            assert!(
                matches!(
                    skolemize(&ds, authority),
                    Err(SkolemError::InvalidAuthority { .. })
                ),
                "skolemize must refuse authority {authority:?}"
            );
            assert!(
                matches!(
                    deskolemize(&ds, authority),
                    Err(SkolemError::InvalidAuthority { .. })
                ),
                "deskolemize must refuse authority {authority:?}"
            );
        }
    }

    // -------------------------------------------------------------------
    // The segment encoding itself
    // -------------------------------------------------------------------

    #[test]
    fn encoding_is_injective_on_a_hostile_pair_set() {
        let pairs: &[(&str, BlankScope)] = &[
            ("a", BlankScope::DEFAULT),
            ("a", BlankScope(1)),
            ("s1-a", BlankScope::DEFAULT),
            ("a-62", BlankScope::DEFAULT),
            ("ab", BlankScope::DEFAULT),
            ("bad label", BlankScope::DEFAULT),
            ("bad-label", BlankScope::DEFAULT),
            ("bad+label", BlankScope::DEFAULT),
            ("\u{1}", BlankScope::DEFAULT),
            ("日", BlankScope::DEFAULT),
            ("", BlankScope::DEFAULT),
            ("", BlankScope(1)),
            ("a", BlankScope(12)),
            ("a12", BlankScope::DEFAULT),
        ];
        let mut seen = std::collections::BTreeMap::new();
        for &(label, scope) in pairs {
            let encoded = encode_blank(label, scope);
            assert!(
                encoded
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-'),
                "encoding must stay in the path-safe alphabet: {encoded:?}"
            );
            if let Some(previous) = seen.insert(encoded.clone(), (label, scope)) {
                panic!(
                    "encoding {encoded:?} conflates {previous:?} with {:?}",
                    (label, scope)
                );
            }
        }
    }

    #[test]
    fn encoding_round_trips_through_decoding() {
        for (label, scope) in [
            ("bad label", BlankScope::DEFAULT),
            ("ctl\u{1}\u{7f}", BlankScope(7)),
            ("日本-λ", BlankScope(3)),
            ("", BlankScope::DEFAULT),
            ("plain0", BlankScope(u32::MAX)),
        ] {
            let encoded = encode_blank(label, scope);
            let decoded = decode_blank(&encoded).expect("every encoding decodes");
            assert_eq!(decoded, (label.to_owned(), scope), "via {encoded:?}");
        }
    }

    #[test]
    fn decoding_rejects_everything_outside_the_canonical_image() {
        for bad in [
            "",
            "s",
            "s-",
            "s0",
            "s01-a",
            "s4294967296-a",
            "x0-a",
            "0-a",
            "s0-A-",
            "s0--4",
            "s0--4A",
            "s0--4g",
            "s0--41",
            "s0-a b",
            "s0--ff-ff",
        ] {
            assert!(decode_blank(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    // -------------------------------------------------------------------
    // The two recourses agree on what they preserve
    // -------------------------------------------------------------------

    #[test]
    fn the_rewrite_preserves_quad_source_locations() {
        let mut b = RdfDatasetBuilder::new();
        let handle = b.next_quad_handle();
        let s = b.intern_blank("bad label", BlankScope::DEFAULT);
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, p, o, None);
        let loc = crate::RdfLocation {
            path: Some("input.jsonld".to_owned()),
            line: Some(7),
            column: Some(3),
            ..Default::default()
        };
        b.attach_location(handle, loc.clone());
        let ds = b.freeze().expect("valid");
        let sk = skolemize(&ds, AUTHORITY).expect("skolemize");
        let back = deskolemize(&sk, AUTHORITY).expect("deskolemize");
        for out in [&sk, &back] {
            assert_eq!(
                out.location_of(QuadHandle::from_index(0)),
                Some(&loc),
                "the rewrite must carry the quad's source location"
            );
        }
    }

    #[test]
    fn skolemize_and_canonical_relabel_both_preserve_isomorphism_class() {
        let ds = hostile_dataset();
        let relabeled = canonical_relabel(&ds).expect("relabel");
        let round = deskolemize(&skolemize(&ds, AUTHORITY).expect("skolemize"), AUTHORITY)
            .expect("deskolemize");
        assert_eq!(canonicalize(&relabeled).nquads, canonicalize(&ds).nquads);
        assert_eq!(canonicalize(&round).nquads, canonicalize(&ds).nquads);
    }

    /// Find the [`TermId`] of an interned IRI by linear scan. A rewrite mints a
    /// fresh term table, so a test that wants to assert on the OUTPUT's view of
    /// a particular IRI (rather than the source's `TermId`, which is only valid
    /// in the source) must look it up by value.
    fn find_iri(ds: &RdfDataset, iri: &str) -> TermId {
        (0..ds.term_count())
            .map(|i| TermId::from_index(i as u32))
            .find(|&id| matches!(ds.resolve(id), TermRef::Iri(found) if found == iri))
            .unwrap_or_else(|| panic!("IRI {iri:?} not found in the rewritten dataset"))
    }

    #[test]
    fn rewrites_carry_content_addressing_and_predecessors_forward() {
        // The true contract (see `rebuild_dataset`): the source's ContentIdScheme
        // and derivation predicate are threaded onto the rewrite's builder, so
        // content addressing RE-DERIVES over the output's (here: unchanged, since
        // only the blank node is rewritten) IRI bytes, and the predecessor index
        // — a lazy decode of the carried-forward annotation table — resolves on
        // the output exactly as it did on the source.
        let scheme = crate::content_id::ContentIdScheme::new("blake3:").expect("valid scheme");
        const DERIVED_FROM: &str = "http://example.org/derivedFrom";
        const SUCCESSOR_IRI: &str =
            "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        const PREDECESSOR_IRI: &str =
            "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

        let mut b =
            RdfDatasetBuilder::with_content_addressing(scheme.clone(), Some(DERIVED_FROM.into()));
        let successor = b.intern_iri(SUCCESSOR_IRI);
        let predecessor = b.intern_iri(PREDECESSOR_IRI);
        let derived_from = b.intern_iri(DERIVED_FROM);
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_blank("hostile label", BlankScope::DEFAULT);
        b.push_quad(successor, p, o, None);
        b.push_annotation(successor, derived_from, predecessor);
        let ds = b.freeze().expect("freeze");

        assert_eq!(
            ds.content_ids().count(),
            2,
            "source recognizes both content IRIs"
        );
        assert_eq!(
            ds.predecessors(successor),
            &[predecessor],
            "source resolves the configured derivation predicate"
        );

        let sk = skolemize(&ds, AUTHORITY).expect("skolemize");
        let desk = deskolemize(&sk, AUTHORITY).expect("deskolemize");
        let relabeled = canonical_relabel(&ds).expect("relabel");
        for (name, out) in [
            ("skolemize", &sk),
            ("deskolemize", &desk),
            ("canonical_relabel", &relabeled),
        ] {
            assert_eq!(
                out.content_id_scheme(),
                Some(&scheme),
                "{name}: the ContentIdScheme survives the rewrite"
            );
            assert_eq!(
                out.content_ids().count(),
                2,
                "{name}: content addressing re-derives over the rewritten output"
            );
            let out_successor = find_iri(out, SUCCESSOR_IRI);
            let out_predecessor = find_iri(out, PREDECESSOR_IRI);
            assert_eq!(
                out.predecessors(out_successor),
                &[out_predecessor],
                "{name}: the predecessor index resolves on the rewritten output"
            );
        }
    }

    #[test]
    fn rewrites_of_a_dataset_without_content_addressing_fabricate_nothing() {
        // The other half of the contract: a source that never configured content
        // addressing must not have one conjured for it — the output's scheme and
        // content-id table stay exactly as absent/empty as the source's.
        let ds = hostile_dataset();
        assert_eq!(
            ds.content_id_scheme(),
            None,
            "source never configured a scheme"
        );
        assert_eq!(ds.content_ids().count(), 0);

        let sk = skolemize(&ds, AUTHORITY).expect("skolemize");
        let relabeled = canonical_relabel(&ds).expect("relabel");
        for (name, out) in [("skolemize", &sk), ("canonical_relabel", &relabeled)] {
            assert!(
                out.content_id_scheme().is_none(),
                "{name}: no scheme to fabricate"
            );
            assert_eq!(
                out.content_ids().count(),
                0,
                "{name}: no content ids to fabricate"
            );
        }
    }
}
