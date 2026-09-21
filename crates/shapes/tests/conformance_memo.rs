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
//! named one must allocate strictly less.
//!
//! # Why the inner shape is SPARQL-backed, and why that is not incidental
//!
//! An allocation count can only see a skipped traversal that would have
//! allocated, and on this revision a CONFORMING SHACL Core traversal allocates
//! nothing: that is the whole result `tests/change_path_alloc.rs` pins, where
//! thirty-nine constraint and path cases validate thousands of conforming focus
//! nodes for six allocations in total. A memo that skips such a traversal saves
//! real work and saves ZERO allocations, so an allocation instrument pointed at a
//! Core inner shape reads equal whether the memo fires or is deleted outright.
//! Measured: with the inner shape spelled `sh:property [ sh:path ex:count ;
//! sh:equals ex:limit ]`, both arms cost exactly 5 — and the memo was confirmed
//! by instrumentation to be HITTING in that fixture. That measurement proves
//! nothing, and a test resting on it cannot fail for the reason it states.
//!
//! [`INNER_CONSTRAINT`] is therefore a `sh:sparql` SELECT, which is the one class
//! of constraint in this crate whose conforming evaluation has a real, pinned
//! per-focus-node cost — 83 allocations for exactly this surface, per
//! `tests/sparql_path_alloc.rs`. Skipping one of those is an event an allocation
//! count can see, and the figures below are the size of one query evaluation
//! rather than of a rounding difference.
//!
//! # What the window may contain, and why it matters here
//!
//! The two shapes graphs are DIFFERENT DOCUMENTS. The named one declares one
//! shared shape and the inline one declares two inner ones, so they do not parse
//! to the same number of `Shape` values and they do not cost the same to parse or
//! to lower. A measurement that opened its window around a whole
//! `data + shapes → report` call would therefore be satisfied by the PARSE alone:
//! the named arm would read lower than the inline arm with the memo ripped out
//! entirely, and the assertion would be unfalsifiable in exactly the way this file
//! exists to prevent.
//!
//! So every parse, every lowering and every dataset binding happens OUTSIDE the
//! window. Both arms are prepared up front over ONE shared
//! [`purrdf::RdfDataset`], both mint a [`FocusId`] for the same focus node, and
//! the measured region is one [`PreparedValidator::validate_focus_node_ids`]
//! call per arm. What is left inside the window is the traversal and nothing
//! else, which is the only thing the memo can act on.
//!
//! # The figures, and the proof they can move
//!
//! Measured on this revision: the named arm costs **98** allocations and the
//! inline arm **187**. Disabling the memo — forcing the `MemoKey` construction in
//! `purrdf_shapes`' `constraints` module to `None`, so every ask recomputes and
//! nothing is ever recorded — moves the named arm to **191**, leaves the inline
//! arm at 187, and turns the assertion below red. The margin is one `sh:sparql`
//! evaluation, which is exactly what a memo hit skips.
//!
//! The counts are compared, not pinned, so a cheaper query interface moves both
//! arms without touching this file. What the assertion states is the INEQUALITY,
//! because the claim is that one ask happened instead of two.
//!
//! # The instrument
//!
//! [`WholeProcessWindow`] reads one PROCESS-GLOBAL ledger, so a sibling test
//! allocating concurrently would land its traffic inside an open window.
//! [`MEASURE_LOCK`] serializes every test in this binary, and
//! [`every_test_in_this_binary_takes_the_measure_lock_first`] enforces that by
//! reading this file's own source: a future test added here without the lock
//! must fail loudly and name itself rather than show up as an occasional,
//! unattributed shift in the figures below.
//!
//! One focus node is far below `crate::parallel::PARALLEL_MIN_FOCUS_NODES`, so
//! both arms stay on the serial path and neither `rayon`'s job injector nor the
//! `regex` crate's contended cache pool — the two third-party residuals
//! `tests/change_path_alloc.rs` documents — is on the route. The figures here are
//! therefore exact rather than floored over repetitions.
//!
//! Every IRI is under `example.org`: PurRDF mints no vocabulary IRIs.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use purrdf::RdfDataset;
use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_shapes::engine::{FocusId, PreparedShapes, PreparedValidator, parse_shapes};
use purrdf_shapes::term::NamedNode;
use purrdf_shapes::text_ingest::parse_ntriples_to_dataset;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Serializes every measured region in this binary.
///
/// [`WholeProcessWindow`] reads one process-global ledger and `cargo test` runs
/// test functions concurrently, so two measurements in flight at once would each
/// report the union of both regions while appearing to report their own.
static MEASURE_LOCK: Mutex<()> = Mutex::new(());

/// Take [`MEASURE_LOCK`], absorbing poison.
///
/// A panicking assertion inside a measured region poisons the mutex. Propagating
/// that would turn one real failure into a cascade of unrelated ones and bury the
/// diagnosis; the lock guards a counter, not an invariant that a panic could have
/// left half-written.
fn measure_lock() -> MutexGuard<'static, ()> {
    MEASURE_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
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
/// A conforming focus node produces the empty solution bag — `ex:count` is a
/// literal, so the `FILTER` admits nothing — which is the CONFORMING evaluation
/// whose cost this file relies on being non-zero. The predicate is spelled as an
/// absolute IRI so no `sh:prefixes` declaration stands between the reader and
/// what the query matches.
const INNER_CONSTRAINT: &str = concat!(
    "sh:sparql [ a sh:SPARQLConstraint ; sh:select \"\"\"\n",
    "    SELECT $this WHERE {\n",
    "      $this <http://example.org/purrdf/memo#count> ?n .\n",
    "      FILTER(!isLiteral(?n))\n",
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
    /// The prepared validator, built entirely outside any measurement window.
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

    /// Validate the focus node, requiring conformance, and return how many
    /// allocations the VALIDATION made.
    ///
    /// The measurement is taken after a warm-up run with the same arguments: the
    /// class-membership index, the thread-local SPARQL plan cache and the
    /// allocator's own arenas are first-touch lazies, and charging them to
    /// whichever call ran first would make the comparison a statement about
    /// start-up. The plan cache matters most here — the inline arm has two
    /// anonymous inner shapes and so two query bodies to compile, and an
    /// unwarmed window would charge it for that compilation on top of the second
    /// evaluation this file is trying to see. Preparation is not in the window at
    /// all — see this file's header for why that is the whole point.
    fn allocations_to_validate(&self, label: &str) -> u64 {
        let run = || {
            let report = self
                .validator
                .validate_focus_node_ids(&self.focus)
                .unwrap_or_else(|error| panic!("{label}: the fixture must validate: {error}"));
            assert!(
                report.conforms && report.results.is_empty(),
                "{label}: the fixture must CONFORM, or the figure below describes a workload that \
                 never reached the inner shape ({} result(s))",
                report.results.len()
            );
        };
        run();
        let window = WholeProcessWindow::open();
        run();
        window.close().allocations
    }
}

/// **The two spellings mean the same thing, and the memoizable one costs less.**
///
/// The equality half is what makes the inequality half safe to assert: a memo
/// that returned a wrong answer would make the two reports disagree, and a memo
/// that never fired would make the two costs equal.
#[test]
fn a_named_inner_shape_is_evaluated_once_for_a_shared_value_node() {
    let _guard = measure_lock();
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

    let named = named_arm.allocations_to_validate("named");
    let inline = inline_arm.allocations_to_validate("inline");
    assert!(
        named < inline,
        "naming the inner shape at both sites must let the second site reuse the first site's \
         conformance answer, so it must allocate strictly less than spelling the shape out twice \
         ({named} vs {inline}). Both arms were PREPARED outside the window, so the only thing \
         left inside it is the traversal: a figure that is not lower means the second site ran \
         the inner shape's query again, which is the conformance memo not firing."
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
/// Not "somewhere in the body": [`WholeProcessWindow`] reads one process-global
/// ledger for the whole test, so a lock taken after even one allocation has
/// already let that allocation land unguarded. It must also be a `let` binding
/// and not a bare `measure_lock();` statement — the returned [`MutexGuard`] is a
/// temporary that drops at the end of a bare statement, which releases the lock
/// immediately rather than holding it for the test.
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
/// This file's own documentation states that rule; this is what enforces it. A
/// binary-wide [`WholeProcessWindow`] reads one process-global ledger, and
/// `cargo test` runs a binary's test functions CONCURRENTLY, so a test that
/// allocates without holding the lock — whether or not it takes a measurement of
/// its own — can land its traffic inside a sibling test's open window and shift a
/// measured figure nondeterministically. A future test that omits the lock is
/// exactly the contamination source this file must rule out, so it fails loudly
/// and names itself rather than showing up as an occasional, unattributed shift
/// in someone else's figure.
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
         so they can allocate concurrently with another test's WholeProcessWindow and silently \
         shift its figure: {offenders:?}"
    );
}
