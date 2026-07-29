// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Running the declared calculus: [`calculus_program`](crate::calculus_program) evaluated
//! by `purrdf-datalog`.
//!
//! # The declaration IS the implementation
//!
//! [`crate::calculus`] renders every rule this crate fires as a [`DlClause`]. This module
//! does not re-state a single one of them: it seeds a [`RelationStore`] from the dataset's
//! default graph, hands `purrdf-datalog` the clauses the calculus declares, and reads the
//! answer back. There is therefore one statement of the calculus, and the digest a
//! [`ReasoningReport`](crate::ReasoningReport) carries is the digest of the very clauses
//! that ran.
//!
//! # The bridge is a lexical surface, and it is injective
//!
//! `purrdf-datalog` interns a term by its **lexical surface** (`&str`), while the RDF 1.2
//! IR identifies a term by a [`TermValue`]. [`surface_of`] is the one translation, and
//! [`Terms`] is the inverse: every surface that enters the store is recorded against the
//! value it came from, so materializing an answer never has to *parse* a surface back.
//!
//! Two properties make that sound, and both are load-bearing:
//!
//! * [`surface_of`] is INJECTIVE — distinct [`TermValue`]s render to distinct surfaces —
//!   so two terms can never collide into one store term. It is the repository's own
//!   canonical N-Quads term spelling (`purrdf_core::canonicalize`'s), with a blank node
//!   qualified by its scope because a scope is part of a blank node's identity (C0.2)
//!   and is not recoverable from a canonical label.
//! * The evaluator MINTS NO TERMS. Every clause is range-restricted (`compile` refuses
//!   one that is not) and none is existential, so every term in the answer is either a
//!   term this module seeded or a constant of the program itself. [`Terms`] records both
//!   before evaluation starts, which is what makes its lookup total.
//!
//! An IRI's surface is `<iri>`, which is exactly what
//! [`ClauseTerm::iri`](purrdf_datalog::clause::ClauseTerm::iri) renders to — so a clause
//! constant and a dataset term compare as the same bytes, without a second convention.
//!
//! # What the RDF 1.2 IR cannot hold
//!
//! The evaluator's term space is wider than RDF 1.2's: nothing there stops a literal
//! reaching subject position or a blank node reaching predicate position, because a
//! [`Fact`](purrdf_datalog::store::Fact) is four terms with no positional restriction.
//! Those conclusions are GENERALIZED-RDF triples, and the [`RdfDataset`] IR cannot
//! represent them. [`close`] therefore drops such a conclusion at the materialization
//! boundary rather than fabricating a term for it, counts the drop, and the count is what
//! raises the [`Construct::GeneralizedRdf`](crate::Construct::GeneralizedRdf) boundary.
//!
//! The drop is at the BOUNDARY, not in the calculus: the generalized fact stays in the
//! store and may still serve as a premise, so a conclusion that is itself representable is
//! not withheld merely because its derivation passed through one that was not.
//!
//! # Determinism
//!
//! Derivations arrive in `purrdf-datalog`'s total order — lexical by `(fact, rule,
//! sources)` — and are emitted in that order, so the derived quads reach the builder in a
//! sequence that is a function of the fact set alone. [`Terms`] is a `BTreeMap`, so no
//! hash iteration reaches anything either.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_datalog::clause::{ClauseTerm, DlClause};
use purrdf_datalog::seminaive::{Derivation, compile, evaluate};
use purrdf_datalog::store::RelationStore;

use crate::axioms::axioms_for;
use crate::calculus::{ChaseRule, clash_rule, program_with_attribution};
use crate::datatypes::LiteralIndex;
use crate::interner::intern_into;
use crate::lists::{CLASH_RELATION, ListIndex, is_internal};
use crate::report::RunStats;
use crate::report::{InconsistencyWitness, WitnessTriple};
use crate::vocab::{XSD_NONNEGATIVEINTEGER, XSD_STRING};
use crate::{EntailError, Regime};

/// Every LITERAL constant the declared calculus names, as `(lexical form, datatype IRI)`.
///
/// Two, and both are OWL 2 Profiles §4.3 Table 6's own: `cls-maxc1`, `cls-maxc2` and the
/// four `cls-maxqc*` rules match a cardinality against `"0"^^xsd:nonNegativeInteger` or
/// `"1"^^xsd:nonNegativeInteger`. They are declared here rather than in the family module
/// because [`Terms`] has to be able to read a surface BACK as a [`TermValue`], and a
/// [`ClauseTerm::Literal`] carries a rendered surface with no structure to recover one
/// from. One table, two consumers: [`literal_surface`] renders the clause constant and
/// [`Terms::record_literals`] records the value it reads back as, so the two cannot drift.
const DECLARED_LITERALS: [(&str, &str); 2] =
    [("0", XSD_NONNEGATIVEINTEGER), ("1", XSD_NONNEGATIVEINTEGER)];

/// The store surface of the typed literal `"lexical"^^<datatype>`.
///
/// The one rendering convention, shared by the clause constants of
/// [`crate::calculus::cls`] and by the dataset literals they must compare equal to.
pub(crate) fn literal_surface(lexical: &str, datatype: &str) -> String {
    surface_of(&TermValue::typed_literal(lexical, datatype))
}

/// A faithful copy of `ds` (the identity closure for `Simple`).
pub(crate) fn copy_of(ds: &RdfDataset) -> Result<Arc<RdfDataset>, EntailError> {
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    b.freeze().map_err(|e| EntailError::Build(e.to_string()))
}

/// Close `ds` under `regime`'s declared calculus and emit `original + inferred`.
///
/// # The axiomatic triples are seeded, not concluded
///
/// [`crate::axioms`]'s finite table is inserted into the fact store beside `ds`'s own
/// quads, because that is what the definition of RDFS entailment says it is: a premise
/// every interpretation satisfies, not something a rule derives. Its CONSEQUENCES are
/// derivations like any other and are credited to the rule that drew them —
/// `:a rdfs:subClassOf :b` reaches `:a rdfs:subClassOf :a` through `rdfs3` and then
/// `rdfs10`, and both firings appear in the report.
///
/// The axioms themselves are therefore NOT emitted: they are neither in `ds` nor
/// derivations, and inventing a rule id to credit them to would put a firing in the tally
/// that no rule of the specification's tables licenses. A closure that omits an entailed
/// triple is an incompleteness, so it is reported —
/// [`Construct::AxiomaticTriples`](crate::Construct::AxiomaticTriples) says both this and
/// the unbounded `rdf:_n` family in one boundary.
///
/// The default graph alone supplies premises and receives conclusions — every atom over
/// SPEC vocabulary names
/// [`ClauseTerm::DefaultGraph`](purrdf_datalog::clause::ClauseTerm::DefaultGraph), which
/// `the_declared_programs_read_and_write_the_default_graph_only` asserts — so quads in a
/// named graph are carried through untouched and the run reports the
/// [`Construct::NamedGraph`](crate::Construct::NamedGraph) boundary. The atoms of the
/// INTERNAL relations ([`crate::lists`]) use that fourth position for the relation's third
/// argument instead, which is not a graph at all and never reaches the answer.
///
/// # The collections are walked before the clauses run
///
/// The `OWL-RL` lane's rule table writes `LIST[…]`, a meta-notation no clause has, so this
/// function walks each RDF collection an OWL axiom points at into an internal relation
/// before evaluating. A malformed or cyclic collection is [`EntailError::MalformedList`]
/// rather than a closure over its well-formed prefix.
pub(crate) fn close(
    ds: &RdfDataset,
    regime: Regime,
) -> Result<(Arc<RdfDataset>, RunStats), EntailError> {
    let (program, attribution) = program_with_attribution(regime);

    let mut terms = Terms::default();
    terms.record_program(&program);
    terms.record_literals();
    let mut edb = RelationStore::new();
    // The axiomatic triples are PREMISES, not conclusions: `S RDFS entails E` is defined
    // over the interpretations satisfying S *and* the axioms, and no rule of §9.2.1
    // concludes one. Seeding them beside the dataset's own quads is that definition,
    // written down. See `crate::axioms` for the table and for which lanes assert it.
    for &(subject, predicate, object) in axioms_for(regime) {
        let subject = terms.record(&TermValue::iri(subject));
        let predicate = terms.record(&TermValue::iri(predicate));
        let object = terms.record(&TermValue::iri(object));
        let _ = edb.insert(&subject, &predicate, &object, RelationStore::DEFAULT_GRAPH);
    }
    let mut lists = ListIndex::default();
    let mut literals = LiteralIndex::default();
    for quad in ds.quads() {
        if quad.g.is_some() {
            continue; // entailment operates over the default graph
        }
        let subject = terms.record(&ds.term_value(quad.s));
        let predicate = terms.record(&ds.term_value(quad.p));
        let object = terms.record(&ds.term_value(quad.o));
        if walks_collections(regime) {
            lists.observe(&subject, &predicate, &object);
        }
        if decides_datatypes(regime) {
            for (surface, value) in [
                (&subject, ds.term_value(quad.s)),
                (&predicate, ds.term_value(quad.p)),
                (&object, ds.term_value(quad.o)),
            ] {
                observe_literal(&mut literals, surface, &value);
            }
        }
        let _ = edb.insert(&subject, &predicate, &object, RelationStore::DEFAULT_GRAPH);
    }
    // The RDF collections the OWL 2 axioms point at, walked ONCE into the internal
    // relations the `LIST[…]` rules join against. A malformed or cyclic collection stops
    // the run here rather than producing a closure over the well-formed prefix of it.
    if walks_collections(regime) {
        for fact in lists
            .materialize()
            .map_err(|error| EntailError::MalformedList(error.to_string()))?
        {
            let _ = edb.insert(&fact.subject, fact.predicate, &fact.object, &fact.graph);
        }
    }
    // The XSD value spaces OWL 2 Profiles Table 8 quantifies over, decided ONCE over the
    // literals the dataset holds. See [`crate::datatypes`] for why an infinite premise is
    // a boundary rather than a loop, and why an unmodelled datatype is not judged.
    if decides_datatypes(regime) {
        // A datatype the pre-pass names is a TERM of the store, and `dt-type2` writes it
        // into an `rdf:type` object, so the dictionary has to be able to read it back —
        // including the datatype of an ILL-TYPED literal, which need not be one of the
        // thirty-two the program's own constants already cover.
        let datatypes: Vec<String> = literals.datatypes().map(str::to_owned).collect();
        for datatype in datatypes {
            let _ = terms.record(&TermValue::iri(datatype));
        }
        for fact in literals.materialize() {
            let _ = edb.insert(&fact.subject, fact.predicate, &fact.object, &fact.graph);
        }
    }

    let executable = compile(program).map_err(EntailError::Evaluate)?;
    let evaluation = evaluate(&executable, edb).map_err(EntailError::Evaluate)?;

    // AN INCONSISTENCY IS DECIDED BEFORE AN ANSWER IS BUILT. Seventeen OWL 2 RL rules
    // conclude `false`, and a match on one of them says the knowledge base entails
    // everything — so there is no closure to hand back, only evidence. The first clash in
    // the evaluation's own total derivation order is the witness, which makes the choice a
    // function of the program and the data rather than of the round a rule happened to
    // fire in.
    if let Some(witness) = first_clash(&evaluation, &attribution, &terms, regime) {
        return Err(EntailError::Inconsistent(Box::new(witness)));
    }

    // The budget is the evaluator's own measurement, not a second tally kept alongside it.
    let mut stats = RunStats::of_budget(evaluation.budget());
    let mut b = RdfDatasetBuilder::new();
    b.push_dataset(ds);
    for derivation in evaluation.derivations() {
        let fact = derivation.fact();
        // An INTERNAL conclusion is bookkeeping, not an answer. `prp-spo2` and `prp-key`
        // accumulate their list traversals in relations whose predicate is an
        // interner-local id ([`crate::lists`]), and those rows are premises for the rule's
        // own final clause and nothing else. They are neither materialized — no internal
        // id may reach the dataset builder, let alone a serializer — nor credited, because
        // a per-rule count is "triples this rule was first to add" and a traversal row is
        // not a triple. Dropping them is also NOT the generalized-RDF boundary: nothing
        // was lost, so nothing is reported.
        if is_internal(&fact.predicate) {
            continue;
        }
        let subject = terms.value(&fact.subject);
        let predicate = terms.value(&fact.predicate);
        if !admits_subject(subject) || !admits_predicate(predicate) {
            stats.drop_generalized();
            continue;
        }
        // Credited only once the conclusion is known to be representable, so the per-rule
        // counts sum to exactly the inferred triples a caller can see.
        stats.commit(attribution[derivation.rule()]);
        let s = intern_into(&mut b, subject);
        let p = intern_into(&mut b, predicate);
        let o = intern_into(&mut b, terms.value(&fact.object));
        b.push_quad(s, p, o, None);
    }
    let dataset = b.freeze().map_err(|e| EntailError::Build(e.to_string()))?;
    Ok((dataset, stats))
}

/// Whether `regime`'s lane walks the RDF collections its axioms point at.
///
/// `OWL-RL` alone: it is the only lane whose rule table writes `LIST[…]`, and it is the
/// only calculus that REQUIRES those objects to be well-formed collections. An `RDFS` run
/// over a cyclic `owl:members` list is not an error, because RDFS says nothing about
/// `owl:members` and the cycle is ordinary data there.
const fn walks_collections(regime: Regime) -> bool {
    matches!(regime, Regime::OwlRl)
}

/// Whether `regime`'s lane decides XSD value spaces before it evaluates.
///
/// The two lanes that fire OWL 2 Profiles §4.3 Table 8: `OWL-RL`, which owns the table,
/// and `D`, which IS that table (see [`crate::rules::rules`]). No other lane looks inside
/// a literal at all — RDFS entailment compares a literal by its lexical form and datatype
/// IRI, never by its data value, which is what its own
/// [`Construct::DatatypeValueSpace`](crate::Construct::DatatypeValueSpace) boundary says.
const fn decides_datatypes(regime: Regime) -> bool {
    matches!(regime, Regime::OwlRl | Regime::D)
}

/// Record `value` in the datatype pre-pass's index, if it is a literal.
fn observe_literal(literals: &mut LiteralIndex, surface: &str, value: &TermValue) {
    if let TermValue::Literal {
        lexical_form,
        datatype,
        language,
        ..
    } = value
    {
        literals.observe(surface, lexical_form, datatype, language.is_some());
    }
}

/// The FIRST inconsistency the evaluation witnessed, in its own total derivation order.
///
/// A [`CLASH_RELATION`] row is what a `false`-headed rule's lowering
/// ([`crate::calculus::constraint_clause`]) derives, and its subject names the rule. The
/// derivation's sources are the matched body facts in the rule's AUTHORED body order, so
/// the witness's premises line up against the specification's own rule-table entry.
///
/// An INTERNAL source is dropped from the witness rather than rendered: `prp-adp`,
/// `cax-adc`, `eq-diff2`, `eq-diff3` and `dt-not-type` all match rows of this crate's
/// bookkeeping relations, and a row of `LIST(head, index, member)` is not an asserted
/// triple a caller can look for in their data. What remains is exactly the triples that
/// are.
fn first_clash(
    evaluation: &purrdf_datalog::seminaive::Evaluation,
    attribution: &[ChaseRule],
    terms: &Terms,
    regime: Regime,
) -> Option<InconsistencyWitness> {
    let owl = matches!(regime, Regime::OwlRl);
    evaluation
        .derivations()
        .iter()
        .find(|derivation| derivation.fact().predicate == CLASH_RELATION)
        .map(|derivation| witness_of(derivation, attribution, terms, owl))
}

/// The witness a clash derivation carries.
fn witness_of(
    derivation: &Derivation,
    attribution: &[ChaseRule],
    terms: &Terms,
    owl: bool,
) -> InconsistencyWitness {
    // The rule is read from the clash row's own subject where it names one, and from the
    // clause attribution otherwise; the two agree, and `a_clash_row_names_its_own_rule`
    // asserts so. Reading the marker first is what keeps the witness right even for a
    // rule stated as more than one clause.
    let rule = clash_rule(&derivation.fact().subject)
        .unwrap_or_else(|| attribution[derivation.rule()])
        .rule_id(owl);
    let premises = derivation
        .sources()
        .iter()
        .filter(|source| !is_internal(&source.predicate))
        .map(|source| {
            WitnessTriple::new(
                terms.value(&source.subject).clone(),
                terms.value(&source.predicate).clone(),
                terms.value(&source.object).clone(),
            )
        })
        .collect();
    // The chase reads and writes the default graph only, so a witness is always drawn
    // from it; `None` IS the default graph, and naming one would be inventing a graph the
    // premises did not come from.
    InconsistencyWitness::new(rule, premises, None)
}

/// Whether `value` may occupy a triple SUBJECT position in RDF 1.2 — an IRI or a blank
/// node, never a literal and never a triple term.
fn admits_subject(value: &TermValue) -> bool {
    matches!(value, TermValue::Iri(_) | TermValue::Blank { .. })
}

/// Whether `value` may occupy a triple PREDICATE position in RDF 1.2 — an IRI, and
/// nothing else.
///
/// Checked as well as the subject because a rule may conclude into predicate position
/// too: `rdfs7` / `prp-spo1` writes the OBJECT of a `rdfs:subPropertyOf` triple there, and
/// `prp-inv1` / `prp-inv2` write the object of an `owl:inverseOf` triple. Neither
/// specification requires that object to be an IRI, so a graph that declares
/// `p rdfs:subPropertyOf "cat"` licenses a conclusion the IR cannot hold, in exactly the
/// way a literal subject does.
fn admits_predicate(value: &TermValue) -> bool {
    matches!(value, TermValue::Iri(_))
}

/// The surface → value dictionary that lets an answer be read back as RDF 1.2 terms.
///
/// Every surface the store can ever hold is recorded here before evaluation begins: the
/// constants of the program ([`Self::record_program`]) and the terms of the seeded facts
/// ([`Self::record`]). The evaluator mints no terms, so those two sets are exhaustive and
/// [`Self::value`] is total — see the [module docs](self).
#[derive(Debug, Default)]
struct Terms {
    /// Surfaces to the values they were rendered from, in lexical surface order.
    by_surface: BTreeMap<String, TermValue>,
}

impl Terms {
    /// Record `value` and return its surface.
    fn record(&mut self, value: &TermValue) -> String {
        let surface = surface_of(value);
        if !self.by_surface.contains_key(&surface) {
            self.by_surface.insert(surface.clone(), value.clone());
        }
        surface
    }

    /// Record every constant term of `program`.
    ///
    /// Every constant this crate's calculus names is an IRI — PurRDF mints no vocabulary,
    /// and the rules quantify over data rather than comparing it against literals — which
    /// `every_clause_constant_is_an_iri` asserts over every declared program. A literal
    /// constant is therefore NOT handled here, and deliberately so: a
    /// [`ClauseTerm::Literal`] carries an already-rendered surface with no structure to
    /// recover a [`TermValue`] from, so guessing one would put a term in the dictionary
    /// that [`surface_of`] does not agree with. The rule that introduces the first literal
    /// constant has to give this module a way to read it back, and the test is what makes
    /// that a failure rather than a silent wrong term.
    fn record_program(&mut self, program: &[DlClause]) {
        for clause in program {
            for atom in clause.body().iter().chain(clause.head_atoms()) {
                for term in atom.terms() {
                    if let ClauseTerm::Iri(iri) = term {
                        let _ = self.record(&TermValue::iri(iri.clone()));
                    }
                }
            }
        }
    }

    /// Record every LITERAL constant the declared calculus names.
    ///
    /// [`DECLARED_LITERALS`] is the table, and it is a table rather than a walk over the
    /// program for the reason [`Self::record_program`] documents: a
    /// [`ClauseTerm::Literal`] is an already-rendered surface with no structure to recover
    /// a [`TermValue`] from, so the value has to be stated beside the rendering rather than
    /// guessed from it. `every_clause_literal_is_declared_or_internal` is what keeps the
    /// table exhaustive.
    fn record_literals(&mut self) {
        for (lexical, datatype) in DECLARED_LITERALS {
            let _ = self.record(&TermValue::typed_literal(lexical, datatype));
        }
    }

    /// The value behind a surface the store produced.
    ///
    /// # Panics
    ///
    /// Panics if the surface was never recorded. That is unreachable rather than merely
    /// unlikely: `compile` refuses a clause whose head carries a variable no positive body
    /// atom binds, and no declared clause is existential, so every term of every derived
    /// fact came from a seeded fact or from a program constant — and both were recorded
    /// before evaluation started.
    fn value(&self, surface: &str) -> &TermValue {
        self.by_surface.get(surface).unwrap_or_else(|| {
            panic!("the evaluator mints no terms, so {surface} must have been recorded")
        })
    }
}

/// The lexical surface `purrdf-datalog` interns `value` under.
///
/// This is the repository's canonical N-Quads term spelling — the same bytes
/// `purrdf_core::canonicalize` writes — with one deliberate difference: a blank node is
/// qualified by its scope ordinal, because C0.2 makes the scope part of the node's
/// identity while a canonical label is assigned by the canonicalization algorithm rather
/// than carried by the term.
///
/// # Injectivity
///
/// Distinct [`TermValue`]s render to distinct surfaces, which is what stops two terms
/// collapsing into one store term:
///
/// * the four kinds are told apart by their first byte — `<` for an IRI, `_` for a blank
///   node, `"` for a literal, and `<<(` for a triple term, whose second byte is a `<` no
///   IRI surface can carry because [`write_iri_escaped`] escapes `<` and `>`;
/// * an IRI's surface is its escaped text bracketed once, and the escape is injective;
/// * a blank node's scope is decimal digits terminated by the `.` that no digit can be,
///   and the label is the verbatim remainder;
/// * a literal's lexical form is escaped so it carries no bare `"`, so the closing quote
///   is unambiguous, and what follows is either `@` (a language tag, hence the datatype
///   `rdf:langString` by C0.1) or `^^<` (a datatype IRI) or nothing (`xsd:string`);
/// * a triple term's three components are separated by the spaces its delimiters reserve,
///   and each recurses through the same argument.
fn surface_of(value: &TermValue) -> String {
    let mut out = String::new();
    write_surface(value, &mut out);
    out
}

/// Append [`surface_of`]'s rendering of `value` to `out`.
fn write_surface(value: &TermValue, out: &mut String) {
    match value {
        TermValue::Iri(iri) => {
            out.push('<');
            write_iri_escaped(iri, out);
            out.push('>');
        }
        TermValue::Blank { label, scope } => {
            let _ = write!(out, "_:{}.{label}", scope.ordinal());
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            out.push('"');
            write_literal_escaped(lexical_form, out);
            out.push('"');
            if let Some(language) = language {
                // A language-tagged literal's datatype is `rdf:langString` by C0.1 — the
                // builder re-derives it — so the tag determines it and spelling it out
                // would add bytes that carry no identity.
                out.push('@');
                out.push_str(language);
                if let Some(direction) = direction {
                    out.push_str("--");
                    out.push_str(direction.as_str());
                }
            } else if datatype != XSD_STRING {
                out.push_str("^^<");
                write_iri_escaped(datatype, out);
                out.push('>');
            }
        }
        TermValue::Triple { s, p, o } => {
            out.push_str("<<( ");
            write_surface(s, out);
            out.push(' ');
            write_surface(p, out);
            out.push(' ');
            write_surface(o, out);
            out.push_str(" )>>");
        }
    }
}

/// Escape an IRI for a `<…>` surface, matching canonical N-Quads.
///
/// Every character the IRIREF grammar forbids becomes a `\uXXXX` escape, so no IRI's
/// surface can carry a bare `<` or `>` — which is what keeps a bracketed IRI and a
/// `<<( … )>>` triple term apart. A spec `rdf:`/`rdfs:`/`owl:` IRI contains none of them,
/// so a clause constant's surface is its plain bracketed text.
fn write_iri_escaped(iri: &str, out: &mut String) {
    for ch in iri.chars() {
        match ch {
            c if c.is_control() || c == ' ' => write_u_escape(c, out),
            '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\' => write_u_escape(ch, out),
            _ => out.push(ch),
        }
    }
}

/// Escape a literal's lexical form for a `"…"` surface, matching canonical N-Quads.
fn write_literal_escaped(value: &str, out: &mut String) {
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => write_u_escape(c, out),
            c => out.push(c),
        }
    }
}

/// Write `\uXXXX` (or `\UXXXXXXXX` beyond the BMP) for `ch`.
fn write_u_escape(ch: char, out: &mut String) {
    let cp = ch as u32;
    if cp <= 0xFFFF {
        let _ = write!(out, "\\u{cp:04X}");
    } else {
        let _ = write!(out, "\\U{cp:08X}");
    }
}

#[cfg(test)]
mod tests {
    use super::{admits_predicate, admits_subject, close, surface_of};
    use crate::Regime;
    use crate::calculus::ALL_REGIMES;
    use crate::calculus_program;
    use crate::vocab::{
        OWL_HASKEY, OWL_INTERSECTIONOF, OWL_PROPERTYCHAINAXIOM, OWL_UNIONOF, RDF_FIRST, RDF_NIL,
        RDF_REST, RDF_TYPE, RDFS_SUBCLASSOF, XSD_STRING,
    };
    use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfTextDirection, TermValue};
    use purrdf_datalog::clause::{ClauseTerm, DlClause};
    use purrdf_datalog::store::RelationStore;
    use std::collections::BTreeSet;

    /// A fixture IRI. PurRDF mints no vocabulary, so every fixture term is `example.org`.
    const EX_S: &str = "http://example.org/s";
    /// A fixture predicate IRI.
    const EX_P: &str = "http://example.org/p";
    /// A fixture object IRI.
    const EX_O: &str = "http://example.org/o";
    /// A fixture class, and the subject of the intersection axiom.
    const EX_C: &str = "http://example.org/C";
    /// A fixture class, and the first member of both collections.
    const EX_A: &str = "http://example.org/A";
    /// A fixture class, and the second member of both collections.
    const EX_B: &str = "http://example.org/B";
    /// A fixture class, and the subject of the union axiom.
    const EX_D: &str = "http://example.org/D";
    /// The first collection cell.
    const EX_L0: &str = "http://example.org/l0";
    /// The second collection cell.
    const EX_L1: &str = "http://example.org/l1";
    /// The first cell of the chain list.
    const EX_L2: &str = "http://example.org/l2";
    /// The second cell of the chain list.
    const EX_L3: &str = "http://example.org/l3";
    /// The single cell of the key list.
    const EX_L4: &str = "http://example.org/l4";
    /// The property a chain axiom composes into.
    const EX_CHAINED: &str = "http://example.org/chained";
    /// The first property of the chain.
    const EX_Q: &str = "http://example.org/q";
    /// The second property of the chain.
    const EX_R: &str = "http://example.org/r";
    /// A fixture individual.
    const EX_X: &str = "http://example.org/x";
    /// A fixture individual.
    const EX_Y: &str = "http://example.org/y";
    /// A fixture individual.
    const EX_Z: &str = "http://example.org/z";
    /// A fixture individual.
    const EX_W: &str = "http://example.org/w";
    /// A fixture individual.
    const EX_V: &str = "http://example.org/v";

    /// Freeze `triples` into a default-graph dataset.
    fn dataset_of(triples: &[(&str, &str, &str)]) -> std::sync::Arc<purrdf_core::RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for &(s, p, o) in triples {
            let s = b.intern_iri(s);
            let p = b.intern_iri(p);
            let o = b.intern_iri(o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("the fixture freezes")
    }

    /// A dataset that reaches every internal relation this crate has: an intersection and
    /// a union list (`scm-int`, `scm-uni` — the `LIST` relation), a property chain
    /// (`prp-spo2` — `CHAIN`) and a key (`prp-key` — `AGREE`).
    fn collection_fixture() -> std::sync::Arc<purrdf_core::RdfDataset> {
        dataset_of(&[
            // C owl:intersectionOf (A B) and D owl:unionOf (A B), sharing one list.
            (EX_C, OWL_INTERSECTIONOF, EX_L0),
            (EX_D, OWL_UNIONOF, EX_L0),
            (EX_L0, RDF_FIRST, EX_A),
            (EX_L0, RDF_REST, EX_L1),
            (EX_L1, RDF_FIRST, EX_B),
            (EX_L1, RDF_REST, RDF_NIL),
            // chained owl:propertyChainAxiom (q r), with the path x q y r z.
            (EX_CHAINED, OWL_PROPERTYCHAINAXIOM, EX_L2),
            (EX_L2, RDF_FIRST, EX_Q),
            (EX_L2, RDF_REST, EX_L3),
            (EX_L3, RDF_FIRST, EX_R),
            (EX_L3, RDF_REST, RDF_NIL),
            (EX_X, EX_Q, EX_Y),
            (EX_Y, EX_R, EX_Z),
            // C owl:hasKey (r), with two C-instances agreeing on r.
            (EX_C, OWL_HASKEY, EX_L4),
            (EX_L4, RDF_FIRST, EX_R),
            (EX_L4, RDF_REST, RDF_NIL),
            (EX_X, RDF_TYPE, EX_C),
            (EX_W, RDF_TYPE, EX_C),
            (EX_X, EX_R, EX_V),
            (EX_W, EX_R, EX_V),
        ])
    }

    /// A triple term over three IRIs, by value.
    fn quoted(s: &str, p: &str, o: &str) -> TermValue {
        TermValue::Triple {
            s: Box::new(TermValue::iri(s)),
            p: Box::new(TermValue::iri(p)),
            o: Box::new(TermValue::iri(o)),
        }
    }

    /// A clause constant IRI renders to the SAME surface the store will hold for the
    /// dataset term of that IRI — the property that lets a rule constant join against
    /// data at all.
    #[test]
    fn a_clause_constant_and_a_dataset_iri_share_one_surface() {
        assert_eq!(
            surface_of(&TermValue::iri(RDF_TYPE)),
            ClauseTerm::iri(RDF_TYPE)
                .surface()
                .expect("a constant has a surface")
        );
    }

    /// The default graph is the empty surface on both sides of the bridge, so a
    /// default-graph atom addresses the partition the seeded quads went into.
    #[test]
    fn the_default_graph_is_the_empty_surface_on_both_sides() {
        assert_eq!(RelationStore::DEFAULT_GRAPH, "");
        assert_eq!(
            ClauseTerm::DefaultGraph.surface().as_deref(),
            Some(RelationStore::DEFAULT_GRAPH)
        );
    }

    /// [`surface_of`] is injective over terms that differ in ANY identity coordinate,
    /// including the ones a careless rendering drops: a blank node's scope, a literal's
    /// datatype, its language tag and its base direction.
    #[test]
    fn distinct_terms_render_to_distinct_surfaces() {
        let terms = [
            TermValue::iri(EX_S),
            TermValue::iri(EX_O),
            // An IRI whose text is the surface of a triple term: the escape is what
            // stops it colliding with one.
            TermValue::iri("<<( a b c )>>"),
            TermValue::Blank {
                label: "b0".to_owned(),
                scope: BlankScope::DEFAULT,
            },
            TermValue::Blank {
                label: "b0".to_owned(),
                scope: BlankScope(7),
            },
            // A label that itself carries the scope separator.
            TermValue::Blank {
                label: "0.b0".to_owned(),
                scope: BlankScope::DEFAULT,
            },
            TermValue::simple_literal("cat"),
            TermValue::simple_literal("cat\"@en"),
            TermValue::typed_literal("cat", "http://example.org/dt"),
            TermValue::lang_literal("cat", "en"),
            TermValue::Literal {
                lexical_form: "cat".to_owned(),
                datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                language: Some("en".to_owned()),
                direction: Some(RdfTextDirection::Ltr),
            },
            TermValue::Literal {
                lexical_form: "cat".to_owned(),
                datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                language: Some("en".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            },
            quoted(EX_S, EX_P, EX_O),
            quoted(EX_O, EX_P, EX_S),
        ];
        let surfaces: BTreeSet<String> = terms.iter().map(surface_of).collect();
        assert_eq!(
            surfaces.len(),
            terms.len(),
            "two distinct terms share a surface: {surfaces:?}"
        );
        // A plain `xsd:string` literal carries no datatype suffix, exactly as canonical
        // N-Quads writes it — so `simple_literal` and an explicit `xsd:string` literal
        // are ONE term, which is what RDF 1.1 says they are.
        assert_eq!(
            surface_of(&TermValue::simple_literal("cat")),
            surface_of(&TermValue::typed_literal("cat", XSD_STRING))
        );
    }

    /// The positional guards admit exactly what RDF 1.2 admits.
    #[test]
    fn the_positional_guards_match_rdf_12() {
        let iri = TermValue::iri(EX_S);
        let blank = TermValue::blank("b0");
        let literal = TermValue::simple_literal("cat");
        let triple = quoted(EX_S, EX_P, EX_O);
        assert!(admits_subject(&iri) && admits_predicate(&iri));
        assert!(admits_subject(&blank), "a blank node is a legal subject");
        assert!(
            !admits_predicate(&blank),
            "a blank predicate is generalized"
        );
        assert!(!admits_subject(&literal) && !admits_predicate(&literal));
        assert!(!admits_subject(&triple) && !admits_predicate(&triple));
    }

    /// EVERY literal constant of EVERY declared clause is one the surface dictionary can
    /// read BACK, or an internal id that never has to be.
    ///
    /// [`super::Terms`] records IRI constants by walking the program and literal constants
    /// from [`DECLARED_LITERALS`], because a [`ClauseTerm::Literal`] is an already-rendered
    /// surface with no structure to recover a [`TermValue`] from. Two kinds of literal
    /// constant are legitimate and this asserts each against its own condition:
    ///
    /// * an INTERNAL id — a relation name of [`crate::lists`], a list index, a clash
    ///   marker — which is carried in a `ClauseTerm::Literal` because the IR has no fifth
    ///   term kind, and which is never read back at all: every conclusion whose predicate
    ///   is internal is dropped before materialization, and a clash refuses the run;
    /// * a SPECIFICATION literal — the two cardinality literals OWL 2 Profiles Table 6
    ///   writes — which IS read back, and must therefore be in the recorded dictionary.
    ///
    /// A rule that names any other literal fails here rather than panicking in
    /// [`super::Terms::value`] on some input that happens to reach it.
    #[test]
    fn every_clause_literal_is_declared_or_internal() {
        let mut terms = super::Terms::default();
        terms.record_literals();
        let mut declared = 0_usize;
        let mut internal = 0_usize;
        for regime in ALL_REGIMES {
            for clause in calculus_program(regime) {
                for atom in clause.body().iter().chain(clause.head_atoms()) {
                    for term in atom.terms() {
                        let ClauseTerm::Literal(surface) = term else {
                            continue;
                        };
                        if crate::lists::is_internal(surface) {
                            internal += 1;
                            continue;
                        }
                        assert!(
                            terms.by_surface.contains_key(surface),
                            "{regime:?}: a clause names the literal constant {surface:?}, \
                             which the surface dictionary cannot read back"
                        );
                        declared += 1;
                    }
                }
            }
        }
        assert!(internal > 0 && declared > 0, "both cases must be exercised");
        // Every declared literal is actually USED; an entry nothing names is a table that
        // has outlived its rule.
        for (lexical, datatype) in super::DECLARED_LITERALS {
            let surface = super::literal_surface(lexical, datatype);
            assert!(
                calculus_program(Regime::OwlRl).iter().any(|clause| {
                    clause.body().iter().chain(clause.head_atoms()).any(|atom| {
                        atom.terms()
                            .iter()
                            .any(|term| matches!(term, ClauseTerm::Literal(s) if *s == surface))
                    })
                }),
                "{surface:?} is declared and named by no rule"
            );
        }
    }

    /// EVERY atom over SPEC vocabulary names the default graph.
    ///
    /// [`super::close`] seeds the default graph alone and emits every conclusion into it;
    /// this is the statement that makes that a faithful evaluation of the program rather
    /// than a silent restriction of it. A rule that later reasons per-graph fails here,
    /// which is the signal to teach the seeding and the emission about graphs.
    ///
    /// An INTERNAL relation's atom is excluded, and the exclusion is the point rather than
    /// a hole: its fourth position is not a graph at all but the relation's third argument
    /// — a `ClauseAtom` is four terms and an internal ternary relation needs three of them
    /// beside the predicate. See [`crate::lists`] for the convention. The test still ranges
    /// over EVERY atom; it just asks the right question of each kind.
    #[test]
    fn the_declared_programs_read_and_write_the_default_graph_only() {
        let mut internal_atoms = 0_usize;
        for regime in ALL_REGIMES {
            let program: Vec<DlClause> = calculus_program(regime);
            for clause in &program {
                for atom in clause.body().iter().chain(clause.head_atoms()) {
                    let internal = atom
                        .predicate()
                        .surface()
                        .is_some_and(|surface| crate::lists::is_internal(&surface));
                    if internal {
                        internal_atoms += 1;
                        continue;
                    }
                    assert!(
                        atom.graph().is_default_graph(),
                        "{regime:?}: an atom names the graph {:?}",
                        atom.graph()
                    );
                }
            }
        }
        assert!(
            internal_atoms > 0,
            "the exclusion above must be exercised, or it is not a statement about \
             anything"
        );
    }

    /// NO INTERNAL ID REACHES A SERIALIZED CLOSURE.
    ///
    /// The list pre-pass and the two traversal rules put interner-local ids in the fact
    /// store — a relation name, a list index, and the rows the traversals accumulate — and
    /// none of them is an RDF term. This is the assertion that they cannot escape: an
    /// `OWL-RL` closure over a graph that exercises `scm-int`, `scm-uni`, `prp-spo2` and
    /// `prp-key` is canonicalized, and no byte of the result may carry the internal sigil
    /// or any internal relation's name.
    ///
    /// It is asserted over the SERIALIZED form rather than over the dataset's term table
    /// because that is what a caller sees; a term that reached the table but never a quad
    /// would still be a defect, and `no_term_of_the_closure_is_internal` below covers the
    /// table.
    #[test]
    fn no_internal_id_reaches_a_serialized_closure() {
        let closed = close(&collection_fixture(), Regime::OwlRl)
            .expect("the fixture's collections are well formed")
            .0;
        let nquads = purrdf_core::canonicalize(&closed).nquads;
        assert!(
            !nquads.contains(crate::lists::INTERNAL_SIGIL),
            "an internal id reached the serialized closure:\n{nquads}"
        );
        for relation in crate::lists::INTERNAL_RELATIONS {
            assert!(
                !nquads.contains(relation),
                "{relation:?} reached the output"
            );
        }
        // The fixture really does exercise the internal machinery: without these the test
        // would pass over a closure that never had an internal id to leak.
        assert!(
            nquads.contains(&format!("<{EX_C}> <{RDFS_SUBCLASSOF}> <{EX_A}> .")),
            "scm-int did not read the intersection list:\n{nquads}"
        );
        assert!(
            nquads.contains(&format!("<{EX_X}> <{EX_CHAINED}> <{EX_Z}> .")),
            "prp-spo2 did not walk the property chain:\n{nquads}"
        );
    }

    /// The same claim over the closure's TERM TABLE: not one term of the dataset is an
    /// internal id, whether or not a quad mentions it.
    #[test]
    fn no_term_of_the_closure_is_internal() {
        let closed = close(&collection_fixture(), Regime::OwlRl)
            .expect("the fixture's collections are well formed")
            .0;
        for quad in closed.quads() {
            for term in [quad.s, quad.p, quad.o] {
                let surface = surface_of(&closed.term_value(term));
                assert!(!crate::lists::is_internal(&surface), "{surface:?}");
            }
        }
    }

    /// A malformed collection an OWL axiom points at is a HARD ERROR, not a partial answer.
    #[test]
    fn a_malformed_collection_refuses_the_run() {
        // …and no rdf:rest, so the cell is not a collection cell.
        let ds = dataset_of(&[(EX_C, OWL_INTERSECTIONOF, EX_L0), (EX_L0, RDF_FIRST, EX_A)]);
        let error = close(&ds, Regime::OwlRl).expect_err("a malformed collection is refused");
        let rendered = error.to_string();
        assert!(rendered.contains("carries no rdf:rest"), "{rendered}");
        assert!(rendered.contains(EX_L0), "{rendered}");
        // The RDFS lane says nothing about `owl:intersectionOf`, so the same graph is
        // ordinary data there and closes without complaint.
        assert!(close(&ds, Regime::Rdfs).is_ok());
    }

    /// A CYCLIC collection terminates with a refusal rather than hanging.
    #[test]
    fn a_cyclic_collection_refuses_the_run_rather_than_hanging() {
        let ds = dataset_of(&[
            (EX_C, OWL_INTERSECTIONOF, EX_L0),
            (EX_L0, RDF_FIRST, EX_A),
            (EX_L0, RDF_REST, EX_L1),
            (EX_L1, RDF_FIRST, EX_B),
            (EX_L1, RDF_REST, EX_L0),
        ]);
        let error = close(&ds, Regime::OwlRl).expect_err("a cycle is refused");
        assert!(error.to_string().contains("cyclic"), "{error}");
    }
}
