// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Term crossing — the single canonical bridge between RDF terms and the C
//! boundary.
//!
//! Three representations are offered (the spec rejects per-row N-Triples reparse
//! as the only path):
//!   1. **Structured term views** ([`PurrdfTermView`]) with borrowed UTF-8
//!      slices — the hot path for iteration.
//!   2. A **cursor-scoped opaque term id** (`PurrdfTermView::term_id`) — lets a
//!      caller re-address a term (notably a quoted triple, whose components do
//!      not fit a flat view) against the dataset it came from.
//!   3. The **N-Triples convenience** function [`purrdf_term_to_ntriples`] for
//!      the simple/robust path and for materializing quoted-triple terms.
//!
//! This module owns BOTH directions (view → owned `TermValue` for inputs;
//! `TermRef`/id → view for outputs) so the mapping lives in exactly one place.

use purrdf_core::model::{RdfLiteral, RdfTerm, RdfTextDirection};
use purrdf_core::{emit_term, BlankScope, RdfDataset, TermId, TermRef, TermValue};

use crate::buffer::PurrdfBuffer;
use crate::error::PurrdfError;
use crate::handles::PurrdfDataset;
use crate::status::PurrdfStatus;

/// The IRI of `xsd:string`, the default datatype for a literal with no language.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
/// The IRI of `rdf:langString`, the datatype of a language-tagged literal.
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// The kind tag of a [`PurrdfTermView`].
///
/// These are the canonical discriminant values for the `int32_t kind` field of
/// [`PurrdfTermView`]. An unknown value in that field yields
/// [`PurrdfStatus::InvalidArgument`] — never UB.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PurrdfTermKind {
    /// An IRI; `lexical` is the IRI string.
    Iri = 0,
    /// A blank node; `lexical` is the label, `blank_scope` the scope ordinal.
    Blank = 1,
    /// A literal; `lexical`/`datatype`/`language`/`direction` are meaningful.
    Literal = 2,
    /// An RDF-1.2 quoted triple; `lexical` is empty — materialize the components
    /// via `purrdf_term_to_ntriples` using this view's `term_id`.
    Triple = 3,
}

impl TryFrom<i32> for PurrdfTermKind {
    type Error = PurrdfError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PurrdfTermKind::Iri),
            1 => Ok(PurrdfTermKind::Blank),
            2 => Ok(PurrdfTermKind::Literal),
            3 => Ok(PurrdfTermKind::Triple),
            _ => Err(PurrdfError::new(
                PurrdfStatus::InvalidArgument,
                format!("unknown PurrdfTermKind discriminant: {value}"),
            )),
        }
    }
}

/// The optional base direction of a literal (RDF-1.2 `i18n` direction).
///
/// These are the canonical discriminant values for the `int32_t direction` field
/// of [`PurrdfTermView`]. An unknown value yields [`PurrdfStatus::InvalidArgument`]
/// — never UB.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PurrdfDirection {
    /// No base direction.
    None = 0,
    /// Left-to-right.
    Ltr = 1,
    /// Right-to-left.
    Rtl = 2,
}

impl TryFrom<i32> for PurrdfDirection {
    type Error = PurrdfError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PurrdfDirection::None),
            1 => Ok(PurrdfDirection::Ltr),
            2 => Ok(PurrdfDirection::Rtl),
            _ => Err(PurrdfError::new(
                PurrdfStatus::InvalidArgument,
                format!("unknown PurrdfDirection discriminant: {value}"),
            )),
        }
    }
}

/// A borrowed UTF-8 slice. `ptr` is **not** NUL-terminated — use `len`. The
/// memory is owned by libpurrdf; the C side must **never** `free()` it. As an
/// output, a view's pointers are valid only until the next `*_cursor_next` /
/// `*_rowcursor_next` on the same cursor, or until the owning handle is freed.
/// As an input, the caller owns the bytes and they need only outlive the call.
/// `ptr` may be null when `len == 0`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PurrdfStr {
    /// Pointer to the first byte (borrowed; never freed by the other side).
    pub ptr: *const u8,
    /// Length in bytes.
    pub len: usize,
}

impl PurrdfStr {
    /// An empty slice (null pointer, zero length).
    pub(crate) fn empty() -> Self {
        Self {
            ptr: std::ptr::null(),
            len: 0,
        }
    }

    /// Borrow a `&str` as a `PurrdfStr` pointing into the same memory.
    pub(crate) fn from_str(value: &str) -> Self {
        Self {
            ptr: value.as_ptr(),
            len: value.len(),
        }
    }

    /// Read a `PurrdfStr` back as `&str`. Empty (`len == 0`) yields `""`.
    ///
    /// # Safety
    /// For non-empty slices, `ptr` must be valid for `len` bytes for `'a`.
    pub(crate) unsafe fn as_str<'a>(self) -> Result<&'a str, PurrdfError> {
        if self.len == 0 {
            return Ok("");
        }
        if self.ptr.is_null() {
            return Err(PurrdfError::new(
                PurrdfStatus::NullPointer,
                "PurrdfStr has a null pointer with non-zero length",
            ));
        }
        let bytes = std::slice::from_raw_parts(self.ptr, self.len);
        std::str::from_utf8(bytes).map_err(|_| {
            PurrdfError::new(PurrdfStatus::InvalidUtf8, "PurrdfStr is not valid UTF-8")
        })
    }
}

/// A structured, borrowed term view. Which fields are meaningful depends on
/// `kind` (see [`PurrdfTermKind`]). All `PurrdfStr` fields follow the borrowing
/// contract on [`PurrdfStr`]. `term_id` is the dataset-local opaque id of the
/// term (`0` when the view was not produced from an interned dataset term, e.g.
/// a SPARQL solution value); it is meaningful ONLY against the dataset that
/// produced the view and must never be compared across datasets.
///
/// `kind` is an `int32_t` carrying a [`PurrdfTermKind`] discriminant (0–3).
/// `direction` is an `int32_t` carrying a [`PurrdfDirection`] discriminant (0–2).
/// An unknown value in either field yields [`PurrdfStatus::InvalidArgument`] —
/// never undefined behaviour.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PurrdfTermView {
    /// The term kind tag (`int32_t`; see [`PurrdfTermKind`] for valid values).
    /// An unknown discriminant yields `PurrdfStatus::InvalidArgument`, not UB.
    pub kind: i32,
    /// IRI string / blank label / literal lexical form (empty for `Triple`).
    pub lexical: PurrdfStr,
    /// Datatype IRI (`Literal` only; empty otherwise).
    pub datatype: PurrdfStr,
    /// Language tag (`Literal` only; empty when absent).
    pub language: PurrdfStr,
    /// Base direction (`int32_t`; see [`PurrdfDirection`] for valid values;
    /// `Literal` only). An unknown discriminant yields `PurrdfStatus::InvalidArgument`,
    /// not UB.
    pub direction: i32,
    /// Blank-node scope ordinal (`Blank` only).
    pub blank_scope: u32,
    /// Dataset-local opaque term id (`0` = none). See the struct docs.
    pub term_id: u32,
}

impl PurrdfTermView {
    /// A zeroed view (kind `Iri`, all slices empty, no id). Used to initialize
    /// out-params before a cursor fills them.
    pub(crate) fn empty() -> Self {
        Self {
            kind: PurrdfTermKind::Iri as i32,
            lexical: PurrdfStr::empty(),
            datatype: PurrdfStr::empty(),
            language: PurrdfStr::empty(),
            direction: PurrdfDirection::None as i32,
            blank_scope: 0,
            term_id: 0,
        }
    }
}

/// The kind tag of a [`PurrdfGraphMatch`].
///
/// These are the canonical discriminant values for the `int32_t kind` field of
/// [`PurrdfGraphMatch`]. An unknown value yields [`PurrdfStatus::InvalidArgument`]
/// — never UB.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PurrdfGraphMatchKind {
    /// Match any graph (default or named).
    Any = 0,
    /// Match only the default graph.
    Default = 1,
    /// Match the named graph given by `name`.
    Named = 2,
}

impl TryFrom<i32> for PurrdfGraphMatchKind {
    type Error = PurrdfError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PurrdfGraphMatchKind::Any),
            1 => Ok(PurrdfGraphMatchKind::Default),
            2 => Ok(PurrdfGraphMatchKind::Named),
            _ => Err(PurrdfError::new(
                PurrdfStatus::InvalidArgument,
                format!("unknown PurrdfGraphMatchKind discriminant: {value}"),
            )),
        }
    }
}

/// A graph-slot match passed by value into `purrdf_quads_for_pattern`. For
/// `Named`, `name` is an **input** term view the caller fills.
///
/// `kind` is an `int32_t` carrying a [`PurrdfGraphMatchKind`] discriminant (0–2).
/// An unknown value yields [`PurrdfStatus::InvalidArgument`] — never UB.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PurrdfGraphMatch {
    /// Which graphs to match (`int32_t`; see [`PurrdfGraphMatchKind`] for valid
    /// values). An unknown discriminant yields `PurrdfStatus::InvalidArgument`, not UB.
    pub kind: i32,
    /// The named-graph term (meaningful only when `kind == Named`).
    pub name: PurrdfTermView,
}

/// Encode a dataset term id as the view's `term_id` field (`index + 1`, so `0`
/// stays the "none" sentinel).
fn encode_id(id: TermId) -> u32 {
    id.index() as u32 + 1
}

/// Decode a view `term_id` back to a [`TermId`], or `None` for the `0` sentinel.
fn decode_id(term_id: u32) -> Option<TermId> {
    term_id.checked_sub(1).map(TermId::from_index)
}

/// Render a dataset term (by id) into a borrowed structured view. The view's
/// `PurrdfStr` pointers borrow into `dataset`'s arena and are valid as long as
/// the dataset (or a cursor pinning it) is alive. For literals this performs the
/// nested datatype resolve (the datatype is itself a term id).
///
/// # Safety
/// `dataset` must outlive every use of the slices written into `view`.
pub(crate) unsafe fn render_term(dataset: &RdfDataset, id: TermId, view: &mut PurrdfTermView) {
    view.term_id = encode_id(id);
    view.blank_scope = 0;
    view.datatype = PurrdfStr::empty();
    view.language = PurrdfStr::empty();
    view.direction = PurrdfDirection::None as i32;
    match dataset.resolve(id) {
        TermRef::Iri(iri) => {
            view.kind = PurrdfTermKind::Iri as i32;
            view.lexical = PurrdfStr::from_str(iri);
        }
        TermRef::Blank { label, scope } => {
            view.kind = PurrdfTermKind::Blank as i32;
            view.lexical = PurrdfStr::from_str(label);
            view.blank_scope = scope.ordinal();
        }
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            view.kind = PurrdfTermKind::Literal as i32;
            view.lexical = PurrdfStr::from_str(lexical);
            // The datatype is a term id; resolve it to the IRI slice (C0.1).
            if let TermRef::Iri(dt) = dataset.resolve(datatype) {
                view.datatype = PurrdfStr::from_str(dt);
            }
            if let Some(language) = language {
                view.language = PurrdfStr::from_str(language);
            }
            view.direction = match direction {
                Option::None => PurrdfDirection::None as i32,
                Some(RdfTextDirection::Ltr) => PurrdfDirection::Ltr as i32,
                Some(RdfTextDirection::Rtl) => PurrdfDirection::Rtl as i32,
            };
        }
        TermRef::Triple { .. } => {
            view.kind = PurrdfTermKind::Triple as i32;
            view.lexical = PurrdfStr::empty();
        }
    }
}

/// Render an owned, dataset-independent [`TermValue`] into a borrowed structured
/// view whose `PurrdfStr` pointers borrow into `value`'s strings (so `value`
/// must outlive every use of the view). `term_id` is `0` (the value is not
/// dataset-interned). Used by the SPARQL row cursor. Quoted-triple solution
/// values render as `kind == Triple` with empty slices and no id (a documented
/// v0.1 limitation — they cannot be re-materialized via `term_to_ntriples`).
///
/// # Safety
/// `value` must outlive every use of the slices written into `view`.
pub(crate) unsafe fn render_value(value: &TermValue, view: &mut PurrdfTermView) {
    view.term_id = 0;
    view.blank_scope = 0;
    view.datatype = PurrdfStr::empty();
    view.language = PurrdfStr::empty();
    view.direction = PurrdfDirection::None as i32;
    match value {
        TermValue::Iri(iri) => {
            view.kind = PurrdfTermKind::Iri as i32;
            view.lexical = PurrdfStr::from_str(iri);
        }
        TermValue::Blank { label, scope } => {
            view.kind = PurrdfTermKind::Blank as i32;
            view.lexical = PurrdfStr::from_str(label);
            view.blank_scope = scope.ordinal();
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            view.kind = PurrdfTermKind::Literal as i32;
            view.lexical = PurrdfStr::from_str(lexical_form);
            view.datatype = PurrdfStr::from_str(datatype);
            if let Some(language) = language {
                view.language = PurrdfStr::from_str(language);
            }
            view.direction = match direction {
                Option::None => PurrdfDirection::None as i32,
                Some(RdfTextDirection::Ltr) => PurrdfDirection::Ltr as i32,
                Some(RdfTextDirection::Rtl) => PurrdfDirection::Rtl as i32,
            };
        }
        TermValue::Triple { .. } => {
            view.kind = PurrdfTermKind::Triple as i32;
            view.lexical = PurrdfStr::empty();
        }
    }
}

/// Convert an input term view to an owned, dataset-independent [`TermValue`].
/// Quoted-triple terms cannot be reconstructed from a flat view, so they are
/// rejected as inputs (a documented v0.1 limitation).
///
/// # Safety
/// The view's `PurrdfStr` slices must be valid for the call.
pub(crate) unsafe fn view_to_value(view: &PurrdfTermView) -> Result<TermValue, PurrdfError> {
    let lexical = view.lexical.as_str()?;
    let kind = PurrdfTermKind::try_from(view.kind)?;
    match kind {
        PurrdfTermKind::Iri => Ok(TermValue::Iri(lexical.to_owned())),
        PurrdfTermKind::Blank => Ok(TermValue::Blank {
            label: lexical.to_owned(),
            scope: BlankScope(view.blank_scope),
        }),
        PurrdfTermKind::Literal => {
            let language = if view.language.len == 0 {
                None
            } else {
                Some(view.language.as_str()?.to_owned())
            };
            let datatype_in = view.datatype.as_str()?;
            let datatype = if !datatype_in.is_empty() {
                datatype_in.to_owned()
            } else if language.is_some() {
                RDF_LANG_STRING.to_owned()
            } else {
                XSD_STRING.to_owned()
            };
            let direction = match PurrdfDirection::try_from(view.direction)? {
                PurrdfDirection::None => None,
                PurrdfDirection::Ltr => Some(RdfTextDirection::Ltr),
                PurrdfDirection::Rtl => Some(RdfTextDirection::Rtl),
            };
            Ok(TermValue::Literal {
                lexical_form: lexical.to_owned(),
                datatype,
                language,
                direction,
            })
        }
        PurrdfTermKind::Triple => Err(PurrdfError::new(
            PurrdfStatus::InvalidArgument,
            "quoted-triple terms cannot be passed by value as an input view in libpurrdf 0.1",
        )),
    }
}

/// Build an owned [`RdfTerm`] from an input view (non-triple), for N-Triples
/// rendering when the view carries no dataset id.
unsafe fn view_to_rdf_term(view: &PurrdfTermView) -> Result<RdfTerm, PurrdfError> {
    match view_to_value(view)? {
        TermValue::Iri(iri) => Ok(RdfTerm::iri(iri)),
        TermValue::Blank { label, .. } => Ok(RdfTerm::blank_node(label)),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => Ok(RdfTerm::literal(RdfLiteral {
            lexical_form,
            datatype: Some(datatype),
            language,
            direction,
        })),
        TermValue::Triple { .. } => Err(PurrdfError::new(
            PurrdfStatus::InvalidArgument,
            "cannot render a quoted-triple term without a dataset id",
        )),
    }
}

/// Render a single term view to one N-Triples term token (e.g. `<iri>`, `_:b`,
/// `"lex"^^<dt>`, or `<< s p o >>` for a quoted triple) into `*out_buffer`
/// (UTF-8, no trailing dot). When the view carries a dataset `term_id`, the term
/// is resolved against `dataset` (so quoted triples materialize recursively);
/// otherwise the token is built from the view fields (non-triple only).
///
/// # Safety
/// `dataset` must be the live handle the view's `term_id` came from (when
/// non-zero); `view` and the out-params must be valid.
#[no_mangle]
pub unsafe extern "C" fn purrdf_term_to_ntriples(
    dataset: *const PurrdfDataset,
    view: *const PurrdfTermView,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    ffi_try!(out_error, {
        if view.is_null() || out_buffer.is_null() {
            return Err(PurrdfError::new(
                PurrdfStatus::NullPointer,
                "null pointer argument to purrdf_term_to_ntriples",
            ));
        }
        let view = &*view;
        let token = match decode_id(view.term_id) {
            Some(id) => {
                if dataset.is_null() {
                    return Err(PurrdfError::new(
                        PurrdfStatus::NullPointer,
                        "a view with a dataset term_id requires its dataset to render N-Triples",
                    ));
                }
                emit_term(&PurrdfDataset::dataset(dataset).to_owned_term(id))
            }
            None => emit_term(&view_to_rdf_term(view)?),
        };
        *out_buffer = PurrdfBuffer::into_raw(token.into_bytes());
        Ok(PurrdfStatus::Ok)
    })
}
