// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The owned, dataset-independent triple view both sides of a match are read through.
//!
//! # Why an owned view at all
//!
//! [`TermRef`](purrdf_core::TermRef) borrows into ONE dataset's term table and carries a
//! literal's datatype as a [`TermId`](purrdf_core::TermId) local to it, so two `TermRef`s
//! from two independently-parsed datasets cannot be compared: the same IRI is a different
//! id in each. A conclusion-directed service compares exactly that — a conclusion graph
//! the caller parsed against a closure this crate built — so both sides are resolved into
//! [`TermValue`], whose every coordinate is by value and therefore means the same thing in
//! every dataset.
//!
//! # Why only the default graph
//!
//! [`crate::materialize`] closes the default graph against itself and each named graph
//! against the union of itself and the default graph, landing each conclusion in the graph
//! that produced it. An entailment question asked of a DATASET therefore has to name which
//! graph answers it, and this service names the default graph: it is the graph SPARQL's
//! entailment regimes call the active graph by default, it is where an RDF/XML or Turtle
//! document's whole content lands, and it is where the chase's own conclusions about that
//! content land. Reading a named graph as part of the answer would let a conclusion be
//! "entailed" by a graph the question never mentioned.

use std::fmt::Write as _;

use purrdf_core::{RdfDataset, RdfTextDirection, TermValue};

/// A `(subject, predicate, object)` triple of owned, dataset-independent terms.
pub(crate) type Triple = [TermValue; 3];

/// Every default-graph triple of `ds`, as owned terms, in the dataset's frozen quad order.
///
/// Frozen order is a function of the dataset alone, so two runs over one dataset produce
/// the same vector — which is what keeps the index below, and every diagnostic built from
/// it, reproducible.
pub(crate) fn default_graph_triples(ds: &RdfDataset) -> Vec<Triple> {
    ds.quads()
        .filter(|quad| quad.g.is_none())
        .map(|quad| {
            [
                ds.term_value(quad.s),
                ds.term_value(quad.p),
                ds.term_value(quad.o),
            ]
        })
        .collect()
}

/// The datatype a literal's own surface shape already implies, per RDF 1.2 C0.1.
///
/// A literal whose datatype is this one is fully described by its lexical form, language
/// tag and direction, so [`show`] can leave the datatype off without losing anything that
/// distinguishes it from another literal.
fn implicit_datatype(
    language: Option<&String>,
    direction: Option<RdfTextDirection>,
) -> &'static str {
    match (language, direction) {
        (Some(_), Some(_)) => crate::vocab::RDF_DIRLANGSTRING,
        (Some(_), None) => crate::vocab::RDF_LANGSTRING,
        (None, _) => crate::vocab::XSD_STRING,
    }
}

/// Render a term the way a diagnostic prints it.
///
/// Not a serialization: it is deliberately lossy because its only consumer is a human
/// reading a [`MissReason`](super::homomorphism::MissReason). Anything that must
/// round-trip goes through a codec, not through this.
///
/// # What a literal keeps
///
/// The lexical form always, then the language tag as `@tag` and the base direction as
/// `--ltr`/`--rtl` when present, and finally `^^<datatype>` — but ONLY when the datatype
/// is not the one the rest of the rendering already implies ([`implicit_datatype`]:
/// `xsd:string` with no language tag, `rdf:langString` with one, `rdf:dirLangString` with
/// a tag and a direction). That is the whole rule, and it is what makes the lossiness
/// safe: two literals that render the same text ARE the same literal, so a `miss` line
/// never shows `"1"^^xsd:integer` and `"1"^^xsd:string` as one term, while a graph of
/// plain strings is not padded with `^^<…#string>` on every line.
pub(crate) fn show(term: &TermValue) -> String {
    match term {
        TermValue::Iri(iri) => format!("<{iri}>"),
        TermValue::Blank { label, scope } => format!("_:{label}#{}", scope.ordinal()),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            let mut out = format!("{lexical_form:?}");
            if let Some(lang) = language {
                let _ = write!(out, "@{lang}");
            }
            if let Some(dir) = direction {
                let _ = write!(out, "--{}", dir.as_str());
            }
            if datatype != implicit_datatype(language.as_ref(), *direction) {
                let _ = write!(out, "^^<{datatype}>");
            }
            out
        }
        TermValue::Triple { s, p, o } => {
            format!("<<{} {} {}>>", show(s), show(p), show(o))
        }
    }
}

#[cfg(test)]
mod tests {
    use purrdf_core::{RdfTextDirection, TermValue};

    use super::show;

    /// A literal with an explicit datatype and no language tag.
    fn typed(lexical_form: &str, datatype: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: lexical_form.to_owned(),
            datatype: datatype.to_owned(),
            language: None,
            direction: None,
        }
    }

    /// A language-tagged literal, optionally carrying a base direction.
    fn tagged(
        lexical_form: &str,
        language: &str,
        direction: Option<RdfTextDirection>,
    ) -> TermValue {
        TermValue::Literal {
            lexical_form: lexical_form.to_owned(),
            datatype: if direction.is_some() {
                crate::vocab::RDF_DIRLANGSTRING.to_owned()
            } else {
                crate::vocab::RDF_LANGSTRING.to_owned()
            },
            language: Some(language.to_owned()),
            direction,
        }
    }

    /// The exception the doc promises: a datatype that is NOT the implicit one is the only
    /// thing distinguishing these two literals, so it has to survive into the rendering.
    #[test]
    fn a_distinguishing_datatype_is_rendered() {
        let integer = typed("1", "http://www.w3.org/2001/XMLSchema#integer");
        let string = typed("1", crate::vocab::XSD_STRING);
        assert_eq!(
            show(&integer),
            "\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
        assert_eq!(show(&string), "\"1\"");
        assert_ne!(show(&integer), show(&string));
    }

    /// The other half of the rule: a plain literal is NOT padded with `^^<…#string>`.
    #[test]
    fn an_implicit_datatype_is_dropped() {
        assert_eq!(show(&typed("foo", crate::vocab::XSD_STRING)), "\"foo\"");
        assert_eq!(show(&tagged("foo", "en", None)), "\"foo\"@en");
    }

    /// Direction is part of a literal's identity, so two directions must not render alike.
    #[test]
    fn direction_is_rendered_and_its_datatype_is_not() {
        let ltr = tagged("foo", "en", Some(RdfTextDirection::Ltr));
        let rtl = tagged("foo", "en", Some(RdfTextDirection::Rtl));
        assert_eq!(show(&ltr), "\"foo\"@en--ltr");
        assert_eq!(show(&rtl), "\"foo\"@en--rtl");
        assert_ne!(show(&ltr), show(&tagged("foo", "en", None)));
    }

    /// The rule applies inside a triple term too, since `show` recurses through itself.
    #[test]
    fn a_triple_term_renders_its_object_by_the_same_rule() {
        let term = TermValue::Triple {
            s: Box::new(TermValue::iri("http://example.org/s")),
            p: Box::new(TermValue::iri("http://example.org/p")),
            o: Box::new(typed("1", "http://www.w3.org/2001/XMLSchema#integer")),
        };
        assert_eq!(
            show(&term),
            "<<<http://example.org/s> <http://example.org/p> \
             \"1\"^^<http://www.w3.org/2001/XMLSchema#integer>>>"
        );
    }
}
