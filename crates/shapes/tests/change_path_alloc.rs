// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **Validating a conforming graph through the SHACL change path allocates a
//! bounded amount, independent of how many focus nodes the change touched.**
//!
//! The change path is the incremental route — [`PreparedValidator::validate_focus_node_ids`]
//! and [`PreparedValidator::validate_focus_nodes`] — that a caller drives after a
//! data change, handing the validator only the focus nodes that change could have
//! affected. Its whole reason to exist is that it is cheaper than revalidating the
//! graph. A per-focus-node allocation term destroys that: the route stays
//! asymptotically correct and quietly costs what it was built to avoid, and
//! nothing in a latency benchmark on a quiet machine says so, because allocator
//! traffic hides inside a number that also moves with cache state and scheduling.
//!
//! So the claim is stated here as an executable contract over the production
//! surfaces, in allocation COUNTS, which are facts about the code rather than
//! about the host.
//!
//! # What each test pins
//!
//! * [`conforming_change_path_allocation_has_no_growth_term_in_focus_count`] —
//!   the headline. One bound dataset, two validations of a **conforming** focus
//!   set differing only in size, through BOTH change-path entry points; the
//!   allocation deltas must be equal.
//! * [`violating_change_path_allocation_scales_with_violations_not_focus_count`]
//!   — the companion that makes the headline mean anything. A conforming-only
//!   assertion is satisfied perfectly by a validator that has stopped validating,
//!   so this one holds the violation count fixed while the conforming population
//!   doubles (cost must not move) and holds the conforming population fixed while
//!   the violation count doubles (cost must move), and checks that the results
//!   are really there.
//! * [`change_path_report_bytes_match_pinned_golden`] — the silent-drop guard.
//!   Allocation counts alone cannot distinguish "cheaper" from "no longer
//!   checking"; this pins the serialized report, byte for byte, for every
//!   constraint kind and path form the other tests drive.
//! * [`bind_allocation_is_independent_of_dataset_size_beyond_the_catalog`] and
//!   [`admit_allocation_is_independent_of_dataset_size`] — the two once-per-
//!   snapshot seams the change path sits on top of. A change path that allocates
//!   nothing per focus node is worth nothing if the bind in front of it is
//!   linear in the graph.
//!
//! `PreparedShapes::bind_dataset` is deliberately not one of the measured seams.
//! It projects the data graph into an owned snapshot before binding, so it is
//! linear in the graph by construction and always will be; asserting otherwise
//! over it would be asserting that a copy is free.
//!
//! # What holds today, and what does not
//!
//! Three of the five are red on this revision and carry `#[ignore]` with the
//! invariant they pin as the reason. They are executable specifications of work
//! not yet done, not disabled tests. Measured here:
//!
//! * the conforming change path allocates 10,254 for 2,048 focus nodes and
//!   20,498 for 4,096 on the first case — about five allocations per focus node,
//!   which is the growth term the whole exercise is about;
//! * with eight violations held fixed, the same doubling moves the figure from
//!   10,475 to 20,719 on the first case, so the violating path carries it too;
//! * binding costs 93 allocations for the seam dataset and 96 for twice its
//!   instance data, a much smaller term but the same shape of defect.
//!
//! The golden and the admit seam are green, and must stay that way.
//!
//! # NO-REBLESSING RULE
//!
//! **`fixtures/change-path-report.golden.txt` is written once, here.** It is the
//! guard against the one failure mode an allocation test cannot see: validation
//! that got cheaper by no longer producing results. A later change that finds
//! [`change_path_report_bytes_match_pinned_golden`] failing has found a SEMANTIC
//! CHANGE in SHACL validation and must stop and diagnose it. Regenerating the
//! golden to make the test pass is forbidden, and a regenerated golden is
//! indistinguishable from the bug it exists to catch.
//!
//! # The instrument, and the trap it is threaded around
//!
//! SHACL validation fans focus nodes out over `rayon` above its parallel
//! threshold, and that is precisely the regime whose allocation behaviour matters.
//! A [`purrdf_alloc_probe::CurrentThreadWindow`] cannot see a worker thread, so it
//! would report the parallel path as almost free and every assertion below would
//! be green while measuring nearly nothing. Every measurement here therefore uses
//! a [`WholeProcessWindow`], which counts every thread.
//!
//! That window reads one process-global ledger, and `cargo test` runs a binary's
//! test functions concurrently, so a second test measuring at the same time would
//! land its traffic inside the first one's figures. [`MEASURE_LOCK`] serializes
//! every measured region in this binary; each measuring test takes it first and
//! holds it for its whole body. Poisoning is absorbed rather than propagated, so
//! one failing test does not cascade into unrelated failures that hide it.
//!
//! [`FOCUS_NODES`] and its double are both above [`PARALLEL_MIN_FOCUS_NODES`], so
//! the invariant is stated over the parallel path rather than over a
//! single-threaded shadow of it, and [`assert_parallel_path_is_reachable`] refuses
//! to report a result from a build or host where that path cannot be taken.
//!
//! # Warm-up is part of the measurement
//!
//! Plenty of first-touch work in this stack allocates exactly once and would
//! otherwise be charged to whichever measurement happened to run first: `rayon`
//! builds its global thread pool lazily on the first parallel validation, the
//! regex cache and the class-membership index are populated on first use, the
//! SPARQL plan cache fills on first evaluation, and the allocator's own arenas
//! grow. Every measured region below is therefore executed once, with the same
//! arguments, BEFORE the window opens. That is why the warm-up calls look
//! redundant: they are the reason the figures are about steady-state validation
//! and not about process start-up.
//!
//! # Fixtures
//!
//! [`CASES`] is one case per SHACL Core constraint kind — including the recursive
//! ones, `sh:node`, `sh:and`, `sh:or`, `sh:xone`, `sh:not` and
//! `sh:qualifiedValueShape`, where "conforms fast" and "stopped validating" are
//! one refactor apart — plus one case per SHACL path form. Tests 1 and 2 are
//! generated over it, so a new constraint kind cannot regress the invariant
//! without a named failure saying which kind broke. Every IRI is under
//! `example.org`: PurRDF mints no vocabulary IRIs, and a test fixture is no more
//! entitled to invent one than a release build is.
//!
//! # Running it
//!
//! ```text
//! cargo test -p purrdf-shapes --test change_path_alloc
//! cargo test -p purrdf-shapes --test change_path_alloc -- --ignored
//! ```
//!
//! The `#[ignore]`d tests are the ones whose invariant the current code does not
//! yet satisfy. Their reason strings say what they pin; they are executable
//! specifications, not disabled tests, and weakening one to make it pass removes
//! the only statement of the contract this file exists for.

use std::fmt::Write as _;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use purrdf::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId};
use purrdf_alloc_probe::{CountingAllocator, Measurement, WholeProcessWindow};
use purrdf_shapes::engine::{PreparedShapes, PreparedValidator, parse_shapes};
use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::term::{NamedNode, Term};

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

/// Run `operation` inside a whole-process allocation window.
///
/// The caller is responsible for holding [`measure_lock`] and for having warmed
/// the operation first; this helper only brackets it.
fn measure<T>(operation: impl FnOnce() -> T) -> (T, Measurement) {
    let window = WholeProcessWindow::open();
    let value = operation();
    (value, window.close())
}

/// The fixture namespace. Caller-supplied and `example.org` by rule.
const NS: &str = "http://example.org/purrdf/change-path#";

/// `rdf:type`, spelled out because the fixtures are built id-natively.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// `rdfs:subClassOf`, which the seam fixture needs to reach the class-membership
/// index at all; see [`seam_dataset`].
const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

/// `xsd:integer`, for the numeric-range and datatype cases.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// The class every generated focus node carries and every generated shape targets.
const FOCUS_CLASS: &str = "Focus";

/// Mirrors `PARALLEL_MIN_FOCUS_NODES` in `crates/shapes/src/parallel.rs`.
///
/// That constant is `pub(crate)`, so an integration test cannot read it. It is
/// mirrored rather than approximated because the whole point of [`FOCUS_NODES`]
/// is to sit above it; [`assert_parallel_path_is_reachable`] is what turns the
/// mirror into a checked claim about the size relationship, and a threshold that
/// moved UP past [`FOCUS_NODES`] would show up as tests 1 and 2 suddenly
/// measuring a serial path — which is why the sizes carry a wide margin rather
/// than sitting one node above the line.
const PARALLEL_MIN_FOCUS_NODES: usize = 1_024;

/// The focus-node count `N` in the `delta(2N) == delta(N)` assertions.
///
/// Chosen so that both `N` and `2N` are comfortably above
/// [`PARALLEL_MIN_FOCUS_NODES`]: the invariant is being stated over the parallel
/// path, not over the serial path a smaller `N` would quietly select.
const FOCUS_NODES: usize = 2_048;

/// The violation count `V` in the violation-scaling assertions.
///
/// Small and fixed on purpose: it is the quantity that must drive the cost, so it
/// has to be independently variable from the focus count, and it has to stay far
/// enough below it that a per-focus-node term cannot masquerade as a per-violation
/// one.
const VIOLATIONS: usize = 8;

/// Focus nodes per dataset in the bind and admit size-independence tests.
const SEAM_FOCUS_NODES: usize = 4_096;

/// How many allocations one `bind_shared_dataset` costs over the
/// [`SEAM_FOCUS_NODES`]-focus-node seam dataset.
///
/// MEASURED on this revision, not chosen: binding that dataset makes 93
/// allocations, and binding twice as much instance data makes 96. The three extra
/// are the growth term the companion assertion in
/// [`bind_allocation_is_independent_of_dataset_size_beyond_the_catalog`] refuses,
/// which is why that test is red and this pin is not — the two assertions state
/// different things and only one of them is a claim about today.
///
/// A determinism pin, NOT a timing threshold. An allocation count is a fact about
/// the code: the same revision produces the same number on every host, under any
/// load, at any core count, because nothing in binding consults a clock, a source
/// of randomness or the scheduler. A host-sensitive figure would have no business
/// being asserted; this one has no business being merely logged.
const BIND_ALLOC_CONST: u64 = 93;

/// How many allocations one prepared-product `admit` costs.
///
/// The same kind of pin as [`BIND_ALLOC_CONST`], over the other once-per-snapshot
/// seam, and here the figure really is constant: admission makes 282 allocations
/// with either seam dataset bound.
const ADMIT_ALLOC_CONST: u64 = 282;

/// Conforming focus nodes per case in the golden fixture.
const GOLDEN_CONFORMING: usize = 2;

/// Violating focus nodes per case in the golden fixture.
const GOLDEN_VIOLATING: usize = 2;

/// The pinned report text. See the NO-REBLESSING RULE in this module's docs.
const GOLDEN: &str = include_str!("fixtures/change-path-report.golden.txt");

/// Turtle prefixes every generated shapes graph opens with.
const PREFIXES: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n",
    "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
    "@prefix ex: <http://example.org/purrdf/change-path#> .\n",
);

// ---------------------------------------------------------------------------
// The case table
// ---------------------------------------------------------------------------

/// The data-graph writer handed to a case, scoped to one focus node.
///
/// Every case builds its focus node's data through this, so a case is a few lines
/// of "what this node has" rather than a transcription of the builder API, and
/// the conforming and violating shapes of one constraint sit next to each other
/// where a reader can see that they really differ.
#[derive(Debug)]
struct Emit<'builder> {
    /// The dataset under construction.
    builder: &'builder mut RdfDatasetBuilder,
    /// The focus node this call is populating.
    focus: TermId,
    /// The focus node's local name, used to derive per-node auxiliary IRIs so
    /// that no two focus nodes share a target, parent or sibling node.
    scope: String,
    /// The focus node's ordinal, for cases whose values vary with it.
    index: usize,
}

impl Emit<'_> {
    /// Intern `{NS}{local}`.
    fn iri(&mut self, local: &str) -> TermId {
        let iri = format!("{NS}{local}");
        self.builder.intern_iri(&iri)
    }

    /// Intern an auxiliary node belonging to this focus node alone.
    fn scoped(&mut self, suffix: &str) -> TermId {
        let iri = format!("{NS}{}-{suffix}", self.scope);
        self.builder.intern_iri(&iri)
    }

    /// Intern a literal.
    fn lit(&mut self, literal: RdfLiteral) -> TermId {
        self.builder.intern_literal(literal)
    }

    /// Push `subject predicate object` into the default graph.
    fn quad(&mut self, subject: TermId, predicate: &str, object: TermId) {
        let predicate = self.iri(predicate);
        self.builder.push_quad(subject, predicate, object, None);
    }

    /// Push a triple whose subject is the focus node.
    fn prop(&mut self, predicate: &str, object: TermId) {
        let subject = self.focus;
        self.quad(subject, predicate, object);
    }

    /// Attach a simple literal to the focus node.
    fn text(&mut self, predicate: &str, value: &str) {
        let object = self.lit(RdfLiteral::simple(value));
        self.prop(predicate, object);
    }

    /// Attach an `xsd:integer` to the focus node.
    fn integer(&mut self, predicate: &str, value: i64) {
        let object = self.lit(RdfLiteral::typed(value.to_string(), XSD_INTEGER));
        self.prop(predicate, object);
    }

    /// Attach a language-tagged literal to the focus node.
    fn tagged(&mut self, predicate: &str, value: &str, language: &str) {
        let object = self.lit(RdfLiteral::language_tagged(value, language));
        self.prop(predicate, object);
    }

    /// Type `subject` as `{NS}{class}`.
    ///
    /// `rdf:type` is the one predicate that does not live under [`NS`], so it has
    /// its own method rather than a case spelling an absolute IRI inline.
    fn classify(&mut self, subject: TermId, class: &str) {
        let class = self.iri(class);
        let predicate = self.builder.intern_iri(RDF_TYPE);
        self.builder.push_quad(subject, predicate, class, None);
    }

    /// Attach a per-focus-node target node typed `ex:Target`, returning it.
    fn target(&mut self, predicate: &str) -> TermId {
        let target = self.scoped("target");
        self.classify(target, "Target");
        self.prop(predicate, target);
        target
    }
}

/// One constraint kind or path form: the shape that states it, and the data that
/// makes a focus node satisfy or break it.
struct ConstraintCase {
    /// Appears in every assertion message, so a failure says which kind broke.
    name: &'static str,
    /// The shapes graph body, appended to [`PREFIXES`].
    shapes: &'static str,
    /// Writes one focus node's data; `violating` selects which shape it takes.
    emit: fn(&mut Emit<'_>, bool),
}

impl std::fmt::Debug for ConstraintCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstraintCase")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// Every constraint kind and path form the change path has to keep cheap AND keep
/// checking.
///
/// A case is a triple of claims: the shape states one constraint, the conforming
/// branch really conforms, and the violating branch really violates. All three are
/// executed — the conforming branch by test 1, which asserts the report conforms,
/// and the violating branch by test 2, which asserts the results are produced and
/// scale, and by the golden, which pins what they say.
const CASES: &[ConstraintCase] = &[
    ConstraintCase {
        name: "class",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:ref ; sh:class ex:Target ] .",
        emit: |emit, violating| {
            if violating {
                let stray = emit.scoped("stray");
                emit.classify(stray, "Other");
                emit.prop("ref", stray);
            } else {
                emit.target("ref");
            }
        },
    },
    ConstraintCase {
        name: "datatype",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:datatype xsd:integer ] .",
        emit: |emit, violating| {
            if violating {
                emit.text("count", "not-a-number");
            } else {
                let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
                emit.integer("count", index);
            }
        },
    },
    ConstraintCase {
        name: "node_kind",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:ref ; sh:nodeKind sh:IRI ] .",
        emit: |emit, violating| {
            if violating {
                emit.text("ref", "a literal is not an IRI");
            } else {
                emit.target("ref");
            }
        },
    },
    ConstraintCase {
        name: "min_count",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .",
        emit: |emit, violating| {
            if !violating {
                let index = emit.index;
                emit.text("name", &format!("item-{index}"));
            }
        },
    },
    ConstraintCase {
        name: "max_count",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:maxCount 1 ] .",
        emit: |emit, violating| {
            let index = emit.index;
            emit.text("name", &format!("item-{index}"));
            if violating {
                emit.text("name", &format!("item-{index}-again"));
            }
        },
    },
    ConstraintCase {
        name: "min_inclusive",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:minInclusive 0 ] .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.integer("count", if violating { -1 } else { index });
        },
    },
    ConstraintCase {
        name: "max_inclusive",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:maxInclusive 1000000 ] .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.integer("count", if violating { 1_000_001 } else { index });
        },
    },
    ConstraintCase {
        name: "min_exclusive",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:minExclusive -1 ] .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.integer("count", if violating { -1 } else { index });
        },
    },
    ConstraintCase {
        name: "max_exclusive",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:maxExclusive 1000000 ] .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.integer("count", if violating { 1_000_000 } else { index });
        },
    },
    ConstraintCase {
        name: "min_length",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:minLength 4 ] .",
        emit: |emit, violating| {
            let index = emit.index;
            if violating {
                emit.text("name", "ab");
            } else {
                emit.text("name", &format!("item-{index}"));
            }
        },
    },
    ConstraintCase {
        name: "max_length",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:maxLength 32 ] .",
        emit: |emit, violating| {
            let index = emit.index;
            if violating {
                emit.text(
                    "name",
                    "this label is far longer than the thirty-two characters allowed",
                );
            } else {
                emit.text("name", &format!("item-{index}"));
            }
        },
    },
    ConstraintCase {
        name: "pattern",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:pattern \"^item-[0-9]+$\" ] .",
        emit: |emit, violating| {
            let index = emit.index;
            if violating {
                emit.text("name", "does-not-match");
            } else {
                emit.text("name", &format!("item-{index}"));
            }
        },
    },
    ConstraintCase {
        name: "language_in",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:label ; sh:languageIn ( \"en\" ) ] .",
        emit: |emit, violating| {
            if violating {
                emit.tagged("label", "bonjour", "fr");
            } else {
                emit.tagged("label", "hello", "en");
            }
        },
    },
    ConstraintCase {
        name: "unique_lang",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:label ; sh:uniqueLang true ] .",
        emit: |emit, violating| {
            emit.tagged("label", "hello", "en");
            if violating {
                emit.tagged("label", "hello again", "en");
            }
        },
    },
    ConstraintCase {
        name: "equals",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:equals ex:alias ] .",
        emit: |emit, violating| {
            let index = emit.index;
            emit.text("name", &format!("item-{index}"));
            if violating {
                emit.text("alias", &format!("other-{index}"));
            } else {
                emit.text("alias", &format!("item-{index}"));
            }
        },
    },
    ConstraintCase {
        name: "disjoint",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:disjoint ex:alias ] .",
        emit: |emit, violating| {
            let index = emit.index;
            emit.text("name", &format!("item-{index}"));
            if violating {
                emit.text("alias", &format!("item-{index}"));
            } else {
                emit.text("alias", &format!("other-{index}"));
            }
        },
    },
    ConstraintCase {
        name: "less_than",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:lessThan ex:limit ] .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.integer("count", index);
            emit.integer("limit", if violating { index } else { index + 1 });
        },
    },
    ConstraintCase {
        name: "less_than_or_equals",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:count ; sh:lessThanOrEquals ex:limit ] .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.integer("count", index);
            emit.integer("limit", if violating { index - 1 } else { index });
        },
    },
    ConstraintCase {
        name: "not",
        // The negated shape is spelled as a NODE shape wrapping a property
        // shape. Measured on this build, the anonymous PROPERTY-shape spelling
        // — `sh:not [ sh:path ex:flag ; sh:minCount 1 ]`, with or without an
        // explicit `a sh:PropertyShape` — reports a violation for EVERY focus
        // node, including nodes carrying no `ex:flag` at all, which is the
        // opposite of what SHACL states for `sh:not`. That behaviour is a
        // question about the engine, not about allocation, so the fixture uses
        // the spelling whose conforming and violating branches really are what
        // their names say; a case that reported every node as violating would
        // make the conforming half of this file untestable.
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:not [ a sh:NodeShape ; sh:property [ sh:path ex:flag ; sh:minCount 1 ] ] .",
        emit: |emit, violating| {
            if violating {
                emit.text("flag", "present");
            }
        },
    },
    ConstraintCase {
        name: "and",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:and (
                [ a sh:PropertyShape ; sh:path ex:name ; sh:minCount 1 ]
                [ a sh:PropertyShape ; sh:path ex:count ; sh:minCount 1 ]
            ) .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.text("name", "present");
            if !violating {
                emit.integer("count", index);
            }
        },
    },
    ConstraintCase {
        name: "or",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:or (
                [ a sh:PropertyShape ; sh:path ex:name ; sh:minCount 1 ]
                [ a sh:PropertyShape ; sh:path ex:count ; sh:minCount 1 ]
            ) .",
        emit: |emit, violating| {
            if !violating {
                emit.text("name", "present");
            }
        },
    },
    ConstraintCase {
        name: "xone",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:xone (
                [ a sh:PropertyShape ; sh:path ex:name ; sh:minCount 1 ]
                [ a sh:PropertyShape ; sh:path ex:count ; sh:minCount 1 ]
            ) .",
        emit: |emit, violating| {
            let index = i64::try_from(emit.index).expect("fixture indices fit in i64");
            emit.text("name", "present");
            if violating {
                emit.integer("count", index);
            }
        },
    },
    ConstraintCase {
        name: "node",
        shapes: "ex:TargetShape a sh:NodeShape ;
            sh:property [ sh:path ex:key ; sh:minCount 1 ] .
        ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:ref ; sh:node ex:TargetShape ] .",
        emit: |emit, violating| {
            let target = emit.target("ref");
            if !violating {
                let key = emit.lit(RdfLiteral::simple("present"));
                emit.quad(target, "key", key);
            }
        },
    },
    ConstraintCase {
        name: "property",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:ref ; sh:node [
                a sh:NodeShape ;
                sh:property [ sh:path ex:key ; sh:minLength 4 ]
            ] ] .",
        emit: |emit, violating| {
            let target = emit.target("ref");
            let key = emit.lit(RdfLiteral::simple(if violating { "ab" } else { "present" }));
            emit.quad(target, "key", key);
        },
    },
    ConstraintCase {
        name: "qualified_min_count",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [
                sh:path ex:ref ;
                sh:qualifiedValueShape [ a sh:PropertyShape ; sh:path ex:key ; sh:minCount 1 ] ;
                sh:qualifiedMinCount 1
            ] .",
        emit: |emit, violating| {
            let target = emit.target("ref");
            if !violating {
                let key = emit.lit(RdfLiteral::simple("present"));
                emit.quad(target, "key", key);
            }
        },
    },
    ConstraintCase {
        name: "qualified_max_count",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [
                sh:path ex:ref ;
                sh:qualifiedValueShape [ a sh:PropertyShape ; sh:path ex:key ; sh:minCount 1 ] ;
                sh:qualifiedMaxCount 1
            ] .",
        emit: |emit, violating| {
            let target = emit.target("ref");
            let key = emit.lit(RdfLiteral::simple("present"));
            emit.quad(target, "key", key);
            if violating {
                let second = emit.scoped("second");
                emit.classify(second, "Target");
                let other = emit.lit(RdfLiteral::simple("also present"));
                emit.quad(second, "key", other);
                emit.prop("ref", second);
            }
        },
    },
    ConstraintCase {
        name: "closed",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:closed true ;
            sh:ignoredProperties ( rdf:type ) ;
            sh:property [ sh:path ex:name ] .",
        emit: |emit, violating| {
            emit.text("name", "present");
            if violating {
                emit.text("stray", "not permitted by the closed shape");
            }
        },
    },
    ConstraintCase {
        name: "has_value",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:tag ; sh:hasValue \"alpha\" ] .",
        emit: |emit, violating| {
            emit.text("tag", if violating { "gamma" } else { "alpha" });
        },
    },
    ConstraintCase {
        name: "in",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:tag ; sh:in ( \"alpha\" \"beta\" ) ] .",
        emit: |emit, violating| {
            emit.text("tag", if violating { "gamma" } else { "alpha" });
        },
    },
    ConstraintCase {
        name: "path_predicate",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .",
        emit: |emit, violating| {
            if !violating {
                emit.text("name", "present");
            }
        },
    },
    ConstraintCase {
        name: "path_inverse",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path [ sh:inversePath ex:child ] ; sh:minCount 1 ] .",
        emit: |emit, violating| {
            if !violating {
                let parent = emit.scoped("parent");
                let focus = emit.focus;
                emit.quad(parent, "child", focus);
            }
        },
    },
    ConstraintCase {
        name: "path_sequence",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path ( ex:ref ex:key ) ; sh:minCount 1 ] .",
        emit: |emit, violating| {
            let target = emit.target("ref");
            if !violating {
                let key = emit.lit(RdfLiteral::simple("present"));
                emit.quad(target, "key", key);
            }
        },
    },
    ConstraintCase {
        name: "path_alternative",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [
                sh:path [ sh:alternativePath ( ex:name ex:alias ) ] ;
                sh:minCount 1
            ] .",
        emit: |emit, violating| {
            if !violating {
                emit.text("alias", "present");
            }
        },
    },
    ConstraintCase {
        name: "path_zero_or_more",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path [ sh:zeroOrMorePath ex:next ] ; sh:nodeKind sh:IRI ] .",
        emit: |emit, violating| {
            // A zero-or-more path always reaches the focus node itself, so the
            // constraint has to be one the REACHED nodes can break: a literal
            // successor is what makes this violate rather than a missing one.
            if violating {
                emit.text("next", "a literal is not an IRI");
            } else {
                emit.target("next");
            }
        },
    },
    ConstraintCase {
        name: "path_one_or_more",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path [ sh:oneOrMorePath ex:next ] ; sh:minCount 1 ] .",
        emit: |emit, violating| {
            if !violating {
                emit.target("next");
            }
        },
    },
    ConstraintCase {
        name: "path_zero_or_one",
        shapes: "ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
            sh:property [ sh:path [ sh:zeroOrOnePath ex:next ] ; sh:nodeKind sh:IRI ] .",
        emit: |emit, violating| {
            // Same reason as `path_zero_or_more`: the zero-length leg always
            // matches, so only a bad successor can break it.
            if violating {
                emit.text("next", "a literal is not an IRI");
            } else {
                emit.target("next");
            }
        },
    },
];

// ---------------------------------------------------------------------------
// Fixture construction
// ---------------------------------------------------------------------------

/// A case's dataset, its prepared validator, and the focus ids to drive it with.
#[derive(Debug)]
struct CaseFixture {
    /// The bound validator; the change path under measurement hangs off this.
    validator: PreparedValidator,
    /// Focus nodes that satisfy the case's shape, in construction order.
    conforming: Vec<TermId>,
    /// Focus nodes that break it, in construction order.
    violating: Vec<TermId>,
    /// The same conforming population in the term key space, for the term-keyed
    /// change path. Built here so that constructing the argument is never part of
    /// a measured region.
    conforming_terms: Vec<Term>,
}

/// Build and bind one case's dataset.
///
/// Conforming and violating focus nodes live in the SAME dataset, because the
/// change path validates only the ids it is handed: keeping both populations in
/// one graph is what lets the violation count and the focus count be varied
/// independently without rebuilding or rebinding anything, which in turn is what
/// keeps the two figures each test compares measurements of one preparation
/// rather than of two.
fn build_case(case: &ConstraintCase, conforming: usize, violating: usize) -> CaseFixture {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let focus_class = builder.intern_iri(&format!("{NS}{FOCUS_CLASS}"));

    let mut conforming_names = Vec::with_capacity(conforming);
    let mut violating_names = Vec::with_capacity(violating);
    for (prefix, count, names) in [
        ('c', conforming, &mut conforming_names),
        ('v', violating, &mut violating_names),
    ] {
        for index in 0..count {
            let scope = format!("{prefix}{index}");
            let focus = builder.intern_iri(&format!("{NS}{scope}"));
            builder.push_quad(focus, rdf_type, focus_class, None);
            let mut emit = Emit {
                builder: &mut builder,
                focus,
                scope: scope.clone(),
                index,
            };
            (case.emit)(&mut emit, prefix == 'v');
            names.push(scope);
        }
    }

    let dataset = builder
        .freeze()
        .unwrap_or_else(|error| panic!("case {} must freeze: {error}", case.name));
    let shapes = parse_shapes(&format!("{PREFIXES}{}", case.shapes), None)
        .unwrap_or_else(|error| panic!("case {} shapes must parse: {error}", case.name));
    let validator = PreparedShapes::new(Arc::new(shapes))
        .bind_shared_dataset(Arc::clone(&dataset))
        .unwrap_or_else(|error| panic!("case {} must bind: {error}", case.name));

    let resolve = |names: &[String]| -> Vec<TermId> {
        names
            .iter()
            .map(|scope| {
                dataset
                    .term_id_by_iri(&format!("{NS}{scope}"))
                    .unwrap_or_else(|| panic!("case {} focus {scope} must be interned", case.name))
            })
            .collect()
    };
    CaseFixture {
        conforming: resolve(&conforming_names),
        violating: resolve(&violating_names),
        conforming_terms: conforming_names
            .iter()
            .map(|scope| NamedNode::new_unchecked(format!("{NS}{scope}")).into_term())
            .collect(),
        validator,
    }
}

impl CaseFixture {
    /// A focus set of `violations` violating ids followed by `conforming`
    /// conforming ones.
    ///
    /// The two populations are dimensioned INDEPENDENTLY rather than as a share
    /// of one total, and that is load-bearing. Swapping conforming nodes for
    /// violating ones to keep a total fixed changes two things at once, and the
    /// conforming and violating branches of a case do not cost the same: measured
    /// on this build, exchanging eight conforming `pattern` nodes for eight
    /// violating ones LOWERS the allocation count, because a name that fails the
    /// regex at its first character is cheaper to reject than a matching one is to
    /// accept. An assertion that "more violations cost more" written over that
    /// swap is simply false, and would have been softened into meaninglessness to
    /// make it pass. Holding the conforming population identical and ADDING
    /// violating nodes makes the difference attributable to the violations and
    /// nothing else.
    fn focus_set(&self, violations: usize, conforming: usize) -> Vec<TermId> {
        let mut ids = Vec::with_capacity(violations + conforming);
        ids.extend_from_slice(&self.violating[..violations]);
        ids.extend_from_slice(&self.conforming[..conforming]);
        ids
    }
}

/// The shapes graph both once-per-snapshot seam tests use.
///
/// Deliberately more than one shape and more than one class: the bind test's
/// claim is that binding costs the CLASS CATALOG and not the graph, and a
/// single-class catalog would let a figure that was really "one class" pass for
/// "constant".
fn seam_shapes_source() -> String {
    format!(
        "{PREFIXES}
ex:FocusShape a sh:NodeShape ; sh:targetClass ex:Focus ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] ;
    sh:property [ sh:path ex:count ; sh:datatype xsd:integer ] ;
    sh:property [ sh:path ex:ref ; sh:class ex:Target ] .

ex:TargetShape a sh:NodeShape ; sh:targetClass ex:Target ;
    sh:property [ sh:path ex:key ; sh:minCount 1 ] .
"
    )
}

/// One focus node in every `SEAM_VIOLATION_PERIOD` omits `ex:name`.
///
/// A seam fixture where everything conformed would let a binding or an admission
/// that produced a HUSK — a validator holding no shapes, a preparation that lost
/// its constraints — report exactly what a working one reports, and "cheap" and
/// "empty" would be the same measurement. Both seam tests check the violation
/// count outside their windows for that reason.
///
/// The period divides both seam sizes, so the violating population doubles with
/// the graph and the quad count still doubles exactly.
const SEAM_VIOLATION_PERIOD: usize = 512;

/// How many results a seam dataset of `focus_nodes` focus nodes must produce.
const fn seam_violations(focus_nodes: usize) -> usize {
    focus_nodes / SEAM_VIOLATION_PERIOD
}

/// How many classes the seam fixture's subclass hierarchy holds.
///
/// This IS "the catalog" the bind test's name excludes: it is the part of the
/// data graph that describes the schema rather than the instances, it is the same
/// size at both measured sizes, and binding is allowed to cost it.
const SEAM_HIERARCHY_CLASSES: usize = 8;

/// The seam fixture's fixed schema preamble, in quads.
const SEAM_HIERARCHY_QUADS: usize = SEAM_HIERARCHY_CLASSES;

/// A data graph of `SEAM_HIERARCHY_QUADS + 6 * focus_nodes -
/// seam_violations(focus_nodes)` quads.
///
/// Outside the fixed hierarchy nothing is shared between focus nodes — no shared
/// target, no shared literal — so doubling `focus_nodes` doubles the instance
/// data exactly, and the size-independence tests assert that rather than assume
/// it.
///
/// # Why there is a subclass hierarchy at all
///
/// The class-membership index is the most graph-proportional thing binding does,
/// and it short-circuits to nothing when the data graph interns no
/// `rdfs:subClassOf` — so a seam fixture with a flat type hierarchy never reaches
/// it and reports a constant bind cost by DODGING the work rather than by not
/// doing it. A fixture that passes because it avoided the expensive path is worse
/// than no fixture: it says the invariant holds when nobody checked. Measured on
/// this build, the flat variant binds in 6 allocations at every size, and the
/// variant below in tens — the whole question the test asks lives in the
/// difference.
///
/// The hierarchy is FIXED at [`SEAM_HIERARCHY_CLASSES`] while the instances scale,
/// which is what makes "independent of dataset size beyond the catalog" the
/// claim being tested. A fixture that gave every focus node its own class would
/// scale the schema along with the data and could then only report that binding
/// costs something per class, which nobody disputes.
fn seam_dataset(focus_nodes: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(RDF_TYPE);
    let subclass_of = builder.intern_iri(RDFS_SUBCLASS_OF);
    let focus_class = builder.intern_iri(&format!("{NS}{FOCUS_CLASS}"));
    let target_class = builder.intern_iri(&format!("{NS}Target"));
    let name = builder.intern_iri(&format!("{NS}name"));
    let count = builder.intern_iri(&format!("{NS}count"));
    let reference = builder.intern_iri(&format!("{NS}ref"));
    let key = builder.intern_iri(&format!("{NS}key"));

    let subclasses: Vec<_> = (0..SEAM_HIERARCHY_CLASSES)
        .map(|class| builder.intern_iri(&format!("{NS}seam-k{class}")))
        .collect();
    for &subclass in &subclasses {
        builder.push_quad(subclass, subclass_of, focus_class, None);
    }

    for index in 0..focus_nodes {
        let focus = builder.intern_iri(&format!("{NS}seam-c{index}"));
        let target = builder.intern_iri(&format!("{NS}seam-t{index}"));
        let number = builder.intern_literal(RdfLiteral::typed(index.to_string(), XSD_INTEGER));
        let target_key = builder.intern_literal(RdfLiteral::simple(format!("key-{index}")));
        builder.push_quad(
            focus,
            rdf_type,
            subclasses[index % SEAM_HIERARCHY_CLASSES],
            None,
        );
        if index % SEAM_VIOLATION_PERIOD != 0 {
            let label = builder.intern_literal(RdfLiteral::simple(format!("item-{index}")));
            builder.push_quad(focus, name, label, None);
        }
        builder.push_quad(focus, count, number, None);
        builder.push_quad(focus, reference, target, None);
        builder.push_quad(target, rdf_type, target_class, None);
        builder.push_quad(target, key, target_key, None);
    }
    builder.freeze().expect("the seam dataset must freeze")
}

/// N crosses the parallel threshold, checked when the file compiles.
const _: () = assert!(
    FOCUS_NODES > PARALLEL_MIN_FOCUS_NODES,
    "N must exceed the parallel threshold, or the invariant is stated over the serial path"
);

/// And so does 2N, or the two measurements would be of two different schedulers.
const _: () = assert!(
    2 * FOCUS_NODES > PARALLEL_MIN_FOCUS_NODES,
    "2N must exceed the parallel threshold too, or the two measurements are of different code"
);

/// Refuse to report a change-path figure from a run where the parallel path
/// cannot be taken.
///
/// Both sizes under measurement are above the threshold at compile time, but a
/// single-threaded `rayon` pool keeps validation serial whatever the sizes are,
/// and a serial figure asserted under a parallel test name is exactly the
/// "green because it measured almost nothing" failure this file is built to
/// avoid. That is a property of the host, so it is checked at run time.
fn assert_parallel_path_is_reachable() {
    assert!(
        rayon::current_num_threads() > 1,
        "SHACL validation stays serial on a single-threaded rayon pool, so this host cannot \
         exercise the parallel change path these assertions are written about"
    );
}

/// Require a bound validator to actually validate the seam dataset it is bound
/// to, producing exactly the violations that dataset carries.
///
/// Both seam tests assert an allocation count is CONSTANT. A validator that bound
/// no shapes, or a preparation that was restored without its constraints, would
/// satisfy a constant just as well as a correct one — and more easily. This is
/// what separates the two, and it runs outside every measurement window so it
/// costs the figures nothing.
fn assert_bound_validator_is_live(validator: &PreparedValidator, focus_nodes: usize) {
    let expected = seam_violations(focus_nodes);
    let report = validator.validate().expect("the seam dataset validates");
    assert_eq!(
        report.results.len(),
        expected,
        "a seam dataset of {focus_nodes} focus nodes must produce {expected} results; {} means \
         the binding under measurement is not the binding the assertions describe",
        report.results.len()
    );
}

/// Which of the two change-path entry points a measurement drives.
///
/// There are two, they are documented as twins, and only one of them is named in
/// most of the discussion of this work — which is exactly the situation in which
/// one of them quietly keeps a cost the other sheds. Both are measured.
#[derive(Clone, Copy, Debug)]
enum ChangePath {
    /// [`PreparedValidator::validate_focus_node_ids`] — the id-native route.
    Ids,
    /// [`PreparedValidator::validate_focus_nodes`] — the term-keyed route.
    Terms,
}

impl ChangePath {
    /// The name that appears in an assertion message.
    const fn label(self) -> &'static str {
        match self {
            Self::Ids => "validate_focus_node_ids",
            Self::Terms => "validate_focus_nodes",
        }
    }
}

/// Validate the first `focus_nodes` conforming nodes of `fixture` through
/// `path`, requiring the report to conform.
///
/// Both argument slices are prepared by [`build_case`], so the only thing inside
/// a window built around this call is the validation itself.
fn validate_conforming(
    fixture: &CaseFixture,
    path: ChangePath,
    focus_nodes: usize,
    case: &str,
) -> ValidationReport {
    let report = match path {
        ChangePath::Ids => fixture
            .validator
            .validate_focus_node_ids(&fixture.conforming[..focus_nodes]),
        ChangePath::Terms => fixture
            .validator
            .validate_focus_nodes(&fixture.conforming_terms[..focus_nodes]),
    }
    .unwrap_or_else(|error| panic!("case {case}/{} must validate: {error}", path.label()));
    assert!(
        report.conforms,
        "case {case}/{}: the conforming population must conform, or the allocation figure below \
         describes a workload that never reached the constraint ({} result(s))",
        path.label(),
        report.results.len()
    );
    report
}

// ---------------------------------------------------------------------------
// 1. The headline
// ---------------------------------------------------------------------------

/// **Validating a conforming focus set costs the same whether it holds N nodes or
/// 2N.**
///
/// One dataset, one binding, two validations of the same production surface
/// differing only in how many conforming focus nodes they are handed. Equality is
/// asserted EXACTLY: an allocation count is a fact about the code — measured
/// eight times in a row on this build, the same validation reports the same
/// figure every time — so a change path with no per-focus-node allocation term
/// produces the same number twice, and any tolerance here would be a place for
/// exactly the term this test exists to forbid to hide.
///
/// Both change-path entry points are driven, and the failure message names which.
#[test]
#[ignore = "the change path must allocate the same amount for N and 2N conforming focus nodes, on \
            both the id-native and the term-keyed entry point; un-ignored once focus \
            materialization is deferred"]
fn conforming_change_path_allocation_has_no_growth_term_in_focus_count() {
    let _guard = measure_lock();
    assert_parallel_path_is_reachable();

    for case in CASES {
        let fixture = build_case(case, 2 * FOCUS_NODES, 0);
        for path in [ChangePath::Ids, ChangePath::Terms] {
            // Warm-up, OUTSIDE every window and with the exact arguments measured
            // below: rayon's global pool, the regex cache, the class-membership
            // index and the SPARQL plan cache are all first-touch lazies, and
            // charging them to whichever measurement ran first would make the
            // comparison a statement about start-up rather than about focus-node
            // count.
            drop(validate_conforming(&fixture, path, FOCUS_NODES, case.name));
            drop(validate_conforming(
                &fixture,
                path,
                2 * FOCUS_NODES,
                case.name,
            ));

            let (half_report, half_measured) =
                measure(|| validate_conforming(&fixture, path, FOCUS_NODES, case.name));
            drop(half_report);
            let (full_report, full_measured) =
                measure(|| validate_conforming(&fixture, path, 2 * FOCUS_NODES, case.name));
            drop(full_report);

            assert_eq!(
                half_measured.allocations,
                full_measured.allocations,
                "case {}/{}: the change path allocated {} for {FOCUS_NODES} conforming focus \
                 nodes and {} for {}, so its cost carries a growth term in the focus \
                 count\n  N  = {half_measured:?}\n  2N = {full_measured:?}",
                case.name,
                path.label(),
                half_measured.allocations,
                full_measured.allocations,
                2 * FOCUS_NODES,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 2. The companion that makes the headline mean anything
// ---------------------------------------------------------------------------

/// **Validating a violating focus set costs what the violations cost, not what
/// the focus count costs.**
///
/// [`conforming_change_path_allocation_has_no_growth_term_in_focus_count`] is
/// satisfied exactly as well by a validator that has stopped validating as by one
/// that got cheaper, so it cannot stand alone. This test drives every case's
/// VIOLATING branch and asserts three things:
///
/// 1. the results are really produced, and doubling the violating population
///    doubles them — a validator that quietly skipped a constraint fails here
///    first;
/// 2. holding the violation count fixed while doubling the CONFORMING population
///    does not move the allocation count, which is the same invariant as test 1
///    stated over a non-vacuous workload;
/// 3. holding the conforming population fixed while doubling the VIOLATING one
///    DOES move it, which is what says the remaining cost is the reporting work
///    and not a constant that stopped tracking anything.
///
/// The two populations are varied one at a time and never traded against each
/// other; [`CaseFixture::focus_set`] says why that distinction is not cosmetic.
#[test]
#[ignore = "with the violation count held fixed the change path must cost the same for N and 2N \
            conforming focus nodes, and must still scale with violations; un-ignored once focus \
            materialization is deferred"]
fn violating_change_path_allocation_scales_with_violations_not_focus_count() {
    let _guard = measure_lock();
    assert_parallel_path_is_reachable();

    for case in CASES {
        let fixture = build_case(case, 2 * FOCUS_NODES, 2 * VIOLATIONS);
        let few_small = fixture.focus_set(VIOLATIONS, FOCUS_NODES);
        let few_large = fixture.focus_set(VIOLATIONS, 2 * FOCUS_NODES);
        let many_small = fixture.focus_set(2 * VIOLATIONS, FOCUS_NODES);

        let validate = |ids: &[TermId]| {
            fixture
                .validator
                .validate_focus_node_ids(ids)
                .unwrap_or_else(|error| panic!("case {} must validate: {error}", case.name))
        };

        // Warm-up for all three regions, outside every window; see test 1.
        drop(validate(&few_small));
        drop(validate(&few_large));
        drop(validate(&many_small));

        let (few_small_report, few_small_measured) = measure(|| validate(&few_small));
        let (few_large_report, few_large_measured) = measure(|| validate(&few_large));
        let (many_small_report, many_small_measured) = measure(|| validate(&many_small));

        let few = few_small_report.results.len();
        let many = many_small_report.results.len();
        assert!(
            few > 0,
            "case {}: the violating population produced no results at all, so every allocation \
             figure in this file is describing a validator that is not validating",
            case.name
        );
        assert!(
            !few_small_report.conforms && !few_large_report.conforms,
            "case {}: a focus set containing {VIOLATIONS} violating nodes reported conformance",
            case.name
        );
        assert_eq!(
            few_large_report.results.len(),
            few,
            "case {}: adding conforming focus nodes changed the result count",
            case.name
        );
        assert_eq!(
            many,
            2 * few,
            "case {}: doubling the violating population must double the results ({few} -> {many})",
            case.name
        );

        assert_eq!(
            few_small_measured.allocations,
            few_large_measured.allocations,
            "case {}: with {VIOLATIONS} violations held fixed, the change path allocated {} for \
             {FOCUS_NODES} conforming focus nodes and {} for {}, so its cost still tracks the \
             focus count\n  N  = {few_small_measured:?}\n  2N = {few_large_measured:?}",
            case.name,
            few_small_measured.allocations,
            few_large_measured.allocations,
            2 * FOCUS_NODES,
        );
        assert!(
            many_small_measured.allocations > few_small_measured.allocations,
            "case {}: doubling the violations from {VIOLATIONS} to {} did not increase the \
             allocation count ({} -> {}), which means the reporting cost has stopped tracking \
             the results it is supposed to be building\n  V  = {few_small_measured:?}\n  2V = {many_small_measured:?}",
            case.name,
            2 * VIOLATIONS,
            few_small_measured.allocations,
            many_small_measured.allocations,
        );
    }
}

// ---------------------------------------------------------------------------
// 3. The silent-drop guard
// ---------------------------------------------------------------------------

/// **The change path's report text is pinned, byte for byte, for every constraint
/// kind and path form.**
///
/// See the NO-REBLESSING RULE in this module's documentation. This test takes no
/// allocation measurement and needs no lock; it exists so that "the change path
/// got cheaper" can never be satisfied by "the change path stopped saying
/// anything".
#[test]
fn change_path_report_bytes_match_pinned_golden() {
    let rendered = render_golden();
    assert_eq!(
        rendered, GOLDEN,
        "the SHACL change path's report text changed. This is a SEMANTIC CHANGE, not a golden \
         that needs refreshing: diagnose what moved. Regenerating \
         fixtures/change-path-report.golden.txt is forbidden."
    );
}

/// Render every case's change-path report into the golden's exact text.
fn render_golden() -> String {
    let mut rendered = String::with_capacity(64 * 1024);
    for case in CASES {
        let fixture = build_case(case, GOLDEN_CONFORMING, GOLDEN_VIOLATING);
        let ids = fixture.focus_set(GOLDEN_VIOLATING, GOLDEN_CONFORMING);
        let report = fixture
            .validator
            .validate_focus_node_ids(&ids)
            .unwrap_or_else(|error| panic!("case {} must validate: {error}", case.name));
        assert!(
            !report.conforms && !report.results.is_empty(),
            "case {}: the golden fixture must produce results, or it pins nothing",
            case.name
        );
        writeln!(rendered, "### case {}", case.name).expect("writing to a String cannot fail");
        writeln!(rendered, "### results {}", report.results.len())
            .expect("writing to a String cannot fail");
        rendered.push_str(&report.to_ntriples());
        rendered.push('\n');
    }

    // The term-keyed change path is the same validation reached through a
    // different key space, so it belongs in the same pin: a refactor that moved
    // the two apart would otherwise only be caught by whichever one had a test.
    let case = &CASES[0];
    let fixture = build_case(case, GOLDEN_CONFORMING, GOLDEN_VIOLATING);
    let terms: Vec<_> = (0..GOLDEN_VIOLATING)
        .map(|index| NamedNode::new_unchecked(format!("{NS}v{index}")).into_term())
        .collect();
    let report = fixture
        .validator
        .validate_focus_nodes(&terms)
        .expect("the term-keyed change path must validate");
    writeln!(rendered, "### case {}/validate_focus_nodes", case.name)
        .expect("writing to a String cannot fail");
    writeln!(rendered, "### results {}", report.results.len())
        .expect("writing to a String cannot fail");
    rendered.push_str(&report.to_ntriples());
    rendered.push('\n');
    rendered
}

// ---------------------------------------------------------------------------
// 4 and 5. The once-per-snapshot seams
// ---------------------------------------------------------------------------

/// **Binding a dataset costs the class catalog, not the graph.**
///
/// The change path's whole value is that a caller binds once and then validates
/// small focus sets many times. A bind that is linear in the graph moves the cost
/// rather than removing it, and it does so invisibly, because the bind happens
/// once and nobody times it.
///
/// [`BIND_ALLOC_CONST`] pins the constant term. It is a DETERMINISM pin, not a
/// timing threshold: an allocation count is a fact about the code, reproducible on
/// any host at any core count, so asserting it costs nothing in flakiness and buys
/// a build failure the day binding starts allocating per triple.
#[test]
#[ignore = "binding must cost the same for M and 2M triples of instance data behind one fixed \
            class hierarchy; un-ignored once the per-snapshot work stops scaling with the graph"]
fn bind_allocation_is_independent_of_dataset_size_beyond_the_catalog() {
    let _guard = measure_lock();

    let shapes = Arc::new(parse_shapes(&seam_shapes_source(), None).expect("seam shapes parse"));
    let prepared = PreparedShapes::new(shapes);
    let small = seam_dataset(SEAM_FOCUS_NODES);
    let large = seam_dataset(2 * SEAM_FOCUS_NODES);
    assert_eq!(
        large.quad_count() - SEAM_HIERARCHY_QUADS,
        2 * (small.quad_count() - SEAM_HIERARCHY_QUADS),
        "the two seam datasets must hold exactly twice the instance data, or 'independent of \
         dataset size' is being asserted over sizes that are not what the names say. The fixed \
         {SEAM_HIERARCHY_QUADS}-quad hierarchy is excluded because it IS the catalog this test's \
         name sets aside: it is identical in both, so it cannot be what a difference measures."
    );

    // Warm-up, outside every window: first-touch lazies and allocator arenas.
    // Deliberately over DIFFERENT dataset instances of the same two sizes. Warming
    // with the very snapshots about to be measured would leave any per-snapshot
    // memoization already populated, and the test would then be reporting a cache
    // hit twice instead of what binding a fresh snapshot costs.
    drop(prepared.bind_shared_dataset(seam_dataset(SEAM_FOCUS_NODES)));
    drop(prepared.bind_shared_dataset(seam_dataset(2 * SEAM_FOCUS_NODES)));

    let (small_bound, small_measured) =
        measure(|| prepared.bind_shared_dataset(Arc::clone(&small)));
    let small_bound = small_bound.expect("the small seam dataset binds");
    let (large_bound, large_measured) =
        measure(|| prepared.bind_shared_dataset(Arc::clone(&large)));
    let large_bound = large_bound.expect("the large seam dataset binds");

    // Non-vacuity, outside every window: a bind that produced a husk would be
    // beautifully constant too.
    assert_bound_validator_is_live(&small_bound, SEAM_FOCUS_NODES);
    assert_bound_validator_is_live(&large_bound, 2 * SEAM_FOCUS_NODES);

    assert_eq!(
        small_measured.allocations, BIND_ALLOC_CONST,
        "bind's pinned figure moved from {BIND_ALLOC_CONST} to {}; this is a determinism pin, so \
         the question is what changed in binding, not what the number should be now",
        small_measured.allocations,
    );
    assert_eq!(
        small_measured.allocations,
        large_measured.allocations,
        "bind allocated {} for {} quads and {} for {}, so binding carries a term in the instance \
         count\n  M  = {small_measured:?}\n  2M = {large_measured:?}",
        small_measured.allocations,
        small.quad_count(),
        large_measured.allocations,
        large.quad_count(),
    );
}

/// **Admitting a prepared product costs the product, not the dataset.**
///
/// `admit` takes no dataset — it restores a preparation from carried bytes — so
/// this reads as a tautology, and that is exactly what it is pinning. The seam is
/// one refactor away from folding dataset-derived state (a resolved class table, a
/// membership index) into admission, at which point the once-per-preparation cost
/// silently becomes a once-per-snapshot cost and the product stops being the cheap
/// path it is sold as. The measurement is therefore taken with a bound validator
/// over each dataset size held live, so a figure that started depending on the
/// data in scope would move.
#[test]
fn admit_allocation_is_independent_of_dataset_size() {
    let _guard = measure_lock();

    let shapes = Arc::new(parse_shapes(&seam_shapes_source(), None).expect("seam shapes parse"));
    let prepared = PreparedShapes::new(shapes);
    let product = prepared
        .to_product(&ShapesProfile::CORE)
        .expect("the seam shapes graph encodes as a product");
    let host = HostBindings::empty();

    let mut measured = Vec::with_capacity(2);
    for focus_nodes in [SEAM_FOCUS_NODES, 2 * SEAM_FOCUS_NODES] {
        let data = seam_dataset(focus_nodes);
        let bound = prepared
            .bind_shared_dataset(Arc::clone(&data))
            .expect("the seam dataset binds");
        // Warm-up, outside the window. `admit` consumes its view, so the warm-up
        // and the measured call each open their own; both opens stay outside the
        // window, so the figure is admission and not admission plus the
        // structural tier.
        let warm = ShapesProduct::open(&product).expect("the product opens");
        drop(warm.admit(&ShapesProfile::CORE, &host));

        let view = ShapesProduct::open(&product).expect("the product opens");
        let (admitted, sample) = measure(|| view.admit(&ShapesProfile::CORE, &host));
        let admitted = admitted.expect("the product admits");
        measured.push(sample);

        // Non-vacuity, outside the window: the admitted preparation has to be able
        // to do the job the original one does, or a constant admission cost is
        // just the cost of restoring nothing.
        let restored = admitted
            .bind_shared_dataset(Arc::clone(&data))
            .expect("the admitted preparation binds");
        assert_bound_validator_is_live(&restored, focus_nodes);
        drop(bound);
    }

    let (small, large) = (measured[0], measured[1]);
    assert_eq!(
        small.allocations,
        large.allocations,
        "admit allocated {} with a {SEAM_FOCUS_NODES}-focus-node snapshot bound and {} with a \
         {}-focus-node one, so admission has started depending on the data\n  M  = {small:?}\n  2M = {large:?}",
        small.allocations,
        large.allocations,
        2 * SEAM_FOCUS_NODES,
    );
    assert_eq!(
        small.allocations, ADMIT_ALLOC_CONST,
        "admit's constant term moved from {ADMIT_ALLOC_CONST} to {}; this is a determinism pin, so \
         the question is what changed in admission, not what the number should be now",
        small.allocations,
    );
}
