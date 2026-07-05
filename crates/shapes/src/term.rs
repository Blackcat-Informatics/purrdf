// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SHACL engine's native RDF 1.2 term value model.
//!
//! The engine, constraint evaluator, path evaluator, shape parser, and report all
//! work over ONE term value type. Historically that type was
//! `oxigraph::model::Term`; this module replaces it with an oxigraph-free native
//! model built from `String` IRIs and [`purrdf::ir::TermRef`] resolution.
//!
//! # Rendering contract (behavior-preserving)
//!
//! `Term::to_string` reproduces oxigraph's `Term::to_string()` **byte-for-byte**,
//! because the engine uses the string rendering as its deterministic sort key
//! ([`crate::engine`]) and the report serialization / Python surface
//! ([`crate::report`]) compare on it. The contract verified against
//! oxigraph 0.5 is:
//!
//! - IRI → `<iri>`
//! - blank node → `_:label`
//! - plain `xsd:string` / lang-string-typed literal → `"lex"` (NO datatype)
//! - other typed literal → `"lex"^^<datatype>`
//! - language-tagged literal → `"lex"@tag` (plus `--ltr`/`--rtl` when directional)
//! - quoted triple → `<<( <s> <p> <o> )>>`
//!
//! Literal lexical forms escape `\\ \" \n \r \t` plus C0 control chars as `\u00XX`,
//! exactly as oxigraph's N-Triples literal writer.

use ::purrdf::{RdfDataset, TermRef};
use ::purrdf::{RdfTextDirection, TermId, TermValue};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// A native RDF term IRI (named node). Wraps a `String`; mirrors the slice of the
/// oxigraph `NamedNode` API the engine actually uses (`as_str`, `Ord`, `Display`).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NamedNode(String);

impl NamedNode {
    /// Construct from an IRI string without validation (the IR has already validated
    /// lexical well-formedness at ingest).
    #[inline]
    pub fn new_unchecked(iri: impl Into<String>) -> Self {
        Self(iri.into())
    }

    /// The IRI string.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into the owned IRI string.
    #[inline]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Wrap this IRI into a [`Term::NamedNode`].
    #[inline]
    pub fn into_term(self) -> Term {
        Term::NamedNode(self)
    }
}

impl std::fmt::Display for NamedNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{}>", self.0)
    }
}

impl From<&str> for NamedNode {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

/// A native RDF literal. Carries the lexical form, the datatype IRI (always present
/// — the IR expands `xsd:string`/`rdf:langString` per C0.1), and the optional
/// language tag + base direction.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Literal {
    lexical: String,
    datatype: String,
    language: Option<String>,
    direction: Option<RdfTextDirection>,
}

impl Literal {
    /// A plain `xsd:string` literal.
    #[inline]
    pub fn new_simple_literal(value: impl Into<String>) -> Self {
        Self {
            lexical: value.into(),
            datatype: XSD_STRING.to_owned(),
            language: None,
            direction: None,
        }
    }

    /// A typed literal with an explicit datatype IRI.
    #[inline]
    pub fn new_typed_literal(value: impl Into<String>, datatype: NamedNode) -> Self {
        Self {
            lexical: value.into(),
            datatype: datatype.0,
            language: None,
            direction: None,
        }
    }

    /// A language-tagged literal (datatype `rdf:langString`).
    #[inline]
    pub fn new_language_tagged_literal_unchecked(
        value: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        Self {
            lexical: value.into(),
            datatype: RDF_LANG_STRING.to_owned(),
            language: Some(language.into()),
            direction: None,
        }
    }

    /// A directional language-tagged literal (RDF 1.2).
    #[inline]
    pub fn new_directional_language_tagged_literal_unchecked(
        value: impl Into<String>,
        language: impl Into<String>,
        direction: RdfTextDirection,
    ) -> Self {
        Self {
            lexical: value.into(),
            datatype: RDF_LANG_STRING.to_owned(),
            language: Some(language.into()),
            direction: Some(direction),
        }
    }

    /// The lexical form.
    #[inline]
    pub fn value(&self) -> &str {
        &self.lexical
    }

    /// The language tag, if this is a language-tagged literal.
    #[inline]
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// The datatype IRI as a [`NamedNode`] view.
    #[inline]
    pub fn datatype(&self) -> NamedNode {
        NamedNode(self.datatype.clone())
    }

    /// The datatype IRI string (allocation-free).
    #[inline]
    pub fn datatype_str(&self) -> &str {
        &self.datatype
    }
}

/// A native RDF 1.2 quoted triple (statement-layer term).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Triple {
    pub subject: Term,
    pub predicate: NamedNode,
    pub object: Term,
}

impl Triple {
    #[inline]
    pub fn new(subject: Term, predicate: NamedNode, object: Term) -> Self {
        Self {
            subject,
            predicate,
            object,
        }
    }
}

/// A native RDF 1.2 term — the SHACL engine's value model. Variants mirror
/// `oxigraph::model::Term` so the constraint/shape/path logic keeps its shape.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Term {
    /// An IRI.
    NamedNode(NamedNode),
    /// A blank node (label only; the IR scope-qualifies the label at conversion).
    BlankNode(String),
    /// A literal.
    Literal(Literal),
    /// A quoted triple (RDF 1.2).
    Triple(Box<Triple>),
}

impl Term {
    /// Construct a blank-node term from its label.
    #[inline]
    pub fn blank(label: impl Into<String>) -> Self {
        Self::BlankNode(label.into())
    }

    /// The blank-node label, if this term is a blank node.
    #[inline]
    pub fn blank_label(&self) -> Option<&str> {
        match self {
            Self::BlankNode(b) => Some(b.as_str()),
            _ => None,
        }
    }

    /// Whether this term can occupy a subject position (IRI or blank node).
    #[inline]
    pub fn is_subject(&self) -> bool {
        matches!(self, Self::NamedNode(_) | Self::BlankNode(_))
    }

    /// Convert this native term into the owned [`RdfTerm`](purrdf::RdfTerm) model — used when
    /// building a report dataset for serialization.
    pub fn to_rdf_term(&self) -> ::purrdf::RdfTerm {
        use purrdf::{RdfLiteral, RdfTerm, RdfTriple};
        match self {
            Self::NamedNode(n) => RdfTerm::iri(n.0.clone()),
            Self::BlankNode(b) => RdfTerm::blank_node(b.clone()),
            Self::Literal(l) => {
                // The owned model carries `datatype: None` for a plain `xsd:string`
                // and for a language-tagged literal (the tag implies rdf:langString);
                // an explicit datatype otherwise — matching how the codec round-trips.
                let datatype = if l.language.is_some() || l.datatype == XSD_STRING {
                    None
                } else {
                    Some(l.datatype.clone())
                };
                RdfTerm::Literal(RdfLiteral {
                    lexical_form: l.lexical.clone(),
                    datatype,
                    language: l.language.clone(),
                    direction: l.direction,
                })
            }
            Self::Triple(t) => RdfTerm::triple(RdfTriple::new(
                t.subject.to_rdf_term(),
                t.predicate.0.clone(),
                t.object.to_rdf_term(),
            )),
        }
    }

    /// Convert this native term into a dataset-independent [`TermValue`] — the SPARQL
    /// substitution value and the canonical lookup key.
    pub fn to_term_value(&self) -> TermValue {
        match self {
            Self::NamedNode(n) => TermValue::Iri(n.0.clone()),
            // The IR conversion scope-qualified the label; round-trip it in the
            // DEFAULT scope (single-scope data is byte-unchanged).
            Self::BlankNode(b) => TermValue::blank(b.clone()),
            Self::Literal(l) => TermValue::Literal {
                lexical_form: l.lexical.clone(),
                datatype: l.datatype.clone(),
                language: l.language.clone(),
                direction: l.direction,
            },
            Self::Triple(t) => TermValue::Triple {
                s: Box::new(t.subject.to_term_value()),
                p: Box::new(t.predicate.to_term_value_iri()),
                o: Box::new(t.object.to_term_value()),
            },
        }
    }

    /// The candidate [`TermValue`] lookup keys to resolve this pattern term against a
    /// dataset's value→id index.
    ///
    /// For most terms this is a single key ([`to_term_value`](Self::to_term_value)).
    /// A blank node is the exception: [`term_ref_to_native`] flattens the IR's
    /// `(label, scope)` into ONE qualified label string (`"{label}.s{n}"` for a
    /// non-default scope `n`). That qualified label round-trips correctly when the
    /// dataset stored the blank at the DEFAULT scope (the SHACL-projected dataset
    /// re-interns owned blanks there), but NOT when the dataset preserves the original
    /// non-default scope (the raw shapes dataset). So for a blank carrying a `.s{n}`
    /// suffix we ALSO offer the de-qualified `(label, scope_n)` key; the caller tries
    /// each until one resolves. The DEFAULT-scope key stays FIRST so single-scope data
    /// keeps its fast path.
    pub fn lookup_term_values(&self) -> Vec<TermValue> {
        match self {
            Self::BlankNode(b) => {
                let mut keys = vec![TermValue::blank(b.clone())];
                if let Some((label, scope)) = split_scope_suffix(b) {
                    keys.push(TermValue::Blank {
                        label: label.to_owned(),
                        scope: ::purrdf::BlankScope(scope),
                    });
                }
                keys
            }
            other => vec![other.to_term_value()],
        }
    }
}

/// Split a scope-qualified blank label `"{label}.s{n}"` (n > 0) into `(label, n)`,
/// the inverse of [`BlankScope::qualify_label`](::purrdf::BlankScope::qualify_label).
/// Returns `None` for a bare (default-scope) label.
fn split_scope_suffix(qualified: &str) -> Option<(&str, u32)> {
    let dot = qualified.rfind(".s")?;
    let digits = &qualified[dot + 2..];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let scope: u32 = digits.parse().ok()?;
    if scope == 0 {
        return None;
    }
    Some((&qualified[..dot], scope))
}

impl NamedNode {
    /// The IRI as a [`TermValue::Iri`].
    #[inline]
    fn to_term_value_iri(&self) -> TermValue {
        TermValue::Iri(self.0.clone())
    }
}

impl std::fmt::Display for Term {
    /// Render byte-for-byte as `oxigraph::model::Term::to_string()` — the engine's
    /// deterministic sort key and report identity depend on this.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NamedNode(n) => write!(f, "<{}>", n.0),
            Self::BlankNode(b) => write!(f, "_:{b}"),
            Self::Literal(l) => write!(f, "{}", render_literal(l)),
            Self::Triple(t) => write!(f, "<<( {} <{}> {} )>>", t.subject, t.predicate.0, t.object),
        }
    }
}

/// Render a literal exactly as oxigraph's `Term::to_string()` does.
fn render_literal(l: &Literal) -> String {
    let lex = escape_literal(&l.lexical);
    if let Some(lang) = &l.language {
        return match l.direction {
            Some(RdfTextDirection::Ltr) => format!("\"{lex}\"@{lang}--ltr"),
            Some(RdfTextDirection::Rtl) => format!("\"{lex}\"@{lang}--rtl"),
            None => format!("\"{lex}\"@{lang}"),
        };
    }
    // Plain `xsd:string` (and the rare `rdf:langString` without a tag) render with
    // NO datatype suffix, matching oxigraph.
    if l.datatype == XSD_STRING || l.datatype == RDF_LANG_STRING {
        return format!("\"{lex}\"");
    }
    format!("\"{lex}\"^^<{}>", l.datatype)
}

/// Escape a literal lexical form exactly as oxigraph's N-Triples literal writer:
/// `\\ \" \n \r \t` plus C0 control characters as `\u00XX`.
fn escape_literal(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            other => out.push(other),
        }
    }
    out
}

/// Convert a resolved IR [`TermRef`] into a native [`Term`], recursing into triple
/// components via the dataset's [`resolve`](RdfDataset::resolve).
///
/// Blank labels are scope-qualified so two same-label blanks from different
/// [`BlankScope`](::purrdf::BlankScope)s never conflate (C0.2); the DEFAULT
/// scope keeps the bare label so single-scope data is byte-unchanged.
pub fn term_ref_to_native(dataset: &RdfDataset, term: TermRef<'_>) -> Term {
    match term {
        TermRef::Iri(iri) => Term::NamedNode(NamedNode::new_unchecked(iri)),
        TermRef::Blank { label, scope } => Term::BlankNode(scope.qualify_label(label).into_owned()),
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let datatype_iri = match dataset.resolve(datatype) {
                TermRef::Iri(iri) => iri.to_owned(),
                other => unreachable!("a literal datatype must resolve to an IRI, got {other:?}"),
            };
            Term::Literal(Literal {
                lexical: lexical.to_owned(),
                datatype: datatype_iri,
                language: language.map(str::to_owned),
                direction,
            })
        }
        TermRef::Triple { s, p, o } => {
            let subject = term_ref_to_native(dataset, dataset.resolve(s));
            let predicate = match term_ref_to_native(dataset, dataset.resolve(p)) {
                Term::NamedNode(n) => n,
                other => unreachable!("a triple predicate must be an IRI, got {other:?}"),
            };
            let object = term_ref_to_native(dataset, dataset.resolve(o));
            Term::Triple(Box::new(Triple::new(subject, predicate, object)))
        }
    }
}

/// Convert a resolved IR term id into a native [`Term`].
#[inline]
pub fn term_id_to_native(dataset: &RdfDataset, id: TermId) -> Term {
    term_ref_to_native(dataset, dataset.resolve(id))
}

/// Convert a dataset-independent [`TermValue`] (e.g. a SPARQL egress binding) into a
/// native [`Term`].
pub fn term_value_to_native(value: &TermValue) -> Term {
    match value {
        TermValue::Iri(iri) => Term::NamedNode(NamedNode::new_unchecked(iri.clone())),
        TermValue::Blank { label, .. } => Term::BlankNode(label.clone()),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => Term::Literal(Literal {
            lexical: lexical_form.clone(),
            datatype: datatype.clone(),
            language: language.clone(),
            direction: *direction,
        }),
        TermValue::Triple { s, p, o } => {
            let predicate = match term_value_to_native(p) {
                Term::NamedNode(n) => n,
                other => unreachable!("a triple predicate must be an IRI, got {other:?}"),
            };
            Term::Triple(Box::new(Triple::new(
                term_value_to_native(s),
                predicate,
                term_value_to_native(o),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nn(iri: &str) -> Term {
        Term::NamedNode(NamedNode::new_unchecked(iri))
    }

    #[test]
    fn renders_iri_and_blank_like_oxigraph() {
        assert_eq!(nn("http://e/s").to_string(), "<http://e/s>");
        assert_eq!(Term::blank("b0").to_string(), "_:b0");
    }

    #[test]
    fn renders_plain_string_without_datatype() {
        let t = Term::Literal(Literal::new_simple_literal("hi"));
        assert_eq!(t.to_string(), "\"hi\"");
    }

    #[test]
    fn renders_typed_literal_with_datatype() {
        let t = Term::Literal(Literal::new_typed_literal(
            "42",
            NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
        ));
        assert_eq!(
            t.to_string(),
            "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
    }

    #[test]
    fn renders_lang_and_directional_literal() {
        let lang = Term::Literal(Literal::new_language_tagged_literal_unchecked("hi", "en"));
        assert_eq!(lang.to_string(), "\"hi\"@en");
        let dir = Term::Literal(Literal::new_directional_language_tagged_literal_unchecked(
            "hi",
            "en",
            RdfTextDirection::Rtl,
        ));
        assert_eq!(dir.to_string(), "\"hi\"@en--rtl");
    }

    #[test]
    fn renders_quoted_triple_like_oxigraph() {
        let t = Term::Triple(Box::new(Triple::new(
            NamedNode::new_unchecked("http://e/s").into_term(),
            NamedNode::new_unchecked("http://e/p"),
            NamedNode::new_unchecked("http://e/o").into_term(),
        )));
        assert_eq!(
            t.to_string(),
            "<<( <http://e/s> <http://e/p> <http://e/o> )>>"
        );
    }

    #[test]
    fn escapes_special_chars_like_oxigraph() {
        let t = Term::Literal(Literal::new_simple_literal("a\"b\nc\td\\e\u{0007}f"));
        assert_eq!(t.to_string(), "\"a\\\"b\\nc\\td\\\\e\\u0007f\"");
    }
}
