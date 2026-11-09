// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Append-only source lifetimes over the original OWL preparation and services.

use std::sync::Arc;

use purrdf_core::{
    CanonicalizationControl, DatasetStateDigest, DatasetView, FallibleDatasetView, GraphMatch,
    TermValue, checkpointed_drain,
};
use purrdf_datalog::StopSignal;
use purrdf_lex::allocation::{
    Memory, OrderedTable, Resident, StorageError, increment_count as count,
};

use crate::EntailError;
use crate::owl_dl::proof::layout::{self, Counts, PreparationLayout};
use crate::owl_dl::{Kb, parser};

use super::{Certified, DlAxiom, Question, Reasoner, SchemaPreparationBudget, Verdict};

mod limits;
pub use limits::{PositiveWorldError, PositiveWorldLimits};

type Row = (u32, u32, u32, Option<u32>);

/// The original RDF statement family, preserved independently through updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositiveSourceLayer {
    /// An ordinary source quad.
    Ordinary,
    /// An RDF 1.2 reifier declaration.
    Reifier,
    /// An annotation statement.
    Annotation,
}

impl PositiveSourceLayer {
    fn index(self) -> usize {
        match self {
            Self::Ordinary => 0,
            Self::Reifier => 1,
            Self::Annotation => 2,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Ordinary => "ordinary",
            Self::Reifier => "reifier",
            Self::Annotation => "annotation",
        }
    }
}

/// An exact append-only or frozen-schema violation, with original source values.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PositiveUpdateRefusal {
    /// A new preparation payload cannot be represented by the native proof codec.
    ProofEncoding {
        /// Original typed codec refusal, including term-depth refusal.
        cause: Box<crate::DlProofError>,
    },
    /// A caller-selected aggregate retention ceiling refused growth.
    RetentionLimit {
        /// The resource whose ceiling was exceeded.
        resource: &'static str,
        /// Requested aggregate value.
        attempted: u128,
        /// Caller-selected ceiling.
        limit: u128,
    },
    /// A previously reached term has no identity in the proposed next snapshot.
    MissingTerm {
        /// The original retained value.
        term: TermValue,
    },
    /// A previously retained statement disappeared from its original family.
    RemovedStatement {
        /// Original statement family.
        layer: PositiveSourceLayer,
        /// Original subject.
        subject: TermValue,
        /// Original predicate.
        predicate: TermValue,
        /// Original object.
        object: TermValue,
        /// Original named graph, absent for the default graph.
        graph: Option<TermValue>,
    },
    /// A named graph declaration disappeared, including an empty one.
    RemovedGraph {
        /// Original graph name.
        graph: TermValue,
    },
    /// A new default-graph row would alter the frozen schema.
    FrozenSchema {
        /// Original subject.
        subject: TermValue,
        /// Original predicate.
        predicate: TermValue,
        /// Original object.
        object: TermValue,
    },
}

impl purrdf_hash::error::StructuredError for PositiveUpdateRefusal {
    purrdf_hash::error_visit! {
        ProofEncoding { cause } => "positive-proof-encoding" { "cause" => Cause(cause.as_ref()) },
        RetentionLimit { resource, attempted, limit } => "positive-retention-limit" {
            "resource" => Code(resource), "attempted" => Unsigned(*attempted), "limit" => Unsigned(*limit),
        },
        MissingTerm { term } => "positive-source-term-removed" { "term" => Term(term) },
        RemovedStatement { layer, subject, predicate, object, graph } => "positive-source-statement-removed" {
            "layer" => Code(layer.name()), "subject" => Term(subject), "predicate" => Term(predicate), "object" => Term(object),
            "graph" => List(graph.iter().map(|term| purrdf_hash::error::ErrorValue::Term(term))),
        },
        RemovedGraph { graph } => "positive-source-graph-removed" { "graph" => Term(graph) },
        FrozenSchema { subject, predicate, object } => "positive-frozen-schema" {
            "subject" => Term(subject), "predicate" => Term(predicate), "object" => Term(object),
        },
    }
}

impl std::fmt::Display for PositiveUpdateRefusal {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProofEncoding { cause } => {
                write!(out, "positive preparation proof encoding refused: {cause}")
            }
            Self::RetentionLimit {
                resource,
                attempted,
                limit,
            } => write!(
                out,
                "positive-world {resource} requires {attempted}; caller limit is {limit}"
            ),
            Self::MissingTerm { term } => {
                write!(out, "positive source update removed term {term:?}")
            }
            Self::RemovedStatement {
                layer,
                subject,
                predicate,
                object,
                graph,
            } => write!(
                out,
                "positive source update removed {} statement ({subject:?}, {predicate:?}, {object:?}, {graph:?})",
                layer.name()
            ),
            Self::RemovedGraph { graph } => write!(
                out,
                "positive source update removed graph declaration {graph:?}"
            ),
            Self::FrozenSchema {
                subject,
                predicate,
                object,
            } => write!(
                out,
                "positive source update changes frozen schema with ({subject:?}, {predicate:?}, {object:?})"
            ),
        }
    }
}
purrdf_hash::error_sources!(PositiveUpdateRefusal, patterns Self::ProofEncoding { cause } => cause.as_ref());

/// Actual source-frontier and parser operations of the latest successful advance.
/// Complete-state and legacy proof identities are separate whole-source
/// operations, disclosed by their flag and governed by the caller's control.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PositiveUpdateWork {
    /// Borrowed reverse lookups rebinding original terms into the next source.
    pub borrowed_term_lookups: u64,
    /// Exact source probes checking retained statement and declaration aliases.
    pub retained_row_probes: u64,
    /// Incoming physical and virtual rows inspected without reconstructing old terms.
    pub scanned_rows: u64,
    /// Owned source term reads: all cold parser reads, then unknown frontier
    /// identities only. Repeated cold physical rows still count their reads.
    pub copied_terms: u64,
    /// New rows retained across the three separate RDF statement families.
    pub new_rows: u64,
    /// New named graph declarations, including empty graphs.
    pub new_graphs: u64,
    /// New default-graph rows passed to the original semantic decoder.
    pub interpreted_default_rows: u64,
    /// Whether complete-state and legacy canonical proof identities were recomputed.
    pub identity_recomputed: bool,
}

/// A retained caller snapshot and a frozen-schema OWL positive assertion world.
///
/// Advancing consumes this owner. A failed read, allocation, cancellation, schema
/// change or inference returns its original typed error and no usable partial
/// world. The caller may retain its own snapshot handles independently.
///
/// Native term IDs are rebound through exact borrowed source lookups on each
/// advance; sources need not preserve their local IDs between snapshots. Old
/// assertions and their parser/provenance state are retained, without a second
/// RDF dataset or repeated semantic lowering. Frontier validation inspects the
/// full next source; recording additionally computes both complete identities.
pub struct PositiveWorld<D: FallibleDatasetView> {
    source: D,
    reasoner: Reasoner,
    ingress: parser::Ingress,
    frontier: Frontier,
    work: PositiveUpdateWork,
    limits: PositiveWorldLimits,
    layout: Option<PreparationLayout>,
    step_cap: Option<u64>,
    work_cap: Option<u64>,
}

impl<D: FallibleDatasetView> std::fmt::Debug for PositiveWorld<D> {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("PositiveWorld")
            .field("reasoner", &self.reasoner)
            .field("source_terms", &self.frontier.terms.len())
            .field("work", &self.work)
            .finish_non_exhaustive()
    }
}

impl<D: FallibleDatasetView> PositiveWorld<D> {
    /// Prepare the caller's snapshot without recording proofs.
    /// # Errors
    /// Original native preparation, source-read and storage failures.
    pub fn new(
        source: D,
        limits: PositiveWorldLimits,
    ) -> Result<Self, PositiveWorldError<D::Error, D::Evidence>> {
        Self::build(
            source,
            None,
            None,
            SchemaPreparationBudget::default(),
            None,
            limits,
        )
    }

    /// Prepare the same world with caller-controlled canonical proof identity.
    /// # Errors
    /// Original source/preparation and canonicalization failures.
    pub fn with_proofs(
        source: D,
        limits: PositiveWorldLimits,
        control: &mut CanonicalizationControl<'_>,
    ) -> Result<Self, PositiveWorldError<D::Error, D::Evidence>> {
        let state =
            DatasetStateDigest::from_view(&source, control).map_err(PositiveWorldError::State)?;
        let input = checkpointed_drain(&source, |source| {
            crate::owl_dl::proof::try_ontology_identity(source, control)
        })
        .map_err(PositiveWorldError::Drain)?
        .map_err(EntailError::Canonicalization)?;
        Self::build(
            source,
            Some(input),
            Some(state),
            SchemaPreparationBudget::default(),
            None,
            limits,
        )
    }

    /// Retain explicit preparation limits and cancellation for every world revision.
    /// # Errors
    /// Original source/preparation failures or cancellation.
    pub fn with_stop(
        source: D,
        limits: PositiveWorldLimits,
        budget: SchemaPreparationBudget,
        stop: Arc<dyn StopSignal>,
    ) -> Result<Self, PositiveWorldError<D::Error, D::Evidence>> {
        Self::build(source, None, None, budget, Some(stop), limits)
    }

    /// Record proofs under the same explicit preparation and stop policy.
    /// # Errors
    /// Original preparation, cancellation or canonicalization failure.
    pub fn with_proofs_and_stop(
        source: D,
        limits: PositiveWorldLimits,
        budget: SchemaPreparationBudget,
        stop: Arc<dyn StopSignal>,
        control: &mut CanonicalizationControl<'_>,
    ) -> Result<Self, PositiveWorldError<D::Error, D::Evidence>> {
        if stop.stopped() {
            return Err(EntailError::Stopped.into());
        }
        let state =
            DatasetStateDigest::from_view(&source, control).map_err(PositiveWorldError::State)?;
        let input = checkpointed_drain(&source, |source| {
            crate::owl_dl::proof::try_ontology_identity(source, control)
        })
        .map_err(PositiveWorldError::Drain)?
        .map_err(EntailError::Canonicalization)?;
        Self::build(source, Some(input), Some(state), budget, Some(stop), limits)
    }

    fn build(
        source: D,
        input: Option<[u8; 32]>,
        state: Option<DatasetStateDigest>,
        budget: SchemaPreparationBudget,
        stop: Option<Arc<dyn StopSignal>>,
        limits: PositiveWorldLimits,
    ) -> Result<Self, PositiveWorldError<D::Error, D::Evidence>> {
        let (reasoner, ingress, frontier, work) = checkpointed_drain(&source, |source| {
            let (mut reasoner, ingress) = Reasoner::build_prepared(source, input, budget, stop)?;
            let mut frontier = Frontier::new(limits);
            let mut work = PositiveUpdateWork::default();
            source
                .checked_read(|source| frontier.read(source, &mut reasoner.kb, &mut work, true))
                .map_err(EntailError::source_read)??;
            work.interpreted_default_rows = u64::try_from(ingress.initial_order.len())
                .map_err(|_| StorageError::SizeOverflow)?;
            work.copied_terms = work
                .copied_terms
                .checked_add(ingress.source_term_reads)
                .ok_or(StorageError::SizeOverflow)?;
            work.identity_recomputed = input.is_some();
            Ok::<_, EntailError<D::ReadError>>((reasoner, ingress, frontier, work))
        })
        .map_err(PositiveWorldError::Drain)??;
        let mut world = Self {
            source,
            reasoner,
            ingress,
            frontier,
            work,
            limits,
            layout: state.map(PreparationLayout::new),
            step_cap: None,
            work_cap: None,
        };
        let lowering = std::mem::take(&mut world.ingress.initial_order);
        world
            .record_source([0; 5], &lowering, true)
            .map_err(|error| PositiveWorldError::Entail(error.with_read_error()))?;
        checkpointed_drain(&world.source, |_| ready(&world.reasoner.kb))
            .map_err(PositiveWorldError::Drain)??;
        Ok(world)
    }

    /// The latest retained caller source, with all statement families and declarations.
    pub const fn source(&self) -> &D {
        &self.source
    }

    /// Narrow the native decision step ceiling for this and every later revision.
    #[must_use]
    pub fn with_step_cap(mut self, cap: u64) -> Self {
        self.step_cap = Some(self.step_cap.map_or(cap, |held| held.min(cap)));
        self.reasoner = self.reasoner.with_step_cap(cap);
        self
    }

    /// Narrow the native decision work ceiling for this and every later revision.
    #[must_use]
    pub fn with_work_cap(mut self, cap: u64) -> Self {
        self.work_cap = Some(self.work_cap.map_or(cap, |held| held.min(cap)));
        self.reasoner = self.reasoner.with_work_cap(cap);
        self
    }
    /// Check consistency through the original native service.
    pub fn consistency(&self) -> Result<Certified<Verdict>, EntailError> {
        self.certify(self.reasoner.consistency())
    }
    /// Prepare the caller's question, retaining the same source/schema owner.
    /// # Errors
    /// Original query preparation failure; no partially prepared owner is returned.
    pub fn prepare(mut self, question: &Question) -> Result<Self, EntailError> {
        ready(&self.reasoner.kb)?;
        self.record_question(question)?;
        self.reasoner.prepare(question)?;
        ready(&self.reasoner.kb)?;
        Ok(self)
    }
    /// Ask class satisfiability through the original service.
    /// # Errors
    /// Original preparation or decision failure, consuming this owner.
    pub fn class_satisfiability(
        mut self,
        class: &TermValue,
    ) -> Result<(Self, Certified<Verdict>), EntailError> {
        let answer = self.reasoner.class_satisfiability(class)?;
        let answer = self.certify(answer)?;
        if let Some(proof) = answer.proof() {
            self.record_question(proof.question())?;
        }
        Ok((self, answer))
    }
    /// Ask axiom entailment through the original service.
    /// # Errors
    /// Original preparation or decision failure, consuming this owner.
    pub fn entails(mut self, axiom: &DlAxiom) -> Result<(Self, Certified<Verdict>), EntailError> {
        let answer = self.reasoner.entails(axiom)?;
        let answer = self.certify(answer)?;
        if let Some(proof) = answer.proof() {
            self.record_question(proof.question())?;
        }
        Ok((self, answer))
    }
    /// Retrieve instances through the original service.
    /// # Errors
    /// Original preparation or decision failure, consuming this owner.
    pub fn instances(
        mut self,
        class: &TermValue,
    ) -> Result<(Self, Certified<Vec<TermValue>>), EntailError> {
        let answer = self.reasoner.instances(class)?;
        let answer = self.certify(answer)?;
        if let Some(proof) = answer.proof() {
            self.record_question(proof.question())?;
        }
        Ok((self, answer))
    }
    /// Actual frontier and delta-parser work of the latest successful operation.
    pub const fn work(&self) -> PositiveUpdateWork {
        self.work
    }

    fn certify<T>(&self, mut answer: Certified<T>) -> Result<Certified<T>, EntailError> {
        if let Some(layout) = &self.layout {
            answer.attach_positive_layout(layout.copy(self.limits, self.frontier.live)?)?;
        }
        Ok(answer)
    }

    fn record_question(&mut self, question: &Question) -> Result<(), EntailError> {
        if let Some(layout) = &mut self.layout {
            layout.question(question, self.limits, self.frontier.live)?;
        }
        Ok(())
    }

    fn frontier_counts(&self) -> [usize; 5] {
        [
            self.frontier.terms.len(),
            self.frontier.rows[0].len(),
            self.frontier.rows[1].len(),
            self.frontier.rows[2].len(),
            self.frontier.graphs.len(),
        ]
    }

    fn record_source(
        &mut self,
        start: [usize; 5],
        lowering: &[(u32, u32, u32)],
        initial: bool,
    ) -> Result<(), EntailError> {
        let end = self.frontier_counts();
        let Some(program) = &mut self.layout else {
            return Ok(());
        };
        let counts = Counts {
            terms: end[0] - start[0],
            rows: end[1..]
                .iter()
                .zip(&start[1..])
                .map(|(end, start)| end - start)
                .sum(),
            question_bytes: 0,
        };
        program.push(counts, self.limits, self.frontier.live, |out| {
            out.push(u8::from(!initial));
            super::proof::length(out, counts.terms);
            for (&slot, ()) in self.frontier.terms.iter().skip(start[0]) {
                out.extend_from_slice(&slot.to_le_bytes());
                super::proof::encode_term(out, self.reasoner.kb.interner.value(slot));
            }
            super::proof::length(
                out,
                (end[1] - start[1]) + (end[2] - start[2]) + (end[3] - start[3]),
            );
            for family in 0..3 {
                for (&row, ()) in self.frontier.rows[family].iter().skip(start[family + 1]) {
                    out.push(u8::try_from(family).expect("three statement families fit u8"));
                    layout::encode_row(out, row);
                }
            }
            super::proof::length(out, end[4] - start[4]);
            for (&graph, ()) in self.frontier.graphs.iter().skip(start[4]) {
                out.extend_from_slice(&graph.to_le_bytes());
            }
            super::proof::length(out, lowering.len());
            for &triple in lowering {
                for term in <[u32; 3]>::from(triple) {
                    out.extend_from_slice(&term.to_le_bytes());
                }
            }
        })
    }

    /// Advance to an append-only snapshot using the existing assertion decoder.
    ///
    /// The original schema, source term identities and assertion storage survive.
    /// Recording uses the supplied canonical control; unrecorded worlds leave it
    /// untouched. An unchanged source performs no semantic parsing, key inference
    /// or canonicalization. Failed advances consume the owner and publish nothing.
    /// Independent proof consumers prepare their own world and service questions.
    ///
    /// # Errors
    /// Original read/storage/cancellation/canonicalization/inference failures;
    /// [`EntailError::PositiveUpdate`] for removal or frozen-schema mutation.
    pub fn advance(
        mut self,
        source: D,
        control: &mut CanonicalizationControl<'_>,
    ) -> Result<Self, PositiveWorldError<D::Error, D::Evidence>> {
        checkpointed_drain(&source, |source| self.advance_inner(source, control))
            .map_err(PositiveWorldError::Drain)??;
        self.source = source;
        if let Some(cap) = self.step_cap {
            self.reasoner = self.reasoner.with_step_cap(cap);
        }
        if let Some(cap) = self.work_cap {
            self.reasoner = self.reasoner.with_work_cap(cap);
        }
        Ok(self)
    }

    fn advance_inner(
        &mut self,
        source: &D,
        control: &mut CanonicalizationControl<'_>,
    ) -> Result<(), PositiveWorldError<D::Error, D::Evidence>> {
        let start = self.frontier_counts();
        let layout_bytes = self.layout.as_ref().map_or(0, |layout| layout.live);
        self.frontier.storage.limit = self
            .limits
            .metadata_bytes
            .checked_sub(layout_bytes)
            .ok_or_else(|| {
                PositiveWorldError::Entail(
                    retention(
                        "metadata bytes",
                        layout_bytes as u128,
                        self.limits.metadata_bytes as u128,
                    )
                    .with_read_error(),
                )
            })?;
        let mut work = PositiveUpdateWork::default();
        let triples = source
            .checked_read(|source| {
                self.frontier
                    .read(source, &mut self.reasoner.kb, &mut work, false)
            })
            .map_err(EntailError::source_read)??;
        ready(&self.reasoner.kb)?;
        if work.new_rows != 0 || work.new_graphs != 0 {
            if let Some(layout) = &mut self.layout {
                layout.state = *DatasetStateDigest::from_view(source, control)
                    .map_err(PositiveWorldError::State)?
                    .as_bytes();
            }
            let input = if self.reasoner.records_proofs() {
                work.identity_recomputed = true;
                Some(
                    crate::owl_dl::proof::try_ontology_identity(source, control)
                        .map_err(EntailError::Canonicalization)?,
                )
            } else {
                None
            };
            ready(&self.reasoner.kb)?;
            self.reasoner
                .append_positive(&mut self.ingress, &triples, input)
                .map_err(|error| PositiveWorldError::Entail(error.with_read_error()))?;
            self.record_source(start, &triples, false)
                .map_err(|error| PositiveWorldError::Entail(error.with_read_error()))?;
        }
        ready(&self.reasoner.kb)?;
        self.work = work;
        Ok(())
    }
}

struct Frontier {
    terms: OrderedTable<u32, ()>,
    rows: [OrderedTable<Row, ()>; 3],
    graphs: OrderedTable<u32, ()>,
    storage: layout::Metadata,
    live: usize,
    limits: PositiveWorldLimits,
}

impl Frontier {
    fn new(limits: PositiveWorldLimits) -> Self {
        Self {
            terms: OrderedTable::default(),
            rows: std::array::from_fn(|_| OrderedTable::default()),
            graphs: OrderedTable::default(),
            storage: layout::Metadata {
                limit: limits.metadata_bytes,
                attempted: 0,
            },
            live: 0,
            limits,
        }
    }

    fn admit_row<E>(&self) -> Result<(), EntailError<E>> {
        let rows = self.rows.iter().map(OrderedTable::len).sum::<usize>() as u128
            + self.graphs.len() as u128
            + 1;
        if rows > self.limits.source_rows as u128 {
            return Err(retention(
                "source rows and declarations",
                rows,
                self.limits.source_rows as u128,
            )
            .with_read_error());
        }
        Ok(())
    }

    fn admit_term<E>(&self) -> Result<(), EntailError<E>> {
        if self.terms.len() >= self.limits.source_terms {
            return Err(retention(
                "source terms",
                self.terms.len() as u128 + 1,
                self.limits.source_terms as u128,
            )
            .with_read_error());
        }
        Ok(())
    }
    fn read<D: DatasetView>(
        &mut self,
        source: &D,
        kb: &mut Kb,
        work: &mut PositiveUpdateWork,
        initial: bool,
    ) -> Result<Vec<layout::Triple>, EntailError<D::ReadError>> {
        ready(kb)?;
        let mut aliases = OrderedTable::default();
        let mut originals = OrderedTable::default();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut bind = |native| -> Result<(), EntailError<D::ReadError>> {
            ready(kb)?;
            count(&mut work.borrowed_term_lookups)?;
            let id = source
                .term_id_by_value(kb.interner.value(native))
                .map_err(EntailError::source_read)?;
            if let Some(id) = id {
                aliases.insert(id, native, &mut memory)?;
                originals.insert(native, id, &mut memory)?;
            } else if !initial {
                return Err(EntailError::PositiveUpdate(Box::new(
                    PositiveUpdateRefusal::MissingTerm {
                        term: kb.interner.value(native).clone(),
                    },
                )));
            }
            Ok(())
        };
        if initial {
            for native in
                0..u32::try_from(kb.interner.len()).map_err(|_| StorageError::SizeOverflow)?
            {
                bind(native)?;
            }
        } else {
            for (&native, ()) in self.terms.iter() {
                bind(native)?;
            }
        }
        for layer in [
            PositiveSourceLayer::Ordinary,
            PositiveSourceLayer::Reifier,
            PositiveSourceLayer::Annotation,
        ] {
            for (&(s, p, o, g), ()) in self.rows[layer.index()].iter() {
                ready(kb)?;
                count(&mut work.retained_row_probes)?;
                let (s_id, p_id, o_id) = (
                    originals.get(&s).copied().expect("retained term rebound"),
                    originals.get(&p).copied().expect("retained term rebound"),
                    originals.get(&o).copied().expect("retained term rebound"),
                );
                let graph = g.map_or(GraphMatch::Default, |g| {
                    GraphMatch::Named(*originals.get(&g).expect("retained graph rebound"))
                });
                let present = match layer {
                    PositiveSourceLayer::Ordinary => source
                        .quads_for_pattern(Some(s_id), Some(p_id), Some(o_id), graph)
                        .next()
                        .is_some(),
                    PositiveSourceLayer::Reifier => source
                        .reifier_quads_for_target(o_id, graph)
                        .any(|q| q.s == s_id && q.p == p_id),
                    PositiveSourceLayer::Annotation => source
                        .annotation_quads_for_pattern(Some(p_id), Some(o_id), graph)
                        .any(|q| q.s == s_id),
                };
                ready(kb)?;
                if !present {
                    return Err(EntailError::PositiveUpdate(Box::new(
                        PositiveUpdateRefusal::RemovedStatement {
                            layer,
                            subject: kb.interner.value(s).clone(),
                            predicate: kb.interner.value(p).clone(),
                            object: kb.interner.value(o).clone(),
                            graph: g.map(|g| kb.interner.value(g).clone()),
                        },
                    )));
                }
            }
        }
        for (&graph, ()) in self.graphs.iter() {
            ready(kb)?;
            count(&mut work.retained_row_probes)?;
            let id = *originals.get(&graph).expect("retained graph rebound");
            if !source.named_graphs().any(|candidate| candidate == id) {
                return Err(EntailError::PositiveUpdate(Box::new(
                    PositiveUpdateRefusal::RemovedGraph {
                        graph: kb.interner.value(graph).clone(),
                    },
                )));
            }
        }
        let mut triples = Vec::new();
        let rows = source
            .quads()
            .map(|q| (PositiveSourceLayer::Ordinary, q))
            .chain(
                source
                    .reifier_quads()
                    .map(|q| (PositiveSourceLayer::Reifier, q)),
            )
            .chain(
                source
                    .annotation_quads()
                    .map(|q| (PositiveSourceLayer::Annotation, q)),
            );
        for (layer, q) in rows {
            ready(kb)?;
            count(&mut work.scanned_rows)?;
            let mut term = |id| self.term(source, kb, id, &mut aliases, &mut memory, work);
            let row = (
                term(q.s)?,
                term(q.p)?,
                term(q.o)?,
                q.g.map(&mut term).transpose()?,
            );
            if self.rows[layer.index()].get(&row).is_none() {
                self.admit_row()?;
                let mut retained = Memory::resume(&mut self.storage, self.live);
                let result = self.rows[layer.index()].insert(row, (), &mut retained);
                self.live = retained.admitted_bytes();
                result.map_err(|error| self.storage.cause(error).with_read_error())?;
                count(&mut work.new_rows)?;
                if matches!(layer, PositiveSourceLayer::Ordinary) && row.3.is_none() {
                    memory.push(&mut triples, (row.0, row.1, row.2))?;
                    count(&mut work.interpreted_default_rows)?;
                }
            }
        }
        for graph in source.named_graphs() {
            ready(kb)?;
            let graph = self.term(source, kb, graph, &mut aliases, &mut memory, work)?;
            if self.graphs.get(&graph).is_none() {
                self.admit_row()?;
                let mut retained = Memory::resume(&mut self.storage, self.live);
                let result = self.graphs.insert(graph, (), &mut retained);
                self.live = retained.admitted_bytes();
                result.map_err(|error| self.storage.cause(error).with_read_error())?;
                count(&mut work.new_graphs)?;
            }
        }
        ready(kb)?;
        Ok(triples)
    }

    fn term<D: DatasetView>(
        &mut self,
        source: &D,
        kb: &mut Kb,
        id: D::Id,
        aliases: &mut OrderedTable<D::Id, u32>,
        memory: &mut Memory<'_, Resident>,
        work: &mut PositiveUpdateWork,
    ) -> Result<u32, EntailError<D::ReadError>> {
        let native = if let Some(&native) = aliases.get(&id) {
            native
        } else {
            self.admit_term()?;
            count(&mut work.copied_terms)?;
            let value = source.term_value_with_storage(id, EntailError::from, |_| {
                EntailError::Storage(StorageError::AllocationFailed)
            })?;
            let native = kb.source_terms.intern(&mut kb.interner, value)?;
            aliases.insert(id, native, memory)?;
            native
        };
        if self.terms.get(&native).is_none() {
            self.admit_term()?;
            let mut retained = Memory::resume(&mut self.storage, self.live);
            let result = self.terms.insert(native, (), &mut retained);
            self.live = retained.admitted_bytes();
            result.map_err(|error| self.storage.cause(error).with_read_error())?;
            kb.source_terms.record(native)?;
        }
        Ok(native)
    }
}

pub(crate) fn retention(resource: &'static str, attempted: u128, limit: u128) -> EntailError {
    EntailError::PositiveUpdate(Box::new(PositiveUpdateRefusal::RetentionLimit {
        resource,
        attempted,
        limit,
    }))
}

fn ready<E>(kb: &Kb) -> Result<(), EntailError<E>> {
    if purrdf_core::governor::is_stopped(kb.stop.as_deref()) {
        Err(EntailError::Stopped)
    } else {
        Ok(())
    }
}
