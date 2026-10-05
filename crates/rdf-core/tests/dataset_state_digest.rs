// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete-state identity witnesses and an independent finite-bijection oracle.

#![cfg(not(target_arch = "wasm32"))]

#[path = "support/dataset_state_fixtures.rs"]
mod fixtures;

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_core::{
    BlankScope, CanonHash, CompositeDatasetView, DatasetMut, DatasetStateDigest, DatasetStateError,
    DatasetView, DrainCheckpoint, FallibleDatasetView, GraphMatch, InMemoryPageProvider,
    MutableDataset, PagedDataset, PagedQueryLimits, QuadIds, QuadValues, RdfDataset,
    RdfDatasetBuilder, RdfLiteral, RdfStoreCapabilities, RdfTextDirection, TermId, TermRef,
    TermValue, ViewLimits, ViewOperationStatus, WorkspaceReservation, canonicalize,
    graph_digest_view, try_flat_digest_view,
};
use purrdf_iri::vocab::rdf::REIFIES;

#[global_allocator]
static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

const P: &str = "http://example.org/p";
const Q: &str = "http://example.org/q";
const G: &str = "http://example.org/g";
const H: &str = "http://example.org/h";

/// A small fixture grammar, independent of production canonicalization.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    Iri(&'static str),
    Blank(u8),
    Literal {
        lexical: &'static str,
        datatype: &'static str,
        language: Option<&'static str>,
        direction: Option<RdfTextDirection>,
    },
    Triple(Box<[Self; 3]>),
    Composite {
        chunks: Vec<String>,
        blanks: Vec<u8>,
        datatype: &'static str,
    },
}

impl Node {
    fn blanks(&self, out: &mut BTreeSet<u8>) {
        match self {
            Self::Iri(_) | Self::Literal { .. } => {}
            Self::Blank(id) => {
                out.insert(*id);
            }
            Self::Triple(parts) => {
                for part in parts.as_ref() {
                    part.blanks(out);
                }
            }
            Self::Composite { blanks, .. } => out.extend(blanks),
        }
    }

    fn agrees(&self, other: &Self, map: &[(u8, u8)]) -> bool {
        let mapped = |a, b| map.iter().any(|&(left, right)| left == a && right == b);
        match (self, other) {
            (Self::Iri(a), Self::Iri(b)) => a == b,
            (a @ Self::Literal { .. }, b @ Self::Literal { .. }) => a == b,
            (Self::Blank(a), Self::Blank(b)) => mapped(*a, *b),
            (Self::Triple(a), Self::Triple(b)) => {
                a.iter().zip(b.iter()).all(|(a, b)| a.agrees(b, map))
            }
            (
                Self::Composite {
                    chunks: a,
                    blanks: x,
                    datatype: da,
                },
                Self::Composite {
                    chunks: b,
                    blanks: y,
                    datatype: db,
                },
            ) => {
                da == db
                    && a == b
                    && x.len() == y.len()
                    && x.iter().zip(y).all(|(&a, &b)| mapped(a, b))
            }
            _ => false,
        }
    }

    fn intern(
        &self,
        builder: &mut RdfDatasetBuilder,
        labels: &impl Fn(u8) -> (String, BlankScope),
    ) -> TermId {
        match self {
            Self::Iri(iri) => builder.intern_iri(iri),
            Self::Blank(id) => {
                let (label, scope) = labels(*id);
                builder.intern_blank(&label, scope)
            }
            Self::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => builder.intern_literal(RdfLiteral {
                lexical_form: (*lexical).to_owned(),
                datatype: Some((*datatype).to_owned()),
                language: language.map(str::to_owned),
                direction: *direction,
            }),
            Self::Triple(parts) => {
                let [s, p, o] = parts.each_ref().map(|part| part.intern(builder, labels));
                builder.intern_triple(s, p, o)
            }
            Self::Composite {
                chunks,
                blanks,
                datatype,
            } => {
                assert_eq!(chunks.len(), blanks.len() + 1);
                let mut lexical = chunks[0].clone();
                for (&blank, chunk) in blanks.iter().zip(&chunks[1..]) {
                    let (label, scope) = labels(blank);
                    let encoded = purrdf_core::blank_label::encode_blank_label(
                        &label,
                        scope,
                        purrdf_core::blank_label::LabelAlphabet::BlankNodeLabel,
                    );
                    write!(lexical, "_:{encoded}").expect("writing to a string cannot fail");
                    lexical.push_str(chunk);
                }
                builder.intern_literal(RdfLiteral::typed(lexical, *datatype))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Ordinary,
    Reifier,
    Annotation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Row {
    role: Role,
    terms: [Node; 3],
    graph: Option<Node>,
}

impl Row {
    fn agrees(&self, other: &Self, map: &[(u8, u8)]) -> bool {
        self.role == other.role
            && self
                .terms
                .iter()
                .zip(&other.terms)
                .all(|(a, b)| a.agrees(b, map))
            && match (&self.graph, &other.graph) {
                (Some(a), Some(b)) => a.agrees(b, map),
                (None, None) => true,
                _ => false,
            }
    }
}

#[derive(Clone, Debug, Default)]
struct State {
    graphs: Vec<Node>,
    rows: Vec<Row>,
}

impl State {
    fn digest(&self) -> DatasetStateDigest {
        DatasetStateDigest::from_view(&self.materialize()).expect("valid finite state")
    }
    fn blanks(&self) -> Vec<u8> {
        let mut nodes = BTreeSet::new();
        for graph in &self.graphs {
            graph.blanks(&mut nodes);
        }
        for row in &self.rows {
            for term in &row.terms {
                term.blanks(&mut nodes);
            }
            if let Some(graph) = &row.graph {
                graph.blanks(&mut nodes);
            }
        }
        nodes.into_iter().collect()
    }

    fn graph_names(&self) -> impl Iterator<Item = &Node> {
        self.graphs
            .iter()
            .chain(self.rows.iter().filter_map(|row| row.graph.as_ref()))
    }

    fn agrees(&self, other: &Self, map: &[(u8, u8)]) -> bool {
        self.graph_names()
            .all(|a| other.graph_names().any(|b| a.agrees(b, map)))
            && other
                .graph_names()
                .all(|b| self.graph_names().any(|a| a.agrees(b, map)))
            && self
                .rows
                .iter()
                .all(|a| other.rows.iter().any(|b| a.agrees(b, map)))
            && other
                .rows
                .iter()
                .all(|b| self.rows.iter().any(|a| a.agrees(b, map)))
    }

    fn materialize(&self) -> Arc<RdfDataset> {
        self.materialize_with(&|id| (format!("n{id}"), BlankScope::DEFAULT))
    }

    fn materialize_with(&self, labels: &impl Fn(u8) -> (String, BlankScope)) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        for graph in &self.graphs {
            let graph = graph.intern(&mut builder, labels);
            builder.declare_named_graph(graph);
        }
        for row in &self.rows {
            let [s, p, o] = row
                .terms
                .each_ref()
                .map(|term| term.intern(&mut builder, labels));
            let graph = row
                .graph
                .as_ref()
                .map(|graph| graph.intern(&mut builder, labels));
            match row.role {
                Role::Ordinary => builder.push_quad(s, p, o, graph),
                Role::Reifier => {
                    assert_eq!(row.terms[1], Node::Iri(REIFIES));
                    builder.push_reifier_in_graph(s, o, graph);
                }
                Role::Annotation => builder.push_annotation_in_graph(s, p, o, graph),
            }
        }
        builder
            .freeze()
            .expect("the finite fixture is structurally valid")
    }
}

/// Enumerate whole-state bijections, without any production labeling or codec.
fn isomorphic(a: &State, b: &State) -> bool {
    fn search(a: &State, b: &State, left: &[u8], right: &[u8], map: &mut Vec<(u8, u8)>) -> bool {
        let Some((&head, tail)) = left.split_first() else {
            return a.agrees(b, map);
        };
        for (index, &candidate) in right.iter().enumerate() {
            map.push((head, candidate));
            let mut remaining = right.to_vec();
            remaining.remove(index);
            let matched = search(a, b, tail, &remaining, map);
            map.pop();
            if matched {
                return true;
            }
        }
        false
    }
    let left = a.blanks();
    let right = b.blanks();
    left.len() == right.len() && search(a, b, &left, &right, &mut Vec::new())
}

fn role_fixture(role: Role) -> State {
    let quoted = Node::Triple(Box::new([Node::Iri(G), Node::Iri(P), Node::Iri(H)]));
    let binding = Row {
        role: Role::Reifier,
        terms: [Node::Blank(0), Node::Iri(REIFIES), quoted],
        graph: None,
    };
    State {
        graphs: Vec::new(),
        rows: vec![
            binding,
            Row {
                role,
                terms: [Node::Blank(0), Node::Iri(Q), Node::Iri(H)],
                graph: None,
            },
        ],
    }
}

fn crossing_fixture(crossed: bool) -> State {
    State {
        graphs: vec![Node::Iri(G), Node::Iri(H)],
        rows: [(P, G, 0, 1), (Q, H, u8::from(crossed), u8::from(!crossed))]
            .into_iter()
            .map(|(predicate, graph, s, o)| Row {
                role: Role::Ordinary,
                terms: [Node::Blank(s), Node::Iri(predicate), Node::Blank(o)],
                graph: Some(Node::Iri(graph)),
            })
            .collect(),
    }
}

fn legacy_canonical_form_omits_an_empty_named_declaration() {
    let a = State::default();
    let b = State {
        graphs: vec![Node::Iri(G)],
        rows: Vec::new(),
    };
    assert!(!isomorphic(&a, &b));
    assert_ne!(a.digest(), b.digest());
    assert_eq!(
        canonicalize(&a.materialize()).nquads,
        canonicalize(&b.materialize()).nquads
    );
}

fn legacy_flat_digest_erases_ordinary_annotation_roles() {
    let a = role_fixture(Role::Ordinary);
    let b = role_fixture(Role::Annotation);
    assert!(!isomorphic(&a, &b));
    assert_ne!(a.digest(), b.digest());
    assert_eq!(
        try_flat_digest_view(&*a.materialize(), CanonHash::Sha256).unwrap(),
        try_flat_digest_view(&*b.materialize(), CanonHash::Sha256).unwrap(),
    );
}

fn legacy_flat_digest_erases_ordinary_reifier_roles() {
    let mut a = role_fixture(Role::Ordinary);
    a.rows.pop();
    let mut b = a.clone();
    b.rows[0].role = Role::Ordinary;
    assert!(!isomorphic(&a, &b));
    assert_ne!(a.digest(), b.digest());
    assert_eq!(
        try_flat_digest_view(&*a.materialize(), CanonHash::Sha256).unwrap(),
        try_flat_digest_view(&*b.materialize(), CanonHash::Sha256).unwrap(),
    );
}

fn per_graph_digests_do_not_enforce_a_global_bijection() {
    let a = crossing_fixture(false);
    let b = crossing_fixture(true);
    assert!(!isomorphic(&a, &b));
    assert_ne!(a.digest(), b.digest());
    for graph in [G, H] {
        assert_eq!(
            graph_digest_view(&*a.materialize(), graph),
            graph_digest_view(&*b.materialize(), graph)
        );
    }
    for kept in 0..2 {
        let mut left = a.clone();
        let mut right = b.clone();
        left.rows = vec![left.rows.remove(kept)];
        right.rows = vec![right.rows.remove(kept)];
        assert!(
            isomorphic(&left, &right),
            "either row's removal eliminates the witness"
        );
    }
}

fn oracle_has_one_mapping_across_graphs_nested_and_composite_terms() {
    let make = |inside| State {
        graphs: vec![Node::Blank(0)],
        rows: vec![Row {
            role: Role::Ordinary,
            terms: [
                Node::Blank(0),
                Node::Iri(Q),
                Node::Triple(Box::new([
                    Node::Blank(1),
                    Node::Iri(P),
                    Node::Composite {
                        chunks: ["[", ", ", "]"].map(str::to_owned).to_vec(),
                        blanks: vec![inside, 1],
                        datatype: purrdf_cdt::CDT_LIST,
                    },
                ])),
            ],
            graph: Some(Node::Blank(0)),
        }],
    };
    let a = make(0);
    assert!(isomorphic(&a, &a));
    assert!(!isomorphic(&a, &make(1)));
    assert_ne!(a.digest(), make(1).digest());
    assert!(a.materialize().quad_count() > 0);
}

fn cycle_fixture(split: bool, names: [u8; 6]) -> State {
    State {
        graphs: vec![Node::Iri(G)],
        rows: (0..6)
            .map(|i| {
                let next = if split {
                    (i / 3) * 3 + (i + 1) % 3
                } else {
                    (i + 1) % 6
                };
                Row {
                    role: Role::Ordinary,
                    terms: [
                        Node::Blank(names[i]),
                        Node::Iri(P),
                        Node::Blank(names[next]),
                    ],
                    graph: Some(Node::Iri(G)),
                }
            })
            .collect(),
    }
}

fn oracle_distinguishes_same_color_connected_and_disconnected_cycles() {
    let a = cycle_fixture(false, [0, 1, 2, 3, 4, 5]);
    let b = cycle_fixture(true, [0, 1, 2, 3, 4, 5]);
    assert!(!isomorphic(&a, &b));
    assert_ne!(a.digest(), b.digest());
    assert_eq!(a.materialize().quad_count(), 6);
    assert_eq!(b.materialize().quad_count(), 6);
}

fn oracle_accepts_global_relabeling_and_insertion_permutations() {
    let a = cycle_fixture(false, [0, 1, 2, 3, 4, 5]);
    let mut b = cycle_fixture(false, [9, 3, 7, 2, 8, 1]);
    b.rows.reverse();
    assert!(isomorphic(&a, &b));
    assert_eq!(a.digest(), b.digest());
    assert_eq!(
        canonicalize(&a.materialize()).nquads,
        canonicalize(&b.materialize()).nquads
    );
}

fn exhaustive_three_node_states_match_global_bijection_oracle() {
    let states: Vec<_> = (0_u16..512)
        .map(|mask| State {
            graphs: Vec::new(),
            rows: (0_u8..9)
                .filter(|&edge| mask & (1 << edge) != 0)
                .map(|edge| Row {
                    role: Role::Ordinary,
                    terms: [Node::Blank(edge / 3), Node::Iri(P), Node::Blank(edge % 3)],
                    graph: None,
                })
                .collect(),
        })
        .collect();
    let digests: Vec<_> = states.iter().map(State::digest).collect();
    for (i, a) in states.iter().enumerate() {
        for (j, b) in states.iter().enumerate().take(i + 1) {
            assert_eq!(digests[i] == digests[j], isomorphic(a, b), "states {i}/{j}");
        }
    }
}

purrdf_lex::message_error! {
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct SourceFault;
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FaultMode {
    Ready,
    Before,
    Truncated,
    After,
    PointRead,
    Reservation,
    MissingEmbedded,
    WrongEmbedded,
    Capacity,
    InvalidDatatype,
}

struct Probe {
    source: Arc<RdfDataset>,
    mode: FaultMode,
    checkpoints: Cell<u32>,
    reads: Cell<u32>,
    faulted: Cell<bool>,
    reservation_live: Cell<bool>,
    reservation_peak: Cell<u64>,
    duplicate_rows: bool,
    omit_named: bool,
    workspace_limit: u64,
    allocation_window: RefCell<Option<purrdf_alloc_probe::CurrentThreadWindow>>,
}

impl Probe {
    fn new(source: Arc<RdfDataset>, mode: FaultMode) -> Self {
        Self {
            source,
            mode,
            checkpoints: Cell::new(0),
            reads: Cell::new(0),
            faulted: Cell::new(false),
            reservation_live: Cell::new(false),
            reservation_peak: Cell::new(0),
            duplicate_rows: false,
            omit_named: false,
            workspace_limit: u64::MAX,
            allocation_window: RefCell::new(None),
        }
    }
}

struct Admission<'a>(&'a Probe);

impl WorkspaceReservation for Admission<'_> {
    type Error = SourceFault;
    fn resize(&mut self, bytes: u64) -> Result<(), Self::Error> {
        assert!(
            self.0.reservation_live.get(),
            "admission covers every allocation"
        );
        if let Some(window) = self.0.allocation_window.borrow().as_ref() {
            let observed = window.sample();
            assert!(
                u64::try_from(observed.peak_working_bytes).unwrap()
                    <= self.0.reservation_peak.get(),
                "allocations must fit the admission preceding this resize: {observed:?}",
            );
        }
        self.0
            .reservation_peak
            .set(self.0.reservation_peak.get().max(bytes));
        if bytes > self.0.workspace_limit {
            Err(SourceFault::new("workspace ceiling"))
        } else if self.0.mode == FaultMode::Reservation && bytes > 0 {
            Err(SourceFault::new("workspace refused"))
        } else {
            Ok(())
        }
    }
}

impl Drop for Admission<'_> {
    fn drop(&mut self) {
        self.0.reservation_live.set(false);
    }
}

impl DatasetView for Probe {
    type Id = TermId;
    type ReadError = SourceFault;
    type TermGuard<'a> = TermRef<'a>;
    type ProbePlan = ();

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.reads.set(self.reads.get() + 1);
        if self.mode == FaultMode::Truncated {
            self.faulted.set(true);
        }
        let rows = self
            .source
            .quads()
            .take(if self.mode == FaultMode::Truncated {
                0
            } else {
                usize::MAX
            });
        rows.chain(
            self.source
                .quads()
                .take(if self.duplicate_rows { usize::MAX } else { 0 }),
        )
    }

    fn resolve(&self, id: TermId) -> Result<TermRef<'_>, SourceFault> {
        self.reads.set(self.reads.get() + 1);
        if self.mode == FaultMode::PointRead {
            self.faulted.set(true);
            Err(SourceFault::new("point read refused"))
        } else {
            let term = self.source.as_ref().resolve(id);
            if self.mode == FaultMode::InvalidDatatype
                && term == TermRef::Iri(purrdf_xsd::datatype::XSD_STRING)
            {
                Ok(TermRef::Blank {
                    label: "not-a-datatype",
                    scope: BlankScope::DEFAULT,
                })
            } else {
                Ok(term)
            }
        }
    }

    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, SourceFault> {
        match self.mode {
            FaultMode::MissingEmbedded => Ok(None),
            FaultMode::WrongEmbedded => Ok(Some(self.source.quads().next().unwrap().p)),
            _ => Ok(self.source.as_ref().term_id_by_value(value)),
        }
    }

    fn reserve_workspace(
        &self,
        _: u64,
    ) -> Result<impl WorkspaceReservation<Error = SourceFault> + '_, SourceFault> {
        assert!(!self.reservation_live.replace(true), "one owned admission");
        Ok(Admission(self))
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        self.source.capabilities()
    }
    fn term_count(&self) -> u64 {
        if self.mode == FaultMode::Capacity {
            u64::MAX
        } else {
            self.source.term_count()
        }
    }
    fn named_graphs(&self) -> impl Iterator<Item = TermId> + '_ {
        self.source.named_graphs().filter(|_| !self.omit_named)
    }
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.source.reifier_quads()
    }
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.source.annotation_quads()
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
        self.quads_for_pattern(s, p, o, g)
    }
}

impl FallibleDatasetView for Probe {
    type Error = SourceFault;
    type Evidence = (u32, u32);
    fn operation_status(&self) -> ViewOperationStatus<SourceFault, Self::Evidence> {
        let checkpoint = self.checkpoints.get() + 1;
        self.checkpoints.set(checkpoint);
        if self.mode == FaultMode::Before
            || self.faulted.get()
            || (self.mode == FaultMode::After && checkpoint == 2)
        {
            ViewOperationStatus::Failed {
                error: SourceFault::new(if self.mode == FaultMode::PointRead {
                    "point read refused"
                } else {
                    "source incomplete"
                }),
                evidence: (checkpoint, self.reads.get()),
            }
        } else {
            ViewOperationStatus::Ready {
                evidence: (checkpoint, self.reads.get()),
            }
        }
    }
}

fn operational_failures_never_certify_partial_state() {
    let state = crossing_fixture(false);
    for (mode, expected) in [
        (FaultMode::Before, DrainCheckpoint::Before),
        (FaultMode::Truncated, DrainCheckpoint::After),
        (FaultMode::After, DrainCheckpoint::After),
        (FaultMode::PointRead, DrainCheckpoint::After),
    ] {
        let probe = Probe::new(state.materialize(), mode);
        let Err(DatasetStateError::Operation(failure)) = DatasetStateDigest::from_view(&probe)
        else {
            panic!("an incomplete source must refuse")
        };
        assert_eq!(failure.checkpoint, expected);
        assert_eq!(
            failure.evidence,
            (probe.checkpoints.get(), probe.reads.get())
        );
        if mode == FaultMode::Before {
            assert_eq!(probe.reads.get(), 0);
        }
        if mode == FaultMode::PointRead {
            assert_eq!(failure.error, SourceFault::new("point read refused"));
        }
        assert!(
            !probe.reservation_live.get(),
            "all owned workspace is released"
        );
    }
    let neighbor = Probe::new(state.materialize(), FaultMode::Ready);
    assert_eq!(
        DatasetStateDigest::from_view(&neighbor).unwrap(),
        state.digest()
    );
    assert_eq!(neighbor.checkpoints.get(), 2);
    assert!(neighbor.reservation_peak.get() > 0);
    assert!(!neighbor.reservation_live.get());
}

fn workspace_refusal_and_embedded_lookup_refusals_have_valid_neighbors() {
    let state = State {
        graphs: Vec::new(),
        rows: vec![Row {
            role: Role::Ordinary,
            terms: [
                Node::Iri(G),
                Node::Iri(P),
                Node::Composite {
                    chunks: ["[", "]"].map(str::to_owned).to_vec(),
                    blanks: vec![0],
                    datatype: purrdf_cdt::CDT_LIST,
                },
            ],
            graph: None,
        }],
    };
    let refused = Probe::new(state.materialize(), FaultMode::Reservation);
    assert_eq!(
        DatasetStateDigest::from_view(&refused),
        Err(DatasetStateError::Read(SourceFault::new(
            "workspace refused"
        )))
    );
    assert_eq!(
        refused.reads.get(),
        0,
        "admission refuses before any term/row allocation"
    );
    assert!(!refused.reservation_live.get());
    for mode in [FaultMode::MissingEmbedded, FaultMode::WrongEmbedded] {
        let probe = Probe::new(state.materialize(), mode);
        assert_eq!(
            DatasetStateDigest::from_view(&probe),
            Err(DatasetStateError::IncoherentEmbeddedBlank)
        );
        assert!(!probe.reservation_live.get());
    }
    let good = Probe::new(state.materialize(), FaultMode::Ready);
    assert_eq!(
        DatasetStateDigest::from_view(&good).unwrap(),
        state.digest()
    );
}

fn repeated_same_role_rows_are_set_semantic_before_refinement() {
    let state = crossing_fixture(false);
    let mut duplicate = Probe::new(state.materialize(), FaultMode::Ready);
    duplicate.duplicate_rows = true;
    assert_eq!(
        DatasetStateDigest::from_view(&duplicate).unwrap(),
        state.digest()
    );
    let mut explicit = state.clone();
    explicit.graphs.extend([Node::Iri(G), Node::Iri(G)]);
    explicit.rows.extend(state.rows.clone());
    assert!(
        isomorphic(&state, &explicit),
        "the independent oracle compares row/declaration sets"
    );
    assert_eq!(state.digest(), explicit.digest());
}

fn named_graphs_from_each_role_are_part_of_state_even_without_a_declaration_iterator() {
    for role in [Role::Ordinary, Role::Reifier, Role::Annotation] {
        let mut state = role_fixture(Role::Ordinary);
        state.rows.retain(|row| row.role == Role::Reifier);
        if role == Role::Ordinary {
            state.rows[0].role = Role::Ordinary;
        }
        if role == Role::Annotation {
            state = role_fixture(Role::Annotation);
        }
        for row in &mut state.rows {
            row.graph = Some(Node::Iri(G));
        }
        let mut probe = Probe::new(state.materialize(), FaultMode::Ready);
        probe.omit_named = true;
        assert_eq!(
            DatasetStateDigest::from_view(&probe).unwrap(),
            state.digest()
        );
        let mut default = state.clone();
        for row in &mut default.rows {
            row.graph = None;
        }
        assert_ne!(state.digest(), default.digest());
    }
}

fn role_coexistence_and_empty_graph_kinds_remain_distinct() {
    let ordinary = role_fixture(Role::Ordinary);
    let annotation = role_fixture(Role::Annotation);
    let mut both = ordinary.clone();
    both.rows.push(annotation.rows[1].clone());
    assert!(!isomorphic(&both, &ordinary));
    assert!(!isomorphic(&both, &annotation));
    assert_ne!(both.digest(), ordinary.digest());
    assert_ne!(both.digest(), annotation.digest());
    let iri = State {
        graphs: vec![Node::Iri(G)],
        rows: Vec::new(),
    };
    let blank = State {
        graphs: vec![Node::Blank(0)],
        rows: Vec::new(),
    };
    let two = State {
        graphs: vec![Node::Blank(0), Node::Blank(1)],
        rows: Vec::new(),
    };
    assert_ne!(iri.digest(), blank.digest());
    assert_ne!(blank.digest(), two.digest());
}

fn exact_literal_lexical_datatype_language_and_direction_bytes_participate() {
    use purrdf_iri::vocab::rdf::{DIR_LANG_STRING, LANG_STRING};
    use purrdf_xsd::datatype::{XSD_INTEGER, XSD_STRING};
    let literal = |lexical, datatype, language, direction| Node::Literal {
        lexical,
        datatype,
        language,
        direction,
    };
    let values = [
        literal("1", XSD_INTEGER, None, None),
        literal("01", XSD_INTEGER, None, None),
        literal("1", XSD_STRING, None, None),
        literal("1", LANG_STRING, Some("en"), None),
        literal("1", LANG_STRING, Some("fr"), None),
        literal(
            "1",
            DIR_LANG_STRING,
            Some("en"),
            Some(RdfTextDirection::Ltr),
        ),
        literal(
            "1",
            DIR_LANG_STRING,
            Some("en"),
            Some(RdfTextDirection::Rtl),
        ),
    ];
    let states: Vec<_> = values
        .into_iter()
        .map(|object| State {
            graphs: Vec::new(),
            rows: vec![Row {
                role: Role::Ordinary,
                terms: [Node::Iri(G), Node::Iri(P), object],
                graph: None,
            }],
        })
        .collect();
    for (i, a) in states.iter().enumerate() {
        for b in states.iter().take(i) {
            assert!(!isomorphic(a, b));
            assert_ne!(a.digest(), b.digest());
        }
    }
}

fn scoped_blanks_share_one_mapping_with_cdt_lists_maps_and_all_roles() {
    let composites = [
        Node::Composite {
            chunks: [
                "[ 01, [ ",
                " ], <<(",
                " <http://example.org/p> ",
                ")>>, \"_:not-a-reference\" ]",
            ]
            .map(str::to_owned)
            .to_vec(),
            blanks: vec![0, 1, 0],
            datatype: purrdf_cdt::CDT_LIST,
        },
        Node::Composite {
            chunks: ["{ ", ": [", "], \"key\": ", " }"]
                .map(str::to_owned)
                .to_vec(),
            blanks: vec![0, 1, 0],
            datatype: purrdf_cdt::CDT_MAP,
        },
        Node::Composite {
            chunks: vec![
                "[\"[".to_owned(),
                format!("]\"^^<{}>, \"{{\\\"k\\\": ", purrdf_cdt::CDT_LIST),
                format!("}}\"^^<{}>, ", purrdf_cdt::CDT_MAP),
                "]".to_owned(),
            ],
            blanks: vec![0, 1, 0],
            datatype: purrdf_cdt::CDT_LIST,
        },
    ];
    for object in composites {
        let mut state = role_fixture(Role::Annotation);
        state.graphs = vec![Node::Blank(1), Node::Iri(H)];
        state.rows[0].graph = Some(Node::Blank(1));
        state.rows[1].terms[2] = Node::Blank(1);
        state.rows.push(Row {
            role: Role::Ordinary,
            terms: [Node::Blank(0), Node::Iri(Q), object],
            graph: Some(Node::Blank(1)),
        });
        let renamed = state.materialize_with(&|id| {
            (
                "same label é".to_owned(),
                BlankScope(u32::MAX - u32::from(id)),
            )
        });
        assert_eq!(
            state.digest(),
            DatasetStateDigest::from_view(&renamed).unwrap()
        );
        let mut crossed = state.clone();
        let Node::Composite { blanks, .. } = &mut crossed.rows[2].terms[2] else {
            panic!("composite fixture")
        };
        blanks[0] = 1;
        assert!(!isomorphic(&state, &crossed));
        assert_ne!(state.digest(), crossed.digest());
    }
}

fn interchangeable_empty_blank_declarations_do_not_require_factorial_search() {
    let state = State {
        graphs: (0..128).map(Node::Blank).collect(),
        rows: Vec::new(),
    };
    let mut reversed = state.clone();
    reversed.graphs.reverse();
    let renamed = reversed.materialize_with(&|id| {
        (
            format!("different{}", 255 - id),
            BlankScope(u32::from(id) + 1),
        )
    });
    assert_eq!(
        state.digest(),
        DatasetStateDigest::from_view(&renamed).unwrap()
    );
}

fn refinable_connected_cycle_finishes_without_exhausting_search_work() {
    let state = State {
        graphs: Vec::new(),
        rows: (0..128_u8)
            .map(|i| Row {
                role: Role::Ordinary,
                terms: [Node::Blank(i), Node::Iri(P), Node::Blank((i + 1) % 128)],
                graph: None,
            })
            .collect(),
    };
    assert!(DatasetStateDigest::from_view(&state.materialize()).is_ok());
    assert!(
        DatasetStateDigest::from_view(&cycle_fixture(false, [0, 1, 2, 3, 4, 5]).materialize())
            .is_ok()
    );
}

fn complete_state_agrees_across_production_view_carriers() {
    let mut state = role_fixture(Role::Annotation);
    state.graphs = vec![Node::Blank(0), Node::Iri(H)];
    for row in &mut state.rows {
        row.graph = Some(Node::Blank(0));
    }
    state.rows.push(Row {
        role: Role::Ordinary,
        terms: [
            Node::Blank(0),
            Node::Iri(Q),
            Node::Composite {
                chunks: ["[ ", ", [", "] ]"].map(str::to_owned).to_vec(),
                blanks: vec![0, 0],
                datatype: purrdf_cdt::CDT_LIST,
            },
        ],
        graph: Some(Node::Blank(0)),
    });
    let source = state.materialize();
    let expected = state.digest();
    let independent =
        CompositeDatasetView::new(vec![source.clone()], ViewLimits::default()).unwrap();
    let shared = CompositeDatasetView::with_shared_scopes(
        vec![source.clone(), source.clone()],
        ViewLimits::default(),
    )
    .unwrap();
    let mut mutable = MutableDataset::new(source.clone());
    let scratch = QuadValues::triple(
        TermValue::iri("http://example.org/scratch"),
        TermValue::iri(P),
        TermValue::iri(H),
    );
    assert!(mutable.insert(scratch.clone()).unwrap());
    assert!(mutable.remove(&scratch));
    let delta = mutable.snapshot_view().unwrap();
    let paged =
        PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(vec![source]))).unwrap();
    let reader = paged.query_view(PagedQueryLimits::UNBOUNDED);
    for (name, digest) in [
        (
            "independent composite",
            DatasetStateDigest::from_view(&independent).unwrap(),
        ),
        (
            "shared composite",
            DatasetStateDigest::from_view(&shared).unwrap(),
        ),
        (
            "mutated delta",
            DatasetStateDigest::from_view(&delta).unwrap(),
        ),
        (
            "paged query",
            DatasetStateDigest::from_view(&reader).unwrap(),
        ),
    ] {
        assert_eq!(digest, expected, "{name}");
    }
}

fn nonautomorphic_same_color_orbits_are_not_pruned() {
    let make = |split: bool, names: [u8; 7]| State {
        graphs: Vec::new(),
        rows: (0..7)
            .map(|i| {
                let next = if split && i < 3 {
                    (i + 1) % 3
                } else if split {
                    3 + (i - 3 + 1) % 4
                } else {
                    (i + 1) % 7
                };
                Row {
                    role: Role::Ordinary,
                    terms: [
                        Node::Blank(names[i]),
                        Node::Iri(P),
                        Node::Blank(names[next]),
                    ],
                    graph: None,
                }
            })
            .collect(),
    };
    let split = make(true, [0, 1, 2, 3, 4, 5, 6]);
    let mut renamed = make(true, [4, 5, 6, 0, 1, 2, 3]);
    renamed.rows.rotate_left(3);
    let connected = make(false, [0, 1, 2, 3, 4, 5, 6]);
    assert!(isomorphic(&split, &renamed));
    assert!(!isomorphic(&split, &connected));
    assert_eq!(split.digest(), renamed.digest());
    assert_ne!(split.digest(), connected.digest());
}

fn exact_lexical_bytes_and_maximum_accepted_nesting_have_neighbors() {
    let mut state = State {
        graphs: vec![Node::Blank(0)],
        rows: vec![Row {
            role: Role::Ordinary,
            terms: [
                Node::Blank(0),
                Node::Iri(Q),
                Node::Composite {
                    chunks: ["[ ", " ]"].map(str::to_owned).to_vec(),
                    blanks: vec![0],
                    datatype: purrdf_cdt::CDT_LIST,
                },
            ],
            graph: Some(Node::Blank(0)),
        }],
    };
    let mut differently_spelled = state.clone();
    let Node::Composite { chunks, .. } = &mut differently_spelled.rows[0].terms[2] else {
        panic!("composite fixture")
    };
    assert_eq!(chunks[0].pop(), Some(' '));
    assert!(!isomorphic(&state, &differently_spelled));
    assert_ne!(state.digest(), differently_spelled.digest());

    let mut nested = Node::Blank(0);
    for _ in 0..purrdf_events::MAX_TERM_NESTING_DEPTH {
        nested = Node::Triple(Box::new([Node::Blank(0), Node::Iri(P), nested]));
    }
    state.rows[0].terms[2] = nested;
    let renamed = state.materialize_with(&|_| ("renamed".to_owned(), BlankScope(u32::MAX)));
    assert_eq!(
        state.digest(),
        DatasetStateDigest::from_view(&renamed).unwrap()
    );
    let accepted = state.digest();
    state.rows[0].terms[2] = Node::Triple(Box::new([Node::Blank(0), Node::Iri(Q), Node::Blank(0)]));
    assert_ne!(accepted, state.digest());
}

fn authored_reserved_namespace_iris_remain_ordinary_terms() {
    let mut state = State {
        graphs: vec![Node::Iri("urn:purrdf:rdfc:authored")],
        rows: vec![Row {
            role: Role::Ordinary,
            terms: [
                Node::Iri("urn:purrdf:rdfc:authored"),
                Node::Iri(P),
                Node::Iri(H),
            ],
            graph: None,
        }],
    };
    let ordinary = state.digest();
    state.rows[0].role = Role::Annotation;
    assert_ne!(ordinary, state.digest());
}

fn capacity_and_invalid_datatype_refusals_have_ready_neighbors() {
    let state = State {
        graphs: Vec::new(),
        rows: vec![Row {
            role: Role::Ordinary,
            terms: [
                Node::Iri(G),
                Node::Iri(P),
                Node::Literal {
                    lexical: "value",
                    datatype: purrdf_xsd::datatype::XSD_STRING,
                    language: None,
                    direction: None,
                },
            ],
            graph: None,
        }],
    };
    let source = state.materialize();
    let capacity = Probe::new(source.clone(), FaultMode::Capacity);
    assert_eq!(
        DatasetStateDigest::from_view(&capacity),
        Err(DatasetStateError::Capacity)
    );
    assert_eq!(
        capacity.reads.get(),
        0,
        "refuse before any row or term read"
    );
    assert_eq!(capacity.checkpoints.get(), 2);
    assert!(!capacity.reservation_live.get());
    let invalid = Probe::new(source.clone(), FaultMode::InvalidDatatype);
    assert_eq!(
        DatasetStateDigest::from_view(&invalid),
        Err(DatasetStateError::InvalidTerm)
    );
    assert!(invalid.reads.get() > 0);
    assert_eq!(invalid.checkpoints.get(), 2);
    assert!(!invalid.reservation_live.get());
    let ready = Probe::new(source, FaultMode::Ready);
    assert_eq!(
        DatasetStateDigest::from_view(&ready).unwrap(),
        state.digest()
    );
}

fn disconnected_symmetry_refuses_production_search_and_has_a_small_neighbor() {
    assert_eq!(
        DatasetStateDigest::from_view(&fixtures::triangle_components(8)),
        Err(DatasetStateError::SearchBudgetExceeded),
    );
    assert!(DatasetStateDigest::from_view(&fixtures::triangle_components(2)).is_ok());
}

fn capped_workspace_admits_independent_anchors_and_covers_observed_allocations() {
    let source = fixtures::anchored_blanks(256, false);
    let expected = DatasetStateDigest::from_view(&source).unwrap();
    let mut probe = Probe::new(source, FaultMode::Ready);
    probe.workspace_limit = 2 * 1024 * 1024;
    probe
        .allocation_window
        .replace(Some(purrdf_alloc_probe::CurrentThreadWindow::open()));
    let actual = DatasetStateDigest::from_view(&probe);
    let observed = probe.allocation_window.take().unwrap().close();
    assert_eq!(actual.unwrap(), expected);
    assert!(u64::try_from(observed.peak_working_bytes).unwrap() <= probe.reservation_peak.get());
    assert!(probe.reservation_peak.get() <= probe.workspace_limit);
    assert!(!probe.reservation_live.get());
    let mut lower = Probe::new(Arc::clone(&probe.source), FaultMode::Ready);
    lower.workspace_limit = probe.reservation_peak.get() - 1;
    assert!(matches!(
        DatasetStateDigest::from_view(&lower),
        Err(DatasetStateError::Read(error)) if error == SourceFault::new("workspace ceiling"),
    ));
    assert!(!lower.reservation_live.get());
    eprintln!(
        "anchored workspace: admitted={} observed_peak={}",
        probe.reservation_peak.get(),
        observed.peak_working_bytes
    );
}

fn high_incidence_nested_cdt_and_duplicate_rows_have_admitted_allocation_neighbors() {
    let mut nested = Node::Blank(0);
    for _ in 0..16 {
        nested = Node::Triple(Box::new([Node::Blank(0), Node::Iri(P), nested]));
    }
    let mut chunks = vec!["[ ".to_owned()];
    chunks.extend((0..31).map(|_| ", ".to_owned()));
    chunks.push(" ]".to_owned());
    let state = State {
        graphs: (0..32).map(Node::Blank).collect(),
        rows: vec![
            Row {
                role: Role::Ordinary,
                terms: [
                    Node::Blank(0),
                    Node::Iri(Q),
                    Node::Composite {
                        chunks,
                        blanks: (0..32).collect(),
                        datatype: purrdf_cdt::CDT_LIST,
                    },
                ],
                graph: Some(Node::Blank(0)),
            },
            Row {
                role: Role::Ordinary,
                terms: [Node::Blank(0), Node::Iri(P), nested],
                graph: Some(Node::Blank(0)),
            },
        ],
    };
    let expected = state.digest();
    for duplicate_rows in [false, true] {
        let mut probe = Probe::new(state.materialize(), FaultMode::Ready);
        probe.duplicate_rows = duplicate_rows;
        probe.workspace_limit = 2 * 1024 * 1024;
        probe
            .allocation_window
            .replace(Some(purrdf_alloc_probe::CurrentThreadWindow::open()));
        let actual = DatasetStateDigest::from_view(&probe);
        let observed = probe.allocation_window.take().unwrap().close();
        assert_eq!(actual.unwrap(), expected);
        assert!(
            u64::try_from(observed.peak_working_bytes).unwrap() <= probe.reservation_peak.get()
        );
        assert!(probe.reservation_peak.get() <= probe.workspace_limit);
        assert!(!probe.reservation_live.get());
        eprintln!(
            "nested CDT duplicate={duplicate_rows}: admitted={} observed_peak={}",
            probe.reservation_peak.get(),
            observed.peak_working_bytes
        );
    }
}

purrdf_testkit::harness_main!(
    legacy_canonical_form_omits_an_empty_named_declaration,
    legacy_flat_digest_erases_ordinary_annotation_roles,
    legacy_flat_digest_erases_ordinary_reifier_roles,
    per_graph_digests_do_not_enforce_a_global_bijection,
    oracle_has_one_mapping_across_graphs_nested_and_composite_terms,
    oracle_distinguishes_same_color_connected_and_disconnected_cycles,
    oracle_accepts_global_relabeling_and_insertion_permutations,
    exhaustive_three_node_states_match_global_bijection_oracle,
    operational_failures_never_certify_partial_state,
    workspace_refusal_and_embedded_lookup_refusals_have_valid_neighbors,
    repeated_same_role_rows_are_set_semantic_before_refinement,
    named_graphs_from_each_role_are_part_of_state_even_without_a_declaration_iterator,
    role_coexistence_and_empty_graph_kinds_remain_distinct,
    exact_literal_lexical_datatype_language_and_direction_bytes_participate,
    scoped_blanks_share_one_mapping_with_cdt_lists_maps_and_all_roles,
    interchangeable_empty_blank_declarations_do_not_require_factorial_search,
    refinable_connected_cycle_finishes_without_exhausting_search_work,
    complete_state_agrees_across_production_view_carriers,
    nonautomorphic_same_color_orbits_are_not_pruned,
    exact_lexical_bytes_and_maximum_accepted_nesting_have_neighbors,
    authored_reserved_namespace_iris_remain_ordinary_terms,
    capacity_and_invalid_datatype_refusals_have_ready_neighbors,
    disconnected_symmetry_refuses_production_search_and_has_a_small_neighbor,
    capped_workspace_admits_independent_anchors_and_covers_observed_allocations,
    high_incidence_nested_cdt_and_duplicate_rows_have_admitted_allocation_neighbors,
);
