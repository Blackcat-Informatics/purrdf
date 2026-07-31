// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The CONCRETE DOMAIN of the OWL-Direct reasoner: data ranges and literal values.
//!
//! OWL 2 interprets an ontology over TWO domains — the object domain `Δ_I`, which
//! `owl:Thing` denotes and the completion graph's abstract nodes inhabit, and the data
//! domain `Δ_D`, whose elements are the values of literals. A data range is a subset of
//! `Δ_D`, and the question the tableau asks of one is always the same: **is it empty?**
//! A node that must inhabit an empty range has no interpretation, so the branch closes.
//!
//! That question is not a class-expression question and cannot be answered by the abstract
//! machinery: `"5"^^xsd:integer` and `"5.0"^^xsd:decimal` are two RDF TERMS denoting one
//! VALUE, and `xsd:minInclusive` is a statement about an order on values rather than about
//! any class. It is answered here by [`purrdf_xsd::range`], which decides emptiness,
//! membership and cardinality over the XSD value spaces and reports honestly when it cannot.
//!
//! # Three answers, and the asymmetry between them
//!
//! [`purrdf_xsd::range::satisfiability`] is three-valued, and the three are not
//! interchangeable:
//!
//! * `Empty` is a PROOF, so the tableau may close a branch on it;
//! * `Inhabited` exhibits a witness, so the tableau may not close;
//! * `Undecided` is "this decision procedure cannot say", and the tableau treats it exactly
//!   as `Inhabited` — no clash — because inventing an inconsistency is the one error a
//!   reasoner cannot recover from, while missing a clash merely admits more models.
//!
//! An `Undecided` range is therefore also a REPORTED boundary rather than a silent
//! weakening: [`DataRangeTable::exactly_decided`] is what the reverse mapping consults to
//! decide whether the run owes the caller a [`Construct::DataRange`](crate::Construct)
//! boundary, and it is exactly the predicate `purrdf-xsd` answers rather than an
//! approximation of it, so the two cannot drift.
//!
//! # Literal values are not opaque terms
//!
//! Each literal that reaches the knowledge base carries two consequences into the
//! completion graph:
//!
//! 1. the singleton data range `{value}`, asserted on the literal's own node, so a `∀p.DR`
//!    over a data property is CHECKED against the value the ontology actually stated;
//! 2. a VALUE CLASS, which is what makes two literals distinct elements of `Δ_D`. The data
//!    domain admits no unique-name freedom — two names denote one element exactly when they
//!    denote one value — so `"1"^^xsd:integer` and `"01"^^xsd:integer` share a class and can
//!    never be counted twice, while `"1"^^xsd:integer` and `"2"^^xsd:integer` are in
//!    different classes and are therefore distinct without any `owl:differentFrom`.
//!
//! A literal whose lexical form is not in its datatype's lexical space denotes no value at
//! all, and OWL 2 makes an ontology asserting one inconsistent. It is given the EMPTY data
//! range, which is how that inconsistency reaches the tableau through the same rule as
//! every other empty range rather than through a special case.
//!
//! # Determinism
//!
//! Range ids are assigned in first-seen (parse) order; value classes are assigned by
//! ascending literal term id; every index is a `BTreeMap` or an insertion-ordered `Vec`.
//! Nothing is read out of a hash map and nothing consults a clock.

use std::collections::BTreeMap;

use purrdf_core::TermValue;
use purrdf_xsd::range::{Cardinality, DataRange, Satisfiability};
use purrdf_xsd::{XsdDatatype, XsdError, XsdValue};

/// One data range, with everything the tableau asks of it decided once at parse time.
///
/// The decisions are cached rather than recomputed because the clash rule visits every node
/// on every saturation round, and a range's emptiness is a function of the range alone.
struct Decided {
    /// The range itself, as `purrdf-xsd` models it.
    range: DataRange,
    /// Whether the range is PROVABLY empty on its own.
    empty: bool,
    /// Whether every question `purrdf-xsd` answers about this range is answered exactly, so
    /// that no boolean combination of it with other exactly-decided ranges can be undecided.
    exact: bool,
}

/// The data ranges one knowledge base holds, by dense id.
///
/// A `Concept::Data(id)` leaf carries an id into this table. Ids are assigned in first-seen
/// parse order, so the table — and hence every concept id derived from it — is reproducible
/// run to run.
#[derive(Default)]
pub(crate) struct DataRangeTable {
    /// id → the decided range, in first-seen order.
    ranges: Vec<Decided>,
}

impl DataRangeTable {
    /// Whether the knowledge base holds no data range at all.
    ///
    /// The tableau's concrete-domain clash rule is skipped wholesale when this holds, so an
    /// ontology without a data range pays nothing for the concrete domain.
    pub(crate) fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Record `range`, deciding it once, and return its dense id.
    pub(crate) fn intern(&mut self, range: DataRange) -> u32 {
        let empty = matches!(
            purrdf_xsd::range::satisfiability(&range),
            Satisfiability::Empty
        );
        let exact = purrdf_xsd::range::is_exactly_decided(&range);
        let id = u32::try_from(self.ranges.len()).expect("data range count fits u32");
        self.ranges.push(Decided {
            range,
            empty,
            exact,
        });
        id
    }

    /// Whether the range with this id is provably empty on its own.
    ///
    /// Read by the consequence-based classifier, which turns an empty range into the axiom
    /// `Data(r) ⊑ ⊥` so a class forced into one is derived empty rather than only refuted.
    pub(crate) fn is_range_empty(&self, id: u32) -> bool {
        self.ranges[id as usize].empty
    }

    /// Whether EVERY range in the table is exactly decided.
    ///
    /// The reverse mapping raises its data-range boundary exactly when this is false.
    /// Answering per-table rather than per-range is what makes the answer sound under
    /// combination: the tableau conjoins the ranges on one node, and a conjunction of
    /// exactly-decided ranges is itself exactly decided.
    pub(crate) fn exactly_decided(&self) -> bool {
        self.ranges.iter().all(|decided| decided.exact)
    }

    /// How many ranges the table holds. Read by the module's own tests, which assert that an
    /// ontology stating no data range and holding no literal interns none.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Whether the conjunction of the `positive` ranges with the complements of the
    /// `negative` ones is PROVABLY empty.
    ///
    /// This is the whole content of the concrete-domain clash: a node labelled
    /// `Data(r₁) … Data(rₘ) ¬Data(s₁) … ¬Data(sₖ)` denotes a value in
    /// `r₁ ∩ … ∩ rₘ ∩ ¬s₁ ∩ … ∩ ¬sₖ`, and if that set is empty there is no such value.
    /// `Undecided` answers `false` — the branch stays open — because the unsound direction
    /// is claiming emptiness.
    pub(crate) fn conjunction_is_empty(&self, positive: &[u32], negative: &[u32]) -> bool {
        matches!(
            purrdf_xsd::range::satisfiability(&self.conjunction(positive, negative)),
            Satisfiability::Empty
        )
    }

    /// Whether the conjunction of `positive` provably holds FEWER than `n` distinct values.
    ///
    /// `≥n r.DR` needs `n` pairwise-distinct values of `DR`, and the data domain admits no
    /// unique-name freedom, so a range with fewer than `n` values refutes the restriction
    /// outright. A cardinality the decision procedure cannot pin down, or one that is only
    /// bounded from below, answers `false`.
    pub(crate) fn provably_fewer_than(&self, positive: &[u32], n: u32) -> bool {
        if n == 0 {
            return false;
        }
        match purrdf_xsd::range::cardinality(&self.conjunction(positive, &[])) {
            Cardinality::Exactly(held) => held < u64::from(n),
            Cardinality::AtLeast(_) | Cardinality::Unbounded | Cardinality::Undecided => false,
        }
    }

    /// The range `positive₁ ∩ … ∩ ¬negative₁ ∩ …`, as one [`DataRange`].
    fn conjunction(&self, positive: &[u32], negative: &[u32]) -> DataRange {
        let mut operands: Vec<DataRange> = Vec::with_capacity(positive.len() + negative.len());
        for &id in positive {
            operands.push(self.ranges[id as usize].range.clone());
        }
        for &id in negative {
            operands.push(DataRange::Not(Box::new(
                self.ranges[id as usize].range.clone(),
            )));
        }
        match operands.len() {
            // An empty conjunction is the whole data domain, which is inhabited.
            0 => DataRange::Any,
            1 => operands.pop().expect("one operand"),
            _ => DataRange::And(operands),
        }
    }
}

/// What one literal term denotes, as the concrete domain sees it.
pub(crate) enum LiteralValue {
    /// The literal denotes this XSD value.
    Value(XsdValue),
    /// The literal is language-tagged, so its value is the `(lexical form, language,
    /// direction)` triple the TERM already is: `rdf:langString`'s value space is exactly
    /// those triples, and the IR lower-cases the language tag as part of a literal's
    /// identity, so two distinct term ids here denote two distinct values. Value identity is
    /// term identity, and no XSD value is needed to decide it.
    TermIdentified,
    /// The lexical form is not in the datatype's lexical space, so the literal denotes NO
    /// value and an ontology that asserts it is inconsistent.
    IllTyped,
    /// The datatype is outside the value space `purrdf-xsd` models, or the value is beyond
    /// the domain it can represent, so nothing is known about what the literal denotes —
    /// including whether it denotes the same value as any other literal.
    Unmodelled,
}

/// What `literal` denotes, or `None` when the term is not a literal at all.
pub(crate) fn literal_value(literal: &TermValue) -> Option<LiteralValue> {
    let TermValue::Literal {
        lexical_form,
        datatype,
        language,
        ..
    } = literal
    else {
        return None;
    };
    if language.is_some() {
        return Some(LiteralValue::TermIdentified);
    }
    let Some(kind) = XsdDatatype::from_iri(datatype) else {
        return Some(LiteralValue::Unmodelled);
    };
    Some(match purrdf_xsd::parse(lexical_form, kind) {
        Ok(value) => LiteralValue::Value(value),
        // A lexical form the datatype's own lexical space rejects is ill-typed. One that is
        // merely beyond this crate's representable domain is NOT: the ontology is
        // well-formed and the value simply cannot be examined here.
        Err(XsdError::InvalidLexical { .. }) => LiteralValue::IllTyped,
        Err(
            XsdError::OutOfRange { .. }
            | XsdError::DivisionByZero { .. }
            | XsdError::TypeMismatch { .. },
        ) => LiteralValue::Unmodelled,
    })
}

/// The VALUE-space partition of the literals a knowledge base holds.
pub(crate) struct LiteralClasses {
    /// Literal term id → its value class. Two ids share a class exactly when they denote one
    /// value; two ids with different classes denote different values. A literal whose value
    /// cannot be examined is absent, which is what keeps the missing knowledge from being
    /// read as either identity or distinctness.
    pub(crate) class_of: BTreeMap<u32, u32>,
    /// Whether some literal's datatype is outside the modelled value space, so that neither
    /// its well-typedness nor its identity with any other literal was decided.
    pub(crate) any_unmodelled: bool,
}

/// Partition `literals` — `(term id, what it denotes)`, ascending by term id — into value
/// classes.
///
/// The scan is quadratic in the literals of ONE value-space family, because value-space
/// identity is not a hash on a lexical form: `"5"^^xsd:integer` and `"5.0"^^xsd:decimal` are
/// one value with two distinct canonical lexicals. Bucketing by family first keeps the
/// comparison count quadratic per family rather than over the whole literal table, and
/// values of different families are never one value, so a bucket boundary can never separate
/// two literals that should share a class. The buckets are an insertion-ordered `Vec` rather
/// than a map because there are at most as many of them as there are value spaces.
pub(crate) fn literal_classes(literals: &[(u32, LiteralValue)]) -> LiteralClasses {
    let mut out = LiteralClasses {
        class_of: BTreeMap::new(),
        any_unmodelled: false,
    };
    let mut buckets: Vec<(&'static str, Vec<(XsdValue, u32)>)> = Vec::new();
    let mut next_class = 0u32;
    for (term, value) in literals {
        match value {
            LiteralValue::Value(value) => {
                let family = family_of(value);
                let position = buckets.iter().position(|(name, _)| *name == family);
                let index = match position {
                    Some(index) => index,
                    None => {
                        buckets.push((family, Vec::new()));
                        buckets.len() - 1
                    }
                };
                let bucket = &mut buckets[index].1;
                let existing = bucket
                    .iter()
                    .find(|(candidate, _)| purrdf_xsd::range::same_value(candidate, value))
                    .map(|&(_, class)| class);
                let class = existing.unwrap_or_else(|| {
                    let class = next_class;
                    bucket.push((value.clone(), class));
                    class
                });
                if existing.is_none() {
                    next_class += 1;
                }
                out.class_of.insert(*term, class);
            }
            LiteralValue::TermIdentified => {
                out.class_of.insert(*term, next_class);
                next_class += 1;
            }
            LiteralValue::Unmodelled => out.any_unmodelled = true,
            // An ill-typed literal denotes nothing, so it belongs to no class: giving it one
            // would make it distinct from — or equal to — a value it does not have. The empty
            // data range the caller gives it is what closes the branch instead.
            LiteralValue::IllTyped => {}
        }
    }
    out
}

/// A stable name for the VALUE SPACE a value inhabits, used only to bucket the
/// value-identity scan. Values from different families are never one value, so a bucket
/// boundary can never merge two classes that should be one.
fn family_of(value: &XsdValue) -> &'static str {
    match value {
        XsdValue::Integer { .. } | XsdValue::Decimal(_) => "decimal",
        XsdValue::Float(_) => "float",
        XsdValue::Double(_) => "double",
        XsdValue::Boolean(_) => "boolean",
        XsdValue::String(_) => "string",
        XsdValue::DateTime(_) => "dateTime",
        XsdValue::Date(_) => "date",
        XsdValue::Time(_) => "time",
        XsdValue::Duration(_) => "duration",
        XsdValue::Gregorian(g) => g.datatype().iri(),
        XsdValue::Binary { datatype, .. } => datatype.iri(),
        // `XsdValue` is `#[non_exhaustive]`. A value space this arm has not learned to name
        // shares one bucket with every other, which costs comparisons and never costs
        // correctness: bucketing only decides which candidates `same_value` is asked about, and
        // asking about too many is safe where asking about too few would separate two literals
        // that denote one value.
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermId, TermValue};

    use crate::owl_dl::Kb;
    use crate::report::Construct;
    use crate::vocab::{
        OWL_DATATYPECOMPLEMENTOF, OWL_FUNCTIONALPROPERTY, OWL_INTERSECTIONOF,
        OWL_MINQUALIFIEDCARDINALITY, OWL_ONDATARANGE, OWL_ONDATATYPE, OWL_ONEOF, OWL_ONPROPERTY,
        OWL_RESTRICTION, OWL_SOMEVALUESFROM, OWL_WITHRESTRICTIONS, RDF_FIRST, RDF_NIL, RDF_REST,
        RDF_TYPE, RDFS_DATATYPE, RDFS_LITERAL, RDFS_RANGE, RDFS_SUBCLASSOF, XSD_ANYURI,
        XSD_DECIMAL, XSD_FLOAT, XSD_INTEGER, XSD_MAXINCLUSIVE, XSD_MININCLUSIVE,
        XSD_NONNEGATIVEINTEGER, XSD_PATTERN, XSD_STRING,
    };
    use crate::{Completeness, EntailError, QTriple, materialize_dl_reported};

    /// A fixture class.
    const EX_C: &str = "http://example.org/C";
    /// A fixture individual.
    const EX_A: &str = "http://example.org/a";
    /// A fixture data property.
    const EX_P: &str = "http://example.org/p";

    /// A tiny OWL-in-RDF fixture writer. Terms are `example.org`: PurRDF mints no vocabulary.
    struct Fixture {
        builder: RdfDatasetBuilder,
        cells: usize,
        blanks: usize,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                builder: RdfDatasetBuilder::new(),
                cells: 0,
                blanks: 0,
            }
        }

        fn iri(&mut self, iri: &str) -> TermId {
            self.builder.intern_iri(iri)
        }

        fn blank(&mut self) -> TermId {
            self.blanks += 1;
            self.builder
                .intern_blank(&format!("n{}", self.blanks), BlankScope::DEFAULT)
        }

        fn literal(&mut self, lexical: &str, datatype: &str) -> TermId {
            crate::interner::intern_into(
                &mut self.builder,
                &TermValue::typed_literal(lexical, datatype),
            )
        }

        fn lang_literal(&mut self, lexical: &str, language: &str) -> TermId {
            crate::interner::intern_into(
                &mut self.builder,
                &TermValue::lang_literal(lexical, language),
            )
        }

        fn quad(&mut self, s: TermId, p: TermId, o: TermId) {
            self.builder.push_quad(s, p, o, None);
        }

        /// Write `members` as an RDF collection, returning its head.
        fn list(&mut self, members: &[TermId]) -> TermId {
            let first = self.iri(RDF_FIRST);
            let rest = self.iri(RDF_REST);
            let mut head = self.iri(RDF_NIL);
            for &member in members.iter().rev() {
                self.cells += 1;
                let cell = self
                    .builder
                    .intern_blank(&format!("cell{}", self.cells), BlankScope::DEFAULT);
                self.quad(cell, first, member);
                self.quad(cell, rest, head);
                head = cell;
            }
            head
        }

        /// A `[ rdf:type rdfs:Datatype ; owl:onDatatype <base> ; owl:withRestrictions (…) ]`
        /// node over `facets`, each a `(facet IRI, lexical form, datatype IRI)`.
        fn restricted_datatype(&mut self, base: &str, facets: &[(&str, &str, &str)]) -> TermId {
            let node = self.blank();
            let ty = self.iri(RDF_TYPE);
            let datatype = self.iri(RDFS_DATATYPE);
            let on_datatype = self.iri(OWL_ONDATATYPE);
            let with_restrictions = self.iri(OWL_WITHRESTRICTIONS);
            let base = self.iri(base);
            self.quad(node, ty, datatype);
            self.quad(node, on_datatype, base);
            let mut cells = Vec::with_capacity(facets.len());
            for &(facet, lexical, facet_datatype) in facets {
                let cell = self.blank();
                let facet = self.iri(facet);
                let value = self.literal(lexical, facet_datatype);
                self.quad(cell, facet, value);
                cells.push(cell);
            }
            let head = self.list(&cells);
            self.quad(node, with_restrictions, head);
            node
        }

        /// `ex:C rdfs:subClassOf [ owl:onProperty ex:p ; owl:someValuesFrom <range> ]` plus
        /// `ex:a rdf:type ex:C` — a class whose every member must inhabit `range`.
        fn some_values_from(&mut self, range: TermId) {
            let ty = self.iri(RDF_TYPE);
            let sub_class = self.iri(RDFS_SUBCLASSOF);
            let restriction_class = self.iri(OWL_RESTRICTION);
            let on_property = self.iri(OWL_ONPROPERTY);
            let some_values = self.iri(OWL_SOMEVALUESFROM);
            let class = self.iri(EX_C);
            let individual = self.iri(EX_A);
            let property = self.iri(EX_P);
            let node = self.blank();
            self.quad(class, sub_class, node);
            self.quad(node, ty, restriction_class);
            self.quad(node, on_property, property);
            self.quad(node, some_values, range);
            self.quad(individual, ty, class);
        }

        fn freeze(self) -> Arc<RdfDataset> {
            self.builder.freeze().expect("the fixture freezes")
        }
    }

    /// Run the public OWL-Direct seam and report whether the ontology is consistent, plus the
    /// constructs the run could not fully handle.
    fn run(dataset: &RdfDataset) -> (bool, Vec<Construct>) {
        match materialize_dl_reported(dataset, &[] as &[QTriple]) {
            Ok((_, report)) => (
                true,
                report
                    .boundaries()
                    .iter()
                    .map(|boundary| boundary.construct())
                    .collect(),
            ),
            Err(EntailError::Unsatisfiable) => (false, Vec::new()),
            Err(other) => panic!("the run must decide, not fail: {other}"),
        }
    }

    /// AN EMPTY DATATYPE RANGE IS AN INCONSISTENCY, not a boundary.
    ///
    /// `xsd:integer` with `minInclusive 5` and `maxInclusive 3` holds no value at all, so a
    /// class whose every member must have a `p`-value in it has no members — and an
    /// individual asserted into that class makes the ontology unsatisfiable. This is the
    /// answer a layer with no concrete-domain procedure cannot give: without evaluating the
    /// facets over the integers there is nothing to notice.
    #[test]
    fn an_empty_facet_range_makes_the_ontology_inconsistent() {
        let mut f = Fixture::new();
        let range = f.restricted_datatype(
            XSD_INTEGER,
            &[
                (XSD_MININCLUSIVE, "5", XSD_INTEGER),
                (XSD_MAXINCLUSIVE, "3", XSD_INTEGER),
            ],
        );
        f.some_values_from(range);
        let (consistent, boundaries) = run(&f.freeze());
        assert!(
            !consistent,
            "a class whose members must inhabit xsd:integer[5..3] is unsatisfiable"
        );
        assert!(boundaries.is_empty(), "{boundaries:?}");
    }

    /// …and a SATISFIABLE facet stays satisfiable. Over-eager emptiness would be unsoundness
    /// in the other direction, and it is the more dangerous one: an invented inconsistency
    /// entails every answer.
    #[test]
    fn a_satisfiable_facet_range_stays_satisfiable() {
        for (low, high) in [("3", "5"), ("3", "3"), ("-2", "0")] {
            let mut f = Fixture::new();
            let range = f.restricted_datatype(
                XSD_INTEGER,
                &[
                    (XSD_MININCLUSIVE, low, XSD_INTEGER),
                    (XSD_MAXINCLUSIVE, high, XSD_INTEGER),
                ],
            );
            f.some_values_from(range);
            let (consistent, boundaries) = run(&f.freeze());
            assert!(consistent, "xsd:integer[{low}..{high}] holds a value");
            assert!(boundaries.is_empty(), "{boundaries:?}");
        }
    }

    /// The complement of the WHOLE data domain is empty, so `owl:datatypeComplementOf
    /// rdfs:Literal` is the range no value inhabits. Complementing against the data domain
    /// rather than against the base datatype is what makes this the right answer.
    #[test]
    fn a_complement_of_the_data_domain_is_empty() {
        let mut f = Fixture::new();
        let ty = f.iri(RDF_TYPE);
        let datatype = f.iri(RDFS_DATATYPE);
        let complement_of = f.iri(OWL_DATATYPECOMPLEMENTOF);
        let literal = f.iri(RDFS_LITERAL);
        let range = f.blank();
        f.quad(range, ty, datatype);
        f.quad(range, complement_of, literal);
        f.some_values_from(range);
        let (consistent, boundaries) = run(&f.freeze());
        assert!(!consistent, "Δ_D ∖ Δ_D is empty");
        assert!(boundaries.is_empty(), "{boundaries:?}");
    }

    /// A datatype intersected with its own complement is empty too — the same emptiness
    /// reached through the range ALGEBRA rather than through one atom.
    #[test]
    fn a_datatype_intersected_with_its_own_complement_is_empty() {
        let mut f = Fixture::new();
        let ty = f.iri(RDF_TYPE);
        let datatype = f.iri(RDFS_DATATYPE);
        let complement_of = f.iri(OWL_DATATYPECOMPLEMENTOF);
        let intersection_of = f.iri(OWL_INTERSECTIONOF);
        let integer = f.iri(XSD_INTEGER);
        let complement = f.blank();
        f.quad(complement, ty, datatype);
        f.quad(complement, complement_of, integer);
        let head = f.list(&[integer, complement]);
        let range = f.blank();
        f.quad(range, ty, datatype);
        f.quad(range, intersection_of, head);
        f.some_values_from(range);
        let (consistent, boundaries) = run(&f.freeze());
        assert!(!consistent, "xsd:integer ⊓ ¬xsd:integer is empty");
        assert!(boundaries.is_empty(), "{boundaries:?}");
    }

    /// …and a datatype intersected with a DIFFERENT datatype of the same value space is not.
    #[test]
    fn a_datatype_intersected_with_a_wider_one_of_its_own_space_is_inhabited() {
        let mut f = Fixture::new();
        let ty = f.iri(RDF_TYPE);
        let datatype = f.iri(RDFS_DATATYPE);
        let intersection_of = f.iri(OWL_INTERSECTIONOF);
        let integer = f.iri(XSD_INTEGER);
        let decimal = f.iri(XSD_DECIMAL);
        let head = f.list(&[integer, decimal]);
        let range = f.blank();
        f.quad(range, ty, datatype);
        f.quad(range, intersection_of, head);
        f.some_values_from(range);
        let (consistent, boundaries) = run(&f.freeze());
        assert!(consistent, "every integer is a decimal");
        assert!(boundaries.is_empty(), "{boundaries:?}");
    }

    /// `ex:a ex:p <lexical>^^<datatype>` for each pair, with `ex:p` functional.
    fn functional_property_over(values: &[(&str, &str)]) -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let ty = f.iri(RDF_TYPE);
        let functional = f.iri(OWL_FUNCTIONALPROPERTY);
        let individual = f.iri(EX_A);
        let property = f.iri(EX_P);
        f.quad(property, ty, functional);
        for &(lexical, datatype) in values {
            let value = f.literal(lexical, datatype);
            f.quad(individual, property, value);
        }
        f.freeze()
    }

    /// ONE VALUE, TWO LEXICAL FORMS. `"1"^^xsd:integer` and `"01"^^xsd:integer` are distinct
    /// RDF terms denoting one element of the data domain, so a FUNCTIONAL data property may
    /// hold both — while two terms denoting two values may not. Without value-space identity
    /// both halves are wrong: the first invents an inconsistency and the second misses one.
    #[test]
    fn a_functional_data_property_counts_values_rather_than_lexical_forms() {
        let (consistent, boundaries) = run(&functional_property_over(&[
            ("1", XSD_INTEGER),
            ("01", XSD_INTEGER),
        ]));
        assert!(
            consistent,
            "\"1\"^^xsd:integer and \"01\"^^xsd:integer are ONE value"
        );
        assert!(boundaries.is_empty(), "{boundaries:?}");

        let (consistent, _) = run(&functional_property_over(&[
            ("1", XSD_INTEGER),
            ("2", XSD_INTEGER),
        ]));
        assert!(
            !consistent,
            "a functional property cannot hold two different values"
        );
    }

    /// A functional data property over two LANGUAGE-TAGGED literals.
    ///
    /// `rdf:langString`'s value space is the `(lexical form, language, direction)` triples,
    /// so `"hello"@en` and `"goodbye"@en` are two values and a functional property may not
    /// hold both. This case is separated ONLY by value-class distinctness: unlike two
    /// numeric literals, whose incompatible ranges make the merged node's constraint set
    /// unsatisfiable and clash through the concrete domain, two `rdf:langString` values sit
    /// in ONE range, so nothing but their distinctness forces them apart. Disabling that
    /// check left every other test in this workspace passing, which is what this one is for.
    #[test]
    fn a_functional_data_property_separates_two_language_tagged_values() {
        let mut f = Fixture::new();
        let ty = f.iri(RDF_TYPE);
        let functional = f.iri(OWL_FUNCTIONALPROPERTY);
        let individual = f.iri(EX_A);
        let property = f.iri(EX_P);
        f.quad(property, ty, functional);
        let hello = f.lang_literal("hello", "en");
        let goodbye = f.lang_literal("goodbye", "en");
        f.quad(individual, property, hello);
        f.quad(individual, property, goodbye);
        let (consistent, _) = run(&f.freeze());
        assert!(
            !consistent,
            "\"hello\"@en and \"goodbye\"@en are two rdf:langString values, so a \
             functional property cannot hold both"
        );

        // The same property over ONE value twice is consistent — the assertion above must
        // fail for distinctness, not because language-tagged literals clash on sight.
        let mut g = Fixture::new();
        let ty = g.iri(RDF_TYPE);
        let functional = g.iri(OWL_FUNCTIONALPROPERTY);
        let individual = g.iri(EX_A);
        let property = g.iri(EX_P);
        g.quad(property, ty, functional);
        let once = g.lang_literal("hello", "en");
        g.quad(individual, property, once);
        let (consistent, _) = run(&g.freeze());
        assert!(consistent, "one value held once is consistent");
    }

    /// The numeric tower, exactly where the specification puts it. OWL 2's datatype map nests
    /// the integers inside the decimals, so `"5"^^xsd:integer` and `"5.0"^^xsd:decimal` are one
    /// value; it makes `xsd:float`, `xsd:double` and `owl:real` — and hence `xsd:decimal` —
    /// PAIRWISE DISJOINT, so `"5"^^xsd:integer` and `"5"^^xsd:float` are two.
    #[test]
    fn the_numeric_tower_identifies_only_what_the_datatype_map_nests() {
        let (consistent, _) = run(&functional_property_over(&[
            ("5", XSD_INTEGER),
            ("5.0", XSD_DECIMAL),
        ]));
        assert!(
            consistent,
            "the integer value space is a subset of the decimal one"
        );

        let (consistent, _) = run(&functional_property_over(&[
            ("5", XSD_INTEGER),
            ("5", XSD_FLOAT),
        ]));
        assert!(
            !consistent,
            "the xsd:float value space is disjoint from the decimal one, so these are two \
             values and a functional property cannot hold both"
        );

        let (consistent, _) = run(&functional_property_over(&[
            ("5", XSD_INTEGER),
            ("5", XSD_STRING),
        ]));
        assert!(
            !consistent,
            "a number and a string are two values of two value spaces"
        );
    }

    /// AN ASSERTED LITERAL IS CHECKED AGAINST THE RANGE ITS PROPERTY DECLARES.
    ///
    /// `rdfs:range` over a data property is the axiom `⊤ ⊑ ∀p.DR`, and the `∀`-rule pushes the
    /// range onto the literal's own node — where it meets the singleton range the literal's
    /// value is. `7` and `xsd:integer[≤3]` have an empty intersection, so the branch closes.
    #[test]
    fn a_literal_outside_its_property_range_is_inconsistent() {
        let build = |lexical: &str| {
            let mut f = Fixture::new();
            let range_of = f.iri(RDFS_RANGE);
            let property = f.iri(EX_P);
            let individual = f.iri(EX_A);
            let range = f.restricted_datatype(XSD_INTEGER, &[(XSD_MAXINCLUSIVE, "3", XSD_INTEGER)]);
            f.quad(property, range_of, range);
            let value = f.literal(lexical, XSD_INTEGER);
            f.quad(individual, property, value);
            f.freeze()
        };
        let (consistent, boundaries) = run(&build("7"));
        assert!(!consistent, "7 is not in xsd:integer[≤3]");
        assert!(boundaries.is_empty(), "{boundaries:?}");
        let (consistent, _) = run(&build("2"));
        assert!(consistent, "2 is in xsd:integer[≤3]");
    }

    /// A literal whose lexical form is not in its datatype's LEXICAL space denotes no value,
    /// and OWL 2 makes an ontology that asserts one inconsistent. It reaches the tableau as
    /// the EMPTY data range, through the same clash as every other empty range.
    #[test]
    fn an_ill_typed_literal_makes_the_ontology_inconsistent() {
        let mut f = Fixture::new();
        let property = f.iri(EX_P);
        let individual = f.iri(EX_A);
        let value = f.literal("not-an-integer", XSD_INTEGER);
        f.quad(individual, property, value);
        let (consistent, _) = run(&f.freeze());
        assert!(
            !consistent,
            "\"not-an-integer\"^^xsd:integer denotes nothing"
        );
    }

    /// `≥n p.DR` demands `n` pairwise-distinct VALUES of `DR`, and the data domain has no
    /// unique-name freedom to invent them from. An enumeration of one value therefore refutes
    /// `≥2` — a counting question no per-node emptiness check can see.
    #[test]
    fn a_min_cardinality_over_a_smaller_enumeration_is_inconsistent() {
        let build = |demanded: &str| {
            let mut f = Fixture::new();
            let ty = f.iri(RDF_TYPE);
            let sub_class = f.iri(RDFS_SUBCLASSOF);
            let restriction_class = f.iri(OWL_RESTRICTION);
            let on_property = f.iri(OWL_ONPROPERTY);
            let min_qcard = f.iri(OWL_MINQUALIFIEDCARDINALITY);
            let on_data_range = f.iri(OWL_ONDATARANGE);
            let datatype = f.iri(RDFS_DATATYPE);
            let one_of = f.iri(OWL_ONEOF);
            let class = f.iri(EX_C);
            let property = f.iri(EX_P);
            let individual = f.iri(EX_A);
            let one = f.literal("1", XSD_INTEGER);
            let head = f.list(&[one]);
            let range = f.blank();
            f.quad(range, ty, datatype);
            f.quad(range, one_of, head);
            let node = f.blank();
            let count = f.literal(demanded, XSD_NONNEGATIVEINTEGER);
            f.quad(class, sub_class, node);
            f.quad(node, ty, restriction_class);
            f.quad(node, on_property, property);
            f.quad(node, min_qcard, count);
            f.quad(node, on_data_range, range);
            f.quad(individual, ty, class);
            f.freeze()
        };
        let (consistent, boundaries) = run(&build("2"));
        assert!(!consistent, "{{1}} has no two distinct values");
        assert!(boundaries.is_empty(), "{boundaries:?}");
        let (consistent, _) = run(&build("1"));
        assert!(consistent, "{{1}} has one value");
    }

    /// THE RESIDUE IS NAMED, NOT SILENT. An `xsd:pattern` facet is a regular-language
    /// question this layer does not decide, so the range is UNDECIDED: no clash is invented,
    /// and the boundary says which part of the concrete domain the run did not close.
    #[test]
    fn a_pattern_facet_is_reported_rather_than_guessed() {
        let mut f = Fixture::new();
        let range = f.restricted_datatype(XSD_STRING, &[(XSD_PATTERN, "[a-z]+", XSD_STRING)]);
        f.some_values_from(range);
        let (consistent, boundaries) = run(&f.freeze());
        assert!(
            consistent,
            "an undecided range may never close a branch: that would invent an inconsistency"
        );
        assert_eq!(boundaries, vec![Construct::DataRange]);
    }

    /// A datatype outside the modelled value space is the second residue class. It may
    /// OVERLAP a modelled space — every `xsd:decimal` value is an `owl:real` value — so it
    /// cannot be assumed disjoint either, and the honest answer is the boundary.
    #[test]
    fn an_unmodelled_datatype_is_reported_rather_than_guessed() {
        let mut f = Fixture::new();
        let property = f.iri(EX_P);
        let individual = f.iri(EX_A);
        let value = f.literal("http://example.org/x", XSD_ANYURI);
        f.quad(individual, property, value);
        let (consistent, boundaries) = run(&f.freeze());
        assert!(consistent);
        assert_eq!(boundaries, vec![Construct::DataRange]);
    }

    /// A decidable data range leaves the certificate FLATLY exact, so the boundary is evidence
    /// about an input rather than a standing disclaimer about the concrete domain.
    #[test]
    fn a_decidable_data_range_leaves_the_run_exact() {
        let mut f = Fixture::new();
        let range = f.restricted_datatype(XSD_INTEGER, &[(XSD_MININCLUSIVE, "0", XSD_INTEGER)]);
        f.some_values_from(range);
        let (_, report) =
            materialize_dl_reported(&f.freeze(), &[] as &[QTriple]).expect("consistent");
        assert_eq!(report.completeness(), Completeness::Exact);
    }

    /// The knowledge base's data-range table is EMPTY for an ontology that states no data
    /// range and holds no literal — which is what makes the concrete-domain rules free for
    /// every such ontology rather than a cost every run pays.
    #[test]
    fn an_ontology_without_a_literal_holds_no_data_range() {
        let mut f = Fixture::new();
        let ty = f.iri(RDF_TYPE);
        let class = f.iri(EX_C);
        let individual = f.iri(EX_A);
        f.quad(individual, ty, class);
        let kb = Kb::from_dataset(&f.freeze()).expect("parse");
        assert_eq!(kb.data_ranges.len(), 0, "no data range was interned");
        assert!(kb.literal_class.is_empty(), "no literal was classed");
        assert!(kb.boundaries().is_empty(), "{:?}", kb.boundaries());
    }
}
