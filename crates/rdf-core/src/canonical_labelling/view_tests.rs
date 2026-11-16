// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The comparison entry over a dataset view: how each row of a view becomes a
//! line, what a record beside a view does, the typed outcomes that are not a
//! completed result, and the label spelling.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use purrdf_lex::allocation::{Ceiling, Memory, Resident, StorageError};
use purrdf_testkit::error_capture::CallerRefusal;

use super::reference;
use super::{
    InputReason, Labelled, Limit, Line, LineSet, Record, Refusal, Term, ViewError, admit,
    compare_view, compare_view_with_memory, label, lines_of_view, write_label,
};
use crate::{
    BlankScope, CanonicalizationControl, DatasetView, GraphMatch, QuadIds, RdfDataset,
    RdfDatasetBuilder, RdfStoreCapabilities, SearchPolicy, StopSignal, TermId, TermRef, TermValue,
};

/// `<x>` is the IRI `http://example.org/x`; any other name is a blank node of
/// the default scope.
fn iri_of(name: &str) -> Option<String> {
    name.strip_prefix('<')
        .and_then(|n| n.strip_suffix('>'))
        .map(|n| format!("http://example.org/{n}"))
}

/// The term of `name` as a typed line sees a view's term: a blank node's source
/// identifier is its scope ordinal, four bytes big-endian, then its label.
fn line_term(name: &str) -> Term {
    iri_of(name).map_or_else(
        || {
            let mut source = 0u32.to_be_bytes().to_vec();
            source.extend_from_slice(name.as_bytes());
            Term::Blank(source.into())
        },
        |iri| Term::iri(iri.as_bytes()),
    )
}

fn value_of(name: &str) -> TermValue {
    iri_of(name).map_or_else(
        || TermValue::Blank {
            label: name.to_owned(),
            scope: BlankScope::DEFAULT,
        },
        TermValue::Iri,
    )
}

fn id_of(builder: &mut RdfDatasetBuilder, name: &str) -> TermId {
    match iri_of(name) {
        Some(iri) => builder.intern_iri(&iri),
        None => builder.intern_blank(name, BlankScope::DEFAULT),
    }
    .expect("interns")
}

/// A dataset of the quads `(s, p, o)`, all in the default graph.
fn dataset(quads: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for &(s, p, o) in quads {
        let (s, p, o) = (
            id_of(&mut builder, s),
            id_of(&mut builder, p),
            id_of(&mut builder, o),
        );
        builder.push_quad(s, p, o, None).expect("pushes");
    }
    builder.freeze().expect("freezes")
}

fn line_quad(s: &str, p: &str, o: &str, graph: Option<&str>) -> Line {
    Line::Quad {
        subject: line_term(s),
        predicate: line_term(p),
        object: line_term(o),
        graph: graph.map(line_term),
    }
}

/// The directed 4-cycle `a -> b -> c -> d -> a`.
fn cycle() -> Arc<RdfDataset> {
    dataset(&[
        ("a", "<p>", "b"),
        ("b", "<p>", "c"),
        ("c", "<p>", "d"),
        ("d", "<p>", "a"),
    ])
}

/// The labelling of `view` with one record `m` on each named node.
fn label_with_records<D: DatasetView>(view: &D, on: &[&str]) -> (LineSet, Labelled) {
    let terms: Vec<[TermValue; 1]> = on.iter().map(|n| [value_of(n)]).collect();
    let records: Vec<Record<'_>> = terms.iter().map(|t| Record::new(b"m", t)).collect();
    let set = lines_of_view(view, &records).expect("well formed");
    let labelled = reference::label_set(&set, reference::ENGINE_LIMIT, false).expect("labelled");
    (set, labelled)
}

/// A view over a resident dataset that can fail to read and can repeat the
/// rows of its annotation table.
struct Probe {
    inner: Arc<RdfDataset>,
    fault: bool,
    annotation_rows: usize,
}

impl Probe {
    fn over(inner: Arc<RdfDataset>) -> Self {
        Self {
            inner,
            fault: false,
            annotation_rows: 1,
        }
    }
}

const FAULT: CallerRefusal = CallerRefusal { limit: 23 };

impl DatasetView for Probe {
    type Id = TermId;
    type ReadError = CallerRefusal;
    type TermGuard<'a> = TermRef<'a>;
    type ProbePlan = ();

    fn read_error(&self) -> Option<CallerRefusal> {
        self.fault.then_some(FAULT)
    }
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.quads()
    }
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.reifier_quads()
    }
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner
            .annotation_quads()
            .flat_map(|row| std::iter::repeat_n(row, self.annotation_rows))
    }
    fn named_graphs(&self) -> impl Iterator<Item = TermId> + '_ {
        DatasetView::named_graphs(self.inner.as_ref())
    }
    fn resolve(&self, id: TermId) -> Result<TermRef<'_>, CallerRefusal> {
        Ok(self.inner.as_ref().resolve(id))
    }
    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, CallerRefusal> {
        Ok(self.inner.as_ref().term_id_by_value(value))
    }
    fn capabilities(&self) -> RdfStoreCapabilities {
        self.inner.capabilities()
    }
    fn probe_plan(&self, _: bool, _: bool, _: bool, _: GraphMatch) {}
    fn quads_for_pattern_with_plan(
        &self,
        (): &(),
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.quads_for_pattern(s, p, o, g)
    }
    fn term_count(&self) -> u64 {
        self.inner.term_count()
    }
}

fn control() -> CanonicalizationControl<'static> {
    CanonicalizationControl::new(SearchPolicy::Finite(1 << 20), None)
}

/// 11 and 3.2 of the definition: each kind of row of a view becomes its own
/// line kind. The dataset below lowers to the five lines written beside it,
/// and its labelling equals theirs; the same lines without the empty graph
/// label differently, so the named graph that no line carries is a line.
#[test]
fn every_row_kind_of_a_view_lowers_to_its_line() {
    let mut b = RdfDatasetBuilder::new();
    let (a, bb, r) = (id_of(&mut b, "a"), id_of(&mut b, "b"), id_of(&mut b, "r"));
    let (p, o, g, e) = (
        id_of(&mut b, "<p>"),
        id_of(&mut b, "<o>"),
        id_of(&mut b, "<g>"),
        id_of(&mut b, "<e>"),
    );
    let (q, v) = (id_of(&mut b, "<q>"), id_of(&mut b, "<v>"));
    b.push_quad(a, p, bb, None).expect("quad");
    b.push_quad(bb, p, o, Some(g)).expect("quad in a graph");
    let triple = b.intern_triple(a, p, bb).expect("triple term");
    b.push_reifier(r, triple).expect("reifier row");
    b.push_annotation(r, q, v).expect("annotation row");
    b.declare_named_graph(e).expect("empty graph");
    let view = b.freeze().expect("freezes");

    let lines = vec![
        line_quad("a", "<p>", "b", None),
        line_quad("b", "<p>", "<o>", Some("<g>")),
        Line::ReifierBinding {
            reifier: line_term("r"),
            triple: Term::triple(line_term("a"), line_term("<p>"), line_term("b")),
            graph: None,
        },
        Line::Annotation {
            count: 1,
            reifier: line_term("r"),
            predicate: line_term("<q>"),
            object: line_term("<v>"),
            graph: None,
        },
        Line::EmptyGraph(line_term("<e>")),
    ];
    let (set, labelled) = label_with_records(view.as_ref(), &[]);
    assert_eq!(set.len(), 5, "one line per row; `<g>` is carried by a quad");
    let (_, expected) = label(&lines).expect("labelled");
    assert_eq!(labelled.digest, expected.digest);
    assert_eq!(labelled.lines, expected.lines);
    assert_eq!(labelled.record.blank_nodes, 3);
    let (_, without) = label(&lines[..4]).expect("labelled");
    assert_ne!(labelled.digest, without.digest);
}

/// What a view can yield for a repeated annotation row: a resident dataset
/// yields each row once, so its line has count one; a view that yields a row
/// twice carries the repeat in the count, the line of `annotation(2, ...)`.
#[test]
fn an_annotation_row_a_view_repeats_is_counted() {
    let mut b = RdfDatasetBuilder::new();
    let (s, p, o) = (id_of(&mut b, "s"), id_of(&mut b, "<p>"), id_of(&mut b, "o"));
    let (r, q, v) = (
        id_of(&mut b, "r"),
        id_of(&mut b, "<q>"),
        id_of(&mut b, "<v>"),
    );
    let triple = b.intern_triple(s, p, o).expect("triple term");
    b.push_reifier(r, triple).expect("reifier row");
    b.push_annotation(r, q, v).expect("annotation row");
    let mut view = Probe::over(b.freeze().expect("freezes"));
    let digest = |view: &Probe| label_with_records(view, &[]).1.digest;
    let once = digest(&view);
    view.annotation_rows = 2;
    let twice = digest(&view);
    let annotation = |count| Line::Annotation {
        count,
        reifier: line_term("r"),
        predicate: line_term("<q>"),
        object: line_term("<v>"),
        graph: None,
    };
    let binding = Line::ReifierBinding {
        reifier: line_term("r"),
        triple: Term::triple(line_term("s"), line_term("<p>"), line_term("o")),
        graph: None,
    };
    let counted = |count| {
        label(&[binding.clone(), annotation(count)])
            .expect("labelled")
            .1
            .digest
    };
    assert_eq!(once, counted(1));
    assert_eq!(twice, counted(2));
    assert_ne!(once, twice);
}

/// 3.2: a record's blank node is the view's blank node of equal `(label,
/// scope)`, and a record breaks a symmetry the view alone has. One record on
/// one node of the 4-cycle labels alike wherever it sits; two records on
/// adjacent nodes and on opposite nodes are different inputs with different
/// certificates, and a record on a blank node of another scope names a new
/// node, not the cycle's.
#[test]
fn a_record_beside_a_view_breaks_the_symmetry_of_its_cycle() {
    let view = cycle();
    let digest = |on: &[&str]| label_with_records(view.as_ref(), on).1.digest;
    assert_eq!(digest(&["a"]), digest(&["c"]), "one record, any node");
    let adjacent = digest(&["a", "b"]);
    let opposite = digest(&["a", "c"]);
    assert_eq!(adjacent, digest(&["c", "d"]), "adjacent pairs are alike");
    assert_eq!(opposite, digest(&["b", "d"]), "opposite pairs are alike");
    assert_ne!(adjacent, opposite);
    assert_ne!(digest(&[]), digest(&["a"]));
    // The lines are the view's and one per record: records add no graph.
    let (set, labelled) = label_with_records(view.as_ref(), &["a", "b"]);
    assert_eq!(set.len(), 6);
    assert_eq!(labelled.record.blank_nodes, 4);
    // A blank node of another scope is another node.
    let elsewhere = [TermValue::Blank {
        label: "a".to_owned(),
        scope: BlankScope(1),
    }];
    let records = [Record::new(b"m", &elsewhere)];
    let set = lines_of_view(view.as_ref(), &records).expect("well formed");
    assert_eq!(set.blank_nodes(), 5);
}

/// W3 on a view with records: two identical records are one line stated
/// twice and are refused, and a neighbouring record with another payload is
/// accepted.
#[test]
fn identical_records_beside_a_view_are_refused_and_distinct_ones_pass() {
    let view = cycle();
    let terms = [value_of("a")];
    let same = [Record::new(b"m", &terms), Record::new(b"m", &terms)];
    match lines_of_view(view.as_ref(), &same) {
        Err(ViewError::Refused(Refusal::Input { reason })) => {
            assert_eq!(reason, InputReason::Repeated);
        }
        other => panic!("expected the repeated refusal, got {other:?}"),
    }
    let distinct = [Record::new(b"m", &terms), Record::new(b"n", &terms)];
    assert!(lines_of_view(view.as_ref(), &distinct).is_ok());
}

/// 7.11: the engine's limit is a typed refusal, never a completed result, and
/// a neighbouring limit that the search fits completes. The input is the
/// 13-node circulant of quadratic residues, in both directions, which the
/// search-free run cannot label.
#[test]
fn a_limit_refusal_over_a_view_is_typed_and_a_larger_limit_completes() {
    let squares: Vec<usize> = (1..13).map(|x| x * x % 13).collect();
    let names: Vec<String> = (0..13).map(|i| format!("v{i}")).collect();
    let mut quads = Vec::new();
    for u in 0..13 {
        for v in u + 1..13 {
            if squares.contains(&((v - u) % 13)) {
                quads.push((names[u].as_str(), "<p>", names[v].as_str()));
                quads.push((names[v].as_str(), "<p>", names[u].as_str()));
            }
        }
    }
    let view = dataset(&quads);
    let refused = compare_view(view.as_ref(), &[], &mut control(), Limit::new(0));
    match refused {
        Err(ViewError::Refused(Refusal::Limit {
            broken_by, nodes, ..
        })) => {
            assert_eq!(nodes, 13);
            assert_eq!(broken_by.len(), 1);
        }
        other => panic!(
            "expected the limit refusal, got {:?}",
            other.map(|c| c.labelled.digest)
        ),
    }
    let done = compare_view(view.as_ref(), &[], &mut control(), Limit::REFERENCE)
        .expect("the search fits");
    assert!(!done.labelled.record.guaranteed);
}

/// A view that cannot be read gives back its own error value, typed, and
/// not a refusal and not a result; the same view, readable, completes.
#[test]
fn a_view_read_error_comes_back_as_the_views_own_value() {
    let mut view = Probe::over(cycle());
    let healthy = compare_view(&view, &[], &mut control(), Limit::REFERENCE).expect("completes");
    assert_eq!(healthy.lines.len(), 4);
    view.fault = true;
    assert_eq!(
        compare_view(&view, &[], &mut control(), Limit::REFERENCE).err(),
        Some(ViewError::Read(FAULT))
    );
    assert_eq!(
        lines_of_view(&view, &[]).err(),
        Some(ViewError::Read(FAULT))
    );
}

#[derive(Debug, Default)]
struct Flag(AtomicBool);

impl StopSignal for Flag {
    fn stopped(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// A signal that is clear for its first `polls` polls and set afterwards.
#[derive(Debug)]
struct After(AtomicU64);

impl StopSignal for After {
    fn stopped(&self) -> bool {
        // Polled from one thread; once the count reaches zero it stays there,
        // so the signal latches as the trait requires.
        let left = self.0.load(Ordering::Relaxed);
        if left == 0 {
            return true;
        }
        self.0.store(left - 1, Ordering::Relaxed);
        false
    }
}

/// The stop signal of the control stops the labelling with a typed outcome
/// that is neither a result nor a refusal; the same view with the signal clear
/// completes.
#[test]
fn the_stop_signal_of_the_control_stops_the_comparison() {
    let view = cycle();
    let flag = Flag::default();
    let mut control = CanonicalizationControl::new(SearchPolicy::Finite(1 << 20), Some(&flag));
    assert!(compare_view(view.as_ref(), &[], &mut control, Limit::REFERENCE).is_ok());
    flag.0.store(true, Ordering::Relaxed);
    let stopped = compare_view(view.as_ref(), &[], &mut control, Limit::REFERENCE);
    assert_eq!(stopped.err(), Some(ViewError::Stopped));
}

/// The engine polls the signal at each call, not only on entry: a signal that
/// is clear for the entry poll and set at the first call stops the run, and
/// one that stays clear completes it.
#[test]
fn the_engine_polls_the_stop_signal_at_each_call() {
    let lines = [
        line_quad("a", "<p>", "b", None),
        line_quad("b", "<p>", "c", None),
    ];
    let set = admit(&lines).expect("well formed");
    let run = |clear_polls: u64| {
        reference::label_traced(
            &set,
            reference::ENGINE_LIMIT,
            false,
            false,
            Some(&After(AtomicU64::new(clear_polls))),
        )
    };
    assert!(matches!(run(1), Err(super::Halt::Stopped)));
    assert!(run(u64::MAX).is_ok());
}

/// 7.14: the label of position `i` is `i` in decimal, no leading zero and no
/// affix, in the blank-node form of the syntax.
#[test]
fn a_position_is_spelled_as_a_decimal_blank_node_label() {
    let spell = |position| {
        let mut out = String::new();
        write_label(position, &mut out);
        out
    };
    assert_eq!(spell(0), "_:0");
    assert_eq!(spell(7), "_:7");
    assert_eq!(spell(10), "_:10");
    assert_eq!(spell(u64::MAX), "_:18446744073709551615");
}

/// 7.12: the working state of the labelling is admitted once, through the
/// caller's account, before the engine starts. At the exact bound the
/// comparison completes with the result of the unadmitted entry and holds
/// precisely the bytes it admitted; one byte under, the admission refuses with
/// the typed storage outcome and nothing is labelled.
#[test]
fn the_working_state_is_admitted_once_and_a_refusal_labels_nothing() {
    let view = cycle();
    let plain = compare_view(view.as_ref(), &[], &mut control(), Limit::ENGINE).expect("completes");
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let done = compare_view_with_memory(
        view.as_ref(),
        &[],
        &mut control(),
        Limit::ENGINE,
        &mut memory,
    )
    .expect("admitted");
    assert_eq!(done.labelled.digest, plain.labelled.digest);
    assert_eq!(done.labelled.placements, plain.labelled.placements);
    assert_eq!(memory.admitted_bytes(), done.admitted_bytes);
    let admitted = done.admitted_bytes;
    assert!(admitted > 0);

    let mut at_bound = Ceiling(admitted);
    let fits = compare_view_with_memory(
        view.as_ref(),
        &[],
        &mut control(),
        Limit::ENGINE,
        &mut Memory::new(&mut at_bound),
    );
    assert!(fits.is_ok());
    let mut under = Ceiling(admitted - 1);
    let refused = compare_view_with_memory(
        view.as_ref(),
        &[],
        &mut control(),
        Limit::ENGINE,
        &mut Memory::new(&mut under),
    );
    assert_eq!(
        refused.err(),
        Some(ViewError::Storage(StorageError::AdmissionFailed))
    );
}

/// An error after the admission returns the grant before it returns the
/// error: the account holds nothing once the stop signal ended the run, and
/// holds the comparison's bytes when the same view completes.
#[test]
fn an_admitted_comparison_that_ends_in_an_error_returns_its_grant() {
    let view = cycle();
    let flag = Flag::default();
    let mut ceiling = Ceiling(usize::MAX);
    let mut memory = Memory::new(&mut ceiling);
    let mut control = CanonicalizationControl::new(SearchPolicy::Finite(1 << 20), Some(&flag));
    let done =
        compare_view_with_memory(view.as_ref(), &[], &mut control, Limit::ENGINE, &mut memory)
            .expect("completes");
    assert_eq!(memory.admitted_bytes(), done.admitted_bytes);
    drop(done);
    memory
        .release_bytes(memory.admitted_bytes())
        .expect("the grant is released");
    flag.0.store(true, Ordering::Relaxed);
    let stopped =
        compare_view_with_memory(view.as_ref(), &[], &mut control, Limit::ENGINE, &mut memory);
    assert_eq!(stopped.err(), Some(ViewError::Stopped));
    assert_eq!(memory.admitted_bytes(), 0);
}
