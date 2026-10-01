// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RDF/JS `Term` and `Quad` types over the [`purrdf`] value model.
//!
//! The [RDF/JS](https://rdf.js.org/data-model-spec/) data model is by-value: a term is
//! a plain object with a `termType` discriminator, a `value` string, and structural
//! `equals`. Triple terms use flat shared views built through [`TermValue`]'s work list, while
//! leaf terms hold their strings, extended with the two RDF/JS term kinds the RDF
//! data model lacks (`Variable`, `DefaultGraph`).
//!
//! The **RDF-1.2 wedge** lives here: a quoted-triple term (`RdfTerm::Triple`) surfaces
//! as `termType: "Quad"` with `subject`/`predicate`/`object` accessors, and literals
//! carry the RDF-1.2 base `direction` — neither of which any incumbent RDF/JS library
//! models.

use crate::convert::{BlankScopeMode, term_value_into_rdf_term};
#[cfg(test)]
use purrdf::RdfTriple;
use purrdf::ir::{Nested, try_fold_nested};
use purrdf::{RdfLiteral, RdfTerm, RdfTextDirection, TermBox, TermValue};
use purrdf_core::langtag::identity_fold;
use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[cfg(test)]
use purrdf_core::datatype::XSD_STRING;
#[cfg(test)]
use purrdf_core::vocab::rdf::{
    DIR_LANG_STRING as RDF_DIR_LANG_STRING, LANG_STRING as RDF_LANG_STRING,
};

/// The internal shape of a [`Term`]. Covers the four RDF term kinds plus the two
/// query/graph term kinds RDF/JS adds (`Variable`, `DefaultGraph`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TermInner {
    /// `termType: "NamedNode"` — an IRI.
    Named(String),
    /// `termType: "BlankNode"` — a blank-node label (without the `_:` prefix).
    Blank(String),
    /// `termType: "Literal"`.
    Literal(RdfLiteral),
    /// `termType: "Quad"` used as a term, with iterative core clone, equality and drop.
    Quoted(QuotedTerm),
    /// `termType: "Variable"` — a SPARQL variable name (without the `?`).
    Variable(String),
    /// `termType: "DefaultGraph"`.
    DefaultGraph,
}

/// A shared flat triple-term tree. Getter views retain the same allocation, so walking
/// a remote result does not clone every remaining suffix. Flat nodes drop without recursion.
#[derive(Debug)]
enum QuotedNode {
    Leaf(TermValue),
    Triple { s: usize, p: usize, o: usize },
}

#[derive(Clone)]
pub(crate) struct QuotedTerm {
    nodes: Arc<Vec<QuotedNode>>,
    root: usize,
}

impl QuotedTerm {
    fn new(value: TermValue) -> Self {
        let nodes = RefCell::new(Vec::new());
        let root = value
            .try_fold_owned::<usize, std::convert::Infallible>(
                |leaf| {
                    let mut nodes = nodes.borrow_mut();
                    let index = nodes.len();
                    nodes.push(QuotedNode::Leaf(leaf));
                    Ok(index)
                },
                |s, p, o| {
                    let mut nodes = nodes.borrow_mut();
                    let index = nodes.len();
                    nodes.push(QuotedNode::Triple { s, p, o });
                    Ok(index)
                },
            )
            .unwrap_or_else(|never| match never {});
        Self {
            nodes: Arc::new(nodes.into_inner()),
            root,
        }
    }

    fn to_value(&self) -> TermValue {
        try_fold_nested::<_, _, _, std::convert::Infallible>(
            self.root,
            &mut (),
            |(), index| {
                Ok(match &self.nodes[index] {
                    QuotedNode::Leaf(value) => Nested::Leaf(value.clone()),
                    QuotedNode::Triple { s, p, o } => Nested::Triple(*s, *p, *o),
                })
            },
            |(), _, s, p, o| {
                Ok(TermValue::Triple {
                    s: TermBox::new(s),
                    p: TermBox::new(p),
                    o: TermBox::new(o),
                })
            },
        )
        .unwrap_or_else(|never| match never {})
    }

    fn component(&self, pick: impl FnOnce(usize, usize, usize) -> usize) -> Term {
        let QuotedNode::Triple { s, p, o } = self.nodes[self.root] else {
            unreachable!("a quoted term's root is a triple")
        };
        let root = pick(s, p, o);
        match &self.nodes[root] {
            QuotedNode::Triple { .. } => Term::from_inner(TermInner::Quoted(Self {
                nodes: Arc::clone(&self.nodes),
                root,
            })),
            QuotedNode::Leaf(value) => Term::from_canonical_rdf_term(
                term_value_into_rdf_term(value.clone(), BlankScopeMode::Keep)
                    .expect("a leaf has no triple predicate"),
            ),
        }
    }
}

impl PartialEq for QuotedTerm {
    fn eq(&self, other: &Self) -> bool {
        (self.root == other.root && Arc::ptr_eq(&self.nodes, &other.nodes))
            || self.to_value() == other.to_value()
    }
}
impl Eq for QuotedTerm {}
impl std::fmt::Debug for QuotedTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_value().fmt(f)
    }
}

/// An RDF/JS [Term](https://rdf.js.org/data-model-spec/#term-interface).
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct Term {
    pub(crate) inner: TermInner,
}

/// Expand the datatype through the native RDF 1.2 identity rule and lowercase
/// the language tag. Factory terms, parsed terms and reported datatypes share
/// the same identity, including directional literals.
pub(crate) fn canonicalize_literal(lit: RdfLiteral) -> RdfLiteral {
    let datatype = lit.datatype_iri().to_owned();
    RdfLiteral {
        datatype: Some(datatype),
        language: lit.language.as_deref().map(identity_fold),
        ..lit
    }
}

impl Term {
    pub(crate) fn from_inner(inner: TermInner) -> Self {
        Self { inner }
    }

    /// Build a literal [`Term`], canonicalizing the literal (see [`canonicalize_literal`]).
    pub(crate) fn literal(lit: RdfLiteral) -> Self {
        Self {
            inner: TermInner::Literal(canonicalize_literal(lit)),
        }
    }

    /// Build a [`Term`] from the engine's owned [`RdfTerm`].
    #[cfg(test)]
    pub(crate) fn from_rdf_term(t: &RdfTerm) -> Self {
        match t {
            RdfTerm::Iri(iri) => Self::from_inner(TermInner::Named(iri.clone())),
            RdfTerm::BlankNode(label) => Self::from_inner(TermInner::Blank(label.clone())),
            RdfTerm::Literal(lit) => Self::literal(lit.clone()),
            RdfTerm::Triple(_) => Self::from_inner(TermInner::Quoted(QuotedTerm::new(
                TermValue::from_rdf_term(t),
            ))),
        }
    }

    /// Build a [`Term`] from an already-canonical owned engine term without
    /// cloning its strings. `TermValue` egress is canonical by construction.
    pub(crate) fn from_canonical_rdf_term(t: RdfTerm) -> Self {
        match t {
            RdfTerm::Iri(iri) => Self::from_inner(TermInner::Named(iri)),
            RdfTerm::BlankNode(label) => Self::from_inner(TermInner::Blank(label)),
            RdfTerm::Literal(lit) => Self::literal(lit),
            RdfTerm::Triple(_) => unreachable!("only a leaf is converted to the owned model"),
        }
    }

    /// Lower this term to the engine's owned [`RdfTerm`].
    ///
    /// Errors for `Variable`/`DefaultGraph`, which are not part of the RDF data model
    /// and cannot occur as a subject/predicate/object/graph-name of a stored quad.
    #[cfg(test)]
    pub(crate) fn to_rdf_term(&self) -> Result<RdfTerm, String> {
        match &self.inner {
            TermInner::Named(iri) => Ok(RdfTerm::Iri(iri.clone())),
            TermInner::Blank(label) => Ok(RdfTerm::BlankNode(label.clone())),
            TermInner::Literal(lit) => Ok(RdfTerm::Literal(lit.clone())),
            TermInner::Quoted(value) => value.to_value().into_rdf_term().map_err(|e| e.to_string()),
            TermInner::Variable(_) => Err("a Variable is not a valid RDF term".to_owned()),
            TermInner::DefaultGraph => {
                Err("the DefaultGraph is not a valid subject/predicate/object".to_owned())
            }
        }
    }

    /// Build a binding term without materializing a recursively owned triple.
    /// Predicates are checked at every depth; scope merging is an explicit egress policy.
    pub(crate) fn from_value(mut value: TermValue, mode: BlankScopeMode) -> Result<Self, String> {
        if !matches!(value, TermValue::Triple { .. }) {
            return term_value_into_rdf_term(value, mode).map(Self::from_canonical_rdf_term);
        }
        let mut work = vec![&mut value];
        while let Some(node) = work.pop() {
            match node {
                TermValue::Triple { s, p, o } => {
                    if !matches!(**p, TermValue::Iri(_)) {
                        return Err("a triple-term predicate must be an IRI".to_owned());
                    }
                    work.push(&mut **s);
                    work.push(&mut **o);
                }
                TermValue::Blank { scope, .. } if mode == BlankScopeMode::Merge => {
                    *scope = purrdf::BlankScope::DEFAULT;
                }
                _ => {}
            }
        }
        Ok(Self::from_inner(TermInner::Quoted(QuotedTerm::new(value))))
    }

    /// Lower directly to the iterative core value, including the scope envelope inverse.
    pub(crate) fn to_value(&self) -> Result<TermValue, String> {
        let leaf = match &self.inner {
            TermInner::Quoted(value) => return Ok(value.to_value()),
            TermInner::Named(iri) => RdfTerm::Iri(iri.clone()),
            TermInner::Blank(label) => RdfTerm::BlankNode(label.clone()),
            TermInner::Literal(lit) => RdfTerm::Literal(lit.clone()),
            TermInner::Variable(_) => return Err("a Variable is not a valid RDF term".to_owned()),
            TermInner::DefaultGraph => {
                return Err("the DefaultGraph is not a valid subject/predicate/object".to_owned());
            }
        };
        Ok(TermValue::from_rdf_term(&leaf))
    }

    /// Construct a quoted triple using the core's stack-safe ownership.
    pub(crate) fn quoted(subject: &Self, predicate: String, object: &Self) -> Result<Self, String> {
        Ok(Self::from_inner(TermInner::Quoted(QuotedTerm::new(
            TermValue::Triple {
                s: TermBox::new(subject.to_value()?),
                p: TermBox::new(TermValue::Iri(predicate)),
                o: TermBox::new(object.to_value()?),
            },
        ))))
    }

    /// The effective datatype IRI of a literal per RDF 1.2: `rdf:dirLangString` when a
    /// base direction is present, `rdf:langString` for a plain language tag, the
    /// explicit datatype otherwise, falling back to `xsd:string`.
    fn literal_datatype_iri(lit: &RdfLiteral) -> String {
        lit.datatype_iri().to_owned()
    }
}

#[wasm_bindgen]
impl Term {
    /// `termType` — the RDF/JS discriminator.
    #[wasm_bindgen(getter = termType)]
    pub fn term_type(&self) -> String {
        match &self.inner {
            TermInner::Named(_) => "NamedNode",
            TermInner::Blank(_) => "BlankNode",
            TermInner::Literal(_) => "Literal",
            TermInner::Quoted(_) => "Quad",
            TermInner::Variable(_) => "Variable",
            TermInner::DefaultGraph => "DefaultGraph",
        }
        .to_owned()
    }

    /// `value` — the IRI, blank label, lexical form, or variable name. Empty for a
    /// quoted triple and the default graph (per RDF/JS).
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> String {
        match &self.inner {
            TermInner::Named(v) | TermInner::Blank(v) | TermInner::Variable(v) => v.clone(),
            TermInner::Literal(lit) => lit.lexical_form.clone(),
            TermInner::Quoted(_) | TermInner::DefaultGraph => String::new(),
        }
    }

    /// `language` — the literal's language tag, or `""` for a non-language-tagged term
    /// (RDF/JS uses the empty string, not `undefined`).
    #[wasm_bindgen(getter)]
    pub fn language(&self) -> String {
        match &self.inner {
            TermInner::Literal(lit) => lit.language.clone().unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// `direction` — the RDF-1.2 base direction (`"ltr"`/`"rtl"`), or `""` when absent.
    /// The deliberate extension to stock RDF/JS (`.goals`: overcome, don't inherit).
    #[wasm_bindgen(getter)]
    pub fn direction(&self) -> String {
        match &self.inner {
            TermInner::Literal(lit) => lit.direction.map(|d| d.as_str().to_owned()),
            _ => None,
        }
        .unwrap_or_default()
    }

    /// `datatype` — the literal's datatype as a `NamedNode`, or `undefined` for a
    /// non-literal.
    #[wasm_bindgen(getter)]
    pub fn datatype(&self) -> Option<Self> {
        match &self.inner {
            TermInner::Literal(lit) => Some(Self::from_inner(TermInner::Named(
                Self::literal_datatype_iri(lit),
            ))),
            _ => None,
        }
    }

    /// `subject` of a quoted-triple term (`termType: "Quad"`), else `undefined`.
    #[wasm_bindgen(getter)]
    pub fn subject(&self) -> Option<Self> {
        match &self.inner {
            TermInner::Quoted(value) => Some(value.component(|s, _, _| s)),
            _ => None,
        }
    }

    /// `predicate` of a quoted-triple term as a `NamedNode`, else `undefined`.
    #[wasm_bindgen(getter)]
    pub fn predicate(&self) -> Option<Self> {
        match &self.inner {
            TermInner::Quoted(value) => Some(value.component(|_, p, _| p)),
            _ => None,
        }
    }

    /// `object` of a quoted-triple term, else `undefined`.
    #[wasm_bindgen(getter)]
    pub fn object(&self) -> Option<Self> {
        match &self.inner {
            TermInner::Quoted(value) => Some(value.component(|_, _, o| o)),
            _ => None,
        }
    }

    /// `graph` of a quoted-triple term — always the default graph (a quoted triple has
    /// no graph slot), else `undefined`.
    #[wasm_bindgen(getter)]
    pub fn graph(&self) -> Option<Self> {
        match &self.inner {
            TermInner::Quoted(_) => Some(Self::from_inner(TermInner::DefaultGraph)),
            _ => None,
        }
    }

    /// Structural RDF/JS term equality.
    pub fn equals(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

/// An RDF/JS [Quad](https://rdf.js.org/data-model-spec/#quad-interface) — a statement
/// `(subject, predicate, object, graph)` with `termType: "Quad"`.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct Quad {
    pub(crate) subject: Term,
    pub(crate) predicate: Term,
    pub(crate) object: Term,
    pub(crate) graph: Term,
}

impl Quad {
    pub(crate) fn from_parts(subject: Term, predicate: Term, object: Term, graph: Term) -> Self {
        Self {
            subject,
            predicate,
            object,
            graph,
        }
    }
}

#[wasm_bindgen]
impl Quad {
    /// Always `"Quad"` (a Quad is itself an RDF/JS term).
    #[wasm_bindgen(getter = termType)]
    pub fn term_type(&self) -> String {
        "Quad".to_owned()
    }

    /// Empty for a Quad (per RDF/JS).
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> String {
        String::new()
    }

    /// The subject [`Term`] of the quad.
    #[wasm_bindgen(getter)]
    pub fn subject(&self) -> Term {
        self.subject.clone()
    }

    /// The predicate [`Term`] of the quad.
    #[wasm_bindgen(getter)]
    pub fn predicate(&self) -> Term {
        self.predicate.clone()
    }

    /// The object [`Term`] of the quad.
    #[wasm_bindgen(getter)]
    pub fn object(&self) -> Term {
        self.object.clone()
    }

    /// The graph [`Term`] of the quad (`DefaultGraph` when unnamed).
    #[wasm_bindgen(getter)]
    pub fn graph(&self) -> Term {
        self.graph.clone()
    }

    /// A quoted-triple [`Term`] (`termType: "Quad"`) viewing this quad's `(s, p, o)` —
    /// the RDF-1.2 wedge: pass the result as a subject/object to embed it.
    #[wasm_bindgen(js_name = asTerm)]
    pub fn as_term(&self) -> Result<Term, JsError> {
        let TermInner::Named(predicate) = &self.predicate.inner else {
            return Err(JsError::new("a quad predicate must be a NamedNode"));
        };
        Term::quoted(&self.subject, predicate.clone(), &self.object).map_err(|e| JsError::new(&e))
    }

    /// Structural RDF/JS quad equality.
    pub fn equals(&self, other: &Self) -> bool {
        self.subject.inner == other.subject.inner
            && self.predicate.inner == other.predicate.inner
            && self.object.inner == other.object.inner
            && self.graph.inner == other.graph.inner
    }
}

/// Parse a base direction string (`"ltr"`/`"rtl"`) into the engine enum.
///
/// Returns a plain `String` error (native-testable; a `JsError` panics off wasm).
pub(crate) fn parse_direction(direction: &str) -> Result<RdfTextDirection, String> {
    RdfTextDirection::from_str_token(direction).ok_or_else(|| {
        format!("invalid base direction {direction:?} (expected \"ltr\" or \"rtl\")")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_node_round_trips_through_rdf_term() {
        let t = Term::from_inner(TermInner::Named("https://e/s".to_owned()));
        assert_eq!(t.term_type(), "NamedNode");
        assert_eq!(t.value(), "https://e/s");
        assert_eq!(t.to_rdf_term().unwrap(), RdfTerm::iri("https://e/s"));
    }

    #[test]
    fn plain_literal_reports_xsd_string_datatype() {
        let t = Term::from_inner(TermInner::Literal(RdfLiteral::simple("hi")));
        assert_eq!(t.term_type(), "Literal");
        assert_eq!(t.value(), "hi");
        assert_eq!(t.language(), "");
        assert_eq!(t.datatype().unwrap().value(), XSD_STRING);
    }

    #[test]
    fn directional_literal_reports_dir_lang_string_and_direction() {
        let lit = RdfLiteral {
            lexical_form: "مرحبا".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        };
        let t = Term::from_inner(TermInner::Literal(lit));
        assert_eq!(t.language(), "ar");
        assert_eq!(t.direction(), "rtl");
        assert_eq!(t.datatype().unwrap().value(), RDF_DIR_LANG_STRING);
    }

    /// Factory lookup keys and datatype accessors agree with native RDF 1.2 identity.
    #[test]
    fn canonicalize_literal_stores_directional_datatype() {
        let lit = RdfLiteral {
            lexical_form: "مرحبا".to_owned(),
            datatype: None,
            language: Some("AR".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        };
        let canonical = canonicalize_literal(lit);
        assert_eq!(canonical.datatype.as_deref(), Some(RDF_DIR_LANG_STRING));
        // Language tag must be lowercased.
        assert_eq!(canonical.language.as_deref(), Some("ar"));
        // Direction is preserved (this is what distinguishes it in the identity key).
        assert_eq!(canonical.direction, Some(RdfTextDirection::Rtl));
        // Lexical form is unchanged.
        assert_eq!(canonical.lexical_form, "مرحبا");
        // The getter reports that same stored datatype.
        let t = Term::literal(RdfLiteral {
            lexical_form: "مرحبا".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        });
        assert_eq!(
            t.datatype().unwrap().value(),
            RDF_DIR_LANG_STRING,
            "the getter and stored identity agree"
        );
    }

    /// A factory-built directional literal and a separately-built identical literal
    /// must be structurally equal (the add/has in-memory path: both go through
    /// `canonicalize_literal`, so all fields agree on both sides).
    #[test]
    fn directional_literal_add_has_identity_is_consistent() {
        let make = || {
            let lit = RdfLiteral {
                lexical_form: "مرحبا".to_owned(),
                datatype: None,
                language: Some("ar".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            };
            Term::literal(lit)
        };

        let a = make();
        let b = make();
        // Structural equality via `equals` — exercises the same code path as the
        // in-memory `has`/`match` lookup (both use the canonicalized `TermInner`).
        assert!(
            a.equals(&b),
            "two independently-built directional literals must be structurally equal"
        );
        // The DERIVED datatype getter agrees between both copies.
        assert_eq!(a.datatype().unwrap().value(), RDF_DIR_LANG_STRING);
        assert_eq!(b.datatype().unwrap().value(), RDF_DIR_LANG_STRING);

        // A non-directional literal of the same text must NOT equal a directional one.
        let plain_lang = Term::literal(RdfLiteral {
            lexical_form: "مرحبا".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: None,
        });
        assert!(
            !a.equals(&plain_lang),
            "directional and non-directional literals with the same text/language must be distinct"
        );
    }

    /// `canonicalize_literal` for a plain (non-directional) language literal produces
    /// `rdf:langString` in both the stored field and the derived getter.
    #[test]
    fn canonicalize_literal_stores_lang_string_when_no_direction() {
        let lit = RdfLiteral {
            lexical_form: "hello".to_owned(),
            datatype: None,
            language: Some("en".to_owned()),
            direction: None,
        };
        let canonical = canonicalize_literal(lit);
        assert_eq!(
            canonical.datatype.as_deref(),
            Some(RDF_LANG_STRING),
            "no direction → stored datatype must remain rdf:langString"
        );
        assert_eq!(canonical.language.as_deref(), Some("en"));
        assert_eq!(canonical.direction, None);
        // And the getter agrees (no direction → langString).
        let t = Term::literal(RdfLiteral {
            lexical_form: "hello".to_owned(),
            datatype: None,
            language: Some("en".to_owned()),
            direction: None,
        });
        assert_eq!(t.datatype().unwrap().value(), RDF_LANG_STRING);
    }

    /// Hostile nested results remain values through getters and mutation boundaries.
    #[test]
    fn a_deep_cell_clones_compares_and_drops_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let mut value = TermValue::iri("https://e/end");
                for _ in 0..100_000 {
                    value = TermValue::Triple {
                        s: TermBox::new(TermValue::iri("https://e/s")),
                        p: TermBox::new(TermValue::iri("https://e/p")),
                        o: TermBox::new(value),
                    };
                }
                let term = Term::from_value(value, BlankScopeMode::Keep).expect("IRI predicates");
                let copy = term.clone();
                assert!(term.equals(&copy));
                let mut at = term.clone();
                let mut levels = 0;
                while let Some(next) = at.object() {
                    at = next;
                    levels += 1;
                }
                assert_eq!(levels, 100_000);
                assert_eq!(at.value(), "https://e/end");
                let object = term.object().expect("object");
                let quad = Quad::from_parts(
                    Term::from_inner(TermInner::Named("https://e/s".to_owned())),
                    Term::from_inner(TermInner::Named("https://e/p".to_owned())),
                    object,
                    Term::from_inner(TermInner::DefaultGraph),
                );
                let values =
                    crate::convert::quad_to_quad_values(&quad).expect("lowers without owned terms");
                let back = crate::convert::quad_values_to_quad(&values)
                    .expect("lifts without owned terms");
                assert!(quad.equals(&back));
                drop((term, copy, quad, values, back));
            })
            .expect("thread")
            .join()
            .expect("no stack overflow");
    }

    #[test]
    fn nested_scopes_and_predicates_keep_their_boundary_contract() {
        let nested = |scope, predicate| TermValue::Triple {
            s: TermBox::new(TermValue::Blank {
                label: "b".to_owned(),
                scope,
            }),
            p: TermBox::new(TermValue::iri("https://e/p")),
            o: TermBox::new(TermValue::Triple {
                s: TermBox::new(TermValue::Blank {
                    label: "b".to_owned(),
                    scope,
                }),
                p: TermBox::new(predicate),
                o: TermBox::new(TermValue::iri("https://e/o")),
            }),
        };
        let one = nested(purrdf::BlankScope(1), TermValue::iri("https://e/q"));
        let two = nested(purrdf::BlankScope(2), TermValue::iri("https://e/q"));
        let first = Term::from_value(one.clone(), BlankScopeMode::Keep).expect("valid");
        let second = Term::from_value(two.clone(), BlankScopeMode::Keep).expect("valid");
        assert!(!first.equals(&second));
        assert_ne!(
            first.object().unwrap().subject().unwrap().value(),
            second.object().unwrap().subject().unwrap().value()
        );
        assert_eq!(first.to_value().expect("round trip"), one);
        let merge_one = Term::from_value(one, BlankScopeMode::Merge).expect("valid");
        let merge_two = Term::from_value(two, BlankScopeMode::Merge).expect("valid");
        assert!(merge_one.equals(&merge_two));
        assert_eq!(merge_one.object().unwrap().subject().unwrap().value(), "b");
        assert!(
            Term::from_value(
                nested(
                    purrdf::BlankScope(1),
                    TermValue::from_rdf_term(&RdfTerm::literal(RdfLiteral::simple("p")))
                ),
                BlankScopeMode::Keep
            )
            .is_err()
        );
    }

    #[test]
    fn quoted_triple_term_exposes_components() {
        let triple = RdfTriple::new(
            RdfTerm::iri("https://e/s"),
            "https://e/p",
            RdfTerm::iri("https://e/o"),
        );
        let t = Term::from_rdf_term(&RdfTerm::Triple(Box::new(triple)));
        assert_eq!(t.term_type(), "Quad");
        assert_eq!(t.value(), "");
        assert_eq!(t.subject().unwrap().value(), "https://e/s");
        assert_eq!(t.predicate().unwrap().value(), "https://e/p");
        assert_eq!(t.predicate().unwrap().term_type(), "NamedNode");
        assert_eq!(t.object().unwrap().value(), "https://e/o");
        assert_eq!(t.graph().unwrap().term_type(), "DefaultGraph");
    }

    #[test]
    fn quad_getters_and_equality() {
        let q = Quad::from_parts(
            Term::from_inner(TermInner::Named("https://e/s".to_owned())),
            Term::from_inner(TermInner::Named("https://e/p".to_owned())),
            Term::from_inner(TermInner::Literal(RdfLiteral::simple("v"))),
            Term::from_inner(TermInner::DefaultGraph),
        );
        assert_eq!(q.term_type(), "Quad");
        assert_eq!(q.subject().value(), "https://e/s");
        assert_eq!(q.object().value(), "v");
        assert_eq!(q.graph().term_type(), "DefaultGraph");
        assert!(q.equals(&q.clone()));
    }

    #[test]
    fn variable_and_default_graph_are_not_rdf_terms() {
        assert!(
            Term::from_inner(TermInner::Variable("x".to_owned()))
                .to_rdf_term()
                .is_err()
        );
        assert!(
            Term::from_inner(TermInner::DefaultGraph)
                .to_rdf_term()
                .is_err()
        );
    }

    #[test]
    fn equals_is_structural() {
        let a = Term::from_inner(TermInner::Named("https://e/x".to_owned()));
        let b = Term::from_inner(TermInner::Named("https://e/x".to_owned()));
        let c = Term::from_inner(TermInner::Named("https://e/y".to_owned()));
        assert!(a.equals(&b));
        assert!(!a.equals(&c));
    }
}
