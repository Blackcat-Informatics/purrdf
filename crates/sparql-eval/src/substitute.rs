// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
}

impl<'a> Prebindings<'a> {
    /// How many variables are pre-bound.
    pub(crate) fn len(self) -> usize {
        match self {
            Self::Owned(list) => list.len(),
            Self::Borrowed(list) => list.len(),
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
    let mut probes = Vec::with_capacity(substitutions.len());
    for (name, value) in substitutions.iter() {
        probes.push((
            Variable::new(name.to_owned()),
            ground_term_from_value(value)?,
        ));
    }
    if probes.is_empty() {
        // Nothing to push and nothing to seed. [`push_probe_constants`] returns its
        // argument untouched for an empty probe list and a `map_core_pattern` whose
        // body is the identity rebuilds the query it was handed, so descending at all
        // here would be a walk with no rewrite in it.
        return Ok(query);
    }
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
    if has_repeated_variable(&probes) {
        let mut query = query.map_core_pattern(|core| push_probe_constants(core, &probes));
        for (var, ground) in probes {
            query = query.substitute_variable(&var, ground);
        }
        return Ok(query);
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
    Ok(query.map_core_pattern(move |core| {
        let core = push_probe_constants(core, &probes);
        let mut variables = Vec::with_capacity(probes.len());
        let mut row = Vec::with_capacity(probes.len());
        for (var, ground) in probes {
            variables.push(var);
            row.push(Some(ground));
        }
        GraphPattern::Join {
            left: Box::new(GraphPattern::Values {
                variables,
                bindings: vec![row],
            }),
            right: Box::new(core),
        }
    }))
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
fn push_probe_constants(core: GraphPattern, probes: &[(Variable, GroundTerm)]) -> GraphPattern {
    if probes.is_empty() {
        return core;
    }
    push_probes(core, probes, true)
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
fn push_probes(
    pattern: GraphPattern,
    probes: &[(Variable, GroundTerm)],
    at_core_root: bool,
) -> GraphPattern {
    match pattern {
        GraphPattern::Bgp { patterns } => {
            let mut probed = Vec::new();
            let patterns = patterns
                .into_iter()
                .map(|triple| probe_triple_pattern(triple, probes, &mut probed))
                .collect();
            restore_probed_bindings(
                GraphPattern::Bgp { patterns },
                &probed,
                probes,
                at_core_root,
            )
        }
        GraphPattern::Path {
            subject,
            path,
            object,
        } => {
            let mut probed = Vec::new();
            let leaf = GraphPattern::Path {
                subject: probe_term_pattern(subject, probes, &mut probed),
                path,
                object: probe_term_pattern(object, probes, &mut probed),
            };
            restore_probed_bindings(leaf, &probed, probes, at_core_root)
        }
        GraphPattern::Join { left, right } => GraphPattern::Join {
            left: Box::new(push_probes(*left, probes, false)),
            right: Box::new(push_probes(*right, probes, false)),
        },
        GraphPattern::Union { left, right } => GraphPattern::Union {
            left: Box::new(push_probes(*left, probes, false)),
            right: Box::new(push_probes(*right, probes, false)),
        },
        GraphPattern::Graph { name, inner } => GraphPattern::Graph {
            name,
            inner: Box::new(push_probes(*inner, probes, false)),
        },
        GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
            expr,
            inner: Box::new(push_probes(*inner, probes, false)),
        },
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => {
            // `variable` is this node's own binding, so it is not bound in `inner`
            // and there is nothing there to narrow. Narrowing the candidate set is
            // only ever needed for an ill-formed query the parser would reject, and
            // costs one allocation on a branch no well-formed query takes.
            let narrowed: Vec<(Variable, GroundTerm)>;
            let inner_probes = if probes.iter().any(|(var, _)| *var == variable) {
                narrowed = probes
                    .iter()
                    .filter(|(var, _)| *var != variable)
                    .cloned()
                    .collect();
                &narrowed
            } else {
                probes
            };
            GraphPattern::Extend {
                inner: Box::new(push_probes(*inner, inner_probes, false)),
                variable,
                expression,
            }
        }
        // Left operand only — see [`push_probe_constants`]'s soundness note.
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => GraphPattern::LeftJoin {
            left: Box::new(push_probes(*left, probes, false)),
            right,
            expression,
        },
        GraphPattern::Minus { left, right } => GraphPattern::Minus {
            left: Box::new(push_probes(*left, probes, false)),
            right,
        },
        GraphPattern::Lateral { left, right } => GraphPattern::Lateral {
            left: Box::new(push_probes(*left, probes, false)),
            right,
        },
        other => other,
    }
}

/// Re-bind the columns a leaf rewrite consumed, unless the seed is about to.
///
/// `probed` holds the indices into `probes` of the variables this leaf really
/// replaced, so a leaf that matched none is returned untouched and costs nothing.
fn restore_probed_bindings(
    leaf: GraphPattern,
    probed: &[usize],
    probes: &[(Variable, GroundTerm)],
    at_core_root: bool,
) -> GraphPattern {
    if probed.is_empty() || at_core_root {
        return leaf;
    }
    let variables = probed.iter().map(|&i| probes[i].0.clone()).collect();
    let row = probed.iter().map(|&i| Some(probes[i].1.clone())).collect();
    GraphPattern::Join {
        left: Box::new(leaf),
        right: Box::new(GraphPattern::Values {
            variables,
            bindings: vec![row],
        }),
    }
}

/// Push constants into a triple pattern's subject and object, recording which
/// candidates were consumed. The predicate is a [`NamedNodePattern`] and a
/// pre-binding that reached it would be pre-binding a PREDICATE variable, which
/// [`substitute_in_named_node_pattern`] already handles on the SHACL path.
fn probe_triple_pattern(
    triple: TriplePattern,
    probes: &[(Variable, GroundTerm)],
    probed: &mut Vec<usize>,
) -> TriplePattern {
    TriplePattern {
        subject: probe_term_pattern(triple.subject, probes, probed),
        predicate: triple.predicate,
        object: probe_term_pattern(triple.object, probes, probed),
    }
}

/// Replace one term position with its pre-bound constant, recursing into a quoted
/// triple's own positions. A position that is not a candidate variable is returned
/// unchanged.
fn probe_term_pattern(
    term: TermPattern,
    probes: &[(Variable, GroundTerm)],
    probed: &mut Vec<usize>,
) -> TermPattern {
    match term {
        TermPattern::Variable(var) => {
            let found = probes.iter().position(|(candidate, _)| *candidate == var);
            let Some(index) = found else {
                return TermPattern::Variable(var);
            };
            let Some(constant) = term_pattern_from_ground(&probes[index].1) else {
                return TermPattern::Variable(var);
            };
            if !probed.contains(&index) {
                probed.push(index);
            }
            constant
        }
        TermPattern::Triple(triple) => {
            TermPattern::Triple(Box::new(probe_triple_pattern(*triple, probes, probed)))
        }
        other => other,
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
    let query = apply_substitutions(query, substitutions)?;

    let mut entries = Vec::with_capacity(substitutions.len());
    for (name, value) in substitutions.iter() {
        entries.push((name, expression_from_term_value(value)?));
    }
    let expr_subs = ExprSubs(entries);

    Ok(map_patterns_in_query(query, |pattern| {
        substitute_in_graph_pattern(pattern, &expr_subs)
    }))
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

/// Convert a dataset-independent [`TermValue`] to an [`Expression`] when it is an
/// IRI or literal; return `None` for blank nodes or quoted triples, which must be
/// handled via the VALUES-join path.
fn expression_from_term_value(value: &TermValue) -> Result<Option<Expression>, RdfDiagnostic> {
    match ground_term_from_value(value)? {
        GroundTerm::NamedNode(node) => Ok(Some(Expression::NamedNode(node))),
        GroundTerm::Literal(lit) => Ok(Some(Expression::Literal(lit))),
        GroundTerm::BlankNode(_) | GroundTerm::Triple(_) => Ok(None),
    }
}

/// Walk and rebuild the whole [`Query`], applying `f` to every [`GraphPattern`]
/// contained in it (including sub-queries).
fn map_patterns_in_query(query: Query, mut f: impl FnMut(GraphPattern) -> GraphPattern) -> Query {
    match query {
        Query::Select {
            pattern,
            dataset,
            base_iri,
            version,
        } => Query::Select {
            pattern: f(pattern),
            dataset,
            base_iri,
            version,
        },
        Query::Construct {
            template,
            pattern,
            dataset,
            base_iri,
            version,
        } => Query::Construct {
            template,
            pattern: f(pattern),
            dataset,
            base_iri,
            version,
        },
        Query::Describe {
            pattern,
            targets,
            dataset,
            base_iri,
            version,
        } => Query::Describe {
            pattern: f(pattern),
            targets,
            dataset,
            base_iri,
            version,
        },
        Query::Ask {
            pattern,
            dataset,
            base_iri,
            version,
        } => Query::Ask {
            pattern: f(pattern),
            dataset,
            base_iri,
            version,
        },
    }
}

/// Recursively substitute pre-bound variables into a [`GraphPattern`].
fn substitute_in_graph_pattern(pattern: GraphPattern, expr_subs: &ExprSubs<'_>) -> GraphPattern {
    match pattern {
        GraphPattern::Bgp { patterns } => GraphPattern::Bgp { patterns },
        GraphPattern::Path {
            subject,
            path,
            object,
        } => GraphPattern::Path {
            subject,
            path,
            object,
        },
        GraphPattern::Join { left, right } => GraphPattern::Join {
            left: Box::new(substitute_in_graph_pattern(*left, expr_subs)),
            right: Box::new(substitute_in_graph_pattern(*right, expr_subs)),
        },
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => GraphPattern::LeftJoin {
            left: Box::new(substitute_in_graph_pattern(*left, expr_subs)),
            right: Box::new(substitute_in_graph_pattern(*right, expr_subs)),
            expression: expression.map(|e| substitute_in_expression(e, expr_subs)),
        },
        GraphPattern::Lateral { left, right } => GraphPattern::Lateral {
            left: Box::new(substitute_in_graph_pattern(*left, expr_subs)),
            right: Box::new(substitute_in_graph_pattern(*right, expr_subs)),
        },
        GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
            expr: substitute_in_expression(expr, expr_subs),
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
        },
        GraphPattern::Union { left, right } => GraphPattern::Union {
            left: Box::new(substitute_in_graph_pattern(*left, expr_subs)),
            right: Box::new(substitute_in_graph_pattern(*right, expr_subs)),
        },
        GraphPattern::Graph { name, inner } => GraphPattern::Graph {
            name: substitute_in_named_node_pattern(name, expr_subs),
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
        },
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => GraphPattern::Extend {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
            variable,
            expression: substitute_in_expression(expression, expr_subs),
        },
        // The operand is substituted; the two targets are this nodes OWN bindings
        // and are carried through untouched, exactly as `Extend`s target is.
        GraphPattern::Unfold {
            inner,
            expression,
            element,
            companion,
        } => GraphPattern::Unfold {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
            expression: substitute_in_expression(expression, expr_subs),
            element,
            companion,
        },
        GraphPattern::Minus { left, right } => GraphPattern::Minus {
            left: Box::new(substitute_in_graph_pattern(*left, expr_subs)),
            right: Box::new(substitute_in_graph_pattern(*right, expr_subs)),
        },
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => GraphPattern::Service {
            name: substitute_in_named_node_pattern(name, expr_subs),
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
            silent,
        },
        GraphPattern::Values {
            variables,
            bindings,
        } => GraphPattern::Values {
            variables,
            bindings,
        },
        GraphPattern::OrderBy { inner, expression } => GraphPattern::OrderBy {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
            expression: expression
                .into_iter()
                .map(|e| substitute_in_order_expression(e, expr_subs))
                .collect(),
        },
        GraphPattern::Project { inner, variables } => GraphPattern::Project {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
            variables,
        },
        GraphPattern::Distinct { inner } => GraphPattern::Distinct {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
        },
        GraphPattern::Reduced { inner } => GraphPattern::Reduced {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
        },
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => GraphPattern::Slice {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
            start,
            length,
        },
        // A property function's arguments are INVOCATION INPUTS, evaluated per row like
        // a function call's arguments rather than matched against the graph like a BGP
        // term — so they are substituted here, on the same rule and for the same reason
        // expression positions are. The VALUES-join rewrite alone would not reach an
        // occurrence inside a sub-`SELECT` that does not project the pre-bound variable,
        // because that inner variable is a separate scope the join cannot correlate
        // with. IRI and literal values substitute; blank-node and quoted-triple values
        // pass through to the VALUES join, exactly as in expression positions.
        GraphPattern::PropertyFunction(call) => {
            GraphPattern::PropertyFunction(purrdf_sparql_algebra::PropertyFunctionCall {
                iri: call.iri,
                subject_args: call
                    .subject_args
                    .into_iter()
                    .map(|term| substitute_in_term_pattern(term, expr_subs))
                    .collect(),
                object_args: call
                    .object_args
                    .into_iter()
                    .map(|term| substitute_in_term_pattern(term, expr_subs))
                    .collect(),
            })
        }
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => GraphPattern::Group {
            inner: Box::new(substitute_in_graph_pattern(*inner, expr_subs)),
            variables,
            aggregates: aggregates
                .into_iter()
                .map(|(var, agg)| (var, substitute_in_aggregate(agg, expr_subs)))
                .collect(),
        },
    }
}

/// Replace a pre-bound variable in a property-function argument position with its
/// constant term.
///
/// Only the IRI and literal cases substitute, matching
/// [`expression_from_term_value`]: a blank-node or quoted-triple pre-binding has no
/// constant expression form and rides the VALUES join instead. A non-variable argument
/// is already a constant and passes through unchanged.
fn substitute_in_term_pattern(term: TermPattern, expr_subs: &ExprSubs<'_>) -> TermPattern {
    let TermPattern::Variable(var) = &term else {
        return term;
    };
    match expr_subs.get(var.as_str()) {
        Some(Some(Expression::NamedNode(node))) => TermPattern::NamedNode(node.clone()),
        Some(Some(Expression::Literal(literal))) => TermPattern::Literal(literal.clone()),
        _ => term,
    }
}

/// Replace a pre-bound variable in a `GRAPH`/`SERVICE` name with its IRI constant.
fn substitute_in_named_node_pattern(
    pattern: NamedNodePattern,
    expr_subs: &ExprSubs<'_>,
) -> NamedNodePattern {
    match pattern {
        NamedNodePattern::Variable(var) => {
            let name = var.as_str();
            if expr_subs.contains_key(name)
                && let Some(Some(Expression::NamedNode(node))) = expr_subs.get(name)
            {
                return NamedNodePattern::NamedNode(node.clone());
            }
            NamedNodePattern::Variable(var)
        }
        named @ NamedNodePattern::NamedNode(_) => named,
    }
}

/// Recursively substitute pre-bound variables into an [`Expression`].
fn substitute_in_expression(expr: Expression, expr_subs: &ExprSubs<'_>) -> Expression {
    match expr {
        Expression::Variable(var) => {
            let name = var.as_str();
            if let Some(Some(subst)) = expr_subs.get(name) {
                subst.clone()
            } else {
                Expression::Variable(var)
            }
        }
        Expression::Bound(var) => {
            let name = var.as_str();
            if expr_subs.contains_key(name) {
                true_literal()
            } else {
                Expression::Bound(var)
            }
        }
        Expression::NamedNode(node) => Expression::NamedNode(node),
        Expression::Literal(lit) => Expression::Literal(lit),
        Expression::Or(left, right) => Expression::Or(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::And(left, right) => Expression::And(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Equal(left, right) => Expression::Equal(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::SameTerm(left, right) => Expression::SameTerm(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Greater(left, right) => Expression::Greater(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::GreaterOrEqual(left, right) => Expression::GreaterOrEqual(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Less(left, right) => Expression::Less(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::LessOrEqual(left, right) => Expression::LessOrEqual(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Add(left, right) => Expression::Add(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Subtract(left, right) => Expression::Subtract(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Multiply(left, right) => Expression::Multiply(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::Divide(left, right) => Expression::Divide(
            Box::new(substitute_in_expression(*left, expr_subs)),
            Box::new(substitute_in_expression(*right, expr_subs)),
        ),
        Expression::UnaryPlus(inner) => {
            Expression::UnaryPlus(Box::new(substitute_in_expression(*inner, expr_subs)))
        }
        Expression::UnaryMinus(inner) => {
            Expression::UnaryMinus(Box::new(substitute_in_expression(*inner, expr_subs)))
        }
        Expression::Not(inner) => {
            Expression::Not(Box::new(substitute_in_expression(*inner, expr_subs)))
        }
        Expression::In(target, list) => Expression::In(
            Box::new(substitute_in_expression(*target, expr_subs)),
            list.into_iter()
                .map(|e| substitute_in_expression(e, expr_subs))
                .collect(),
        ),
        Expression::If(cond, then_expr, else_expr) => Expression::If(
            Box::new(substitute_in_expression(*cond, expr_subs)),
            Box::new(substitute_in_expression(*then_expr, expr_subs)),
            Box::new(substitute_in_expression(*else_expr, expr_subs)),
        ),
        Expression::Coalesce(list) => Expression::Coalesce(
            list.into_iter()
                .map(|e| substitute_in_expression(e, expr_subs))
                .collect(),
        ),
        Expression::FunctionCall(function, args) => Expression::FunctionCall(
            function,
            args.into_iter()
                .map(|e| substitute_in_expression(e, expr_subs))
                .collect(),
        ),
        Expression::Exists(inner) => {
            Expression::Exists(Box::new(substitute_in_graph_pattern(*inner, expr_subs)))
        }
    }
}

/// Substitute inside an [`OrderExpression`] sort key.
fn substitute_in_order_expression(
    order: OrderExpression,
    expr_subs: &ExprSubs<'_>,
) -> OrderExpression {
    match order {
        OrderExpression::Asc(expr) => {
            OrderExpression::Asc(substitute_in_expression(expr, expr_subs))
        }
        OrderExpression::Desc(expr) => {
            OrderExpression::Desc(substitute_in_expression(expr, expr_subs))
        }
    }
}

/// Substitute inside a [`GROUP BY`][`AggregateExpression`] aggregate.
fn substitute_in_aggregate(
    agg: AggregateExpression,
    expr_subs: &ExprSubs<'_>,
) -> AggregateExpression {
    let (function, args, scalarvals, order_by, distinct) = agg.into_parts();
    let args = args
        .into_iter()
        .map(|e| substitute_in_expression(e, expr_subs))
        .collect();
    // A `FOLD`'s own sort keys are per-row expressions over the SAME solutions
    // its arguments read, so a substitution that rewrites `?x` in the argument
    // must rewrite it in the sort key too — leaving them alone would order the
    // fold by a variable the substituted query no longer binds.
    let order_by = order_by
        .into_iter()
        .map(|order| substitute_in_order_expression(order, expr_subs))
        .collect();
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
