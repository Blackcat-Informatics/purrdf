// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ONE definition of this crate's deterministic SplitMix64 test/bench
//! generator, following the same `#[doc(hidden)]`-module pattern other
//! crates in this workspace use to share code between a library and its
//! `tests/` and `benches/` targets: those are separate compilation units
//! that can only reach shared code through the library.
//!
//! `#[doc(hidden)]` at the crate root: shipped, because the targets that
//! consume it are built from the same crate, but not public API. No
//! shipping code path calls it.
//!
//! Two call patterns live here side by side because they are two DIFFERENT
//! streams from the same seed, and collapsing them into one would change
//! what every caller draws:
//!
//! * [`splitmix64_next`] — `state` is a plain incrementing counter; each
//!   draw advances it by the golden-ratio increment and re-mixes it from
//!   scratch.
//! * [`splitmix64_step`] — self-composed: `state` itself becomes the fully
//!   mixed value, so the next draw mixes the previous draw's output. Used by
//!   the `distance` bench fixture.
//!
//! Both share the same SplitMix64 finalizer; they differ only in what
//! they feed it.

/// The SplitMix64 finalizer, without the golden-ratio increment: two
/// xor-shift-multiply rounds and a final xor-shift.
const fn mix_rounds(z: u64) -> u64 {
    let mut z = z;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A SplitMix64 step over a plain counter: `state` is advanced by the
/// golden-ratio increment, then the advanced value is mixed and returned.
/// The next call advances the SAME raw counter again, not the mixed output.
#[doc(hidden)]
pub const fn splitmix64_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    mix_rounds(*state)
}

/// A self-composed SplitMix64 step: the golden-ratio increment is added to
/// `state` and the result mixed, and the caller is expected to feed the
/// returned value back in as the next `state` — so the counter itself is
/// the fully mixed value, not a raw increment.
#[doc(hidden)]
#[must_use]
pub const fn splitmix64_step(state: u64) -> u64 {
    mix_rounds(state.wrapping_add(0x9E37_79B9_7F4A_7C15))
}

/// Which positions of a generated triple term [`term_value`] may fill with which terms.
#[doc(hidden)]
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

/// A term value drawn from the [`splitmix64_next`] stream at `state`, holding at most
/// `budget` triple terms, its triple terms shaped by `shape`.
///
/// Every IRI is absolute and under `http://example.org/`; blank nodes come in two
/// labels over three scopes; literals are plain, language-tagged (with and without a
/// base direction) and typed, and one lexical form carries a quote, a backslash, a
/// newline and a non-ASCII letter, so a writer's escaping is exercised.
///
/// This draws recursively: a generated term nests at most `budget` levels, and the
/// generator is for tests of the walks, not one of them.
#[doc(hidden)]
pub fn term_value(state: &mut u64, budget: &mut usize, shape: TermShape) -> crate::TermValue {
    use crate::{BlankScope, RdfTextDirection, TermBox, TermValue};
    const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
    const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
    const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
    const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";
    let draw = |state: &mut u64, n: u64| splitmix64_next(state) % n;
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
            TermShape::Any | TermShape::IriPredicates => term_value(state, budget, shape),
        };
        let p = match shape {
            TermShape::Any => term_value(state, budget, shape),
            TermShape::IriPredicates | TermShape::WellFormed => {
                TermValue::iri(format!("http://example.org/p{}", draw(state, 2)))
            }
        };
        let o = term_value(state, budget, shape);
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
#[doc(hidden)]
#[must_use]
pub fn triple_chain(levels: usize) -> crate::TermValue {
    use crate::{TermBox, TermValue};
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
