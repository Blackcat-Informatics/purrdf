// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Blank-node identity for SEP-0009 composite-datatype (`cdt:List` / `cdt:Map`)
//! literals.
//!
//! A composite literal's lexical form is not opaque text. It may hold
//! `BLANK_NODE_LABEL` tokens, and those tokens **denote blank nodes in the same
//! scope as the surrounding document** — exactly as a bare `_:b` written in a
//! subject or object position does. This module is the one place that fact is
//! implemented, and every ingress path, the RDFC-1.0 canonicalizer and the
//! whole-dataset rewriters route through it.
//!
//! # The scoping rule
//!
//! One parsed document (one input stream, one query, one carrier segment) has
//! exactly **one** blank-node scope. Every `BLANK_NODE_LABEL` occurring anywhere
//! in that document resolves through that single scope:
//!
//! 1. in a term position (`_:b` as a subject, object, graph or triple-term
//!    component), and
//! 2. inside the lexical form of a `cdt:List` / `cdt:Map` literal, at **any**
//!    nesting depth — whether the nesting is a direct `[…]` / `{…}` sub-value or
//!    a composite-typed literal embedded as a string
//!    (`"[_:b, '[_:b]'^^<…/List>]"^^cdt:List`).
//!
//! Consequently two occurrences of the same label in one document are the same
//! node no matter where they occur, and two occurrences of the same label in
//! DIFFERENT documents are different nodes, because the documents carry
//! different scopes. Nesting depth never opens a new scope.
//!
//! # Why the ingress rewrite is not canonicalization
//!
//! The IR identifies a blank node by the pair `(label, scope)`
//! ([`BlankScope`]). A blank node written as a term keeps that pair
//! structurally, in the interner. A blank node written INSIDE a literal has
//! only the literal's bytes to keep it in, so the pair has to be spelled into
//! those bytes. [`bind_cdt_blank_labels`] does exactly that and nothing else: it
//! rewrites the `BLANK_NODE_LABEL` token spans, in place, to the canonical
//! `(label, scope)` spelling that
//! [`encode_blank_label`] produces, and
//! leaves **every other byte of the lexical form untouched** — whitespace,
//! numeric spellings, map-entry order, quote style, escape spellings and
//! datatype IRIs all survive byte for byte.
//!
//! That is a reference resolution, not a normalization. It is the same kind of
//! act as resolving a relative IRI against the document base, which every parser
//! already performs inside term positions without anyone calling it
//! canonicalization: the token in the document is a *name in a scope*, and
//! ingress is where that name is bound. Canonicalizing the value —
//! [`purrdf_cdt::canonical_lexical`] — would reorder map entries and normalize
//! numeric lexical forms, and this module never calls it.
//!
//! It is the single documented carve-out from the workspace rule that a
//! literal's lexical form is preserved byte for byte, it applies to exactly two
//! datatype IRIs, and it is applied exactly **once**, at ingress, at the same
//! moment and by the same `(label, scope)` rule as the document's bare blank
//! terms. Under a [`BlankBinding::Decoded`] binding — every text syntax — it is
//! additionally a fixpoint, because encoding an already-decoded token
//! reproduces it byte for byte.
//!
//! # An ill-formed composite literal rejects its document
//!
//! [`bind_cdt_blank_labels`] parses the lexical form with the real SEP-0009
//! grammar ([`purrdf_cdt::parse_cdt_by_iri`]) and returns
//! [`CdtBlankError::Malformed`] when it does not parse, when it nests deeper
//! than [`purrdf_cdt::MAX_NESTING_DEPTH`], when it holds more than
//! [`purrdf_cdt::MAX_ELEMENTS`] elements, or when it is longer than
//! [`purrdf_cdt::MAX_LEXICAL_BYTES`]. Ingress turns that into a parse
//! diagnostic and the **whole document is refused**, even though it is
//! otherwise syntactically valid.
//!
//! That is deliberate and it is not a style choice. A composite literal that
//! does not parse has an undefined set of embedded blank nodes, so admitting it
//! would leave the document's blank-node scope — and therefore the identity of
//! blank nodes OUTSIDE the literal, which may share its labels — undefined too.
//! There is no "opaque literal" fallback: a lexical form typed `cdt:List` or
//! `cdt:Map` is a composite value, full stop, and a store that accepted a
//! broken one would be quietly wrong rather than loudly empty.
//!
//! # Traversal is iterative
//!
//! Every walk here uses an explicit stack. Rust aborts on stack overflow and the
//! abort is not catchable, so a 64-deep hostile literal must never become 64
//! frames of recursion.

use std::borrow::Cow;

use purrdf_cdt::{
    CDT_LIST, CDT_MAP, CdtContents, CdtError, CdtTerm, CdtValue, MAX_NESTING_DEPTH,
    parse_cdt_by_iri,
};

use crate::blank_label::{LabelAlphabet, decode_blank_label, encode_blank_label};
use crate::ir::term::BlankScope;

/// Whether `datatype` is one of the two SEP-0009 composite datatype IRIs.
///
/// The dispatch guard for every hot path in this module: a literal whose
/// datatype is not `cdt:List` or `cdt:Map` is never scanned, so the cost on an
/// ordinary literal is two string comparisons and nothing else.
#[must_use]
#[inline]
pub fn is_cdt_datatype(datatype: &str) -> bool {
    datatype == CDT_LIST || datatype == CDT_MAP
}

/// How an ingress path binds the blank-node labels of the document it is
/// reading.
///
/// The SAME value governs a bare `_:b` term and a label embedded in a composite
/// literal, which is what makes the two agree. A path that decodes its blank
/// tokens against a syntax alphabet uses [`Decoded`](Self::Decoded); a carrier
/// that assigns every label of a source one fixed scope uses
/// [`Ambient`](Self::Ambient).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlankBinding {
    /// The document's tokens carry their own scope in the egress envelope this
    /// workspace writes; decode them against the alphabet of the syntax they
    /// were read from, exactly as
    /// [`intern_text_blank`](crate::ir::builder::RdfDatasetBuilder::intern_text_blank)
    /// does for a bare term.
    Decoded(LabelAlphabet),
    /// Every label in this source belongs to one fixed scope, verbatim: the
    /// carrier stored the IR's `(label, scope)` pair structurally and the label
    /// was never encoded. Used by the GTS readers (one scope per segment) and by
    /// the multi-source owned-quad loader (one scope per source), matching their
    /// [`intern_blank`](crate::ir::builder::RdfDatasetBuilder::intern_blank)
    /// calls for bare terms.
    Ambient(BlankScope),
}

impl BlankBinding {
    /// The `(label, scope)` pair `token` denotes under this binding.
    #[must_use]
    pub fn resolve(self, token: &str) -> (Cow<'_, str>, BlankScope) {
        match self {
            Self::Decoded(alphabet) => decode_blank_label(token, alphabet),
            Self::Ambient(scope) => (Cow::Borrowed(token), scope),
        }
    }

    /// The `BLANK_NODE_LABEL` spelling that names the same node as `token` once
    /// bound — the form written back into a composite literal's lexical bytes.
    ///
    /// Borrowed and byte-identical whenever the binding is already a fixpoint on
    /// `token`, which is the case for every plain label read from a text syntax
    /// at [`BlankScope::DEFAULT`].
    #[must_use]
    pub fn rebind(self, token: &str) -> Cow<'_, str> {
        let (label, scope) = self.resolve(token);
        match encode_blank_label(&label, scope, LabelAlphabet::BlankNodeLabel) {
            // Re-borrow from `token` rather than from the temporary `label` when
            // the whole round trip was the identity, so the caller keeps its
            // no-allocation fast path.
            Cow::Borrowed(encoded) if encoded == token => Cow::Borrowed(token),
            Cow::Borrowed(encoded) => Cow::Owned(encoded.to_owned()),
            Cow::Owned(encoded) => Cow::Owned(encoded),
        }
    }
}

/// A refusal from this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CdtBlankError {
    /// The lexical form is not a well-formed value of its composite datatype, or
    /// it exceeds one of the `purrdf-cdt` resource limits. The document that
    /// carried it is refused.
    Malformed {
        /// The composite datatype IRI the literal claimed.
        datatype: &'static str,
        /// The grammar's own refusal.
        source: CdtError,
    },
    /// The lexical scanner and the SEP-0009 grammar disagree about which
    /// `BLANK_NODE_LABEL` tokens a well-formed lexical form holds.
    ///
    /// Unreachable by construction — the two implement the same production over
    /// the same bytes — and reported rather than papered over precisely because
    /// silently binding the wrong set of labels would corrupt blank-node
    /// identity instead of failing.
    ScannerDisagreement {
        /// The labels the byte scanner located, in occurrence order.
        scanned: Vec<String>,
        /// The labels the grammar produced, in occurrence order.
        parsed: Vec<String>,
    },
}

impl std::fmt::Display for CdtBlankError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed { datatype, source } => {
                write!(f, "ill-formed <{datatype}> lexical form: {source}")
            }
            Self::ScannerDisagreement { scanned, parsed } => write!(
                f,
                "composite blank-label scan disagrees with the grammar: \
                 scanned {scanned:?}, parsed {parsed:?}"
            ),
        }
    }
}

impl std::error::Error for CdtBlankError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Malformed { source, .. } => Some(source),
            Self::ScannerDisagreement { .. } => None,
        }
    }
}

impl From<CdtBlankError> for crate::RdfDiagnostic {
    /// The parse diagnostic a codec reports when a document carries an
    /// ill-formed composite literal. The whole document is refused: see the
    /// module docs for why there is no opaque-literal fallback.
    fn from(err: CdtBlankError) -> Self {
        let code = match err {
            CdtBlankError::Malformed { .. } => "cdt-literal-malformed",
            CdtBlankError::ScannerDisagreement { .. } => "cdt-literal-scan-disagreement",
        };
        Self::error(code, err.to_string()).with_detail(
            "a cdt:List / cdt:Map lexical form denotes blank nodes in the enclosing document's \
             scope, so one that does not parse leaves that scope -- and the identity of \
             same-labelled blank nodes outside the literal -- undefined",
        )
    }
}

/// Validate `lexical` as a value of `datatype` and bind every
/// `BLANK_NODE_LABEL` it holds into `binding`.
///
/// Returns the lexical form BORROWED and byte-identical when no token needed
/// rewriting, which is the overwhelmingly common case: a plain label read from a
/// text syntax at [`BlankScope::DEFAULT`] already spells its own
/// `(label, scope)` pair.
///
/// This is the ingress entry point. It is a no-op for any datatype that is not
/// composite, so callers may hand it every literal they intern.
///
/// # Errors
/// [`CdtBlankError::Malformed`] when `lexical` is not a well-formed value of
/// `datatype`, including a violation of `purrdf-cdt`'s nesting-depth,
/// element-count or lexical-length limits; the caller must refuse the whole
/// document. [`CdtBlankError::ScannerDisagreement`] never occurs on a
/// well-formed input.
pub fn bind_cdt_blank_labels<'a>(
    lexical: &'a str,
    datatype: &str,
    binding: BlankBinding,
) -> Result<Cow<'a, str>, CdtBlankError> {
    if !is_cdt_datatype(datatype) {
        return Ok(Cow::Borrowed(lexical));
    }
    let value = parse_composite(lexical, datatype)?;
    let spans = scan_tokens(lexical);
    check_scan_agrees(&spans, &value)?;
    Ok(splice(lexical, &spans, &mut |token| match token {
        TokenKind::Blank(label) => match binding.rebind(label) {
            // Nothing to write: the bound spelling IS the token already there.
            Cow::Borrowed(_) => None,
            Cow::Owned(bound) => Some(format!("_:{bound}")),
        },
        TokenKind::Iri { .. } => None,
    }))
}

/// [`bind_cdt_blank_labels`] without the grammar check: bind the labels the byte
/// scanner finds and take the lexical form as given.
///
/// The binding is IDENTICAL — same tokens, same spelling, same result on any
/// well-formed input. Only the refusal is dropped, so this is total.
///
/// It exists for the OWNED-model bridge
/// ([`intern_owned_term_scoped`](crate::ir::builder::RdfDatasetBuilder::intern_owned_term_scoped)),
/// which is infallible by contract and re-materializes datasets that a document
/// ingress already validated. Binding must still happen there — a merge assigns
/// each source a fresh [`BlankScope`], and an embedded label left unbound would
/// name a node the merged dataset does not have — but there is no document to
/// refuse and no diagnostic channel to refuse it through.
///
/// Every path that reads a DOCUMENT uses
/// [`intern_literal_bound`](crate::ir::builder::RdfDatasetBuilder::intern_literal_bound),
/// which validates.
#[must_use]
pub fn bind_cdt_blank_labels_unchecked<'a>(
    lexical: &'a str,
    datatype: &str,
    binding: BlankBinding,
) -> Cow<'a, str> {
    rewrite_cdt_blank_terms(
        lexical,
        datatype,
        &mut |label| match binding.rebind(label) {
            Cow::Borrowed(_) => None,
            Cow::Owned(bound) => Some(format!("_:{bound}")),
        },
    )
}

/// Rewrite the blank-node AND IRI tokens of an ALREADY-BOUND composite lexical
/// form, so a whole-dataset term rewrite reaches inside composite literals.
///
/// `on_blank` receives a token's label (without the `_:`); `on_iri` receives an
/// IRI's unescaped value and whether it sits in an IRI-ONLY position (the
/// datatype of an embedded literal, which can never hold a blank node). Either
/// returns the full replacement element text — `_:other`, or `<iri>` — or
/// `None` to leave the token alone.
///
/// Both halves matter and they matter together: a skolemization that rewrote
/// only the blank tokens would not be invertible, because the de-skolemizing
/// pass would then have no way to turn the genid IRIs it wrote back into blank
/// nodes.
///
/// Returns the lexical form borrowed when nothing changed, and is a no-op for a
/// non-composite datatype.
#[must_use]
pub fn rewrite_cdt_terms<'a>(
    lexical: &'a str,
    datatype: &str,
    on_blank: &mut dyn FnMut(&str) -> Option<String>,
    on_iri: &mut dyn FnMut(&str, bool) -> Option<String>,
) -> Cow<'a, str> {
    if !is_cdt_datatype(datatype) {
        return Cow::Borrowed(lexical);
    }
    let spans = scan_tokens(lexical);
    splice(lexical, &spans, &mut |token| match token {
        TokenKind::Blank(label) => on_blank(label),
        TokenKind::Iri { iri, iri_only } => on_iri(iri, *iri_only),
    })
}

/// Rewrite each `BLANK_NODE_LABEL` of an ALREADY-BOUND composite lexical form
/// through `rewrite`, which receives the token's label (without the `_:`) and
/// returns the full replacement element text — `_:other` to rename it, or
/// `<iri>` to skolemize it away.
///
/// Used by the whole-dataset rewriters (RDFC-1.0 relabeling, skolemization and
/// its inverse) so a blank node embedded in a composite literal is rewritten in
/// lockstep with the same node written as a term. A rewriter that skipped this
/// would leave the embedded occurrence DANGLING: pointing at a label the
/// rewritten dataset no longer has.
///
/// Returns the lexical form borrowed when `rewrite` asked for no change, and is
/// a no-op for a non-composite datatype.
#[must_use]
pub fn rewrite_cdt_blank_terms<'a>(
    lexical: &'a str,
    datatype: &str,
    rewrite: &mut dyn FnMut(&str) -> Option<String>,
) -> Cow<'a, str> {
    rewrite_cdt_terms(lexical, datatype, rewrite, &mut |_, _| None)
}

/// Every `(label, scope)` pair an ALREADY-BOUND composite lexical form
/// references, in occurrence order and with duplicates kept.
///
/// This is how a consumer — the SPARQL evaluator's `cdt:get`, the
/// canonicalizer's blank-node discovery, the store's ingress registration —
/// turns a composite literal's bytes back into the blank-node identities they
/// name. Each pair is exactly what
/// [`intern_blank`](crate::ir::builder::RdfDatasetBuilder::intern_blank) was
/// given for the corresponding bare term, so
/// [`term_id_by_blank`](crate::ir::dataset::RdfDataset::term_id_by_blank)
/// resolves it to the very [`TermId`](crate::TermId) the dataset already holds.
///
/// Empty for a non-composite datatype.
#[must_use]
pub fn cdt_embedded_blanks(lexical: &str, datatype: &str) -> Vec<(String, BlankScope)> {
    if !is_cdt_datatype(datatype) {
        return Vec::new();
    }
    scan_tokens(lexical)
        .into_iter()
        .filter_map(|span| match span.kind {
            TokenKind::Blank(token) => {
                let (label, scope) = decode_blank_label(&token, LabelAlphabet::BlankNodeLabel);
                Some((label.into_owned(), scope))
            }
            TokenKind::Iri { .. } => None,
        })
        .collect()
}

/// Parse `lexical` as a value of the composite `datatype`, mapping every
/// refusal — grammar and resource limit alike — onto [`CdtBlankError`].
fn parse_composite(lexical: &str, datatype: &str) -> Result<CdtValue, CdtBlankError> {
    let iri = if datatype == CDT_LIST {
        CDT_LIST
    } else {
        CDT_MAP
    };
    match parse_cdt_by_iri(lexical, datatype) {
        Ok(Some(value)) => Ok(value),
        Ok(None) => unreachable!("is_cdt_datatype admitted a datatype parse_cdt_by_iri does not"),
        Err(source) => Err(CdtBlankError::Malformed {
            datatype: iri,
            source,
        }),
    }
}

/// A rewritable term token located inside a composite lexical form.
#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
    /// A `BLANK_NODE_LABEL`, carrying the label without its `_:` prefix.
    Blank(String),
    /// An `IRIREF`, carrying its UNESCAPED value.
    Iri {
        /// The IRI the token denotes, with `UCHAR` escapes resolved.
        iri: String,
        /// Whether the token sits where a blank node can never go: the datatype
        /// of an embedded literal. A rewriter that mints blanks from IRIs must
        /// refuse there rather than produce an invalid literal — the same
        /// contract [`TermMapper::map_iri`](crate::ir::skolem) states for a
        /// predicate slot.
        iri_only: bool,
    },
}

/// One token's byte span in ROOT-lexical-form coordinates, plus its value.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TokenSpan {
    /// Offset of the token's first byte (the `_` of `_:`, or the `<`).
    start: usize,
    /// One past the token's last byte (the label's end, or past the `>`).
    end: usize,
    /// What the token is.
    kind: TokenKind,
}

/// Replace the located tokens for which `rewrite` returns a replacement,
/// splicing into a fresh string and leaving every other byte alone.
fn splice<'a>(
    lexical: &'a str,
    spans: &[TokenSpan],
    rewrite: &mut dyn FnMut(&TokenKind) -> Option<String>,
) -> Cow<'a, str> {
    let mut out: Option<String> = None;
    let mut cursor = 0usize;
    for span in spans {
        // A token the previous splice already consumed (only reachable if the
        // scan produced overlapping spans, which it does not) is skipped rather
        // than allowed to slice mid-character.
        if span.start < cursor {
            continue;
        }
        let Some(replacement) = rewrite(&span.kind) else {
            continue;
        };
        let buf = out.get_or_insert_with(|| String::with_capacity(lexical.len() + 16));
        buf.push_str(&lexical[cursor..span.start]);
        buf.push_str(&replacement);
        cursor = span.end;
    }
    match out {
        Some(mut buf) => {
            buf.push_str(&lexical[cursor..]);
            Cow::Owned(buf)
        }
        None => Cow::Borrowed(lexical),
    }
}

/// Cross-check the byte scanner against the SEP-0009 grammar: both must report
/// the same blank-node labels.
fn check_scan_agrees(spans: &[TokenSpan], value: &CdtValue) -> Result<(), CdtBlankError> {
    let parsed = grammar_blank_labels(value)?;
    let scanned: Vec<String> = spans
        .iter()
        .filter_map(|span| match &span.kind {
            TokenKind::Blank(label) => Some(label.clone()),
            TokenKind::Iri { .. } => None,
        })
        .collect();
    // Occurrence ORDER differs legitimately: the grammar walk visits a map's
    // entries in the parsed value's key order while the scanner walks raw bytes.
    // Identity only depends on the SET of labels, so compare as multisets.
    let mut a = scanned.clone();
    let mut b = parsed.clone();
    a.sort_unstable();
    b.sort_unstable();
    if a == b {
        return Ok(());
    }
    Err(CdtBlankError::ScannerDisagreement { scanned, parsed })
}

/// Every blank label the parsed value holds, found with an EXPLICIT STACK.
///
/// Descends into nested composites, triple-term components and composite-typed
/// embedded literals, because SEP-0009 puts all three in the document's one
/// blank-node scope (`bnodes-turtle-21` and `bnodes-turtle-41` pin the last two
/// respectively).
///
/// An embedded composite-typed literal is a composite value that the OUTER parse
/// only saw as a string, so its own well-formedness is checked HERE. A malformed
/// one refuses the document exactly as a malformed outer form does — its blank
/// nodes are in the same scope, so admitting it would leave that scope undefined
/// just the same.
fn grammar_blank_labels(root: &CdtValue) -> Result<Vec<String>, CdtBlankError> {
    let mut out = Vec::new();
    // Composite-typed literals found embedded as strings, still to be parsed.
    // Kept as a separate worklist rather than pushed onto the borrow stack so no
    // frame ever borrows from a value another frame owns.
    let mut embedded: Vec<(String, String)> = Vec::new();
    walk_value(root, &mut out, &mut embedded);
    while let Some((lexical, datatype)) = embedded.pop() {
        let value = parse_composite(&lexical, &datatype)?;
        walk_value(&value, &mut out, &mut embedded);
    }
    Ok(out)
}

/// Collect `value`'s own blank labels and queue any composite-typed literal it
/// embeds, iteratively.
fn walk_value(value: &CdtValue, out: &mut Vec<String>, embedded: &mut Vec<(String, String)>) {
    let mut stack: Vec<&CdtTerm> = Vec::new();
    push_children(&mut stack, value);
    // The parse already enforced MAX_NESTING_DEPTH and MAX_ELEMENTS, so the walk
    // is bounded; the counter also bounds a value assembled PROGRAMMATICALLY,
    // which carries no such guarantee.
    let mut budget = MAX_NESTING_DEPTH.saturating_mul(purrdf_cdt::MAX_ELEMENTS);
    while let Some(term) = stack.pop() {
        if budget == 0 {
            return;
        }
        budget -= 1;
        match term {
            CdtTerm::Blank(label) => out.push(label.clone()),
            CdtTerm::Composite(inner) => push_children(&mut stack, inner),
            CdtTerm::TripleTerm(triple) => {
                stack.push(&triple.subject);
                stack.push(&triple.predicate);
                stack.push(&triple.object);
            }
            CdtTerm::Literal(lit) if is_cdt_datatype(&lit.datatype) => {
                embedded.push((lit.lexical.clone(), lit.datatype.clone()));
            }
            CdtTerm::Literal(_) | CdtTerm::Iri(_) | CdtTerm::Null => {}
        }
    }
}

/// Push a composite's element terms onto the walk stack. A `cdt:Map` key is
/// never a blank node (`CdtKey` makes that unrepresentable), so only values are
/// visited.
fn push_children<'a>(stack: &mut Vec<&'a CdtTerm>, value: &'a CdtValue) {
    match value.contents() {
        CdtContents::List(items) => stack.extend(items.iter()),
        CdtContents::Map(entries) => stack.extend(entries.iter().map(|entry| &entry.value)),
    }
}

// ── The byte scanner ────────────────────────────────────────────────────────

/// A region of text still to be scanned for blank-node labels: either the root
/// lexical form itself, or the UNESCAPED content of a composite-typed literal
/// embedded inside it.
struct Region {
    /// The bytes to scan.
    text: String,
    /// Root offset of each byte of `text`, with a sentinel entry at `text.len()`.
    /// Empty for the root, where the mapping is the identity.
    map: Vec<usize>,
}

impl Region {
    /// The root-lexical-form offset of byte `i` of this region.
    fn root_of(&self, i: usize) -> usize {
        if self.map.is_empty() { i } else { self.map[i] }
    }
}

/// Locate every rewritable term token (`BLANK_NODE_LABEL` and `IRIREF`) in a
/// composite lexical form, at any nesting depth, returning byte spans in ROOT
/// coordinates.
///
/// Total: it never fails and never panics on any input, so the infallible
/// interning path can use it directly. Well-formedness is the parser's job, and
/// [`check_scan_agrees`] cross-checks the two on the validating path.
///
/// Spans are returned in ascending root order and never overlap, which is what
/// makes [`splice`] a single forward pass.
///
/// # Byte spans, not re-rendering
///
/// A `BLANK_NODE_LABEL` is drawn from `PN_CHARS`, which contains neither a quote
/// nor a backslash, so a label is spelled identically at every nesting depth and
/// never carries an escape. That is why an embedded literal's labels can be
/// rewritten by splicing directly into the ROOT bytes: nothing is ever
/// unescaped-then-re-escaped, so a nested literal's escape spellings survive a
/// rewrite of its labels byte for byte.
fn scan_tokens(lexical: &str) -> Vec<TokenSpan> {
    let mut spans = Vec::new();
    let mut queue = vec![Region {
        text: lexical.to_owned(),
        map: Vec::new(),
    }];
    // One region per embedded composite literal; the outer parse's element
    // budget bounds how many there can be.
    let mut budget = purrdf_cdt::MAX_ELEMENTS;
    while let Some(region) = queue.pop() {
        if budget == 0 {
            break;
        }
        budget -= 1;
        scan_region(&region, &mut spans, &mut queue);
    }
    spans.sort_unstable_by_key(|span| span.start);
    // A token located inside an embedded literal is nested INSIDE that
    // literal's own IRIREF-bearing span only in the datatype case, which is
    // scanned in the parent region; the label regions never overlap. Drop any
    // span that would still overlap its predecessor rather than trust that.
    spans.dedup_by(|later, earlier| later.start < earlier.end);
    spans
}

/// Scan one region, appending token spans and queueing the unescaped content of
/// every composite-typed literal it embeds.
fn scan_region(region: &Region, spans: &mut Vec<TokenSpan>, queue: &mut Vec<Region>) {
    let bytes = region.text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            // A triple term opens with `<<(`, whose leading `<` is not an IRI.
            b'<' if region.text[i..].starts_with("<<(") => i += 3,
            b'<' => {
                let (end, close) = scan_iriref(bytes, i);
                if let Some(close) = close {
                    // An element-position IRI: `iri_only` is false, because the
                    // grammar admits a blank node in this very position.
                    push_iri_span(region, i, close, end, false, spans);
                }
                i = end;
            }
            b'"' | b'\'' => i = scan_string(region, i, spans, queue),
            b'_' if bytes.get(i + 1) == Some(&b':') => {
                let label_start = i + 2;
                let end = scan_label_body(&region.text, label_start);
                if end > label_start {
                    spans.push(TokenSpan {
                        start: region.root_of(i),
                        end: region.root_of(end),
                        kind: TokenKind::Blank(region.text[label_start..end].to_owned()),
                    });
                }
                i = end.max(label_start);
            }
            _ => {
                // Advance a whole scalar so a multi-byte character is never split.
                i += region.text[i..].chars().next().map_or(1, char::len_utf8);
            }
        }
    }
}

/// Scan an `IRIREF` opening at `start`, returning `(end, close)` — the offset
/// just past its `>`, and the offset OF that `>`.
///
/// `close` is `None` for an unterminated `IRIREF`, whose body would not be a
/// well-defined slice; the validating path refuses such a form outright and the
/// total path simply reports no token for it.
///
/// `UCHAR` escapes ride through: no escape spells a raw `>`.
fn scan_iriref(bytes: &[u8], start: usize) -> (usize, Option<usize>) {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'>' => return (i + 1, Some(i)),
            b'\\' => i += 2,
            _ => i += 1,
        }
    }
    (bytes.len(), None)
}

/// The end offset of the `BLANK_NODE_LABEL` body starting at `start`.
///
/// `BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')* PN_CHARS)?`
/// — the trailing `.` the production forbids is trimmed here, so `_:b.` scans as
/// the label `b` exactly as the grammar reads it.
fn scan_label_body(text: &str, start: usize) -> usize {
    let mut end = start;
    for ch in text[start..].chars() {
        if crate::blank_label::is_pn_chars(ch) || ch == '.' {
            end += ch.len_utf8();
        } else {
            break;
        }
    }
    // Trim trailing dots: they are legal mid-label but never final.
    while end > start && text.as_bytes()[end - 1] == b'.' {
        end -= 1;
    }
    // The first character must be PN_CHARS_U or a digit for this to be a label.
    match text[start..end].chars().next() {
        Some(first) if crate::blank_label::is_pn_chars_u(first) || first.is_ascii_digit() => end,
        _ => start,
    }
}

/// Record one `IRIREF` token: `open` is the `<`, `close` the `>`, `end` one past
/// the `>`.
fn push_iri_span(
    region: &Region,
    open: usize,
    close: usize,
    end: usize,
    iri_only: bool,
    spans: &mut Vec<TokenSpan>,
) {
    spans.push(TokenSpan {
        start: region.root_of(open),
        end: region.root_of(end),
        kind: TokenKind::Iri {
            iri: unescape_iri(&region.text[open + 1..close]).into_owned(),
            iri_only,
        },
    });
}

/// Scan a quoted string opening at `start`, queueing its unescaped content when
/// a `^^` datatype marks it as an embedded composite literal. Returns the offset
/// just past the literal (including any datatype or language tag).
fn scan_string(
    region: &Region,
    start: usize,
    spans: &mut Vec<TokenSpan>,
    queue: &mut Vec<Region>,
) -> usize {
    let bytes = region.text.as_bytes();
    let quote = bytes[start];
    let long = region.text[start..].starts_with(if quote == b'"' { "\"\"\"" } else { "'''" });
    let open = if long { 3 } else { 1 };
    let content_start = start + open;

    let mut i = content_start;
    let mut content_end = bytes.len();
    let mut after = bytes.len();
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == quote {
            if long {
                if region.text[i..].starts_with(if quote == b'"' { "\"\"\"" } else { "'''" }) {
                    content_end = i;
                    after = i + 3;
                    break;
                }
                i += 1;
                continue;
            }
            content_end = i;
            after = i + 1;
            break;
        }
        i += 1;
    }
    if after >= bytes.len() {
        return bytes.len();
    }

    // `String (LANGTAG | '^^' IRIREF)?` — only the typed form can be composite.
    if !region.text[after..].starts_with("^^") {
        return after;
    }
    let iri_start = after + 2;
    if bytes.get(iri_start) != Some(&b'<') {
        return iri_start;
    }
    let (iri_end, close) = scan_iriref(bytes, iri_start);
    let Some(close) = close else {
        return iri_end;
    };
    let datatype = unescape_iri(&region.text[iri_start + 1..close]);
    if is_cdt_datatype(&datatype) {
        queue.push(unescaped_region(region, content_start, content_end));
    }
    // A literal's datatype can never be a blank node, so a rewriter that mints
    // blanks from IRIs must refuse here rather than write an invalid literal.
    push_iri_span(region, iri_start, close, iri_end, true, spans);
    iri_end
}

/// Unescape an `IRIREF` body's `UCHAR` escapes so a datatype written with them
/// still compares equal to the composite IRIs.
fn unescape_iri(raw: &str) -> Cow<'_, str> {
    if !raw.contains('\\') {
        return Cow::Borrowed(raw);
    }
    let mut out = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            let width = match bytes.get(i + 1) {
                Some(b'u') => 4,
                Some(b'U') => 8,
                _ => {
                    i += 2;
                    continue;
                }
            };
            let hex = raw.get(i + 2..i + 2 + width).unwrap_or("");
            if let Some(ch) = u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) {
                out.push(ch);
            }
            i += 2 + width;
        } else {
            let ch = raw[i..].chars().next().unwrap_or('\u{fffd}');
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    Cow::Owned(out)
}

/// Build the scan region for an embedded composite literal: its content
/// unescaped, plus the root offset of every resulting byte.
fn unescaped_region(region: &Region, content_start: usize, content_end: usize) -> Region {
    let raw = &region.text[content_start..content_end];
    let mut text = String::with_capacity(raw.len());
    let mut map = Vec::with_capacity(raw.len() + 1);
    let bytes = raw.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let root = region.root_of(content_start + i);
        if bytes[i] == b'\\' {
            let (ch, width) = decode_escape(raw, i);
            let mut buf = [0u8; 4];
            for _ in ch.encode_utf8(&mut buf).as_bytes() {
                // Every byte an escape produces points at the escape's opening
                // backslash: a label never rides in an escape, so this offset is
                // only ever consulted for bytes that are not part of a label.
                map.push(root);
            }
            text.push(ch);
            i += width;
        } else {
            let ch = raw[i..].chars().next().unwrap_or('\u{fffd}');
            for k in 0..ch.len_utf8() {
                map.push(root + k);
            }
            text.push(ch);
            i += ch.len_utf8();
        }
    }
    map.push(region.root_of(content_end));
    Region { text, map }
}

/// Decode one escape sequence at `at`, returning the scalar and the raw width
/// consumed. An unrecognized sequence yields its own backslash so the scan stays
/// total; the grammar has already refused such a form on the validating path.
fn decode_escape(raw: &str, at: usize) -> (char, usize) {
    let bytes = raw.as_bytes();
    match bytes.get(at + 1) {
        Some(b't') => ('\t', 2),
        Some(b'b') => ('\u{8}', 2),
        Some(b'n') => ('\n', 2),
        Some(b'r') => ('\r', 2),
        Some(b'f') => ('\u{c}', 2),
        Some(b'"') => ('"', 2),
        Some(b'\'') => ('\'', 2),
        Some(b'\\') => ('\\', 2),
        Some(marker @ (b'u' | b'U')) => {
            let width = if *marker == b'u' { 4 } else { 8 };
            let hex = raw.get(at + 2..at + 2 + width).unwrap_or("");
            u32::from_str_radix(hex, 16)
                .ok()
                .and_then(char::from_u32)
                .map_or(('\\', 1), |ch| (ch, 2 + width))
        }
        _ => ('\\', 1),
    }
}

#[cfg(test)]
mod tests;
