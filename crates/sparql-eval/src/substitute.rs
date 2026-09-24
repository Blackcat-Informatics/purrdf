// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Engine-side variable **pre-binding** (purrdf S6).
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

use purrdf_core::{DatasetView, RdfDiagnostic, RdfTextDirection, TermRef, TermValue};
use purrdf_sparql_algebra::{
    AggregateExpression, BaseDirection, BlankNode, Expression, GraphPattern, GroundTerm,
    GroundTriple, Literal, NamedNode, NamedNodePattern, OrderExpression, PropertyFunctionCall,
    Query, TermPattern, TriplePattern, Variable,
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
    Paired(&'a [Variable], &'a [Option<ParameterValue>]),
}

/// What one parameter slot of a prepared execution currently holds.
///
/// Two variants because a parameter arrives through two doors, and the difference
/// between them is exactly one round trip.
///
/// The value door — [`PreparedExecution::bind`](crate::PreparedExecution::bind) —
/// takes a caller-owned [`TermValue`], which is what a caller holding a term and no
/// dataset has. The id door —
/// [`PreparedExecution::bind_id`](crate::PreparedExecution::bind_id) — takes the
/// dataset's OWN id for a term it already interns, and [`ground_term_from_id`]
/// resolves it straight into the algebra term the rewrite wants. Without that door a
/// caller holding an id had to spell the term out as a `TermValue` (an owned `String`
/// per component) purely so [`ground_term_from_value`] could allocate it a SECOND
/// time into the algebra, and so `crate::bgp`'s `compile_term` could then hash it
/// back to the very id the caller started from.
///
/// # Why the id is consumed at the door and never stored here
///
/// A dataset-local id means nothing without the view that minted it, and a handle
/// outlives any one run: the SHACL handle cache hands one worker's handle to
/// validators over DIFFERENT datasets. So a slot holding an id would be holding a
/// number whose meaning depends on a dataset the slot does not name — an id from one
/// view used against another is in range, resolves, and denotes the wrong term, with
/// nothing anywhere saying so.
///
/// This variant therefore holds the RESOLVED algebra term, not the id. The id exists
/// only inside [`PreparedExecution::bind_id`](crate::PreparedExecution::bind_id),
/// where the dataset that is to interpret it is an argument of the same call, so
/// there is no window in which an id could meet a different dataset. See that
/// function for the full statement of what that does and does not rule out.
#[derive(Clone, Debug)]
pub(crate) enum ParameterValue {
    /// A caller-owned term, carrying no dataset-local identity.
    Value(TermValue),
    /// The algebra term a dataset's own entry resolved to, already grounded.
    Ground(GroundTerm),
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

    /// The `index`-th pre-binding's NAME.
    ///
    /// Indexed rather than delegating to the three underlying iterators, because the
    /// three shapes have different iterator TYPES and one function cannot return
    /// them all: the alternatives are a boxed trait object, which allocates on a path
    /// that exists to stop allocating, or chaining `Option`s of which exactly one is
    /// ever non-empty, which reads like a mistake. `Self` is `Copy`, so the match
    /// below folds to a single branch.
    ///
    /// # Panics
    ///
    /// If `index` is out of range, exactly as indexing the underlying slice would.
    pub(crate) fn name(self, index: usize) -> &'a str {
        match self {
            Self::Owned(list) => list[index].0.as_str(),
            Self::Borrowed(list) => list[index].variable,
            Self::Paired(names, _) => names[index].as_str(),
        }
    }

    /// The `index`-th pre-binding's value, as the algebra term the rewrite consumes.
    ///
    /// The two doors converge HERE and nowhere earlier. A value-door binding is
    /// grounded now, as it always was; an id-door binding was grounded at the door,
    /// against the dataset that was an argument of the same call, and is cloned — a
    /// refcount bump for an IRI or a literal — rather than spelled out as a
    /// `TermValue` and grounded a second time.
    ///
    /// # Errors
    ///
    /// As [`ground_term_from_value`], and only for a value-door binding: an id-door
    /// binding did its refusing at the door.
    ///
    /// # Panics
    ///
    /// If `index` is out of range, or if a prepared execution's slot is still
    /// unbound — which
    /// [`NativeSparqlEngine::execute`](crate::NativeSparqlEngine::execute) refuses
    /// before it reaches this.
    fn ground(self, index: usize) -> Result<GroundTerm, RdfDiagnostic> {
        match self {
            Self::Owned(list) => ground_term_from_value(&list[index].1),
            Self::Borrowed(list) => ground_term_from_value(&list[index].value),
            Self::Paired(_, values) => match values[index]
                .as_ref()
                .expect("a prepared execution refuses to run with a parameter unbound")
            {
                ParameterValue::Value(value) => ground_term_from_value(value),
                ParameterValue::Ground(ground) => Ok(ground.clone()),
            },
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
            _ => interned_variable(self.name(index)),
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
    static INTERNED_VARIABLES: std::cell::RefCell<(
        crate::DetHashMap<Box<str>, Variable>,
        crate::plan_memory::InternerCharge,
    )> = std::cell::RefCell::new((
        crate::DetHashMap::default(),
        crate::plan_memory::InternerCharge::new(),
    ));
}

/// The [`Variable`] for `name`, interned per worker.
///
/// Every insert charges its estimated retained size — the key bytes plus
/// one `Variable`'s own payload — to [`crate::plan_memory::interner_memory_observer`],
/// and a cap-triggered clear credits the whole table back, so this per-worker
/// table is no longer memory a deployment's `CacheLimits` cannot see.
pub(crate) fn interned_variable(name: &str) -> Variable {
    INTERNED_VARIABLES.with(|cache| {
        let mut cache = cache.borrow_mut();
        let (entries, charge) = &mut *cache;
        if let Some(var) = entries.get(name) {
            return var.clone();
        }
        if entries.len() >= INTERNED_VARIABLE_CAP {
            entries.clear();
            charge.clear();
        }
        let var = Variable::new(name);
        let bytes = size_of::<Box<str>>()
            .saturating_add(name.len())
            .saturating_add(size_of::<Variable>());
        entries.insert(Box::from(name), var.clone());
        charge.add(bytes);
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
    build_probes_into(&mut probes, substitutions)?;
    Ok(probes)
}

/// [`build_probes`], into a buffer the caller keeps.
///
/// A [`PreparedExecution`](crate::PreparedExecution) runs the same query over and
/// over, so the probe list has the same LENGTH every time and only its cells change.
/// Filling a retained buffer therefore costs no allocation at all after the first
/// run, where `build_probes`' fresh `Vec` charged one per run — on a path whose whole
/// purpose is to stop allocating per run.
///
/// # Errors
///
/// As [`build_probes`]: a datatype IRI that is not a valid IRI, or a language tag the
/// concrete syntaxes would not have lexed. The buffer is cleared before the first
/// value is grounded, so a refused pre-binding leaves no earlier run's terms behind
/// for a caller that ignores the error to read.
pub(crate) fn build_probes_into(
    probes: &mut Vec<(Variable, GroundTerm)>,
    substitutions: Prebindings<'_>,
) -> Result<(), RdfDiagnostic> {
    probes.clear();
    for index in 0..substitutions.len() {
        let ground = substitutions.ground(index)?;
        probes.push((substitutions.variable(index), ground));
    }
    Ok(())
}

/// [`apply_substitutions`]'s rewrite, over probes that are already grounded and
/// already known to be non-empty.
pub(crate) fn apply_probes(query: Query, probes: Vec<(Variable, GroundTerm)>) -> Query {
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
pub(crate) fn has_repeated_variable(probes: &[(Variable, GroundTerm)]) -> bool {
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
/// * a property-function call — a leaf like a `Bgp`, written into and restored the
///   same way, so the relation is invoked with the pre-bound position bound;
/// * `Lateral` — the left operand, and the right one only when it is a
///   property-function call, which is the shape the parser gives every call that
///   follows another atom;
/// * `LeftJoin`, `Minus` — the LEFT operand only. Restricting the right
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
/// nodes keep the `VALUES`-join path, which interns the blank as the term it is. A
/// property-function call that names one as an argument is driven by a one-row
/// `VALUES` of its own instead, so the relation is still invoked with that argument
/// bound — see [`drive_call_arguments`].
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
        // A relation call is a leaf like the two above, and it is the leaf where
        // the difference between an index probe and a scan is largest. Left alone, the
        // call keeps the variable, the relation is invoked with that position
        // FREE, and it generates every row it holds for the single-row `VALUES`
        // seed to discard all but one of — which for a ranked producer is its
        // whole index, once per pre-bound candidate. With the constant pushed in,
        // the relation is invoked with the position BOUND and answers the one
        // question that was asked.
        //
        // The access pattern can only get more bound, so nothing a prepare
        // admitted becomes infeasible here: a declared mode subsumes an
        // invocation when its bound positions are a SUBSET of the invocation's,
        // and this rewrite only adds to that set.
        GraphPattern::PropertyFunction(call) => {
            let mut probed = Vec::new();
            for argument in call
                .subject_args
                .iter_mut()
                .chain(call.object_args.iter_mut())
            {
                probe_term_pattern(argument, probes, &mut probed);
            }
            // A value the pattern cannot carry — a blank node — is driven into the
            // call instead; see [`drive_call_arguments`].
            drive_call_arguments(pattern, probes, Unwritable::InPattern);
            restore_probed_bindings(pattern, &probed, probes, at_core_root);
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
        GraphPattern::LeftJoin { left, .. } | GraphPattern::Minus { left, .. } => {
            push_probes(left, probes, false);
        }
        // A `Lateral` whose right operand is a property-function call is how the
        // parser writes every call that follows another atom in its group, and the
        // call is the one right operand the rewrite may enter. It is driven once per
        // left row and inner-joined with it, so restricting it restricts the node
        // exactly as restricting either operand of a `Join` does: a left row that
        // binds the variable to some other term matched a call row carrying that
        // term before the rewrite and is dropped by the seed join; after it, the
        // call is invoked with the pre-bound constant, and the restoring `VALUES`
        // row is incompatible with that left row, so it is dropped there instead.
        //
        // The restoring `VALUES` wraps the whole `Lateral`, never the call inside
        // it. The evaluator drives a call that is a `Lateral`'s DIRECT right operand
        // with each left row in hand, which is what hands it a literal, blank-node
        // or quoted-triple binding from the left as a bound argument; wrapping the
        // call in a join would demote it to the generic correlated path, where those
        // bindings arrive free. Any other right operand keeps its correlated
        // re-evaluation untouched.
        //
        // A value the pattern cannot carry — a blank node — reaches the call through
        // that same per-row drive: the left operand is joined with a one-row `VALUES`
        // binding it, so every left row hands the call its term as a bound argument.
        // See [`drive_call_arguments`].
        GraphPattern::Lateral { left, right } => {
            push_probes(left, probes, false);
            let mut probed = Vec::new();
            if let Some(call) = lateral_call_mut(right) {
                for argument in call
                    .subject_args
                    .iter_mut()
                    .chain(call.object_args.iter_mut())
                {
                    probe_term_pattern(argument, probes, &mut probed);
                }
            }
            drive_call_arguments(pattern, probes, Unwritable::InPattern);
            restore_probed_bindings(pattern, &probed, probes, at_core_root);
        }
        _ => {}
    }
}

/// The property-function call a `Lateral`'s right operand IS, when it is one — the
/// one right operand of a `Lateral` the pushdown writes a pre-bound value into.
///
/// This is the single definition of that position. [`push_probes`] writes into it,
/// [`drive_call_arguments`] and the SHACL walk drive into it, the evaluator drives a
/// call there once per left row with that row in hand, and the prepare-time planner
/// (`crate::property_fn_plan`'s `collect_chain`) admits a call under the pushdown's
/// promise exactly when the plan it produces puts the call here. Any other right
/// operand — a group holding a call beside something else, an `OPTIONAL`, a
/// sub-`SELECT` — is one the pushdown does not enter, and the planner admits a call
/// inside it as though the pre-bound variable were free.
pub(crate) const fn lateral_call(right: &GraphPattern) -> Option<&PropertyFunctionCall> {
    match right {
        GraphPattern::PropertyFunction(call) => Some(call),
        _ => None,
    }
}

/// [`lateral_call`], for the rewrite that writes into the call.
const fn lateral_call_mut(right: &mut GraphPattern) -> Option<&mut PropertyFunctionCall> {
    match right {
        GraphPattern::PropertyFunction(call) => Some(call),
        _ => None,
    }
}

/// Which pre-bound values a rewrite cannot WRITE into a property-function argument
/// and must therefore DRIVE into it — see [`drive_call_arguments`].
#[derive(Clone, Copy, Debug)]
enum Unwritable {
    /// The pushdown's rule: every value [`term_pattern_from_ground`] refuses — a blank
    /// node, or a quoted triple with one inside it.
    InPattern,
    /// The SHACL walk's rule, which is [`substitute_in_term_pattern`]'s: every value
    /// with no expression form — a blank node or any quoted triple.
    InArgument,
}

impl Unwritable {
    /// Whether `ground` is a value this rule cannot write, and so must drive.
    fn holds(self, ground: &GroundTerm) -> bool {
        match self {
            Self::InPattern => term_pattern_from_ground(ground).is_none(),
            Self::InArgument => match Pushability::of(ground) {
                Pushability::Iri(_) | Pushability::Literal(_) => false,
                Pushability::QuotedTriple(_) | Pushability::SeedOnly => true,
            },
        }
    }
}

/// The indices into `probes` of the pre-bound variables `call` names in an argument
/// that `rule` cannot write there, less any `already` binds.
///
/// Such a value has no spelling the rewrite may write — a blank node in a query is an
/// anonymous variable — so the variable stays in place, and on its own the call would
/// then be invoked with that position FREE. A variable nested inside a quoted-triple
/// argument counts too: the drive binds the variable, wherever the argument mentions
/// it.
fn driven_arguments(
    call: &PropertyFunctionCall,
    probes: &[(Variable, GroundTerm)],
    rule: Unwritable,
    already: Option<&GraphPattern>,
) -> Vec<usize> {
    fn visit(
        term: &TermPattern,
        probes: &[(Variable, GroundTerm)],
        rule: Unwritable,
        already: Option<&GraphPattern>,
        driven: &mut Vec<usize>,
    ) {
        match term {
            TermPattern::Variable(var) => {
                if let Some(index) = probes.iter().position(|(candidate, _)| candidate == var)
                    && rule.holds(&probes[index].1)
                    && !driven.contains(&index)
                    && !already.is_some_and(|left| drives(left, var))
                {
                    driven.push(index);
                }
            }
            TermPattern::Triple(triple) => {
                visit(&triple.subject, probes, rule, already, driven);
                visit(&triple.object, probes, rule, already, driven);
            }
            TermPattern::NamedNode(_) | TermPattern::BlankNode(_) | TermPattern::Literal(_) => {}
        }
    }
    let mut driven = Vec::new();
    for argument in call.subject_args.iter().chain(call.object_args.iter()) {
        visit(argument, probes, rule, already, &mut driven);
    }
    driven
}

/// Whether `left` — a `Lateral`'s left operand — is already a driver for `var`: the
/// one-row `VALUES` [`drive_call_arguments`] builds, alone or joined onto the operand.
///
/// This is what makes the drive idempotent. The SHACL rewrite runs the pushdown and
/// then its own walk, and both drive calls; a call the pushdown already drove is
/// recognized here rather than wrapped twice.
fn drives(left: &GraphPattern, var: &Variable) -> bool {
    let driver = match left {
        GraphPattern::Join { left, .. } => &**left,
        other => other,
    };
    matches!(
        driver,
        GraphPattern::Values { variables, bindings }
            if bindings.len() == 1 && variables.contains(var)
    )
}

/// A one-row `VALUES` binding the probes at `driven`.
fn seed_row(driven: &[usize], probes: &[(Variable, GroundTerm)]) -> GraphPattern {
    GraphPattern::Values {
        variables: driven.iter().map(|&i| probes[i].0.clone()).collect(),
        bindings: vec![driven.iter().map(|&i| Some(probes[i].1.clone())).collect()],
    }
}

/// Drive every pre-bound value `rule` cannot write into the property-function call at
/// `pattern` — a stand-alone call, or a `Lateral` whose right operand is one — so the
/// relation is invoked with that argument BOUND.
///
/// A call's arguments are invocation inputs, and a relation that serves only the bound
/// mode refuses a free one. A prepared execution admitted the call on the promise that
/// its parameter is bound there (`crate::property_fn_plan`'s `Promise`), so leaving
/// the position free would turn that admission into a refusal on every run bound to,
/// say, a blank node.
///
/// The distinction the rewrite draws is kept exactly. A blank node written into the
/// query text is a non-distinguished variable, and is never written there by this
/// rewrite. A blank node BOUND as a value is a term identity — the dataset's own blank
/// node — and the evaluator already has a door that carries it as one: a call that is
/// the DIRECT right operand of a `Lateral` is driven per left row with that row in
/// hand, and a blank node in the row is handed to the relation as a bound argument.
/// The `VALUES` row interns the value as the term it is, which is how the seed itself
/// already binds a blank node. So:
///
/// * a stand-alone call becomes the right operand of a `Lateral` over a one-row
///   `VALUES` binding the driven values;
/// * a call that is already a `Lateral`'s right operand keeps that position — wrapping
///   it would demote it to the generic correlated path, where such bindings arrive
///   free — and the one-row `VALUES` is joined onto its left operand instead, so every
///   left row carries the values in.
///
/// Sound for the same reason the pushdown is: joining the one-row `var = b` onto the
/// call keeps exactly the call's rows that bind `var` to `b`, and a call driven with
/// `var = b` produces exactly those rows. In a scope the seed does not reach — the
/// SHACL walk's `OPTIONAL` arms and unprojected sub-`SELECT`s — it binds `var` to `b`
/// in the call's own group, which is what writing the value there would have done had
/// it a spelling. The variable is still in the call, so its column survives and needs
/// no restoring `VALUES`. An `EXISTS` body is the exception: the row it filters may
/// already bind `var`, so the SHACL walk drives a call there inside a projection that
/// does not carry `var` out — see [`plant_scoped_driver`].
///
/// Idempotent: a value the left operand already drives is not driven again (see
/// [`drives`]), so the SHACL walk passing over a call the pushdown already drove
/// leaves it as it is.
///
/// Which values are driven, and the one-row `VALUES` that drives them, are
/// [`call_driver`]'s decision; this function only plants that driver in place. The
/// correlated per-row walk (`crate::expr`'s `substitute_pattern_impl`) plants the SAME
/// driver, decided by the same function through [`bind_call_arguments`], in the same
/// two positions — it builds the nodes itself only because each one it builds must
/// also be mapped back to the plan node it stands for.
fn drive_call_arguments(
    pattern: &mut GraphPattern,
    probes: &[(Variable, GroundTerm)],
    rule: Unwritable,
) {
    match pattern {
        GraphPattern::PropertyFunction(call) => {
            if let Some(seed) = call_driver(call, probes, rule, None) {
                plant_stand_alone_driver(pattern, seed);
            }
        }
        GraphPattern::Lateral { left, right } => {
            let Some(call) = lateral_call(right) else {
                return;
            };
            if let Some(seed) = call_driver(call, probes, rule, Some(left)) {
                plant_left_driver(left, seed);
            }
        }
        _ => {}
    }
}

/// The one-row `VALUES` that must drive `call` — every value of `values` that `rule`
/// cannot write into an argument `call` names, less any `already` (the left operand
/// of the `Lateral` `call` is the right operand of) drives — or `None` when there is
/// nothing to drive. See [`drive_call_arguments`].
fn call_driver(
    call: &PropertyFunctionCall,
    values: &[(Variable, GroundTerm)],
    rule: Unwritable,
    already: Option<&GraphPattern>,
) -> Option<GraphPattern> {
    let driven = driven_arguments(call, values, rule, already);
    (!driven.is_empty()).then(|| seed_row(&driven, values))
}

/// A stand-alone call, driven: `Lateral(seed, call)`.
fn plant_stand_alone_driver(call: &mut GraphPattern, seed: GraphPattern) {
    purrdf_sparql_algebra::substitute::take_and_replace(call, |call| GraphPattern::Lateral {
        left: Box::new(seed),
        right: Box::new(call),
    });
}

/// A `Lateral`'s left operand, carrying the driver of the call on its right:
/// `Join(seed, left)` — the shape [`drives`] recognizes.
fn plant_left_driver(left: &mut GraphPattern, seed: GraphPattern) {
    purrdf_sparql_algebra::substitute::take_and_replace(left, |left| GraphPattern::Join {
        left: Box::new(seed),
        right: Box::new(left),
    });
}

/// Where the SHACL walk ([`substitute_in_graph_pattern`]) is: above the `VALUES` seed,
/// beneath it, or inside an `EXISTS` body — which decides where a call's driver may be
/// planted, and whether an expression that reads a value it cannot spell needs one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WalkScope {
    /// On the solution-modifier descent to the core `WHERE` pattern — the wrappers
    /// `Query::map_core_pattern` passes through, which the `VALUES` seed is joined
    /// beneath. Every row an expression here reads carries every pre-bound value, so
    /// nothing needs driving. A call is never reached here: the first node that is not
    /// such a wrapper is the seed's own `Join`, and everything beneath it is
    /// [`Self::Group`].
    Descent,
    /// Beneath the seed and outside every `EXISTS` body: a call's driver binds the
    /// driven variable where the call is, as the `VALUES` seed binds it.
    Group,
    /// Inside an `EXISTS` or `NOT EXISTS` body, at any depth: the driver is planted in
    /// a scope of its own — see [`plant_scoped_driver`].
    ExistsBody,
}

impl WalkScope {
    /// The scope of a child of a node that is not a solution-modifier wrapper: once
    /// the descent reaches such a node it is the core, and the seed sits above
    /// everything beneath it.
    const fn beneath(self) -> Self {
        match self {
            Self::Descent => Self::Group,
            other => other,
        }
    }
}

/// A call inside an `EXISTS` body, driven in a scope of its own:
/// `Project(kept, Lateral(seed, call))`, where `kept` is every variable the call names
/// except the ones `seed` drives ([`undriven_variables`]).
///
/// # Why the driver cannot bind the driven variable here
///
/// An `EXISTS` body is evaluated against the row being filtered, and that row usually
/// already binds the pre-bound variable: the `VALUES` seed joined onto the core
/// pattern is below the `FILTER`. A `VALUES` inside the body that binds the same
/// variable is a REBINDING of it, which SPARQL's substitution semantics define no
/// answer for, and the evaluator refuses it rather than guess
/// (`crate::governor::soundness::exists_row_collision`). A stand-alone driver there
/// turned every blank-node or quoted-triple focus node into that refusal, while an IRI
/// focus node — written into the argument as a constant — validated.
///
/// # Why a projection is the sound place for it
///
/// A projection is a scope boundary for both the collision check and correlated
/// substitution: a variable it does not carry out is a different variable inside it.
/// So the call is still invoked with the driven argument BOUND to the pre-bound term,
/// and its output no longer carries that variable — which is exactly the output of the
/// same call with an IRI written into the argument, the answer the walk already gives
/// for an IRI focus node. Every other variable the call names, including a predicate
/// variable inside a quoted-triple argument, is carried out unchanged, so what the
/// call binds for the rest of the body, and what it correlates with, is unchanged.
fn plant_scoped_driver(call: &mut GraphPattern, seed: GraphPattern, kept: Vec<Variable>) {
    purrdf_sparql_algebra::substitute::take_and_replace(call, |call| GraphPattern::Project {
        inner: Box::new(GraphPattern::Lateral {
            left: Box::new(seed),
            right: Box::new(call),
        }),
        variables: kept,
    });
}

/// Every variable `call` names in an argument — a predicate variable inside a
/// quoted-triple argument included — that `seed` does not drive, in first-mention
/// order: what [`plant_scoped_driver`]'s projection carries out.
///
/// A `seed` that is not the one-row `VALUES` [`seed_row`] builds drives nothing, so
/// every variable is kept: the collision check then refuses the rebinding loudly,
/// rather than a variable the call binds being hidden from the rest of the body.
fn undriven_variables(call: &PropertyFunctionCall, seed: &GraphPattern) -> Vec<Variable> {
    fn keep(var: &Variable, driven: &[Variable], kept: &mut Vec<Variable>) {
        if !driven.contains(var) && !kept.contains(var) {
            kept.push(var.clone());
        }
    }
    fn visit(term: &TermPattern, driven: &[Variable], kept: &mut Vec<Variable>) {
        match term {
            TermPattern::Variable(var) => keep(var, driven, kept),
            TermPattern::Triple(triple) => {
                visit(&triple.subject, driven, kept);
                if let NamedNodePattern::Variable(var) = &triple.predicate {
                    keep(var, driven, kept);
                }
                visit(&triple.object, driven, kept);
            }
            TermPattern::NamedNode(_) | TermPattern::BlankNode(_) | TermPattern::Literal(_) => {}
        }
    }
    let driven: &[Variable] = match seed {
        GraphPattern::Values { variables, .. } => variables,
        _ => &[],
    };
    let mut kept = Vec::new();
    for argument in call.subject_args.iter().chain(call.object_args.iter()) {
        visit(argument, driven, &mut kept);
    }
    kept
}

/// Write a row's values into a property-function call's arguments, and return the
/// one-row `VALUES` that must drive the ones that cannot be written — the single
/// definition of "put this value into that call" both whole-query rewrites and the
/// per-row correlated walk share.
///
/// An IRI or a literal is written into the argument as the constant it is, so the
/// relation reads it exactly as if the query had spelled it
/// ([`substitute_in_term_pattern`]). A blank node and a quoted triple are not: a blank
/// node in a query is an anonymous variable, and a quoted triple has no argument
/// spelling this rule admits (the pre-binding walk shares the EXPRESSION rule,
/// [`Unwritable::InArgument`]). Those are DRIVEN instead — the returned `VALUES` row
/// carries them into the call as the terms they are (see [`drive_call_arguments`]),
/// placed by the caller as the left operand of a `Lateral` over the call when `call`
/// stands alone, or joined onto `already` — the left operand of the `Lateral` whose
/// right operand `call` is — when it does not.
///
/// A value no argument names is neither written nor driven, so a row carrying
/// unrelated bindings leaves `call` as it was and returns `None`.
pub(crate) fn bind_call_arguments(
    call: &mut PropertyFunctionCall,
    values: &[(Variable, GroundTerm)],
    already: Option<&GraphPattern>,
) -> Option<GraphPattern> {
    for term in call
        .subject_args
        .iter_mut()
        .chain(call.object_args.iter_mut())
    {
        substitute_in_term_pattern(term, values);
    }
    call_driver(call, values, Unwritable::InArgument, already)
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

/// What a grounded pre-binding may be USED for — decided once, in one place.
///
/// # The defect this closes
///
/// The rewrite consumes a pre-bound value in four positions, and each of them used
/// to re-decide this question over its OWN subset of [`GroundTerm`]:
/// [`term_pattern_from_ground`] recursed through a quoted triple and refused a blank;
/// [`expression_from_ground`] refused a blank AND a quoted triple;
/// [`substitute_in_term_pattern`] admitted an IRI and a literal and fell through on
/// everything else; [`named_node_from_ground`] — the `GRAPH`/`SERVICE` name — admitted
/// an IRI alone. Three of the four spelled the refusal as a catch-all, so a
/// [`GroundTerm`] variant added later COMPILED in three of them and was silently
/// handled by whatever the catch-all beside it happened to say. That is the
/// silent-drop shape and the over-refusal shape at once: a value admitted into a
/// position nobody decided it belonged in, or refused from one it did.
///
/// [`Pushability::of`] is now the only place the classification is made, over a
/// match that is exhaustive over [`GroundTerm`] **with no wildcard arm**. A new
/// `GroundTerm` variant fails to compile there; a new CLASS of value — a new variant
/// of this enum — fails to compile at every site that consumes one. The four cannot
/// drift apart again, because there is no longer a place for them to drift from.
///
/// # Why each variant lands where it does
///
/// * [`Self::Iri`] — every position. An IRI denotes the same node in a matched
///   triple-pattern position, in an expression, and as a graph name, so it is the one
///   class with no position it is refused from.
/// * [`Self::Literal`] — a matched pattern position and an expression, but never a
///   `GRAPH`/`SERVICE` name: that position names a GRAPH, and a graph is named by an
///   IRI. Substituting a literal there would not narrow the query, it would produce
///   algebra the grammar has no spelling for.
/// * [`Self::QuotedTriple`] — a matched pattern position ONLY, and then only
///   component by component: an RDF 1.2 quoted triple has no constant
///   [`Expression`] form to become (the algebra's expression constants are an IRI and
///   a literal, and nothing else), so in every evaluated position it rides the
///   `VALUES` seed instead. Its pattern form is built recursively, which is why a
///   nested [`Self::SeedOnly`] anywhere inside it takes the WHOLE triple out of the
///   pushdown rather than only that one position.
/// * [`Self::SeedOnly`] — a blank node, and this is the load-bearing rule of the
///   whole classification. **A blank node is bound only through `VALUES` rows and is
///   never written into a pattern.** A blank node written into a query pattern is a
///   NON-DISTINGUISHED VARIABLE (SPARQL 1.2 §4.1.4), not a request to match one
///   particular dataset blank — so pushing one would WIDEN the match to every term in
///   that position where every other class narrows it, turning a pre-binding into its
///   own opposite. The consequence is that the pushed-constant set and the seed set
///   are genuinely DIFFERENT sets: the seed carries every pre-binding, the pushdown
///   carries only those the pattern can narrow on. A property-function call that names
///   a blank-node pre-binding as an argument is the one leaf that still needs it
///   BOUND, and it gets it through a `VALUES` row too — the one-row driver
///   [`drive_call_arguments`] puts on the call's left, which hands the relation
///   the dataset's blank node as the term it is. [`probe_term_pattern`] records no
///   probed index for a blank, so no restoring `VALUES` is emitted for a column no
///   leaf ever consumed.
///
/// # Why it borrows
///
/// The payload is the classified term itself, borrowed, so a site that has decided
/// a value is admissible does not then re-destructure the [`GroundTerm`] to get at
/// it — which would be the same decision made twice, in the same function, with
/// nothing forcing the two spellings to agree.
#[derive(Clone, Copy, Debug)]
enum Pushability<'a> {
    /// An IRI: admitted everywhere.
    Iri(&'a NamedNode),
    /// A literal: a matched pattern position and an expression, never a graph name.
    Literal(&'a Literal),
    /// A quoted triple: a matched pattern position, component-wise, and nothing else.
    QuotedTriple(&'a GroundTriple),
    /// A blank node: bound through `VALUES` rows — the seed, and a property-function
    /// call's driver — and never written into a pattern.
    SeedOnly,
}

impl<'a> Pushability<'a> {
    /// Classify one grounded pre-binding.
    ///
    /// Exhaustive over [`GroundTerm`] and deliberately wildcard-free: a variant added
    /// to that enum must be decided about HERE, and the compiler is what says so.
    fn of(ground: &'a GroundTerm) -> Self {
        match ground {
            GroundTerm::NamedNode(node) => Self::Iri(node),
            GroundTerm::Literal(literal) => Self::Literal(literal),
            GroundTerm::Triple(triple) => Self::QuotedTriple(triple.as_ref()),
            GroundTerm::BlankNode(_) => Self::SeedOnly,
        }
    }
}

/// The triple-pattern spelling of a ground pre-binding, or `None` for a value
/// [`Pushability`] keeps out of matched pattern positions.
///
/// [`Pushability::SeedOnly`] is that value — a blank in a pattern is an anonymous
/// variable, so it would match everything rather than the pre-bound blank. See
/// [`push_probe_constants`], "What is not pushed", and [`Pushability`]'s own note on
/// why the pushed set and the seed set differ. A [`Pushability::QuotedTriple`] is
/// admitted here and nowhere else, and only if every one of its own positions is
/// admitted too — which is what the `?`s below propagate.
pub(crate) fn term_pattern_from_ground(ground: &GroundTerm) -> Option<TermPattern> {
    match Pushability::of(ground) {
        Pushability::Iri(node) => Some(TermPattern::NamedNode(node.clone())),
        Pushability::Literal(literal) => Some(TermPattern::Literal(literal.clone())),
        Pushability::QuotedTriple(triple) => Some(TermPattern::Triple(Box::new(TriplePattern {
            subject: term_pattern_from_ground(&triple.subject)?,
            predicate: NamedNodePattern::NamedNode(triple.predicate.clone()),
            object: term_pattern_from_ground(&triple.object)?,
        }))),
        Pushability::SeedOnly => None,
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
/// expression positions: above the core the VALUES-join binds them, and beneath it an
/// expression that reads one has the value driven into the node that evaluates it —
/// see `drive_expression_reads`. In a property-function call's arguments they are
/// driven in too, wherever the call is, so the relation is invoked with them bound —
/// see `drive_call_arguments`.
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
    Ok(apply_shacl_probes(query, probes))
}

/// [`apply_shacl_prebinding`]'s rewrite, over probes that are already grounded and
/// already known to be non-empty.
///
/// Split out for the same reason [`apply_probes`] is: a
/// [`PreparedExecution`](crate::PreparedExecution) grounds its bindings into a
/// retained buffer rather than through a [`Prebindings`] list, and
/// [`crate::prebind_memo`] has to be able to run EXACTLY this rewrite — not a second
/// spelling of it — to check a memo against it.
pub(crate) fn apply_shacl_probes(query: Query, probes: Vec<(Variable, GroundTerm)>) -> Query {
    // The expression-position constants come from the SAME grounded values the seed
    // is about to carry, not from a second conversion of the same `TermValue`s.
    // `NamedNode`, `Literal` and `Variable` are all `Arc<str>`-backed, so lifting one
    // out of a probe is a refcount bump; re-grounding it is a fresh allocation, once
    // per pre-bound value, per focus node.
    //
    // Cloned rather than borrowed because `apply_probes` below CONSUMES the probe
    // list — it moves each `(Variable, GroundTerm)` into the seed's `Values` row —
    // and the walk that reads these runs after it, in that order, for the reason
    // `apply_probes` gives.
    let expr_subs = ExprSubs(probes.clone());

    let mut query = apply_probes(query, probes);
    map_patterns_in_query(&mut query, |pattern| {
        substitute_in_graph_pattern(pattern, &expr_subs, WalkScope::Descent);
    });
    query
}

/// What each pre-bound variable becomes in an EXPRESSION position, keyed by the
/// pre-bound [`Variable`] itself.
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
///   for, reintroduced one layer down. A [`Variable`] key keeps that property
///   without borrowing: it is `Arc<str>`-backed, so the key is a refcount bump on
///   the name the probe list already holds, and the list is then free of any
///   borrow of the caller's request — which is what lets [`apply_shacl_probes`]
///   take its probes BY VALUE and hand them straight to [`apply_probes`].
///
/// The value each entry carries is the GROUNDED TERM, not a pre-computed constant
/// expression. That is what lets the three consumers below — an expression position,
/// a property-function argument and a `GRAPH` name — each ask [`Pushability`] what
/// this particular position may do with it, instead of reading a decision some
/// earlier conversion already made for one of the three and then re-deriving the
/// other two from its leftovers. It costs nothing to carry: `NamedNode`, `Literal`
/// and `Variable` are all `Arc<str>`-backed, so an entry is two refcount bumps,
/// exactly as a lifted `Expression` was.
///
/// PRESENCE in the list means "pre-bound", which is a different question from "has a
/// form this position can take" and must stay separable from it —
/// [`substitute_in_expression`]'s `Bound` arm turns on exactly that distinction: a
/// variable pre-bound to a blank node has no expression form at all and `BOUND()`
/// must still say `true`.
struct ExprSubs(Vec<(Variable, GroundTerm)>);

impl ExprSubs {
    /// The entry for `name`, mirroring `HashMap::get`.
    fn get(&self, name: &str) -> Option<&GroundTerm> {
        self.0
            .iter()
            .find_map(|(key, ground)| (key.as_str() == name).then_some(ground))
    }

    /// Whether `name` is pre-bound at all, mirroring `HashMap::contains_key`.
    fn contains_key(&self, name: &str) -> bool {
        self.0.iter().any(|(key, _)| key.as_str() == name)
    }
}

/// Lift an already-grounded pre-binding into an [`Expression`] when [`Pushability`]
/// admits one there; `None` for a blank node or a quoted triple, neither of which has
/// an expression form, both of which ride the `VALUES` join instead.
///
/// Takes the [`GroundTerm`] rather than the [`TermValue`] it came from precisely so
/// the conversion is not repeated: the probe list already holds it, and both
/// `NamedNode` and `Literal` are `Arc<str>`-backed, so this clone allocates nothing.
pub(crate) fn expression_from_ground(ground: &GroundTerm) -> Option<Expression> {
    match Pushability::of(ground) {
        Pushability::Iri(node) => Some(Expression::NamedNode(node.clone())),
        Pushability::Literal(literal) => Some(Expression::Literal(literal.clone())),
        Pushability::QuotedTriple(_) | Pushability::SeedOnly => None,
    }
}

/// The IRI of a ground pre-binding, or `None` for every class [`Pushability`] keeps
/// out of a `GRAPH`/`SERVICE` name position.
///
/// The narrowest of the four consumers: that position names a GRAPH, and only an IRI
/// names one. [`substitute_in_named_node_pattern`] and
/// [`crate::prebind_memo`]'s replay of the same position both ask THIS rather than
/// each destructuring the term themselves, so the one place that decides is
/// [`Pushability::of`] and the one place that reads the decision for this position is
/// here.
pub(crate) fn named_node_from_ground(ground: &GroundTerm) -> Option<NamedNode> {
    match Pushability::of(ground) {
        Pushability::Iri(node) => Some(node.clone()),
        Pushability::Literal(_) | Pushability::QuotedTriple(_) | Pushability::SeedOnly => None,
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
fn substitute_in_graph_pattern(pattern: &mut GraphPattern, expr_subs: &ExprSubs, scope: WalkScope) {
    // A node that is not a solution-modifier wrapper hands its children the scope
    // beneath the seed; a wrapper hands its inner pattern its own.
    let beneath = scope.beneath();
    // Wildcard-free on purpose: a `GraphPattern` variant added later must fail to
    // compile here rather than silently pass through unsubstituted.
    let reads_expressions = match pattern {
        // A leaf's term positions are matched against the graph, not evaluated, and
        // `apply_substitutions`' pushdown has already written the pre-bound constants
        // into the ones that can carry them. A `Values` block's cells are data for the
        // same reason.
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => false,
        // BOTH arms, unlike the pushdown. Replacing a variable with a constant
        // EXPRESSION removes no column from any schema, so the divergence that stops
        // the pushdown at an `OPTIONAL`'s or a `MINUS`'s right arm does not arise here.
        // See `crate::enf`'s "The SHACL pre-binding fork".
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right } => {
            substitute_in_graph_pattern(left, expr_subs, beneath);
            substitute_in_graph_pattern(right, expr_subs, beneath);
            false
        }
        // A call that is a `Lateral`'s right operand is substituted in place and never
        // walked as a stand-alone call: that position is what hands it its left rows,
        // and the drive below keeps it there.
        //
        // Inside an `EXISTS` body the driver cannot join the left operand: that would
        // put the driven variable into the rows the body joins with the row being
        // filtered, which may already bind it. The call is driven in a scope of its own
        // instead — see [`plant_scoped_driver`] — and stays the `Lateral`'s right
        // operand, now correlated with each left row through the ordinary per-row path.
        GraphPattern::Lateral { left, right } => {
            substitute_in_graph_pattern(left, expr_subs, beneath);
            if let Some(call) = lateral_call_mut(right) {
                match beneath {
                    WalkScope::Descent | WalkScope::Group => {
                        if let Some(seed) = bind_call_arguments(call, &expr_subs.0, Some(left)) {
                            plant_left_driver(left, seed);
                        }
                    }
                    WalkScope::ExistsBody => {
                        if let Some(seed) = bind_call_arguments(call, &expr_subs.0, None) {
                            let kept = undriven_variables(call, &seed);
                            plant_scoped_driver(right, seed, kept);
                        }
                    }
                }
            } else {
                substitute_in_graph_pattern(right, expr_subs, beneath);
            }
            false
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            substitute_in_graph_pattern(left, expr_subs, beneath);
            substitute_in_graph_pattern(right, expr_subs, beneath);
            if let Some(expression) = expression {
                substitute_in_expression(expression, expr_subs);
            }
            expression.is_some()
        }
        GraphPattern::Filter { expr, inner } => {
            substitute_in_expression(expr, expr_subs);
            substitute_in_graph_pattern(inner, expr_subs, scope);
            true
        }
        GraphPattern::Graph { name, inner } | GraphPattern::Service { name, inner, .. } => {
            substitute_in_named_node_pattern(name, expr_subs);
            substitute_in_graph_pattern(inner, expr_subs, beneath);
            false
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
            substitute_in_graph_pattern(inner, expr_subs, scope);
            substitute_in_expression(expression, expr_subs);
            true
        }
        GraphPattern::OrderBy { inner, expression } => {
            substitute_in_graph_pattern(inner, expr_subs, scope);
            for order in expression.iter_mut() {
                substitute_in_order_expression(order, expr_subs);
            }
            true
        }
        // No `Project`-boundary narrowing: a SHACL pre-binding must reach an
        // UNPROJECTED scope inside a nested sub-`SELECT`, which is divergence 1 in
        // `crate::enf`'s module doc.
        GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => {
            substitute_in_graph_pattern(inner, expr_subs, scope);
            false
        }
        // A property function's arguments are INVOCATION INPUTS, evaluated per row like
        // a function call's arguments rather than matched against the graph like a BGP
        // term — so they are substituted here, on the same rule and for the same reason
        // expression positions are. The VALUES-join rewrite alone would not reach an
        // occurrence inside a sub-`SELECT` that does not project the pre-bound variable,
        // because that inner variable is a separate scope the join cannot correlate
        // with. IRI and literal values substitute. A blank-node or quoted-triple value
        // has no constant spelling here, and the `VALUES` seed does not reach this
        // scope either, so it is DRIVEN into the call through a one-row `VALUES` of its
        // own — see [`drive_call_arguments`] — and the relation is invoked with it
        // bound, exactly as with an IRI. Inside an `EXISTS` body that driver is planted
        // in a scope of its own, for the reason [`plant_scoped_driver`] gives.
        GraphPattern::PropertyFunction(call) => {
            if let Some(seed) = bind_call_arguments(call, &expr_subs.0, None) {
                match beneath {
                    WalkScope::Descent | WalkScope::Group => {
                        plant_stand_alone_driver(pattern, seed);
                    }
                    WalkScope::ExistsBody => {
                        let kept = undriven_variables(call, &seed);
                        plant_scoped_driver(pattern, seed, kept);
                    }
                }
            }
            false
        }
        GraphPattern::Group {
            inner, aggregates, ..
        } => {
            substitute_in_graph_pattern(inner, expr_subs, scope);
            // `AggregateExpression` is rebuilt through its consuming `into_parts`, so
            // the entries are taken by value and collected back. `Vec::into_iter().
            // collect()` into the same element type reuses the buffer, so the take and
            // the collect together allocate nothing.
            let taken = std::mem::take(aggregates);
            *aggregates = taken
                .into_iter()
                .map(|(var, agg)| (var, substitute_in_aggregate(agg, expr_subs)))
                .collect();
            true
        }
    };
    if reads_expressions && scope != WalkScope::Descent {
        drive_expression_reads(pattern, expr_subs);
    }
}

/// Drive into an expression-bearing node beneath the seed every pre-bound value its
/// expressions still read — a blank node or a quoted triple, which have no expression
/// form and so were left in place as the variable — so the expression reads the value
/// rather than an unbound variable.
///
/// # The defect this closes
///
/// An IRI or a literal is written into every expression as a constant. A blank node
/// and a quoted triple are not, on the premise that "the `VALUES` seed binds them" —
/// which holds only where the seed reaches: on the solution-modifier descent above the
/// core. Beneath it the seed is a sibling, joined AFTER the node is evaluated, so
/// `SELECT $this WHERE { BIND($this AS ?x) ?x <rel> ?why }` read `$this` unbound for a
/// blank or quoted focus node: `?x` was left unbound, the relation was invoked with
/// its input FREE and answered from its whole extent, and every focus node the
/// relation held ANY row for was reported — while the same constraint over an IRI
/// focus node reported only its own verdict.
///
/// # The rewrite, and why it changes nothing else
///
/// The node's evaluated operand (the inner pattern; an `OPTIONAL`'s left operand,
/// whose rows its condition reads) is joined with a one-row `VALUES` binding the
/// value — the same driver a call is given — and the node is wrapped in a projection
/// onto exactly the columns it exposed before. A value the operand's rows already
/// carry is not driven: the expression reads that row's binding, which the seed or the
/// enclosing correlation restricts as before. So the node's schema is unchanged, the
/// driven variable never escapes it — no `MINUS` sees a new shared variable, no
/// `EXISTS` body rebinds the row it filters (a projection is a scope boundary for
/// both) — and the only change is the one intended: the expression reads the value.
///
/// An `EXISTS` inside the expression reads the node's rows too, so an expression
/// holding one is driven with every value that has no expression form, whether or not
/// the body names it — its body's own reads (a triple pattern naming `$this`, which
/// cannot carry a blank node either) are correlated through those rows.
///
/// A `GROUP BY` hides its operand's columns already, so it is driven without the
/// projection.
fn drive_expression_reads(node: &mut GraphPattern, expr_subs: &ExprSubs) {
    // An IRI or a literal was written into the expression, so a run whose every value
    // is one — every IRI focus node — has nothing to drive and reads nothing more.
    if !expr_subs
        .0
        .iter()
        .any(|(_, ground)| Unwritable::InArgument.holds(ground))
    {
        return;
    }
    let mut read = Vec::new();
    match &*node {
        GraphPattern::Extend { expression, .. } | GraphPattern::Unfold { expression, .. } => {
            unwritten_reads(expression, expr_subs, &mut read);
        }
        GraphPattern::Filter { expr, .. } => unwritten_reads(expr, expr_subs, &mut read),
        GraphPattern::LeftJoin {
            expression: Some(expression),
            ..
        } => unwritten_reads(expression, expr_subs, &mut read),
        GraphPattern::OrderBy { expression, .. } => {
            for order in expression {
                match order {
                    OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => {
                        unwritten_reads(expr, expr_subs, &mut read);
                    }
                }
            }
        }
        GraphPattern::Group { aggregates, .. } => {
            for (_, aggregate) in aggregates {
                for arg in aggregate.args() {
                    unwritten_reads(arg, expr_subs, &mut read);
                }
                for order in aggregate.order_by() {
                    match order {
                        OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => {
                            unwritten_reads(expr, expr_subs, &mut read);
                        }
                    }
                }
            }
        }
        _ => {}
    }
    if read.is_empty() {
        return;
    }
    let exposed = crate::eval::syntactic_schema(node);
    read.retain(|&index| !exposed.contains(&expr_subs.0[index].0));
    if read.is_empty() {
        return;
    }
    let seed = seed_row(&read, &expr_subs.0);
    let hides_operand = matches!(node, GraphPattern::Group { .. });
    match node {
        GraphPattern::Extend { inner, .. }
        | GraphPattern::Unfold { inner, .. }
        | GraphPattern::Filter { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Group { inner, .. }
        | GraphPattern::LeftJoin { left: inner, .. } => plant_left_driver(inner, seed),
        _ => return,
    }
    if !hides_operand {
        let variables = exposed.vars().to_vec();
        purrdf_sparql_algebra::substitute::take_and_replace(node, |node| GraphPattern::Project {
            inner: Box::new(node),
            variables,
        });
    }
}

/// The indices into `expr_subs` of the pre-bound values `expr` still reads after
/// [`substitute_in_expression`] — each one a value with no expression form, left in
/// place as its variable — plus every such value when `expr` holds an `EXISTS`. See
/// [`drive_expression_reads`].
fn unwritten_reads(expr: &Expression, expr_subs: &ExprSubs, read: &mut Vec<usize>) {
    let mut note = |index: usize| {
        if !read.contains(&index) {
            read.push(index);
        }
    };
    match expr {
        Expression::Variable(var) => {
            if let Some(index) = expr_subs
                .0
                .iter()
                .position(|(candidate, _)| candidate == var)
            {
                note(index);
            }
        }
        Expression::Exists(_) => {
            for (index, (_, ground)) in expr_subs.0.iter().enumerate() {
                if Unwritable::InArgument.holds(ground) {
                    note(index);
                }
            }
        }
        Expression::NamedNode(_) | Expression::Literal(_) | Expression::Bound(_) => {}
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
            unwritten_reads(left, expr_subs, read);
            unwritten_reads(right, expr_subs, read);
        }
        Expression::UnaryPlus(inner) | Expression::UnaryMinus(inner) | Expression::Not(inner) => {
            unwritten_reads(inner, expr_subs, read);
        }
        Expression::In(target, list) => {
            unwritten_reads(target, expr_subs, read);
            for item in list {
                unwritten_reads(item, expr_subs, read);
            }
        }
        Expression::If(cond, then_expr, else_expr) => {
            unwritten_reads(cond, expr_subs, read);
            unwritten_reads(then_expr, expr_subs, read);
            unwritten_reads(else_expr, expr_subs, read);
        }
        Expression::Coalesce(list) | Expression::FunctionCall(_, list) => {
            for item in list {
                unwritten_reads(item, expr_subs, read);
            }
        }
    }
}

/// Replace a pre-bound variable in a property-function argument position with its
/// constant term.
///
/// This position writes a [`TermPattern`] but follows the EXPRESSION rule, not the
/// matched-term rule, which is why [`Pushability::QuotedTriple`] is refused here and
/// admitted by [`term_pattern_from_ground`]: a property function's arguments are
/// invocation INPUTS, evaluated per row like a function call's, rather than terms
/// matched against the graph. A value with no expression form therefore has nothing
/// to be substituted with here, and is driven into the call instead — see
/// [`drive_call_arguments`]. A non-variable argument is already a constant and
/// passes through unchanged.
///
/// A quoted-triple argument is entered: its subject and object are argument
/// positions too, and a variable there is an input of the call exactly as a bare
/// argument is. Left unwritten, the relation would be handed a triple with that
/// component free — refused by a relation serving only the bound mode, and answered
/// from the relation's whole extent by one serving both. [`driven_arguments`] enters
/// the same two positions for the values it drives, so every variable an argument
/// names, at any depth, is either written or driven. The predicate is not entered:
/// it names a predicate, which only an IRI can, and neither rule writes there.
fn substitute_in_term_pattern(term: &mut TermPattern, values: &[(Variable, GroundTerm)]) {
    let var = match term {
        TermPattern::Variable(var) => var,
        TermPattern::Triple(triple) => {
            substitute_in_term_pattern(&mut triple.subject, values);
            substitute_in_term_pattern(&mut triple.object, values);
            return;
        }
        TermPattern::NamedNode(_) | TermPattern::BlankNode(_) | TermPattern::Literal(_) => {
            return;
        }
    };
    let Some((_, ground)) = values.iter().find(|(candidate, _)| candidate == var) else {
        return;
    };
    // Wildcard-free over [`Pushability`], so a new class of value cannot slip through
    // this position on the strength of a catch-all written for the old ones.
    let replacement = match Pushability::of(ground) {
        Pushability::Iri(node) => TermPattern::NamedNode(node.clone()),
        Pushability::Literal(literal) => TermPattern::Literal(literal.clone()),
        Pushability::QuotedTriple(_) | Pushability::SeedOnly => return,
    };
    *term = replacement;
}

/// Replace a pre-bound variable in a `GRAPH`/`SERVICE` name with its IRI constant.
///
/// The decision is [`named_node_from_ground`]'s, which is [`Pushability`]'s: a graph
/// is named by an IRI, so every other class is left for the `VALUES` seed. (The
/// earlier spelling asked `contains_key` and then re-asked `get`, which read as two
/// tests and was one — presence is implied by a `Some` entry.)
fn substitute_in_named_node_pattern(pattern: &mut NamedNodePattern, expr_subs: &ExprSubs) {
    let NamedNodePattern::Variable(var) = pattern else {
        return;
    };
    let Some(node) = expr_subs.get(var.as_str()).and_then(named_node_from_ground) else {
        return;
    };
    *pattern = NamedNodePattern::NamedNode(node);
}

/// Recursively substitute pre-bound variables into an [`Expression`].
fn substitute_in_expression(expr: &mut Expression, expr_subs: &ExprSubs) {
    // Wildcard-free on purpose, for the same reason the graph-pattern walk is.
    match expr {
        Expression::Variable(var) => {
            // Resolved before the assignment so `var`'s borrow of `*expr` has ended.
            // `expression_from_ground` is the [`Pushability`] consumer for this
            // position, and it is asked per OCCURRENCE rather than once per probe —
            // which costs the same, because the answer it builds for the two classes
            // that have one is a refcount bump on an `Arc<str>` either way.
            let replacement = expr_subs.get(var.as_str()).and_then(expression_from_ground);
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
        // Everything below is an `EXISTS` body, however deeply it is nested there.
        Expression::Exists(inner) => {
            substitute_in_graph_pattern(inner, expr_subs, WalkScope::ExistsBody);
        }
    }
}

/// Substitute inside an [`OrderExpression`] sort key.
fn substitute_in_order_expression(order: &mut OrderExpression, expr_subs: &ExprSubs) {
    match order {
        OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => {
            substitute_in_expression(expr, expr_subs);
        }
    }
}

/// Substitute inside a [`GROUP BY`][`AggregateExpression`] aggregate.
fn substitute_in_aggregate(agg: AggregateExpression, expr_subs: &ExprSubs) -> AggregateExpression {
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

/// Convert a dataset's own term id straight to the algebra's [`GroundTerm`],
/// without spelling the term out as a [`TermValue`] on the way.
///
/// # What this saves, and why it is not merely a shortcut
///
/// The value door's route from an interned term to the algebra is
/// `TermId` → [`TermValue`] → [`GroundTerm`]: the first step owns a `String` per
/// component (one for an IRI, up to three for a literal, a whole tree for a quoted
/// triple), and the second immediately re-owns the same bytes into the `Arc<str>`
/// the algebra holds — after which `crate::bgp`'s `compile_term` hashes the result
/// back to the id it started from. This route drops the middle allocation entirely:
/// [`DatasetView::resolve`] hands back a borrowed [`TermRef`], and the algebra term
/// is built from that borrow. A caller with an id in hand pays ONE materialization
/// instead of two.
///
/// It is a strictly narrower door, not a looser one: every component still goes
/// through the same [`node`] and [`lang`] admission the value door uses, so a
/// dataset holding a term the algebra would refuse is refused here identically
/// rather than admitted because it came from "inside".
///
/// # Errors
///
/// As [`ground_term_from_value`]: an IRI a [`NamedNode`] would refuse, a language tag
/// this profile does not lex, or a quoted triple whose predicate position is not an
/// IRI.
pub(crate) fn ground_term_from_id<D: DatasetView>(
    dataset: &D,
    id: D::Id,
) -> Result<GroundTerm, RdfDiagnostic> {
    match dataset.resolve(id) {
        TermRef::Iri(iri) => Ok(GroundTerm::NamedNode(node(iri)?)),
        // Qualified exactly as `ground_term_from_value` qualifies a `TermValue::Blank`,
        // because the two doors must produce the same algebra term for the same
        // dataset node — the algebra's `BlankNode` has one string slot, and the
        // scope-qualified rendering is how every single-slot blank surface carries a
        // `(label, scope)` pair.
        TermRef::Blank { label, scope } => Ok(GroundTerm::BlankNode(BlankNode::new(
            scope.qualify_label(label).into_owned(),
        ))),
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let TermRef::Iri(datatype) = dataset.resolve(datatype) else {
                return Err(RdfDiagnostic::error(
                    "native-sparql-subst-literal-datatype",
                    "a literal's datatype must be an IRI".to_owned(),
                ));
            };
            Ok(GroundTerm::Literal(literal_from_value(
                lexical, datatype, language, direction,
            )?))
        }
        TermRef::Triple { s, p, o } => {
            let subject = ground_term_from_id(dataset, s)?;
            let GroundTerm::NamedNode(predicate) = ground_term_from_id(dataset, p)? else {
                return Err(RdfDiagnostic::error(
                    "native-sparql-subst-triple-predicate",
                    "a quoted-triple predicate must be an IRI".to_owned(),
                ));
            };
            let object = ground_term_from_id(dataset, o)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `http://www.w3.org/2001/XMLSchema#string`, for a plain literal fixture.
    const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

    /// The five ground values the classification distinguishes, as `(label, term)`.
    ///
    /// Five rather than four because [`GroundTerm::Triple`] splits: a quoted triple
    /// every position of which is pushable behaves differently from one with a blank
    /// node nested inside it, and that split is a property of the RECURSION in
    /// [`term_pattern_from_ground`] rather than of [`Pushability`] itself. A table
    /// that held only one quoted triple could not tell the two apart, and the nested
    /// case is exactly where the blank-node rule has to survive a level of nesting.
    fn ground_fixtures() -> Vec<(&'static str, GroundTerm)> {
        let iri = || NamedNode::new_unchecked("http://example.org/i");
        let literal = || Literal::new_typed("v", NamedNode::new_unchecked(XSD_STRING));
        vec![
            ("iri", GroundTerm::NamedNode(iri())),
            ("literal", GroundTerm::Literal(literal())),
            (
                "quoted-triple",
                GroundTerm::Triple(Box::new(GroundTriple {
                    subject: GroundTerm::NamedNode(iri()),
                    predicate: iri(),
                    object: GroundTerm::Literal(literal()),
                })),
            ),
            (
                "quoted-triple-with-nested-blank",
                GroundTerm::Triple(Box::new(GroundTriple {
                    subject: GroundTerm::BlankNode(BlankNode::new("nested")),
                    predicate: iri(),
                    object: GroundTerm::Literal(literal()),
                })),
            ),
            ("blank", GroundTerm::BlankNode(BlankNode::new("b"))),
        ]
    }

    /// **The pushability truth table, asserted at all four sites at once.**
    ///
    /// The four consumers of a pre-bound value each used to re-decide what a value
    /// may be used for, over their own subset of [`GroundTerm`] and with a catch-all
    /// under three of the four. [`Pushability`] now decides once. This is the table
    /// that says the four still accept and refuse EXACTLY what they accepted and
    /// refused before that change — every site, every class, both answers, written
    /// out rather than summarized, because a refactor of a refusal is precisely where
    /// a quietly-widened or quietly-narrowed rule hides.
    ///
    /// Read the expectations as: `term` = the pushdown's matched-pattern position
    /// ([`term_pattern_from_ground`]); `expr` = a constant expression
    /// ([`expression_from_ground`]); `arg` = a property-function argument
    /// ([`substitute_in_term_pattern`]); `graph` = a `GRAPH`/`SERVICE` name
    /// ([`substitute_in_named_node_pattern`]).
    #[test]
    fn the_pushability_classifier_reproduces_every_site_s_decision() {
        // (label, term, expr, arg, graph) — the behaviour BEFORE the classifier, read
        // off the four original matches, and the behaviour required after it.
        let expected: &[(&str, bool, bool, bool, bool)] = &[
            ("iri", true, true, true, true),
            ("literal", true, true, true, false),
            // Pushed into a matched position component-wise; no expression form, and
            // a property-function argument follows the expression rule.
            ("quoted-triple", true, false, false, false),
            // The nested blank takes the WHOLE triple out of the pushdown.
            (
                "quoted-triple-with-nested-blank",
                false,
                false,
                false,
                false,
            ),
            // The load-bearing row: a blank node is written into no position; it is
            // bound through `VALUES` rows alone.
            ("blank", false, false, false, false),
        ];

        let variable = Variable::new("v");
        for (label, ground) in ground_fixtures() {
            let row = expected
                .iter()
                .find(|(name, ..)| *name == label)
                .unwrap_or_else(|| panic!("no expectation row for {label}"));
            let (_, want_term, want_expr, want_arg, want_graph) = *row;

            assert_eq!(
                term_pattern_from_ground(&ground).is_some(),
                want_term,
                "term_pattern_from_ground({label})"
            );
            assert_eq!(
                expression_from_ground(&ground).is_some(),
                want_expr,
                "expression_from_ground({label})"
            );

            let subs = ExprSubs(vec![(variable.clone(), ground)]);

            let mut arg = TermPattern::Variable(variable.clone());
            substitute_in_term_pattern(&mut arg, &subs.0);
            assert_eq!(
                !matches!(arg, TermPattern::Variable(_)),
                want_arg,
                "substitute_in_term_pattern({label})"
            );

            let mut name = NamedNodePattern::Variable(variable.clone());
            substitute_in_named_node_pattern(&mut name, &subs);
            assert_eq!(
                matches!(name, NamedNodePattern::NamedNode(_)),
                want_graph,
                "substitute_in_named_node_pattern({label})"
            );

            // PRESENCE is a separate question from admissibility, and the `Bound`
            // arm depends on the two staying separate: every class above is
            // pre-bound, including the three no position can spell.
            assert!(
                subs.contains_key("v"),
                "{label} is pre-bound whatever any position can do with it"
            );
            let mut bound = Expression::Bound(variable.clone());
            substitute_in_expression(&mut bound, &subs);
            assert_eq!(
                bound,
                true_literal(),
                "BOUND(?v) must fold to true for {label}, which has no expression form \
                 for three of these five classes and is bound in all five"
            );
        }
    }

    /// **A variable that is NOT pre-bound is untouched at every site.**
    ///
    /// The neighbour of the table above, and not a formality: three of the four sites
    /// now reach their decision through `ExprSubs::get`, so a lookup that answered
    /// `Some` for an absent name would substitute a value into a position for a
    /// variable the caller never bound — the widening direction of the same bug. The
    /// `Bound` arm is the one that can SEE the difference, so it is checked here too:
    /// `BOUND(?other)` must stay `BOUND(?other)` rather than folding to `true`.
    #[test]
    fn an_unbound_variable_is_left_alone_at_every_site() {
        let subs = ExprSubs(vec![(
            Variable::new("v"),
            GroundTerm::NamedNode(NamedNode::new_unchecked("http://example.org/i")),
        )]);
        let other = Variable::new("other");

        assert!(!subs.contains_key("other"));
        assert!(subs.get("other").is_none());

        let mut arg = TermPattern::Variable(other.clone());
        substitute_in_term_pattern(&mut arg, &subs.0);
        assert_eq!(arg, TermPattern::Variable(other.clone()));

        let mut name = NamedNodePattern::Variable(other.clone());
        substitute_in_named_node_pattern(&mut name, &subs);
        assert_eq!(name, NamedNodePattern::Variable(other.clone()));

        let mut expr = Expression::Variable(other.clone());
        substitute_in_expression(&mut expr, &subs);
        assert_eq!(expr, Expression::Variable(other.clone()));

        let mut bound = Expression::Bound(other.clone());
        substitute_in_expression(&mut bound, &subs);
        assert_eq!(
            bound,
            Expression::Bound(other),
            "an unbound variable's BOUND() must not fold"
        );
    }

    /// `interned_variable`'s per-worker table retains bytes that must stay
    /// charged against the observer for as long as the table holds them —
    /// `INTERNED_VARIABLE_CAP` bounds the table's ENTRY count but not its
    /// bytes against the SAME `PlanMemoryStats`
    /// a caller already reads to bound `PlanCache`'s retained plan bytes
    /// (`crate::plan_memory::interner_memory_observer`; see
    /// `crate::solution::tests::interning_schemas_moves_the_thread_local_memory_observer`
    /// for the layout-table twin of this test). This is a MOVING assertion, not
    /// a smoke test: it first proves interning grows the observer's total, then
    /// interns several `INTERNED_VARIABLE_CAP` MULTIPLES worth of distinct
    /// names (several full clear cycles) and asserts the total stays bounded to
    /// roughly one table's worth of entries rather than the far larger number
    /// ever inserted — the property that only holds if the cap-triggered clear
    /// actually credits the observer back down each cycle.
    #[test]
    fn interning_variables_moves_the_thread_local_memory_observer() {
        let observer = crate::plan_memory::interner_memory_observer();
        let before = observer.stats().retained_bytes;

        for i in 0..8 {
            let _ = interned_variable(&format!("f4_var_grow_{i}"));
        }
        let after_growth = observer.stats().retained_bytes;
        let grown = after_growth.saturating_sub(before);
        assert!(
            grown > 0,
            "interning new names must grow the observer's retained bytes \
             (before: {before}, after growth: {after_growth})"
        );
        let per_entry = (grown / 8).max(1);

        let cycles = 3;
        let total_inserted = INTERNED_VARIABLE_CAP * cycles;
        for i in 0..total_inserted {
            let _ = interned_variable(&format!("f4_var_fill_{i}"));
        }
        let after_fill = observer.stats().retained_bytes;
        // Generous (2x) bound on ONE table's worth of entries: without
        // credit-on-clear, `total_inserted` (three full tables) would instead be
        // charged in full.
        let bound = per_entry.saturating_mul(INTERNED_VARIABLE_CAP * 2);
        assert!(
            after_fill <= bound,
            "a cap-triggered clear must keep the observer's total bounded to \
             roughly one table's worth of entries, not the {total_inserted} names \
             ever inserted (after fill: {after_fill}, bound: {bound})"
        );
    }

    /// **A variable inside a quoted-triple argument is written or driven, like a bare
    /// one.** The component is an input of the call; [`bind_call_arguments`] writes an
    /// IRI there and drives a blank node, and the neighbouring variable no value names
    /// is left alone.
    #[test]
    fn a_variable_inside_a_quoted_triple_argument_is_written_or_driven() {
        let v = Variable::new("v");
        let a = NamedNode::new_unchecked("http://example.org/a");
        let r = NamedNode::new_unchecked("http://example.org/r");
        let i = NamedNode::new_unchecked("http://example.org/i");
        let quoted = |object: TermPattern| {
            TermPattern::Triple(Box::new(TriplePattern {
                subject: TermPattern::NamedNode(a.clone()),
                predicate: NamedNodePattern::NamedNode(r.clone()),
                object,
            }))
        };
        let call = || PropertyFunctionCall {
            iri: "http://example.org/rel".to_owned(),
            subject_args: vec![quoted(TermPattern::Variable(v.clone()))],
            object_args: vec![TermPattern::Variable(Variable::new("out"))],
        };

        let mut written = call();
        let seed = bind_call_arguments(
            &mut written,
            &[(v.clone(), GroundTerm::NamedNode(i.clone()))],
            None,
        );
        assert!(seed.is_none(), "an IRI is written, so nothing is driven");
        assert_eq!(
            written.subject_args,
            vec![quoted(TermPattern::NamedNode(i))]
        );
        assert_eq!(
            written.object_args,
            vec![TermPattern::Variable(Variable::new("out"))],
            "a variable no value names is left alone"
        );

        let blank = GroundTerm::BlankNode(BlankNode::new("b"));
        let mut driven = call();
        let seed = bind_call_arguments(&mut driven, &[(v.clone(), blank.clone())], None);
        assert_eq!(
            driven.subject_args,
            call().subject_args,
            "a blank node is never written into the argument"
        );
        assert_eq!(
            seed,
            Some(GraphPattern::Values {
                variables: vec![v],
                bindings: vec![vec![Some(blank)]],
            }),
            "it is driven by a one-row VALUES instead"
        );
    }

    /// **Inside an `EXISTS` body a call is driven in a scope of its own; outside one it
    /// is driven where it is.** The same call, pre-bound to a blank node, is rewritten
    /// both ways in one query: outside the `EXISTS`, `Lateral(VALUES, call)` binds
    /// `?this` beside the call's own variables; inside it, the same driver sits under a
    /// projection carrying out every variable the call names — a predicate variable
    /// inside a quoted-triple argument included — except `?this`, so the body never
    /// rebinds the `?this` of the row it filters. The `FILTER` itself sits beneath the
    /// seed, so the rows it filters are driven with `?this` too, under a projection
    /// onto the columns the `FILTER` exposed — none — which keeps `?this` from escaping
    /// it (see [`drive_expression_reads`]).
    #[test]
    fn a_call_inside_an_exists_body_is_driven_in_a_scope_of_its_own() {
        let this = Variable::new("this");
        let p = Variable::new("p");
        let o = Variable::new("o");
        let out = Variable::new("out");
        let call = GraphPattern::PropertyFunction(PropertyFunctionCall {
            iri: "http://example.org/rel".to_owned(),
            subject_args: vec![TermPattern::Triple(Box::new(TriplePattern {
                subject: TermPattern::Variable(this.clone()),
                predicate: NamedNodePattern::Variable(p.clone()),
                object: TermPattern::Variable(o.clone()),
            }))],
            object_args: vec![TermPattern::Variable(out.clone())],
        });
        let blank = GroundTerm::BlankNode(BlankNode::new("b"));
        let driver = || GraphPattern::Lateral {
            left: Box::new(GraphPattern::Values {
                variables: vec![this.clone()],
                bindings: vec![vec![Some(blank.clone())]],
            }),
            right: Box::new(call.clone()),
        };
        let mut pattern = GraphPattern::Join {
            left: Box::new(call.clone()),
            right: Box::new(GraphPattern::Filter {
                expr: Expression::Exists(Box::new(call.clone())),
                inner: Box::new(GraphPattern::Bgp {
                    patterns: Vec::new(),
                }),
            }),
        };
        let subs = ExprSubs(vec![(this.clone(), blank.clone())]);
        substitute_in_graph_pattern(&mut pattern, &subs, WalkScope::Group);
        assert_eq!(
            pattern,
            GraphPattern::Join {
                left: Box::new(driver()),
                right: Box::new(GraphPattern::Project {
                    inner: Box::new(GraphPattern::Filter {
                        expr: Expression::Exists(Box::new(GraphPattern::Project {
                            inner: Box::new(driver()),
                            variables: vec![p, o, out],
                        })),
                        inner: Box::new(GraphPattern::Join {
                            left: Box::new(GraphPattern::Values {
                                variables: vec![this.clone()],
                                bindings: vec![vec![Some(blank.clone())]],
                            }),
                            right: Box::new(GraphPattern::Bgp {
                                patterns: Vec::new(),
                            }),
                        }),
                    }),
                    variables: Vec::new(),
                }),
            }
        );
    }
}
