// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The **homomorphism** mechanism: map the question into the closure.
//!
//! This is the one mechanism this module tree currently has, and it is the one OWL 2
//! Profiles §4.3 states the entailment procedure in terms of. Once the closure of the
//! premise has been computed, a conclusion graph is entailed exactly when there is a
//! mapping from its blank nodes to closure terms under which every conclusion triple is a
//! triple of the closure — a graph HOMOMORPHISM, which is RDF 1.2 Semantics' interpolation
//! lemma read as an algorithm.
//!
//! # Soundness, completeness, and which one this module owes
//!
//! Finding a mapping is a PROOF: the rule set is sound, so every closure triple is entailed
//! by the premise, and a conclusion all of whose triples are entailed with a consistent
//! reading of its existentials is itself entailed. That direction needs no precondition and
//! this module owes it unconditionally.
//!
//! Failing to find one is NOT a proof of non-entailment. It is a proof only when the rule
//! set is also complete for the premise at hand, which is a property of the PREMISE and not
//! of this search — so this module reports the failure as a diagnosis ([`MissReason`]) and
//! leaves the question of what that failure licenses to [`super::precondition`]. Reading a
//! [`MissReason`] as "not entailed" without that check is the overclaim the whole module
//! tree is arranged to prevent.
//!
//! # Termination
//!
//! The search is depth-first over a fixed pattern list with backtracking, so it terminates
//! on its own: each level consumes one pattern, and no level can revisit a candidate. What
//! it does not do on its own is terminate QUICKLY — the search is exponential in the number
//! of distinct blank nodes in the worst case — so it runs against [`MATCH_BUDGET`],
//! measured in candidate triples visited. Exhausting the budget is an error rather than a
//! verdict, because "I stopped looking" and "there is nothing to find" are different claims
//! and only one of them is true.
//!
//! # Why the search carries its own stack
//!
//! One level of the search is one pattern of the question, so the search is as DEEP as the
//! conclusion graph is large — a ten-thousand-triple conclusion is ten thousand levels. As
//! call frames that is a stack overflow, which is not an error a caller can catch: the
//! process aborts, nothing unwinds, and a host embedding this library dies with it. The
//! budget is no defence, because it counts candidates rather than levels and a conclusion
//! with distinct predicates costs one candidate per level.
//!
//! So the solver carries an explicit frame stack on the HEAP and the depth it can reach
//! is a function of available memory rather than of a thread's stack rlimit — which differs
//! by an order of magnitude between a native binary and `wasm32` and is not something a
//! library can either read or raise. [`MATCH_BUDGET`] stays exactly what it was: the
//! backstop against combinatorial blowup, reported as [`EntailError::MatchBudget`].
//!
//! # Determinism
//!
//! The index is a `BTreeMap` keyed by the predicate IRI, each bucket in the closure's own
//! frozen order; patterns are visited in a stably-sorted, most-constrained-first order; and
//! the trail undoes exactly the bindings an attempt introduced. Two runs over one closure
//! and one question therefore visit the same candidates in the same order and return the
//! same binding — the FIRST one in that order, not an arbitrary one. The frame stack is
//! the call stack written down, so it visits that same order: a frame's candidates are
//! enumerated when the frame is pushed (against the bindings in force at that moment), a
//! child is explored to exhaustion before its parent advances, and the budget is spent one
//! unit per candidate in the order the candidates are reached.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::TermValue;

use crate::EntailError;
use crate::entails::graph::Triple;
use crate::entails::pattern::{Pat, PatTriple, VarKey, var_count};

/// The match budget, in candidate triples visited.
///
/// Sized so that no well-formed question reaches it — the search visits one candidate per
/// (pattern, closure triple sharing its predicate) pair per backtracking branch, and a
/// conclusion graph is typically a handful of triples — while a pathological question over
/// a large closure still stops. It is a STEP count and never a clock reading, so the
/// refusal is reproducible on every target including `wasm32`, where there is no clock to
/// read.
pub const MATCH_BUDGET: u64 = 5_000_000;

/// A solution: what each variable of the question was bound to.
pub type Binding = BTreeMap<VarKey, TermValue>;

/// The closure, indexed by predicate — the one position that is ground in almost every
/// question, and the one RDF guarantees is an IRI.
type Index = BTreeMap<String, Vec<Triple>>;

/// The index key of a term.
///
/// An IRI keys by itself. Nothing else can occupy a predicate position in RDF 1.2, so the
/// remaining arm exists only to keep the function total; keying such a term by its debug
/// rendering means both sides of a comparison compute the same key, so a generalized-RDF
/// triple that somehow reached here matches itself and nothing else.
fn index_key(term: &TermValue) -> String {
    match term {
        TermValue::Iri(iri) => iri.clone(),
        other => format!("{other:?}"),
    }
}

/// The closure a question is answered against: its triples, indexed for matching.
///
/// Carried as a whole rather than as a borrow of the closure dataset because an
/// [`EntailmentWarrant`](super::warrant::EntailmentWarrant) outlives the run that produced
/// it and has to be re-checkable without re-running the chase.
#[derive(Debug, Clone)]
pub(crate) struct Closure {
    /// Every default-graph triple, as a set, for membership in `O(log n)`.
    members: BTreeSet<Triple>,
    /// The same triples bucketed by predicate, for candidate enumeration.
    index: Index,
}

impl Closure {
    /// Index `triples`.
    pub(crate) fn of(triples: Vec<Triple>) -> Self {
        let mut index: Index = BTreeMap::new();
        let mut members = BTreeSet::new();
        for triple in triples {
            index
                .entry(index_key(&triple[1]))
                .or_default()
                .push(triple.clone());
            members.insert(triple);
        }
        Self { members, index }
    }

    /// Whether the closure holds `triple`.
    pub(crate) fn contains(&self, triple: &Triple) -> bool {
        self.members.contains(triple)
    }

    /// How many distinct triples the closure holds.
    pub(crate) fn len(&self) -> usize {
        self.members.len()
    }

    /// Every triple whose predicate is `iri`, in the closure's own frozen order.
    ///
    /// The index is already bucketed by predicate for candidate enumeration, so this is a
    /// lookup rather than a scan — which matters because [`datarange`] asks it once per
    /// recognized conclusion triple.
    ///
    /// [`datarange`]: super::datarange
    pub(crate) fn with_predicate(&self, iri: &str) -> &[Triple] {
        self.index.get(iri).map_or(&[][..], Vec::as_slice)
    }

    /// This closure plus `extra`, as a new closure.
    ///
    /// The original is left alone because a warrant carries it: [`comprehension`] mints
    /// licensed triples on top of the premise's closure and has to be able to report the two
    /// apart, since only one of them is a conclusion of the chase.
    ///
    /// [`comprehension`]: super::comprehension
    pub(crate) fn extended_with(&self, extra: Vec<Triple>) -> Self {
        let mut triples: Vec<Triple> = self.members.iter().cloned().collect();
        triples.extend(extra);
        Self::of(triples)
    }
}

/// Whether `pat` unifies with `ground`, binding variables as it goes.
///
/// Every key this newly binds is recorded on `trail`, so a caller whose later patterns fail
/// can undo exactly the bindings this attempt introduced — and not the ones an earlier
/// attempt is still relying on.
fn try_unify(pat: &Pat, ground: &TermValue, bound: &mut Binding, trail: &mut Vec<VarKey>) -> bool {
    match pat {
        Pat::Var(key) => {
            if let Some(previous) = bound.get(key) {
                return previous == ground;
            }
            bound.insert(key.clone(), ground.clone());
            trail.push(key.clone());
            true
        }
        Pat::Triple(inner) => match ground {
            TermValue::Triple { s, p, o } => {
                try_unify(&inner[0], s, bound, trail)
                    && try_unify(&inner[1], p, bound, trail)
                    && try_unify(&inner[2], o, bound, trail)
            }
            _ => false,
        },
        Pat::Ground(term) => term == ground,
    }
}

/// The bucket a ground predicate term reads, if the closure has one.
///
/// The IRI arm borrows rather than allocating a key, because this runs once per pattern per
/// backtracking branch; the remaining arm cannot occur in RDF 1.2 and pays for its own
/// impossibility.
fn bucket_for<'a>(term: &TermValue, index: &'a Index) -> Option<&'a Vec<Triple>> {
    match term {
        TermValue::Iri(iri) => index.get(iri.as_str()),
        other => index.get(format!("{other:?}").as_str()),
    }
}

/// The closure buckets a pattern's candidates can come from.
///
/// An iterator rather than a collected `Vec`, so the overwhelmingly common case — a ground
/// predicate, one bucket — allocates nothing at all inside the search.
enum Buckets<'a> {
    /// A ground (or already-bound) predicate: exactly the bucket it names, if any.
    One(Option<&'a Vec<Triple>>),
    /// An open predicate variable: every bucket, in the index's own key order.
    Every(std::collections::btree_map::Values<'a, String, Vec<Triple>>),
}

impl<'a> Iterator for Buckets<'a> {
    type Item = &'a Vec<Triple>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::One(slot) => slot.take(),
            Self::Every(values) => values.next(),
        }
    }
}

/// The closure triples that could satisfy `pat` given what is already bound.
///
/// A pattern whose predicate is ground — or is a variable already bound to one — reads one
/// bucket. A pattern whose predicate is still open has to consider every bucket, because a
/// predicate variable ranges over the closure's whole predicate vocabulary. A pattern whose
/// predicate is a triple TERM matches nothing: RDF 1.2 predicates are IRIs, so no bucket
/// can hold one.
fn candidates<'a>(pat: &PatTriple, index: &'a Index, bound: &Binding) -> Buckets<'a> {
    match &pat[1] {
        Pat::Ground(term) => Buckets::One(bucket_for(term, index)),
        Pat::Var(var) => match bound.get(var) {
            Some(term) => Buckets::One(bucket_for(term, index)),
            None => Buckets::Every(index.values()),
        },
        Pat::Triple(_) => Buckets::One(None),
    }
}

/// What a solver does with a solution it reached.
enum Collect<'a> {
    /// Stop at the first solution — the boolean question.
    First,
    /// Record the projection of every solution — the enumerating question.
    All {
        /// The projected variables, in the order a row lists them.
        vars: &'a [VarKey],
        /// The rows found so far, deduplicated and ordered by the row itself.
        rows: &'a mut BTreeSet<Vec<TermValue>>,
    },
}

/// Record the solution `bound` reached, if the caller asked to see it.
///
/// Every variable of the question is bound at a complete solution, so the row is total; a
/// projected variable that no pattern mentions cannot exist, because `projected_vars` reads
/// the names out of the patterns themselves.
fn record(collect: &mut Collect<'_>, bound: &Binding) {
    if let Collect::All { vars, rows } = collect {
        let row = vars
            .iter()
            .map(|var| {
                bound.get(var).cloned().expect(
                    "a projected variable is read out of the patterns themselves, and every \
                     pattern is unified at a complete solution",
                )
            })
            .collect();
        rows.insert(row);
    }
}

/// One level of the search: one pattern, and where its enumeration has got to.
///
/// This is the state a call frame of the recursive form held implicitly — the candidate
/// iterator, the trail of the attempt in flight, and whether anything below has succeeded —
/// written down so the stack it lives on is the heap.
struct Frame<'a> {
    /// The pattern this level consumes, as an index into the solver's pattern list.
    pattern: usize,
    /// The buckets this level's candidates come from, fixed when the frame was pushed.
    buckets: Buckets<'a>,
    /// The remainder of the bucket currently being walked.
    bucket: std::slice::Iter<'a, Triple>,
    /// The bindings the attempt in flight introduced, to be undone when it is abandoned.
    ///
    /// Reused across this frame's attempts rather than reallocated per candidate, which is
    /// what a `Vec` local to the loop body was.
    trail: Vec<VarKey>,
    /// Whether any attempt at this level has reached a complete solution.
    found: bool,
}

impl<'a> Frame<'a> {
    /// A frame for `pattern`, drawing candidates from `buckets`.
    fn new(pattern: usize, buckets: Buckets<'a>) -> Self {
        Self {
            pattern,
            buckets,
            bucket: [].iter(),
            trail: Vec::new(),
            found: false,
        }
    }

    /// The next candidate at this level, walking into the next bucket when one runs out.
    fn next_candidate(&mut self) -> Option<&'a Triple> {
        loop {
            if let Some(candidate) = self.bucket.next() {
                return Some(candidate);
            }
            self.bucket = self.buckets.next()?.iter();
        }
    }
}

/// What the solver's loop does next, decided while the top frame is borrowed and acted on
/// after that borrow ends — because acting on it may push a frame onto the same stack.
enum Step<'a> {
    /// Try this candidate against the top frame's pattern.
    Try(&'a Triple),
    /// The top frame has no candidates left; it reports this answer to its parent.
    Exhausted(bool),
}

/// Solve `pats` against `index`, backtracking over variable bindings.
///
/// Returns whether at least one solution was reached. Under [`Collect::All`] the search
/// continues past the first, so the answer is "the row set is now complete" rather than
/// "stop".
///
/// The search is depth-first with an explicit frame stack, in exactly the shape and order
/// the recursive form had — see this module's *Why the search carries its own stack*.
///
/// # Errors
///
/// [`EntailError::MatchBudget`] when `budget` runs out before the search finishes.
fn solve(
    pats: &[PatTriple],
    index: &Index,
    bound: &mut Binding,
    budget: &mut u64,
    collect: &mut Collect<'_>,
) -> Result<bool, EntailError> {
    // The empty question is satisfied by the empty mapping: there is nothing to place.
    let Some(first) = pats.first() else {
        record(collect, bound);
        return Ok(true);
    };
    let mut stack = vec![Frame::new(0, candidates(first, index, bound))];
    // The answer a just-finished level reported, waiting to be folded into its parent — the
    // value the recursive form read straight out of its `solve(…)?` call.
    let mut reported: Option<bool> = None;
    loop {
        let step = {
            let frame = stack
                .last_mut()
                .expect("the loop returns as soon as the stack empties");
            if let Some(below) = reported.take() {
                if below {
                    frame.found = true;
                    if matches!(collect, Collect::First) {
                        // The bindings of the winning attempt are the answer, so they are
                        // deliberately left in place rather than undone.
                        return Ok(true);
                    }
                }
                for undo in frame.trail.drain(..) {
                    bound.remove(&undo);
                }
            }
            match frame.next_candidate() {
                Some(candidate) => Step::Try(candidate),
                None => Step::Exhausted(frame.found),
            }
        };
        let candidate = match step {
            Step::Exhausted(found) => {
                stack.pop();
                if stack.is_empty() {
                    return Ok(found);
                }
                reported = Some(found);
                continue;
            }
            Step::Try(candidate) => candidate,
        };
        *budget = budget.checked_sub(1).ok_or(EntailError::MatchBudget)?;
        let (pattern, matched) = {
            let frame = stack
                .last_mut()
                .expect("the top frame is the one that produced this candidate");
            let pat = &pats[frame.pattern];
            frame.trail.clear();
            let matched = try_unify(&pat[0], &candidate[0], bound, &mut frame.trail)
                && try_unify(&pat[1], &candidate[1], bound, &mut frame.trail)
                && try_unify(&pat[2], &candidate[2], bound, &mut frame.trail);
            (frame.pattern, matched)
        };
        if !matched {
            let frame = stack
                .last_mut()
                .expect("the top frame is the one that produced this candidate");
            for undo in frame.trail.drain(..) {
                bound.remove(&undo);
            }
            continue;
        }
        // Descend, or — with no pattern left to place — report the complete solution back
        // into the frame that just made it complete.
        match pats.get(pattern + 1) {
            Some(next) => stack.push(Frame::new(pattern + 1, candidates(next, index, bound))),
            None => {
                record(collect, bound);
                reported = Some(true);
            }
        }
    }
}

/// Why a question did not map into a closure.
///
/// The distinction is what a reader needs to act: a pattern with no candidate at all names
/// a conclusion the chase never produced, whereas patterns that are each individually
/// satisfiable but not jointly satisfiable name a blank-node IDENTITY the chase did not
/// establish. The first is a missing rule or a missing premise; the second is an equality
/// the closure does not know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissReason {
    /// These question triples have no candidate in the closure at all, rendered for a
    /// human.
    NoCandidate(Vec<String>),
    /// Every question triple has a candidate, but no single variable mapping satisfies them
    /// all at once.
    NoConsistentMapping,
}

impl MissReason {
    /// A one-line summary for a log or a report.
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::NoCandidate(triples) => format!("closure lacks {}", triples.join(" ; ")),
            Self::NoConsistentMapping => {
                "every target triple is present but no consistent blank-node mapping exists"
                    .to_owned()
            }
        }
    }
}

impl std::fmt::Display for MissReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.summary())
    }
}

/// Order patterns most-constrained-first.
///
/// A stable sort by variable count, so a fully ground pattern is tried before one with two
/// open positions: the ground one either fails immediately (ending the search without
/// branching) or fixes nothing, and either way the branching factor at every later level is
/// smaller. Stability keeps the visit order — and therefore the binding returned — a
/// function of the question's own triple order.
fn most_constrained_first(mut pats: Vec<PatTriple>) -> Vec<PatTriple> {
    pats.sort_by_key(|triple| triple.iter().map(var_count).sum::<usize>());
    pats
}

/// Find ONE mapping of `pats` into `closure`, or diagnose why there is none.
///
/// # Errors
///
/// [`EntailError::MatchBudget`] when the search visits [`MATCH_BUDGET`] candidates without
/// finishing — which is neither a mapping nor the absence of one, and is reported as
/// neither.
pub(crate) fn find_one(
    pats: Vec<PatTriple>,
    closure: &Closure,
) -> Result<Result<Binding, MissReason>, EntailError> {
    let pats = most_constrained_first(pats);
    let mut bound = Binding::new();
    let mut budget = MATCH_BUDGET;
    if solve(
        &pats,
        &closure.index,
        &mut bound,
        &mut budget,
        &mut Collect::First,
    )? {
        return Ok(Ok(bound));
    }
    // Diagnose. A pattern that cannot be satisfied even on its own names a conclusion the
    // closure simply does not hold; if every pattern is individually satisfiable then the
    // failure is joint, and naming any one of them would be misleading.
    let mut orphans = Vec::new();
    for pat in &pats {
        let mut solo_bound = Binding::new();
        let mut solo_budget = MATCH_BUDGET;
        if !solve(
            std::slice::from_ref(pat),
            &closure.index,
            &mut solo_bound,
            &mut solo_budget,
            &mut Collect::First,
        )? {
            orphans.push(show_pattern(pat));
        }
    }
    Ok(Err(if orphans.is_empty() {
        MissReason::NoConsistentMapping
    } else {
        MissReason::NoCandidate(orphans)
    }))
}

/// Enumerate the projections of EVERY mapping of `pats` into `closure`.
///
/// The rows are a `BTreeSet`, so they are deduplicated (two mappings that agree on the
/// projected variables are one answer) and ordered by the row itself rather than by the
/// order the search happened to reach them.
///
/// # Errors
///
/// [`EntailError::MatchBudget`], as [`find_one`].
pub(crate) fn find_all(
    pats: Vec<PatTriple>,
    closure: &Closure,
    vars: &[VarKey],
) -> Result<BTreeSet<Vec<TermValue>>, EntailError> {
    let pats = most_constrained_first(pats);
    let mut bound = Binding::new();
    let mut budget = MATCH_BUDGET;
    let mut rows = BTreeSet::new();
    solve(
        &pats,
        &closure.index,
        &mut bound,
        &mut budget,
        &mut Collect::All {
            vars,
            rows: &mut rows,
        },
    )?;
    Ok(rows)
}

/// Substitute `bound` into `pat`, or `None` if a position is still open.
///
/// The `None` case is what makes a warrant CHECKABLE rather than merely readable: a mapping
/// that leaves a conclusion variable unbound produces no triple to look for, and
/// [`verify`](super::verify) rejects it instead of quietly treating the pattern as
/// satisfied.
pub(crate) fn substitute(pat: &Pat, bound: &Binding) -> Option<TermValue> {
    match pat {
        Pat::Ground(term) => Some(term.clone()),
        Pat::Var(key) => bound.get(key).cloned(),
        Pat::Triple(inner) => Some(TermValue::Triple {
            s: Box::new(substitute(&inner[0], bound)?),
            p: Box::new(substitute(&inner[1], bound)?),
            o: Box::new(substitute(&inner[2], bound)?),
        }),
    }
}

/// Render a pattern triple the way a diagnostic prints it.
pub(crate) fn show_pattern(pat: &PatTriple) -> String {
    let render = |position: &Pat| -> String {
        match position {
            Pat::Ground(term) => crate::entails::graph::show(term),
            Pat::Var(VarKey::Blank { label, scope }) => {
                // Diagnostics-only rendering, never RDF document egress — label
                // syntax is not enforced here.
                format!("_:{label}#{}", scope.ordinal())
            }
            Pat::Var(VarKey::Projected(name)) => format!("?{name}"),
            Pat::Triple(_) => "<<…>>".to_owned(),
        }
    };
    format!(
        "{} {} {}",
        render(&pat[0]),
        render(&pat[1]),
        render(&pat[2])
    )
}

#[cfg(test)]
mod tests {
    use purrdf_core::BlankScope;

    use super::{Closure, MissReason, find_all, find_one};
    use crate::entails::pattern::{Pat, PatTriple, VarKey};
    use purrdf_core::TermValue;

    fn iri(value: &str) -> TermValue {
        TermValue::Iri(value.to_owned())
    }

    fn ground(value: &str) -> Pat {
        Pat::Ground(iri(value))
    }

    fn blank(label: &str) -> Pat {
        Pat::Var(VarKey::Blank {
            label: label.to_owned(),
            scope: BlankScope::DEFAULT,
        })
    }

    fn projected(name: &str) -> Pat {
        Pat::Var(VarKey::Projected(name.to_owned()))
    }

    fn closure_of(triples: &[[&str; 3]]) -> Closure {
        Closure::of(
            triples
                .iter()
                .map(|[s, p, o]| [iri(s), iri(p), iri(o)])
                .collect(),
        )
    }

    fn matches(closure: &Closure, target: Vec<PatTriple>) -> bool {
        find_one(target, closure).expect("within budget").is_ok()
    }

    /// A fully ground question is present or it is not — the base case the whole search
    /// degenerates to when nothing is existential.
    #[test]
    fn ground_target_must_be_present() {
        let closure = closure_of(&[["s", "p", "o"]]);
        assert!(matches(
            &closure,
            vec![[ground("s"), ground("p"), ground("o")]]
        ));
        assert!(!matches(
            &closure,
            vec![[ground("s"), ground("p"), ground("x")]]
        ));
    }

    /// A blank node of the question is an EXISTENTIAL: it may map to anything.
    #[test]
    fn blank_nodes_are_existentials() {
        let closure = closure_of(&[["s", "p", "o"]]);
        assert!(matches(
            &closure,
            vec![[blank("b"), ground("p"), ground("o")]]
        ));
        assert!(matches(
            &closure,
            vec![[blank("b"), ground("p"), blank("c")]]
        ));
    }

    /// …but ONE existential is one node, not one per occurrence.
    #[test]
    fn a_repeated_blank_must_map_consistently() {
        // `_:b p o1 . _:b p o2` needs ONE node with both edges; a closure that gives two
        // different nodes with one edge each must NOT match.
        let closure = closure_of(&[["s1", "p", "o1"], ["s2", "p", "o2"]]);
        let target = || {
            vec![
                [blank("b"), ground("p"), ground("o1")],
                [blank("b"), ground("p"), ground("o2")],
            ]
        };
        assert!(!matches(&closure, target()));
        let wider = closure_of(&[["s1", "p", "o1"], ["s2", "p", "o2"], ["s1", "p", "o2"]]);
        assert!(matches(&wider, target()));
    }

    /// A bad first choice is UNDONE, not lived with.
    #[test]
    fn matching_backtracks_over_a_bad_first_choice() {
        // The first candidate for `_:b p ?` is `s1`, which cannot satisfy the second
        // pattern; the search must undo it and try `s2`.
        let closure = closure_of(&[["s1", "p", "o1"], ["s2", "p", "o1"], ["s2", "q", "o2"]]);
        assert!(matches(
            &closure,
            vec![
                [blank("b"), ground("p"), ground("o1")],
                [blank("b"), ground("q"), ground("o2")],
            ]
        ));
    }

    /// Two literals with the same lexical form and different datatypes are different terms.
    #[test]
    fn literals_compare_on_datatype_too() {
        let literal = |lexical: &str, datatype: &str| TermValue::Literal {
            lexical_form: lexical.to_owned(),
            datatype: datatype.to_owned(),
            language: None,
            direction: None,
        };
        let closure = Closure::of(vec![[iri("s"), iri("p"), literal("1", "xsd:integer")]]);
        assert!(matches(
            &closure,
            vec![[
                ground("s"),
                ground("p"),
                Pat::Ground(literal("1", "xsd:integer"))
            ]]
        ));
        assert!(!matches(
            &closure,
            vec![[
                ground("s"),
                ground("p"),
                Pat::Ground(literal("1", "xsd:string"))
            ]]
        ));
    }

    /// A predicate the closure never uses ends the search at the index lookup.
    #[test]
    fn an_absent_predicate_short_circuits() {
        let closure = closure_of(&[["s", "p", "o"]]);
        assert!(!matches(
            &closure,
            vec![[blank("b"), ground("absent"), blank("c")]]
        ));
    }

    /// A miss says WHICH kind of miss it is, and the two are distinguishable.
    #[test]
    fn a_miss_names_the_orphan_or_the_joint_failure() {
        let closure = closure_of(&[["s1", "p", "o1"], ["s2", "p", "o2"]]);
        let Err(MissReason::NoCandidate(orphans)) = find_one(
            vec![[ground("s1"), ground("absent"), ground("o1")]],
            &closure,
        )
        .expect("within budget") else {
            panic!("a pattern with no candidate at all is NoCandidate");
        };
        assert_eq!(orphans.len(), 1);
        let Err(reason) = find_one(
            vec![
                [blank("b"), ground("p"), ground("o1")],
                [blank("b"), ground("p"), ground("o2")],
            ],
            &closure,
        )
        .expect("within budget") else {
            panic!("two nodes cannot satisfy one existential");
        };
        assert_eq!(reason, MissReason::NoConsistentMapping);
    }

    /// A PROJECTED variable enumerates, and it enumerates every distinct binding once.
    #[test]
    fn a_projected_variable_enumerates_its_bindings() {
        let closure = closure_of(&[["s1", "p", "o"], ["s2", "p", "o"], ["s1", "q", "o"]]);
        let rows = find_all(
            vec![[projected("x"), ground("p"), ground("o")]],
            &closure,
            &[VarKey::Projected("x".to_owned())],
        )
        .expect("within budget");
        assert_eq!(
            rows.into_iter().collect::<Vec<_>>(),
            vec![vec![iri("s1")], vec![iri("s2")]]
        );
    }

    /// A variable in PREDICATE position ranges over the whole predicate vocabulary — the
    /// case the predicate index cannot answer with one bucket.
    #[test]
    fn a_variable_predicate_scans_every_bucket() {
        let closure = closure_of(&[["s", "p", "o"], ["s", "q", "o"]]);
        let rows = find_all(
            vec![[ground("s"), projected("r"), ground("o")]],
            &closure,
            &[VarKey::Projected("r".to_owned())],
        )
        .expect("within budget");
        assert_eq!(
            rows.into_iter().collect::<Vec<_>>(),
            vec![vec![iri("p")], vec![iri("q")]]
        );
    }

    /// How many levels the depth tests below drive the search to.
    ///
    /// One level is one pattern, so this is a question of 25 000 triples. It is far past
    /// the depth call frames could reach on any stack this library runs on — roughly seven
    /// thousand on a native thread's 8 MiB, about eight times fewer on a `wasm32` module's
    /// 1 MiB, and fewer again on the thread the test harness spawns — so a search that went
    /// back to recursing would abort the process here rather than fail an assertion.
    const DEPTH: usize = 25_000;

    /// A closure of [`DEPTH`] triples, one per predicate `p0…`, all sharing `s` and `o`.
    ///
    /// Distinct predicates are the point: each index bucket holds exactly one candidate, so
    /// the search spends one budget unit per level and `MATCH_BUDGET` — five million — is
    /// nowhere near reached. The budget cannot stand in for a depth bound on this shape.
    fn deep_closure() -> Closure {
        Closure::of(
            (0..DEPTH)
                .map(|level| [iri("s"), iri(&format!("p{level}")), iri("o")])
                .collect(),
        )
    }

    /// [`DEPTH`] patterns matching [`deep_closure`], with `subject` in subject position.
    fn deep_patterns(subject: &Pat) -> Vec<PatTriple> {
        (0..DEPTH)
            .map(|level| {
                [
                    subject.clone(),
                    Pat::Ground(iri(&format!("p{level}"))),
                    ground("o"),
                ]
            })
            .collect()
    }

    /// A question is answered at a depth the call stack could not have held — CORRECTLY,
    /// and in each of the three directions an answer can go.
    ///
    /// One level of the search is one triple of the question, so a large conclusion is a
    /// deep search. Depth held in call frames is not an error a caller can catch: the
    /// process aborts, nothing unwinds, and a host embedding this library dies with it.
    #[test]
    fn a_deep_question_is_answered_rather_than_overflowing_the_stack() {
        let closure = deep_closure();

        // Ground: every level is present, so the question maps.
        assert!(matches(&closure, deep_patterns(&ground("s"))));

        // Existential: ONE node has to carry all 25 000 edges, so the binding made at the
        // first level must survive to the last — the mapping is not merely level-local.
        let binding = find_one(deep_patterns(&blank("b")), &closure)
            .expect("within budget")
            .expect("one node carries every edge");
        assert_eq!(
            binding.get(&VarKey::Blank {
                label: "b".to_owned(),
                scope: BlankScope::DEFAULT,
            }),
            Some(&iri("s")),
        );

        // A miss at the deepest level is diagnosed, not lost: the diagnosis re-runs the
        // search once per pattern, so it is 25 000 more searches.
        let mut missing = deep_patterns(&ground("s"));
        missing.push([ground("s"), ground("absent"), ground("o")]);
        let Err(MissReason::NoCandidate(orphans)) =
            find_one(missing, &closure).expect("within budget")
        else {
            panic!("the closure has no `absent` bucket at all");
        };
        assert_eq!(orphans, vec!["<s> <absent> <o>".to_owned()]);

        // And enumeration reaches the same depth: one mapping, so one row.
        let rows = find_all(
            deep_patterns(&projected("x")),
            &closure,
            &[VarKey::Projected("x".to_owned())],
        )
        .expect("within budget");
        assert_eq!(rows.into_iter().collect::<Vec<_>>(), vec![vec![iri("s")]]);
    }

    /// The budget is still the backstop against combinatorial blowup, and still an ERROR.
    ///
    /// Moving depth onto the heap removed a limit that was never a limit; it did not remove
    /// the one that is. A question whose branching factor is large exhausts the budget and
    /// says so, rather than returning "no" — "I stopped looking" is not "there is nothing".
    #[test]
    fn an_exploding_search_still_refuses_by_budget() {
        // 60 subjects each carrying `p`, and a question of 60 patterns whose subjects are
        // all distinct existentials over `p`: 60^60 mappings, none of which satisfies the
        // final pattern, so the search cannot stop early. The final pattern carries a
        // variable of its own so that `most_constrained_first` cannot hoist it to the front
        // and end the search before it branches.
        let closure = Closure::of(
            (0..60)
                .map(|node| [iri(&format!("s{node}")), iri("p"), iri("o")])
                .collect(),
        );
        let mut pats: Vec<PatTriple> = (0..60)
            .map(|slot| [blank(&format!("b{slot}")), ground("p"), ground("o")])
            .collect();
        pats.push([blank("last"), ground("absent"), ground("o")]);
        let refused = find_one(pats, &closure).expect_err("the budget stops it");
        assert!(
            matches!(refused, crate::EntailError::MatchBudget),
            "{refused}"
        );
    }

    /// The closure's membership test and its candidate index agree about what it holds.
    #[test]
    fn the_closure_holds_exactly_what_it_was_given() {
        let closure = closure_of(&[["s", "p", "o"], ["s", "p", "o"], ["s", "q", "o"]]);
        assert_eq!(closure.len(), 2, "a duplicate triple is one triple");
        assert!(closure.contains(&[iri("s"), iri("p"), iri("o")]));
        assert!(!closure.contains(&[iri("s"), iri("p"), iri("x")]));
    }
}
