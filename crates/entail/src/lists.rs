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
//! `eq-diff2`, `eq-diff3`, `prp-adp` and `cax-adc` are written over two members `?ci` and
//! `?cj` with the side condition `i ≠ j`, and a DL clause has no inequality. So the
//! pre-pass materializes the condition itself:
//!
//! ```text
//! INDEX_DISTINCT(i, j)   for every ORDERED pair of DIFFERENT indices it created
//! ```
//!
//! and `INDEX_DISTINCT(?i, ?j)` is exactly `?i ≠ ?j` over the indices that exist. It is a
//! POSITIVE relation rather than the negation of a reflexive one, and that is load-bearing
//! rather than a style choice: these four rules all conclude `false`, so they are lowered
//! into clauses whose head is the internal clash relation, and a negated body atom in a
//! program whose rules quantify over the PREDICATE position puts the negative dependency
//! edge inside a cycle — `purrdf-datalog` then refuses the whole program as
//! non-stratifiable, correctly, because a variable predicate can range over the negated
//! relation itself. Materializing the inequality is quadratic in the LENGTH OF THE LONGEST
//! LIST and in nothing else, so the cost is a handful of rows.
//!
//! A rule stated without its side condition would be UNSOUND rather than merely slower:
//! `i = j` would match, and one class assertion would be an inconsistency.
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

/// The GRAPH every BINARY internal relation's rows live in.
///
/// A ternary internal relation spends the atom's fourth position on its third argument
/// ([`LIST_RELATION`], [`CHAIN_RELATION`], [`AGREE_RELATION`]), so its rows already sit
/// outside the default partition. A BINARY one has that position spare, and it may NOT
/// spend it on the default graph: the OWL-RL lane fires rules whose body is
/// `T(?s, ?p, ?o)` with an unbound PREDICATE — `eq-ref`, `eq-rep-s`, `eq-rep-p`,
/// `eq-rep-o` — and such an atom sweeps every partition of the graph it names. A binary
/// internal relation in the default graph would therefore be swept as if it were data, and
/// `eq-ref` would conclude `⟪dt-equal⟫ owl:sameAs ⟪dt-equal⟫` about this crate's own
/// bookkeeping.
///
/// So the rows go in a graph whose name leads with [`INTERNAL_SIGIL`] and is therefore not
/// a term any dataset can hold. `no_internal_id_reaches_a_serialized_closure` is what keeps
/// that true end to end.
pub(crate) const INTERNAL_GRAPH: &str = "\u{0}graph";

/// The internal predicate of `LIST(head, index, member)`.
pub(crate) const LIST_RELATION: &str = "\u{0}list";

/// The internal predicate of `INDEX_DISTINCT(i, j)` — the `i ≠ j` side condition
/// `eq-diff2`, `eq-diff3`, `prp-adp` and `cax-adc` are written with, materialized as a
/// POSITIVE relation over the index pairs that exist.
pub(crate) const INDEX_DISTINCT_RELATION: &str = "\u{0}index-distinct";

/// The internal predicate of `CHAIN(cell, u, v)` — `prp-spo2`'s ordered traversal.
pub(crate) const CHAIN_RELATION: &str = "\u{0}chain";

/// The internal predicate of `AGREE(cell, x, y)` — `prp-key`'s ordered traversal.
pub(crate) const AGREE_RELATION: &str = "\u{0}agree";

/// The internal predicate of `ALL_TYPES(cell, y)` — `cls-int1`'s universal traversal.
///
/// "`?y` is an instance of every class from `?cell` onwards", which is the conjunction of
/// `n` body atoms `cls-int1` writes as `LIST[?x, ?c1, …, ?cn]` followed by `?y rdf:type
/// ?c1 … ?y rdf:type ?cn`. Same shape as [`AGREE_RELATION`], one argument narrower, so its
/// third position is the default graph rather than a third argument.
pub(crate) const ALL_TYPES_RELATION: &str = "\u{0}all-types";

/// The internal predicate of `CLASH(rule, a, b)` — a satisfied inconsistency body.
///
/// The eight rules of Tables 5 and 7, the five of Table 6, the three of Table 4 and the
/// one of Table 8 whose conclusion is `false` are DECLARED with `false`
/// ([`HeadForm::Inconsistency`](purrdf_datalog::clause::HeadForm::Inconsistency)) and
/// LOWERED, mechanically, to a clause whose head is one atom of this relation — see
/// [`crate::calculus::constraint_clause`]. The evaluator therefore runs the
/// specification's own body, and a match becomes
/// [`EntailError::Inconsistent`](crate::EntailError) carrying the matched body facts as
/// the witness rather than a triple in the closure.
///
/// A row of this relation can never reach an answer for two independent reasons: its
/// predicate is internal, like every other relation here, and the run it belongs to is
/// refused outright.
pub(crate) const CLASH_RELATION: &str = "\u{0}clash";

/// The internal predicate of `DT_VALUE(literal, datatype)` — the datatype pre-pass's
/// membership relation.
///
/// "the data value of `literal` lies in the value space of `datatype`", computed once per
/// run by `purrdf-xsd` over the literals the dataset actually holds. `dt-type2` reads it;
/// see [`crate::datatypes`] for why a value space is walked by a pre-pass rather than
/// quantified over by a clause.
pub(crate) const DT_VALUE_RELATION: &str = "\u{0}dt-value";

/// The internal predicate of `DT_EQUAL(lt1, lt2)` — two literals with ONE data value.
pub(crate) const DT_EQUAL_RELATION: &str = "\u{0}dt-equal";

/// The internal predicate of `DT_DIFFERENT(lt1, lt2)` — two literals with DIFFERENT data
/// values.
///
/// The `dt-diff` side of the pair [`DT_EQUAL_RELATION`] answers, and POSITIVE for the same
/// reason [`INDEX_DISTINCT_RELATION`] is: this program quantifies over the predicate
/// position, so every negated body atom's dependency edge lands inside a cycle and
/// `purrdf-datalog` refuses the whole program as non-stratifiable. An inequality a rule
/// needs is therefore materialized, never negated.
pub(crate) const DT_DIFFERENT_RELATION: &str = "\u{0}dt-different";

/// The internal predicate of `DT_ILL_TYPED(lt, dt)` — a literal OUTSIDE `dt`'s value
/// space, which `dt-not-type` turns into an inconsistency.
pub(crate) const DT_ILL_TYPED_RELATION: &str = "\u{0}dt-ill-typed";

/// The internal predicate of `DATATYPED(literal, datatype)` — a literal of the graph whose
/// datatype RDF 1.2 Semantics §8 makes MANDATORY for every interpretation to recognize.
///
/// `rdfD1`'s premise, and a pre-pass relation for the same reason `DT_VALUE` is: the clause
/// language has no term-kind test, so "a triple in which a datatyped literal appears" is a
/// question about the SHAPE of a term, and only a pass over the dataset can answer it.
pub(crate) const DATATYPED_RELATION: &str = "\u{0}datatyped";

/// The internal predicate of `QUOTED(triple-term, rdfs:Proposition)` — a triple term of the
/// graph, paired with the class `rdfs14` types its surrogate with.
///
/// The object is a constant, and it is carried anyway so this relation has exactly
/// [`DATATYPED_RELATION`]'s shape: the two rules differ in what they observe, not in what
/// they do about it, and one clause shape for both is one fewer place to be wrong.
pub(crate) const QUOTED_RELATION: &str = "\u{0}quoted";

/// The internal predicate of `SURROGATE_D(literal, _:nnn)` — the fresh blank node `rdfD1`
/// invents for a datatyped literal.
///
/// The relation is what gives the surrogate an ADDRESS. `rdfD1` concludes about a fresh
/// `_:nnn` in every position the literal occupied, and a chase mints a witness as a
/// function of its FRONTIER — so without an atom that mentions the literal, two literals
/// under the same subject and predicate would share one witness. Naming the literal in the
/// head is what makes the witness a function of the literal, which is what the rule says.
pub(crate) const SURROGATE_D_RELATION: &str = "\u{0}surrogate-d";

/// The internal predicate of `SURROGATE_T(triple-term, _:nnn)` — `rdfs14`'s surrogate.
///
/// Separate from [`SURROGATE_D_RELATION`] so a firing is attributable: the substitution
/// clauses read one relation each, so the rule that invented a surrogate is the rule that
/// gets credited for putting it into a triple.
pub(crate) const SURROGATE_T_RELATION: &str = "\u{0}surrogate-t";

/// Every internal relation this crate names, for the tests that range over all of them.
#[cfg(test)]
pub(crate) const INTERNAL_RELATIONS: [&str; 14] = [
    LIST_RELATION,
    INDEX_DISTINCT_RELATION,
    CHAIN_RELATION,
    AGREE_RELATION,
    ALL_TYPES_RELATION,
    CLASH_RELATION,
    DT_VALUE_RELATION,
    DT_EQUAL_RELATION,
    DT_DIFFERENT_RELATION,
    DT_ILL_TYPED_RELATION,
    DATATYPED_RELATION,
    QUOTED_RELATION,
    SURROGATE_D_RELATION,
    SURROGATE_T_RELATION,
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
        // The `i ≠ j` side condition, materialized: one row per ORDERED pair of DIFFERENT
        // indices the walk created. Quadratic in the length of the longest list and in
        // nothing else, and positive rather than a negation — see the [module docs](self)
        // for why that difference decides whether the whole program stratifies.
        for left in &indices {
            for right in &indices {
                if left != right {
                    facts.push(InternalFact {
                        subject: index_surface(*left),
                        predicate: INDEX_DISTINCT_RELATION,
                        object: index_surface(*right),
                        graph: INTERNAL_GRAPH.to_owned(),
                    });
                }
            }
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
        AGREE_RELATION, CHAIN_RELATION, INDEX_DISTINCT_RELATION, INTERNAL_RELATIONS, LIST_RELATION,
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
        let distinct: Vec<(&str, &str)> = facts
            .iter()
            .filter(|f| f.predicate == INDEX_DISTINCT_RELATION)
            .map(|f| (f.subject.as_str(), f.object.as_str()))
            .collect();
        assert_eq!(
            distinct,
            vec![("\u{0}0", "\u{0}1"), ("\u{0}1", "\u{0}0")],
            "INDEX_DISTINCT holds exactly the ordered pairs of DIFFERENT indices"
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
        // All fourteen are named individually so a rename cannot silently drop one.
        assert_eq!(
            INTERNAL_RELATIONS,
            [
                LIST_RELATION,
                INDEX_DISTINCT_RELATION,
                CHAIN_RELATION,
                AGREE_RELATION,
                super::ALL_TYPES_RELATION,
                super::CLASH_RELATION,
                super::DT_VALUE_RELATION,
                super::DT_EQUAL_RELATION,
                super::DT_DIFFERENT_RELATION,
                super::DT_ILL_TYPED_RELATION,
                super::DATATYPED_RELATION,
                super::QUOTED_RELATION,
                super::SURROGATE_D_RELATION,
                super::SURROGATE_T_RELATION,
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
