// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Engine-side variable **pre-binding** (purrdf S6,  GAP-A).
//!
//! Bridges the engine's egress term model ([`TermValue`]) to the algebra's
//! [`Query::substitute_variable`] rewrite. Each `(name, value)` of a
//! [`SparqlRequest::substitutions`](purrdf_core::SparqlRequest) pre-binds the
//! query variable `name` to `value` before evaluation, exactly mirroring oxigraph's
//! `PreparedSparqlQuery::substitute_variable` (the SHACL `$this` focus-node path).
//!
//! The substitution is applied to a **clone** of the cached (un-substituted) parse,
//! so the plan cache is never poisoned by a focus-node-specific binding.
//!
//! The rewrite has two halves. The `VALUES` seed BINDS the variable; the pushdown in
//! [`push_probe_constants`] makes the pattern MATCH on it, by writing the constant
//! into the triple-pattern positions that can carry it. Without the second half the
//! seed join is the only thing narrowing the answer, and a join evaluates both of its
//! operands in full — so a pre-bound `$this` used to cost a scan of the whole graph
//! per focus node. See that function for the measurement and the soundness argument.
//!
//! [`substitute_in_graph_pattern`]'s walk below is a DIFFERENT, DELIBERATELY DIVERGENT
//! substitution from `crate::expr`'s `Replace`/Values-Insertion walk (SEP-0007's
//! mechanism, which [`crate::expr::exists`]'s per-row definition path and `LATERAL`'s
//! per-row evaluation both use) — see `crate::enf`'s module doc, "The SHACL pre-binding
//! fork", for the three specific differences and why each is a deliberate design choice
//! rather than an inconsistency.

use purrdf_core::{RdfDiagnostic, RdfTextDirection, TermValue};
use purrdf_sparql_algebra::{
    AggregateExpression, BaseDirection, BlankNode, Expression, GraphPattern, GroundTerm,
    GroundTriple, Literal, NamedNode, NamedNodePattern, OrderExpression, Query, TermPattern,
    TriplePattern, Variable,
};

/// The pre-binding list, in whichever of the two shapes the caller has.
///
/// `Copy` and reference-sized: this is the whole point of it. The two request
/// types — [`SparqlRequest`](purrdf_core::SparqlRequest)'s owned
/// `&[(String, TermValue)]` and the interned entries' borrowed-name
/// [`Prebinding`](crate::interned::Prebinding) slice — reach the same rewrite
/// without either of them being converted into the other, which would have cost
/// the allocation-per-focus-node the borrowed form exists to remove.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Prebindings<'a> {
    /// A generic consumer's list, names owned.
    Owned(&'a [(String, TermValue)]),
    /// An interned entry point's list, names borrowed from the caller.
    Borrowed(&'a [crate::interned::Prebinding<'a>]),
    /// A prepared execution's parameters and their current values, as parallel
    /// slices.
    ///
    /// The handle owns its parameter list as already-interned [`Variable`]s and its
    /// bindings as a separate vector it overwrites per execution, so neither can be
    /// zipped into one slice without building it — which is the per-execution
    /// allocation a prepared execution exists to remove. Reading them side by side
    /// costs nothing, and the name never has to be re-interned because the
    /// [`Variable`] is already here.
    Paired(&'a [Variable], &'a [Option<TermValue>]),
}

impl<'a> Prebindings<'a> {
    /// How many variables are pre-bound.
    pub(crate) fn len(self) -> usize {
        match self {
            Self::Owned(list) => list.len(),
            Self::Borrowed(list) => list.len(),
            Self::Paired(names, _) => names.len(),
        }
    }

    /// Whether nothing is pre-bound, which is the rewrite-free hot path.
    pub(crate) fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// The `(name, value)` pairs, names borrowed in both shapes.
    pub(crate) fn iter(self) -> impl Iterator<Item = (&'a str, &'a TermValue)> {
        (0..self.len()).map(move |index| self.get(index))
    }

    /// The `index`-th `(name, value)` pair.
    ///
    /// Indexed rather than delegating to the two underlying iterators, because the
    /// two shapes have different iterator TYPES and one function cannot return
    /// both: the alternatives are a boxed trait object, which allocates on a path
    /// that exists to stop allocating, or chaining two `Option`s of which exactly
    /// one is always empty, which reads like a mistake. `Self` is `Copy`, so the
    /// closure in [`Self::iter`] captures it by value and the match below folds to
    /// a single branch.
    ///
    /// # Panics
    ///
    /// If `index` is out of range, exactly as indexing the underlying slice would.
    fn get(self, index: usize) -> (&'a str, &'a TermValue) {
        match self {
            Self::Owned(list) => (list[index].0.as_str(), &list[index].1),
            Self::Borrowed(list) => (list[index].variable, &list[index].value),
            Self::Paired(names, values) => (
                names[index].as_str(),
                values[index]
                    .as_ref()
                    .expect("a prepared execution refuses to run with a parameter unbound"),
            ),
        }
    }

    /// The `index`-th pre-binding's [`Variable`].
    ///
    /// A prepared execution already holds one, so it is cloned — an `Arc` refcount
    /// bump — rather than looked up by name. Every other shape carries only the name
    /// and goes through the per-worker interner.
    fn variable(self, index: usize) -> Variable {
        match self {
            Self::Paired(names, _) => names[index].clone(),
            _ => interned_variable(self.get(index).0),
        }
    }
}

/// Apply every `(name, value)` substitution to `query` as a pre-binding rewrite,
/// returning the rewritten query. Each value is mapped to the algebra's
/// [`GroundTerm`] (blank-node focus nodes ride the injection-only
/// [`GroundTerm::BlankNode`]) and injected as a single-row `VALUES` join at the core
/// `WHERE` pattern, beneath the solution-modifier stack but visible to the projected
/// variable list.
///
/// # Errors
///
/// Returns a [`RdfDiagnostic`] if a literal substitution carries a datatype IRI that
/// is not a syntactically valid IRI, or a language tag the RDF concrete syntaxes
/// would not have lexed (the two ways a [`TermValue`] cannot become a
/// [`GroundTerm`]). A pre-binding is an instruction to narrow the answer, so a
/// component that cannot be made into a term is reported to the caller rather than
/// degraded into an `UNDEF` cell that would silently widen it — see [`lang`].
pub(crate) fn apply_substitutions(
    query: Query,
    substitutions: Prebindings<'_>,
) -> Result<Query, RdfDiagnostic> {
    let probes = build_probes(substitutions)?;
    if probes.is_empty() {
        // Nothing to push and nothing to seed. [`push_probe_constants`] returns its
        // argument untouched for an empty probe list and a `map_core_pattern` whose
        // body is the identity rebuilds the query it was handed, so descending at all
        // here would be a walk with no rewrite in it.
        return Ok(query);
    }
    Ok(apply_probes(query, probes))
}

/// How many distinct pre-binding names one worker keeps interned.
///
/// A pre-binding name is shape text — `$this`, `$value`, `$PATH`, a component's
/// declared parameters — so a workload's whole vocabulary is a few dozen entries.
/// The cap exists so a process that evaluates an unbounded stream of distinct query
/// texts cannot grow this without limit; reaching it clears rather than evicts,
/// because the table is a memo of a pure function and losing it costs one
/// reconstruction rather than a wrong answer.
const INTERNED_VARIABLE_CAP: usize = 1_024;

thread_local! {
    /// `Variable`s for pre-binding names, interned per worker.
    ///
    /// `Variable::new` takes `impl Into<String>`, so building one from a borrowed
    /// name allocates a `String` AND the `Arc<str>` it converts into — twice per
    /// pre-bound variable, per focus node, for a name that is constant across every
    /// focus node in the run. `Prebinding::variable` is a `&str` precisely to avoid
    /// owning that text; this stops the rewrite re-owning it one layer down.
    ///
    /// Keyed by `Box<str>` and probed by `&str`: `Box<str>: Borrow<str>`, so a HIT
    /// hashes the borrowed name and allocates nothing, and only a MISS owns a copy.
    /// That is the same borrowed-probe shape the plan cache's key buffer uses.
    ///
    /// Thread-local rather than engine-owned because the value it caches has no
    /// lifetime relation to any engine, and because `NativeSparqlEngine` is itself
    /// held in a thread-local by every parallel caller — so per-worker is what
    /// engine-owned would have given anyway, without threading a cache reference
    /// through four call layers into a free function. Like
    /// `crate::parallel`'s sequencing flag it is per-worker state with no staleness
    /// dimension: a name maps to one `Variable` forever, and nothing about a dataset
    /// or a plan is captured in it.
    static INTERNED_VARIABLES: std::cell::RefCell<crate::DetHashMap<Box<str>, Variable>> =
        std::cell::RefCell::new(crate::DetHashMap::default());
}

/// The [`Variable`] for `name`, interned per worker.
pub(crate) fn interned_variable(name: &str) -> Variable {
    INTERNED_VARIABLES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(var) = cache.get(name) {
            return var.clone();
        }
        if cache.len() >= INTERNED_VARIABLE_CAP {
            cache.clear();
        }
        let var = Variable::new(name);
        cache.insert(Box::from(name), var.clone());
        var
    })
}

/// Ground every pre-binding once: one [`Variable`] per name, one [`GroundTerm`] per
/// value.
///
/// Separate from [`apply_probes`] because [`apply_shacl_prebinding`] needs the
/// grounded values for its expression-position rewrite as well as for the seed, and
/// deriving them twice from the same [`TermValue`]s was a measured per-focus-node
/// cost: the conversion allocates, and SHACL runs it once per focus node, per value
/// node, or per argument tuple.
///
/// # Errors
///
/// As [`apply_substitutions`]: a datatype IRI that is not a valid IRI, or a language
/// tag the concrete syntaxes would not have lexed.
fn build_probes(
    substitutions: Prebindings<'_>,
) -> Result<Vec<(Variable, GroundTerm)>, RdfDiagnostic> {
    let mut probes = Vec::with_capacity(substitutions.len());
    for index in 0..substitutions.len() {
        let (_, value) = substitutions.get(index);
        probes.push((
            substitutions.variable(index),
            ground_term_from_value(value)?,
        ));
    }
    Ok(probes)
}

/// [`apply_substitutions`]'s rewrite, over probes that are already grounded and
/// already known to be non-empty.
fn apply_probes(query: Query, probes: Vec<(Variable, GroundTerm)>) -> Query {
    // ONE seed carrying every pre-binding, not one seed per pre-binding.
    //
    // `Query::substitute_variable` joins a single-row `VALUES` binding ONE variable
    // onto the core, descending the whole solution-modifier wrapper stack to find
    // it. Calling it once per substitution therefore rebuilt that wrapper stack, and
    // minted a fresh `Join` and a fresh single-row `VALUES`, once per pre-bound
    // variable — three times over for a `sh:ask` validator's `$this`/`$value`/
    // parameter set, on every value node of every focus node. A single row binding
    // {a, b, c} and three successive single-row joins binding a, then b, then c are
    // the same solution mapping (each row is compatible with the next by
    // construction, the variables being distinct), so the whole set rides one node.
    //
    // A REPEATED variable name is the one case where that equivalence fails: two
    // seeds binding the same variable to different terms are incompatible and yield
    // the empty solution, while one `VALUES` row cannot even spell the second
    // binding. That case keeps the original per-variable path — its own pushdown
    // descent, then one `substitute_variable` per pre-binding — so its behaviour is
    // unchanged rather than approximated.
    let mut query = query;
    if has_repeated_variable(&probes) {
        query.map_core_pattern_mut(|core| push_probe_constants(core, &probes));
        for (var, ground) in probes {
            query.substitute_variable_mut(&var, ground);
        }
        return query;
    }
    // ONE descent doing both rewrites, in the order the two separate descents ran
    // them.
    //
    // Pushdown FIRST, seed second: the pushdown's `at_core_root` peephole is stated
    // about the node the seed is going to wrap. Running the seed first would put a
    // `Values` mentioning the pre-bound variable between the core and this rewrite,
    // and the peephole would be describing the seed rather than the pattern. That
    // ordering is preserved here exactly — `push_probe_constants` is applied to the
    // core, and the `Join` is built AROUND its result — and now costs one descent
    // instead of two, because both walks reached the same node through the same
    // `map_core_pattern` descent and the pushdown leaves that node's variant alone
    // (`push_probes` maps every arm onto its own variant, and `at_core_root`
    // suppresses the restoring `Values` there), so the second descent could only
    // ever have stopped where the first one did.
    query.map_core_pattern_mut(move |core| {
        push_probe_constants(core, &probes);
        let mut variables = Vec::with_capacity(probes.len());
        let mut row = Vec::with_capacity(probes.len());
        for (var, ground) in probes {
            variables.push(var);
            row.push(Some(ground));
        }
        purrdf_sparql_algebra::substitute::take_and_replace(core, |core| GraphPattern::Join {
            left: Box::new(GraphPattern::Values {
                variables,
                bindings: vec![row],
            }),
            right: Box::new(core),
        });
    });
    query
}

/// Whether any variable is pre-bound twice, which is the one shape the combined
/// seed in [`apply_substitutions`] cannot represent.
///
/// Quadratic on purpose: a pre-binding list is the handful of variables one shape
/// names (`$this`, `$value`, `$shapesGraph`, `$currentShape`, the component's
/// parameters), and a hash set over it would cost an allocation per focus node to
/// avoid a comparison that never runs more than a few times.
fn has_repeated_variable(probes: &[(Variable, GroundTerm)]) -> bool {
    probes
        .iter()
        .enumerate()
        .any(|(index, (var, _))| probes[..index].iter().any(|(seen, _)| seen == var))
}

/// Push each pre-binding's constant into the triple-pattern positions that can
/// match it, so the pattern is an **index probe on the bound position** instead of
/// a scan the single-row `VALUES` join filters afterwards.
///
/// # The defect this closes
///
/// [`Query::substitute_variable`] binds a pre-bound variable by joining a
/// single-row `VALUES` seed onto the core `WHERE` pattern, and NOTHING else — the
/// triple patterns keep the variable. `crate::binop::eval_join` evaluates both
/// operands in full and then hash-joins, so `$this <p> ?o` with `$this` pre-bound
/// enumerated EVERY `<p>` quad in the graph and discarded all but one subject's.
/// Per focus node that is O(|graph|), and SHACL runs one query per focus node, so a
/// validation was O(focus nodes x |graph|) in a place whose whole cost should have
/// been the size of one subject's fan-out. Measured on a `sh:sparql` node shape at a
/// fixed 64 focus nodes, the per-focus-node cost tracked the data graph exactly:
/// 383 allocations over a 768-quad graph, 8,324 over a 24,576-quad graph — a
/// doubling of the graph doubled the per-focus-node cost. It is a planning defect,
/// not the honest price of running a query.
///
/// After this rewrite the pattern carries the constant, so `crate::bgp`'s
/// `compile_pattern` resolves it to a [`Pos::Bound`](crate::bgp) and the
/// index-nested-loop join probes the P4 permutation index directly.
///
/// # Why it is sound
///
/// Joining a SINGLE-ROW `VALUES` binding `var = c` onto a pattern `P` keeps exactly
/// those solutions of `P` that bind `var` to `c` or leave it unbound, and adds
/// `var = c` to each. Replacing `var` with `c` inside a `Bgp`/`Path` leaf produces
/// exactly the leaf's `var = c` solutions — with the `var` COLUMN dropped, which is
/// the only observable difference. [`restore_probed_bindings`] puts that column
/// back with its own single-row `VALUES`, so every enclosing operator sees an
/// unchanged schema and a restricted row set, and "restrict the leaf" and "restrict
/// the subtree" coincide.
///
/// The recursion therefore descends only the operators for which restricting an
/// operand restricts the node's output the same way:
///
/// * `Join`, `Union`, `Graph`, `Filter`, `Extend` — both/inner operands;
/// * `LeftJoin`, `Minus`, `Lateral` — the LEFT operand only. Restricting the right
///   arm of an `OPTIONAL` is NOT the same rewrite: a left row whose only match
///   binds `var` to some other `d` is a MATCH before the rewrite (and its `var = d`
///   output row is then dropped by the seed join, taking the left row with it) and
///   a MISS after it (so the left row survives, null-padded). `MINUS` diverges the
///   same way through its compatibility test. Both keep their right arms untouched.
///
/// Every other operator stops the descent. `Project` is the one worth naming: a
/// sub-`SELECT` is a separate scope, and a `var` inside one that the projection does
/// not carry is a DIFFERENT variable that the seed join cannot correlate with.
///
/// `Extend` binds `var` itself when `var` is its target, so that variable is dropped
/// from the candidate set for its operand rather than pushed into a subtree where it
/// does not yet exist.
///
/// # What is not pushed
///
/// A [`GroundTerm::BlankNode`] pre-binding. A blank node in a query pattern is a
/// non-distinguished VARIABLE (SPARQL 1.2 §4.1.4), not a request to match a
/// particular dataset blank, so writing one into a triple pattern would widen the
/// match to every term rather than narrow it to the focus node. Blank-node focus
/// nodes keep the `VALUES`-join path, which interns the blank as the term it is.
fn push_probe_constants(core: &mut GraphPattern, probes: &[(Variable, GroundTerm)]) {
    if probes.is_empty() {
        return;
    }
    push_probes(core, probes, true);
}

/// [`push_probe_constants`]'s recursion.
///
/// `at_core_root` is true only for the node the single-row seed will be joined
/// directly onto. A leaf THERE needs no restoring `VALUES` of its own: the seed is
/// about to supply exactly that binding, one node up, to everything above it. This
/// is the shape every SHACL-SPARQL constraint body has — `SELECT $this WHERE {
/// $this <p> ?v . FILTER(...) }` lowers to `Project(Filter(Bgp))` and
/// `map_core_pattern` descends both wrappers — so the hot path pays for no extra
/// algebra node at all.
fn push_probes(pattern: &mut GraphPattern, probes: &[(Variable, GroundTerm)], at_core_root: bool) {
    match pattern {
        GraphPattern::Bgp { patterns } => {
            let mut probed = Vec::new();
            for triple in patterns.iter_mut() {
                probe_triple_pattern(triple, probes, &mut probed);
            }
            restore_probed_bindings(pattern, &probed, probes, at_core_root);
        }
        GraphPattern::Path {
            subject, object, ..
        } => {
            let mut probed = Vec::new();
            probe_term_pattern(subject, probes, &mut probed);
            probe_term_pattern(object, probes, &mut probed);
            restore_probed_bindings(pattern, &probed, probes, at_core_root);
        }
        GraphPattern::Join { left, right } | GraphPattern::Union { left, right } => {
            push_probes(left, probes, false);
            push_probes(right, probes, false);
        }
        GraphPattern::Graph { inner, .. } | GraphPattern::Filter { inner, .. } => {
            push_probes(inner, probes, false);
        }
        GraphPattern::Extend {
            inner, variable, ..
        } => {
            // `variable` is this node's own binding, so it is not bound in `inner`
            // and there is nothing there to narrow. Narrowing the candidate set is
            // only ever needed for an ill-formed query the parser would reject, and
            // costs one allocation on a branch no well-formed query takes.
            let narrowed: Vec<(Variable, GroundTerm)>;
            let inner_probes = if probes.iter().any(|(var, _)| var == &*variable) {
                narrowed = probes
                    .iter()
                    .filter(|(var, _)| var != &*variable)
                    .cloned()
                    .collect();
                &narrowed
            } else {
                probes
            };
            push_probes(inner, inner_probes, false);
        }
        // Left operand only — see [`push_probe_constants`]'s soundness note. The
        // right arms are not merely left unrecursed: they are never reached through
        // this match at all, which is what makes the boundary a property of the
        // shape of this function rather than of remembering to stop.
        GraphPattern::LeftJoin { left, .. }
        | GraphPattern::Minus { left, .. }
        | GraphPattern::Lateral { left, .. } => push_probes(left, probes, false),
        _ => {}
    }
}

/// Re-bind the columns a leaf rewrite consumed, unless the seed is about to.
///
/// `probed` holds the indices into `probes` of the variables this leaf really
/// replaced, so a leaf that matched none is returned untouched and costs nothing.
fn restore_probed_bindings(
    leaf: &mut GraphPattern,
    probed: &[usize],
    probes: &[(Variable, GroundTerm)],
    at_core_root: bool,
) {
    if probed.is_empty() || at_core_root {
        return;
    }
    let variables = probed.iter().map(|&i| probes[i].0.clone()).collect();
    let row = probed.iter().map(|&i| Some(probes[i].1.clone())).collect();
    purrdf_sparql_algebra::substitute::take_and_replace(leaf, |leaf| GraphPattern::Join {
        left: Box::new(leaf),
        right: Box::new(GraphPattern::Values {
            variables,
            bindings: vec![row],
        }),
    });
}

/// Push constants into a triple pattern's subject and object, recording which
/// candidates were consumed. The predicate is a [`NamedNodePattern`] and a
/// pre-binding that reached it would be pre-binding a PREDICATE variable, which
/// [`substitute_in_named_node_pattern`] already handles on the SHACL path.
fn probe_triple_pattern(
    triple: &mut TriplePattern,
    probes: &[(Variable, GroundTerm)],
    probed: &mut Vec<usize>,
) {
    probe_term_pattern(&mut triple.subject, probes, probed);
    probe_term_pattern(&mut triple.object, probes, probed);
}

/// Replace one term position with its pre-bound constant, recursing into a quoted
/// triple's own positions. A position that is not a candidate variable is returned
/// unchanged.
fn probe_term_pattern(
    term: &mut TermPattern,
    probes: &[(Variable, GroundTerm)],
    probed: &mut Vec<usize>,
) {
    match term {
        TermPattern::Variable(var) => {
            // Both early exits leave `probed` alone as well as the term. Recording an
            // index here without writing the constant would emit a restoring `VALUES`
            // for a column the leaf never consumed; for a blank-node pre-binding that
            // is the difference between the injection-only `VALUES` path and matching
            // every term in the graph.
            let Some(index) = probes.iter().position(|(candidate, _)| candidate == var) else {
                return;
            };
            let Some(constant) = term_pattern_from_ground(&probes[index].1) else {
                return;
            };
            if !probed.contains(&index) {
                probed.push(index);
            }
            *term = constant;
        }
        TermPattern::Triple(triple) => probe_triple_pattern(triple, probes, probed),
        _ => {}
    }
}

/// The triple-pattern spelling of a ground pre-binding, or `None` for the one term
/// kind a pattern position cannot carry.
///
/// [`GroundTerm::BlankNode`] is that kind: a blank in a pattern is an anonymous
/// variable, so it would match everything rather than the pre-bound blank. See
/// [`push_probe_constants`], "What is not pushed".
fn term_pattern_from_ground(ground: &GroundTerm) -> Option<TermPattern> {
    match ground {
        GroundTerm::NamedNode(node) => Some(TermPattern::NamedNode(node.clone())),
        GroundTerm::Literal(literal) => Some(TermPattern::Literal(literal.clone())),
        GroundTerm::Triple(triple) => Some(TermPattern::Triple(Box::new(TriplePattern {
            subject: term_pattern_from_ground(&triple.subject)?,
            predicate: NamedNodePattern::NamedNode(triple.predicate.clone()),
            object: term_pattern_from_ground(&triple.object)?,
        }))),
        GroundTerm::BlankNode(_) => None,
    }
}

/// Apply SHACL-SPARQL pre-binding to `query`.
///
/// First performs the ordinary VALUES-join rewrite via [`apply_substitutions`]
/// (so triple-pattern positions and projectable variables work exactly like the
/// generic pre-binding path). Then walks the algebra and, for every pre-bound
/// variable whose value is an IRI or literal:
///
/// * replaces `Expression::Variable(v)` with the constant IRI/literal,
/// * replaces `Expression::Bound(v)` with the `true` boolean literal,
///
/// recursing into nested graph patterns (`EXISTS`, `GRAPH`, sub-queries, etc.).
/// Blank-node and quoted-triple values are deliberately left unsubstituted in
/// expression positions; the VALUES-join binds them.
///
/// Returns a diagnostic on the same error conditions as [`apply_substitutions`].
pub(crate) fn apply_shacl_prebinding(
    query: Query,
    substitutions: Prebindings<'_>,
) -> Result<Query, RdfDiagnostic> {
    let probes = build_probes(substitutions)?;
    if probes.is_empty() {
        // Both halves are the identity over an empty pre-binding list, and the
        // expression walk below is a full walk-and-REBUILD of every pattern in the
        // query — so without this the no-substitution case paid for a complete copy
        // of the algebra to change nothing in it.
        return Ok(query);
    }

    // The expression-position constants come from the SAME grounded values the seed
    // is about to carry, not from a second conversion of the same `TermValue`s.
    // `NamedNode`, `Literal` and `Variable` are all `Arc<str>`-backed, so lifting one
    // out of a probe is a refcount bump; re-grounding it is a fresh allocation, once
    // per pre-bound value, per focus node.
    let mut entries = Vec::with_capacity(probes.len());
    for ((name, _), (_, ground)) in substitutions.iter().zip(probes.iter()) {
        entries.push((name, expression_from_ground(ground)));
    }
    let expr_subs = ExprSubs(entries);

    let mut query = apply_probes(query, probes);
    map_patterns_in_query(&mut query, |pattern| {
        substitute_in_graph_pattern(pattern, &expr_subs);
    });
    Ok(query)
}

/// What each pre-bound variable becomes in an EXPRESSION position, keyed by the
/// variable's borrowed name.
///
/// An association list scanned linearly, deliberately, where this was a
/// `HashMap<String, _>`. Two reasons, and the second is the one that matters:
///
/// * **It is the wrong size for a map.** A pre-binding list is the handful of
///   variables one shape names. Hashing a string to find one of three entries is
///   slower than comparing three short strings, and it charged one heap allocation
///   for the table on every query — which SHACL runs once per focus node.
/// * **The keys were copies of constants.** `HashMap<String, _>` forced an owned
///   key, so every entry cloned a name that came in borrowed and was already
///   allocated inside the loaded shapes graph — the same per-focus-node copy of
///   shape text [`Prebinding`](crate::interned::Prebinding) exists to stop paying
///   for, reintroduced one layer down.
///
/// The `Option` is the variable's constant expression, or `None` for a blank-node
/// or quoted-triple pre-binding, which has no expression form and rides the
/// `VALUES` join instead. Its ABSENCE from the list means "not pre-bound at all",
/// which is a different answer — [`substitute_in_expression`]'s `Bound` arm turns
/// on exactly that distinction.
struct ExprSubs<'a>(Vec<(&'a str, Option<Expression>)>);

impl ExprSubs<'_> {
    /// The entry for `name`, mirroring `HashMap::get`.
    fn get(&self, name: &str) -> Option<&Option<Expression>> {
        self.0
            .iter()
            .find_map(|(key, expr)| (*key == name).then_some(expr))
    }

    /// Whether `name` is pre-bound at all, mirroring `HashMap::contains_key`.
    fn contains_key(&self, name: &str) -> bool {
        self.0.iter().any(|(key, _)| *key == name)
    }
}

/// Lift an already-grounded pre-binding into an [`Expression`] when it is an IRI or
/// literal; `None` for a blank node or quoted triple, which has no expression form
/// and rides the `VALUES` join instead.
///
/// Takes the [`GroundTerm`] rather than the [`TermValue`] it came from precisely so
/// the conversion is not repeated: the probe list already holds it, and both
/// `NamedNode` and `Literal` are `Arc<str>`-backed, so this clone allocates nothing.
fn expression_from_ground(ground: &GroundTerm) -> Option<Expression> {
    match ground {
        GroundTerm::NamedNode(node) => Some(Expression::NamedNode(node.clone())),
        GroundTerm::Literal(lit) => Some(Expression::Literal(lit.clone())),
        GroundTerm::BlankNode(_) | GroundTerm::Triple(_) => None,
    }
}

/// Walk and rebuild the whole [`Query`], applying `f` to every [`GraphPattern`]
/// contained in it (including sub-queries).
fn map_patterns_in_query(query: &mut Query, f: impl FnOnce(&mut GraphPattern)) {
    match query {
        Query::Select { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. }
        | Query::Ask { pattern, .. } => f(pattern),
    }
}

/// Recursively substitute pre-bound variables into a [`GraphPattern`].
fn substitute_in_graph_pattern(pattern: &mut GraphPattern, expr_subs: &ExprSubs<'_>) {
    // Wildcard-free on purpose: a `GraphPattern` variant added later must fail to
    // compile here rather than silently pass through unsubstituted.
    match pattern {
        // A leaf's term positions are matched against the graph, not evaluated, and
        // `apply_substitutions`' pushdown has already written the pre-bound constants
        // into the ones that can carry them. A `Values` block's cells are data for the
        // same reason.
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => {}
        // BOTH arms, unlike the pushdown. Replacing a variable with a constant
        // EXPRESSION removes no column from any schema, so the divergence that stops
        // the pushdown at an `OPTIONAL`'s or a `MINUS`'s right arm does not arise here.
        // See `crate::enf`'s "The SHACL pre-binding fork".
        GraphPattern::Join { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right } => {
            substitute_in_graph_pattern(left, expr_subs);
            substitute_in_graph_pattern(right, expr_subs);
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            substitute_in_graph_pattern(left, expr_subs);
            substitute_in_graph_pattern(right, expr_subs);
            if let Some(expression) = expression {
                substitute_in_expression(expression, expr_subs);
            }
        }
        GraphPattern::Filter { expr, inner } => {
            substitute_in_expression(expr, expr_subs);
            substitute_in_graph_pattern(inner, expr_subs);
        }
        GraphPattern::Graph { name, inner } | GraphPattern::Service { name, inner, .. } => {
            substitute_in_named_node_pattern(name, expr_subs);
            substitute_in_graph_pattern(inner, expr_subs);
        }
        // The operand and the expression are substituted; the target bindings
        // (`Extend`'s `variable`, `Unfold`'s `element`/`companion`) are this node's
        // OWN and are carried through untouched.
        GraphPattern::Extend {
            inner, expression, ..
        }
        | GraphPattern::Unfold {
            inner, expression, ..
        } => {
            substitute_in_graph_pattern(inner, expr_subs);
            substitute_in_expression(expression, expr_subs);
        }
        GraphPattern::OrderBy { inner, expression } => {
            substitute_in_graph_pattern(inner, expr_subs);
            for order in expression.iter_mut() {
                substitute_in_order_expression(order, expr_subs);
            }
        }
        // No `Project`-boundary narrowing: a SHACL pre-binding must reach an
        // UNPROJECTED scope inside a nested sub-`SELECT`, which is divergence 1 in
        // `crate::enf`'s module doc.
        GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => substitute_in_graph_pattern(inner, expr_subs),
        // A property function's arguments are INVOCATION INPUTS, evaluated per row like
        // a function call's arguments rather than matched against the graph like a BGP
        // term — so they are substituted here, on the same rule and for the same reason
        // expression positions are. The VALUES-join rewrite alone would not reach an
        // occurrence inside a sub-`SELECT` that does not project the pre-bound variable,
        // because that inner variable is a separate scope the join cannot correlate
        // with. IRI and literal values substitute; blank-node and quoted-triple values
        // pass through to the VALUES join, exactly as in expression positions.
        GraphPattern::PropertyFunction(call) => {
            for term in call
                .subject_args
                .iter_mut()
                .chain(call.object_args.iter_mut())
            {
                substitute_in_term_pattern(term, expr_subs);
            }
        }
        GraphPattern::Group {
            inner, aggregates, ..
        } => {
            substitute_in_graph_pattern(inner, expr_subs);
            // `AggregateExpression` is rebuilt through its consuming `into_parts`, so
            // the entries are taken by value and collected back. `Vec::into_iter().
            // collect()` into the same element type reuses the buffer, so the take and
            // the collect together allocate nothing.
            let taken = std::mem::take(aggregates);
            *aggregates = taken
                .into_iter()
                .map(|(var, agg)| (var, substitute_in_aggregate(agg, expr_subs)))
                .collect();
        }
    }
}

/// Replace a pre-bound variable in a property-function argument position with its
/// constant term.
///
/// Only the IRI and literal cases substitute, matching
/// [`expression_from_term_value`]: a blank-node or quoted-triple pre-binding has no
/// constant expression form and rides the VALUES join instead. A non-variable argument
/// is already a constant and passes through unchanged.
fn substitute_in_term_pattern(term: &mut TermPattern, expr_subs: &ExprSubs<'_>) {
    let TermPattern::Variable(var) = term else {
        return;
    };
    let replacement = match expr_subs.get(var.as_str()) {
        Some(Some(Expression::NamedNode(node))) => TermPattern::NamedNode(node.clone()),
        Some(Some(Expression::Literal(literal))) => TermPattern::Literal(literal.clone()),
        _ => return,
    };
    *term = replacement;
}

/// Replace a pre-bound variable in a `GRAPH`/`SERVICE` name with its IRI constant.
fn substitute_in_named_node_pattern(pattern: &mut NamedNodePattern, expr_subs: &ExprSubs<'_>) {
    let NamedNodePattern::Variable(var) = pattern else {
        return;
    };
    let name = var.as_str();
    let replacement = if expr_subs.contains_key(name)
        && let Some(Some(Expression::NamedNode(node))) = expr_subs.get(name)
    {
        node.clone()
    } else {
        return;
    };
    *pattern = NamedNodePattern::NamedNode(replacement);
}

/// Recursively substitute pre-bound variables into an [`Expression`].
fn substitute_in_expression(expr: &mut Expression, expr_subs: &ExprSubs<'_>) {
    // Wildcard-free on purpose, for the same reason the graph-pattern walk is.
    match expr {
        Expression::Variable(var) => {
            // Resolved before the assignment so `var`'s borrow of `*expr` has ended.
            let replacement = expr_subs.get(var.as_str()).and_then(Clone::clone);
            if let Some(subst) = replacement {
                *expr = subst;
            }
        }
        Expression::Bound(var) => {
            // `contains_key`, not `get`: a variable pre-bound to a blank node or a
            // quoted triple has NO expression form and so is absent from the value
            // side, but it IS bound, and `BOUND()` must say so.
            if expr_subs.contains_key(var.as_str()) {
                *expr = true_literal();
            }
        }
        Expression::NamedNode(_) | Expression::Literal(_) => {}
        Expression::Or(left, right)
        | Expression::And(left, right)
        | Expression::Equal(left, right)
        | Expression::SameTerm(left, right)
        | Expression::Greater(left, right)
        | Expression::GreaterOrEqual(left, right)
        | Expression::Less(left, right)
        | Expression::LessOrEqual(left, right)
        | Expression::Add(left, right)
        | Expression::Subtract(left, right)
        | Expression::Multiply(left, right)
        | Expression::Divide(left, right) => {
            substitute_in_expression(left, expr_subs);
            substitute_in_expression(right, expr_subs);
        }
        Expression::UnaryPlus(inner) | Expression::UnaryMinus(inner) | Expression::Not(inner) => {
            substitute_in_expression(inner, expr_subs);
        }
        Expression::In(target, list) => {
            substitute_in_expression(target, expr_subs);
            for item in list.iter_mut() {
                substitute_in_expression(item, expr_subs);
            }
        }
        Expression::If(cond, then_expr, else_expr) => {
            substitute_in_expression(cond, expr_subs);
            substitute_in_expression(then_expr, expr_subs);
            substitute_in_expression(else_expr, expr_subs);
        }
        Expression::Coalesce(list) => {
            for item in list.iter_mut() {
                substitute_in_expression(item, expr_subs);
            }
        }
        Expression::FunctionCall(_, args) => {
            for arg in args.iter_mut() {
                substitute_in_expression(arg, expr_subs);
            }
        }
        // Back into graph-pattern territory: the two walks convert together.
        Expression::Exists(inner) => substitute_in_graph_pattern(inner, expr_subs),
    }
}

/// Substitute inside an [`OrderExpression`] sort key.
fn substitute_in_order_expression(order: &mut OrderExpression, expr_subs: &ExprSubs<'_>) {
    match order {
        OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => {
            substitute_in_expression(expr, expr_subs);
        }
    }
}

/// Substitute inside a [`GROUP BY`][`AggregateExpression`] aggregate.
fn substitute_in_aggregate(
    agg: AggregateExpression,
    expr_subs: &ExprSubs<'_>,
) -> AggregateExpression {
    let (function, mut args, scalarvals, mut order_by, distinct) = agg.into_parts();
    for arg in &mut args {
        substitute_in_expression(arg, expr_subs);
    }
    // A `FOLD`'s own sort keys are per-row expressions over the SAME solutions
    // its arguments read, so a substitution that rewrites `?x` in the argument
    // must rewrite it in the sort key too — leaving them alone would order the
    // fold by a variable the substituted query no longer binds.
    for order in &mut order_by {
        substitute_in_order_expression(order, expr_subs);
    }
    // `substitute_in_expression` rewrites each argument in place and never
    // changes the argument COUNT, and rewriting a sort key never removes one,
    // so this can never turn a valid `agg` into an invalid one.
    AggregateExpression::new(function, args, scalarvals, order_by, distinct)
        .expect("substitution preserves argument count, so arity stays valid")
}

/// The SPARQL `true` boolean literal (`"true"^^xsd:boolean`).
fn true_literal() -> Expression {
    Expression::Literal(Literal::new_typed(
        "true",
        NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#boolean"),
    ))
}

/// Convert a dataset-independent [`TermValue`] to the algebra's [`GroundTerm`].
fn ground_term_from_value(value: &TermValue) -> Result<GroundTerm, RdfDiagnostic> {
    match value {
        TermValue::Iri(iri) => Ok(GroundTerm::NamedNode(node(iri)?)),
        // The algebra's `BlankNode` has ONE string slot, so a `(label, scope)`
        // pair is carried in it the way every other single-slot blank surface
        // carries one: as the scope-qualified rendering. `ground_term_to_value`
        // decodes it back, so an injected blank focus node still denotes the
        // dataset node it was resolved from rather than a fresh, unrelated one.
        TermValue::Blank { label, scope } => Ok(GroundTerm::BlankNode(BlankNode::new(
            scope.qualify_label(label).into_owned(),
        ))),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => Ok(GroundTerm::Literal(literal_from_value(
            lexical_form,
            datatype,
            language.as_deref(),
            *direction,
        )?)),
        TermValue::Triple { s, p, o } => {
            let subject = ground_term_from_value(s)?;
            let GroundTerm::NamedNode(predicate) = ground_term_from_value(p)? else {
                return Err(RdfDiagnostic::error(
                    "native-sparql-subst-triple-predicate",
                    "a quoted-triple predicate must be an IRI".to_owned(),
                ));
            };
            let object = ground_term_from_value(o)?;
            Ok(GroundTerm::Triple(Box::new(GroundTriple {
                subject,
                predicate,
                object,
            })))
        }
    }
}

/// Build an algebra [`Literal`] from a value's components, choosing the plain /
/// typed / lang / dir-lang constructor that matches its shape.
///
/// Both component doors refuse rather than build: a datatype IRI goes through
/// [`node`], and a language tag through [`lang`]. This is the pre-binding
/// INGRESS — the one place a caller-supplied [`TermValue`] becomes algebra — and
/// a caller that hands over a malformed component has to learn that it did.
fn literal_from_value(
    lexical_form: &str,
    datatype: &str,
    language: Option<&str>,
    direction: Option<RdfTextDirection>,
) -> Result<Literal, RdfDiagnostic> {
    match (language, direction) {
        (Some(language), dir) => Ok(Literal::new_lang(
            lexical_form,
            lang(language)?,
            dir.map(|d| match d {
                RdfTextDirection::Ltr => BaseDirection::Ltr,
                RdfTextDirection::Rtl => BaseDirection::Rtl,
            }),
        )),
        (None, _) => Ok(Literal::new_typed(lexical_form, node(datatype)?)),
    }
}

/// Validate-and-wrap an IRI, surfacing a malformed IRI as a diagnostic.
fn node(iri: &str) -> Result<NamedNode, RdfDiagnostic> {
    NamedNode::new(iri).map_err(|e| RdfDiagnostic::error("native-sparql-subst-iri", e.to_string()))
}

/// Validate a pre-bound literal's language tag, surfacing an ungrammatical one
/// as a diagnostic exactly the way [`node`] surfaces a malformed datatype IRI.
///
/// The tag is judged on [`crate::scratch::LANGTAG_PROFILE`], the profile the
/// SPARQL parser holds query text to and the profile `RdfLiteral` holds a
/// dataset term to, so a pre-binding is admitted on precisely the terms a term
/// written in the query itself would have been.
///
/// # Why this is an ERROR and not an unbound binding
///
/// [`Query::substitute_variable`] injects the pre-binding as a single-row
/// `VALUES` join, and an unbound cell in a `VALUES` row is `UNDEF` — compatible
/// with every solution. So degrading a refused pre-binding to "unbound" would
/// not cost the caller a binding, it would delete the CONSTRAINT: the query
/// would return the whole unrestricted relation, more rows than the caller asked
/// for, with nothing anywhere saying why. A pre-binding is an instruction to
/// narrow, and an instruction that cannot be carried out is reported, not
/// quietly dropped.
fn lang(tag: &str) -> Result<&str, RdfDiagnostic> {
    match purrdf_iri::langtag::parse_with(tag, crate::scratch::LANGTAG_PROFILE) {
        Ok(_) => Ok(tag),
        Err(error) => Err(RdfDiagnostic::error(
            "native-sparql-subst-langtag",
            format!(
                "a pre-bound literal's language tag {tag:?} is not one this profile lexes: \
                 {error} [{code}]",
                code = error.diagnostic_code()
            ),
        )),
    }
}
