// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Path **witnesses** as property functions: a relation that binds not merely the
//! endpoints of a traversal but the traversal itself — every hop, in order, as a
//! first-class RDF 1.2 statement term.
//!
//! # Why a walk, and not a node sequence
//!
//! The core SPARQL grammar's property paths answer a *reachability* question: `?s ex:p+
//! ?o` says that some sequence of `ex:p` edges leads from `?s` to `?o`, and the answer
//! is the endpoint pair. The derivation — *which* edges — is not expressible, so a query
//! that needs to explain, weight, filter or re-join the route has nowhere to look.
//!
//! This module's relations answer the derivation question. The object they enumerate is
//! a **walk**
//!
//! ```text
//! n0 --e1--> n1 --e2--> ... --ek--> nk
//! ```
//!
//! where each `ei` is a traversed **statement**, not merely a predicate. That
//! distinction is load-bearing rather than decorative. A [`PathStep`] is an *ordered
//! alternation* of directed predicates, so two DIFFERENT statements can join the SAME
//! pair of nodes — `ex:a ex:p ex:b` and `ex:b ex:q ex:a` both take you from `ex:a` to
//! `ex:b` under the step `(ex:p, Forward) | (ex:q, Inverse)`. A node-only model reports
//! one walk where there are two, silently erasing a derivation. Recording the statement
//! keeps the two apart, and it does so in a form the rest of the query language already
//! understands: in RDF 1.2 a statement IS a term, so each hop is emitted as a
//! [`TermValue::Triple`] in **asserted orientation** (subject, predicate, object as
//! written in the data, whichever way the step traversed it) and joins straight back
//! into the dataset by an ordinary basic graph pattern. Direction is not lost by that
//! choice — it is recoverable by comparing the statement's subject to the node the hop
//! started from.
//!
//! # The row shape
//!
//! Both relations declare [`PfArity::new(1, 6)`](PfArity::new) — one subject-side
//! argument and six object-side ones — so a call reads
//!
//! ```text
//! ?start <caller-iri> ( ?end ?pathId ?len ?step ?node ?edge )
//! ```
//!
//! and the flattened positions are `[0] = start`, `[1] = end`, `[2] = pathId`,
//! `[3] = len`, `[4] = step`, `[5] = node`, `[6] = edge`.
//!
//! **One row per hop.** A walk of `k` hops emits exactly `k` rows; row `i` (1-based) is
//! `(n0, nk, pathId, k, i, n_i, e_i)`. The whole walk is therefore recoverable inside
//! the query language: group by `?pathId`, order by `?step`, and the node sequence is
//! `?start` followed by `?node`, while `?node` at `?step = ?len` is `?end`. `?step` and
//! `?len` are `xsd:integer`-typed literals rather than simple ones precisely so
//! `ORDER BY ?step` orders numerically; a simple literal would sort `"10"` before `"2"`
//! and silently scramble the reconstruction.
//!
//! ## Reconstructing the whole walk: the `GROUP_CONCAT` recipe
//!
//! One row per hop is not a lossy shape. It carries strictly more than an `rdf:List` of
//! nodes would — it names the traversed STATEMENT of every hop, which a node list cannot
//! express — and the node sequence an `rdf:List` would have held is recoverable in the
//! query language itself, with no unfolding operator and no host code:
//!
//! ```text
//! PREFIX ex: <http://example.org/>
//! SELECT ?start ?end ?len (GROUP_CONCAT(?node; separator="->") AS ?route)
//! WHERE {
//!   ?start <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge )
//! }
//! GROUP BY ?pathId ?start ?end ?len
//! ORDER BY ?len ?start
//! ```
//!
//! `GROUP BY ?pathId` is the whole trick: the identifier is constant across one walk's
//! rows and distinct between walks, so each group IS one walk. `?start`, `?end` and
//! `?len` are constant within a group, so grouping by them alongside is free and lets
//! them be projected. The concatenation order is `?step` order, which is why `?step` is
//! an `xsd:integer` and not a simple literal.
//!
//! The same grouping reconstructs the EDGE sequence — swap `?node` for `?edge` — and the
//! same group is where an aggregate over a walk belongs: `SUM` of a per-hop weight joined
//! in through `?edge`, `MIN` of a per-hop confidence, a `HAVING` that keeps only routes
//! whose every hop is annotated. None of that is reachable from a relation that returned
//! endpoints, and none of it needed a list term.
//!
//! # The snapshot is not the queried dataset
//!
//! **A relation is built from a dataset at construction, and the property-function seam
//! hands a relation no dataset at evaluation time.** [`PropertyFunction::open`] receives
//! bound arguments and a row ceiling — nothing else. So a [`PathGraph`] snapshotted from
//! dataset *A* and registered in a registry that is then used to evaluate a query against
//! dataset *B* will answer, silently and without any diagnostic, about *A*'s edges. The
//! query text names no dataset the relation can check itself against, and the seam has no
//! place to put one.
//!
//! This is a precondition on the HOST, and it is not negotiable. Do not try to fix it by
//! reshaping the seam: a relation that took a dataset would be a relation whose answers
//! depended on a value the planner cannot see when it prices the call, and every
//! host-injected relation — a text index, a vector store, a remote service — would then
//! have to explain what it does with a dataset it does not read.
//!
//! The pattern that avoids it is to build the relation where the dataset is chosen, per
//! dataset, and never to hoist a registry above the thing it describes:
//!
//! ```text
//! // WRONG: one registry, built once, evaluated against whatever arrives.
//! let registry = build_walk_registry(&startup_dataset);
//! for dataset in incoming { engine.query_with_options_view(dataset, .., &registry) }
//!
//! // RIGHT: the snapshot's lifetime is the dataset's.
//! for dataset in incoming {
//!     let registry = build_walk_registry(dataset);
//!     engine.query_with_options_view(dataset, .., &registry)
//! }
//! ```
//!
//! A host that caches snapshots deliberately — because rebuilding one per query is real
//! work — asserts the pairing instead of assuming it.
//! [`PathGraph::snapshot_fingerprint`] is the surface for that: it records the source
//! view's [`DatasetView::stats_fingerprint`] and [`DatasetView::term_count`] alongside the
//! snapshot's own node and edge counts, so a cache entry can be compared against the
//! dataset about to be queried and rebuilt when it moves. It is a discriminator, not a
//! content digest, so equality is evidence rather than proof — which is exactly the right
//! strength for a cache key, and exactly the wrong thing to skip.
//!
//! # Wiring one up
//!
//! End to end, from a dataset to answers, using only public API:
//!
//! ```
//! use std::sync::Arc;
//!
//! use purrdf_core::{
//!     GraphMatch, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue,
//! };
//! use purrdf_sparql_eval::{
//!     NativeSparqlEngine, ParserOptions, PathDirection, PathGraph, PathLimits, PathStep,
//!     PathWitnessRelation, PropertyFunctionRegistry, QueryOptions,
//! };
//!
//! // The caller's IRI for this relation. PurRDF mints none.
//! const WALK: &str = "http://example.org/pf#walk";
//!
//! // 1. The data: a three-edge chain.
//! let mut builder = RdfDatasetBuilder::new();
//! let a = builder.intern_iri("http://example.org/a");
//! let b = builder.intern_iri("http://example.org/b");
//! let c = builder.intern_iri("http://example.org/c");
//! let p = builder.intern_iri("http://example.org/p");
//! builder.push_quad(a, p, b, None);
//! builder.push_quad(b, p, c, None);
//! let dataset = builder.freeze()?;
//!
//! // 2. The step, and the snapshot of it over THIS dataset (see the precondition above).
//! let step = PathStep::new(vec![(
//!     TermValue::iri("http://example.org/p"),
//!     PathDirection::Forward,
//! )])?;
//! let graph = Arc::new(PathGraph::from_dataset(&*dataset, &step, GraphMatch::Default)?);
//!
//! // 3. The envelope. There is no `Default`: the host states what it will buy.
//! let limits = PathLimits::new(1, 4, 1_024, 100_000)?;
//!
//! // 4. Register the relation under the caller's IRI.
//! let mut registry = PropertyFunctionRegistry::new();
//! registry.register(WALK.to_owned(), Arc::new(PathWitnessRelation::new(graph, limits)));
//!
//! // 5. Parse-time recognition. Without the IRI here the same text is an ordinary
//! //    triple pattern reading the graph.
//! let engine = NativeSparqlEngine::new().with_parser_options(ParserOptions {
//!     extension_fn_namespaces: Vec::new(),
//!     property_fn_namespaces: Vec::new(),
//!     property_fn_iris: vec![WALK.to_owned()],
//! });
//!
//! // 6. Run.
//! let query = format!(
//!     "SELECT ?end ?len ?step ?node ?edge WHERE {{ \
//!      <http://example.org/a> <{WALK}> ( ?end ?pathId ?len ?step ?node ?edge ) \
//!      }} ORDER BY ?len ?step"
//! );
//! let result = engine.query_with_options_view(
//!     &*dataset,
//!     SparqlRequest { query: &query, base_iri: None, substitutions: &[] },
//!     QueryOptions { property_functions: &registry, ..QueryOptions::EMPTY },
//! )?;
//!
//! let SparqlResult::Solutions { rows, .. } = result else { unreachable!("a SELECT") };
//! // a→b (one row) and a→b→c (two rows).
//! assert_eq!(rows.len(), 3);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Walk semantics: the simple-prefix walk
//!
//! Every node on an enumerated walk is distinct, EXCEPT that the FINAL node may repeat
//! an earlier node. Such a walk closes a cycle and **terminates** there: it is emitted,
//! and it is not extended further.
//!
//! Three properties follow, and each one is why the rule is this and not something
//! simpler:
//!
//! * **It is finite on cyclic input.** The proper prefix `n0 … n_{k-1}` is simple, so
//!   `k` is bounded by the snapshot's node count — and independently by
//!   [`PathLimits::max_hops`]. No visited-set-free enumeration is needed, and no cycle
//!   can diverge.
//! * **Its endpoint projection equals `p+`.** When `max_hops` is at least the node
//!   count, the set of `(?start, ?end)` pairs this relation produces is exactly the
//!   reachability relation the core grammar's `p+` computes over the same edges. A
//!   *strictly* simple rule would not have that property: under it a node never reaches
//!   itself, whereas `p+` says a node reaches itself precisely when it lies on a cycle.
//!   Allowing the final node alone to repeat restores exactly those answers and no
//!   others.
//! * **It is not the same as the ALP fixpoint for EXACT repetition.** `p{n,m}` in the
//!   core grammar is `k`-fold composition, which admits walks that revisit INTERIOR
//!   nodes. Over `ex:a ex:p ex:b . ex:b ex:p ex:a .` the query `ex:a ex:p{2,2} ?x` binds
//!   `?x = ex:a` through the walk `a → b → a`, which is simple-prefix and which this
//!   relation therefore also finds. But a longer walk that revisits an interior node —
//!   `a → b → a → b` for `p{3,3}` — is NOT enumerated here, because its prefix
//!   `a → b → a` is not simple. That is the exact boundary: this relation is complete
//!   for reachability and for every simple-prefix derivation of it, and it is
//!   deliberately incomplete for interior-revisiting derivations, which are unbounded in
//!   number on any cyclic graph and so have no finite witness enumeration at all.
//!
//! # Two relations, not one relation with a mode switch
//!
//! [`PathWitnessRelation`] enumerates EVERY simple-prefix walk. That is the complete
//! answer to the derivation question, and its cardinality is exponential in the worst
//! case, because the number of simple paths in a dense graph is.
//!
//! [`ShortestPathWitnessRelation`] yields ONE shortest witness per reachable
//! `(seed, end)` pair, breadth-first with per-node best-depth pruning. Its cardinality
//! is polynomial — at most one walk per node pair — and it is the form most "how is
//! `a` connected to `b`" questions actually want. It is the analogue of Virtuoso's
//! `T_SHORTEST_ONLY` transitivity option.
//!
//! They are two public TYPES over one shared [`Arc<PathGraph>`], not one type carrying a
//! runtime mode flag. A mode flag would make cardinality — the single most important
//! thing the planner reads from a relation, via
//! [`PropertyFunction::rows_per_invocation`] — a property of a value the planner cannot
//! see. As two types, "exponential" and "polynomial" are two different registrations
//! under two different IRIs, and a host that wants only the cheap one simply never
//! registers the other.
//!
//! # The walk identifier
//!
//! `?pathId` is a simple literal holding the lowercase hex of the **full, untruncated**
//! 32-byte SHA-256 of
//!
//! ```text
//! PATH_ID_DOMAIN_V1 || cb(n0) || ( cb(e_i) || cb(n_i) )* || u64_le(hop_count)
//! ```
//!
//! where `cb` is [`TermValue::canonical_bytes`], the injective encoding. The digest is
//! not truncated because the identifier is a GROUPING KEY in query answers — `GROUP BY
//! ?pathId` is how a caller reassembles a walk from its hop rows — so a collision is not
//! a slowdown, it is two walks fused into one wrong answer, and this crate does not trade
//! correctness for bytes.
//!
//! ## What the digest deliberately does NOT cover
//!
//! * **The graph selector.** [`GraphMatch<D::Id>`](GraphMatch) is parameterised by
//!   DATASET-LOCAL ids, so folding it in would make the identifier a function of intern
//!   order — two datasets holding identical data would mint different identifiers purely
//!   because their term tables were built in different orders.
//! * **The step definition.** Two steps that happen to admit the same walk describe the
//!   same derivation, and a caller comparing results across two configured relations
//!   should see that.
//! * **The limits.** The same walk must get the same identifier under a `max_hops` of 4
//!   and of 400; an envelope decides which walks are enumerated, never what they are.
//!
//! ## Stability scope, stated honestly
//!
//! An identifier is identical across runs, across processes, across row ceilings, across
//! read-subsets of the same data, and across independently built datasets holding the
//! same data — **except** for a walk whose nodes or statements contain a blank node.
//! [`TermValue::Blank`] carries a label and a scope, both of which depend on how the data
//! was parsed, so two datasets that are isomorphic but were loaded differently will mint
//! different identifiers for the same blank-node-bearing walk. That is not a defect of
//! the digest: it is blank-node identity, and no content-derived key can be stabler than
//! the terms it is derived from.
//!
//! # Determinism
//!
//! Every ordered surface here is a pure function of the snapshot's contents:
//!
//! * Node indices are assigned in [`TermValue`] `Ord` order, so sorting by dense index
//!   IS sorting by value, and no map iteration order (hash-seeded or otherwise) can
//!   reach an emitted row.
//! * Each node's neighbour list is frozen in `(to, statement)` order at snapshot time.
//! * Seeds are visited in ascending node index — that is, ascending `TermValue` order.
//!
//! See each relation's own emission-order contract for the resulting row order.

use std::collections::VecDeque;
use std::sync::{Arc, OnceLock};

use core::fmt::Write as _;

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{DatasetView, GraphMatch, TermValue};
use sha2::{Digest, Sha256};

use crate::error::EvalError;
use crate::property_fn::{PfArgs, PfArity, PfCursor, PfRow, PropertyFunction};
use crate::user_fn::Volatility;

// ---------------------------------------------------------------------------
// The step definition
// ---------------------------------------------------------------------------

/// Which way a [`PathStep`] alternative traverses the statements of its predicate.
///
/// The statement recorded on a hop is the ASSERTED triple either way (see the module
/// docs); this only says which of its two ends the walk arrives at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PathDirection {
    /// Traverse `subject → object`: the walk arrives at the statement's object.
    Forward,
    /// Traverse `object → subject`: the walk arrives at the statement's subject.
    Inverse,
}

/// One hop's worth of edge definition: an **ordered alternation** of directed
/// predicates.
///
/// A step is what a single arrow of a walk may be. `(ex:p, Forward) | (ex:q, Inverse)`
/// means a hop may follow an `ex:p` statement from its subject to its object, or an
/// `ex:q` statement from its object to its subject. The alternation is ordered because
/// its order is one of the inputs to the snapshot's frozen neighbour order, and hence to
/// emission order — but note that the ORDER of alternatives does not change WHICH walks
/// exist, only the shape of the intermediate lists before they are sorted into
/// `(to, statement)` order, so two callers who list the same alternatives differently
/// still get byte-identical results.
///
/// # Duplicates are refused, not tolerated
///
/// [`new`](Self::new) hard-errors on a repeated `(predicate, direction)` pair, for the
/// same reason
/// [`PropertyFunctionRegistry::register`](crate::property_fn::PropertyFunctionRegistry::register)
/// panics on a repeated IRI: a duplicate is silent at the call site and wrong in the
/// answer. Listing `(ex:p, Forward)` twice would record every `ex:p` statement as two
/// edges, so every walk through such a hop would be enumerated twice, with two
/// identifiers, and there is no spelling of the query that could reveal the difference
/// between "this graph has two derivations" and "the host wrote the predicate down
/// twice". A configuration that can only be wrong is caught where it is committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathStep {
    alternatives: Vec<(TermValue, PathDirection)>,
}

impl PathStep {
    /// Build a step from its ordered alternation.
    ///
    /// # Errors
    ///
    /// [`EvalError::Config`] when:
    ///
    /// * `alternatives` is empty — a step that can traverse nothing is not a step whose
    ///   walks are all length zero, it is a configuration the caller has not finished
    ///   writing;
    /// * any alternative's term is not a [`TermValue::Iri`] — RDF predicates are IRIs,
    ///   and a literal or blank node in predicate position names no statements at all;
    /// * a `(predicate, direction)` pair is repeated — see the type docs.
    pub fn new(alternatives: Vec<(TermValue, PathDirection)>) -> Result<Self, EvalError> {
        if alternatives.is_empty() {
            return Err(EvalError::config(
                "a path step needs at least one (predicate, direction) alternative; an empty \
                 alternation traverses nothing and so defines no hop",
            ));
        }
        for (index, (predicate, _)) in alternatives.iter().enumerate() {
            if !matches!(predicate, TermValue::Iri(_)) {
                return Err(EvalError::config(format!(
                    "path step alternative {index} is {predicate:?}; a predicate must be an IRI"
                )));
            }
        }
        // Quadratic in the alternation's length, which is a hand-written list of a
        // handful of predicates — and quadratic over a `Vec` avoids introducing a set
        // whose iteration order could later be mistaken for a source of the frozen
        // neighbour order this type feeds.
        for (index, alternative) in alternatives.iter().enumerate() {
            if alternatives[..index].contains(alternative) {
                return Err(EvalError::config(format!(
                    "path step alternative {index} repeats {:?} in the {:?} direction; a \
                     duplicated alternative doubles every walk that traverses it, with no \
                     observable difference at the call site",
                    alternative.0, alternative.1
                )));
            }
        }
        Ok(Self { alternatives })
    }

    /// The step's alternation, in the order it was supplied.
    #[must_use]
    pub fn alternatives(&self) -> &[(TermValue, PathDirection)] {
        &self.alternatives
    }
}

// ---------------------------------------------------------------------------
// The traversal envelope
// ---------------------------------------------------------------------------

/// The hard ceiling on [`PathLimits::max_hops`].
///
/// A traversal envelope with no depth bound is not merely slow, it is *unsound as a
/// containment story*. Every other host-code hazard at this seam — a panicking relation,
/// a relation that emits a malformed row — is contained by
/// [`open_contained`](crate::property_fn::open_contained) /
/// [`next_contained`](crate::property_fn::next_contained), which catch unwinding panics.
/// A stack overflow is NOT an unwinding panic: it is an abort, and it takes the whole
/// process with it, escaping panic containment entirely. The traversals in this module
/// are written with explicit heap stacks for exactly that reason, and this cap is the
/// second half of the same defence — it bounds the per-walk state (the digest stack, the
/// hop vector) that a depth-parameterised traversal accumulates, so no configuration
/// value can turn a deep graph into an unrecoverable failure. 4096 is far above any
/// depth at which the walk count itself remains tractable, so the cap is a safety rail,
/// not a functional limit.
pub const MAX_HOPS_CAP: u32 = 4096;

/// The traversal envelope one relation runs inside: the accepted walk lengths, and the
/// two resource guards.
///
/// # There is deliberately no `Default`
///
/// Every field here is a statement about how much work the host is willing to buy, and
/// there is no value of "how many walks may one seed enumerate" that is right for an
/// unknown graph. A default would be a number this crate invented and the caller never
/// read — precisely the fabricated configuration the project forbids. The caller states
/// the envelope, explicitly, every time.
///
/// # The guards are resource guards, not semantics
///
/// [`max_paths_per_seed`](Self::max_paths_per_seed) and
/// [`max_expansions_per_invocation`](Self::max_expansions_per_invocation) bound work
/// **actually performed**, so whether either one fires depends on how much work the
/// engine asked for: a query under `LIMIT 1` may stop before the guard, while the same
/// query without `LIMIT` trips it. They are never silent truncation — a breach is an
/// [`Err`], never a short [`Ok`], because a short row stream offered as complete is
/// exactly the wrong answer the crate's hard-fail doctrine exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathLimits {
    min_hops: u32,
    max_hops: u32,
    max_paths_per_seed: u64,
    max_expansions_per_invocation: u64,
}

impl PathLimits {
    /// Build a traversal envelope.
    ///
    /// # Errors
    ///
    /// [`EvalError::Config`] when:
    ///
    /// * `min_hops == 0`. A zero-hop path has no witness. It is the identity relation:
    ///   its `?len` is `0`, so it emits no rows at all under this module's one-row-per-hop
    ///   shape, and its `?edge`, `?step` and `?node` are simply undefined. A caller who
    ///   wants `?start = ?end` does not need a relation for it — that is a `FILTER` or a
    ///   repeated variable, and asking a traversal to express it would produce a row
    ///   shape whose columns cannot be filled.
    /// * `min_hops > max_hops`. The accepted length interval would be empty, which is a
    ///   caller who has not finished deciding rather than a relation that legitimately
    ///   matches nothing.
    /// * `max_hops > MAX_HOPS_CAP`. See [`MAX_HOPS_CAP`].
    /// * `max_paths_per_seed == 0` or `max_expansions_per_invocation == 0`. A guard set
    ///   to zero fails on the first unit of work, so it can only ever produce an error;
    ///   a caller who wants no rows should not register the relation.
    pub fn new(
        min_hops: u32,
        max_hops: u32,
        max_paths_per_seed: u64,
        max_expansions_per_invocation: u64,
    ) -> Result<Self, EvalError> {
        if min_hops == 0 {
            return Err(EvalError::config(
                "min_hops must be at least 1; a zero-hop path is the identity and has no \
                 witness — no statement was traversed, so there is no hop to bind",
            ));
        }
        if min_hops > max_hops {
            return Err(EvalError::config(format!(
                "min_hops ({min_hops}) exceeds max_hops ({max_hops}); the accepted walk-length \
                 interval would be empty"
            )));
        }
        if max_hops > MAX_HOPS_CAP {
            return Err(EvalError::config(format!(
                "max_hops ({max_hops}) exceeds the hard cap {MAX_HOPS_CAP}; an unbounded \
                 traversal depth is a stack-overflow abort, which escapes the property-function \
                 seam's panic containment entirely"
            )));
        }
        if max_paths_per_seed == 0 {
            return Err(EvalError::config(
                "max_paths_per_seed must be at least 1; a guard of zero fails on the first \
                 candidate walk and so can only ever produce an error",
            ));
        }
        if max_expansions_per_invocation == 0 {
            return Err(EvalError::config(
                "max_expansions_per_invocation must be at least 1; a guard of zero fails on the \
                 first traversed edge and so can only ever produce an error",
            ));
        }
        Ok(Self {
            min_hops,
            max_hops,
            max_paths_per_seed,
            max_expansions_per_invocation,
        })
    }

    /// The shortest walk length this envelope accepts (never zero).
    #[must_use]
    pub const fn min_hops(&self) -> u32 {
        self.min_hops
    }

    /// The longest walk length this envelope accepts.
    #[must_use]
    pub const fn max_hops(&self) -> u32 {
        self.max_hops
    }

    /// The most candidate walks one seed node may enumerate before the traversal fails.
    #[must_use]
    pub const fn max_paths_per_seed(&self) -> u64 {
        self.max_paths_per_seed
    }

    /// The most edges one invocation may traverse before the traversal fails.
    #[must_use]
    pub const fn max_expansions_per_invocation(&self) -> u64 {
        self.max_expansions_per_invocation
    }
}

// ---------------------------------------------------------------------------
// The frozen snapshot
// ---------------------------------------------------------------------------

/// What a [`PathGraph`] was built from, as a comparable summary.
///
/// A snapshot is frozen at construction, which is what makes both relations
/// [`Volatility::Stable`] and so eligible for fork-join evaluation. That guarantee is
/// only as good as the caller's discipline about rebuilding it, so the snapshot carries
/// enough of its provenance to be checked: a host that caches a `PathGraph` across
/// dataset revisions can compare this value and rebuild when it moves.
///
/// It is a *discriminator*, not a content digest — [`DatasetView::stats_fingerprint`] is
/// itself documented as a cache discriminator — so equality is evidence of sameness, not
/// proof of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PathSnapshotFingerprint {
    /// The source view's [`DatasetView::stats_fingerprint`] at snapshot time.
    pub stats_fingerprint: u64,
    /// The source view's [`DatasetView::term_count`] at snapshot time.
    pub term_count: usize,
    /// The number of distinct nodes participating in at least one edge.
    pub node_count: usize,
    /// The number of edges recorded across every adjacency list.
    pub edge_count: usize,
}

/// One directed edge of the snapshot multigraph.
///
/// `fold_bytes` is the per-edge contribution to a walk's identifier —
/// `canonical_bytes(statement) || canonical_bytes(target node)` — precomputed here so
/// that traversal, which touches an edge once per prefix it extends, never re-encodes a
/// term. It lives in the edge rather than in a parallel vector because two structures
/// indexed in lockstep are two structures that can fall out of lockstep.
#[derive(Debug, Clone)]
struct Edge {
    /// The dense index of the node this hop arrives at.
    to: u32,
    /// The dense index of the statement this hop traverses.
    statement: u32,
    /// `canonical_bytes(statements[statement]) || canonical_bytes(nodes[to])`.
    fold_bytes: Box<[u8]>,
}

/// One hop of a walk, named by where it started and which of that node's frozen
/// neighbour entries it took.
///
/// Both traversals speak this rather than carrying resolved terms, so a walk in flight
/// is two `u32`s per hop and no allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Hop {
    /// The dense index of the node this hop left.
    from: u32,
    /// The position of the taken edge within `adjacency[from]`.
    adj: usize,
}

/// A frozen, dense-`u32` multigraph snapshot of one [`PathStep`] over one dataset view.
///
/// # Why a snapshot, and why dense
///
/// A traversal touches the same neighbour list once per prefix that reaches it, which on
/// a branching graph is many times. Re-asking [`DatasetView::quads_for_pattern`] each
/// time would repay the backend's index lookup on every visit, and would leave the
/// traversal's cost at the mercy of a `Vec<TermValue>` comparison per neighbour. So the
/// step's edges are read ONCE, resolved to dataset-independent [`TermValue`]s (this
/// crate's C0.8 boundary: a relation never holds a dataset-local id), and re-indexed into
/// a dense `u32` space in which the whole traversal is integer work.
///
/// # It is a MULTIGRAPH
///
/// Neighbour lists are sorted by `(to, statement)` and are NOT collapsed to distinct
/// targets. Two alternatives that reach one node do so via two different statements, and
/// those are two derivations, not one — erasing the second is exactly the failure the
/// module docs open with. The only entries that are merged are entries identical in BOTH
/// `to` and `statement`, which is one statement traversed to one node recorded twice.
/// That can arise only from a self-loop (`s == o`) under a step declaring the same
/// predicate in both directions, and the two entries are then the same hop, with the same
/// bytes and the same identifier; keeping both would make the walk identifier
/// non-injective over walks, which
/// [`PropertyFunction::rows_per_invocation`]'s bound depends on.
///
/// # Why the emission order is a pure function of the data
///
/// Node indices are assigned in [`TermValue`] `Ord` order, so sorting a neighbour list by
/// `to` IS sorting it by the target's value; statement indices likewise. Nothing about
/// the order in which quads arrived from the view, nor any hash seed, survives into the
/// frozen structure. Two processes that snapshot the same data get byte-identical
/// adjacency, hence byte-identical rows.
#[derive(Debug)]
pub struct PathGraph {
    /// Distinct participating nodes, sorted ascending by [`TermValue`] `Ord`.
    nodes: Vec<TermValue>,
    /// Distinct traversed statements, sorted ascending by [`TermValue`] `Ord`.
    statements: Vec<TermValue>,
    /// `adjacency[n]` is node `n`'s out-edges, frozen in `(to, statement)` order.
    adjacency: Vec<Vec<Edge>>,
    /// The reverse node adjacency, built on first use.
    ///
    /// It is needed by exactly one pushdown — the reverse breadth-first distance table a
    /// bound `?end` uses to prune — and by no other caller, so building it eagerly would
    /// charge every snapshot for a structure most invocations never read. It is cached in
    /// the SNAPSHOT rather than rebuilt per invocation because it is a pure function of
    /// the frozen adjacency: a relation is driven once per row of the bag feeding it, and
    /// an invocation-local rebuild would recompute an O(V + E) structure on every one of
    /// those rows.
    reverse: OnceLock<Vec<Vec<u32>>>,
    fingerprint: PathSnapshotFingerprint,
}

impl PathGraph {
    /// Snapshot `step`'s edges over `dataset`, scoped to `graph`.
    ///
    /// Each alternative is driven as one `quads_for_pattern(None, Some(predicate), None,
    /// graph)` scan. A `Forward` alternative records the edge `subject → object`; an
    /// `Inverse` one records `object → subject`. In BOTH cases the statement recorded on
    /// the edge is the **asserted** triple `(subject, predicate, object)`, so a consumer
    /// that wants the traversal direction back reads it off the statement by comparing
    /// its subject to the node the hop left.
    ///
    /// # Errors
    ///
    /// [`EvalError::Data`] if a declared alternative's predicate IRI is not interned in
    /// `dataset` at all. This follows the precedent
    /// [`MemoryRelation::from_graph`](crate::property_fn::MemoryRelation::from_graph)
    /// sets for an absent list head: a name that the dataset has never seen is a
    /// configuration pointing at nothing, and reporting it as an empty adjacency would
    /// convert a host's typo into a silently empty answer that no query text can
    /// distinguish from an honest one. A predicate that IS interned but participates in no
    /// quad within `graph` is a different thing entirely — that is a real, observable
    /// emptiness, and it is accepted.
    ///
    /// [`EvalError::Data`] if the step's edges span more than [`u32::MAX`] distinct nodes
    /// or statements, which the dense index space cannot address.
    pub fn from_dataset<D: DatasetView>(
        dataset: &D,
        step: &PathStep,
        graph: GraphMatch<D::Id>,
    ) -> Result<Self, EvalError> {
        // (from, to, statement) in discovery order; re-indexed below, so this order never
        // reaches an emitted row.
        let mut raw: Vec<(TermValue, TermValue, TermValue)> = Vec::new();
        for (predicate, direction) in &step.alternatives {
            let Some(predicate_id) = dataset.term_id_by_value(predicate) else {
                return Err(EvalError::data(format!(
                    "path step predicate {predicate:?} is not present in the dataset; a \
                     predicate naming nothing is a configuration pointing at nothing, not an \
                     empty adjacency"
                )));
            };
            for quad in dataset.quads_for_pattern(None, Some(predicate_id), None, graph) {
                let subject = crate::scratch::term_id_to_value(dataset, quad.s);
                let object = crate::scratch::term_id_to_value(dataset, quad.o);
                let statement = TermValue::Triple {
                    s: Box::new(subject.clone()),
                    p: Box::new(predicate.clone()),
                    o: Box::new(object.clone()),
                };
                let (from, to) = match direction {
                    PathDirection::Forward => (subject, object),
                    PathDirection::Inverse => (object, subject),
                };
                raw.push((from, to, statement));
            }
        }

        let mut nodes: Vec<TermValue> = Vec::with_capacity(raw.len() * 2);
        let mut statements: Vec<TermValue> = Vec::with_capacity(raw.len());
        for (from, to, statement) in &raw {
            nodes.push(from.clone());
            nodes.push(to.clone());
            statements.push(statement.clone());
        }
        nodes.sort_unstable();
        nodes.dedup();
        statements.sort_unstable();
        statements.dedup();
        if u32::try_from(nodes.len()).is_err() {
            return Err(EvalError::data(format!(
                "path step spans {} distinct nodes, which exceeds the dense u32 index space",
                nodes.len()
            )));
        }
        if u32::try_from(statements.len()).is_err() {
            return Err(EvalError::data(format!(
                "path step spans {} distinct statements, which exceeds the dense u32 index space",
                statements.len()
            )));
        }

        let mut adjacency: Vec<Vec<Edge>> = (0..nodes.len()).map(|_| Vec::new()).collect();
        for (from, to, statement) in raw {
            let from_index = dense_index(&nodes, &from);
            let to_index = dense_index(&nodes, &to);
            let statement_index = dense_index(&statements, &statement);
            let mut fold_bytes = Vec::new();
            statements[statement_index as usize].canonical_bytes(&mut fold_bytes);
            nodes[to_index as usize].canonical_bytes(&mut fold_bytes);
            adjacency[from_index as usize].push(Edge {
                to: to_index,
                statement: statement_index,
                fold_bytes: fold_bytes.into_boxed_slice(),
            });
        }
        let mut edge_count = 0usize;
        for list in &mut adjacency {
            list.sort_unstable_by_key(|edge| (edge.to, edge.statement));
            // Merges only entries identical in BOTH fields — see the type docs.
            list.dedup_by_key(|edge| (edge.to, edge.statement));
            edge_count += list.len();
        }

        Ok(Self {
            fingerprint: PathSnapshotFingerprint {
                stats_fingerprint: dataset.stats_fingerprint(),
                term_count: dataset.term_count(),
                node_count: nodes.len(),
                edge_count,
            },
            nodes,
            statements,
            adjacency,
            reverse: OnceLock::new(),
        })
    }

    /// The number of distinct nodes participating in at least one edge.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// The number of edges across every adjacency list.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.fingerprint.edge_count
    }

    /// What this snapshot was built from. See [`PathSnapshotFingerprint`].
    #[must_use]
    pub fn snapshot_fingerprint(&self) -> PathSnapshotFingerprint {
        self.fingerprint
    }

    /// The dense index of `value`, or `None` when it participates in no edge.
    ///
    /// A binary search rather than a map lookup: `nodes` is already sorted by
    /// [`TermValue`] `Ord`, and a map would add a second membership structure whose
    /// iteration order somebody could later mistake for a source of emission order.
    fn node_index(&self, value: &TermValue) -> Option<u32> {
        self.nodes
            .binary_search(value)
            .ok()
            .and_then(|index| u32::try_from(index).ok())
    }

    /// The node a hop arrives at.
    fn hop_target(&self, hop: Hop) -> u32 {
        self.adjacency[hop.from as usize][hop.adj].to
    }

    /// The statement a hop traverses.
    fn hop_statement(&self, hop: Hop) -> u32 {
        self.adjacency[hop.from as usize][hop.adj].statement
    }

    /// The reverse node adjacency, built on first use and cached. Deduplicated: it is
    /// only ever used for a breadth-first distance, which cares about reachability and
    /// not about how many statements witness a hop.
    fn reverse_adjacency(&self) -> &[Vec<u32>] {
        self.reverse.get_or_init(|| {
            let mut reverse: Vec<Vec<u32>> = (0..self.nodes.len()).map(|_| Vec::new()).collect();
            for (from, list) in self.adjacency.iter().enumerate() {
                let from = u32::try_from(from).expect("node count fits u32 by construction");
                for edge in list {
                    reverse[edge.to as usize].push(from);
                }
            }
            for list in &mut reverse {
                list.sort_unstable();
                list.dedup();
            }
            reverse
        })
    }

    /// Minimum hop counts from every node to `end`, by breadth-first search over the
    /// REVERSE adjacency, capped at `cap` hops. [`u32::MAX`] means "not within `cap`".
    ///
    /// This is a LOWER bound on the hops a prefix ending at `u` still needs, because it
    /// ignores the simple-prefix rule: a walk constrained not to revisit its own nodes can
    /// only ever need MORE hops than the unconstrained shortest distance, never fewer. So
    /// pruning a prefix at depth `d` when `d + distance[u] > max_depth` removes only
    /// prefixes that provably cannot complete — the prune is exact in the sense that
    /// matters, discarding no answer, rather than a heuristic that trades recall for
    /// speed.
    fn min_distance_to(&self, end: u32, cap: u32) -> Vec<u32> {
        let reverse = self.reverse_adjacency();
        let mut distance = vec![u32::MAX; self.nodes.len()];
        distance[end as usize] = 0;
        let mut queue: VecDeque<u32> = VecDeque::new();
        queue.push_back(end);
        while let Some(node) = queue.pop_front() {
            let depth = distance[node as usize];
            if depth >= cap {
                continue;
            }
            for &previous in &reverse[node as usize] {
                if distance[previous as usize] == u32::MAX {
                    distance[previous as usize] = depth + 1;
                    queue.push_back(previous);
                }
            }
        }
        distance
    }
}

/// The dense index of `value` within an already-sorted, deduplicated table it is known to
/// belong to.
fn dense_index(table: &[TermValue], value: &TermValue) -> u32 {
    let index = table
        .binary_search(value)
        .expect("the table was built from exactly these values");
    u32::try_from(index).expect("table length was checked against u32::MAX at construction")
}

// ---------------------------------------------------------------------------
// The walk identifier
// ---------------------------------------------------------------------------

/// The domain-separation prefix every walk identifier is digested under.
///
/// A bare digest of a term encoding is a digest of *some* structure; prefixing it with a
/// name for the question being asked means an identifier minted here can never be equal
/// to one minted by a different scheme over the same bytes. The `-v1` suffix is what
/// makes the encoding revisable: a future layout change takes a new domain, so old and
/// new identifiers are unequal by construction rather than silently interchangeable.
const PATH_ID_DOMAIN_V1: &[u8] = b"path-witness-identifier-v1";

/// The digest state of a zero-hop prefix rooted at `node`.
fn seed_digest(graph: &PathGraph, node: u32) -> Sha256 {
    let mut state = Sha256::new();
    state.update(PATH_ID_DOMAIN_V1);
    let mut bytes = Vec::new();
    graph.nodes[node as usize].canonical_bytes(&mut bytes);
    state.update(&bytes);
    state
}

/// Close a walk's digest and render it as the identifier term's lexical form.
///
/// The hop count is absorbed LAST because it is not known during descent — a prefix is
/// extended before anyone knows how long the accepted walk will be — and it is absorbed
/// at fixed width so the preimage stays uniquely decodable: without it, the encoding of
/// `n0 (e n)*` would be a prefix of the encoding of any longer walk sharing that prefix,
/// and a length suffix that varied in width would reintroduce exactly the framing
/// ambiguity [`TermValue::canonical_bytes`] exists to remove.
///
/// The digest is **not truncated**. The identifier is a grouping key in query answers —
/// `GROUP BY ?pathId` is how a caller reassembles a walk from its hop rows — so a
/// collision is not a hash-table slowdown, it is two different walks fused into one
/// answer. All 32 bytes are rendered, as 64 lowercase hex characters.
fn finish_digest(mut state: Sha256, hop_count: u64) -> String {
    state.update(hop_count.to_le_bytes());
    let digest = state.finalize();
    let mut rendered = String::with_capacity(64);
    for byte in digest {
        write!(rendered, "{byte:02x}").expect("formatting into a String cannot fail");
    }
    rendered
}

// ---------------------------------------------------------------------------
// Row shape
// ---------------------------------------------------------------------------

/// Flattened position of `?start`, the walk's first node.
const POS_START: usize = 0;
/// Flattened position of `?end`, the walk's last node.
const POS_END: usize = 1;
/// Flattened position of `?pathId`, the walk's content-derived identifier.
const POS_PATH_ID: usize = 2;
/// Flattened position of `?len`, the walk's hop count.
const POS_LEN: usize = 3;
/// Flattened position of `?step`, this row's 1-based hop ordinal.
const POS_STEP: usize = 4;
/// Flattened position of `?node`, the node this row's hop arrives at.
const POS_NODE: usize = 5;
/// Flattened position of `?edge`, the statement this row's hop traverses.
const POS_EDGE: usize = 6;
/// The number of flattened positions a row fills.
const ROW_WIDTH: usize = 7;

/// The declared arity of both relations: `?start <iri> ( ?end ?pathId ?len ?step ?node
/// ?edge )`.
fn path_arity() -> PfArity {
    PfArity::new(1, ROW_WIDTH - 1)
}

/// `n` as an `xsd:integer`-typed literal.
///
/// Typed, never simple: `?step` and `?len` exist to be compared and ordered
/// numerically, and `ORDER BY` over simple literals is codepoint order, which puts
/// `"10"` before `"2"` and scrambles every reconstruction of a walk longer than nine
/// hops.
fn integer_literal(n: u64) -> TermValue {
    TermValue::typed_literal(n.to_string(), purrdf_xsd::datatype::XSD_INTEGER)
}

/// Read a bound argument as a hop count for pushdown purposes.
///
/// Returns `None` when the term is not a literal, or its lexical form does not parse as a
/// `u32`. `None` means "no emitted term can ever equal this", because every `?len` and
/// `?step` this module emits is an `xsd:integer` literal whose lexical form is a plain
/// decimal — so the caller turns `None` into an empty cursor rather than into "no
/// pushdown". The parse is only ever a BOUND, never the decision: the per-position
/// equality filter still runs, so a lexical form like `"03"` that parses to 3 but is not
/// the term this relation emits is correctly excluded by the filter, and the derived
/// depth restriction remains sound because it can only ever be narrower than needed.
fn bound_hop_count(term: &TermValue) -> Option<u32> {
    match term {
        TermValue::Literal { lexical_form, .. } => lexical_form.parse::<u32>().ok(),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The shared per-invocation plan
// ---------------------------------------------------------------------------

/// Everything one invocation derived from its bound arguments, shared by both cursors.
///
/// This is where the pushdowns live (which seeds, which depths, which prefixes are
/// prunable) together with the per-position equality filter and the ceiling accounting.
/// Both traversals differ only in HOW they enumerate walks; what a walk must agree with
/// before it becomes rows, and what a row costs, is one implementation.
#[derive(Debug)]
struct Prepared {
    /// Names the relation in guard messages ("path witness" / "shortest path witness").
    kind: &'static str,
    limits: PathLimits,
    /// The bound value at each flattened position, `None` where the position is free.
    bound: [Option<TermValue>; ROW_WIDTH],
    /// The seed node indices to explore, ascending (hence in `TermValue` order).
    seeds: Vec<u32>,
    /// The shortest walk length this invocation can emit, after `?len`/`?step` pushdown.
    min_depth: u32,
    /// The longest walk length this invocation can emit, after `?len` pushdown.
    max_depth: u32,
    /// The dense index of a bound `?end`, when one is bound.
    end_index: Option<u32>,
    /// Lower bounds on the hops from each node to a bound `?end`. See
    /// [`PathGraph::min_distance_to`].
    min_distance_to_end: Option<Vec<u32>>,
    /// What is left of the engine's row licence, or `None` when none was offered.
    remaining: Option<u64>,
    /// Edges traversed so far, against [`PathLimits::max_expansions_per_invocation`].
    expansions: u64,
    /// Candidate walks enumerated for the seed currently being explored, against
    /// [`PathLimits::max_paths_per_seed`].
    candidates: u64,
    /// Rows materialized from accepted walks and not yet drained.
    pending: VecDeque<PfRow>,
}

impl Prepared {
    /// Derive one invocation's plan, or `None` when the bound arguments make the answer
    /// provably empty.
    ///
    /// Returning `None` rather than an empty `seeds` list keeps the "this invocation
    /// cannot match" decision in ONE place, so a cursor never has to re-derive it.
    fn new(
        graph: &PathGraph,
        kind: &'static str,
        limits: PathLimits,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Option<Self> {
        let bound: [Option<TermValue>; ROW_WIDTH] =
            core::array::from_fn(|pos| args.get(pos).cloned());

        let mut min_depth = limits.min_hops();
        let mut max_depth = limits.max_hops();

        // A bound `?len` pins the depth exactly; an unparseable one matches no emitted
        // term at all.
        if let Some(term) = bound[POS_LEN].as_ref() {
            let length = bound_hop_count(term)?;
            if length < min_depth || length > max_depth {
                return None;
            }
            min_depth = length;
            max_depth = length;
        }
        // A bound `?step` raises the depth FLOOR: a walk shorter than the requested hop
        // ordinal has no row at that ordinal.
        if let Some(term) = bound[POS_STEP].as_ref() {
            let step = bound_hop_count(term)?;
            if step == 0 || step > max_depth {
                return None;
            }
            min_depth = min_depth.max(step);
        }
        if min_depth > max_depth {
            return None;
        }

        // A bound `?start` is a single seed; an unknown one participates in no edge.
        let seeds: Vec<u32> = match bound[POS_START].as_ref() {
            Some(value) => vec![graph.node_index(value)?],
            None => (0..u32::try_from(graph.node_count()).ok()?).collect(),
        };

        // A bound `?end` is an exact prune, not merely a filter.
        let (end_index, min_distance_to_end) = match bound[POS_END].as_ref() {
            Some(value) => {
                let end = graph.node_index(value)?;
                (Some(end), Some(graph.min_distance_to(end, max_depth)))
            }
            None => (None, None),
        };

        Some(Self {
            kind,
            limits,
            bound,
            seeds,
            min_depth,
            max_depth,
            end_index,
            min_distance_to_end,
            remaining: ceiling,
            expansions: 0,
            candidates: 0,
            pending: VecDeque::new(),
        })
    }

    /// Whether the value at flattened position `pos` agrees with what was bound there.
    ///
    /// A free position agrees with everything; a bound one agrees only with itself.
    fn agrees(&self, pos: usize, value: &TermValue) -> bool {
        self.bound[pos].as_ref().is_none_or(|bound| bound == value)
    }

    /// Whether a walk of `depth` hops is a candidate for this invocation.
    const fn is_candidate_depth(&self, depth: u32) -> bool {
        depth >= self.min_depth && depth <= self.max_depth
    }

    /// Whether a prefix of `depth` hops ending at `node` can be discarded outright.
    ///
    /// Only ever true under a bound `?end`, and then only when the node's LOWER bound on
    /// the remaining hops already overshoots the depth envelope. See
    /// [`PathGraph::min_distance_to`] for why that discards no answer.
    fn prune(&self, node: u32, depth: u32) -> bool {
        self.min_distance_to_end
            .as_ref()
            .is_some_and(|distance| depth.saturating_add(distance[node as usize]) > self.max_depth)
    }

    /// Begin a seed: reset the per-seed candidate counter.
    const fn begin_seed(&mut self) {
        self.candidates = 0;
    }

    /// Charge one traversed edge against
    /// [`PathLimits::max_expansions_per_invocation`].
    ///
    /// # Errors
    ///
    /// [`EvalError::Function`] on a breach. Charged BEFORE the prune, because an edge
    /// that is examined and discarded is still an edge the traversal read.
    fn charge_expansion(&mut self, graph: &PathGraph, seed: u32) -> Result<(), EvalError> {
        self.expansions += 1;
        if self.expansions > self.limits.max_expansions_per_invocation() {
            return Err(EvalError::function(format!(
                "{} traversal exceeded max_expansions_per_invocation ({}) while exploring from \
                 seed {:?}; this is a resource guard over edges actually traversed, so whether \
                 it fires depends on the row ceiling the engine granted — the same query under \
                 a LIMIT may stop before it",
                self.kind,
                self.limits.max_expansions_per_invocation(),
                graph.nodes[seed as usize]
            )));
        }
        Ok(())
    }

    /// Charge one enumerated candidate walk against [`PathLimits::max_paths_per_seed`].
    ///
    /// # Errors
    ///
    /// [`EvalError::Function`] on a breach.
    fn charge_candidate(&mut self, graph: &PathGraph, seed: u32) -> Result<(), EvalError> {
        self.candidates += 1;
        if self.candidates > self.limits.max_paths_per_seed() {
            return Err(EvalError::function(format!(
                "{} traversal exceeded max_paths_per_seed ({}) at seed {:?}; this is a resource \
                 guard over candidate walks actually enumerated, so whether it fires depends on \
                 the row ceiling the engine granted — the same query under a LIMIT may stop \
                 before it",
                self.kind,
                self.limits.max_paths_per_seed(),
                graph.nodes[seed as usize]
            )));
        }
        Ok(())
    }

    /// Turn one accepted walk into pending rows, applying EVERY bound position's
    /// equality filter first.
    ///
    /// # Why the cursor filters rather than leaving it to the engine
    ///
    /// The engine does apply its own equality filter on bound positions, so a relation
    /// that skipped this would still be *correct*. It would not be correct under a
    /// CEILING. The ceiling counts "rows it emits that agree with the bound positions it
    /// was handed"; a relation that spends its licence on rows it can itself see the
    /// engine will drop hands back fewer usable rows than the engine asked for, and the
    /// engine reads a short bag as an exhausted one. Under `LIMIT 1` with a bound `?end`
    /// that is precisely a short answer labelled complete. So the filter runs here,
    /// walk-level positions first (`?start`, `?end`, `?len`, `?pathId` are constant
    /// across a walk, so a mismatch discards the whole walk without materializing a
    /// single row) and row-level positions (`?step`, `?node`, `?edge`) per hop.
    ///
    /// The identifier is computed only after the three cheap walk-level checks pass, so a
    /// mismatching walk never pays for a digest finalization.
    fn emit_walk(&mut self, graph: &PathGraph, start: u32, hops: &[Hop], state: &Sha256) {
        let Some(&last) = hops.last() else {
            return;
        };
        let start_term = &graph.nodes[start as usize];
        if !self.agrees(POS_START, start_term) {
            return;
        }
        let end_term = &graph.nodes[graph.hop_target(last) as usize];
        if !self.agrees(POS_END, end_term) {
            return;
        }
        let length = hops.len() as u64;
        let len_term = integer_literal(length);
        if !self.agrees(POS_LEN, &len_term) {
            return;
        }
        let path_id = TermValue::simple_literal(finish_digest(state.clone(), length));
        if !self.agrees(POS_PATH_ID, &path_id) {
            return;
        }

        for (index, &hop) in hops.iter().enumerate() {
            let step_term = integer_literal(index as u64 + 1);
            if !self.agrees(POS_STEP, &step_term) {
                continue;
            }
            let node_term = &graph.nodes[graph.hop_target(hop) as usize];
            if !self.agrees(POS_NODE, node_term) {
                continue;
            }
            let edge_term = &graph.statements[graph.hop_statement(hop) as usize];
            if !self.agrees(POS_EDGE, edge_term) {
                continue;
            }
            self.pending.push_back(vec![
                start_term.clone(),
                end_term.clone(),
                path_id.clone(),
                len_term.clone(),
                step_term,
                node_term.clone(),
                edge_term.clone(),
            ]);
        }
    }

    /// Hand back the next already-materialized row, spending one unit of the licence.
    fn take_pending(&mut self) -> Option<PfRow> {
        let row = self.pending.pop_front()?;
        if let Some(remaining) = self.remaining.as_mut() {
            *remaining = remaining.saturating_sub(1);
        }
        Some(row)
    }

    /// Whether the licence is spent, so no further row can reach the query's answer.
    fn licence_spent(&self) -> bool {
        self.remaining == Some(0)
    }
}

/// The row bound shared by both relations, given how many walks one seed can contribute.
///
/// Every product is `saturating`, because a bound that WRAPPED would be reported as a
/// small number — and [`PropertyFunction::rows_per_invocation`] is held to the same
/// honesty contract as a cardinality estimate: an under-statement turns an admission
/// decision into a wrong one, where an over-statement only costs a worse plan.
fn row_bound(mode: BindingPattern, walks_per_seed: u64, node_count: u64, max_hops: u64) -> u64 {
    let seeds = if mode.is_bound(POS_START) {
        1
    } else {
        node_count
    };
    // A walk of `k <= max_hops` hops emits at most `k` rows; a bound `?step` reduces
    // every walk to at most the single row at that ordinal.
    let per_walk_rows = if mode.is_bound(POS_STEP) { 1 } else { max_hops };
    let base = seeds
        .saturating_mul(walks_per_seed)
        .saturating_mul(per_walk_rows);
    if mode.is_bound(POS_PATH_ID) {
        // The identifier is injective over walks — it digests the walk's whole node and
        // statement sequence, and the snapshot records no two distinct walks with the
        // same sequence (see [`PathGraph`]'s multigraph note) — so at most ONE walk can
        // survive the equality filter at that position, contributing at most one walk's
        // worth of rows.
        return base.min(per_walk_rows);
    }
    base
}

// ---------------------------------------------------------------------------
// Every simple-prefix walk
// ---------------------------------------------------------------------------

/// Binds EVERY simple-prefix walk of a [`PathStep`], one row per hop.
///
/// This is the complete answer to the derivation question the module docs pose: for each
/// seed, every walk whose proper prefix is simple and whose length lies in the envelope
/// is enumerated, and each is reported hop by hop with a content-derived identifier that
/// ties its rows together.
///
/// # Emission order (the relation's contract)
///
/// 1. Seeds in ascending dense node index, which is ascending [`TermValue`] order. A
///    bound `?start` is a single seed.
/// 2. Within a seed, **depth-first preorder over accepted walks**: a walk is emitted at
///    the moment its final hop is taken, BEFORE any of its own extensions. So
///    `a → b` precedes `a → b → c`, which precedes `a → b → d`.
/// 3. Neighbours are explored in the snapshot's frozen `(to, statement)` order.
/// 4. Within a walk, rows ascend by `?step`, from 1 to `?len`.
///
/// Every one of those four is a pure function of the snapshot's contents and the bound
/// arguments — no map iteration, no wall clock, no thread scheduling — as
/// [`PfCursor`]'s emission-order contract requires.
///
/// # Cardinality
///
/// Exponential in the worst case, because the number of simple paths in a dense graph is.
/// That is not a defect of the implementation; it is the size of the answer. A caller who
/// wants a polynomial answer wants [`ShortestPathWitnessRelation`], which is a different
/// question with a different answer, registered under a different IRI.
///
/// # Guards
///
/// [`PathLimits::max_paths_per_seed`] and
/// [`PathLimits::max_expansions_per_invocation`] bound work ACTUALLY PERFORMED, so
/// whether either fires depends on the row ceiling the engine granted: a `LIMIT 1` query
/// may stop before the guard while the same query without `LIMIT` trips it. A breach is
/// always an [`Err`] and never a short [`Ok`].
#[derive(Debug)]
pub struct PathWitnessRelation {
    graph: Arc<PathGraph>,
    limits: PathLimits,
    /// The single declared mode (all-free), materialized once so
    /// [`PropertyFunction::modes`] can hand out a slice.
    modes: [BindingPattern; 1],
}

impl PathWitnessRelation {
    /// Bind `graph`'s walks under `limits`.
    ///
    /// The snapshot is shared by [`Arc`] so one graph can back both this relation and a
    /// [`ShortestPathWitnessRelation`] registered alongside it: the two questions differ,
    /// the edges do not.
    #[must_use]
    pub fn new(graph: Arc<PathGraph>, limits: PathLimits) -> Self {
        Self {
            graph,
            limits,
            modes: [path_arity().all_free_mode()],
        }
    }
}

impl PropertyFunction for PathWitnessRelation {
    fn volatility(&self) -> Volatility {
        // The snapshot is frozen at construction and the traversal reads nothing else, so
        // two workers evaluating the same invocation reach the same rows in the same
        // order. That is exactly what the fork-join gate requires of `Stable`.
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        path_arity()
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        // At most `max_paths_per_seed` candidate walks are ACCEPTED per seed: the guard
        // errors on the count that exceeds it, so the traversal never emits from a
        // later one.
        row_bound(
            mode,
            self.limits.max_paths_per_seed(),
            self.graph.node_count() as u64,
            u64::from(self.limits.max_hops()),
        )
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        // `open_contained` already checked this for every engine-driven call; a direct
        // caller gets the same answer rather than a panic on a short argument vector.
        let supplied = args.arity();
        if supplied != path_arity() {
            return Err(EvalError::function(format!(
                "path witness relation expects {} argument(s), got {supplied}",
                path_arity()
            )));
        }
        let Some(prepared) = Prepared::new(&self.graph, "path witness", self.limits, args, ceiling)
        else {
            return Ok(Box::new(EmptyCursor));
        };
        Ok(Box::new(PathWitnessCursor {
            on_path: vec![false; self.graph.node_count()],
            graph: Arc::clone(&self.graph),
            prepared,
            seed_cursor: 0,
            seed: 0,
            stack: Vec::new(),
            hops: Vec::new(),
            digests: Vec::new(),
        }))
    }
}

/// The cursor of an invocation that can match nothing: a bound `?start` naming a node the
/// snapshot has never seen, a `?len` outside the envelope, an unreachable bound `?end`.
///
/// A distinct type rather than a flag on the real cursor, so "provably empty" costs no
/// per-`next` branch on the real traversal and cannot be reached half-initialised.
#[derive(Debug)]
struct EmptyCursor;

impl PfCursor for EmptyCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(None)
    }
}

/// One frame of the explicit depth-first stack: a node, and how far its frozen neighbour
/// list has been consumed.
#[derive(Debug, Clone, Copy)]
struct DfsFrame {
    node: u32,
    next_edge: usize,
}

/// The depth-first cursor [`PathWitnessRelation::open`] returns.
///
/// # Why the stack is on the heap
///
/// A recursive traversal's depth is the graph's, and a deep graph is then a stack
/// overflow — which is an ABORT, not an unwinding panic, so it escapes
/// [`open_contained`](crate::property_fn::open_contained) /
/// [`next_contained`](crate::property_fn::next_contained) entirely and takes the process
/// with it. Every piece of per-depth state here (`stack`, `hops`, `digests`) is therefore
/// an explicit heap vector, and [`MAX_HOPS_CAP`] bounds how tall they can grow.
#[derive(Debug)]
struct PathWitnessCursor {
    graph: Arc<PathGraph>,
    prepared: Prepared,
    /// The next entry of `prepared.seeds` to start from.
    seed_cursor: usize,
    /// The seed currently being explored (named in guard messages).
    seed: u32,
    /// One frame per node of the current prefix, root first.
    stack: Vec<DfsFrame>,
    /// The current prefix's hops; `hops.len()` is its depth.
    hops: Vec<Hop>,
    /// `digests[d]` is the identifier state of the prefix's first `d` hops, so extending
    /// by one hop is one `Sha256` clone plus one absorbed, already-encoded byte string.
    /// `digests.len() == hops.len() + 1`.
    digests: Vec<Sha256>,
    /// Membership of the current prefix, for the simple-prefix rule.
    on_path: Vec<bool>,
}

impl PathWitnessCursor {
    /// Perform one unit of traversal work. Returns `false` only when every seed is
    /// exhausted.
    ///
    /// One unit is: start a seed, take one edge, or pop an exhausted frame. Splitting the
    /// traversal this finely keeps a single `next` call bounded, which is what keeps the
    /// evaluator's stop-check granularity meaningful (see [`PfCursor`]'s deaf-relation
    /// doctrine).
    fn step(&mut self) -> Result<bool, EvalError> {
        if self.stack.is_empty() {
            let Some(&seed) = self.prepared.seeds.get(self.seed_cursor) else {
                return Ok(false);
            };
            self.seed_cursor += 1;
            self.seed = seed;
            self.prepared.begin_seed();
            // A seed from which a bound `?end` is already out of reach contributes
            // nothing, and exploring it would spend the expansion guard on edges no row
            // could come from.
            if self.prepared.prune(seed, 0) {
                return Ok(true);
            }
            self.on_path[seed as usize] = true;
            self.stack.push(DfsFrame {
                node: seed,
                next_edge: 0,
            });
            self.hops.clear();
            self.digests.clear();
            self.digests.push(seed_digest(&self.graph, seed));
            return Ok(true);
        }

        let depth = self.stack.len() - 1;
        let frame_node = self.stack[depth].node;
        let adjacency = &self.graph.adjacency[frame_node as usize];
        let adj_index = self.stack[depth].next_edge;
        if adj_index >= adjacency.len() {
            self.on_path[frame_node as usize] = false;
            self.stack.pop();
            if depth > 0 {
                self.hops.pop();
                self.digests.pop();
            }
            return Ok(true);
        }
        self.stack[depth].next_edge += 1;
        let edge = &adjacency[adj_index];
        let target = edge.to;

        self.prepared.charge_expansion(&self.graph, self.seed)?;
        let child_depth = u32::try_from(depth).expect("depth is bounded by MAX_HOPS_CAP") + 1;
        if self.prepared.prune(target, child_depth) {
            return Ok(true);
        }

        // The guard is charged BEFORE any traversal state moves, so the `?` below cannot
        // leave `stack`, `hops` and `digests` disagreeing about the current depth.
        let is_candidate = self.prepared.is_candidate_depth(child_depth);
        if is_candidate {
            self.prepared.charge_candidate(&self.graph, self.seed)?;
        }

        // Extend the prefix by this hop: push it and its digest state.
        self.hops.push(Hop {
            from: frame_node,
            adj: adj_index,
        });
        let mut child_state = self.digests[depth].clone();
        child_state.update(&edge.fold_bytes);
        self.digests.push(child_state);

        if is_candidate {
            self.prepared
                .emit_walk(&self.graph, self.seed, &self.hops, &self.digests[depth + 1]);
        }

        // The simple-prefix rule: a final node that repeats an earlier one closes a cycle,
        // and the walk terminates there rather than being extended.
        let closes_cycle = self.on_path[target as usize];
        if !closes_cycle && child_depth < self.prepared.max_depth {
            self.on_path[target as usize] = true;
            self.stack.push(DfsFrame {
                node: target,
                next_edge: 0,
            });
        } else {
            self.hops.pop();
            self.digests.pop();
        }
        Ok(true)
    }
}

impl PfCursor for PathWitnessCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        loop {
            if self.prepared.licence_spent() {
                return Ok(None);
            }
            if let Some(row) = self.prepared.take_pending() {
                return Ok(Some(row));
            }
            if !self.step()? {
                return Ok(None);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// One shortest walk per reachable endpoint
// ---------------------------------------------------------------------------

/// Binds ONE shortest witness per reachable `(seed, end)` pair, one row per hop.
///
/// Where [`PathWitnessRelation`] answers "show me every derivation", this answers "show
/// me *a* derivation, the shortest one" — and that is the question most connectivity
/// queries are actually asking. It is the analogue of Virtuoso's `T_SHORTEST_ONLY`
/// transitivity option, and it is the only form of this relation whose cardinality is
/// polynomial rather than exponential: at most one walk per node pair, so at most
/// `node_count²` walks over the whole snapshot.
///
/// # Which shortest witness, when several tie
///
/// The traversal is a breadth-first search with per-node best-depth pruning: a node is
/// recorded the FIRST time it is discovered, and never again. So the chosen witness is
/// deterministic and stateable: it is the one whose immediate predecessor was dequeued
/// earliest, and, among that predecessor's edges, the one earliest in the snapshot's
/// frozen `(to, statement)` order. Ties are therefore broken by the frozen order, which
/// is itself [`TermValue`] order — never by a hash seed or an arrival order.
///
/// Note that "shortest" is a property of the WALK, not of the derivation: two different
/// statements may join the same two nodes, and this relation reports one of them.
///
/// # Emission order (the relation's contract)
///
/// 1. Seeds in ascending dense node index, which is ascending [`TermValue`] order.
/// 2. Within a seed, witnesses in breadth-first DISCOVERY order — non-decreasing by
///    `?len`, and within one length in the order the search reached them.
/// 3. Within a walk, rows ascend by `?step`.
///
/// # The seed can be its own endpoint
///
/// The seed is deliberately NOT pre-marked as discovered at depth zero, so a seed that
/// lies on a cycle is discovered at its own shortest cycle length and reported like any
/// other endpoint. That is what keeps this relation's endpoint projection agreeing with
/// the core grammar's `p+`, under which a node reaches itself exactly when it lies on a
/// cycle. It is not re-expanded when rediscovered: everything reachable through it was
/// already reached from it at depth zero, at a distance no greater.
///
/// # Guards
///
/// As for [`PathWitnessRelation`]: [`PathLimits::max_paths_per_seed`] and
/// [`PathLimits::max_expansions_per_invocation`] bound work ACTUALLY PERFORMED, so
/// whether either fires depends on the row ceiling the engine granted — a `LIMIT 1` query
/// may stop before the guard while the same query without `LIMIT` trips it. A breach is
/// always an [`Err`], never a short [`Ok`].
#[derive(Debug)]
pub struct ShortestPathWitnessRelation {
    graph: Arc<PathGraph>,
    limits: PathLimits,
    /// The single declared mode (all-free).
    modes: [BindingPattern; 1],
}

impl ShortestPathWitnessRelation {
    /// Bind `graph`'s shortest witnesses under `limits`.
    ///
    /// Takes the same [`Arc<PathGraph>`] a [`PathWitnessRelation`] takes, so a host that
    /// registers both under two IRIs pays for one snapshot.
    #[must_use]
    pub fn new(graph: Arc<PathGraph>, limits: PathLimits) -> Self {
        Self {
            graph,
            limits,
            modes: [path_arity().all_free_mode()],
        }
    }
}

impl PropertyFunction for ShortestPathWitnessRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        path_arity()
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        let node_count = self.graph.node_count() as u64;
        // Two independent bounds on the walks one seed can contribute, whichever is
        // tighter: at most ONE witness per distinct endpoint (breadth-first search
        // records a node once), and at most `max_paths_per_seed` candidates before the
        // guard fails the invocation.
        let mut walks_per_seed = node_count.min(self.limits.max_paths_per_seed());
        if mode.is_bound(POS_END) {
            // With the endpoint pinned, at most one of those witnesses — the one for that
            // very pair — can agree with the bound position, and rows from any other are
            // filtered before they are emitted.
            walks_per_seed = walks_per_seed.min(1);
        }
        row_bound(
            mode,
            walks_per_seed,
            node_count,
            u64::from(self.limits.max_hops()),
        )
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let supplied = args.arity();
        if supplied != path_arity() {
            return Err(EvalError::function(format!(
                "shortest path witness relation expects {} argument(s), got {supplied}",
                path_arity()
            )));
        }
        let Some(prepared) = Prepared::new(
            &self.graph,
            "shortest path witness",
            self.limits,
            args,
            ceiling,
        ) else {
            return Ok(Box::new(EmptyCursor));
        };
        Ok(Box::new(ShortestPathWitnessCursor {
            discovered: vec![false; self.graph.node_count()],
            graph: Arc::clone(&self.graph),
            prepared,
            seed_cursor: 0,
            seed: 0,
            visits: Vec::new(),
            emit_cursor: 0,
        }))
    }
}

/// One breadth-first discovery: which node, at what depth, and through which edge of
/// which earlier discovery.
///
/// The parent is a VISIT index rather than a node index because the seed may itself be
/// rediscovered at a positive depth, so a node no longer determines a unique depth; a
/// visit does, and the reconstruction walks visits.
#[derive(Debug, Clone, Copy)]
struct Visit {
    node: u32,
    depth: u32,
    /// `(parent visit index, edge position within the parent node's adjacency)`, or
    /// `None` for the search root.
    parent: Option<(usize, usize)>,
}

/// The breadth-first cursor [`ShortestPathWitnessRelation::open`] returns.
///
/// Like its depth-first sibling, every piece of traversal state is an explicit heap
/// structure: a breadth-first search has no natural recursion, but the reconstruction of a
/// witness from its parent chain does, and it is written as a loop for the same
/// stack-overflow-is-an-abort reason.
#[derive(Debug)]
struct ShortestPathWitnessCursor {
    graph: Arc<PathGraph>,
    prepared: Prepared,
    /// The next entry of `prepared.seeds` to search from.
    seed_cursor: usize,
    /// The seed currently being drained (named in guard messages).
    seed: u32,
    /// The current seed's discoveries, in discovery order; `visits[0]` is the search root
    /// and is not itself a witness.
    visits: Vec<Visit>,
    /// The next entry of `visits` to turn into rows.
    emit_cursor: usize,
    /// Per-node discovery marks for the current seed, reused across seeds.
    discovered: Vec<bool>,
}

impl ShortestPathWitnessCursor {
    /// Search from `seed`, filling `visits` in discovery order.
    ///
    /// # Errors
    ///
    /// [`EvalError::Function`] when the expansion guard is breached.
    fn search(&mut self, seed: u32) -> Result<(), EvalError> {
        self.visits.clear();
        self.visits.push(Visit {
            node: seed,
            depth: 0,
            parent: None,
        });
        self.emit_cursor = 1;
        self.discovered.fill(false);
        // The seed is deliberately left undiscovered, so a cycle through it is reported.
        let mut queue: Vec<usize> = vec![0];
        let mut head = 0usize;
        while head < queue.len() {
            let visit = queue[head];
            head += 1;
            let node = self.visits[visit].node;
            let depth = self.visits[visit].depth;
            if depth >= self.prepared.max_depth {
                continue;
            }
            for (adj_index, edge) in self.graph.adjacency[node as usize].iter().enumerate() {
                self.prepared.charge_expansion(&self.graph, seed)?;
                if self.discovered[edge.to as usize] {
                    continue;
                }
                self.discovered[edge.to as usize] = true;
                let discovery = self.visits.len();
                self.visits.push(Visit {
                    node: edge.to,
                    depth: depth + 1,
                    parent: Some((visit, adj_index)),
                });
                // A bound `?end` makes every other endpoint's witness unemittable, so the
                // search stops the moment it has the one witness that can produce rows.
                if self.prepared.end_index == Some(edge.to) {
                    return Ok(());
                }
                // Rediscovering the seed records the cycle witness but must not re-expand:
                // everything reachable through it was already reached from it at depth
                // zero, at no greater distance. The prune drops expansions that a bound
                // `?end` puts out of reach; it never drops a DISCOVERY, so recorded depths
                // stay minimal.
                if edge.to != seed && !self.prepared.prune(edge.to, depth + 1) {
                    queue.push(discovery);
                }
            }
        }
        Ok(())
    }

    /// Reconstruct the walk that reached `visit`, oldest hop first.
    fn walk_to(&self, visit: usize) -> Vec<Hop> {
        let mut hops = Vec::new();
        let mut current = visit;
        while let Some((parent, adj_index)) = self.visits[current].parent {
            hops.push(Hop {
                from: self.visits[parent].node,
                adj: adj_index,
            });
            current = parent;
        }
        hops.reverse();
        hops
    }

    /// Perform one unit of work: emit one discovered witness, or search the next seed.
    /// Returns `false` only when every seed is exhausted.
    ///
    /// A whole search runs inside one unit. That is bounded by the snapshot (and by the
    /// expansion guard), and it is the natural granularity: a breadth-first search has no
    /// meaningful partial state to suspend at, and per [`PfCursor`]'s deaf-relation
    /// doctrine a coarse unit can make a stop late, never an answer wrong.
    fn step(&mut self) -> Result<bool, EvalError> {
        if self.emit_cursor < self.visits.len() {
            let visit = self.emit_cursor;
            self.emit_cursor += 1;
            if !self.prepared.is_candidate_depth(self.visits[visit].depth) {
                return Ok(true);
            }
            self.prepared.charge_candidate(&self.graph, self.seed)?;
            let hops = self.walk_to(visit);
            let mut state = seed_digest(&self.graph, self.seed);
            for &hop in &hops {
                state.update(&self.graph.adjacency[hop.from as usize][hop.adj].fold_bytes);
            }
            self.prepared
                .emit_walk(&self.graph, self.seed, &hops, &state);
            return Ok(true);
        }
        let Some(&seed) = self.prepared.seeds.get(self.seed_cursor) else {
            return Ok(false);
        };
        self.seed_cursor += 1;
        self.seed = seed;
        self.prepared.begin_seed();
        if self.prepared.prune(seed, 0) {
            self.visits.clear();
            self.emit_cursor = 0;
            return Ok(true);
        }
        self.search(seed)?;
        Ok(true)
    }
}

impl PfCursor for ShortestPathWitnessCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        loop {
            if self.prepared.licence_spent() {
                return Ok(None);
            }
            if let Some(row) = self.prepared.take_pending() {
                return Ok(Some(row));
            }
            if !self.step()? {
                return Ok(None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use purrdf_core::{RdfDataset, RdfDatasetBuilder};

    use super::*;
    use crate::property_fn::{next_contained, open_contained};

    /// The IRI a call would be written under; only ever read back in a panic message.
    const CALL_IRI: &str = "http://example.org/ns#pathWitness";

    fn iri(local: &str) -> TermValue {
        TermValue::iri(format!("http://example.org/{local}"))
    }

    /// The asserted statement term a hop over `(s, p, o)` records.
    fn stmt(s: &str, p: &str, o: &str) -> TermValue {
        TermValue::Triple {
            s: Box::new(iri(s)),
            p: Box::new(iri(p)),
            o: Box::new(iri(o)),
        }
    }

    fn int(n: u64) -> TermValue {
        integer_literal(n)
    }

    /// An all-free argument vector, to be filled position by position.
    fn free() -> [Option<TermValue>; ROW_WIDTH] {
        core::array::from_fn(|_| None)
    }

    fn dataset(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        for (s, p, o) in triples {
            let s = builder.intern_iri(&format!("http://example.org/{s}"));
            let p = builder.intern_iri(&format!("http://example.org/{p}"));
            let o = builder.intern_iri(&format!("http://example.org/{o}"));
            builder.push_quad(s, p, o, None);
        }
        builder.freeze().expect("freeze")
    }

    fn snapshot(data: &RdfDataset, alternatives: &[(&str, PathDirection)]) -> Arc<PathGraph> {
        let step = PathStep::new(
            alternatives
                .iter()
                .map(|(predicate, direction)| (iri(predicate), *direction))
                .collect(),
        )
        .expect("a well-formed step");
        Arc::new(PathGraph::from_dataset(data, &step, GraphMatch::Default).expect("snapshot"))
    }

    fn limits(min: u32, max: u32) -> PathLimits {
        PathLimits::new(min, max, 4096, 1_000_000).expect("a generous envelope")
    }

    /// Drive `relation` directly through the contained entry points, exactly as the
    /// engine does.
    fn invoke(
        relation: &dyn PropertyFunction,
        bound: &[Option<TermValue>; ROW_WIDTH],
        ceiling: Option<u64>,
    ) -> Result<Vec<PfRow>, EvalError> {
        let refs: Vec<Option<&TermValue>> = bound.iter().map(Option::as_ref).collect();
        let (subject, object) = refs.split_at(1);
        let args = PfArgs::new(subject, object);
        let mut cursor = open_contained(relation, CALL_IRI, &args, ceiling)?;
        let mut rows = Vec::new();
        while let Some(row) = next_contained(&mut *cursor, CALL_IRI)? {
            rows.push(row);
        }
        Ok(rows)
    }

    fn drained(
        relation: &dyn PropertyFunction,
        bound: &[Option<TermValue>; ROW_WIDTH],
    ) -> Vec<PfRow> {
        invoke(relation, bound, None).expect("no guard breach")
    }

    /// The distinct `?pathId` values of `rows`, in first-appearance order, each checked to
    /// be a full untruncated SHA-256 rendered as 64 lowercase hex characters.
    fn path_ids(rows: &[PfRow]) -> Vec<TermValue> {
        let mut ids: Vec<TermValue> = Vec::new();
        for row in rows {
            let id = &row[POS_PATH_ID];
            let TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } = id
            else {
                panic!("?pathId must be a literal, got {id:?}");
            };
            assert_eq!(lexical_form.len(), 64, "the digest is never truncated");
            assert!(
                lexical_form
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "lowercase hex only: {lexical_form}"
            );
            assert_eq!(datatype, "http://www.w3.org/2001/XMLSchema#string");
            assert!(language.is_none() && direction.is_none());
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        ids
    }

    // ---- 1: a three-edge chain, exactly ----------------------------------

    fn chain() -> (Arc<RdfDataset>, Arc<PathGraph>) {
        let data = dataset(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "d")]);
        let graph = snapshot(&data, &[("p", PathDirection::Forward)]);
        (data, graph)
    }

    #[test]
    fn a_chain_yields_every_prefix_walk_hop_by_hop() {
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(graph, limits(1, 3));
        let mut bound = free();
        bound[POS_START] = Some(iri("a"));
        let rows = drained(&relation, &bound);

        let ids = path_ids(&rows);
        assert_eq!(ids.len(), 3, "three walks: a→b, a→b→c, a→b→c→d");
        let (ab, abc, abcd) = (ids[0].clone(), ids[1].clone(), ids[2].clone());

        let e1 = stmt("a", "p", "b");
        let e2 = stmt("b", "p", "c");
        let e3 = stmt("c", "p", "d");
        assert_eq!(
            rows,
            vec![
                // a→b, emitted in DFS preorder BEFORE its own extensions.
                vec![iri("a"), iri("b"), ab, int(1), int(1), iri("b"), e1.clone()],
                // a→b→c
                vec![
                    iri("a"),
                    iri("c"),
                    abc.clone(),
                    int(2),
                    int(1),
                    iri("b"),
                    e1.clone()
                ],
                vec![
                    iri("a"),
                    iri("c"),
                    abc,
                    int(2),
                    int(2),
                    iri("c"),
                    e2.clone()
                ],
                // a→b→c→d
                vec![
                    iri("a"),
                    iri("d"),
                    abcd.clone(),
                    int(3),
                    int(1),
                    iri("b"),
                    e1
                ],
                vec![
                    iri("a"),
                    iri("d"),
                    abcd.clone(),
                    int(3),
                    int(2),
                    iri("c"),
                    e2
                ],
                vec![iri("a"), iri("d"), abcd, int(3), int(3), iri("d"), e3],
            ]
        );
    }

    #[test]
    fn the_last_node_of_a_walk_is_its_end() {
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(graph, limits(1, 3));
        let rows = drained(&relation, &free());
        for row in &rows {
            if row[POS_STEP] == row[POS_LEN] {
                assert_eq!(row[POS_NODE], row[POS_END]);
            }
        }
    }

    // ---- 2: two derivations must not collapse into one -------------------

    #[test]
    fn an_alternation_records_two_witnesses_for_one_node_pair() {
        // ex:p forward and ex:q inverse both take ex:a to ex:b. A node-only path model
        // would report one hop; the two statements are two derivations.
        let data = dataset(&[("a", "p", "b"), ("b", "q", "a")]);
        let graph = snapshot(
            &data,
            &[("p", PathDirection::Forward), ("q", PathDirection::Inverse)],
        );
        let relation = PathWitnessRelation::new(graph, limits(1, 1));
        let mut bound = free();
        bound[POS_START] = Some(iri("a"));
        let rows = drained(&relation, &bound);

        assert_eq!(rows.len(), 2, "two statements, two walks: {rows:?}");
        assert_eq!(rows[0][POS_NODE], iri("b"));
        assert_eq!(rows[1][POS_NODE], iri("b"));
        assert_eq!(rows[0][POS_END], iri("b"));
        assert_eq!(rows[1][POS_END], iri("b"));
        // The statements are recorded in ASSERTED orientation whichever way they were
        // traversed, and they are frozen in TermValue order.
        assert_eq!(rows[0][POS_EDGE], stmt("a", "p", "b"));
        assert_eq!(rows[1][POS_EDGE], stmt("b", "q", "a"));
        assert_ne!(
            rows[0][POS_PATH_ID], rows[1][POS_PATH_ID],
            "two derivations must not share an identifier"
        );
    }

    // ---- 3: cycles terminate ---------------------------------------------

    fn cycle() -> (Arc<RdfDataset>, Arc<PathGraph>) {
        let data = dataset(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "a")]);
        let graph = snapshot(&data, &[("p", PathDirection::Forward)]);
        (data, graph)
    }

    #[test]
    fn a_cycle_terminates_at_the_closing_hop() {
        let (_data, graph) = cycle();
        let relation = PathWitnessRelation::new(graph, limits(1, 8));
        let mut bound = free();
        bound[POS_START] = Some(iri("a"));
        let rows = drained(&relation, &bound);

        // Exactly three walks — a→b, a→b→c, a→b→c→a — even though max_hops is 8: the
        // closing hop back to ex:a terminates the walk rather than extending it.
        let ends: Vec<(TermValue, TermValue)> = path_ids(&rows)
            .iter()
            .map(|id| {
                let row = rows.iter().find(|r| &r[POS_PATH_ID] == id).expect("row");
                (row[POS_END].clone(), row[POS_LEN].clone())
            })
            .collect();
        assert_eq!(
            ends,
            vec![(iri("b"), int(1)), (iri("c"), int(2)), (iri("a"), int(3)),]
        );
        assert_eq!(rows.len(), 6, "1 + 2 + 3 rows: {rows:?}");
        // The `p+` agreement: a node on a cycle reaches ITSELF, which a strictly simple
        // walk rule would silently omit.
        assert!(rows.iter().any(|row| row[POS_END] == iri("a")));
    }

    // ---- 4: an RDF 1.2 triple term as an intermediate node ---------------

    /// `ex:a ex:p <<ex:x ex:r ex:y>> . ex:z ex:q <<ex:x ex:r ex:y>> .`
    ///
    /// The triple term is reached forward over `ex:p` and left backward over `ex:q`, so it
    /// is a genuine INTERMEDIATE node of a two-hop walk. It is entered and left by
    /// different directions because the IR refuses a quoted triple in asserted SUBJECT
    /// position (`rdf-ir-triple-subject`), which is exactly why a step is an alternation
    /// of DIRECTED predicates rather than of predicates.
    fn triple_term_graph() -> (Arc<RdfDataset>, Arc<PathGraph>, TermValue) {
        let mut builder = RdfDatasetBuilder::new();
        let a = builder.intern_iri("http://example.org/a");
        let p = builder.intern_iri("http://example.org/p");
        let q = builder.intern_iri("http://example.org/q");
        let r = builder.intern_iri("http://example.org/r");
        let x = builder.intern_iri("http://example.org/x");
        let y = builder.intern_iri("http://example.org/y");
        let z = builder.intern_iri("http://example.org/z");
        let quoted = builder.intern_triple(x, r, y);
        builder.push_quad(a, p, quoted, None);
        builder.push_quad(z, q, quoted, None);
        let data = builder.freeze().expect("freeze");
        let graph = snapshot(
            &data,
            &[("p", PathDirection::Forward), ("q", PathDirection::Inverse)],
        );
        (data, graph, stmt("x", "r", "y"))
    }

    #[test]
    fn a_triple_term_is_an_ordinary_intermediate_node() {
        let (_data, graph, quoted) = triple_term_graph();
        let relation = PathWitnessRelation::new(graph, limits(1, 2));
        let mut bound = free();
        bound[POS_START] = Some(iri("a"));
        let rows = drained(&relation, &bound);

        // a→<<x q y>> (one hop) then a→<<x q y>>→z (two hops): 1 + 2 = 3 rows.
        assert_eq!(rows.len(), 3, "{rows:?}");
        assert_eq!(rows[0][POS_NODE], quoted, "step 1 binds the triple term");
        assert_eq!(rows[0][POS_END], quoted);
        assert_eq!(rows[1][POS_STEP], int(1));
        assert_eq!(rows[1][POS_NODE], quoted);
        assert_eq!(rows[2][POS_STEP], int(2));
        assert_eq!(rows[2][POS_NODE], iri("z"));
        // The hop INTO the triple term records the asserted statement whose object is it.
        assert_eq!(
            rows[0][POS_EDGE],
            TermValue::Triple {
                s: Box::new(iri("a")),
                p: Box::new(iri("p")),
                o: Box::new(quoted.clone()),
            }
        );
        // ...and the hop OUT of it records the ASSERTED orientation of the statement it
        // traversed backward, so the row joins straight back into the dataset.
        assert_eq!(
            rows[2][POS_EDGE],
            TermValue::Triple {
                s: Box::new(iri("z")),
                p: Box::new(iri("q")),
                o: Box::new(quoted),
            }
        );
    }

    // ---- 5: determinism, and a golden identifier -------------------------

    #[test]
    fn draining_one_invocation_twice_is_byte_identical() {
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(graph, limits(1, 3));
        let bound = free();
        assert_eq!(drained(&relation, &bound), drained(&relation, &bound));
    }

    #[test]
    fn the_path_identifier_is_golden() {
        // Pinned so any change to the domain separator, the term encoding, the fold order
        // or the length suffix is a loud failure rather than a silent re-identification of
        // grouping keys already handed to callers.
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(graph, limits(2, 2));
        let mut bound = free();
        bound[POS_START] = Some(iri("a"));
        let rows = drained(&relation, &bound);
        assert_eq!(rows.len(), 2, "one walk a→b→c, two hops");
        assert_eq!(
            rows[0][POS_PATH_ID],
            TermValue::simple_literal(GOLDEN_ABC_PATH_ID)
        );
    }

    /// The identifier of the walk `ex:a --ex:p--> ex:b --ex:p--> ex:c`.
    const GOLDEN_ABC_PATH_ID: &str =
        "3e4c617c5f08362717dfdbdaf9ced0e4db15c8253c13284e4ad7d6b7a8269c08";

    // ---- 6: the guards fail hard -----------------------------------------

    #[test]
    fn max_paths_per_seed_is_a_hard_failure() {
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(
            graph,
            PathLimits::new(1, 3, 1, 1_000_000).expect("envelope"),
        );
        let mut bound = free();
        bound[POS_START] = Some(iri("a"));
        let error = invoke(&relation, &bound, None).expect_err("two candidate walks, one allowed");
        assert!(
            error.to_string().contains("max_paths_per_seed"),
            "got {error}"
        );
        assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
    }

    #[test]
    fn max_expansions_per_invocation_is_a_hard_failure() {
        let (_data, graph) = chain();
        let relation =
            PathWitnessRelation::new(graph, PathLimits::new(1, 3, 4096, 1).expect("envelope"));
        let mut bound = free();
        bound[POS_START] = Some(iri("a"));
        let error = invoke(&relation, &bound, None).expect_err("three edges, one allowed");
        assert!(
            error.to_string().contains("max_expansions_per_invocation"),
            "got {error}"
        );
    }

    #[test]
    fn a_guard_message_names_the_licence_dependence_and_the_seed() {
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(
            graph,
            PathLimits::new(1, 3, 1, 1_000_000).expect("envelope"),
        );
        let mut bound = free();
        bound[POS_START] = Some(iri("a"));
        let error = invoke(&relation, &bound, None).expect_err("guard breach");
        let text = error.to_string();
        assert!(text.contains("path witness"), "{text}");
        assert!(text.contains("http://example.org/a"), "{text}");
        assert!(text.contains("row ceiling"), "{text}");
    }

    #[test]
    fn a_licence_can_stop_before_a_guard_would_fire() {
        // The same invocation that fails above completes under a ceiling of one row: the
        // guards bound work actually performed, not work the query text implies.
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(
            graph,
            PathLimits::new(1, 3, 1, 1_000_000).expect("envelope"),
        );
        let mut bound = free();
        bound[POS_START] = Some(iri("a"));
        let rows = invoke(&relation, &bound, Some(1)).expect("the ceiling stops first");
        assert_eq!(rows.len(), 1);
    }

    // ---- 7: constructor validation ---------------------------------------

    #[test]
    fn a_step_needs_at_least_one_alternative() {
        let error = PathStep::new(Vec::new()).expect_err("an empty alternation is no step");
        assert!(matches!(error, EvalError::Config(_)), "got {error:?}");
    }

    #[test]
    fn a_step_predicate_must_be_an_iri() {
        let error = PathStep::new(vec![(
            TermValue::simple_literal("p"),
            PathDirection::Forward,
        )])
        .expect_err("a literal names no statements");
        assert!(matches!(error, EvalError::Config(_)), "got {error:?}");
        assert!(error.to_string().contains("must be an IRI"), "got {error}");
    }

    #[test]
    fn a_step_refuses_a_duplicated_alternative() {
        let error = PathStep::new(vec![
            (iri("p"), PathDirection::Forward),
            (iri("q"), PathDirection::Inverse),
            (iri("p"), PathDirection::Forward),
        ])
        .expect_err("a duplicate doubles every walk through it");
        assert!(matches!(error, EvalError::Config(_)), "got {error:?}");
        assert!(error.to_string().contains("repeats"), "got {error}");
        // The same predicate in the OTHER direction is a different alternative.
        PathStep::new(vec![
            (iri("p"), PathDirection::Forward),
            (iri("p"), PathDirection::Inverse),
        ])
        .expect("opposite directions are distinct alternatives");
    }

    #[test]
    fn the_limits_reject_every_ill_formed_envelope() {
        for (min, max, paths, expansions) in [
            (0, 3, 1, 1),
            (3, 2, 1, 1),
            (1, MAX_HOPS_CAP + 1, 1, 1),
            (1, 3, 0, 1),
            (1, 3, 1, 0),
        ] {
            let Err(error) = PathLimits::new(min, max, paths, expansions) else {
                panic!("({min}, {max}, {paths}, {expansions}) is ill-formed and must be refused");
            };
            assert!(matches!(error, EvalError::Config(_)), "got {error:?}");
        }
        let ok = PathLimits::new(1, MAX_HOPS_CAP, 2, 3).expect("the cap itself is allowed");
        assert_eq!(ok.min_hops(), 1);
        assert_eq!(ok.max_hops(), MAX_HOPS_CAP);
        assert_eq!(ok.max_paths_per_seed(), 2);
        assert_eq!(ok.max_expansions_per_invocation(), 3);
    }

    #[test]
    fn an_absent_predicate_is_a_data_error_not_an_empty_adjacency() {
        let data = dataset(&[("a", "p", "b")]);
        let step = PathStep::new(vec![(iri("nowhere"), PathDirection::Forward)]).expect("step");
        let error = PathGraph::from_dataset(&*data, &step, GraphMatch::Default)
            .expect_err("a predicate naming nothing is a configuration pointing at nothing");
        assert!(matches!(error, EvalError::Data(_)), "got {error:?}");
    }

    #[test]
    fn an_interned_predicate_with_no_quads_in_scope_is_simply_empty() {
        // The other side of the same coin: `ex:q` IS interned, so its absence from the
        // default graph is a real, observable emptiness rather than a misconfiguration.
        let mut builder = RdfDatasetBuilder::new();
        let a = builder.intern_iri("http://example.org/a");
        let p = builder.intern_iri("http://example.org/p");
        let q = builder.intern_iri("http://example.org/q");
        let b = builder.intern_iri("http://example.org/b");
        let g = builder.intern_iri("http://example.org/g");
        builder.push_quad(a, p, b, None);
        builder.push_quad(a, q, b, Some(g));
        let data = builder.freeze().expect("freeze");
        let step = PathStep::new(vec![(iri("q"), PathDirection::Forward)]).expect("step");
        let graph =
            PathGraph::from_dataset(&*data, &step, GraphMatch::Default).expect("interned is fine");
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn the_snapshot_fingerprint_reports_what_it_was_built_from() {
        let (data, graph) = chain();
        let fingerprint = graph.snapshot_fingerprint();
        assert_eq!(fingerprint.node_count, 4);
        assert_eq!(fingerprint.edge_count, 3);
        assert_eq!(fingerprint.term_count, data.term_count());
        assert_eq!(fingerprint.stats_fingerprint, data.stats_fingerprint());
        assert_eq!(graph.node_count(), fingerprint.node_count);
        assert_eq!(graph.edge_count(), fingerprint.edge_count);
    }

    // ---- 8: the declared row bound is honest -----------------------------

    /// Every access pattern the sweep exercises, as `(label, bound argument vector)`.
    fn sweep_modes(sample_id: &TermValue) -> Vec<(&'static str, [Option<TermValue>; ROW_WIDTH])> {
        let mut bound_start = free();
        bound_start[POS_START] = Some(iri("a"));
        let mut bound_len = free();
        bound_len[POS_LEN] = Some(int(2));
        let mut bound_step = free();
        bound_step[POS_STEP] = Some(int(1));
        let mut bound_path_id = free();
        bound_path_id[POS_PATH_ID] = Some(sample_id.clone());
        vec![
            ("all free", free()),
            ("bound start", bound_start),
            ("bound len", bound_len),
            ("bound step", bound_step),
            ("bound pathId", bound_path_id),
        ]
    }

    fn assert_bound_is_honest(relation: &dyn PropertyFunction, fixture: &str) {
        let sample = drained(relation, &free()).first().map_or_else(
            || TermValue::simple_literal("none"),
            |row| row[POS_PATH_ID].clone(),
        );
        for (label, bound) in sweep_modes(&sample) {
            let refs: Vec<Option<&TermValue>> = bound.iter().map(Option::as_ref).collect();
            let (subject, object) = refs.split_at(1);
            let mode = PfArgs::new(subject, object).mode();
            let declared = relation.rows_per_invocation(mode);
            let emitted = drained(relation, &bound).len() as u64;
            assert!(
                emitted <= declared,
                "{fixture}/{label}: emitted {emitted} rows against a declared bound of {declared}"
            );
        }
    }

    #[test]
    fn the_row_bound_is_an_upper_bound_on_every_mode() {
        let (_data, chain_graph) = chain();
        assert_bound_is_honest(
            &PathWitnessRelation::new(Arc::clone(&chain_graph), limits(1, 3)),
            "chain",
        );
        assert_bound_is_honest(
            &ShortestPathWitnessRelation::new(chain_graph, limits(1, 3)),
            "chain/shortest",
        );

        let (_data, cycle_graph) = cycle();
        assert_bound_is_honest(
            &PathWitnessRelation::new(Arc::clone(&cycle_graph), limits(1, 8)),
            "cycle",
        );
        assert_bound_is_honest(
            &ShortestPathWitnessRelation::new(cycle_graph, limits(1, 8)),
            "cycle/shortest",
        );

        let (_data, quoted_graph, _quoted) = triple_term_graph();
        assert_bound_is_honest(
            &PathWitnessRelation::new(Arc::clone(&quoted_graph), limits(1, 2)),
            "triple term",
        );
        assert_bound_is_honest(
            &ShortestPathWitnessRelation::new(quoted_graph, limits(1, 2)),
            "triple term/shortest",
        );
    }

    #[test]
    fn exactly_one_all_free_mode_is_declared() {
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(graph, limits(1, 3));
        assert_eq!(relation.modes().len(), 1);
        assert_eq!(relation.modes()[0].code(), "fffffff");
        assert_eq!(relation.arity(), PfArity::new(1, 6));
        assert_eq!(relation.volatility(), Volatility::Stable);
        // The all-free mode subsumes every access pattern, so no invocation is refused.
        for code in ["fffffff", "bffffff", "bbfffff", "bbbbbbb"] {
            assert!(relation.admits(BindingPattern::from_code(code)), "{code}");
        }
    }

    // ---- 9: the ceiling is spent only on rows the engine will keep -------

    #[test]
    fn a_ceiling_is_never_spent_on_a_row_the_engine_would_drop() {
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(graph, limits(1, 3));
        let mut bound = free();
        bound[POS_END] = Some(iri("c"));
        let rows = invoke(&relation, &bound, Some(1)).expect("no guard breach");
        assert_eq!(rows.len(), 1, "the ceiling admits exactly one row");
        assert_eq!(
            rows[0][POS_END],
            iri("c"),
            "the single licenced row must agree with the bound ?end, or a LIMIT 1 query \
             would read a short bag as a complete one"
        );
    }

    #[test]
    fn a_bound_end_prunes_rather_than_merely_filters() {
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(graph, limits(1, 3));
        let mut bound = free();
        bound[POS_END] = Some(iri("c"));
        let rows = drained(&relation, &bound);
        assert!(rows.iter().all(|row| row[POS_END] == iri("c")));
        // a→b→c and b→c are the only walks ending at ex:c within three hops.
        assert_eq!(path_ids(&rows).len(), 2, "{rows:?}");
        assert_eq!(rows.len(), 3, "2 + 1 rows: {rows:?}");
    }

    #[test]
    fn an_unmatchable_bound_argument_opens_an_empty_cursor() {
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(graph, limits(1, 3));
        for position in [POS_START, POS_END] {
            let mut bound = free();
            bound[position] = Some(iri("absent"));
            assert!(drained(&relation, &bound).is_empty(), "position {position}");
        }
        for (position, value) in [
            (POS_LEN, int(9)),
            (POS_LEN, TermValue::simple_literal("not a number")),
            (POS_STEP, int(0)),
            (POS_STEP, int(9)),
        ] {
            let mut bound = free();
            bound[position] = Some(value.clone());
            assert!(
                drained(&relation, &bound).is_empty(),
                "position {position} bound to {value:?}"
            );
        }
    }

    #[test]
    fn a_bound_step_yields_one_row_per_walk_long_enough_to_have_it() {
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(graph, limits(1, 3));
        let mut bound = free();
        bound[POS_START] = Some(iri("a"));
        bound[POS_STEP] = Some(int(3));
        let rows = drained(&relation, &bound);
        assert_eq!(rows.len(), 1, "only the three-hop walk has a third step");
        assert_eq!(rows[0][POS_LEN], int(3));
        assert_eq!(rows[0][POS_NODE], iri("d"));
    }

    #[test]
    fn a_bound_path_id_selects_exactly_its_own_walk() {
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(graph, limits(1, 3));
        let all = drained(&relation, &free());
        let ids = path_ids(&all);
        for id in &ids {
            let mut bound = free();
            bound[POS_PATH_ID] = Some(id.clone());
            let rows = drained(&relation, &bound);
            assert!(!rows.is_empty());
            assert!(rows.iter().all(|row| &row[POS_PATH_ID] == id));
            assert_eq!(path_ids(&rows).len(), 1);
        }
    }

    // ---- 10: one shortest witness per reachable endpoint -----------------

    #[test]
    fn the_shortest_relation_yields_one_minimal_witness_per_endpoint() {
        // A diamond (a→b→d, a→c→d) plus a longer detour (a→e→f→d).
        let data = dataset(&[
            ("a", "p", "b"),
            ("b", "p", "d"),
            ("a", "p", "c"),
            ("c", "p", "d"),
            ("a", "p", "e"),
            ("e", "p", "f"),
            ("f", "p", "d"),
        ]);
        let graph = snapshot(&data, &[("p", PathDirection::Forward)]);
        let relation = ShortestPathWitnessRelation::new(graph, limits(1, 8));
        let mut bound = free();
        bound[POS_START] = Some(iri("a"));
        let rows = drained(&relation, &bound);

        // One witness per reachable endpoint, in breadth-first discovery order.
        let witnesses: Vec<(TermValue, TermValue)> = path_ids(&rows)
            .iter()
            .map(|id| {
                let row = rows.iter().find(|r| &r[POS_PATH_ID] == id).expect("row");
                (row[POS_END].clone(), row[POS_LEN].clone())
            })
            .collect();
        assert_eq!(
            witnesses,
            vec![
                (iri("b"), int(1)),
                (iri("c"), int(1)),
                (iri("e"), int(1)),
                (iri("d"), int(2)),
                (iri("f"), int(2)),
            ],
            "one witness per endpoint, minimal length, discovery order: {rows:?}"
        );
        assert_eq!(rows.len(), 1 + 1 + 1 + 2 + 2, "{rows:?}");

        // The tie between a→b→d and a→c→d is broken by the frozen order: ex:b is
        // dequeued before ex:c, so ex:b is the recorded predecessor.
        let to_d: Vec<&PfRow> = rows.iter().filter(|row| row[POS_END] == iri("d")).collect();
        assert_eq!(to_d.len(), 2);
        assert_eq!(to_d[0][POS_NODE], iri("b"));
        assert_eq!(to_d[1][POS_NODE], iri("d"));
        assert_eq!(to_d[0][POS_EDGE], stmt("a", "p", "b"));
        assert_eq!(to_d[1][POS_EDGE], stmt("b", "p", "d"));
    }

    #[test]
    fn the_shortest_relation_reports_a_cycle_through_its_own_seed() {
        let (_data, graph) = cycle();
        let relation = ShortestPathWitnessRelation::new(graph, limits(1, 8));
        let mut bound = free();
        bound[POS_START] = Some(iri("a"));
        let rows = drained(&relation, &bound);
        assert!(
            rows.iter().any(|row| row[POS_END] == iri("a")),
            "a seed on a cycle reaches itself, exactly as p+ says: {rows:?}"
        );
        // b at 1, c at 2, a at 3 — one witness each.
        assert_eq!(rows.len(), 1 + 2 + 3, "{rows:?}");
    }

    #[test]
    fn the_shortest_relation_is_deterministic_and_stops_at_its_ceiling() {
        let (_data, graph) = cycle();
        let relation = ShortestPathWitnessRelation::new(graph, limits(1, 8));
        let bound = free();
        assert_eq!(drained(&relation, &bound), drained(&relation, &bound));
        let capped = invoke(&relation, &bound, Some(2)).expect("no guard breach");
        assert_eq!(capped.len(), 2);
        assert_eq!(capped, drained(&relation, &bound)[..2].to_vec());
    }

    #[test]
    fn the_shortest_relation_guards_fail_hard_too() {
        let (_data, graph) = chain();
        let relation = ShortestPathWitnessRelation::new(
            Arc::clone(&graph),
            PathLimits::new(1, 3, 1, 1_000_000).expect("envelope"),
        );
        let mut bound = free();
        bound[POS_START] = Some(iri("a"));
        let error = invoke(&relation, &bound, None).expect_err("three witnesses, one allowed");
        assert!(
            error.to_string().contains("max_paths_per_seed"),
            "got {error}"
        );
        assert!(
            error.to_string().contains("shortest path witness"),
            "got {error}"
        );

        let relation =
            ShortestPathWitnessRelation::new(graph, PathLimits::new(1, 3, 4096, 1).expect("env"));
        let error = invoke(&relation, &bound, None).expect_err("three edges, one allowed");
        assert!(
            error.to_string().contains("max_expansions_per_invocation"),
            "got {error}"
        );
    }

    #[test]
    fn a_wrong_argument_count_is_refused_before_the_traversal() {
        let (_data, graph) = chain();
        let relation = PathWitnessRelation::new(Arc::clone(&graph), limits(1, 3));
        let subject = [None];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let error = relation
            .open(&args, None)
            .err()
            .expect("a one-value object side does not match the declaration");
        assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
        let shortest = ShortestPathWitnessRelation::new(graph, limits(1, 3));
        assert!(shortest.open(&args, None).is_err());
    }

    #[test]
    fn both_relations_share_one_snapshot() {
        let (_data, graph) = chain();
        let exhaustive = PathWitnessRelation::new(Arc::clone(&graph), limits(1, 3));
        let shortest = ShortestPathWitnessRelation::new(Arc::clone(&graph), limits(1, 3));
        assert_eq!(
            exhaustive.rows_per_invocation(BindingPattern::from_code("fffffff")) > 0,
            shortest.rows_per_invocation(BindingPattern::from_code("fffffff")) > 0
        );
        assert_eq!(Arc::strong_count(&graph), 3);
    }
}
