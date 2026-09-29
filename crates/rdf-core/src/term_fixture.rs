// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generated RDF terms for the traversal regression tests of every crate that
//! walks a [`TermValue`]: arbitrarily shaped terms from a caller's
//! deterministic stream, and triple terms nested to any depth.
//!
//! This is test support, not API: it is hidden from the documentation and
//! carries no stability promise. It lives in the library so that each crate's
//! tests import one definition rather than compiling a copy of it; the
//! pseudo-random stream is the caller's, so the library takes no test-only
//! dependency.

use crate::{BlankScope, RdfTextDirection, TermBox, TermValue};

/// Which positions of a generated triple term [`term_value`] may fill with which terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TermShape {
    /// Any term in any position, a non-IRI predicate included — a walk that refuses one
    /// meets it.
    Any,
    /// An IRI predicate, and any term as subject and object.
    IriPredicates,
    /// RDF 1.2 as a frozen dataset admits it: an IRI or blank-node subject, an IRI
    /// predicate, and triple terms nested only in the object.
    WellFormed,
}

/// A term value drawn from the caller's deterministic stream — `next` advances
/// `state` and returns the next draw — holding at most `budget` triple terms, its
/// triple terms shaped by `shape`.
///
/// Every IRI is absolute and under `http://example.org/`; blank nodes come in two
/// labels over three scopes; literals are plain, language-tagged (with and without a
/// base direction) and typed, and one lexical form carries a quote, a backslash, a
/// newline and a non-ASCII letter, so a writer's escaping is exercised.
///
/// This draws recursively: a generated term nests at most `budget` levels, and the
/// generator is for tests of the walks, not one of them.
pub fn term_value(
    state: &mut u64,
    next: fn(&mut u64) -> u64,
    budget: &mut usize,
    shape: TermShape,
) -> TermValue {
    use purrdf_iri::vocab::rdf::DIR_LANG_STRING as RDF_DIR_LANG_STRING;
    use purrdf_iri::vocab::rdf::LANG_STRING as RDF_LANG_STRING;
    use purrdf_xsd::datatype::XSD_INTEGER;
    use purrdf_xsd::datatype::XSD_STRING;
    let draw = |state: &mut u64, n: u64| next(state) % n;
    if *budget > 0 && draw(state, 3) == 0 {
        *budget -= 1;
        let s = match shape {
            TermShape::WellFormed if draw(state, 2) == 0 => {
                TermValue::iri(format!("http://example.org/i{}", draw(state, 3)))
            }
            TermShape::WellFormed => TermValue::Blank {
                label: format!("b{}", draw(state, 2)),
                scope: BlankScope(u32::try_from(draw(state, 3)).unwrap_or(0)),
            },
            TermShape::Any | TermShape::IriPredicates => term_value(state, next, budget, shape),
        };
        let p = match shape {
            TermShape::Any => term_value(state, next, budget, shape),
            TermShape::IriPredicates | TermShape::WellFormed => {
                TermValue::iri(format!("http://example.org/p{}", draw(state, 2)))
            }
        };
        let o = term_value(state, next, budget, shape);
        return TermValue::Triple {
            s: TermBox::new(s),
            p: TermBox::new(p),
            o: TermBox::new(o),
        };
    }
    let literal =
        |lexical: &str, datatype: &str, language: Option<&str>, direction| TermValue::Literal {
            lexical_form: lexical.to_owned(),
            datatype: datatype.to_owned(),
            language: language.map(str::to_owned),
            direction,
        };
    match draw(state, 8) {
        0 | 1 => TermValue::iri(format!("http://example.org/i{}", draw(state, 3))),
        2 => TermValue::Blank {
            label: format!("b{}", draw(state, 2)),
            scope: BlankScope(u32::try_from(draw(state, 3)).unwrap_or(0)),
        },
        3 => literal("plain", XSD_STRING, None, None),
        4 => literal("tagged", RDF_LANG_STRING, Some("en"), None),
        5 => literal(
            "directed",
            RDF_DIR_LANG_STRING,
            Some("ar"),
            Some(RdfTextDirection::Rtl),
        ),
        6 => literal("7", XSD_INTEGER, None, None),
        _ => literal("q\"b\\n\n\u{e9}", XSD_STRING, None, None),
    }
}

/// A term value nesting `levels` triple terms, each holding the next in its object
/// slot under the subject `http://example.org/s` and the predicate
/// `http://example.org/p`, around the innermost object `http://example.org/o`. Built
/// by a loop, so any depth is cheap to make.
#[must_use]
pub fn triple_chain(levels: usize) -> TermValue {
    let mut term = TermValue::iri("http://example.org/o");
    for _ in 0..levels {
        term = TermValue::Triple {
            s: TermBox::new(TermValue::iri("http://example.org/s")),
            p: TermBox::new(TermValue::iri("http://example.org/p")),
            o: TermBox::new(term),
        };
    }
    term
}
