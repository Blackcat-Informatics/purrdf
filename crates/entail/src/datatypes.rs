// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! XSD value spaces, walked once into the internal relations OWL 2 RL Table 8 reads.
//!
//! # The problem Table 8 poses
//!
//! Four of the five `dt-*` rules are written with NO triple premise. `dt-type2` ranges
//! over "each literal `lt` and each datatype `dt` supported in OWL 2 RL such that the data
//! value of `lt` is in the value space of `dt`"; `dt-eq` and `dt-diff` range over "all
//! literals" with the same or different data values; `dt-not-type` over each literal whose
//! datatype does not accept it. None of those is a join over triples — it is arithmetic
//! over XSD values — and none of them is finite as written, because "all literals" is an
//! infinite set.
//!
//! This module answers both problems at once. It ranges over the literals the DATASET
//! HOLDS, decides value-space membership with `purrdf-xsd`, and materializes the answer as
//! four internal relations the clauses in [`crate::calculus::dt`] join against:
//!
//! ```text
//! DT_VALUE(literal, datatype)      the literal's value lies in that datatype's value space
//! DT_EQUAL(lt1, lt2)               two literals denote ONE data value (REFLEXIVE, both ways)
//! DT_DIFFERENT(lt1, lt2)           two literals denote DIFFERENT values (one orientation)
//! DT_ILL_TYPED(literal, datatype)  the literal's OWN datatype does not accept its lexical
//! ```
//!
//! `DT_DIFFERENT` is the largest of the four and the only one that is quadratic in the
//! number of literals. That is inherent — an inequality over `n` values IS `n²` pairs — and
//! it is bounded by
//! [`MAX_STORED_FACTS`](purrdf_datalog::seminaive::MAX_STORED_FACTS) like every other fact,
//! so a dataset with more distinct valued literals than that admits is REFUSED with an
//! accurate report rather than closed incompletely.
//!
//! Restricting to the literals the dataset holds is the
//! [`Construct::DatatypeValueSpace`](crate::Construct::DatatypeValueSpace) boundary, which
//! every lane that fires these rules reports for every input. It is not a shortcut: a
//! forward chase cannot materialize an infinite premise, and a closure that claimed to have
//! is the failure a [`ReasoningReport`](crate::ReasoningReport) exists to prevent.
//!
//! # Membership is decided by the LEXICAL form, and that is a stated incompleteness
//!
//! `purrdf-xsd` maps a lexical form to a value; it does not expose "is this value in that
//! datatype's value space" as a predicate. So membership is decided as: the literal's
//! lexical form parses under the candidate datatype AND denotes the same value it denotes
//! under its own. That is exact whenever the two datatypes share a lexical space —
//! `"1"^^xsd:integer` is correctly found in `xsd:decimal`, `xsd:long`, `xsd:byte` and every
//! other integer facet whose range admits it — and it is INCOMPLETE where a value is in a
//! datatype's value space but its lexical form is not in that datatype's lexical space:
//! `"1.0"^^xsd:decimal` denotes an integer value and is not found in `xsd:integer`, because
//! `1.0` is not an `xsd:integer` lexical. The same boundary names it.
//!
//! # A datatype `purrdf-xsd` does not model is not JUDGED
//!
//! Three of the thirty-two datatypes OWL 2 RL supports — `rdf:PlainLiteral`,
//! `rdf:XMLLiteral` and `rdfs:Literal` — have no XSD value space and `purrdf-xsd` models
//! none of them. A literal carrying one is neither typed by `dt-type2` nor condemned by
//! `dt-not-type`: an unrecognized datatype is an absence of judgement, not a verdict, which
//! is the same treatment RDF 1.2 Semantics gives a datatype outside the recognized set.
//! The same holds for a language-tagged literal, whose datatype is `rdf:langString` or
//! `rdf:dirLangString` by C0.1 and whose value space is pairs, not XSD values.
//!
//! # Determinism
//!
//! The literals are gathered into a `BTreeMap` keyed by their store surface and walked in
//! that order, and the datatypes are walked in
//! [`SUPPORTED_DATATYPES`](crate::calculus::dt::SUPPORTED_DATATYPES) order, so the emitted
//! facts are a pure function of the dataset.

use std::collections::BTreeMap;

use purrdf_xsd::{XsdValue, parse_by_iri, value_eq};

use crate::calculus::dt::SUPPORTED_DATATYPES;
use crate::lists::{
    DT_DIFFERENT_RELATION, DT_EQUAL_RELATION, DT_ILL_TYPED_RELATION, DT_VALUE_RELATION,
    INTERNAL_GRAPH, InternalFact,
};

/// The literals one dataset's default graph holds, by store surface.
///
/// Built during the pass that seeds the fact store, so deciding Table 8 costs no second
/// traversal of the dataset.
#[derive(Debug, Default)]
pub(crate) struct LiteralIndex {
    /// Store surface → `(lexical form, datatype IRI)`, in surface order.
    literals: BTreeMap<String, (String, String)>,
}

impl LiteralIndex {
    /// Record one literal by the surface the store holds it under.
    ///
    /// A language-tagged literal is deliberately NOT recorded: its datatype is
    /// `rdf:langString` or `rdf:dirLangString`, whose value space is not an XSD one, so
    /// there is nothing here to decide about it. See the [module docs](self).
    pub(crate) fn observe(&mut self, surface: &str, lexical: &str, datatype: &str, tagged: bool) {
        if tagged {
            return;
        }
        self.literals
            .entry(surface.to_owned())
            .or_insert_with(|| (lexical.to_owned(), datatype.to_owned()));
    }

    /// Every datatype IRI the recorded literals carry, so [`crate::engine`] can record the
    /// value each one reads back as before evaluation starts.
    ///
    /// A datatype the pre-pass names in `DT_VALUE` or `DT_ILL_TYPED` is a TERM of the fact
    /// store, and the store's surface dictionary must be total over the terms it holds —
    /// including the datatype of an ill-typed literal, which need not be one of the
    /// thirty-two [`SUPPORTED_DATATYPES`] the program's own constants cover.
    pub(crate) fn datatypes(&self) -> impl Iterator<Item = &str> {
        self.literals
            .values()
            .map(|(_, datatype)| datatype.as_str())
    }

    /// Decide Table 8 over the recorded literals, as internal facts.
    ///
    /// Never fails: a lexical form `purrdf-xsd` refuses is a `DT_ILL_TYPED` row, which is
    /// `dt-not-type`'s premise, rather than an error raised here. The rule decides what an
    /// ill-typed literal MEANS; this pass only decides that it is one.
    pub(crate) fn materialize(&self) -> Vec<InternalFact> {
        let mut facts = Vec::new();
        // The value each literal denotes under its OWN datatype, or `None` when that
        // datatype is one `purrdf-xsd` does not model — an absence of judgement.
        let mut values: BTreeMap<&str, XsdValue> = BTreeMap::new();
        for (surface, (lexical, datatype)) in &self.literals {
            match parse_by_iri(lexical, datatype) {
                Ok(Some(value)) => {
                    let _ = values.insert(surface.as_str(), value);
                }
                // Recognized datatype, refused lexical: the literal is ILL TYPED.
                Err(_) => facts.push(InternalFact {
                    subject: surface.clone(),
                    predicate: DT_ILL_TYPED_RELATION,
                    object: iri_surface(datatype),
                    graph: INTERNAL_GRAPH.to_owned(),
                }),
                // Unrecognized datatype: no judgement either way.
                Ok(None) => {}
            }
        }

        // `dt-type2` — one row per (literal, supported datatype) whose value space holds
        // the literal's value.
        for (surface, value) in &values {
            let (lexical, _) = &self.literals[*surface];
            for candidate in SUPPORTED_DATATYPES {
                if in_value_space(lexical, candidate, value) {
                    facts.push(InternalFact {
                        subject: (*surface).to_owned(),
                        predicate: DT_VALUE_RELATION,
                        object: iri_surface(candidate),
                        graph: INTERNAL_GRAPH.to_owned(),
                    });
                }
            }
        }

        // `dt-eq` and `dt-diff` — the pair relations, in the two orientations each rule
        // actually needs.
        //
        // `DT_EQUAL` carries BOTH orientations and the reflexive pairs, because `dt-eq`
        // concludes `owl:sameAs` and the reflexive row is what makes that rule agree with
        // `eq-ref` about a literal and itself.
        //
        // `DT_DIFFERENT` carries ONE orientation, `left < right` in surface order.
        // `dt-diff`'s conclusion feeds `eq-diff1` alone, whose body is
        // `T(?x, owl:sameAs, ?y) ∧ T(?x, owl:differentFrom, ?y)`, and `eq-sym` closes
        // `owl:sameAs` under symmetry — so a clash on the pair `{a, b}` is found through
        // the `a < b` orientation whichever way round the equality was asserted. Emitting
        // the mirror would double the largest relation this crate materializes and find
        // nothing new; `the_asymmetric_dt_different_still_clashes_either_way` is the check.
        //
        // INCOMPARABLE values — `value_eq` is false and the two are not in one value space
        // at all — count as different, because "different data value" is what `dt-diff`
        // says and two values in different spaces are certainly not the same one.
        //
        // This is the quadratic pass, and it is quadratic because the RULE is: an
        // inequality over `n` values is `n²` pairs, and it cannot be expressed as a
        // negation here (see [`crate::calculus::dt`] and [`crate::lists`]). It is bounded
        // by `MAX_STORED_FACTS` like every other fact, so a dataset with a few hundred
        // distinct valued literals is REFUSED with an accurate report rather than
        // truncated.
        for (left, left_value) in &values {
            for (right, right_value) in &values {
                if value_eq(left_value, right_value) {
                    facts.push(InternalFact {
                        subject: (*left).to_owned(),
                        predicate: DT_EQUAL_RELATION,
                        object: (*right).to_owned(),
                        graph: INTERNAL_GRAPH.to_owned(),
                    });
                } else if left < right {
                    facts.push(InternalFact {
                        subject: (*left).to_owned(),
                        predicate: DT_DIFFERENT_RELATION,
                        object: (*right).to_owned(),
                        graph: INTERNAL_GRAPH.to_owned(),
                    });
                }
            }
        }
        facts
    }
}

/// The store surface of a constant IRI: `<iri>`.
///
/// A datatype names a TERM in `DT_VALUE` and `DT_ILL_TYPED`, and `dt-type2` writes that
/// term into the OBJECT of an `rdf:type` triple, so it has to be the same bytes
/// `crate::engine::surface_of` would render for the same IRI. Bracketing here rather than
/// carrying the bare IRI is what makes a pre-pass row and a clause constant the same term.
fn iri_surface(iri: &str) -> String {
    format!("<{iri}>")
}

/// Whether `value` — the value `lexical` denotes under its own datatype — also lies in
/// `candidate`'s value space, decided through `candidate`'s LEXICAL space.
///
/// See the [module docs](self) for why that is exact for the integer tower and incomplete
/// across lexical spaces, and why it is a boundary rather than a defect.
fn in_value_space(lexical: &str, candidate: &str, value: &XsdValue) -> bool {
    matches!(parse_by_iri(lexical, candidate), Ok(Some(other)) if value_eq(value, &other))
}

#[cfg(test)]
mod tests {
    use super::LiteralIndex;
    use crate::lists::{DT_EQUAL_RELATION, DT_ILL_TYPED_RELATION, DT_VALUE_RELATION};

    /// `xsd:integer`.
    const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
    /// `xsd:string`.
    const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
    /// `xsd:byte`.
    const XSD_BYTE: &str = "http://www.w3.org/2001/XMLSchema#byte";
    /// `xsd:decimal`.
    const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";

    /// An index over one typed literal, under the surface the store would hold it by.
    fn index_of(pairs: &[(&str, &str, &str)]) -> LiteralIndex {
        let mut index = LiteralIndex::default();
        for &(surface, lexical, datatype) in pairs {
            index.observe(surface, lexical, datatype, false);
        }
        index
    }

    /// The rows of one relation, as `(subject, object)` pairs.
    fn rows(index: &LiteralIndex, relation: &str) -> Vec<(String, String)> {
        index
            .materialize()
            .into_iter()
            .filter(|fact| fact.predicate == relation)
            .map(|fact| (fact.subject, fact.object))
            .collect()
    }

    /// `dt-type2` finds a value in EVERY supported datatype whose lexical space accepts
    /// it, which for `"1"^^xsd:integer` is the whole integer tower plus `xsd:decimal`.
    #[test]
    fn an_integer_is_found_in_the_integer_tower() {
        let index = index_of(&[("\"1\"^^<{XSD_INTEGER}>", "1", XSD_INTEGER)]);
        let found: Vec<String> = rows(&index, DT_VALUE_RELATION)
            .into_iter()
            .map(|(_, datatype)| datatype)
            .collect();
        for expected in [XSD_INTEGER, XSD_DECIMAL, XSD_BYTE] {
            let bracketed = format!("<{expected}>");
            assert!(found.contains(&bracketed), "{expected}: {found:?}");
        }
        assert!(
            !found.contains(&format!("<{XSD_STRING}>")),
            "an integer is not a string: {found:?}"
        );
        // `xsd:negativeInteger` does not admit 1, so the range facets really are checked
        // rather than the tower being accepted wholesale.
        assert!(
            !found.iter().any(|d| d.ends_with("#negativeInteger")),
            "{found:?}"
        );
    }

    /// `dt-eq` relates two lexically different literals with ONE value, and is reflexive.
    #[test]
    fn two_spellings_of_one_value_are_equal() {
        let index = index_of(&[("a", "1", XSD_INTEGER), ("b", "01", XSD_INTEGER)]);
        let equal = rows(&index, DT_EQUAL_RELATION);
        for pair in [("a", "a"), ("a", "b"), ("b", "a"), ("b", "b")] {
            assert!(
                equal.contains(&(pair.0.to_owned(), pair.1.to_owned())),
                "{pair:?} missing from {equal:?}"
            );
        }
        // Two DIFFERENT values land in the other relation, and only there.
        let index = index_of(&[("a", "1", XSD_INTEGER), ("b", "2", XSD_INTEGER)]);
        let equal = rows(&index, DT_EQUAL_RELATION);
        let different = rows(&index, super::DT_DIFFERENT_RELATION);
        assert!(
            !equal.contains(&("a".to_owned(), "b".to_owned())),
            "{equal:?}"
        );
        assert!(
            equal.contains(&("a".to_owned(), "a".to_owned())),
            "{equal:?}"
        );
        assert!(
            different.contains(&("a".to_owned(), "b".to_owned())),
            "{different:?}"
        );
        assert!(
            !different.contains(&("a".to_owned(), "a".to_owned())),
            "a literal is never different from itself: {different:?}"
        );
        assert!(
            !different.contains(&("b".to_owned(), "a".to_owned())),
            "DT_DIFFERENT carries ONE orientation; `eq-sym` supplies the other: \
             {different:?}"
        );
    }

    /// A lexical form its own datatype refuses is ILL TYPED — `dt-not-type`'s premise —
    /// and is judged by nothing else.
    #[test]
    fn an_ill_typed_literal_is_named_and_not_valued() {
        let index = index_of(&[("bad", "cat", XSD_INTEGER)]);
        assert_eq!(
            rows(&index, DT_ILL_TYPED_RELATION),
            vec![("bad".to_owned(), format!("<{XSD_INTEGER}>"))]
        );
        assert!(rows(&index, DT_VALUE_RELATION).is_empty());
        assert!(rows(&index, DT_EQUAL_RELATION).is_empty());
        assert!(rows(&index, super::DT_DIFFERENT_RELATION).is_empty());
    }

    /// A datatype `purrdf-xsd` does not model is not JUDGED: no typing, no clash.
    #[test]
    fn an_unmodelled_datatype_is_not_judged() {
        let index = index_of(&[(
            "x",
            "<p/>",
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral",
        )]);
        assert!(index.materialize().is_empty());
        // …and neither is a language-tagged literal, which is not recorded at all.
        let mut tagged = LiteralIndex::default();
        tagged.observe(
            "\"cat\"@en",
            "cat",
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString",
            true,
        );
        assert!(tagged.materialize().is_empty());
    }
}
