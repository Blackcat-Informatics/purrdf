// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RDF collections, walked once into an INTERNAL relation the clause language can join.
//!
//! # The problem the rule tables pose
//!
//! Several OWL 2 RL rules are written with a meta-notation no clause language has:
//! `LIST[?x, ?c1, …, ?cn]`. `scm-int`, `scm-uni`, `prp-adp`, `cax-adc`, `prp-key`,
//! `prp-spo2` and most of Table 6 quantify over an RDF collection of unbounded length,
//! which is not a conjunction of a fixed number of atoms and therefore is not a clause
//! body. Two shapes of consumer exist, and this module serves the first:
//!
//! * **membership**, where the rule concludes something about EACH member independently
//!   (`scm-int`, `scm-uni`) or about a PAIR of distinct members (`prp-adp`, `cax-adc`) —
//!   satisfied by a relation with one row per member;
//! * **ordered traversal**, where the rule walks the list in order and the conclusion is a
//!   function of the whole of it (`prp-spo2`, `prp-key`) — satisfied by recursion over
//!   `rdf:first` / `rdf:rest` in the clauses themselves, which is what [`crate::calculus::prp`]
//!   writes.
//!
//! # `LIST(head, index, member)`, and where its three arguments live
//!
//! The pre-pass walks each collection ONCE and emits one fact per member:
//!
//! ```text
//! LIST(head, index, member)
//! ```
//!
//! A [`ClauseAtom`](purrdf_datalog::clause::ClauseAtom) is an arity-4 quad
//! `(subject, predicate, object, graph)`. The predicate names the relation, which leaves
//! three positions for three arguments, so the encoding is forced:
//!
//! ```text
//! quad(head, ⟪list⟫, member, index)
//! ```
//!
//! The fourth position is a GRAPH only for the relations whose predicate is a spec IRI.
//! For an internal relation the positional convention is this crate's to fix, and it fixes
//! it here: the fourth position carries the relation's third argument. That is why
//! `the_declared_programs_read_and_write_the_default_graph_only` is stated over the atoms
//! with a spec predicate rather than over all of them.
//!
//! # The index, and why `i ≠ j` is expressible at all
//!
//! `prp-adp` and `cax-adc` are written over two members `?ci` and `?cj` with the side
//! condition `i ≠ j`, and a DL clause has no inequality. It has NEGATION, so the pre-pass
//! also emits the REFLEXIVE index relation
//!
//! ```text
//! INDEX_EQUAL(i, i)   for every index it created
//! ```
//!
//! and `¬INDEX_EQUAL(?i, ?j)` is exactly `?i ≠ ?j` over the indices that exist. Both rules
//! conclude `false`, so neither is evaluated today — but a rule stated without its side
//! condition would be UNSOUND rather than merely unevaluated, which is why the relation is
//! materialized rather than the condition dropped.
//!
//! # No IRI is minted, and no internal id can reach the output
//!
//! PurRDF mints no vocabulary, so `⟪list⟫` is NOT an IRI and not a term of any kind: it is
//! an interner-local surface, and its surfaces all begin with [`INTERNAL_SIGIL`] — U+0000,
//! the one byte no RDF term's surface can begin with. `crate::engine::surface_of` brackets
//! an IRI with `<`, prefixes a blank node with `_`, opens a literal with `"` and a triple
//! term with `<`, and escapes every control character inside all four, so the internal
//! space and the term space are disjoint by construction rather than by convention.
//!
//! `crate::engine::close` drops every conclusion whose predicate is internal before the
//! answer is materialized, so an internal id cannot reach the dataset builder, let alone a
//! serializer. `no_internal_id_reaches_a_serialized_closure` is the assertion.
//!
//! # A malformed or cyclic list is a hard ERROR
//!
//! The walk is bounded by the number of distinct cells it has already visited, so a cycle
//! terminates on the step that revisits one rather than running forever. A cell with no
//! `rdf:first`, with two, with no `rdf:rest`, with two, or a walk that never reaches
//! `rdf:nil` is equally a refusal: OWL 2 requires these objects to BE well-formed
//! collections, and a chase that silently used the well-formed prefix of a broken one
//! would answer a question nobody asked.
//!
//! Only the objects of the list-valued OWL predicates are walked ([`LIST_VALUED`]). An
//! `rdf:first` / `rdf:rest` triple that no OWL axiom points at is ordinary data — RDF says
//! nothing about its shape — and is left alone.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::vocab::{
    OWL_DISTINCTMEMBERS, OWL_HASKEY, OWL_INTERSECTIONOF, OWL_MEMBERS, OWL_ONEOF,
    OWL_PROPERTYCHAINAXIOM, OWL_UNIONOF, RDF_FIRST, RDF_NIL, RDF_REST,
};

/// The byte every internal surface begins with, and which no RDF term's surface can.
///
/// U+0000 is a control character, and `crate::engine::surface_of` escapes every control
/// character it meets in an IRI and in a literal's lexical form; a blank node's surface
/// begins with `_` and a triple term's with `<`. So `surface.starts_with(INTERNAL_SIGIL)`
/// decides "internal" for every surface the store can hold, with no table to keep in step.
pub(crate) const INTERNAL_SIGIL: char = '\u{0}';

/// The internal predicate of `LIST(head, index, member)`.
pub(crate) const LIST_RELATION: &str = "\u{0}list";

/// The internal predicate of the reflexive `INDEX_EQUAL(i, i)`, whose NEGATION is the
/// `i ≠ j` side condition `prp-adp` and `cax-adc` are written with.
pub(crate) const INDEX_EQUAL_RELATION: &str = "\u{0}index-equal";

/// The internal predicate of `CHAIN(cell, u, v)` — `prp-spo2`'s ordered traversal.
pub(crate) const CHAIN_RELATION: &str = "\u{0}chain";

/// The internal predicate of `AGREE(cell, x, y)` — `prp-key`'s ordered traversal.
pub(crate) const AGREE_RELATION: &str = "\u{0}agree";

/// Every internal relation this crate names, for the tests that range over all of them.
#[cfg(test)]
pub(crate) const INTERNAL_RELATIONS: [&str; 4] = [
    LIST_RELATION,
    INDEX_EQUAL_RELATION,
    CHAIN_RELATION,
    AGREE_RELATION,
];

/// The OWL predicates whose OBJECT is required to be an RDF collection.
///
/// Transcribed from the rule tables that write `LIST[?x, …]`: `owl:intersectionOf` and
/// `owl:unionOf` (`cls-int1`, `cls-int2`, `cls-uni`, `scm-int`, `scm-uni`), `owl:oneOf`
/// (`cls-oo`), `owl:members` (`eq-diff2`, `prp-adp`, `cax-adc`), `owl:distinctMembers`
/// (`eq-diff3`), `owl:propertyChainAxiom` (`prp-spo2`) and `owl:hasKey` (`prp-key`).
///
/// The list is what makes the refusal below meaningful: these seven objects MUST be
/// well-formed collections for the axioms to mean anything, and every other
/// `rdf:first` / `rdf:rest` triple in a graph is data this pre-pass does not judge.
pub(crate) const LIST_VALUED: [&str; 7] = [
    OWL_INTERSECTIONOF,
    OWL_UNIONOF,
    OWL_ONEOF,
    OWL_MEMBERS,
    OWL_DISTINCTMEMBERS,
    OWL_PROPERTYCHAINAXIOM,
    OWL_HASKEY,
];

/// Whether `surface` is an internal id rather than the surface of an RDF term.
pub(crate) fn is_internal(surface: &str) -> bool {
    surface.starts_with(INTERNAL_SIGIL)
}

/// The internal id of list position `index`.
///
/// Ordinals, not IRIs and not literals: the position of a member in a collection is a
/// coordinate of this crate's own relation, and giving it an RDF spelling would put a term
/// in the store that a caller could write down and join against.
fn index_surface(index: usize) -> String {
    format!("{INTERNAL_SIGIL}{index}")
}

/// Why a collection could not be walked.
///
/// Every variant names the CELL it stopped at, because "the list under `?x` is broken" is
/// not actionable and "the cell `<…>` has no `rdf:rest`" is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MalformedList {
    /// The list head the walk started from, as its lexical surface.
    head: String,
    /// The cell the walk stopped at, as its lexical surface.
    cell: String,
    /// What was wrong with that cell.
    fault: Fault,
}

/// The five ways a collection can fail to be one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    /// The cell carries no `rdf:first`.
    NoFirst,
    /// The cell carries more than one `rdf:first`.
    ManyFirst,
    /// The cell carries no `rdf:rest`.
    NoRest,
    /// The cell carries more than one `rdf:rest`.
    ManyRest,
    /// The walk reached a cell it had already visited: the list is cyclic.
    Cycle,
}

impl Fault {
    /// The fault, as the sentence a diagnostic reads.
    const fn describe(self) -> &'static str {
        match self {
            Self::NoFirst => "carries no rdf:first",
            Self::ManyFirst => "carries more than one rdf:first",
            Self::NoRest => "carries no rdf:rest",
            Self::ManyRest => "carries more than one rdf:rest",
            Self::Cycle => "was already visited, so the collection is cyclic",
        }
    }
}

impl fmt::Display for MalformedList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the RDF collection under {} is malformed: the cell {} {}",
            self.head,
            self.cell,
            self.fault.describe()
        )
    }
}

/// One row of the internal relations the pre-pass materializes.
///
/// A quad in the store's own coordinates, so [`crate::engine::close`] inserts it without a
/// second encoding step and this module owns the positional convention in one place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalFact {
    /// The store subject surface.
    pub(crate) subject: String,
    /// The store predicate surface — always one of [`INTERNAL_RELATIONS`].
    pub(crate) predicate: &'static str,
    /// The store object surface.
    pub(crate) object: String,
    /// The store graph surface, which for an internal relation is its third argument.
    pub(crate) graph: String,
}

/// The `rdf:first` / `rdf:rest` structure of one dataset's default graph, plus the heads
/// the OWL axioms point at.
///
/// Built by one pass over the quads that are being seeded anyway, so walking the
/// collections costs no second traversal of the dataset.
#[derive(Debug, Default)]
pub(crate) struct ListIndex {
    /// Cell surface → the objects of its `rdf:first` triples, in insertion order.
    first: BTreeMap<String, Vec<String>>,
    /// Cell surface → the objects of its `rdf:rest` triples, in insertion order.
    rest: BTreeMap<String, Vec<String>>,
    /// The surfaces an OWL list-valued predicate points at, deduplicated and ordered.
    heads: BTreeSet<String>,
}

impl ListIndex {
    /// Record one default-graph triple, by the surfaces the store holds it under.
    ///
    /// `predicate` is compared against the bracketed spec surfaces, which is the same
    /// comparison a clause constant makes, so this pass and the clauses agree on what a
    /// predicate IS without a second rendering convention.
    pub(crate) fn observe(&mut self, subject: &str, predicate: &str, object: &str) {
        if predicate == bracketed(RDF_FIRST) {
            self.first
                .entry(subject.to_owned())
                .or_default()
                .push(object.to_owned());
        } else if predicate == bracketed(RDF_REST) {
            self.rest
                .entry(subject.to_owned())
                .or_default()
                .push(object.to_owned());
        } else if LIST_VALUED.iter().any(|iri| predicate == bracketed(iri)) {
            self.heads.insert(object.to_owned());
        }
    }

    /// Walk every head into [`InternalFact`]s, in head order and then member order.
    ///
    /// # Errors
    ///
    /// [`MalformedList`] the first time a cell is not a well-formed collection cell — see
    /// the [module docs](self) for why that is a refusal rather than a truncation.
    pub(crate) fn materialize(&self) -> Result<Vec<InternalFact>, MalformedList> {
        let nil = bracketed(RDF_NIL);
        let mut facts = Vec::new();
        let mut indices: BTreeSet<usize> = BTreeSet::new();
        for head in &self.heads {
            // `rdf:nil` is the EMPTY collection, and an empty collection is well formed:
            // it simply contributes no member. A rule whose premise needs one therefore
            // does not fire, which is what "no members" means.
            if head == &nil {
                continue;
            }
            let mut cell = head.clone();
            let mut visited: BTreeSet<String> = BTreeSet::new();
            let mut index = 0_usize;
            loop {
                if !visited.insert(cell.clone()) {
                    return Err(self.fault(head, &cell, Fault::Cycle));
                }
                let member = match self.first.get(&cell).map(Vec::as_slice) {
                    Some([only]) => only.clone(),
                    Some(_) => return Err(self.fault(head, &cell, Fault::ManyFirst)),
                    None => return Err(self.fault(head, &cell, Fault::NoFirst)),
                };
                let next = match self.rest.get(&cell).map(Vec::as_slice) {
                    Some([only]) => only.clone(),
                    Some(_) => return Err(self.fault(head, &cell, Fault::ManyRest)),
                    None => return Err(self.fault(head, &cell, Fault::NoRest)),
                };
                facts.push(InternalFact {
                    subject: head.clone(),
                    predicate: LIST_RELATION,
                    object: member,
                    graph: index_surface(index),
                });
                indices.insert(index);
                index += 1;
                if next == nil {
                    break;
                }
                cell = next;
            }
        }
        // The reflexive index relation, whose negation is the `i ≠ j` side condition.
        // One row per index that exists, so `¬INDEX_EQUAL(?i, ?j)` cannot be satisfied by
        // an index the walk never created.
        for index in indices {
            let surface = index_surface(index);
            facts.push(InternalFact {
                subject: surface.clone(),
                predicate: INDEX_EQUAL_RELATION,
                object: surface,
                graph: String::new(),
            });
        }
        Ok(facts)
    }

    /// A refusal naming the head, the cell and the fault.
    fn fault(&self, head: &str, cell: &str, fault: Fault) -> MalformedList {
        MalformedList {
            head: head.to_owned(),
            cell: cell.to_owned(),
            fault,
        }
    }
}

/// The store surface of a constant IRI: `<iri>`.
fn bracketed(iri: &str) -> String {
    format!("<{iri}>")
}

#[cfg(test)]
mod tests {
    use super::{
        AGREE_RELATION, CHAIN_RELATION, INDEX_EQUAL_RELATION, INTERNAL_RELATIONS, LIST_RELATION,
        LIST_VALUED, ListIndex, is_internal,
    };
    use crate::vocab::{OWL_INTERSECTIONOF, RDF_FIRST, RDF_NIL, RDF_REST};
    use std::collections::BTreeSet;

    /// A fixture class. PurRDF mints no vocabulary, so every fixture term is `example.org`.
    const EX_C: &str = "<http://example.org/C>";
    /// A fixture list member.
    const EX_A: &str = "<http://example.org/A>";
    /// A second fixture list member.
    const EX_B: &str = "<http://example.org/B>";
    /// The first fixture list cell.
    const EX_L0: &str = "<http://example.org/l0>";
    /// The second fixture list cell.
    const EX_L1: &str = "<http://example.org/l1>";

    /// The store surface of a constant IRI.
    fn s(iri: &str) -> String {
        format!("<{iri}>")
    }

    /// A two-member intersection list `C owl:intersectionOf (A B)`.
    fn two_member_list() -> ListIndex {
        let mut index = ListIndex::default();
        index.observe(EX_C, &s(OWL_INTERSECTIONOF), EX_L0);
        index.observe(EX_L0, &s(RDF_FIRST), EX_A);
        index.observe(EX_L0, &s(RDF_REST), EX_L1);
        index.observe(EX_L1, &s(RDF_FIRST), EX_B);
        index.observe(EX_L1, &s(RDF_REST), &s(RDF_NIL));
        index
    }

    /// The walk yields one `LIST` row per member, indexed from zero and in list order,
    /// plus one reflexive index row per index it created.
    #[test]
    fn a_well_formed_list_walks_to_one_row_per_member() {
        let facts = two_member_list().materialize().expect("a well-formed list");
        let list: Vec<(&str, &str, &str, &str)> = facts
            .iter()
            .filter(|f| f.predicate == LIST_RELATION)
            .map(|f| {
                (
                    f.subject.as_str(),
                    f.predicate,
                    f.object.as_str(),
                    f.graph.as_str(),
                )
            })
            .collect();
        assert_eq!(
            list,
            vec![
                (EX_L0, LIST_RELATION, EX_A, "\u{0}0"),
                (EX_L0, LIST_RELATION, EX_B, "\u{0}1"),
            ],
            "the head is the LIST subject, the member the object, the index the fourth \
             position"
        );
        let equal: Vec<(&str, &str)> = facts
            .iter()
            .filter(|f| f.predicate == INDEX_EQUAL_RELATION)
            .map(|f| (f.subject.as_str(), f.object.as_str()))
            .collect();
        assert_eq!(
            equal,
            vec![("\u{0}0", "\u{0}0"), ("\u{0}1", "\u{0}1")],
            "INDEX_EQUAL is reflexive, so its negation is inequality"
        );
    }

    /// The empty collection is WELL FORMED and contributes nothing.
    #[test]
    fn rdf_nil_is_the_empty_collection_and_is_not_an_error() {
        let mut index = ListIndex::default();
        index.observe(EX_C, &s(OWL_INTERSECTIONOF), &s(RDF_NIL));
        assert_eq!(index.materialize().expect("nil is well formed"), Vec::new());
    }

    /// An `rdf:first` / `rdf:rest` structure no OWL axiom points at is ordinary data and
    /// is neither walked nor judged — including a broken one.
    #[test]
    fn an_unreferenced_collection_is_left_alone() {
        let mut index = ListIndex::default();
        index.observe(EX_L0, &s(RDF_FIRST), EX_A);
        // No rdf:rest at all, and no head points here.
        assert!(index.materialize().expect("not walked").is_empty());
    }

    /// A CYCLE terminates with a refusal naming the cell, rather than running forever.
    #[test]
    fn a_cyclic_list_is_a_hard_error() {
        let mut index = ListIndex::default();
        index.observe(EX_C, &s(OWL_INTERSECTIONOF), EX_L0);
        index.observe(EX_L0, &s(RDF_FIRST), EX_A);
        index.observe(EX_L0, &s(RDF_REST), EX_L1);
        index.observe(EX_L1, &s(RDF_FIRST), EX_B);
        index.observe(EX_L1, &s(RDF_REST), EX_L0);
        let error = index.materialize().expect_err("a cycle is refused");
        let rendered = error.to_string();
        assert!(rendered.contains("cyclic"), "{rendered}");
        assert!(rendered.contains("http://example.org/l0"), "{rendered}");
    }

    /// Every way a cell can fail to be a collection cell is a refusal that names it.
    #[test]
    fn every_malformation_is_refused_and_named() {
        /// One malformation: the diagnostic it must produce, and the triples that cause
        /// it.
        type Case = (&'static str, fn(&mut ListIndex));
        let cases: [Case; 4] = [
            ("carries no rdf:first", |index| {
                index.observe(EX_L0, &s(RDF_REST), &s(RDF_NIL));
            }),
            ("carries more than one rdf:first", |index| {
                index.observe(EX_L0, &s(RDF_FIRST), EX_A);
                index.observe(EX_L0, &s(RDF_FIRST), EX_B);
                index.observe(EX_L0, &s(RDF_REST), &s(RDF_NIL));
            }),
            ("carries no rdf:rest", |index| {
                index.observe(EX_L0, &s(RDF_FIRST), EX_A);
            }),
            ("carries more than one rdf:rest", |index| {
                index.observe(EX_L0, &s(RDF_FIRST), EX_A);
                index.observe(EX_L0, &s(RDF_REST), &s(RDF_NIL));
                index.observe(EX_L0, &s(RDF_REST), EX_L1);
            }),
        ];
        for (expected, build) in cases {
            let mut index = ListIndex::default();
            index.observe(EX_C, &s(OWL_INTERSECTIONOF), EX_L0);
            build(&mut index);
            let error = index
                .materialize()
                .expect_err("malformed lists are refused");
            let rendered = error.to_string();
            assert!(rendered.contains(expected), "{rendered}");
            assert!(rendered.contains("http://example.org/l0"), "{rendered}");
        }
    }

    /// A list that runs off the end — the last cell's `rdf:rest` names a cell that is not
    /// one — is refused where it breaks, not truncated to its well-formed prefix.
    #[test]
    fn a_list_that_never_reaches_nil_is_refused() {
        let mut index = ListIndex::default();
        index.observe(EX_C, &s(OWL_INTERSECTIONOF), EX_L0);
        index.observe(EX_L0, &s(RDF_FIRST), EX_A);
        index.observe(EX_L0, &s(RDF_REST), EX_L1);
        let error = index.materialize().expect_err("a broken tail is refused");
        assert!(
            error.to_string().contains("http://example.org/l1"),
            "{error}"
        );
    }

    /// Every internal relation is internal, distinct, and disjoint from every term
    /// surface — which is the property that keeps an internal id out of the output.
    #[test]
    fn internal_relations_are_disjoint_from_the_term_space() {
        let distinct: BTreeSet<&str> = INTERNAL_RELATIONS.into_iter().collect();
        assert_eq!(distinct.len(), INTERNAL_RELATIONS.len());
        for relation in INTERNAL_RELATIONS {
            assert!(is_internal(relation), "{relation:?}");
        }
        // The four surfaces an RDF term can begin with, and the empty default graph.
        for surface in [
            "<http://example.org/p>",
            "_:0.b0",
            "\"cat\"",
            "<<( <a> <b> <c> )>>",
            "",
        ] {
            assert!(!is_internal(surface), "{surface:?}");
        }
        // The four are named individually so a rename cannot silently drop one.
        assert_eq!(
            INTERNAL_RELATIONS,
            [
                LIST_RELATION,
                INDEX_EQUAL_RELATION,
                CHAIN_RELATION,
                AGREE_RELATION
            ]
        );
    }

    /// The list-valued predicate table is the OWL vocabulary, deduplicated.
    #[test]
    fn the_list_valued_predicates_are_owl_vocabulary() {
        let distinct: BTreeSet<&str> = LIST_VALUED.into_iter().collect();
        assert_eq!(distinct.len(), LIST_VALUED.len());
        for predicate in LIST_VALUED {
            assert!(
                predicate.starts_with("http://www.w3.org/2002/07/owl#"),
                "{predicate} is not OWL vocabulary"
            );
        }
    }
}
