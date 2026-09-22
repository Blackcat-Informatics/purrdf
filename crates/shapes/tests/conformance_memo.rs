// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **A shape named at two constraint sites is asked about a shared value node
//! once, not twice.**
//!
//! `sh:node`, `sh:and`, `sh:or`, `sh:xone` and `sh:qualifiedValueShape` all ask
//! the same question — "does this node conform to that shape?" — and the answer
//! is a pure function of the data graph, the node and the shape. When two
//! constraint sites name the SAME shape and their value sets overlap, the second
//! ask re-runs a traversal whose answer is already known.
//!
//! The evaluator memoizes those answers for the duration of one focus node's
//! validation. This file is the executable proof that the memo exists and fires,
//! because a cache is the easiest kind of code to ship broken: one that never
//! hits is indistinguishable from one that was never wired up, and every other
//! test in this crate passes either way.
//!
//! # How a memo hit is made visible
//!
//! Two shapes graphs of identical STRUCTURE and identical meaning, differing
//! only in whether the inner shape is reachable by name:
//!
//! * [`named_shapes`] states the inner shape once as `ex:Inner` and refers to it
//!   from both property shapes. Both sites name the same shape, so the second ask
//!   is answered from the memo.
//! * [`inline_shapes`] writes the same constraint out twice as anonymous shapes.
//!   Two anonymous shapes are two DIFFERENT shapes — each belongs to exactly one
//!   site and can share an answer with nothing — so both asks run.
//!
//! Both graphs validate the same data, both must produce the same report, and the
//! named one must evaluate the inner shape's constraint exactly ONCE where the
//! inline one evaluates it twice.
//!
//! # The oracle: the constraint counts its own evaluations
//!
//! The quantity asserted below is **how many times the inner shape's constraint
//! actually ran**, counted by the constraint itself. [`INNER_CONSTRAINT`] is a
//! `sh:sparql` SELECT whose single triple pattern reaches [`ASKED_IRI`], a
//! host-registered RELATION ([`AskCountingRelation`]) whose `open` increments
//! [`ASKS`]. A relation's sole free-free invocation is opened exactly once per
//! evaluation of the query that names it, so [`ASKS`] is a count of constraint
//! evaluations and of nothing else. The cursor yields NO rows, which is what makes
//! the constraint CONFORM: the SELECT returns the empty bag and reports no
//! violation while having run in full.
//!
//! This is the quantity the conformance memo changes, and it is the ONLY quantity
//! it changes. That matters because the obvious alternative instrument — count the
//! allocations of one `validate_focus_node_ids` call and require the named arm to
//! be lower — measures a sum that every layer beneath the memo also moves, and so
//! cannot state what this file claims.
//!
//! That is not a hypothetical. This file DID assert an allocation inequality, and
//! it inverted when a lower layer — the prebind memo a `PreparedExecution` keeps
//! for its substituted algebra — landed underneath it. The conformance memo was
//! still firing perfectly throughout; what moved was that the prebind memo builds
//! its retained tree on the second consecutive run of one value shape, a one-time
//! cost that amortizes over the runs after it, and the two arms interleave such
//! that the named arm's build landed INSIDE its measured window while the inline
//! arm's had already been paid outside its own. An allocation total could not tell
//! those two facts apart, because it is one number for two independent caches.
//!
//! The figures that retired it, for the record — a fixed label for one past
//! measurement, not a live claim, and deliberately restating no constant this
//! workspace pins elsewhere: named 147 against inline 133 with both layers live,
//! and named 72 against inline 137 with the prebind layer held uniform, a margin
//! of one whole `sh:sparql` evaluation. The prebind build accounted for the whole
//! inversion at +75 allocations, paid once.
//!
//! A count of constraint evaluations has no such coupling. Make the rewrite cheaper,
//! make the plan cache warmer, add or remove a memo below this one: the named arm
//! still asks once and the inline arm still asks twice, and the figures below do not
//! move. The only thing that moves them is the thing this file is about.
//!
//! # Why the fixture's two arms are a control and a treatment
//!
//! The inline arm is the control, and it is asserted at **2** rather than merely
//! "more than the named arm". A fixture whose control reads the same as its
//! treatment cannot distinguish "the memo was honoured" from "the constraint was
//! silently never reached at all" — a body whose predicate stopped resolving to
//! [`ASKED_IRI`] (which makes it an ordinary triple pattern over a graph that has
//! no such triple, so the query still conforms and nothing looks broken), or a
//! target that never selected the focus node, would leave both arms at 0 and
//! satisfy any inequality between them. Two exact figures, 1 and 2, are falsifiable
//! in both directions: a memo that stops firing takes the named arm to 2, and an
//! oracle that stops observing takes both to 0.
//!
//! # The instrument
//!
//! [`ASKS`] is one PROCESS-GLOBAL counter, so a sibling test validating
//! concurrently would add its own asks to a reading in flight. [`MEASURE_LOCK`]
//! serializes every test in this binary, and
//! [`every_test_in_this_binary_takes_the_measure_lock_first`] enforces that by
//! reading this file's own source: a future test added here without the lock
//! must fail loudly and name itself rather than show up as an occasional,
//! unattributed shift in the figures below.
//!
//! One focus node is far below `crate::parallel::PARALLEL_MIN_FOCUS_NODES`, so
//! both arms stay on the serial path, which is also why one thread-local
//! `enter_property_function_scope` reaches every evaluation this test performs.
//!
//! Every IRI is under `example.org`: PurRDF mints no vocabulary IRIs.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use purrdf::RdfDataset;
use purrdf_shapes::engine::{FocusId, PreparedShapes, PreparedValidator, parse_shapes};
use purrdf_shapes::sparql::enter_property_function_scope;
use purrdf_shapes::term::NamedNode;
use purrdf_shapes::text_ingest::parse_ntriples_to_dataset;
use purrdf_sparql_eval::{
    BindingPattern, EvalError, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction,
    PropertyFunctionRegistry, ServiceLevel, Volatility,
};

/// Serializes every measured region in this binary.
///
/// [`ASKS`] is one process-global counter and `cargo test` runs test functions
/// concurrently, so two readings in flight at once would each report the union of
/// both regions while appearing to report their own.
static MEASURE_LOCK: Mutex<()> = Mutex::new(());

/// How many times the inner shape's constraint has been evaluated since the last
/// [`take_asks`].
///
/// Process-global rather than thread-local because a registered relation is held
/// as `Arc<dyn PropertyFunction>` and is therefore `Send + Sync` by that seam's own
/// signature — the evaluator may in principle open it from a `rayon` worker, and a
/// thread-local would then silently read zero. [`MEASURE_LOCK`] is what keeps two
/// readings from overlapping.
static ASKS: AtomicUsize = AtomicUsize::new(0);

/// The relation IRI the inner constraint's body reaches to record that it ran.
///
/// PurRDF mints no vocabulary: without [`counting_relations`] registering it, the
/// same predicate is an ordinary triple pattern.
const ASKED_IRI: &str = "http://example.org/purrdf/memo#asked";

/// Take [`MEASURE_LOCK`], absorbing poison.
///
/// A panicking assertion inside a measured region poisons the mutex. Propagating
/// that would turn one real failure into a cascade of unrelated ones and bury the
/// diagnosis; the lock guards a counter, not an invariant that a panic could have
/// left half-written.
fn measure_lock() -> MutexGuard<'static, ()> {
    MEASURE_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Reset [`ASKS`] to zero and return what it held.
fn take_asks() -> usize {
    ASKS.swap(0, Ordering::SeqCst)
}

/// The relation [`INNER_CONSTRAINT`]'s body reaches, and the whole of the oracle:
/// opening it records one evaluation of that constraint.
///
/// A host-registered RELATION rather than a host-registered scalar function,
/// because the scalar seam is not reachable from here: the engine installs the
/// shapes graph's own `sh:SPARQLFunction` registry around every validation
/// (`enter_function_scope` in `crate::engine`), which shadows any registry a caller
/// installed, whereas the property-function scope is READ from the caller
/// (`current_property_functions`) and so passes through. Both positions are free,
/// because `$this` is pre-bound by substitution and the evaluation-order analysis
/// does not see that as a binding.
#[derive(Debug)]
struct AskCountingRelation {
    /// The one calling pattern this relation admits.
    modes: [BindingPattern; 1],
}

/// [`AskCountingRelation`]'s cursor: no rows, which is what makes the constraint
/// CONFORM while still having run.
#[derive(Debug)]
struct EmptyCursor;

impl PfCursor for EmptyCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(None)
    }

    fn service_level(&self) -> ServiceLevel {
        ServiceLevel::Undeclared
    }
}

impl PropertyFunction for AskCountingRelation {
    /// [`Volatility::Volatile`], which is the truthful declaration and not a tuning
    /// choice: [`Self::open`] mutates state that changes between calls within one
    /// query, which is exactly what that variant means. Declaring
    /// [`Volatility::Stable`] would assert the opposite and license an evaluator to
    /// treat two invocations as interchangeable — which is precisely the licence an
    /// oracle that COUNTS invocations must not hand out.
    fn volatility(&self) -> Volatility {
        Volatility::Volatile
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        1
    }

    /// One invocation of the relation, which is one evaluation of the constraint
    /// whose body names it.
    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        ASKS.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(EmptyCursor))
    }
}

/// A registry carrying the one relation [`INNER_CONSTRAINT`] reaches.
fn counting_relations() -> Arc<PropertyFunctionRegistry> {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        ASKED_IRI.to_owned(),
        Arc::new(AskCountingRelation {
            modes: [BindingPattern::from_code("ff")],
        }),
    );
    Arc::new(registry)
}

/// The data graph: one focus node whose two paths lead to the SAME value node,
/// which is what makes the two constraint sites ask about one node.
const DATA: &str = concat!(
    "<http://example.org/purrdf/memo#focus> \
     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
     <http://example.org/purrdf/memo#Focus> .\n",
    "<http://example.org/purrdf/memo#focus> <http://example.org/purrdf/memo#left> \
     <http://example.org/purrdf/memo#shared> .\n",
    "<http://example.org/purrdf/memo#focus> <http://example.org/purrdf/memo#right> \
     <http://example.org/purrdf/memo#shared> .\n",
    "<http://example.org/purrdf/memo#shared> <http://example.org/purrdf/memo#count> \
     \"7\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
);

/// The one constraint the inner shape carries, spelled identically wherever it
/// appears.
///
/// Its one triple pattern reaches [`ASKED_IRI`], the host-registered relation that
/// records the evaluation and yields no rows. A conforming focus node therefore
/// produces the empty solution bag while still having run the query. The predicate
/// is spelled as an absolute IRI so no `sh:prefixes` declaration stands between the
/// reader and what the query matches.
const INNER_CONSTRAINT: &str = concat!(
    "sh:sparql [ a sh:SPARQLConstraint ; sh:select \"\"\"\n",
    "    SELECT $this WHERE {\n",
    "      $this <http://example.org/purrdf/memo#asked> ?n .\n",
    "    }\"\"\" ]",
);

/// The focus node both arms validate, by IRI.
const FOCUS_IRI: &str = "http://example.org/purrdf/memo#focus";

/// The `@prefix` header both shapes graphs share.
const PREFIXES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/purrdf/memo#> .\n",
);

/// The inner shape reached by NAME from both sites: one shape, one answer.
fn named_shapes() -> String {
    format!(
        "{PREFIXES}\
         ex:Inner a sh:NodeShape ; {INNER_CONSTRAINT} .\n\
         ex:Outer a sh:NodeShape ; sh:targetClass ex:Focus ;\n\
         \x20   sh:property [ sh:path ex:left ; sh:node ex:Inner ] ;\n\
         \x20   sh:property [ sh:path ex:right ; sh:node ex:Inner ] .\n"
    )
}

/// The same constraint, written out twice as ANONYMOUS shapes: two shapes, two
/// answers, and nothing for a memo to share.
fn inline_shapes() -> String {
    format!(
        "{PREFIXES}\
         ex:Outer a sh:NodeShape ; sh:targetClass ex:Focus ;\n\
         \x20   sh:property [ sh:path ex:left ; sh:node [\n\
         \x20       a sh:NodeShape ; {INNER_CONSTRAINT}\n\
         \x20   ] ] ;\n\
         \x20   sh:property [ sh:path ex:right ; sh:node [\n\
         \x20       a sh:NodeShape ; {INNER_CONSTRAINT}\n\
         \x20   ] ] .\n"
    )
}

/// One arm of the comparison: a validator prepared over the shared data graph,
/// plus the focus id minted from THAT binding.
///
/// The id is stored beside the validator rather than shared between the arms
/// because a [`FocusId`] names the binding it was minted against and a binding's
/// identity is the retained view, not the dataset underneath it: binding the same
/// `Arc<RdfDataset>` twice yields two identities, and each arm must be driven with
/// its own.
struct Arm {
    /// The prepared validator.
    validator: PreparedValidator,
    /// The single focus node, in this binding's id space.
    focus: Vec<FocusId>,
}

impl Arm {
    /// Prepare `shapes_ttl` over `data`, resolving [`FOCUS_IRI`] against the
    /// resulting binding.
    fn prepare(data: &Arc<RdfDataset>, shapes_ttl: &str, label: &str) -> Self {
        let shapes = parse_shapes(shapes_ttl, None)
            .unwrap_or_else(|error| panic!("{label}: the shapes graph must parse: {error}"));
        let validator = PreparedShapes::new(Arc::new(shapes))
            .bind_shared_dataset(Arc::clone(data))
            .unwrap_or_else(|error| panic!("{label}: the shapes graph must bind: {error}"));
        let focus = validator
            .term_id(&NamedNode::new_unchecked(FOCUS_IRI.to_owned()).into_term())
            .unwrap_or_else(|| panic!("{label}: the focus node must be interned"));
        Self {
            validator,
            focus: vec![focus],
        }
    }

    /// Validate the focus node, requiring conformance, and return how many times the
    /// inner shape's constraint was EVALUATED while doing so.
    ///
    /// [`ASKS`] is reset immediately before the call, so nothing any earlier
    /// validation asked is charged here. No warm-up run is needed or wanted: a count
    /// of constraint evaluations has no first-touch term to warm away — the
    /// class-membership index, the plan cache and every memo underneath change how
    /// much each evaluation COSTS and not how many of them happen — and a warm-up
    /// would only double the figure this returns.
    fn asks_to_validate(&self, label: &str) -> usize {
        let _discarded = take_asks();
        let report = self
            .validator
            .validate_focus_node_ids(&self.focus)
            .unwrap_or_else(|error| panic!("{label}: the fixture must validate: {error}"));
        let asks = take_asks();
        assert!(
            report.conforms && report.results.is_empty(),
            "{label}: the fixture must CONFORM, or the figure below describes a workload that \
             never reached the inner shape ({} result(s))",
            report.results.len()
        );
        asks
    }
}

/// **The two spellings mean the same thing, and the memoizable one is asked once.**
///
/// The equality half is what makes the count half safe to assert: a memo that
/// returned a wrong answer would make the two reports disagree, and a memo that
/// never fired would make the two counts equal.
#[test]
fn a_named_inner_shape_is_evaluated_once_for_a_shared_value_node() {
    let _guard = measure_lock();
    let _relations = enter_property_function_scope(counting_relations());
    let data = parse_ntriples_to_dataset(DATA).unwrap_or_else(|errors| {
        panic!("the fixture data graph must parse: {}", errors.join("\n"))
    });
    let named_arm = Arm::prepare(&data, &named_shapes(), "named");
    let inline_arm = Arm::prepare(&data, &inline_shapes(), "inline");

    let named_report = named_arm
        .validator
        .validate_focus_node_ids(&named_arm.focus)
        .expect("the named fixture validates");
    let inline_report = inline_arm
        .validator
        .validate_focus_node_ids(&inline_arm.focus)
        .expect("the inline fixture validates");
    assert!(
        named_report.conforms && inline_report.conforms,
        "both spellings describe a conforming graph"
    );
    assert_eq!(
        named_report.results.len(),
        inline_report.results.len(),
        "the two spellings state the same constraints and must reach the same verdict"
    );

    let named = named_arm.asks_to_validate("named");
    let inline = inline_arm.asks_to_validate("inline");

    // THE CONTROL, asserted first and asserted exactly: two anonymous inner shapes
    // are two different shapes, so each site's ask runs. A reading of 0 here would
    // mean the constraint never ran at all — an unresolved function IRI, a target
    // that selected nothing — and would satisfy any mere inequality between the two
    // arms while proving nothing about a memo.
    assert_eq!(
        inline, 2,
        "spelling the inner shape out twice as anonymous shapes gives each site its own \
         shape, so the constraint must be evaluated once per site — twice. A reading of 0 \
         means the constraint never ran and this file is measuring nothing; any other \
         reading means the fixture no longer asks once per site."
    );

    // THE TREATMENT: one named shape reached from both sites, one ask.
    assert_eq!(
        named, 1,
        "naming the inner shape at both sites must let the second site reuse the first \
         site's conformance answer, so the constraint must be evaluated exactly ONCE for \
         the value node both paths lead to (got {named}, against {inline} for the same \
         constraint spelled out twice). A reading of {inline} means the second site ran the \
         inner shape's query again, which is the conformance memo not firing."
    );
}

// ── The regression guard: every test in this binary holds MEASURE_LOCK first ───

/// Whether `attrs` carries a bare `#[test]` attribute.
///
/// Matches by attribute PATH, not by scanning the source text for the word
/// "test": a doc comment or a code comment that happens to contain that word
/// must never be read as marking a function.
fn is_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("test"))
}

/// Whether `block`'s FIRST statement is a `let` binding whose initializer is a
/// call to `measure_lock()`.
///
/// Not "somewhere in the body": [`ASKS`] is one process-global counter for the
/// whole test, so a lock taken after even one ask has already let that ask land
/// unguarded. It must also be a `let` binding and not a bare `measure_lock();`
/// statement — the returned [`MutexGuard`] is a temporary that drops at the end of
/// a bare statement, which releases the lock immediately rather than holding it for
/// the test.
fn first_statement_holds_measure_lock(block: &syn::Block) -> bool {
    let Some(syn::Stmt::Local(local)) = block.stmts.first() else {
        return false;
    };
    let Some(init) = &local.init else {
        return false;
    };
    matches!(
        init.expr.as_ref(),
        syn::Expr::Call(call)
            if matches!(
                call.func.as_ref(),
                syn::Expr::Path(path) if path.path.is_ident("measure_lock")
            )
    )
}

/// Every `#[test]` function declared anywhere in the scanned file, in source
/// order.
///
/// Walks the whole file rather than only its top-level items, so a `#[test]`
/// nested inside a `mod` block cannot go unseen.
#[derive(Default)]
struct TestFns(Vec<syn::ItemFn>);

impl<'ast> syn::visit::Visit<'ast> for TestFns {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if is_test_attr(&item.attrs) {
            self.0.push(item.clone());
        }
        syn::visit::visit_item_fn(self, item);
    }
}

/// **Every `#[test]` function in this binary takes [`MEASURE_LOCK`] as the FIRST
/// statement of its body.**
///
/// This file's own documentation states that rule; this is what enforces it.
/// [`ASKS`] is one process-global counter, and `cargo test` runs a binary's test
/// functions CONCURRENTLY, so a test that validates without holding the lock —
/// whether or not it takes a reading of its own — can land its asks inside a
/// sibling test's reading and shift a measured figure nondeterministically. A
/// future test that omits the lock is exactly the contamination source this file
/// must rule out, so it fails loudly and names itself rather than showing up as an
/// occasional, unattributed shift in the figures below.
///
/// A source scan rather than a runtime check: nothing observable at runtime
/// distinguishes "this test forgot to take the lock" from "this test never needed
/// it", so the only place the distinction is visible is the source itself.
#[test]
fn every_test_in_this_binary_takes_the_measure_lock_first() {
    let _guard = measure_lock();
    let source = include_str!("conformance_memo.rs");
    let parsed = syn::parse_file(source)
        .unwrap_or_else(|error| panic!("this file must parse as Rust: {error}"));
    let mut collector = TestFns::default();
    syn::visit::Visit::visit_file(&mut collector, &parsed);

    assert!(
        !collector.0.is_empty(),
        "the scan found no #[test] function in this file at all, so this guard is reading nothing"
    );

    let mut offenders: Vec<String> = collector
        .0
        .iter()
        .filter(|item| !first_statement_holds_measure_lock(&item.block))
        .map(|item| item.sig.ident.to_string())
        .collect();
    offenders.sort();

    assert!(
        offenders.is_empty(),
        "these #[test] functions do not take MEASURE_LOCK as the first statement of their body, \
         so they can validate concurrently with another test's reading of ASKS and silently \
         shift its figure: {offenders:?}"
    );
}
