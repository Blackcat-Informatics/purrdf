// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RDF/JS [DataFactory](https://rdf.js.org/data-model-spec/#datafactory-interface).
//!
//! Maps 1:1 onto the engine's owned term model (and, through it, the
//! `purrdf::backend::TermFactory` interning seam used by the dataset). Extended
//! beyond stock RDF/JS with the RDF-1.2 wedge: [`DataFactory::quoted_triple`] (a
//! triple term usable as a subject/object) and [`DataFactory::directional_literal`]
//! (base-direction literals).

use core::cell::Cell;

use purrdf::{RdfLiteral, RdfTriple};
use wasm_bindgen::prelude::*;

use crate::term::{parse_direction, Quad, Term, TermInner};

/// An RDF/JS `DataFactory`. Stateless except for the auto-generated blank-node
/// counter (`blankNode()` with no argument mints a fresh label).
#[wasm_bindgen]
#[derive(Debug)]
pub struct DataFactory {
    /// Monotonic source of auto-generated blank-node labels. A plain counter (NOT an
    /// RNG): wasm32-unknown-unknown has no entropy backend, and deterministic labels
    /// are friendlier to test anyway. Per-factory, so two factories don't collide
    /// within one document any more than RDF/JS's own counter does.
    blank_counter: Cell<u64>,
}

impl Default for DataFactory {
    fn default() -> Self {
        Self {
            blank_counter: Cell::new(0),
        }
    }
}

#[wasm_bindgen]
impl DataFactory {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// `namedNode(value)` → a `NamedNode` term.
    #[wasm_bindgen(js_name = namedNode)]
    pub fn named_node(&self, value: String) -> Term {
        Term::from_inner(TermInner::Named(value))
    }

    /// `blankNode(value?)` → a `BlankNode` term; a fresh label is minted when omitted.
    #[wasm_bindgen(js_name = blankNode)]
    pub fn blank_node(&self, value: Option<String>) -> Term {
        let label = value.unwrap_or_else(|| {
            let n = self.blank_counter.get();
            self.blank_counter.set(n + 1);
            format!("b{n}")
        });
        Term::from_inner(TermInner::Blank(label))
    }

    /// `literal(value, language?)` → a plain (`xsd:string`) or language-tagged literal.
    ///
    /// The RDF/JS spec's unified `literal(value, languageOrDatatype)` — where the second
    /// argument may be a string *or* a `NamedNode` — is presented by the TypeScript
    /// wrapper, which dispatches the `NamedNode` case to [`DataFactory::typed_literal`].
    /// (A `#[wasm_bindgen]`-exported type cannot be recovered from an untyped `JsValue`
    /// in Rust, so the polymorphism lives one layer out, in JS.) For base-direction
    /// literals (RDF 1.2) use [`DataFactory::directional_literal`].
    #[wasm_bindgen(js_name = literal)]
    pub fn literal(&self, value: String, language: Option<String>) -> Term {
        let literal = match language {
            Some(language) => RdfLiteral::language_tagged(value, language),
            None => RdfLiteral::simple(value),
        };
        Term::literal(literal)
    }

    /// `typedLiteral(value, datatype)` → a datatyped literal. `datatype` must be a
    /// `NamedNode`. (The RDF/JS `literal(value, datatype)` form, surfaced by the TS
    /// wrapper.)
    #[wasm_bindgen(js_name = typedLiteral)]
    pub fn typed_literal(&self, value: String, datatype: &Term) -> Result<Term, JsError> {
        let iri = match &datatype.inner {
            TermInner::Named(iri) => iri.clone(),
            _ => return Err(JsError::new("a literal datatype must be a NamedNode")),
        };
        Ok(Term::literal(RdfLiteral::typed(value, iri)))
    }

    /// `directionalLiteral(value, language, direction)` → an RDF-1.2 base-direction
    /// literal (`direction` is `"ltr"` or `"rtl"`). The deliberate extension to stock
    /// RDF/JS — no incumbent library carries base direction.
    #[wasm_bindgen(js_name = directionalLiteral)]
    pub fn directional_literal(
        &self,
        value: String,
        language: String,
        direction: &str,
    ) -> Result<Term, JsError> {
        let direction = parse_direction(direction).map_err(|e| JsError::new(&e))?;
        Ok(Term::literal(RdfLiteral {
            lexical_form: value,
            datatype: None,
            language: Some(language),
            direction: Some(direction),
        }))
    }

    /// `variable(value)` → a `Variable` term.
    #[wasm_bindgen(js_name = variable)]
    pub fn variable(&self, value: String) -> Term {
        Term::from_inner(TermInner::Variable(value))
    }

    /// `defaultGraph()` → the `DefaultGraph` term.
    #[wasm_bindgen(js_name = defaultGraph)]
    pub fn default_graph(&self) -> Term {
        Term::from_inner(TermInner::DefaultGraph)
    }

    /// `quad(subject, predicate, object, graph?)` → a `Quad`. The graph defaults to the
    /// default graph. A quoted-triple term (from [`DataFactory::quoted_triple`]) may be
    /// passed as `subject` or `object` (the RDF-1.2 wedge).
    #[wasm_bindgen(js_name = quad)]
    pub fn quad(
        &self,
        subject: &Term,
        predicate: &Term,
        object: &Term,
        graph: Option<Term>,
    ) -> Quad {
        let graph = graph.unwrap_or_else(|| Term::from_inner(TermInner::DefaultGraph));
        Quad::from_parts(subject.clone(), predicate.clone(), object.clone(), graph)
    }

    /// `quotedTriple(subject, predicate, object)` → a quoted-triple `Term`
    /// (`termType: "Quad"`) — the RDF-1.2 wedge. Embed it by passing it as the
    /// `subject`/`object` of another quad.
    #[wasm_bindgen(js_name = quotedTriple)]
    pub fn quoted_triple(
        &self,
        subject: &Term,
        predicate: &Term,
        object: &Term,
    ) -> Result<Term, JsError> {
        let predicate_iri = match &predicate.inner {
            TermInner::Named(iri) => iri.clone(),
            _ => {
                return Err(JsError::new(
                    "a quoted triple predicate must be a NamedNode",
                ))
            }
        };
        let triple = RdfTriple::new(
            subject.to_rdf_term().map_err(|e| JsError::new(&e))?,
            predicate_iri,
            object.to_rdf_term().map_err(|e| JsError::new(&e))?,
        );
        Ok(Term::from_inner(TermInner::Quoted(Box::new(triple))))
    }

    /// `fromTerm(original)` → a copy of `original` (RDF/JS structural clone).
    #[wasm_bindgen(js_name = fromTerm)]
    pub fn from_term(&self, original: &Term) -> Term {
        original.clone()
    }

    /// `fromQuad(original)` → a copy of `original`.
    #[wasm_bindgen(js_name = fromQuad)]
    pub fn from_quad(&self, original: &Quad) -> Quad {
        original.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::XSD_STRING;

    #[test]
    fn named_node_and_default_graph() {
        let f = DataFactory::new();
        assert_eq!(
            f.named_node("https://e/s".to_owned()).term_type(),
            "NamedNode"
        );
        assert_eq!(f.default_graph().term_type(), "DefaultGraph");
    }

    #[test]
    fn auto_blank_labels_are_fresh_and_monotonic() {
        let f = DataFactory::new();
        let a = f.blank_node(None);
        let b = f.blank_node(None);
        assert_eq!(a.value(), "b0");
        assert_eq!(b.value(), "b1");
        assert!(!a.equals(&b));
        assert_eq!(f.blank_node(Some("named".to_owned())).value(), "named");
    }

    #[test]
    fn plain_and_language_literals() {
        let f = DataFactory::new();
        let plain = f.literal("hello".to_owned(), None);
        assert_eq!(plain.term_type(), "Literal");
        assert_eq!(plain.value(), "hello");
        assert_eq!(plain.language(), "");
        assert_eq!(plain.datatype().unwrap().value(), XSD_STRING);

        let lang = f.literal("hello".to_owned(), Some("en".to_owned()));
        assert_eq!(lang.language(), "en");
    }

    #[test]
    fn typed_literal_carries_datatype() {
        let f = DataFactory::new();
        let xsd_int = f.named_node("http://www.w3.org/2001/XMLSchema#integer".to_owned());
        let typed = f.typed_literal("42".to_owned(), &xsd_int).unwrap();
        assert_eq!(typed.value(), "42");
        assert_eq!(
            typed.datatype().unwrap().value(),
            "http://www.w3.org/2001/XMLSchema#integer"
        );
    }

    #[test]
    fn directional_literal_carries_the_wedge() {
        let f = DataFactory::new();
        let lit = f
            .directional_literal("שלום".to_owned(), "he".to_owned(), "rtl")
            .unwrap();
        assert_eq!(lit.term_type(), "Literal");
        assert_eq!(lit.language(), "he");
        assert_eq!(lit.direction(), "rtl");
    }

    #[test]
    fn parse_direction_rejects_bad_direction() {
        // The validation behind directional_literal. (The wasm method's error path
        // itself builds a JsError, which can't run off-wasm; the node test in Task 5
        // exercises that boundary. Here we test the pure validator.)
        assert!(parse_direction("ltr").is_ok());
        assert!(parse_direction("rtl").is_ok());
        assert!(parse_direction("sideways").is_err());
    }

    #[test]
    fn quad_defaults_to_the_default_graph() {
        let f = DataFactory::new();
        let s = f.named_node("https://e/s".to_owned());
        let p = f.named_node("https://e/p".to_owned());
        let o = f.named_node("https://e/o".to_owned());
        let q = f.quad(&s, &p, &o, None);
        assert_eq!(q.graph().term_type(), "DefaultGraph");
        // s/p/o were borrowed, not consumed — still usable.
        assert_eq!(s.value(), "https://e/s");
    }

    #[test]
    fn quoted_triple_is_a_quad_term_with_components() {
        let f = DataFactory::new();
        let s = f.named_node("https://e/s".to_owned());
        let p = f.named_node("https://e/p".to_owned());
        let o = f.named_node("https://e/o".to_owned());
        let qt = f.quoted_triple(&s, &p, &o).unwrap();
        assert_eq!(qt.term_type(), "Quad");
        assert_eq!(qt.subject().unwrap().value(), "https://e/s");
        // The wedge embeds: a quoted triple as a quad subject.
        let outer = f.quad(&qt, &p, &o, None);
        assert_eq!(outer.subject().term_type(), "Quad");
    }
}
