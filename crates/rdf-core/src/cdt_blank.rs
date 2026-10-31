// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! [`encode_blank_label`](crate::blank_label::encode_blank_label) produces, and
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
//! [`CdtBlankError::Malformed`] when it does not parse, when it holds more than
//! [`purrdf_cdt::MAX_ELEMENTS`] elements, or when it is longer than
//! [`purrdf_cdt::MAX_LEXICAL_BYTES`]. Nesting depth is not a bound of its own:
//! a level is one element of the level above it, so the element bound caps it,
//! and a literal a hundred thousand brackets deep is a value whose labels are
//! bound like any other's. Ingress turns a refusal into a parse diagnostic and
//! the **whole document is refused**, even though it is otherwise syntactically
//! valid.
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
//! abort is not catchable, so a hostile literal nested as deep as the element
//! bound allows must never become that many frames of recursion — here, or in the
//! `purrdf-cdt` value it parses into, whose every walk (its `Drop` included) is
//! iterative too.

use std::borrow::Cow;

use purrdf_iri::terminals;
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};
use purrdf_lex::terminals::expand_uchars_with_memory;

use purrdf_cdt::{CDT_LIST, CDT_MAP, CdtContents, CdtDatatype, CdtError, CdtTerm, CdtValue};

use crate::blank_label::{
    LabelAlphabet, decode_blank_label, decode_blank_label_with_memory,
    encode_blank_label_with_memory,
};
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
        self.rebind_with_memory(token, &mut Memory::new(&mut Resident))
            .expect("resident blank-label binding allocation failed")
    }

    /// Bind the same label under the caller's original physical admission.
    ///
    /// # Errors
    /// Returns physical refusal without changing label identity or spelling.
    pub fn rebind_with_memory<'a, S: Admission + ?Sized>(
        self,
        token: &'a str,
        memory: &mut Memory<'_, S>,
    ) -> Result<Cow<'a, str>, StorageError> {
        let (label, scope) = match self {
            Self::Decoded(alphabet) => decode_blank_label_with_memory(token, alphabet, memory)?,
            Self::Ambient(scope) => (Cow::Borrowed(token), scope),
        };
        let encoded =
            encode_blank_label_with_memory(&label, scope, LabelAlphabet::BlankNodeLabel, memory)?;
        let bound = match encoded {
            Cow::Borrowed(encoded) => {
                let unchanged = if memory.admission().progress_enabled() {
                    purrdf_lex::control::compare_bytes(
                        encoded.as_bytes(),
                        token.as_bytes(),
                        &mut |units| memory.admission_mut().checkpoint(units),
                    )?
                    .is_eq()
                } else {
                    encoded == token
                };
                if unchanged {
                    Cow::Borrowed(token)
                } else {
                    Cow::Owned(memory.string_with_progress(encoded)?)
                }
            }
            Cow::Owned(encoded) => Cow::Owned(encoded),
        };
        if let Cow::Owned(label) = label {
            memory.release_string(label)?;
        }
        Ok(bound)
    }
}

/// A refusal from this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CdtBlankError {
    /// Logical capacity or physical storage refusal at the original term producer.
    Storage(StorageError),
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
            Self::Storage(error) => error.fmt(f),
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
            Self::Storage(error) => Some(error),
            Self::Malformed { source, .. } => Some(source),
            Self::ScannerDisagreement { .. } => None,
        }
    }
}

purrdf_lex::variant_from!(CdtBlankError { Storage(StorageError) });

impl CdtBlankError {
    /// Retained diagnostic payload, excluding parser and scanner scratch.
    /// # Errors
    /// Returns physical layout overflow rather than undercounting the owner.
    pub fn owned_bytes(&self) -> Result<usize, StorageError> {
        match self {
            Self::Storage(_) => Ok(0),
            Self::Malformed { source, .. } => Ok(source.owned_bytes()),
            Self::ScannerDisagreement { scanned, parsed } => [scanned, parsed]
                .into_iter()
                .try_fold(0usize, |bytes, labels| {
                    let bytes = bytes
                        .checked_add(
                            labels
                                .capacity()
                                .checked_mul(size_of::<String>())
                                .ok_or(StorageError::SizeOverflow)?,
                        )
                        .ok_or(StorageError::SizeOverflow)?;
                    labels.iter().try_fold(bytes, |bytes, label| {
                        bytes
                            .checked_add(label.capacity())
                            .ok_or(StorageError::SizeOverflow)
                    })
                }),
        }
    }
}

impl CdtBlankError {
    /// Render the native CDT refusal through its original diagnostic account.
    /// The consumed cause is destroyed before its admitted payload is released.
    /// # Errors
    /// Returns physical diagnostic construction refusal.
    pub fn into_diagnostic_with_memory<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<crate::RdfDiagnostic, StorageError> {
        let bytes = self.owned_bytes()?;
        let baseline = memory
            .admitted_bytes()
            .checked_sub(bytes)
            .ok_or(StorageError::SizeOverflow)?;
        let result = (|| {
            let code = match &self {
                Self::Storage(error) => return Err(*error),
                Self::Malformed { .. } => "cdt-literal-malformed",
                Self::ScannerDisagreement { .. } => "cdt-literal-scan-disagreement",
            };
            let mut diagnostic = crate::RdfDiagnostic::try_error_with_memory(code, &self, memory)?;
            diagnostic.detail = Some(memory.string_with_progress(
                "a cdt:List / cdt:Map lexical form denotes blank nodes in the enclosing document's \
                 scope, so one that does not parse leaves that scope -- and the identity of \
                 same-labelled blank nodes outside the literal -- undefined",
            )?);
            Ok(diagnostic)
        })();
        drop(self);
        let retained = result
            .as_ref()
            .ok()
            .map_or(Some(0), crate::RdfDiagnostic::retained_bytes)
            .ok_or(StorageError::SizeOverflow)?;
        let Some(released) = baseline
            .checked_add(retained)
            .and_then(|live| memory.admitted_bytes().checked_sub(live))
        else {
            // A native diagnostic cannot escape an invalid ownership census.
            drop(result);
            if let Some(bytes) = memory.admitted_bytes().checked_sub(baseline) {
                let _ = memory.release_bytes(bytes);
            }
            return Err(StorageError::SizeOverflow);
        };
        let cleanup = memory.release_bytes(released);
        if result.is_ok() {
            cleanup?;
        }
        result
    }
}

impl From<CdtBlankError> for crate::RdfDiagnostic {
    fn from(err: CdtBlankError) -> Self {
        let bytes = match err.owned_bytes() {
            Ok(bytes) => bytes,
            Err(error) => return error.into(),
        };
        err.into_diagnostic_with_memory(&mut Memory::resume(&mut Resident, bytes))
            .unwrap_or_else(Into::into)
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
/// `datatype`, including a violation of `purrdf-cdt`'s element-count or
/// lexical-length limits; the caller must refuse the whole document.
/// [`CdtBlankError::ScannerDisagreement`] never occurs on a well-formed input.
pub fn bind_cdt_blank_labels<'a>(
    lexical: &'a str,
    datatype: &str,
    binding: BlankBinding,
) -> Result<Cow<'a, str>, CdtBlankError> {
    bind_cdt_blank_labels_with_memory(lexical, datatype, binding, &mut Memory::new(&mut Resident))
}

/// Validate and bind a document literal through the original physical admission.
/// Grammar, embedded composite traversal and byte-scanner agreement share that
/// account. A returned lexical form or diagnostic retains its admitted payload.
/// # Errors
/// Preserves malformed CDT diagnostics separately from physical storage refusal.
pub fn bind_cdt_blank_labels_with_memory<'a, S: Admission + ?Sized>(
    lexical: &'a str,
    datatype: &str,
    binding: BlankBinding,
    memory: &mut Memory<'_, S>,
) -> Result<Cow<'a, str>, CdtBlankError> {
    memory.admission_mut().checkpoint(0)?;
    if !is_cdt_datatype(datatype) {
        return Ok(Cow::Borrowed(lexical));
    }
    let baseline = memory.admitted_bytes();
    let result: Result<_, CdtBlankError> = (|| {
        let (value, bytes) = parse_composite(lexical, datatype, memory)?;
        let spans = scan_tokens_with_memory(lexical, memory)?;
        check_scan_agrees(&spans, &value, memory)?;
        drop(value);
        memory.release_bytes(bytes)?;
        let output = bind_spans(lexical, &spans, &mut |_| Ok(binding), memory)?;
        release_spans(spans, memory)?;
        Ok(output)
    })();
    // Every local is now destroyed. Only the returned lexical/diagnostic payload
    // survives; a refused cleanup must never replace the original first error.
    let retained = match &result {
        Ok(Cow::Owned(text)) => Ok(text.capacity()),
        Ok(Cow::Borrowed(_)) => Ok(0),
        Err(error) => error.owned_bytes(),
    };
    let cleanup = retained.and_then(|retained| {
        let live = baseline
            .checked_add(retained)
            .ok_or(StorageError::SizeOverflow)?;
        memory.release_bytes(
            memory
                .admitted_bytes()
                .checked_sub(live)
                .ok_or(StorageError::SizeOverflow)?,
        )
    });
    if result.is_ok() {
        cleanup?;
    }
    result
}

/// [`bind_cdt_blank_labels`] without the grammar check: bind the labels the byte
/// scanner finds and take the lexical form as given.
///
/// The binding is IDENTICAL — same tokens, same spelling, same result on any
/// well-formed input. Only the refusal is dropped, so this is total.
///
/// It exists for the OWNED-model bridge
/// ([`intern_owned_term_scoped`](crate::ir::builder::RdfDatasetBuilder::intern_owned_term_scoped)),
/// which re-materializes datasets that a document ingress already validated and
/// reports storage refusal separately. Binding must still happen there — a merge assigns
/// each source a fresh [`BlankScope`], and an embedded label left unbound would
/// name a node the merged dataset does not have — but there is no document to
/// refuse and no new lexical validation verdict to produce.
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

/// Bind the existing unchecked scanner's labels under original physical admission.
///
/// Recognition, root spans, nesting and byte-preserving splicing are identical to
/// the resident path. An owned result remains admitted in `memory`; the caller
/// must retain that original grant until the result is destroyed or transferred.
///
/// # Errors
/// Returns physical refusal; it is never converted to a malformed-text no-op.
pub fn bind_cdt_blank_labels_unchecked_with_memory<'a, S: Admission + ?Sized>(
    lexical: &'a str,
    datatype: &str,
    binding: BlankBinding,
    memory: &mut Memory<'_, S>,
) -> Result<Cow<'a, str>, StorageError> {
    bind_cdt_blank_labels_lazy_with_memory(lexical, datatype, &mut |_| Ok(binding), memory)
}

/// Obtain an independent document binding only when the original scanner visits
/// an actual blank token, including blanks in embedded composite literals.
pub(crate) fn bind_cdt_blank_labels_lazy_with_memory<'a, S: Admission + ?Sized>(
    lexical: &'a str,
    datatype: &str,
    binding: &mut impl FnMut(&mut Memory<'_, S>) -> Result<BlankBinding, StorageError>,
    memory: &mut Memory<'_, S>,
) -> Result<Cow<'a, str>, StorageError> {
    memory.admission_mut().checkpoint(0)?;
    if !is_cdt_datatype(datatype) {
        return Ok(Cow::Borrowed(lexical));
    }
    let spans = scan_tokens_with_memory(lexical, memory)?;
    let output = bind_spans(lexical, &spans, binding, memory)?;
    release_spans(spans, memory)?;
    Ok(output)
}

fn bind_spans<'a, S: Admission + ?Sized>(
    lexical: &'a str,
    spans: &[TokenSpan],
    binding: &mut impl FnMut(&mut Memory<'_, S>) -> Result<BlankBinding, StorageError>,
    memory: &mut Memory<'_, S>,
) -> Result<Cow<'a, str>, StorageError> {
    splice_with_memory(
        lexical,
        spans,
        &mut |token, memory| match token {
            TokenKind::Blank(label) => match binding(memory)?.rebind_with_memory(label, memory)? {
                Cow::Borrowed(_) => Ok(None),
                Cow::Owned(bound) => {
                    let replacement = memory.format(&format_args!("_:{bound}"))?;
                    memory.release_string(bound)?;
                    Ok(Some(replacement))
                }
            },
            TokenKind::Iri { .. } => Ok(None),
        },
        memory,
    )
}

fn release_spans<S: Admission + ?Sized>(
    spans: Vec<TokenSpan>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    let bytes = span_payload_bytes(&spans)?;
    memory.release_vec(spans)?;
    memory.release_bytes(bytes)
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
    rewrite_cdt_terms_with_memory(
        lexical,
        datatype,
        &mut |token, memory| {
            let value = on_blank(token);
            if let Some(value) = &value {
                memory.add_bytes(value.capacity())?;
            }
            Ok(value)
        },
        &mut |iri, only, memory| {
            let value = on_iri(iri, only);
            if let Some(value) = &value {
                memory.add_bytes(value.capacity())?;
            }
            Ok(value)
        },
        &mut Memory::new(&mut Resident),
    )
    .expect("resident composite term rewrite allocation failed")
}

/// Rewrite the same embedded blank and IRI token spans under their original
/// native storage owner. Callback output must be allocated through `memory`.
/// # Errors
/// Returns physical layout, admission or allocator refusal before growth.
pub fn rewrite_cdt_terms_with_memory<'a, S: Admission + ?Sized>(
    lexical: &'a str,
    datatype: &str,
    on_blank: &mut impl FnMut(&str, &mut Memory<'_, S>) -> Result<Option<String>, StorageError>,
    on_iri: &mut impl FnMut(&str, bool, &mut Memory<'_, S>) -> Result<Option<String>, StorageError>,
    memory: &mut Memory<'_, S>,
) -> Result<Cow<'a, str>, StorageError> {
    memory.admission_mut().checkpoint(0)?;
    if !is_cdt_datatype(datatype) {
        return Ok(Cow::Borrowed(lexical));
    }
    let spans = scan_tokens_with_memory(lexical, memory)?;
    let result = splice_with_memory(
        lexical,
        &spans,
        &mut |token, memory| match token {
            TokenKind::Blank(label) => on_blank(label, memory),
            TokenKind::Iri { iri, iri_only } => on_iri(iri, *iri_only, memory),
        },
        memory,
    )?;
    release_spans(spans, memory)?;
    Ok(result)
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

/// Rename the same embedded composite blanks through the original native scanner.
/// # Errors
/// Returns physical refusal; every returned owned byte retains caller `memory`.
pub fn rewrite_cdt_blank_terms_with_memory<'a, S: Admission + ?Sized>(
    lexical: &'a str,
    datatype: &str,
    rewrite: &mut impl FnMut(&str, &mut Memory<'_, S>) -> Result<Option<String>, StorageError>,
    memory: &mut Memory<'_, S>,
) -> Result<Cow<'a, str>, StorageError> {
    rewrite_cdt_terms_with_memory(lexical, datatype, rewrite, &mut |_, _, _| Ok(None), memory)
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
    cdt_embedded_blanks_with_memory(lexical, datatype, &mut Memory::new(&mut Resident))
        .expect("resident composite blank discovery allocation failed")
}

/// Discover the same embedded blank identities through the original scanner,
/// preserving their original buffer grant until the returned labels die.
///
/// # Errors
/// Returns physical layout, admission or allocator refusal.
pub fn cdt_embedded_blanks_with_memory<S: Admission + ?Sized>(
    lexical: &str,
    datatype: &str,
    memory: &mut Memory<'_, S>,
) -> Result<Vec<(String, BlankScope)>, StorageError> {
    memory.admission_mut().checkpoint(0)?;
    if !is_cdt_datatype(datatype) {
        return Ok(Vec::new());
    }
    let spans = scan_tokens_with_memory(lexical, memory)?;
    let mut blanks = Vec::new();
    for span in &spans {
        if let TokenKind::Blank(token) = &span.kind {
            let (label, scope) =
                decode_blank_label_with_memory(token, LabelAlphabet::BlankNodeLabel, memory)?;
            let label = match label {
                Cow::Owned(label) => label,
                Cow::Borrowed(label) => memory.string_with_progress(label)?,
            };
            memory.push(&mut blanks, (label, scope))?;
        }
    }
    release_spans(spans, memory)?;
    Ok(blanks)
}

/// Parse `lexical` as a value of the composite `datatype`, mapping every
/// refusal — grammar and resource limit alike — onto [`CdtBlankError`].
fn parse_composite<S: Admission + ?Sized>(
    lexical: &str,
    datatype: &str,
    memory: &mut Memory<'_, S>,
) -> Result<(CdtValue, usize), CdtBlankError> {
    let iri = if datatype == CDT_LIST {
        CDT_LIST
    } else {
        CDT_MAP
    };
    let datatype = if datatype == CDT_LIST {
        CdtDatatype::List
    } else {
        CdtDatatype::Map
    };
    let mut storage = GrammarStorage { memory, live: 0 };
    match purrdf_cdt::parse::try_parse_cdt(lexical, datatype, &mut storage) {
        Ok(value) => Ok(value),
        Err(purrdf_cdt::memory::ReadError::Storage(error)) => Err(error.into()),
        Err(purrdf_cdt::memory::ReadError::Lexical(source)) => Err(CdtBlankError::Malformed {
            datatype: iri,
            source,
        }),
    }
}

/// The grammar's native boxes and buffers are charged before allocation to the
/// surrounding operation, without replacing its already-live parent total.
struct GrammarStorage<'m, 'a, S: Admission + ?Sized> {
    memory: &'m mut Memory<'a, S>,
    live: usize,
}

impl<S: Admission + ?Sized> Admission for GrammarStorage<'_, '_, S> {
    fn progress_enabled(&self) -> bool {
        self.memory.admission().progress_enabled()
    }
    fn checkpoint(&mut self, units: u64) -> Result<(), StorageError> {
        self.memory.admission_mut().checkpoint(units)
    }
    fn resize(&mut self, next: usize) -> Result<(), StorageError> {
        if next >= self.live {
            self.memory.add_bytes(next - self.live)?;
        } else {
            self.memory.release_bytes(self.live - next)?;
        }
        self.live = next;
        Ok(())
    }
}

impl<S: Admission + ?Sized> purrdf_cdt::memory::Storage for GrammarStorage<'_, '_, S> {}

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

fn splice_with_memory<'a, S: Admission + ?Sized>(
    lexical: &'a str,
    spans: &[TokenSpan],
    rewrite: &mut impl FnMut(&TokenKind, &mut Memory<'_, S>) -> Result<Option<String>, StorageError>,
    memory: &mut Memory<'_, S>,
) -> Result<Cow<'a, str>, StorageError> {
    let mut out: Option<String> = None;
    let mut cursor = 0usize;
    for span in spans {
        memory.admission_mut().checkpoint(1)?;
        if span.start < cursor {
            continue;
        }
        let Some(replacement) = rewrite(&span.kind, memory)? else {
            continue;
        };
        let buf = out.get_or_insert_with(String::new);
        memory.push_str_with_progress(buf, &lexical[cursor..span.start])?;
        memory.push_str_with_progress(buf, &replacement)?;
        memory.release_string(replacement)?;
        cursor = span.end;
    }
    match out {
        Some(mut buf) => {
            memory.push_str_with_progress(&mut buf, &lexical[cursor..])?;
            Ok(Cow::Owned(buf))
        }
        None => Ok(Cow::Borrowed(lexical)),
    }
}

/// Cross-check the byte scanner against the SEP-0009 grammar: both must report
/// the same blank-node labels.
fn check_scan_agrees<S: Admission + ?Sized>(
    spans: &[TokenSpan],
    value: &CdtValue,
    memory: &mut Memory<'_, S>,
) -> Result<(), CdtBlankError> {
    let parsed = grammar_blank_labels(value, memory)?;
    let mut scanned = Vec::new();
    for span in spans {
        memory.admission_mut().checkpoint(1)?;
        if let TokenKind::Blank(label) = &span.kind {
            let label = memory.string_with_progress(label)?;
            memory.push(&mut scanned, label)?;
        }
    }
    // Occurrence ORDER differs legitimately: the grammar walk visits a map's
    // entries in the parsed value's key order while the scanner walks raw bytes.
    // Identity only depends on the SET of labels, so compare as multisets.
    let mut a = memory.collect(scanned.iter().map(String::as_str))?;
    let mut b = memory.collect(parsed.iter().map(String::as_str))?;
    for values in [&mut a, &mut b] {
        purrdf_lex::walk::try_sort_unstable_by(values, |left, right| {
            purrdf_lex::control::compare_bytes(left.as_bytes(), right.as_bytes(), &mut |units| {
                memory.admission_mut().checkpoint(units)
            })
        })?;
    }
    let mut agrees = a.len() == b.len();
    if agrees {
        for (left, right) in a.iter().zip(&b) {
            if !purrdf_lex::control::compare_bytes(
                left.as_bytes(),
                right.as_bytes(),
                &mut |units| memory.admission_mut().checkpoint(units),
            )?
            .is_eq()
            {
                agrees = false;
                break;
            }
        }
    }
    memory.release_vec(a)?;
    memory.release_vec(b)?;
    if agrees {
        for mut labels in [scanned, parsed] {
            while let Some(label) = labels.pop() {
                memory.release_string(label)?;
            }
            memory.release_vec(labels)?;
        }
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
fn grammar_blank_labels<S: Admission + ?Sized>(
    root: &CdtValue,
    memory: &mut Memory<'_, S>,
) -> Result<Vec<String>, CdtBlankError> {
    let mut out = Vec::new();
    // Composite-typed literals found embedded as strings, still to be parsed.
    // Kept as a separate worklist rather than pushed onto the borrow stack so no
    // frame ever borrows from a value another frame owns.
    let mut embedded: Vec<(String, String)> = Vec::new();
    walk_value(root, &mut out, &mut embedded, memory)?;
    while let Some((lexical, datatype)) = embedded.pop() {
        memory.admission_mut().checkpoint(1)?;
        let (value, bytes) = parse_composite(&lexical, &datatype, memory)?;
        walk_value(&value, &mut out, &mut embedded, memory)?;
        drop(value);
        memory.release_bytes(bytes)?;
        memory.release_string(lexical)?;
        memory.release_string(datatype)?;
    }
    memory.release_vec(embedded)?;
    Ok(out)
}

/// Collect `value`'s own blank labels and queue any composite-typed literal it
/// embeds, iteratively.
///
/// The value is a finite owning tree — every term is visited once and pushes only
/// the terms it owns — so the walk terminates on any value, whatever its depth and
/// however it was built.
fn walk_value<S: Admission + ?Sized>(
    value: &CdtValue,
    out: &mut Vec<String>,
    embedded: &mut Vec<(String, String)>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    let mut stack = purrdf_lex::walk::WorkList::<&CdtTerm, 16>::new();
    push_children(&mut stack, value, memory)?;
    while let Some(term) = stack.pop() {
        memory.admission_mut().checkpoint(1)?;
        match term {
            CdtTerm::Blank(label) => {
                let label = memory.string_with_progress(label)?;
                memory.push(out, label)?;
            }
            CdtTerm::Composite(inner) => push_children(&mut stack, inner, memory)?,
            CdtTerm::TripleTerm(triple) => {
                stack.try_push_admitted(&triple.subject, memory)?;
                stack.try_push_admitted(&triple.predicate, memory)?;
                stack.try_push_admitted(&triple.object, memory)?;
            }
            CdtTerm::Literal(lit) if is_cdt_datatype(&lit.datatype) => {
                let lexical = memory.string_with_progress(&lit.lexical)?;
                let datatype = memory.string_with_progress(&lit.datatype)?;
                memory.push(embedded, (lexical, datatype))?;
            }
            CdtTerm::Literal(_) | CdtTerm::Iri(_) | CdtTerm::Null => {}
        }
    }
    stack.release_admitted(memory)
}

/// Push a composite's element terms onto the walk stack. A `cdt:Map` key is
/// never a blank node (`CdtKey` makes that unrepresentable), so only values are
/// visited.
fn push_children<'a, S: Admission + ?Sized>(
    stack: &mut purrdf_lex::walk::WorkList<&'a CdtTerm, 16>,
    value: &'a CdtValue,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    match value.contents() {
        CdtContents::List(items) => {
            for item in items {
                memory.admission_mut().checkpoint(1)?;
                stack.try_push_admitted(item, memory)?;
            }
        }
        CdtContents::Map(entries) => {
            for entry in entries {
                memory.admission_mut().checkpoint(1)?;
                stack.try_push_admitted(&entry.value, memory)?;
            }
        }
    }
    Ok(())
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
fn scan_tokens_with_memory<S: Admission + ?Sized>(
    lexical: &str,
    memory: &mut Memory<'_, S>,
) -> Result<Vec<TokenSpan>, StorageError> {
    let mut spans = Vec::new();
    let mut queue = Vec::new();
    let text = memory.string_with_progress(lexical)?;
    memory.push(
        &mut queue,
        Region {
            text,
            map: Vec::new(),
        },
    )?;
    // One region per embedded composite literal; the outer parse's element
    // budget bounds how many there can be.
    let mut budget = purrdf_cdt::MAX_ELEMENTS;
    while let Some(region) = queue.pop() {
        memory.admission_mut().checkpoint(1)?;
        if budget == 0 {
            release_region(region, memory)?;
            break;
        }
        budget -= 1;
        scan_region(&region, &mut spans, &mut queue, memory)?;
        release_region(region, memory)?;
    }
    purrdf_lex::walk::try_sort_unstable_by(&mut spans, |left, right| {
        memory.admission_mut().checkpoint(1)?;
        Ok::<_, StorageError>(left.start.cmp(&right.start))
    })?;
    // A token located inside an embedded literal is nested INSIDE that
    // literal's own IRIREF-bearing span only in the datatype case, which is
    // scanned in the parent region; the label regions never overlap. Drop any
    // span that would still overlap its predecessor rather than trust that.
    let before = span_payload_bytes(&spans)?;
    let mut kept = 0;
    for read in 0..spans.len() {
        memory.admission_mut().checkpoint(1)?;
        if kept != 0 && spans[read].start < spans[kept - 1].end {
            continue;
        }
        spans.swap(kept, read);
        kept += 1;
    }
    spans.truncate(kept);
    let after = span_payload_bytes(&spans)?;
    memory.release_bytes(
        before
            .checked_sub(after)
            .ok_or(StorageError::SizeOverflow)?,
    )?;
    while let Some(region) = queue.pop() {
        release_region(region, memory)?;
    }
    memory.release_vec(queue)?;
    Ok(spans)
}

fn span_payload_bytes(spans: &[TokenSpan]) -> Result<usize, StorageError> {
    spans.iter().try_fold(0usize, |total, span| {
        total
            .checked_add(match &span.kind {
                TokenKind::Blank(text) => text.capacity(),
                TokenKind::Iri { iri, .. } => iri.capacity(),
            })
            .ok_or(StorageError::SizeOverflow)
    })
}

fn release_region<S: Admission + ?Sized>(
    region: Region,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    memory.release_string(region.text)?;
    memory.release_vec(region.map)
}

/// Scan one region, appending token spans and queueing the unescaped content of
/// every composite-typed literal it embeds.
fn scan_region<S: Admission + ?Sized>(
    region: &Region,
    spans: &mut Vec<TokenSpan>,
    queue: &mut Vec<Region>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    let bytes = region.text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        memory.admission_mut().checkpoint(16)?;
        match bytes[i] {
            // A triple term opens with `<<(`, whose leading `<` is not an IRI.
            b'<' if region.text[i..].starts_with("<<(") => i += 3,
            b'<' => {
                let (end, close) = scan_iriref(bytes, i, memory)?;
                if let Some(close) = close {
                    // An element-position IRI: `iri_only` is false, because the
                    // grammar admits a blank node in this very position.
                    push_iri_span(region, i, close, end, false, spans, memory)?;
                }
                i = end;
            }
            b'"' | b'\'' => i = scan_string(region, i, spans, queue, memory)?,
            b'_' if bytes.get(i + 1) == Some(&b':') => {
                let label_start = i + 2;
                let end = scan_label_body(&region.text, label_start, memory)?;
                if end > label_start {
                    let label = memory.string_with_progress(&region.text[label_start..end])?;
                    memory.push(
                        spans,
                        TokenSpan {
                            start: region.root_of(i),
                            end: region.root_of(end),
                            kind: TokenKind::Blank(label),
                        },
                    )?;
                }
                i = end.max(label_start);
            }
            _ => {
                // Advance a whole scalar so a multi-byte character is never split.
                i += region.text[i..].chars().next().map_or(1, char::len_utf8);
            }
        }
    }
    Ok(())
}

/// Scan an `IRIREF` opening at `start`, returning `(end, close)` — the offset
/// just past its `>`, and the offset OF that `>`.
///
/// `close` is `None` for an unterminated `IRIREF`, whose body would not be a
/// well-defined slice; the validating path refuses such a form outright and the
/// total path simply reports no token for it.
///
/// `UCHAR` escapes ride through: no escape spells a raw `>`.
fn scan_iriref<S: Admission + ?Sized>(
    bytes: &[u8],
    start: usize,
    memory: &mut Memory<'_, S>,
) -> Result<(usize, Option<usize>), StorageError> {
    let mut i = start + 1;
    while i < bytes.len() {
        memory.admission_mut().checkpoint(16)?;
        match bytes[i] {
            b'>' => return Ok((i + 1, Some(i))),
            b'\\' => i += 2,
            _ => i += 1,
        }
    }
    Ok((bytes.len(), None))
}

/// The end offset of the `BLANK_NODE_LABEL` body starting at `start`.
///
/// `BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')* PN_CHARS)?`
/// — the trailing `.` the production forbids is trimmed here, so `_:b.` scans as
/// the label `b` exactly as the grammar reads it.
fn scan_label_body<S: Admission + ?Sized>(
    text: &str,
    start: usize,
    memory: &mut Memory<'_, S>,
) -> Result<usize, StorageError> {
    let mut end = start;
    for ch in text[start..].chars() {
        memory.admission_mut().checkpoint(4)?;
        if crate::blank_label::is_pn_chars(ch) || ch == '.' {
            end += ch.len_utf8();
        } else {
            break;
        }
    }
    // Trim trailing dots: they are legal mid-label but never final.
    while end > start && text.as_bytes()[end - 1] == b'.' {
        memory.admission_mut().checkpoint(1)?;
        end -= 1;
    }
    // The first character must be PN_CHARS_U or a digit for this to be a label.
    Ok(match text[start..end].chars().next() {
        Some(first) if crate::blank_label::is_pn_chars_u(first) || first.is_ascii_digit() => end,
        _ => start,
    })
}

/// Record one `IRIREF` token: `open` is the `<`, `close` the `>`, `end` one past
/// the `>`.
fn push_iri_span<S: Admission + ?Sized>(
    region: &Region,
    open: usize,
    close: usize,
    end: usize,
    iri_only: bool,
    spans: &mut Vec<TokenSpan>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    let iri = match expand_uchars_with_memory(&region.text[open + 1..close], memory)? {
        Cow::Borrowed(text) => memory.string_with_progress(text)?,
        Cow::Owned(text) => text,
    };
    memory.push(
        spans,
        TokenSpan {
            start: region.root_of(open),
            end: region.root_of(end),
            kind: TokenKind::Iri { iri, iri_only },
        },
    )
}

/// Scan a quoted string opening at `start`, queueing its unescaped content when
/// a `^^` datatype marks it as an embedded composite literal. Returns the offset
/// just past the literal (including any datatype or language tag).
fn scan_string<S: Admission + ?Sized>(
    region: &Region,
    start: usize,
    spans: &mut Vec<TokenSpan>,
    queue: &mut Vec<Region>,
    memory: &mut Memory<'_, S>,
) -> Result<usize, StorageError> {
    let bytes = region.text.as_bytes();
    let quote = bytes[start];
    let long = region.text[start..].starts_with(if quote == b'"' { "\"\"\"" } else { "'''" });
    let open = if long { 3 } else { 1 };
    let content_start = start + open;

    let mut i = content_start;
    let mut content_end = bytes.len();
    let mut after = bytes.len();
    while i < bytes.len() {
        memory.admission_mut().checkpoint(16)?;
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
        return Ok(bytes.len());
    }

    // `String (LANGTAG | '^^' IRIREF)?` — only the typed form can be composite.
    if !region.text[after..].starts_with("^^") {
        return Ok(after);
    }
    let iri_start = after + 2;
    if bytes.get(iri_start) != Some(&b'<') {
        return Ok(iri_start);
    }
    let (iri_end, close) = scan_iriref(bytes, iri_start, memory)?;
    let Some(close) = close else {
        return Ok(iri_end);
    };
    // An IRIREF body's `UCHAR` escapes decoded, so a datatype written with them
    // still compares equal to the composite IRIs.
    let datatype = expand_uchars_with_memory(&region.text[iri_start + 1..close], memory)?;
    if is_cdt_datatype(&datatype) {
        let nested = unescaped_region(region, content_start, content_end, memory)?;
        memory.push(queue, nested)?;
    }
    if let Cow::Owned(datatype) = datatype {
        memory.release_string(datatype)?;
    }
    // A literal's datatype can never be a blank node, so a rewriter that mints
    // blanks from IRIs must refuse here rather than write an invalid literal.
    push_iri_span(region, iri_start, close, iri_end, true, spans, memory)?;
    Ok(iri_end)
}

/// Build the scan region for an embedded composite literal: its content
/// unescaped, plus the root offset of every resulting byte.
fn unescaped_region<S: Admission + ?Sized>(
    region: &Region,
    content_start: usize,
    content_end: usize,
    memory: &mut Memory<'_, S>,
) -> Result<Region, StorageError> {
    let raw = &region.text[content_start..content_end];
    let mut text = String::new();
    let mut map = Vec::new();
    memory.reserve_string(&mut text, raw.len())?;
    memory.reserve(
        &mut map,
        raw.len().checked_add(1).ok_or(StorageError::SizeOverflow)?,
    )?;
    let bytes = raw.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        memory.admission_mut().checkpoint(16)?;
        let root = region.root_of(content_start + i);
        if bytes[i] == b'\\' {
            let (ch, width) = decode_escape(raw, i);
            let mut buf = [0u8; 4];
            for _ in ch.encode_utf8(&mut buf).as_bytes() {
                // Every byte an escape produces points at the escape's opening
                // backslash: a label never rides in an escape, so this offset is
                // only ever consulted for bytes that are not part of a label.
                memory.push(&mut map, root)?;
            }
            memory.push_char(&mut text, ch)?;
            i += width;
        } else {
            let ch = raw[i..].chars().next().unwrap_or('\u{fffd}');
            for k in 0..ch.len_utf8() {
                memory.push(&mut map, root + k)?;
            }
            text.push(ch);
            i += ch.len_utf8();
        }
    }
    memory.push(&mut map, region.root_of(content_end))?;
    Ok(Region { text, map })
}

/// Decode one escape sequence at `at`, returning the scalar and the raw width
/// consumed. An unrecognized sequence yields its own backslash so the scan stays
/// total; the grammar has already refused such a form on the validating path.
fn decode_escape(raw: &str, at: usize) -> (char, usize) {
    let bytes = &raw.as_bytes()[at..];
    if let Some(decoded) = bytes.get(1).copied().and_then(terminals::echar_value) {
        return (decoded, 2);
    }
    terminals::decode_uchar(bytes).unwrap_or(('\\', 1))
}

#[cfg(test)]
mod tests;
