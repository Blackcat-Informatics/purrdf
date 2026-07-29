// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The content-addressed plan cache: compile a [`DlClause`] program once, reuse the
//! immutable [`Executable`] every time the same program is presented again.
//!
//! # Content addressing, not identity
//!
//! A cached plan is keyed by a [`PlanIdentity`] — a BLAKE3 digest over the planner's
//! version, the caller's reasoning-contract hash and a canonical digest of the clause
//! program ([`canonical_rule_hash`]). Two structurally identical programs therefore share
//! one cache entry no matter how they were built, and two programs that differ anywhere an
//! execution can observe never share one.
//!
//! BLAKE3 rather than the crate's `ahash` interner hasher: `ahash` is explicitly not
//! version-stable, so its output cannot address content — an `ahash` key would silently
//! change meaning across a dependency bump. Only [`blake3::Hasher::update`] is used, never
//! `update_rayon`, so hashing is sequential on every target and the `wasm32` build carries
//! no thread pool.
//!
//! # The cache is owned, never global
//!
//! There is no process-global cache and no interior mutability: a [`PlanCache`] belongs to
//! whichever planner created it and is threaded through by `&mut`. A process-global would
//! make a lookup's reported cost depend on what some unrelated earlier evaluation happened
//! to compile, which is exactly the hidden state that breaks reproducibility. Two runs of
//! the same program against a fresh cache produce identical results *and* identical
//! [`PlanLookup`] cost coordinates.
//!
//! # Determinism
//!
//! Entries live in a `BTreeMap` keyed by the digest; eviction picks the entry with the
//! smallest use stamp, and stamps come from a strictly increasing counter, so the victim is
//! unique and is a pure function of the call sequence. No map iteration order reaches an
//! output.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::clause::{ClauseAtom, ClauseTerm, DlClause};
use crate::plan::Executable;
use crate::seminaive::{EvalError, compile};

/// Version of the planner and its executable-kernel shape.
///
/// Deliberately independent of the clause digest and the caller's contract hash: a change
/// to index selection, to the sideways-information-passing order or to the cyclic
/// certification invalidates every cached plan even though its logical input is unchanged.
pub const PLAN_SOLVER_VERSION: &str = "purrdf-datalog-plan-v1";

/// Domain-separation tag for [`canonical_rule_hash`].
///
/// Bumped whenever the DL-clause IR grows a field an execution can observe, so a digest
/// computed under an older encoding can never be mistaken for one computed under the
/// current encoding. `v2`: the head gained its disjunct/conjunct nesting level. `v3`: an
/// atom became an arity-4 quad — the predicate is now a TERM (so a variable predicate and
/// a constant one hash under different variant tags, where before every predicate was one
/// length-prefixed string) and the graph position joined the encoding.
const CLAUSE_IR_DIGEST_TAG: &str = "purrdf-datalog-dl-clause-ir-v3";

/// Domain-separation tag for [`PlanIdentity`].
const PLAN_IDENTITY_TAG: &str = "purrdf-datalog-plan-identity-v1";

/// Length-prefix `bytes` into `hasher`.
///
/// Every variable-length field is framed, so no concatenation of two fields can be
/// confused with a different split of the same bytes.
fn frame(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

/// Length-prefix `value`'s UTF-8 bytes into `hasher`.
fn frame_str(hasher: &mut blake3::Hasher, value: &str) {
    frame(hasher, value.as_bytes());
}

/// Hash one clause term under an explicit variant tag.
fn hash_term(hasher: &mut blake3::Hasher, term: &ClauseTerm) {
    match term {
        ClauseTerm::Var(name) => {
            hasher.update(&[0]);
            frame_str(hasher, name);
        }
        ClauseTerm::Iri(iri) => {
            hasher.update(&[1]);
            frame_str(hasher, iri);
        }
        ClauseTerm::Literal(surface) => {
            hasher.update(&[2]);
            frame_str(hasher, surface);
        }
        ClauseTerm::DefaultGraph => {
            hasher.update(&[3]);
        }
    }
}

/// Hash one clause atom: all FOUR positions, polarity included.
///
/// The predicate goes through [`hash_term`] exactly like the subject, the object and the
/// graph, so a variable predicate carries the `Var` tag and a constant one the `Iri` tag:
/// `T(?x, ?p, ?y, ?g)` and `T(?x, <p>, ?y, ?g)` are different programs and cannot share a
/// digest. The graph position is hashed unconditionally, so two rules that differ only in
/// which graph they read or write address differently too.
///
/// The polarity byte is emitted for head atoms too, where it is always `0`: one atom
/// encoder for both positions is cheaper to keep correct than two that must agree.
fn hash_atom(hasher: &mut blake3::Hasher, atom: &ClauseAtom) {
    for term in atom.terms() {
        hash_term(hasher, term);
    }
    hasher.update(&[u8::from(atom.is_negated())]);
}

/// A canonical digest of every execution-relevant field of a DL-clause program.
///
/// # What is hashed
///
/// The whole IR: for each clause, its body literals (subject, predicate, object, GRAPH and
/// polarity — the predicate encoded as a term, so a variable one and a constant one are
/// distinguishable), its existential quantifier list, and its head — a disjunction of
/// conjunctions, encoded at BOTH nesting levels. Every variable-length field is
/// length-prefixed and every enum carries an explicit tag, so this is a structural digest
/// and not a rendering of a debug format.
///
/// # The head's two levels are separately framed
///
/// The head emits the disjunct count, then per disjunct its conjunct count followed by
/// its atoms. Framing only the flattened atom sequence would let `(p ∨ q)` and `(p ∧ q)`
/// — two clauses with entirely different semantics, one a case split and one a joint
/// assertion — hash to the same bytes. With both counts framed they cannot: the
/// disjunction leads with `2, 1, p, 1, q` and the conjunction with `1, 2, p, q`.
///
/// # Order sensitivity
///
/// The digest is **order-sensitive in all five respects**, because nothing in the IR is a
/// set whose order is unobservable:
///
/// * **clause order** — a [`Derivation`](crate::seminaive::Derivation) names its producing
///   clause by authored index, and the round's winner tiebreak compares that index, so
///   permuting a program's clauses changes its output;
/// * **body order** — a derivation reports its sources in authored body order, and the
///   planner's sideways-information-passing order breaks ties on authored position;
/// * **head-disjunct order** — a case split over a disjunctive head has to branch in some
///   deterministic order, and the only order the IR carries is the authored one;
/// * **conjunct order** — the atoms of one disjunct are asserted in authored order, and
///   [`RulePlan`](crate::plan::RulePlan) already assigns variable slots in first-occurrence
///   order across the body and then the head atoms, so permuting them permutes the plan;
/// * **existential order** — a Skolem witness is addressed by the frontier and by the
///   quantifier's position in `ȳ`.
///
/// Hashing any of the five order-insensitively would let two programs with different
/// observable behaviour share a cached plan.
pub fn canonical_rule_hash(rules: &[DlClause]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    frame_str(&mut hasher, CLAUSE_IR_DIGEST_TAG);
    hasher.update(&(rules.len() as u64).to_le_bytes());
    for rule in rules {
        hasher.update(&(rule.body().len() as u64).to_le_bytes());
        for atom in rule.body() {
            hash_atom(&mut hasher, atom);
        }
        hasher.update(&(rule.existentials().len() as u64).to_le_bytes());
        for name in rule.existentials() {
            frame_str(&mut hasher, name);
        }
        hasher.update(&(rule.head_disjuncts().len() as u64).to_le_bytes());
        for disjunct in rule.head_disjuncts() {
            hasher.update(&(disjunct.atoms().len() as u64).to_le_bytes());
            for atom in disjunct.atoms() {
                hash_atom(&mut hasher, atom);
            }
        }
    }
    *hasher.finalize().as_bytes()
}

/// The content address of one compiled plan.
///
/// Three inputs fold into one 32-byte digest: the planner version
/// ([`PLAN_SOLVER_VERSION`]), the caller's reasoning-contract hash, and
/// [`canonical_rule_hash`] of the program. The contract hash alone can never alias two
/// programs, and the clause digest alone can never survive a planner change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanIdentity {
    /// The folded content address — the cache key.
    digest: [u8; 32],
}

impl PlanIdentity {
    /// The identity of `rules` compiled under `contract_hash` by this planner.
    ///
    /// `contract_hash` is whatever digest the caller uses to identify the *configuration*
    /// a program was derived from (a vocabulary selection, an entailment regime). It is
    /// hashed, never interpreted: this crate mints no vocabulary of its own.
    pub fn new(contract_hash: &str, rules: &[DlClause]) -> Self {
        let mut hasher = blake3::Hasher::new();
        frame_str(&mut hasher, PLAN_IDENTITY_TAG);
        frame_str(&mut hasher, PLAN_SOLVER_VERSION);
        frame_str(&mut hasher, contract_hash);
        hasher.update(&canonical_rule_hash(rules));
        Self {
            digest: *hasher.finalize().as_bytes(),
        }
    }

    /// The 32-byte content address.
    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

/// The static size of a planning job, as a deterministic cost coordinate.
///
/// One unit per clause plus one per body literal, head ATOM and existential quantifier —
/// the IR nodes a compile actually walks. Head atoms are counted across all disjuncts,
/// because that is what the planner and the range-restriction check visit; the grouping
/// itself is not a node either of them inspects. It is a count, never a duration, so a
/// cost report never depends on wall time or on machine load.
fn static_planning_units(rules: &[DlClause]) -> u64 {
    rules.iter().fold(0_u64, |units, rule| {
        units
            .saturating_add(1)
            .saturating_add(rule.body().len() as u64)
            .saturating_add(rule.head_atoms().count() as u64)
            .saturating_add(rule.existentials().len() as u64)
    })
}

/// The outcome of one [`PlanCache::get_or_compile`].
///
/// `plan_builds` and `planning_units` are deterministic cost coordinates: a cold lookup
/// reports one build and the exact number of static IR nodes inspected, a warm lookup
/// reports zero for both. Neither observes a clock.
#[derive(Debug, Clone)]
pub struct PlanLookup {
    /// The compiled program, or the refusal that compiling it produced.
    plan: Result<Arc<Executable>, EvalError>,
    /// Whether the entry was already cached.
    cache_hit: bool,
    /// `1` for a cold lookup, `0` for a warm one.
    plan_builds: u64,
    /// The static IR nodes a cold compile inspected; `0` for a warm lookup.
    planning_units: u64,
}

impl PlanLookup {
    /// The compiled program, or the refusal that compiling it produced.
    pub fn plan(&self) -> Result<&Arc<Executable>, &EvalError> {
        self.plan.as_ref()
    }

    /// Take ownership of the compiled program or of the refusal.
    pub fn into_plan(self) -> Result<Arc<Executable>, EvalError> {
        self.plan
    }

    /// Whether the entry was already cached.
    pub fn cache_hit(&self) -> bool {
        self.cache_hit
    }

    /// `1` for a cold lookup, `0` for a warm one.
    pub fn plan_builds(&self) -> u64 {
        self.plan_builds
    }

    /// The static IR nodes a cold compile inspected; `0` for a warm lookup.
    pub fn planning_units(&self) -> u64 {
        self.planning_units
    }
}

/// One cached compilation.
#[derive(Debug)]
struct CachedPlan {
    /// The compiled program, or the refusal — a refused program is cached too, so a
    /// repeatedly-presented bad program is diagnosed once rather than recompiled forever.
    plan: Result<Arc<Executable>, EvalError>,
    /// The use stamp that orders eviction; strictly increasing, so it is unique.
    used: u64,
}

/// A bounded, caller-owned, content-addressed plan cache.
///
/// Compiled plans are immutable [`Arc`] values, so eviction drops only the cache's
/// reference and can never invalidate an in-flight evaluation.
///
/// Lookup is O(log n) in the content address. Eviction sweeps the entries for the smallest
/// use stamp, which is O(n) in a capacity the caller chose and bounded by it — the linear
/// cost is on the rare install path, never on the hit path a hot loop takes.
#[derive(Debug)]
pub struct PlanCache {
    /// The most entries retained; the least recently used is evicted beyond it.
    capacity: usize,
    /// The entries, keyed by content address. A `BTreeMap` rather than a hash table
    /// because eviction sweeps it, so its iteration order is on an output path.
    entries: BTreeMap<PlanIdentity, CachedPlan>,
    /// The strictly increasing use-stamp source.
    clock: u64,
}

impl PlanCache {
    /// A cache retaining at most `capacity` compiled programs.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is zero: a cache that can hold nothing is a silently disabled
    /// cache, and a caller that wants no caching should not construct one.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "a plan cache's capacity must be non-zero");
        Self {
            capacity,
            entries: BTreeMap::new(),
            clock: 0,
        }
    }

    /// The number of entries currently retained.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache holds no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Look `rules` up by content address, compiling them on a miss.
    ///
    /// A refusal is cached exactly like a success, so a program that cannot compile is
    /// diagnosed once and then answered from the cache — the refusal is a *result*, not a
    /// failure to produce one.
    pub fn get_or_compile(&mut self, contract_hash: &str, rules: Vec<DlClause>) -> PlanLookup {
        let identity = PlanIdentity::new(contract_hash, &rules);
        let stamp = self.tick();
        if let Some(entry) = self.entries.get_mut(&identity) {
            entry.used = stamp;
            return PlanLookup {
                plan: entry.plan.clone(),
                cache_hit: true,
                plan_builds: 0,
                planning_units: 0,
            };
        }

        let planning_units = static_planning_units(&rules);
        let plan = compile(rules).map(Arc::new);
        self.insert(identity, plan.clone());
        PlanLookup {
            plan,
            cache_hit: false,
            plan_builds: 1,
            planning_units,
        }
    }

    /// The next use stamp.
    fn tick(&mut self) -> u64 {
        self.clock = self.clock.saturating_add(1);
        self.clock
    }

    /// Install `plan`, evicting the least recently used entry if the cache is full.
    fn insert(&mut self, identity: PlanIdentity, plan: Result<Arc<Executable>, EvalError>) {
        if self.entries.len() >= self.capacity {
            // Use stamps come from a strictly increasing counter, so the minimum is unique
            // and the victim is a pure function of the call sequence — and the map is a
            // `BTreeMap`, so even an impossible tie would break by content address rather
            // than by table layout.
            let victim = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.used)
                .map(|(identity, _)| *identity);
            if let Some(victim) = victim {
                self.entries.remove(&victim);
            }
        }
        let used = self.tick();
        self.entries.insert(identity, CachedPlan { plan, used });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clause::HeadDisjunct;
    use crate::seminaive::evaluate;
    use crate::store::RelationStore;

    const P: &str = "https://example.org/p";
    const Q: &str = "https://example.org/q";
    const R: &str = "https://example.org/r";
    const CONTRACT: &str = "contract-a";

    fn v(name: &str) -> ClauseTerm {
        ClauseTerm::var(name)
    }

    fn atom(subject: &str, predicate: &str, object: &str) -> ClauseAtom {
        ClauseAtom::positive(v(subject), predicate, v(object))
    }

    /// `q(?s, ?o) :- p(?s, ?o).` — the smallest complete program.
    fn transitive_step(predicate: &str) -> Vec<DlClause> {
        vec![DlClause::datalog(
            atom("?s", predicate, "?o"),
            vec![atom("?s", P, "?o")],
        )]
    }

    /// Two rules that derive the same head from different sources, so their AUTHORED order
    /// is observable in the winning derivation's rule index.
    fn colliding_rules() -> Vec<DlClause> {
        vec![
            DlClause::datalog(atom("?s", R, "?o"), vec![atom("?s", P, "?o")]),
            DlClause::datalog(atom("?s", R, "?o"), vec![atom("?s", Q, "?o")]),
        ]
    }

    fn store_of(triples: &[(&str, &str, &str)]) -> RelationStore {
        let mut store = RelationStore::new();
        for &(subject, predicate, object) in triples {
            store.insert(
                &format!("<{subject}>"),
                &format!("<{predicate}>"),
                &format!("<{object}>"),
                RelationStore::DEFAULT_GRAPH,
            );
        }
        store
    }

    /// A rendering of every per-rule plan, byte for byte.
    fn plan_bytes(exe: &Executable) -> String {
        use std::fmt::Write as _;

        let mut out = String::new();
        for index in 0..exe.rule_count() {
            let (rule, plan) = exe.rule_entry(index);
            writeln!(out, "{rule:?}|{plan:?}").expect("writing into a String cannot fail");
        }
        out
    }

    /// Structurally identical programs share one content address; any structural
    /// difference — predicate, term kind, polarity, contract — produces another.
    #[test]
    fn identical_rule_sets_share_an_identity_and_different_ones_do_not() {
        let left = PlanIdentity::new(CONTRACT, &transitive_step(Q));
        let right = PlanIdentity::new(CONTRACT, &transitive_step(Q));
        assert_eq!(left, right, "the same program addresses to the same digest");
        assert_eq!(left.digest(), right.digest());

        // A different head predicate.
        assert_ne!(left, PlanIdentity::new(CONTRACT, &transitive_step(R)));
        // A different contract hash over the same program.
        assert_ne!(left, PlanIdentity::new("contract-b", &transitive_step(Q)));
        // A negated body literal instead of a positive one.
        let negated = vec![DlClause::datalog(
            atom("?s", Q, "?o"),
            vec![ClauseAtom::negated(v("?s"), P, v("?o"))],
        )];
        assert_ne!(left, PlanIdentity::new(CONTRACT, &negated));
        // A constant IRI where the original had a variable.
        let grounded = vec![DlClause::datalog(
            atom("?s", Q, "?o"),
            vec![ClauseAtom::positive(
                ClauseTerm::iri("https://example.org/a"),
                P,
                v("?o"),
            )],
        )];
        assert_ne!(left, PlanIdentity::new(CONTRACT, &grounded));
        // An IRI constant and a literal constant with the same surface bytes are distinct
        // terms, so their tags must keep them apart.
        let as_iri = vec![DlClause::datalog(
            atom("?s", Q, "?o"),
            vec![ClauseAtom::positive(v("?s"), P, ClauseTerm::iri("x"))],
        )];
        let as_literal = vec![DlClause::datalog(
            atom("?s", Q, "?o"),
            vec![ClauseAtom::positive(v("?s"), P, ClauseTerm::literal("x"))],
        )];
        assert_ne!(
            PlanIdentity::new(CONTRACT, &as_iri),
            PlanIdentity::new(CONTRACT, &as_literal)
        );
    }

    /// The digest separates the head disjunction, the existential list and the body: five
    /// clauses built from the same atoms in different roles address differently.
    #[test]
    fn the_digest_separates_the_clause_fields() {
        let datalog = vec![DlClause::datalog(
            atom("?X", Q, "?Y"),
            vec![atom("?X", P, "?Z")],
        )];
        let existential = vec![DlClause::new(
            vec![HeadDisjunct::atom(atom("?X", Q, "?Y"))],
            vec!["?Y".to_owned()],
            vec![atom("?X", P, "?Z")],
        )];
        let disjunctive = vec![DlClause::new(
            vec![
                HeadDisjunct::atom(atom("?X", Q, "?Y")),
                HeadDisjunct::atom(atom("?X", R, "?Y")),
            ],
            Vec::new(),
            vec![atom("?X", P, "?Z")],
        )];
        let conjunctive = vec![DlClause::new(
            vec![HeadDisjunct::new(vec![
                atom("?X", Q, "?Y"),
                atom("?X", R, "?Y"),
            ])],
            Vec::new(),
            vec![atom("?X", P, "?Z")],
        )];
        let inconsistency = vec![DlClause::inconsistency(vec![atom("?X", P, "?Z")])];
        let digests = [
            canonical_rule_hash(&datalog),
            canonical_rule_hash(&existential),
            canonical_rule_hash(&disjunctive),
            canonical_rule_hash(&conjunctive),
            canonical_rule_hash(&inconsistency),
        ];
        for (i, left) in digests.iter().enumerate() {
            for right in &digests[i + 1..] {
                assert_ne!(left, right, "head forms must not collide");
            }
        }
    }

    /// `(p ∨ q)` and `(p ∧ q)` present the SAME flattened atom sequence and differ only in
    /// how it is grouped, so the digest separates them only because both nesting levels
    /// are length-prefixed. A digest that framed the flattening alone would alias a case
    /// split with a joint assertion.
    #[test]
    fn the_digest_separates_a_disjunction_from_a_conjunction() {
        let body = || vec![atom("?X", P, "?Y")];
        let left = atom("?X", Q, "?Y");
        let right = atom("?X", R, "?Y");

        let disjunction = vec![DlClause::new(
            vec![
                HeadDisjunct::atom(left.clone()),
                HeadDisjunct::atom(right.clone()),
            ],
            Vec::new(),
            body(),
        )];
        let conjunction = vec![DlClause::new(
            vec![HeadDisjunct::new(vec![left, right])],
            Vec::new(),
            body(),
        )];

        // The two heads flatten to the identical atom sequence…
        assert_eq!(
            disjunction[0]
                .head_atoms()
                .map(ClauseAtom::predicate)
                .collect::<Vec<_>>(),
            conjunction[0]
                .head_atoms()
                .map(ClauseAtom::predicate)
                .collect::<Vec<_>>()
        );
        // …and the digest must still tell them apart.
        assert_ne!(
            canonical_rule_hash(&disjunction),
            canonical_rule_hash(&conjunction),
            "a disjunction and a conjunction of the same atoms are different programs"
        );
        assert_ne!(
            PlanIdentity::new(CONTRACT, &disjunction),
            PlanIdentity::new(CONTRACT, &conjunction)
        );
    }

    /// The digest separates a VARIABLE predicate from a constant one, and separates two
    /// different graph positions.
    ///
    /// Both are execution-relevant: `T(?x, ?p, ?y, ?g)` sweeps partitions and binds `?p`
    /// where `T(?x, <p>, ?y, ?g)` addresses one and binds nothing there, and a rule that
    /// reads a named graph reads different data from one that reads the default graph.
    /// Sharing a cached plan across either difference would run the wrong program.
    #[test]
    fn the_digest_separates_the_predicate_and_graph_positions() {
        let head = || atom("?X", Q, "?Y");
        let clause = |body: ClauseAtom| vec![DlClause::datalog(head(), vec![body])];
        let g1 = ClauseTerm::iri("https://example.org/g1");
        let g2 = ClauseTerm::iri("https://example.org/g2");

        // A constant predicate versus a variable one, over the same graph.
        let constant = clause(ClauseAtom::quad(
            v("?X"),
            ClauseTerm::iri(P),
            v("?Y"),
            ClauseTerm::DefaultGraph,
        ));
        let variable = clause(ClauseAtom::quad(
            v("?X"),
            v("?P"),
            v("?Y"),
            ClauseTerm::DefaultGraph,
        ));
        assert_ne!(
            canonical_rule_hash(&constant),
            canonical_rule_hash(&variable),
            "a variable predicate and a constant one are different programs"
        );
        assert_ne!(
            PlanIdentity::new(CONTRACT, &constant),
            PlanIdentity::new(CONTRACT, &variable)
        );
        // The convenience constructor is exactly the constant, default-graph form.
        assert_eq!(
            canonical_rule_hash(&clause(atom("?X", P, "?Y"))),
            canonical_rule_hash(&constant)
        );

        // Two different named graphs, and a named graph versus the default one.
        let in_g1 = clause(ClauseAtom::quad(
            v("?X"),
            ClauseTerm::iri(P),
            v("?Y"),
            g1.clone(),
        ));
        let in_g2 = clause(ClauseAtom::quad(v("?X"), ClauseTerm::iri(P), v("?Y"), g2));
        let in_variable_graph = clause(ClauseAtom::quad(
            v("?X"),
            ClauseTerm::iri(P),
            v("?Y"),
            ClauseTerm::var("?G"),
        ));
        let digests = [
            canonical_rule_hash(&constant),
            canonical_rule_hash(&in_g1),
            canonical_rule_hash(&in_g2),
            canonical_rule_hash(&in_variable_graph),
        ];
        for (i, left) in digests.iter().enumerate() {
            for right in &digests[i + 1..] {
                assert_ne!(left, right, "graph positions must not collide");
            }
        }

        // The graph position of a HEAD atom is hashed too, not only a body atom's.
        let head_in_g1 = vec![DlClause::datalog(
            ClauseAtom::quad(v("?X"), ClauseTerm::iri(Q), v("?Y"), g1),
            vec![atom("?X", P, "?Y")],
        )];
        assert_ne!(
            canonical_rule_hash(&head_in_g1),
            canonical_rule_hash(&[DlClause::datalog(head(), vec![atom("?X", P, "?Y")])])
        );
    }

    /// Conjunct order inside one disjunct is hashed: it is the order the atoms are
    /// asserted in, and the order the planner's variable frame is laid out in.
    #[test]
    fn the_digest_is_sensitive_to_conjunct_order() {
        let conjunction = |atoms: Vec<ClauseAtom>| {
            vec![DlClause::new(
                vec![HeadDisjunct::new(atoms)],
                Vec::new(),
                vec![atom("?X", P, "?Y")],
            )]
        };
        assert_ne!(
            canonical_rule_hash(&conjunction(vec![atom("?X", Q, "?Y"), atom("?X", R, "?Y")])),
            canonical_rule_hash(&conjunction(vec![atom("?X", R, "?Y"), atom("?X", Q, "?Y")]))
        );
    }

    /// The digest is order-SENSITIVE in every one of the IR's four sequences, because
    /// every one of them is observable — see [`canonical_rule_hash`].
    #[test]
    fn the_digest_is_sensitive_to_every_authored_order() {
        // Clause order.
        let forward = colliding_rules();
        let mut reversed = forward.clone();
        reversed.reverse();
        assert_ne!(
            canonical_rule_hash(&forward),
            canonical_rule_hash(&reversed),
            "clause order is observable, so it must be hashed"
        );

        // Body order.
        let body_forward = vec![DlClause::datalog(
            atom("?X", R, "?Z"),
            vec![atom("?X", P, "?Y"), atom("?Y", Q, "?Z")],
        )];
        let body_reversed = vec![DlClause::datalog(
            atom("?X", R, "?Z"),
            vec![atom("?Y", Q, "?Z"), atom("?X", P, "?Y")],
        )];
        assert_ne!(
            canonical_rule_hash(&body_forward),
            canonical_rule_hash(&body_reversed)
        );

        // Head-disjunct order.
        let head_forward = vec![DlClause::new(
            vec![
                HeadDisjunct::atom(atom("?X", Q, "?Y")),
                HeadDisjunct::atom(atom("?X", R, "?Y")),
            ],
            Vec::new(),
            vec![atom("?X", P, "?Y")],
        )];
        let head_reversed = vec![DlClause::new(
            vec![
                HeadDisjunct::atom(atom("?X", R, "?Y")),
                HeadDisjunct::atom(atom("?X", Q, "?Y")),
            ],
            Vec::new(),
            vec![atom("?X", P, "?Y")],
        )];
        assert_ne!(
            canonical_rule_hash(&head_forward),
            canonical_rule_hash(&head_reversed)
        );

        // Existential-quantifier order.
        let quantified = |names: Vec<String>| {
            vec![DlClause::new(
                vec![HeadDisjunct::atom(atom("?Y", Q, "?Z"))],
                names,
                vec![atom("?X", P, "?W")],
            )]
        };
        assert_ne!(
            canonical_rule_hash(&quantified(vec!["?Y".to_owned(), "?Z".to_owned()])),
            canonical_rule_hash(&quantified(vec!["?Z".to_owned(), "?Y".to_owned()]))
        );
    }

    /// Clause order is not hashed out of caution: permuting it MOVES an observable answer,
    /// so an order-insensitive digest would alias two programs that behave differently.
    #[test]
    fn clause_order_that_the_digest_separates_is_observable() {
        let edb = || store_of(&[("a", P, "b"), ("a", Q, "b")]);
        let forward = colliding_rules();
        let mut reversed = forward.clone();
        reversed.reverse();

        let run = |rules: Vec<DlClause>| {
            let exe = compile(rules).expect("the fixture compiles");
            let evaluation = evaluate(&exe, edb()).expect("the fixture stays inside every ceiling");
            let derivation = &evaluation.derivations()[0];
            (derivation.rule(), derivation.sources()[0].predicate.clone())
        };
        assert_eq!(run(forward), (0, format!("<{P}>")));
        assert_eq!(
            run(reversed),
            (1, format!("<{P}>")),
            "the winning derivation names the AUTHORED clause index, which the permutation \
             moved — so the digest must move with it"
        );
    }

    /// Planning the same program twice yields byte-identical plans, so a cached plan and a
    /// freshly compiled one are interchangeable.
    #[test]
    fn the_same_rule_set_plans_byte_identically() {
        let reference = compile(colliding_rules()).expect("the fixture compiles");
        let expected = plan_bytes(&reference);
        for _ in 0..8 {
            let again = compile(colliding_rules()).expect("the fixture compiles");
            assert_eq!(plan_bytes(&again), expected);
        }

        let mut cache = PlanCache::new(4);
        let cold = cache.get_or_compile(CONTRACT, colliding_rules());
        let warm = cache.get_or_compile(CONTRACT, colliding_rules());
        let cold_plan = cold.plan().expect("the fixture compiles");
        let warm_plan = warm.plan().expect("the fixture compiles");
        assert_eq!(plan_bytes(cold_plan), expected);
        assert_eq!(plan_bytes(warm_plan), expected);
        assert!(
            Arc::ptr_eq(cold_plan, warm_plan),
            "a warm lookup returns the SAME immutable plan, not an equal copy"
        );
    }

    /// A cold lookup compiles and reports its static cost; a warm one reports zero for
    /// both coordinates and returns the same plan.
    #[test]
    fn a_warm_lookup_reports_no_work() {
        let mut cache = PlanCache::new(4);
        assert!(cache.is_empty());

        let cold = cache.get_or_compile(CONTRACT, transitive_step(Q));
        assert!(!cold.cache_hit());
        assert_eq!(cold.plan_builds(), 1);
        // One clause, one body literal, one head atom, no existential.
        assert_eq!(cold.planning_units(), 3);
        assert_eq!(cache.len(), 1);

        let warm = cache.get_or_compile(CONTRACT, transitive_step(Q));
        assert!(warm.cache_hit());
        assert_eq!(warm.plan_builds(), 0);
        assert_eq!(warm.planning_units(), 0);
        assert_eq!(cache.len(), 1, "a hit installs nothing");

        // A different contract over the same clauses is a different address.
        let other = cache.get_or_compile("contract-b", transitive_step(Q));
        assert!(!other.cache_hit());
        assert_eq!(cache.len(), 2);
    }

    /// A refusal is a result: it is cached, returned verbatim on the next lookup, and
    /// never recompiled.
    #[test]
    fn a_refusal_is_cached_like_a_success() {
        // A disjunctive head has no Datalog semantics, so compiling it refuses.
        let refused = || {
            vec![DlClause::new(
                vec![
                    HeadDisjunct::atom(atom("?X", Q, "?Y")),
                    HeadDisjunct::atom(atom("?X", R, "?Y")),
                ],
                Vec::new(),
                vec![atom("?X", P, "?Y")],
            )]
        };
        let mut cache = PlanCache::new(4);
        let cold = cache.get_or_compile(CONTRACT, refused());
        let cold_error = cold.plan().expect_err("a disjunctive head is refused");
        assert!(matches!(
            cold_error,
            EvalError::NonDatalogHead { rule: 0, .. }
        ));

        let warm = cache.get_or_compile(CONTRACT, refused());
        assert!(warm.cache_hit());
        assert_eq!(warm.plan_builds(), 0);
        assert_eq!(warm.plan().expect_err("still refused"), cold_error);
    }

    /// The cache is bounded and evicts the LEAST RECENTLY USED entry, so a re-touched
    /// entry survives a newer one.
    #[test]
    fn the_cache_evicts_the_least_recently_used_entry() {
        let mut cache = PlanCache::new(2);
        let first = transitive_step(Q);
        let second = transitive_step(R);
        let third = transitive_step("https://example.org/t");

        cache.get_or_compile(CONTRACT, first.clone());
        cache.get_or_compile(CONTRACT, second.clone());
        // Touch the first, making the SECOND the least recently used.
        assert!(cache.get_or_compile(CONTRACT, first.clone()).cache_hit());
        cache.get_or_compile(CONTRACT, third.clone());

        assert_eq!(cache.len(), 2);
        assert!(
            cache.get_or_compile(CONTRACT, first).cache_hit(),
            "the re-touched entry survives"
        );
        assert!(cache.get_or_compile(CONTRACT, third).cache_hit());
        assert!(
            !cache.get_or_compile(CONTRACT, second).cache_hit(),
            "the least recently used entry was the one evicted"
        );
    }

    /// Two independently constructed caches, driven by the same call sequence, agree on
    /// every cost coordinate: the cache is owned state, not ambient state.
    #[test]
    fn cache_cost_reporting_is_reproducible() {
        let sequence = || {
            let mut cache = PlanCache::new(2);
            let calls = [
                transitive_step(Q),
                transitive_step(R),
                transitive_step(Q),
                transitive_step("https://example.org/t"),
                transitive_step(R),
            ];
            calls
                .into_iter()
                .map(|rules| {
                    let lookup = cache.get_or_compile(CONTRACT, rules);
                    (
                        lookup.cache_hit(),
                        lookup.plan_builds(),
                        lookup.planning_units(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(sequence(), sequence());
    }

    #[test]
    #[should_panic(expected = "capacity must be non-zero")]
    fn a_zero_capacity_cache_is_refused() {
        let _ = PlanCache::new(0);
    }

    /// The solver version participates in the address, so a planner change cannot be
    /// answered from a plan built by an older planner.
    #[test]
    fn the_solver_version_participates_in_the_address() {
        let rules = transitive_step(Q);
        let rule_hash = canonical_rule_hash(&rules);
        let identity = PlanIdentity::new(CONTRACT, &rules);
        assert_ne!(
            identity.digest(),
            &rule_hash,
            "the plan address is not the bare clause digest"
        );
        assert!(!PLAN_SOLVER_VERSION.is_empty());
    }
}
