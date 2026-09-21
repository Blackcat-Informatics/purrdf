// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

use crate::term::{Quad, Term, TermInner, parse_direction};

/// Judge a literal's datatype/language/direction agreement — and, inside that,
/// the language tag's grammar — exactly as the other two language bindings do.
///
/// `purrdf-rdf-capi`'s `view_to_value` and `purrdf`'s Python `Literal.__new__`
/// both call [`RdfLiteral::validate_components`] at the constructor; this crate
/// was the one binding of the three that did not, so `factory.literal("x", "en
/// us")` returned a `Literal` and only failed later, at whatever `freeze()` or
/// serializer eventually met the tag. Naming the shared kernel call rather than
/// a local re-check is what keeps the three bindings from drifting apart.
///
/// The judgement is case-blind, so it gives the same verdict either side of the
/// lowercase fold `Term::literal`'s canonicalization applies afterwards.
///
/// Returns the reason as a `&'static str` rather than a [`JsError`], for the
/// same reason [`parse_direction`] does: a `JsError` cannot be constructed off
/// wasm, so a validator that built one would be untestable on the native gate.
/// The two `#[wasm_bindgen]` doors below wrap it at the boundary.
fn validate_literal(literal: &RdfLiteral) -> Result<(), &'static str> {
    RdfLiteral::validate_components(
        literal.datatype_iri(),
        literal.language.as_deref(),
        literal.direction,
    )
}

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
    /// `new DataFactory()` — a fresh factory with its blank-node counter at zero.
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

    /// `literal(value, language?)` → a plain (`xsd:string`) or language-tagged literal,
    /// **judged**. This is the JS door: `DataFactory.prototype.literal`.
    ///
    /// The RDF/JS spec's unified `literal(value, languageOrDatatype)` — where the second
    /// argument may be a string *or* a `NamedNode` — is presented by the TypeScript
    /// wrapper, which dispatches the `NamedNode` case to [`DataFactory::typed_literal`].
    /// (A `#[wasm_bindgen]`-exported type cannot be recovered from an untyped `JsValue`
    /// in Rust, so the polymorphism lives one layer out, in JS.) For base-direction
    /// literals (RDF 1.2) use [`DataFactory::directional_literal`].
    ///
    /// Its Rust name is not `literal`, and that is the point: `literal` is the
    /// published infallible Rust signature (see [`DataFactory::literal`]), which
    /// `purrdf-wasm` shipped in `rust-v2.0.2` and may not change. JS never sees a
    /// Rust name — `js_name = literal` is the whole binding — so the refusal can
    /// be added to the JS surface, which is where the parity gap was, without
    /// touching the Rust surface, which is where the semver guarantee is.
    ///
    /// # Errors
    ///
    /// Throws when `language` is not a language tag the RDF concrete syntaxes
    /// would have lexed — the same [`RdfLiteral::validate_components`] judgement
    /// the C ABI and the Python binding make at their own literal constructors,
    /// so all three language bindings admit one set of tags rather than two.
    /// Raising it *here* is the whole point: the JS stack trace then names the
    /// `literal()` call that chose the tag. Without it the tag rides along until
    /// a `Dataset` freeze or a serializer refuses it, at a call site that never
    /// saw the string.
    #[wasm_bindgen(js_name = literal)]
    pub fn checked_literal(
        &self,
        value: String,
        language: Option<String>,
    ) -> Result<Term, JsError> {
        let literal = match language {
            Some(language) => RdfLiteral::language_tagged(value, language),
            None => RdfLiteral::simple(value),
        };
        validate_literal(&literal).map_err(JsError::new)?;
        Ok(Term::literal(literal))
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
    ///
    /// # Errors
    ///
    /// Throws when `direction` is neither `"ltr"` nor `"rtl"`, and — as of the
    /// language-tag parity fix — when `language` is not a language tag. A base
    /// direction does not make a non-tag into a tag: `"x"@en us--ltr` is as
    /// unwritable as `"x"@en us`, so both halves of a directional literal are
    /// judged, not just the half that already had a checker.
    #[wasm_bindgen(js_name = directionalLiteral)]
    pub fn directional_literal(
        &self,
        value: String,
        language: String,
        direction: &str,
    ) -> Result<Term, JsError> {
        let direction = parse_direction(direction).map_err(|e| JsError::new(&e))?;
        let literal = RdfLiteral {
            lexical_form: value,
            datatype: None,
            language: Some(language),
            direction: Some(direction),
        };
        validate_literal(&literal).map_err(JsError::new)?;
        Ok(Term::literal(literal))
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
                ));
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

/// The Rust-only door. Not `#[wasm_bindgen]`-exported, so it adds nothing to the
/// JS surface — but it is `pub` on a `crate-type = ["cdylib", "rlib"]` crate that
/// `lib.rs` re-exports, so it is real published Rust API and carries the ordinary
/// semver guarantee.
impl DataFactory {
    /// `literal(value, language?)` → a plain (`xsd:string`) or language-tagged
    /// literal, built from `value` and `language` exactly as given.
    ///
    /// # This door does not ask the grammar
    ///
    /// `literal` does **not** judge `language`: `f.literal("x".into(), Some("en
    /// us".into()))` hands back a `Term` carrying that tag, and it will be refused
    /// later — by a `Dataset` freeze or by whatever serializer eventually meets it
    /// — at a call site that never saw the string. That is the same category as
    /// the infallible constructor [`RdfLiteral::language_tagged`] it wraps: the
    /// caller wrote the tag, so the caller owns it.
    ///
    /// It stays that way because this signature is published: `purrdf-wasm`
    /// `rust-v2.0.2` shipped `fn literal(&self, String, Option<String>) -> Term`,
    /// and no version bump is on the table. Turning it fallible would break every
    /// Rust caller that has one.
    ///
    /// For the judged door use [`DataFactory::checked_literal`], which is also
    /// what JS's `DataFactory.prototype.literal` is bound to — the JS surface is
    /// where the parity gap with the C ABI and the Python binding was, and JS
    /// never sees a Rust signature, so it gets the refusal while this one keeps
    /// its shape.
    #[must_use]
    pub fn literal(&self, value: String, language: Option<String>) -> Term {
        let literal = match language {
            Some(language) => RdfLiteral::language_tagged(value, language),
            None => RdfLiteral::simple(value),
        };
        Term::literal(literal)
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

        // The judged door agrees with the plain one on everything it admits.
        let checked = f
            .checked_literal("hello".to_owned(), Some("en".to_owned()))
            .expect("`en` is a language tag");
        assert!(checked.equals(&lang));
        assert!(
            f.checked_literal("hello".to_owned(), None)
                .expect("no tag to judge")
                .equals(&plain)
        );
    }

    /// The published Rust signature, pinned.
    ///
    /// `purrdf-wasm` `rust-v2.0.2` shipped `literal` as infallible — it is `pub`
    /// on an `rlib`, re-exported by `lib.rs`, so a Rust caller holds
    /// `let t: Term = f.literal(v, lang);` and no version bump is permitted. This
    /// test fails to COMPILE if the return type ever becomes a `Result` again,
    /// which is the only way to catch it: a fallible `literal` would still pass
    /// every behavioural assertion in this file.
    #[test]
    fn the_rust_literal_door_is_infallible_and_does_not_ask_the_grammar() {
        let f = DataFactory::new();
        let _: Term = f.literal("v".to_owned(), Some("en".to_owned()));

        // And it admits what it is handed, tag or not: the caller wrote the tag,
        // so the caller owns it, exactly as `RdfLiteral::language_tagged` does.
        let ungrammatical: Term = f.literal("v".to_owned(), Some("en us".to_owned()));
        assert_eq!(ungrammatical.term_type(), "Literal");
        assert_eq!(ungrammatical.language(), "en us");
        // The judged door, on the identical arguments, refuses it.
        assert!(
            validate_literal(&RdfLiteral::language_tagged("v", "en us")).is_err(),
            "the two doors must disagree here — that is what makes them two doors"
        );
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

    /// Three-binding parity, at the constructor: the C ABI (`purrdf-rdf-capi`'s
    /// `view_to_value`), Python (`Literal.__new__`) and this factory all refuse
    /// a non-tag where the caller spells it, rather than admitting it and
    /// letting a later `freeze()` carry the blame.
    ///
    /// The accept half is the load-bearing one — a JS caller building
    /// `@x-purrdf-afrikaans` or `@en-fr-jura` terms must be unaffected — so both
    /// lists are driven, and the directional door is driven with them because a
    /// base direction does not make a non-tag into a tag.
    #[test]
    fn the_factory_admits_the_same_language_tags_as_the_other_two_bindings() {
        let f = DataFactory::new();
        for tag in [
            "en",
            "en-US",
            "zh-Hans-CN",
            "de-CH-x-phonebk",
            "i-enochian",
            "x-purrdf-afrikaans",
            "x-gmeow-english",
            "en-fr-jura",
            "fr-be-fbcl",
            "abcdefgh",
            "en-x-cantbethislong",
        ] {
            assert!(
                f.checked_literal("v".to_owned(), Some((*tag).to_owned()))
                    .is_ok(),
                "{tag} is a tag real data carries and must still build a Literal"
            );
            assert!(
                f.directional_literal("v".to_owned(), (*tag).to_owned(), "ltr")
                    .is_ok(),
                "{tag} must still build a directional Literal"
            );
        }

        // The refusal half goes at `validate_literal`, the pure validator both
        // doors call: the doors' own error path builds a `JsError`, which panics
        // off wasm (see `parse_direction_rejects_bad_direction`). The node lane
        // in `js/tests` exercises the throw itself.
        for tag in [
            "en us",
            "1",
            "9-9",
            "123-456",
            "en-",
            "-",
            "!!!",
            "abcdefghi",
            "",
        ] {
            assert!(
                validate_literal(&RdfLiteral::language_tagged("v", tag)).is_err(),
                "{tag:?} must be refused where the caller named it"
            );
            assert!(
                validate_literal(&RdfLiteral {
                    lexical_form: "v".to_owned(),
                    datatype: None,
                    language: Some((*tag).to_owned()),
                    direction: Some(purrdf::RdfTextDirection::Ltr),
                })
                .is_err(),
                "{tag:?} must be refused with a direction too"
            );
        }

        // An absent tag is not a malformed one: the plain-literal door is
        // untouched.
        assert!(f.checked_literal("v".to_owned(), None).is_ok());
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
