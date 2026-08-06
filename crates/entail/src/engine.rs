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
//! # A dataset is closed graph by graph, and that is a DEFINED choice
//!
//! RDF has no standard entailment relation for a *dataset*: RDF 1.2 Semantics defines
//! entailment over a graph, and SPARQL's entailment regimes are defined over the active
//! graph. A reasoner handed a dataset therefore has to choose, and the choice PurRDF makes
//! is stated here and reported as the [`Construct::NamedGraph`](crate::Construct::NamedGraph)
//! boundary on every run whose input has a named graph:
//!
//! * the DEFAULT graph is closed against itself;
//! * each NAMED graph is closed against the union of itself and the default graph;
//! * a conclusion lands in the graph that produced it — a conclusion the default graph
//!   already draws on its own is a default-graph conclusion and is not restated in the
//!   named graph that also reached it.
//!
//! That is what makes the layout every real dataset uses work: a terminology in the default
//! graph and instances in a named graph derive the expected triples INTO the named graph.
//! Two named graphs never join, because neither is ever in the other's seed.
//!
//! The declared clause program is untouched by this — every atom over specification
//! vocabulary still names
//! [`ClauseTerm::DefaultGraph`](purrdf_datalog::clause::ClauseTerm::DefaultGraph), which
//! `the_declared_programs_read_and_write_the_default_graph_only` still asserts. What varies
//! is the SEED: [`close_graph`] fills the store's default partition with the union it is
//! closing, runs the graph-agnostic program over it, and [`close`] routes the answer back
//! to the graph that produced it. One statement of the calculus, `1 + n` evaluations of it.
//!
//! # Determinism
//!
//! Derivations arrive in `purrdf-datalog`'s total order — lexical by `(fact, rule,
//! sources)` — and are emitted in that order, so the derived quads reach the builder in a
//! sequence that is a function of the fact set alone. [`Terms`] is a `BTreeMap`, so no
//! hash iteration reaches anything either. The named graphs are visited in ascending
//! [`surface_of`] order — a total order over term VALUES, not over interned ids — so the
//! emission sequence is a function of the dataset's content alone.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermId, TermValue};
use purrdf_datalog::cache::PlanCache;
use purrdf_datalog::chase::{ChaseError, chase_until};
use purrdf_datalog::clause::{ClauseTerm, DlClause, HeadForm};
use purrdf_datalog::seminaive::{Derivation, EvalError, evaluate_until};
use purrdf_datalog::stop::StopSignal;
use purrdf_datalog::store::RelationStore;

use crate::axioms::axioms_for;
use crate::calculus::{ChaseRule, calculus_contract_hash, clash_rule, program_with_attribution};
use crate::datatypes::LiteralIndex;
use crate::interner::intern_into;
use crate::lists::{CLASH_RELATION, ListIndex, is_internal};
use crate::report::{InconsistencyWitness, InconsistentRun, ReasoningReport, WitnessTriple};
use crate::report::{RunStats, TerminationCertificate};
use crate::surrogates::SurrogateIndex;
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
    copy_into(&mut b, ds);
    b.freeze().map_err(|e| EntailError::Build(e.to_string()))
}

/// Copy every quad, side-table row and graph declaration of `ds` into `b`, PRESERVING
/// blank-node scopes.
///
/// # Why not `push_dataset`
///
/// [`RdfDatasetBuilder::push_dataset`] is the standardize-apart primitive: it allocates a
/// FRESH [`BlankScope`](purrdf_core::BlankScope) for the dataset it merges, so `_:b` from
/// two sources cannot collide. That is exactly right for a MERGE and exactly wrong here,
/// because a closure is not a merge of two datasets — it is one dataset plus conclusions
/// ABOUT it, and those conclusions name the very blank nodes the copy just re-scoped.
/// Interning a conclusion's `_:b` through [`intern_into`] would then mint a SECOND node with
/// the same label and the original scope, so the closure would carry two blank nodes where
/// the input had one: the copied triples about the first, the inferred triples about the
/// second, and nothing joining them. A blank-node GRAPH NAME is the sharpest case — the
/// conclusions of a named graph's own closure would land in a graph the input does not have
/// — and `a_blank_node_graph_name_survives_the_closure` is the assertion that they do not.
///
/// There is nothing to standardize apart from: one source dataset, whose scopes are already
/// internally consistent, and a set of conclusions drawn over its own terms.
///
/// The reifier and annotation SIDE TABLES ride along, because a closure that silently
/// dropped them would delete every reifier in a caller's data the moment they asked for
/// entailment, and no assertion about the quads would notice.
pub(crate) fn copy_into(b: &mut RdfDatasetBuilder, ds: &RdfDataset) {
    for quad in ds.quads() {
        let s = intern_into(b, &ds.term_value(quad.s));
        let p = intern_into(b, &ds.term_value(quad.p));
        let o = intern_into(b, &ds.term_value(quad.o));
        let g = quad.g.map(|g| intern_into(b, &ds.term_value(g)));
        b.push_quad(s, p, o, g);
    }
    for (reifier, triple, graph) in ds.reifiers_with_graph() {
        let reifier = intern_into(b, &ds.term_value(reifier));
        let triple = intern_into(b, &ds.term_value(triple));
        let graph = graph.map(|g| intern_into(b, &ds.term_value(g)));
        b.push_reifier_in_graph(reifier, triple, graph);
    }
    for (reifier, predicate, object, graph) in ds.annotations_with_graph() {
        let reifier = intern_into(b, &ds.term_value(reifier));
        let predicate = intern_into(b, &ds.term_value(predicate));
        let object = intern_into(b, &ds.term_value(object));
        let graph = graph.map(|g| intern_into(b, &ds.term_value(g)));
        b.push_annotation_in_graph(reifier, predicate, object, graph);
    }
    // A named graph a dataset DECLARES but puts no quad in is part of its content, and a
    // closure that dropped the declaration would answer a different question about which
    // graphs exist.
    for graph in ds.named_graphs() {
        let graph = intern_into(b, &ds.term_value(graph));
        b.declare_named_graph(graph);
    }
}

/// Every term id `ds` holds, in EVERY position [`copy_into`] writes.
///
/// # Why this is one function and not a survey per caller
///
/// A dataset's content is not its quads. A reifier, an annotation and a declared named
/// graph each name terms that occur in no quad at all, and [`copy_into`] carries all four
/// tables because dropping any of them would silently delete part of a caller's data.
///
/// Anything that has to REASON about the whole term space — "what is the highest blank-node
/// scope in here", "which blank-node labels are taken" — has to survey the same four
/// tables, and a survey that read only `quads()` would be right about every dataset whose
/// side tables happen to mention nothing new and wrong about the ones that do. Both such
/// surveys previously carried that bug independently, so the enumeration lives HERE, beside
/// the copy whose coverage it has to match: a position added to [`copy_into`] and not to
/// this iterator is a visible omission in one file rather than a silent one in three.
///
/// Ids repeat (the same term occurs in many positions) and triple terms are NOT unfolded —
/// a caller that cares about nesting resolves the id and walks the [`TermValue`] itself.
pub(crate) fn term_positions(ds: &RdfDataset) -> impl Iterator<Item = TermId> + '_ {
    let quads = ds.quads().flat_map(|quad| {
        [Some(quad.s), Some(quad.p), Some(quad.o), quad.g]
            .into_iter()
            .flatten()
    });
    let reifiers = ds
        .reifiers_with_graph()
        .flat_map(|(reifier, triple, graph)| {
            [Some(reifier), Some(triple), graph].into_iter().flatten()
        });
    let annotations =
        ds.annotations_with_graph()
            .flat_map(|(reifier, predicate, object, graph)| {
                [Some(reifier), Some(predicate), Some(object), graph]
                    .into_iter()
                    .flatten()
            });
    quads
        .chain(reifiers)
        .chain(annotations)
        .chain(ds.named_graphs())
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
/// # A dataset is closed graph by graph
///
/// See the [module docs](self) for the defined semantics and for why it has to BE a defined
/// choice: the default graph is closed against itself, each named graph against the union
/// of itself and the default graph, and a conclusion lands in the graph that produced it.
/// A dataset holding `n` named graphs therefore costs `1 + n` evaluations of the same
/// declared program, which is a real multiplication of the work and is REPORTED —
/// [`RunStats::absorb`] sums the join steps across the runs and takes the peak of the two
/// occupancy coordinates, so a caller reads the total enumeration and the worst single
/// store rather than one lane's slice of either.
///
/// It is `1 + n` EVALUATIONS and exactly ONE compilation. The plan those clauses compile
/// to is a pure function of the clauses, so a `PlanCache` built here and threaded through
/// every [`close_graph`] call turns `1 + n` planning passes over a ~200-clause calculus
/// into one. The cache is scoped to this call and passed by `&mut`: a longer-lived one
/// would make what a run costs — and what it computes, if a compile were ever a function
/// of anything but the clauses — depend on which runs came before it.
///
/// The clause program itself is graph-agnostic: every atom over SPEC vocabulary names
/// [`ClauseTerm::DefaultGraph`](purrdf_datalog::clause::ClauseTerm::DefaultGraph), which
/// `the_declared_programs_read_and_write_the_default_graph_only` asserts, and each run
/// seeds that one partition with the union it is closing. The atoms of the INTERNAL
/// relations ([`crate::lists`]) use that fourth position for the relation's third argument
/// instead, which is not a graph at all and never reaches the answer.
///
/// # The collections are walked before the clauses run
///
/// The `OWL-RL` lane's rule table writes `LIST[…]`, a meta-notation no clause has, so each
/// run walks each RDF collection an OWL axiom points at into an internal relation before
/// evaluating. A malformed or cyclic collection is [`EntailError::MalformedList`] rather
/// than a closure over its well-formed prefix.
pub(crate) fn close(
    ds: &RdfDataset,
    regime: Regime,
    stop: Option<&Arc<dyn StopSignal>>,
) -> Result<(Arc<RdfDataset>, RunStats), EntailError> {
    let (program, attribution) = program_with_attribution(regime);

    // The named graphs, keyed by their canonical surface so the visit order is a function
    // of the dataset's CONTENT rather than of the order its quads happened to intern in.
    let mut named: BTreeMap<String, TermValue> = BTreeMap::new();
    for quad in ds.quads() {
        if let Some(graph) = quad.g {
            let value = ds.term_value(graph);
            named.entry(surface_of(&value)).or_insert(value);
        }
    }

    // ONE compilation per calculus per CALL. `close_graph` is invoked `1 + n` times for a
    // dataset with `n` named graphs, over the same declared program every time, so
    // compiling inside it made a run over a hundred named graphs plan the ~200-clause
    // OWL-RL calculus a hundred and one times for a plan that is a pure function of the
    // clauses. The cache is CALL-SCOPED and threaded by `&mut`: a longer-lived one would
    // make an answer's cost — and, if a compile ever became fallible on state, an answer —
    // depend on what some earlier evaluation happened to compile, which is exactly the
    // hidden history `purrdf-datalog` refuses to keep. Capacity two, because one call
    // presents exactly one program and the second slot is slack rather than a policy.
    let mut plans = PlanCache::new(2);
    let mut stats = RunStats::none();
    let default_run = close_graph(ds, regime, &program, &attribution, None, &mut plans, stop)?;
    stats.absorb(default_run.budget);
    stats.drop_generalized(default_run.generalized_rdf_drops);
    stats.drop_surrogate(default_run.surrogate_drops);
    stats.certify(default_run.termination);
    if let Some(witness) = default_run.clash {
        return Err(refuse(ds, regime, &stats, witness));
    }

    let mut b = RdfDatasetBuilder::new();
    copy_into(&mut b, ds);
    // What the DEFAULT graph draws on its own. A named-graph run re-derives every one of
    // these — the default graph is in its seed — and restating them inside the named graph
    // would put a conclusion in a graph that did not produce it.
    let mut default_conclusions: BTreeSet<(String, String, String)> = BTreeSet::new();
    for conclusion in &default_run.conclusions {
        default_conclusions.insert(conclusion.key());
        stats.commit(conclusion.rule);
        emit(&mut b, conclusion, None);
    }
    for graph in named.values() {
        // A dataset is closed graph by graph, so the loop over the named graphs is a
        // boundary of the CLOSURE just as a fixpoint round is — and it is the only one that
        // sees the copying and emission between two evaluations. Polling here is what stops
        // a hundred-graph dataset from having ninety-nine unpollable seams.
        if stop.is_some_and(|stop| stop.stopped()) {
            return Err(EntailError::Stopped);
        }
        let run = close_graph(
            ds,
            regime,
            &program,
            &attribution,
            Some(graph),
            &mut plans,
            stop,
        )?;
        stats.absorb(run.budget);
        stats.drop_generalized(run.generalized_rdf_drops);
        stats.drop_surrogate(run.surrogate_drops);
        stats.certify(run.termination);
        if let Some(witness) = run.clash {
            return Err(refuse(ds, regime, &stats, witness));
        }
        let g = intern_into(&mut b, graph);
        for conclusion in &run.conclusions {
            if default_conclusions.contains(&conclusion.key()) {
                continue;
            }
            stats.commit(conclusion.rule);
            emit(&mut b, conclusion, Some(g));
        }
    }
    let dataset = b.freeze().map_err(|e| EntailError::Build(e.to_string()))?;
    Ok((dataset, stats))
}

/// The semi-naive evaluator's refusal, wrapped in this crate's vocabulary.
///
/// A STOPPED run is not an evaluation failure and is not reported as one: it says the host
/// ended the run, which is a fact about the caller rather than about the program or the
/// data, so it keeps its own variant all the way out. Every other refusal is what
/// [`EntailError::Evaluate`] has always meant.
fn evaluate_error(error: EvalError) -> EntailError {
    match error {
        EvalError::Stopped { .. } => EntailError::Stopped,
        other => EntailError::Evaluate(other),
    }
}

/// The restricted chase's refusal, wrapped in this crate's vocabulary. See
/// [`evaluate_error`] for why a stop keeps its own variant.
fn chase_error(error: ChaseError) -> EntailError {
    match error {
        ChaseError::Stopped { .. } => EntailError::Stopped,
        other => EntailError::Chase(other),
    }
}

/// The refusal an inconsistent run owes its caller: the witness AND the run's report.
///
/// `stats` is everything the run had measured when it stopped — the graphs already closed
/// plus the budget of the one that clashed — so the report is a description of work that
/// actually happened rather than a placeholder shaped like one. A dataset is closed graph by
/// graph, so a clash in the second named graph leaves the default graph's conclusions
/// tallied in `rules_fired` and both evaluations' join steps summed into the budget.
fn refuse(
    ds: &RdfDataset,
    regime: Regime,
    stats: &RunStats,
    witness: InconsistencyWitness,
) -> EntailError {
    let report = ReasoningReport::of_inconsistent_run(ds, regime, stats, witness.clone());
    EntailError::Inconsistent(Box::new(InconsistentRun::new(witness, report)))
}

/// One conclusion a graph's run drew, already known representable in RDF 1.2.
#[derive(Debug, Clone)]
struct Conclusion {
    /// The rule the derivation is credited to.
    rule: ChaseRule,
    /// The subject term.
    subject: TermValue,
    /// The predicate term.
    predicate: TermValue,
    /// The object term.
    object: TermValue,
}

impl Conclusion {
    /// The triple's identity, as the three canonical surfaces.
    ///
    /// A surface triple rather than the [`TermValue`]s themselves because [`surface_of`] is
    /// injective (see its own documentation) and a `String` triple is `Ord` without asking
    /// `TermValue` to be.
    fn key(&self) -> (String, String, String) {
        (
            surface_of(&self.subject),
            surface_of(&self.predicate),
            surface_of(&self.object),
        )
    }
}

/// What closing one graph produced and consumed.
#[derive(Debug)]
struct GraphRun {
    /// The conclusions, in the evaluator's own total derivation order.
    conclusions: Vec<Conclusion>,
    /// What this evaluation consumed of the three fixed ceilings.
    budget: purrdf_datalog::seminaive::BudgetReport,
    /// Conclusions this run abandoned because the RDF 1.2 IR cannot hold them.
    generalized_rdf_drops: u64,
    /// Conclusions this run withheld because they mention a surrogate blank node.
    surrogate_drops: u64,
    /// The clash this graph's evaluation witnessed, if it witnessed one.
    ///
    /// A refusal carried as a VALUE rather than raised on the spot, because the report the
    /// caller is owed is assembled by [`close`] out of every graph's measurements and this
    /// graph's budget is one of them. Raising from here would hand the one caller who most
    /// needs the certificate — the one whose data is inconsistent — a bare witness and no
    /// run. `conclusions` is empty when this is `Some`: an inconsistent knowledge base
    /// entails every triple, so nothing the evaluation derived is an answer.
    clash: Option<InconsistencyWitness>,
    /// The proof that admitted the program, when the restricted chase evaluated it.
    ///
    /// `None` on the semi-naive path, which invents no term and therefore has no
    /// termination obligation to discharge. `Some` on the chase path, where the analysis
    /// runs before the first round and refusing it is the only reason a program is
    /// rejected for termination — so a `GraphRun` that exists always carries a certificate
    /// that says the program is weakly acyclic.
    termination: Option<TerminationCertificate>,
}

/// Push `conclusion` into `b`, in `graph`.
fn emit(b: &mut RdfDatasetBuilder, conclusion: &Conclusion, graph: Option<TermId>) {
    let s = intern_into(b, &conclusion.subject);
    let p = intern_into(b, &conclusion.predicate);
    let o = intern_into(b, &conclusion.object);
    b.push_quad(s, p, o, graph);
}

/// Close ONE graph of `ds` under `regime`'s declared calculus.
///
/// `graph` is the graph being closed: `None` is the default graph, closed against itself;
/// `Some(g)` is the named graph `g`, closed against the union of itself and the default
/// graph. The union is what is SEEDED — the store's default partition holds it — so the
/// graph-agnostic program is evaluated unchanged and [`close`] routes the answer back.
///
/// # The axiomatic triples are seeded, not concluded
///
/// [`crate::axioms`]'s finite table is inserted into the fact store beside the graph's own
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
fn close_graph(
    ds: &RdfDataset,
    regime: Regime,
    program: &[DlClause],
    attribution: &[ChaseRule],
    graph: Option<&TermValue>,
    plans: &mut PlanCache,
    stop: Option<&Arc<dyn StopSignal>>,
) -> Result<GraphRun, EntailError> {
    let (edb, terms) = seed(ds, regime, program, graph)?;

    // A lane whose calculus states an EXISTENTIAL rule is evaluated by the restricted
    // chase; every other lane keeps the semi-naive evaluator, which refuses a non-atomic
    // head by name. The routing is a property of the PROGRAM rather than a lane list, so a
    // rule that later becomes existential moves its own lane without a second edit here.
    if program
        .iter()
        .any(|clause| clause.head_form() == HeadForm::Existential)
    {
        return chase_graph(program, attribution, edb, &terms, stop);
    }

    // The plan is a pure function of the clause program, and `close` hands every graph of
    // one call the SAME program — so the second graph onward reads the compiled plan back
    // instead of planning it again. The key is the calculus's own contract hash, which is
    // what the report already publishes as the identity of the rule set that ran, so the
    // cache cannot answer with a plan for a different calculus.
    let executable = plans
        .get_or_compile(&calculus_contract_hash(regime).to_hex(), program.to_vec())
        .into_plan()
        .map_err(EntailError::Evaluate)?;
    let evaluation = evaluate_until(
        &executable,
        edb,
        stop.map(|stop| &**stop as &dyn StopSignal),
    )
    .map_err(evaluate_error)?;

    // AN INCONSISTENCY IS DECIDED BEFORE AN ANSWER IS BUILT. Seventeen OWL 2 RL rules
    // conclude `false`, and a match on one of them says the knowledge base entails
    // everything — so there is no closure to hand back, only evidence. The first clash in
    // the evaluation's own total derivation order is the witness, which makes the choice a
    // function of the program and the data rather than of the round a rule happened to
    // fire in. The budget is still carried out, because the report the refusal owes its
    // caller is measured rather than stubbed.
    if let Some(witness) = first_clash(&evaluation, attribution, &terms, regime, graph) {
        return Ok(GraphRun {
            conclusions: Vec::new(),
            budget: evaluation.budget(),
            generalized_rdf_drops: 0,
            surrogate_drops: 0,
            clash: Some(witness),
            termination: None,
        });
    }

    // The budget is the evaluator's own measurement, not a second tally kept alongside it.
    // The semi-naive path states no existential rule and so invents no term: its fixpoint
    // is bounded by the active domain and there is no acyclicity analysis to report.
    let mut run = GraphRun {
        conclusions: Vec::new(),
        budget: evaluation.budget(),
        generalized_rdf_drops: 0,
        surrogate_drops: 0,
        clash: None,
        termination: None,
    };
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
            run.generalized_rdf_drops += 1;
            continue;
        }
        // Recorded only once the conclusion is known to be representable, so the per-rule
        // counts sum to exactly the inferred triples a caller can see.
        run.conclusions.push(Conclusion {
            rule: attribution[derivation.rule()],
            subject: subject.clone(),
            predicate: predicate.clone(),
            object: terms.value(&fact.object).clone(),
        });
    }
    Ok(run)
}

/// Fill a fresh [`RelationStore`] with everything one graph's run reasons FROM, and the
/// surface dictionary that reads its answers back.
///
/// The seed is the union being closed — the default graph always, plus `graph` when there
/// is one — together with `regime`'s axiomatic triples and the three pre-passes whose
/// premises no clause can express. It is a function of `(ds, regime, program, graph)` and
/// nothing else, which is what lets [`crate::explain`] rebuild the very store an answer was
/// produced from and re-derive a conclusion against it.
///
/// # Errors
///
/// [`EntailError::MalformedList`] if an RDF collection an OWL 2 axiom points at is not a
/// well-formed collection.
pub(crate) fn seed(
    ds: &RdfDataset,
    regime: Regime,
    program: &[DlClause],
    graph: Option<&TermValue>,
) -> Result<(RelationStore, Terms), EntailError> {
    let mut terms = Terms::default();
    terms.record_program(program);
    terms.record_literals();
    let mut edb = RelationStore::new();
    // The axiomatic triples are PREMISES, not conclusions: `S RDFS entails E` is defined
    // over the interpretations satisfying S *and* the axioms, and no rule of §9.2.1
    // concludes one. Seeding them beside the graph's own quads is that definition,
    // written down. See `crate::axioms` for the table and for which lanes assert it.
    for &(subject, predicate, object) in axioms_for(regime) {
        let subject = terms.record(&TermValue::iri(subject));
        let predicate = terms.record(&TermValue::iri(predicate));
        let object = terms.record(&TermValue::iri(object));
        let _ = edb.insert(&subject, &predicate, &object, RelationStore::DEFAULT_GRAPH);
    }
    let mut lists = ListIndex::default();
    let mut literals = LiteralIndex::default();
    let mut surrogates = SurrogateIndex::default();
    for quad in ds.quads() {
        // The seed is the union this run closes: the default graph always, plus the named
        // graph when there is one. Every OTHER named graph is left out, which is what makes
        // a cross-graph join impossible rather than merely unobserved.
        let in_seed = match (quad.g, graph) {
            (None, _) => true,
            (Some(g), Some(target)) => ds.term_value(g) == *target,
            (Some(_), None) => false,
        };
        if !in_seed {
            continue;
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
        if mints_surrogates(regime) {
            for (surface, value) in [
                (&subject, ds.term_value(quad.s)),
                (&predicate, ds.term_value(quad.p)),
                (&object, ds.term_value(quad.o)),
            ] {
                surrogates.observe(surface, &value);
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
    // literals this run's seed holds. See [`crate::datatypes`] for why an infinite premise
    // is a boundary rather than a loop, and why an unmodelled datatype is not judged.
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

    // What `rdfD1` and `rdfs14` OBSERVE — a datatyped literal, a triple term — decided
    // once over the seed's own terms. The clause language has no term-kind test, so
    // neither premise is expressible as a clause; see [`crate::surrogates`].
    if mints_surrogates(regime) {
        let iris: Vec<String> = surrogates.iris().map(str::to_owned).collect();
        for iri in iris {
            let _ = terms.record(&TermValue::iri(iri));
        }
        for fact in surrogates.materialize() {
            let _ = edb.insert(&fact.subject, fact.predicate, &fact.object, &fact.graph);
        }
    }

    Ok((edb, terms))
}

/// Evaluate an EXISTENTIAL calculus with the restricted chase and read its answer.
///
/// # Why a second evaluation path, and what it does NOT change
///
/// A least-fixpoint evaluator over definite clauses has no semantics for `∃ȳ. …`, so
/// `compile` refuses one by name; the chase is the consumer that head form was represented
/// for. The two agree on everything else: the chase fires an atomic clause exactly as the
/// semi-naive evaluator does, over the same seeded store, to the same least fixpoint. What
/// differs is that it INVENTS terms, and the two consequences of that are handled here.
///
/// # Termination is COMPUTED, and the certificate is the admission
///
/// `purrdf_datalog::chase::chase` certifies the clause set by constant-refined weak
/// acyclicity before it runs a round: an existential edge of the position dependency graph
/// that lies in a cycle is [`ChaseError::NonTerminating`], and nothing else is refused for
/// termination. There is no caller-supplied budget and no acyclicity parameter — the
/// analysis is a function of the clauses the calculus declares, and `the_existential_lanes_are_certified_terminating`
/// asserts that both lanes that reach here are certified.
///
/// # The surrogates do not reach the answer, and that is REQUIRED
///
/// Every conclusion mentioning a witness the chase invented is dropped at the
/// materialization boundary and counted, which raises the
/// [`Construct::Surrogate`](crate::Construct::Surrogate) boundary. See that construct's
/// reason for the W3C case that makes the exclusion mandatory rather than merely
/// convenient, and for why nothing surrogate-free is lost by it.
///
/// It is also what keeps [`Terms::value`] total. The dictionary is exhaustive over the
/// terms the store was SEEDED with plus the program's own constants, and a witness is
/// neither — so a witness surface is never looked up, because the fact carrying it was
/// already dropped.
fn chase_graph(
    program: &[DlClause],
    attribution: &[ChaseRule],
    edb: RelationStore,
    terms: &Terms,
    stop: Option<&Arc<dyn StopSignal>>,
) -> Result<GraphRun, EntailError> {
    let outcome = chase_until(program, edb, stop.map(|stop| &**stop as &dyn StopSignal))
        .map_err(chase_error)?;
    let witnesses: BTreeSet<&str> = outcome.witnesses().witnesses().collect();
    let mut run = GraphRun {
        conclusions: Vec::new(),
        budget: outcome.budget(),
        generalized_rdf_drops: 0,
        surrogate_drops: 0,
        // The two chased lanes (`RDF`, `RDFS`) state no rule whose conclusion is `false`,
        // so a chase evaluation has nothing to clash on. That is a property of their rule
        // tables, not an omission here.
        clash: None,
        // THE PROOF IS CARRIED OUT, NOT DISCARDED. `chase` certifies the clause set before
        // it runs a round, and this is that verdict — the reason this evaluation was
        // admitted at all — travelling with the facts it justifies.
        termination: TerminationCertificate::of_chase(outcome.termination()),
    };
    for derivation in outcome.derivations() {
        let fact = derivation.fact();
        // Bookkeeping, not an answer — the two surrogate relations and the observation
        // relations they read; see the semi-naive path for the same exclusion.
        if is_internal(&fact.predicate) {
            continue;
        }
        if [&fact.subject, &fact.predicate, &fact.object, &fact.graph]
            .into_iter()
            .any(|surface| witnesses.contains(surface.as_str()))
        {
            run.surrogate_drops += 1;
            continue;
        }
        let subject = terms.value(&fact.subject);
        let predicate = terms.value(&fact.predicate);
        if !admits_subject(subject) || !admits_predicate(predicate) {
            run.generalized_rdf_drops += 1;
            continue;
        }
        run.conclusions.push(Conclusion {
            rule: attribution[derivation.clause()],
            subject: subject.clone(),
            predicate: predicate.clone(),
            object: terms.value(&fact.object).clone(),
        });
    }
    Ok(run)
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

/// Whether `regime`'s lane states a rule that INVENTS a surrogate blank node.
///
/// The two lanes whose rule tables hold `rdfD1` / `rdfD1a` (`RDF`, and `RDFS` because RDFS
/// entailment subsumes RDF entailment) and `rdfs14` / `rdfs14a` (`RDFS`). `OWL-RL` states
/// none of the four — OWL 2 RL/RDF omits the RDF and RDFS axiomatic material — and `D`
/// states OWL 2 Profiles Table 8 alone, so neither pays for the pre-pass or the chase.
const fn mints_surrogates(regime: Regime) -> bool {
    matches!(regime, Regime::Rdf | Regime::Rdfs)
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
    graph: Option<&TermValue>,
) -> Option<InconsistencyWitness> {
    let owl = matches!(regime, Regime::OwlRl);
    evaluation
        .derivations()
        .iter()
        .find(|derivation| derivation.fact().predicate == CLASH_RELATION)
        .map(|derivation| witness_of(derivation, attribution, terms, owl, graph))
}

/// The witness a clash derivation carries.
fn witness_of(
    derivation: &Derivation,
    attribution: &[ChaseRule],
    terms: &Terms,
    owl: bool,
    graph: Option<&TermValue>,
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
    // The graph whose CLOSURE found the clash. A named-graph run is seeded with the union
    // of that graph and the default graph, so its premises may come from either — naming
    // the graph being closed is therefore the honest answer, because it is the run that
    // refused rather than a claim that every premise is asserted there. `None` IS the
    // default graph, which is closed against itself, so for it the two coincide.
    InconsistencyWitness::new(rule, premises, graph.cloned())
}

// ── Refutation: the same calculus, re-run over the premise plus an assertion ──────────

/// The calculus, COMPILED ONCE, ready to be run over a premise plus any number of added
/// assertions.
///
/// # Why this is not just another [`close`] call
///
/// A refutation asks a different question from a closure. `close` wants the conclusions and
/// refuses the moment a `false`-headed rule matches; a refutation wants exactly that match —
/// the clash IS the answer — and it asks for it many times over, once per negated
/// conclusion statement and again once per candidate axiom while a minimal entailing subset
/// is shrunk out. Routing that through `close` would mean handing every one of those runs a
/// fresh `RdfDatasetBuilder`, a fresh materialization pass and a fresh plan compilation for
/// a plan that is a pure function of the clauses.
///
/// So the two costs a repeated run would otherwise pay per iteration are paid ONCE here:
///
/// * **the plan.** [`PlanCache`] is content-addressed on the calculus's own contract hash,
///   so the ~200-clause `OWL-RL` program is compiled the first time [`Self::refute`] runs
///   and read back on every later call, whatever premise subset it is running over. The
///   cache is owned by this value and threaded by `&mut`, never global — the same
///   discipline [`close`] uses, and for the same reason: a longer-lived cache would make a
///   run's cost depend on which runs came before it.
/// * **the seed.** [`Self::seed`] interns the premise's terms, walks its RDF collections
///   into the internal `LIST[…]` relations and decides its literals' value spaces once; the
///   resulting [`RelationStore`] is then CLONED per run and the negation goes in as an
///   insert delta on the already-arranged store. Every pairwise refutation of one
///   `owl:AllDifferent` collection shares one seed and differs by one row.
///
/// What is NOT shared is the FIXPOINT, and the honest reason is worth writing down rather
/// than eliding. `purrdf-datalog`'s store is generic over an abelian
/// [`Weight`](purrdf_datalog::store::Weight) monoid and its consolidation merge compiles for
/// the signed `i64` instantiation — so Z-set retraction, and hence incremental maintenance,
/// is a COMPILED FACT of the representation. What the crate does not yet have is an
/// evaluator entry point that consumes one: [`evaluate`] takes a whole store and seeds its
/// round-1 delta from the whole of it, so each run here recomputes the closure of
/// `premise ∪ Δ` from the arranged seed. The sharing above is real and measurable; calling
/// it "incremental evaluation" would claim a maintenance path that does not exist yet, and a
/// refutation lane that quietly did so would be the wrong place to find that out.
pub(crate) struct Refuter {
    /// The lane whose calculus runs. `OWL-RL` in every caller today; carried rather than
    /// assumed so the attribution and the witness's rule spelling stay the lane's own.
    regime: Regime,
    /// The calculus's contract hash, which is the plan cache's key.
    contract: String,
    /// The clause program, lowered exactly as [`close_graph`] lowers it.
    program: Vec<DlClause>,
    /// Clause index → the rule that clause states, for the witness's attribution.
    attribution: Vec<ChaseRule>,
    /// The compiled plan, kept across every run of this refuter.
    plans: PlanCache,
}

/// A premise seeded ONCE, re-closed against any number of added assertions.
///
/// The dictionary is mutable because an added assertion may mention a term the premise does
/// not — the class of an `owl:complementOf`, say — and [`Terms::value`] has to stay total
/// over everything the store can hold. Recording is monotone and idempotent: a surface only
/// ever gains its value, so a term left over from one run cannot be reached by a later run
/// whose store never held it.
pub(crate) struct Seeded {
    /// The arranged store the premise's own facts and pre-passes produced.
    edb: RelationStore,
    /// The surface → value dictionary that reads an answer back.
    terms: Terms,
}

/// What one refutation run found: the clash, and everything the run derived.
#[derive(Debug)]
pub(crate) struct Clash {
    /// The rule whose premises were all satisfied, and the triples that satisfied them.
    pub(crate) witness: InconsistencyWitness,
    /// Every RDF-1.2-representable triple the run DERIVED, in the evaluator's own total
    /// derivation order.
    ///
    /// The seeded facts are not repeated here: the caller already holds them (they are the
    /// premise subset and the assertion it handed in), so restating them would be a second
    /// copy that could disagree with the first.
    pub(crate) derived: Vec<[TermValue; 3]>,
}

/// What one re-closure over `premise ∪ added` produced: the derived triples, and whether a
/// `false`-headed rule fired on the way.
///
/// The difference from [`Clash`] is which question the caller is asking. A refutation asks
/// "did this clash?" and needs the derivation only when the answer is yes, so
/// [`Refuter::refute`] returns early when nothing clashed and never pays to read the
/// derivations back. [`crate::entails::freeze`] asks "what does this entail?", which needs
/// the derivations in BOTH outcomes — the head it is looking for when the run is consistent,
/// and the body instances of the clash when it is not, because an inconsistent frozen
/// instance establishes the implication VACUOUSLY and still owes evidence for it.
#[derive(Debug)]
pub(crate) struct Closed {
    /// The first `false`-headed rule that fired, if any.
    pub(crate) clash: Option<InconsistencyWitness>,
    /// Every RDF-1.2-representable triple the run DERIVED, in the evaluator's own total
    /// derivation order. As [`Clash::derived`], the seeded facts are not repeated.
    pub(crate) derived: Vec<[TermValue; 3]>,
}

impl Refuter {
    /// A refuter for `regime`'s calculus.
    ///
    /// Capacity one, because a refuter presents exactly one program for its whole life: the
    /// calculus is a function of the regime, and the regime is fixed at construction.
    pub(crate) fn new(regime: Regime) -> Self {
        let (program, attribution) = program_with_attribution(regime);
        Self {
            regime,
            contract: calculus_contract_hash(regime).to_hex(),
            program,
            attribution,
            plans: PlanCache::new(1),
        }
    }

    /// Seed `ds`'s default graph once.
    ///
    /// # Errors
    ///
    /// [`EntailError::MalformedList`] if an RDF collection an OWL 2 axiom points at is not a
    /// well-formed collection — the same refusal [`seed`] owes any caller.
    pub(crate) fn seed(&self, ds: &RdfDataset) -> Result<Seeded, EntailError> {
        let (edb, terms) = seed(ds, self.regime, &self.program, None)?;
        Ok(Seeded { edb, terms })
    }

    /// Run the calculus over `seeded`'s premise PLUS `added`, and report the first clash.
    ///
    /// `Ok(None)` is "no `false`-headed rule matched": under a calculus that is complete for
    /// the input's syntax that is a proof of CONSISTENCY, which is what a caller reads it as
    /// — see [`crate::entails::refutation`] for the precondition that entitles it to.
    ///
    /// # The pre-passes are not re-run, and that is CHECKED rather than assumed
    ///
    /// [`seed`] computes three things no clause can express — the RDF collections an OWL
    /// axiom points at, the XSD value-space judgements, and (in the surrogate lanes) the
    /// term-kind observations — and this method reuses all three from the seed instead of
    /// recomputing them over `premise ∪ added`. That is only sound if `added` cannot change
    /// any of them, so the caller mints assertions from a WHITELIST of exactly two shapes
    /// (`owl:sameAs` between two terms, and `rdf:type` to a named class) and
    /// `an_added_assertion_never_disturbs_a_pre_pass` is what holds it to that: neither
    /// predicate is `rdf:first`, `rdf:rest` or one of the seven list-valued OWL predicates
    /// [`crate::lists::LIST_VALUED`] names, and neither shape carries a literal.
    ///
    /// # Errors
    ///
    /// [`EntailError::Evaluate`] if the plan will not compile or the evaluation passes one
    /// of `purrdf-datalog`'s three fixed ceilings.
    pub(crate) fn refute(
        &mut self,
        seeded: &mut Seeded,
        added: &[[TermValue; 3]],
    ) -> Result<Option<Clash>, EntailError> {
        let evaluation = self.evaluate_with(seeded, added)?;
        let Some(witness) = first_clash(
            &evaluation,
            &self.attribution,
            &seeded.terms,
            self.regime,
            None,
        ) else {
            // NOTHING CLASHED, so there is nothing to explain and the derivations are not
            // read back at all. The shrinking search rejects most candidate subsets, so this
            // is the common path and it must stay the cheap one.
            return Ok(None);
        };
        Ok(Some(Clash {
            witness,
            derived: derived_triples(&evaluation, &seeded.terms),
        }))
    }

    /// Run the calculus over `seeded`'s premise PLUS `added` and report EVERYTHING it drew,
    /// clash or no clash.
    ///
    /// The same evaluation [`Self::refute`] performs, read out differently — see [`Closed`]
    /// for which caller needs which reading, and [`Self::refute`] for the pre-pass whitelist
    /// that makes reusing one seed across many `added` sets sound. That whitelist binds this
    /// method identically: `added` must carry no `rdf:first`, `rdf:rest` or list-valued
    /// predicate and no literal.
    ///
    /// # Errors
    ///
    /// As [`Self::refute`]: [`EntailError::Evaluate`] if the plan will not compile or the
    /// evaluation passes one of `purrdf-datalog`'s three fixed ceilings.
    pub(crate) fn close(
        &mut self,
        seeded: &mut Seeded,
        added: &[[TermValue; 3]],
    ) -> Result<Closed, EntailError> {
        let evaluation = self.evaluate_with(seeded, added)?;
        let clash = first_clash(
            &evaluation,
            &self.attribution,
            &seeded.terms,
            self.regime,
            None,
        );
        Ok(Closed {
            clash,
            derived: derived_triples(&evaluation, &seeded.terms),
        })
    }

    /// Evaluate the program over `seeded`'s arranged store plus `added` as an insert delta.
    fn evaluate_with(
        &mut self,
        seeded: &mut Seeded,
        added: &[[TermValue; 3]],
    ) -> Result<purrdf_datalog::seminaive::Evaluation, EntailError> {
        // The insert DELTA: the seed's arrangement is cloned, and the assertion is the only
        // row that was not already in it.
        let mut edb = seeded.edb.clone();
        for [subject, predicate, object] in added {
            let subject = seeded.terms.record(subject);
            let predicate = seeded.terms.record(predicate);
            let object = seeded.terms.record(object);
            let _ = edb.insert(&subject, &predicate, &object, RelationStore::DEFAULT_GRAPH);
        }

        let program = self.program.clone();
        let executable = self
            .plans
            .get_or_compile(&self.contract, program)
            .into_plan()
            .map_err(EntailError::Evaluate)?;
        // The incremental re-evaluation seam names no stop signal: it re-runs one delta over
        // an already-seeded store for an explanation, not the caller's closure.
        evaluate_until(&executable, edb, None).map_err(evaluate_error)
    }
}

/// Everything `evaluation` DERIVED, as RDF 1.2 triples.
///
/// Read back through the same two filters [`close_graph`] applies: an internal relation is
/// bookkeeping rather than a triple, and a conclusion the RDF 1.2 IR cannot hold is
/// abandoned rather than fabricated around.
fn derived_triples(
    evaluation: &purrdf_datalog::seminaive::Evaluation,
    terms: &Terms,
) -> Vec<[TermValue; 3]> {
    let mut derived = Vec::new();
    for derivation in evaluation.derivations() {
        let fact = derivation.fact();
        if is_internal(&fact.predicate) {
            continue;
        }
        let subject = terms.value(&fact.subject);
        let predicate = terms.value(&fact.predicate);
        if !admits_subject(subject) || !admits_predicate(predicate) {
            continue;
        }
        derived.push([
            subject.clone(),
            predicate.clone(),
            terms.value(&fact.object).clone(),
        ]);
    }
    derived
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
pub(crate) struct Terms {
    /// Surfaces to the values they were rendered from, in lexical surface order.
    by_surface: BTreeMap<String, TermValue>,
}

impl Terms {
    /// Record `value` and return its surface.
    pub(crate) fn record(&mut self, value: &TermValue) -> String {
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
    pub(crate) fn value(&self, surface: &str) -> &TermValue {
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
pub(crate) fn surface_of(value: &TermValue) -> String {
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
            // Internal store-surface identity key (scope-first form; injective
            // because the digit-only scope ends at the first dot) plus the
            // diagnostics built on it — never RDF document egress, so label
            // syntax is not enforced here.
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
    use super::{Conclusion, admits_predicate, admits_subject, close, close_graph, surface_of};
    use crate::Regime;
    use crate::calculus::{ALL_REGIMES, calculus_contract_hash, program_with_attribution};
    use crate::calculus_program;
    use crate::vocab::{
        OWL_HASKEY, OWL_INTERSECTIONOF, OWL_PROPERTYCHAINAXIOM, OWL_UNIONOF, RDF_FIRST, RDF_NIL,
        RDF_REST, RDF_TYPE, RDFS_SUBCLASSOF, XSD_STRING,
    };
    use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfTextDirection, TermValue};
    use purrdf_datalog::cache::PlanCache;
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
    /// A fixture named graph.
    const EX_G: &str = "http://example.org/g";
    /// A SECOND fixture named graph — two are what make a `1 + n` run more than `1 + 1`.
    const EX_H: &str = "http://example.org/h";
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
    /// The declared calculus is GRAPH-AGNOSTIC, and this is the statement that keeps it so.
    /// [`super::close_graph`] seeds the store's one default partition with the union it is
    /// closing — the default graph alone, or a named graph together with the default graph
    /// — evaluates the unchanged program over it, and [`super::close`] routes the answer
    /// back to the graph that produced it. That is what makes `1 + n` evaluations a faithful
    /// evaluation of ONE program rather than `1 + n` variants of it, and it is why the
    /// dataset semantics needed no clause to change.
    ///
    /// A rule that later names a graph of its own fails here, which is the signal that the
    /// per-graph seeding above can no longer stand in for it.
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
        let closed = close(&collection_fixture(), Regime::OwlRl, None)
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
        let closed = close(&collection_fixture(), Regime::OwlRl, None)
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
        let error = close(&ds, Regime::OwlRl, None).expect_err("a malformed collection is refused");
        let rendered = error.to_string();
        assert!(rendered.contains("carries no rdf:rest"), "{rendered}");
        assert!(rendered.contains(EX_L0), "{rendered}");
        // The RDFS lane says nothing about `owl:intersectionOf`, so the same graph is
        // ordinary data there and closes without complaint.
        assert!(close(&ds, Regime::Rdfs, None).is_ok());
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
        let error = close(&ds, Regime::OwlRl, None).expect_err("a cycle is refused");
        assert!(error.to_string().contains("cyclic"), "{error}");
    }

    /// A BLANK NODE IN THE INPUT IS ONE BLANK NODE IN THE CLOSURE, in every position —
    /// including the GRAPH NAME.
    ///
    /// [`super::copy_into`] exists for this. `push_dataset` would have re-scoped the input's
    /// blank nodes as though a merge were happening, and a conclusion re-interned through
    /// [`intern_into`] carries the ORIGINAL scope, so the closure would hold two blank nodes
    /// per input one: the copied triples about the first and the inferred triples about the
    /// second. With a blank-node graph name the split is starker still — the named graph's
    /// own conclusions would land in a graph the input does not have.
    ///
    /// Asserted over the CANONICAL form, because that is what a caller sees and because
    /// RDFC-1.0 assigns one label per distinct node: three labels here would be the defect,
    /// two is the answer.
    #[test]
    fn a_blank_node_graph_name_survives_the_closure() {
        let mut b = RdfDatasetBuilder::new();
        let graph = b.intern_blank("g", BlankScope::DEFAULT);
        let subject = b.intern_blank("s", BlankScope::DEFAULT);
        let sub = b.intern_iri(RDFS_SUBCLASSOF);
        let object = b.intern_iri(EX_B);
        b.push_quad(subject, sub, object, Some(graph));
        let ds = b.freeze().expect("the fixture freezes");

        let closed = close(&ds, Regime::OwlRl, None).expect("owl-rl closes it").0;
        let nquads = purrdf_core::canonicalize(&closed).nquads;
        let labels: BTreeSet<&str> = nquads
            .split_whitespace()
            .filter(|token| token.starts_with("_:"))
            .collect();
        assert_eq!(
            labels.len(),
            2,
            "the input has exactly two blank nodes — the subject and the graph — and the \
             closure must not double either:\n{nquads}"
        );
        // The conclusions really did land in the input's own graph rather than beside it:
        // eq-ref types both blank nodes, and every one of its conclusions is in that graph.
        let derived: Vec<&str> = nquads
            .lines()
            .filter(|line| line.contains(crate::vocab::OWL_SAMEAS) && line.starts_with("_:"))
            .collect();
        assert!(!derived.is_empty(), "eq-ref drew nothing:\n{nquads}");
        for line in derived {
            let graph_token = line
                .split_whitespace()
                .rev()
                .nth(1)
                .expect("a quad has a graph slot before its terminator");
            assert!(
                graph_token.starts_with("_:"),
                "a conclusion left the blank-node graph it was drawn in: {line}"
            );
        }
    }

    /// ONE CALL COMPILES THE CALCULUS ONCE, HOWEVER MANY GRAPHS THE DATASET HOLDS.
    ///
    /// A dataset with `n` named graphs is `1 + n` evaluations of the SAME declared program
    /// — see [`close`] for why the semantics require that — and the plan those clauses
    /// compile to is a pure function of the clauses. Compiling inside [`close_graph`] made
    /// a hundred-graph OWL-RL run plan a ~200-clause calculus a hundred and one times.
    ///
    /// The measurement is the cache's own occupancy, which is honest because
    /// `PlanCache::insert` runs on a MISS and only on a miss: an entry appears exactly when
    /// a compile happened, so counting the calls after which the cache grew counts the
    /// compiles. Four graph closures, one compile.
    ///
    /// It is also what pins the cache to a CALL. `close` builds one and threads it by
    /// `&mut`; nothing here is reachable from a later call, so a run's answer and its cost
    /// cannot depend on what an earlier run happened to compile.
    #[test]
    fn one_call_compiles_the_calculus_once_however_many_graphs_it_holds() {
        let mut b = RdfDatasetBuilder::new();
        let sub = b.intern_iri(RDFS_SUBCLASSOF);
        let a = b.intern_iri(EX_A);
        let c = b.intern_iri(EX_B);
        b.push_quad(a, sub, c, None);
        let mut graphs: Vec<Option<TermValue>> = vec![None];
        for name in [EX_G, EX_H, EX_S] {
            let g = b.intern_iri(name);
            let x = b.intern_iri(EX_X);
            let ty = b.intern_iri(RDF_TYPE);
            b.push_quad(x, ty, a, Some(g));
            graphs.push(Some(TermValue::iri(name)));
        }
        let ds = b.freeze().expect("the fixture freezes");

        let (program, attribution) = program_with_attribution(Regime::OwlRl);
        let mut plans = PlanCache::new(2);
        let mut compiles = 0_usize;
        for graph in &graphs {
            let before = plans.len();
            close_graph(
                &ds,
                Regime::OwlRl,
                &program,
                &attribution,
                graph.as_ref(),
                &mut plans,
                None,
            )
            .expect("each graph closes");
            compiles += usize::from(plans.len() > before);
        }
        assert_eq!(graphs.len(), 4, "the default graph plus three named ones");
        assert_eq!(
            compiles, 1,
            "four graph closures over one declared program must compile it once"
        );
        assert_eq!(plans.len(), 1, "one program, one cached plan");

        // …and the entry is the CALCULUS's, keyed by the identity the report publishes, so
        // the cache cannot answer a lane with another lane's plan.
        let lookup = plans.get_or_compile(
            &calculus_contract_hash(Regime::OwlRl).to_hex(),
            program.clone(),
        );
        assert!(
            lookup.cache_hit(),
            "the plan the four closures shared is warm"
        );
        assert_eq!(lookup.plan_builds(), 0);
        let other = plans.get_or_compile(
            &calculus_contract_hash(Regime::D).to_hex(),
            calculus_program(Regime::D),
        );
        assert!(
            !other.cache_hit(),
            "a different calculus is a different entry"
        );
    }

    /// The cached plan changes NOTHING about the answer.
    ///
    /// The optimization is only worth having if it is invisible. The property at issue is
    /// narrow and is tested as such: closing one graph with a plan that was compiled for a
    /// DIFFERENT graph of the same dataset must give exactly what closing it with a plan
    /// compiled for itself gives. A plan that had absorbed anything from the store it was
    /// first evaluated against — a seeded term, a graph name, a partition — would show up
    /// here as two different conclusion lists for the same graph.
    #[test]
    fn a_warm_plan_and_a_cold_one_close_a_graph_identically() {
        let mut b = RdfDatasetBuilder::new();
        let sub = b.intern_iri(RDFS_SUBCLASSOF);
        let ty = b.intern_iri(RDF_TYPE);
        let a = b.intern_iri(EX_A);
        let c = b.intern_iri(EX_B);
        let x = b.intern_iri(EX_X);
        let y = b.intern_iri(EX_Y);
        let g = b.intern_iri(EX_G);
        let h = b.intern_iri(EX_H);
        b.push_quad(a, sub, c, None);
        b.push_quad(x, ty, a, Some(g));
        b.push_quad(y, ty, c, Some(h));
        let ds = b.freeze().expect("the fixture freezes");

        let (program, attribution) = program_with_attribution(Regime::OwlRl);
        let conclusions = |plans: &mut PlanCache, graph: Option<&TermValue>| {
            close_graph(
                &ds,
                Regime::OwlRl,
                &program,
                &attribution,
                graph,
                plans,
                None,
            )
            .expect("the graph closes")
            .conclusions
            .iter()
            .map(Conclusion::key)
            .collect::<Vec<_>>()
        };

        let target = TermValue::iri(EX_H);
        // COLD: a cache that has never seen this program.
        let mut fresh = PlanCache::new(2);
        let cold = conclusions(&mut fresh, Some(&target));
        // WARM: the very plan the default graph and ex:g already ran, reused.
        let mut shared = PlanCache::new(2);
        let _ = conclusions(&mut shared, None);
        let _ = conclusions(&mut shared, Some(&TermValue::iri(EX_G)));
        assert_eq!(shared.len(), 1, "the two earlier graphs left one plan");
        let warm = conclusions(&mut shared, Some(&target));

        assert!(!cold.is_empty(), "the fixture derived nothing");
        assert_eq!(
            warm, cold,
            "a reused plan produced a different answer than a freshly compiled one"
        );
    }
}
