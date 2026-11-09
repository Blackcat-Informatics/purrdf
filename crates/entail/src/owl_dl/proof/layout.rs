// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Untrusted preparation order, replayed by the native source and query owners.

use std::sync::Arc;

use purrdf_core::{
    CanonicalizationControl, DatasetStateDigest, DatasetView, FallibleDatasetView, QuadIds,
    TermValue,
};
use purrdf_datalog::StopSignal;
use purrdf_hash::{Domain, blake3, frame::frame_le_into};
use purrdf_lex::allocation::{Admission, Memory, OrderedTable, Shared, StorageError};

use crate::reasoner::positive::{PositiveWorldError, PositiveWorldLimits, retention};
use crate::reasoner::proof::{
    PreparationEncoding, PreparationSize, decode_question_with_memory, decode_term_with_memory,
    encode_question, length,
};
use crate::reasoner::{Question, Reasoner, SchemaPreparationBudget};
use crate::{DlProofContext, DlProofError, EntailError};

use super::{Reader, malformed};

type Checked<D, T> = Result<
    T,
    PositiveWorldError<<D as FallibleDatasetView>::Error, <D as FallibleDatasetView>::Evidence>,
>;

pub(crate) type Row<I = u32> = (I, I, I, Option<I>);
pub(crate) type Triple = (u32, u32, u32);

const PROGRAM_DOMAIN: Domain = Domain::new(b"purrdf-positive-preparation/v1");

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Counts {
    pub(crate) terms: usize,
    pub(crate) rows: usize,
    pub(crate) question_bytes: usize,
}

/// Only immutable native step handles are copied into later proofs.
#[derive(Debug, Clone)]
pub(crate) struct PreparationLayout {
    pub(crate) state: [u8; 32],
    steps: Vec<Shared<Vec<u8>>>,
    counts: Counts,
    pub(crate) live: usize,
}

impl PartialEq for PreparationLayout {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
            && self.steps.len() == other.steps.len()
            && self
                .steps
                .iter()
                .zip(&other.steps)
                .all(|(left, right)| **left == **right)
    }
}
impl Eq for PreparationLayout {}

/// The original admission is consulted before every concrete metadata growth.
pub(crate) struct Metadata {
    pub(crate) limit: usize,
    pub(crate) attempted: usize,
}
impl Admission for Metadata {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if bytes > self.limit {
            if self.attempted <= self.limit {
                self.attempted = bytes;
            }
            Err(StorageError::AdmissionFailed)
        } else {
            Ok(())
        }
    }
}

impl Metadata {
    pub(crate) fn cause(&self, error: StorageError) -> EntailError {
        if error == StorageError::AdmissionFailed && self.attempted > self.limit {
            retention("metadata bytes", self.attempted as u128, self.limit as u128)
        } else {
            EntailError::Storage(error)
        }
    }
}

impl PreparationLayout {
    pub(crate) fn new(state: DatasetStateDigest) -> Self {
        Self {
            state: *state.as_bytes(),
            steps: Vec::new(),
            counts: Counts::default(),
            live: 0,
        }
    }

    pub(crate) fn copy(
        &self,
        limits: PositiveWorldLimits,
        other_live: usize,
    ) -> Result<Self, EntailError> {
        let mut account = Metadata {
            limit: limits.metadata_bytes,
            attempted: 0,
        };
        let result = (|| {
            let mut memory = Memory::resume(
                &mut account,
                self.live
                    .checked_add(other_live)
                    .ok_or(StorageError::SizeOverflow)?,
            );
            let mut steps = Vec::new();
            for step in &self.steps {
                memory.push(&mut steps, step.clone())?;
            }
            let live = memory
                .admitted_bytes()
                .checked_sub(other_live)
                .and_then(|bytes| {
                    bytes.checked_sub(self.steps.capacity() * size_of::<Shared<Vec<u8>>>())
                })
                .ok_or(StorageError::SizeOverflow)?;
            Ok::<_, StorageError>(Self {
                state: self.state,
                steps,
                counts: self.counts,
                live,
            })
        })();
        result.map_err(|error| account.cause(error))
    }

    pub(crate) fn push(
        &mut self,
        counts: Counts,
        limits: PositiveWorldLimits,
        other_live: usize,
        encode: impl Fn(&mut dyn PreparationEncoding),
    ) -> Result<(), EntailError> {
        for (resource, old, amount, limit) in [
            ("history steps", self.steps.len(), 1, limits.history_steps),
            (
                "source terms",
                self.counts.terms,
                counts.terms,
                limits.source_terms,
            ),
            (
                "source rows and declarations",
                self.counts.rows,
                counts.rows,
                limits.source_rows,
            ),
            (
                "question bytes",
                self.counts.question_bytes,
                counts.question_bytes,
                limits.question_bytes,
            ),
        ] {
            let attempted = old as u128 + amount as u128;
            if attempted > limit as u128 {
                return Err(retention(resource, attempted, limit as u128));
            }
        }
        let mut size = PreparationSize::new();
        encode(&mut size);
        let size = size.0.ok_or(StorageError::SizeOverflow)?;
        let mut account = Metadata {
            limit: limits.metadata_bytes,
            attempted: 0,
        };
        let baseline = self
            .live
            .checked_add(other_live)
            .ok_or(StorageError::SizeOverflow)?;
        let result = (|| {
            let mut memory = Memory::resume(&mut account, baseline);
            let mut bytes = Vec::new();
            memory.reserve(&mut bytes, size)?;
            encode(&mut bytes);
            let mut inspected = self.counts;
            inspect_step(
                &bytes,
                self.steps.len(),
                &mut inspected,
                limits,
                &mut memory,
            )
            .map_err(|cause| match cause {
                DlProofError::Storage(error) => EntailError::Storage(error),
                cause => EntailError::PositiveUpdate(Box::new(
                    crate::reasoner::PositiveUpdateRefusal::ProofEncoding {
                        cause: Box::new(cause),
                    },
                )),
            })?;
            memory.add_bytes(Shared::<Vec<u8>>::allocation_layout().size())?;
            let step = Shared::try_new(bytes).map_err(|_| StorageError::AllocationFailed)?;
            memory.push(&mut self.steps, step)?;
            self.live = memory
                .admitted_bytes()
                .checked_sub(other_live)
                .ok_or(StorageError::SizeOverflow)?;
            self.counts.terms += counts.terms;
            self.counts.rows += counts.rows;
            self.counts.question_bytes += counts.question_bytes;
            Ok::<_, EntailError>(())
        })();
        result.map_err(|error| match error {
            EntailError::Storage(error) => account.cause(error),
            error => error,
        })
    }

    pub(crate) fn question(
        &mut self,
        question: &Question,
        limits: PositiveWorldLimits,
        other_live: usize,
    ) -> Result<(), EntailError> {
        let mut size = PreparationSize::new();
        encode_question(&mut size, question);
        let size = size.0.ok_or(StorageError::SizeOverflow)?;
        self.push(
            Counts {
                question_bytes: size,
                ..Counts::default()
            },
            limits,
            other_live,
            |out| {
                out.push(2);
                encode_question(out, question);
            },
        )
    }

    pub(crate) fn stamp(&self, contract: [u8; 32]) -> [u8; 32] {
        let mut hash = blake3::Hasher::new();
        hash.update(PROGRAM_DOMAIN.as_bytes());
        hash.update(&self.state);
        for step in &self.steps {
            frame_le_into(&mut hash, step);
        }
        hash.update(&contract);
        *hash.finalize().as_bytes()
    }

    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.state);
        length(out, self.steps.len());
        for step in &self.steps {
            purrdf_hash::frame::frame_le(out, step);
        }
    }

    pub(crate) fn decode(
        reader: &mut Reader<'_>,
        limits: PositiveWorldLimits,
    ) -> Result<Self, DlProofError> {
        let state = reader.digest()?;
        let count = reader.length()?;
        if count > limits.history_steps {
            return Err(malformed(
                "positive preparation exceeds consumer history ceiling",
            ));
        }
        let mut account = Metadata {
            limit: limits.metadata_bytes,
            attempted: 0,
        };
        let mut memory = Memory::new(&mut account);
        let mut steps = Vec::new();
        let mut counts = Counts::default();
        for ordinal in 0..count {
            let bytes = reader.frame()?;
            if bytes.len() > limits.metadata_bytes {
                return Err(malformed(
                    "positive preparation step exceeds consumer metadata byte ceiling",
                ));
            }
            inspect_step(bytes, ordinal, &mut counts, limits, &mut memory)?;
            let mut owned = Vec::new();
            memory.reserve(&mut owned, bytes.len())?;
            owned.extend_from_slice(bytes);
            memory.add_bytes(Shared::<Vec<u8>>::allocation_layout().size())?;
            let step = Shared::try_new(owned).map_err(|_| StorageError::AllocationFailed)?;
            memory.push(&mut steps, step)?;
        }
        if steps.is_empty() {
            return Err(malformed("positive preparation has no initial source"));
        }
        Ok(Self {
            state,
            steps,
            counts,
            live: memory.admitted_bytes(),
        })
    }
}

/// Structural validation shares the native term/question decoders. It runs
/// before retaining each payload, with aggregate counts across every step.
fn inspect_step<S: Admission + ?Sized>(
    bytes: &[u8],
    ordinal: usize,
    totals: &mut Counts,
    limits: PositiveWorldLimits,
    memory: &mut Memory<'_, S>,
) -> Result<(), DlProofError> {
    let mut reader = Reader::new(bytes);
    let kind = reader.byte()?;
    if kind == 2 {
        if ordinal == 0 {
            return Err(malformed("positive question precedes initial source"));
        }
        totals.question_bytes = totals
            .question_bytes
            .checked_add(bytes.len() - 1)
            .ok_or(StorageError::SizeOverflow)?;
        if totals.question_bytes > limits.question_bytes {
            return Err(malformed(
                "positive preparation exceeds aggregate question byte ceiling",
            ));
        }
        let baseline = memory.admitted_bytes();
        drop(decode_question_with_memory(&mut reader, memory)?);
        memory.release_bytes(memory.admitted_bytes() - baseline)?;
    } else {
        if kind > 1 || (kind == 0) != (ordinal == 0) {
            return Err(malformed("invalid positive initial/append source order"));
        }
        let count = reader.length()?;
        totals.terms = totals
            .terms
            .checked_add(count)
            .ok_or(StorageError::SizeOverflow)?;
        if totals.terms > limits.source_terms {
            return Err(malformed(
                "positive preparation exceeds aggregate term binding ceiling",
            ));
        }
        for _ in 0..count {
            reader.u32()?;
            let baseline = memory.admitted_bytes();
            drop(decode_term_with_memory(&mut reader, memory)?);
            memory.release_bytes(memory.admitted_bytes() - baseline)?;
        }
        let count = reader.length()?;
        totals.rows = totals
            .rows
            .checked_add(count)
            .ok_or(StorageError::SizeOverflow)?;
        if totals.rows > limits.source_rows {
            return Err(malformed(
                "positive preparation exceeds aggregate source row ceiling",
            ));
        }
        for _ in 0..count {
            if reader.byte()? > 2 {
                return Err(malformed("unknown positive statement family"));
            }
            row(&mut reader)?;
        }
        let count = reader.length()?;
        totals.rows = totals
            .rows
            .checked_add(count)
            .ok_or(StorageError::SizeOverflow)?;
        if totals.rows > limits.source_rows {
            return Err(malformed(
                "positive preparation exceeds aggregate declaration ceiling",
            ));
        }
        for _ in 0..count {
            reader.u32()?;
        }
        let count = reader.length()?;
        if count > totals.rows {
            return Err(malformed("positive lowering exceeds its source coverage"));
        }
        for _ in 0..count {
            reader.u32()?;
            reader.u32()?;
            reader.u32()?;
        }
    }
    if !reader.is_exhausted() {
        return Err(malformed("trailing positive preparation bytes"));
    }
    Ok(())
}

pub(crate) fn encode_row(out: &mut dyn PreparationEncoding, row: Row) {
    for term in [row.0, row.1, row.2] {
        out.extend_from_slice(&term.to_le_bytes());
    }
    out.push(u8::from(row.3.is_some()));
    if let Some(graph) = row.3 {
        out.extend_from_slice(&graph.to_le_bytes());
    }
}

fn row(reader: &mut Reader<'_>) -> Result<Row, DlProofError> {
    Ok((
        reader.u32()?,
        reader.u32()?,
        reader.u32()?,
        if reader.flag()? {
            Some(reader.u32()?)
        } else {
            None
        },
    ))
}

enum Step<I> {
    Source {
        initial: bool,
        bindings: Vec<(u32, I)>,
        rows: Vec<(u8, Row)>,
        graphs: Vec<u32>,
        lowering: Vec<Triple>,
    },
    Question(usize),
}

struct Work<'a> {
    limits: PositiveWorldLimits,
    spent: u64,
    stop: Option<&'a dyn StopSignal>,
}
impl Work<'_> {
    fn charge<E, V>(&mut self, amount: usize) -> Result<(), PositiveWorldError<E, V>> {
        if self.stop.is_some_and(StopSignal::stopped) {
            return Err(PositiveWorldError::Entail(EntailError::Stopped));
        }
        let next = u128::from(self.spent) + amount as u128;
        if next > u128::from(self.limits.metadata_work) {
            return Err(PositiveWorldError::Limit {
                resource: "metadata work",
                attempted: next,
                limit: u128::from(self.limits.metadata_work),
            });
        }
        self.spent = u64::try_from(next).map_err(|_| {
            PositiveWorldError::Entail(EntailError::Storage(StorageError::SizeOverflow))
        })?;
        Ok(())
    }
}

fn bad<D: FallibleDatasetView>(detail: &'static str) -> PositiveWorldError<D::Error, D::Evidence> {
    PositiveWorldError::Proof(malformed(detail).with_read_error())
}

fn bounded<D: FallibleDatasetView>(
    resource: &'static str,
    old: usize,
    amount: usize,
    limit: usize,
) -> Checked<D, usize> {
    let attempted = old as u128 + amount as u128;
    if attempted > limit as u128 {
        return Err(PositiveWorldError::Limit {
            resource,
            attempted,
            limit: limit as u128,
        });
    }
    usize::try_from(attempted)
        .map_err(|_| PositiveWorldError::Entail(EntailError::Storage(StorageError::SizeOverflow)))
}

fn storage<D: FallibleDatasetView>(
    error: StorageError,
) -> PositiveWorldError<D::Error, D::Evidence> {
    PositiveWorldError::Entail(EntailError::Storage(error))
}

impl PreparationLayout {
    pub(crate) fn prepare<D: FallibleDatasetView>(
        &self,
        source: &D,
        question: &Question,
        limits: PositiveWorldLimits,
        schema: SchemaPreparationBudget,
        stop: Option<Arc<dyn StopSignal>>,
        control: &mut CanonicalizationControl<'_>,
    ) -> Checked<D, DlProofContext<'static>> {
        if stop.as_deref().is_some_and(StopSignal::stopped) {
            return Err(PositiveWorldError::Entail(EntailError::Stopped));
        }
        let state =
            DatasetStateDigest::from_view(source, control).map_err(PositiveWorldError::State)?;
        if state.as_bytes() != &self.state {
            return Err(PositiveWorldError::Proof(DlProofError::InputMismatch {
                expected: state.to_hex(),
                stated: purrdf_hash::hex::encode(&self.state),
            }));
        }
        let final_stop = stop.clone();
        let mut context = purrdf_core::checkpointed_drain(source, |source| {
            self.replay(source, question, limits, schema, stop, control)
        })
        .map_err(PositiveWorldError::Drain)??;
        if final_stop.as_deref().is_some_and(StopSignal::stopped) {
            return Err(PositiveWorldError::Entail(EntailError::Stopped));
        }
        let stamp = self.stamp(context.contract);
        if final_stop.as_deref().is_some_and(StopSignal::stopped) {
            return Err(PositiveWorldError::Entail(EntailError::Stopped));
        }
        context.positive_stamp = Some(stamp);
        Ok(context)
    }

    fn replay<D: FallibleDatasetView>(
        &self,
        source: &D,
        question: &Question,
        limits: PositiveWorldLimits,
        schema: SchemaPreparationBudget,
        stop: Option<Arc<dyn StopSignal>>,
        control: &mut CanonicalizationControl<'_>,
    ) -> Checked<D, DlProofContext<'static>> {
        bounded::<D>("history steps", 0, self.steps.len(), limits.history_steps)?;
        let signal = stop.clone();
        let mut native_stop = stop;
        let mut work = Work {
            limits,
            spent: 0,
            stop: signal.as_deref(),
        };
        // Scratch is admitted by both the caller's replay ceiling and the real
        // provider. The adapter below preserves its first physical refusal.
        let reservation = source
            .reserve_workspace(0)
            .map_err(|error| PositiveWorldError::Entail(EntailError::SourceRead(error)))?;
        let mut account = ReplayStorage {
            reservation,
            limit: limits.metadata_bytes,
            error: None,
            refused: None,
        };
        let result = (|| {
            let mut memory = Memory::new(&mut account);
            memory.add_bytes(self.live).map_err(storage::<D>)?;
            let mut aliases = OrderedTable::<u32, D::Id>::default();
            let mut reverse = OrderedTable::<D::Id, u32>::default();
            let mut used = OrderedTable::<u32, bool>::default();
            let mut coverage = OrderedTable::<(u8, Row<D::Id>), bool>::default();
            let mut declarations = OrderedTable::<D::Id, bool>::default();
            let mut parsed = Vec::new();
            let mut totals = Counts::default();
            for (ordinal, encoded) in self.steps.iter().enumerate() {
                work.charge::<D::Error, D::Evidence>(
                    encoded
                        .len()
                        .checked_add(1)
                        .ok_or_else(|| storage::<D>(StorageError::SizeOverflow))?,
                )?;
                let mut reader = Reader::new(encoded);
                let kind = reader
                    .byte()
                    .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?;
                if kind == 2 {
                    if ordinal == 0 {
                        return Err(bad::<D>(
                            "positive preparation must start with source ingress",
                        ));
                    }
                    totals.question_bytes = bounded::<D>(
                        "question bytes",
                        totals.question_bytes,
                        encoded.len() - 1,
                        limits.question_bytes,
                    )?;
                    // The native decoder establishes the exact syntactic question;
                    // it installs no theory, assumptions, decisions or budgets.
                    let baseline = memory.admitted_bytes();
                    drop(
                        decode_question_with_memory(&mut reader, &mut memory)
                            .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?,
                    );
                    memory
                        .release_bytes(memory.admitted_bytes() - baseline)
                        .map_err(storage::<D>)?;
                    if !reader.is_exhausted() {
                        return Err(bad::<D>("trailing positive question bytes"));
                    }
                    memory
                        .push(&mut parsed, Step::Question(ordinal))
                        .map_err(storage::<D>)?;
                    continue;
                }
                if kind > 1 || (kind == 0) != (ordinal == 0) {
                    return Err(bad::<D>(
                        "positive source initial/append step order is invalid",
                    ));
                }
                let count = reader
                    .length()
                    .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?;
                totals.terms =
                    bounded::<D>("source terms", totals.terms, count, limits.source_terms)?;
                let mut bindings = Vec::new();
                for _ in 0..count {
                    work.charge::<D::Error, D::Evidence>(1)?;
                    let slot = reader
                        .u32()
                        .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?;
                    let baseline = memory.admitted_bytes();
                    let term = decode_term_with_memory(&mut reader, &mut memory)
                        .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?;
                    let id = source
                        .term_id_by_value(&term)
                        .map_err(|error| {
                            PositiveWorldError::Entail(EntailError::SourceRead(error))
                        })?
                        .ok_or_else(|| {
                            bad::<D>("positive source binding is absent from the consumer source")
                        })?;
                    drop(term);
                    memory
                        .release_bytes(memory.admitted_bytes() - baseline)
                        .map_err(storage::<D>)?;
                    if aliases.get(&slot).is_some() || reverse.get(&id).is_some() {
                        return Err(bad::<D>("duplicate positive term binding"));
                    }
                    aliases
                        .insert(slot, id, &mut memory)
                        .map_err(storage::<D>)?;
                    reverse
                        .insert(id, slot, &mut memory)
                        .map_err(storage::<D>)?;
                    used.insert(slot, false, &mut memory)
                        .map_err(storage::<D>)?;
                    memory
                        .push(&mut bindings, (slot, id))
                        .map_err(storage::<D>)?;
                }
                let count = reader
                    .length()
                    .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?;
                totals.rows = bounded::<D>("source rows", totals.rows, count, limits.source_rows)?;
                let mut rows = Vec::new();
                let mut defaults = OrderedTable::default();
                for _ in 0..count {
                    work.charge::<D::Error, D::Evidence>(1)?;
                    let family = reader
                        .byte()
                        .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?;
                    if family > 2 {
                        return Err(bad::<D>("unknown positive statement family"));
                    }
                    let native = row(&mut reader)
                        .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?;
                    let resolved = resolve_row::<D>(native, &aliases, &mut used)?;
                    if coverage.get(&(family, resolved)).is_some() {
                        return Err(bad::<D>("duplicate positive source row"));
                    }
                    coverage
                        .insert((family, resolved), false, &mut memory)
                        .map_err(storage::<D>)?;
                    if family == 0 && native.3.is_none() {
                        defaults
                            .insert((native.0, native.1, native.2), false, &mut memory)
                            .map_err(storage::<D>)?;
                    }
                    memory
                        .push(&mut rows, (family, native))
                        .map_err(storage::<D>)?;
                }
                let count = reader
                    .length()
                    .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?;
                totals.rows = bounded::<D>(
                    "source rows and declarations",
                    totals.rows,
                    count,
                    limits.source_rows,
                )?;
                let mut graphs = Vec::new();
                for _ in 0..count {
                    work.charge::<D::Error, D::Evidence>(1)?;
                    let graph = reader
                        .u32()
                        .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?;
                    let id = *aliases
                        .get(&graph)
                        .ok_or_else(|| bad::<D>("positive declaration has no source binding"))?;
                    *used.get_mut(&graph).ok_or_else(|| {
                        bad::<D>("positive declaration source membership is absent")
                    })? = true;
                    if declarations.get(&id).is_some() {
                        return Err(bad::<D>("duplicate positive named declaration"));
                    }
                    declarations
                        .insert(id, false, &mut memory)
                        .map_err(storage::<D>)?;
                    memory.push(&mut graphs, graph).map_err(storage::<D>)?;
                }
                for &(slot, _) in &bindings {
                    work.charge::<D::Error, D::Evidence>(1)?;
                    if used.get(&slot) != Some(&true) {
                        return Err(bad::<D>(
                            "positive binding has no reaching source row or declaration in its ingress step",
                        ));
                    }
                }
                let count = reader
                    .length()
                    .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?;
                if count != defaults.len() {
                    return Err(bad::<D>(
                        "positive default lowering does not cover its source step",
                    ));
                }
                let mut lowering = Vec::new();
                for _ in 0..count {
                    work.charge::<D::Error, D::Evidence>(1)?;
                    let triple = (
                        reader
                            .u32()
                            .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?,
                        reader
                            .u32()
                            .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?,
                        reader
                            .u32()
                            .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?,
                    );
                    let seen = defaults.get_mut(&triple).ok_or_else(|| {
                        bad::<D>("positive lowering row is not a new ordinary default statement")
                    })?;
                    if *seen {
                        return Err(bad::<D>("duplicate positive default lowering row"));
                    }
                    *seen = true;
                    memory.push(&mut lowering, triple).map_err(storage::<D>)?;
                }
                if !reader.is_exhausted() {
                    return Err(bad::<D>("trailing positive source preparation bytes"));
                }
                memory
                    .push(
                        &mut parsed,
                        Step::Source {
                            initial: kind == 0,
                            bindings,
                            rows,
                            graphs,
                            lowering,
                        },
                    )
                    .map_err(storage::<D>)?;
            }
            if parsed.is_empty() {
                return Err(bad::<D>("positive preparation has no initial source"));
            }
            for (family, quad) in source
                .quads()
                .map(|q| (0, q))
                .chain(source.reifier_quads().map(|q| (1, q)))
                .chain(source.annotation_quads().map(|q| (2, q)))
            {
                work.charge::<D::Error, D::Evidence>(1)?;
                let seen = coverage
                    .get_mut(&(family, (quad.s, quad.p, quad.o, quad.g)))
                    .ok_or_else(|| {
                        bad::<D>("positive preparation omits a consumer source statement")
                    })?;
                *seen = true;
            }
            for graph in source.named_graphs() {
                work.charge::<D::Error, D::Evidence>(1)?;
                *declarations.get_mut(&graph).ok_or_else(|| {
                    bad::<D>("positive preparation omits a consumer named declaration")
                })? = true;
            }
            if coverage.iter().any(|(_, seen)| !seen)
                || declarations.iter().any(|(_, seen)| !seen)
                || used.iter().any(|(_, used)| !used)
            {
                return Err(bad::<D>(
                    "positive preparation contains extra source rows, declarations or bindings",
                ));
            }
            work.charge::<D::Error, D::Evidence>(0)?;
            let mut rows = Vec::new();
            let mut graphs = Vec::new();
            let mut defaults = Vec::new();
            let mut reasoner: Option<Reasoner> = None;
            let mut ingress = None;
            for step in &parsed {
                work.charge::<D::Error, D::Evidence>(1)?;
                match step {
                    Step::Question(ordinal) => {
                        let mut reader = Reader::new(&self.steps[*ordinal][1..]);
                        let baseline = memory.admitted_bytes();
                        let question = decode_question_with_memory(&mut reader, &mut memory)
                            .map_err(|error| PositiveWorldError::Proof(error.with_read_error()))?;
                        reasoner
                            .as_mut()
                            .ok_or_else(|| bad::<D>("query precedes native source preparation"))?
                            .prepare(&question)
                            .map_err(|error: EntailError| {
                                PositiveWorldError::Entail(error.with_read_error())
                            })?;
                        drop(question);
                        memory
                            .release_bytes(memory.admitted_bytes() - baseline)
                            .map_err(storage::<D>)?;
                    }
                    Step::Source {
                        initial,
                        bindings,
                        rows: delta,
                        graphs: new_graphs,
                        lowering,
                    } => {
                        for &(family, row) in delta {
                            let row = lookup_row::<D>(row, &aliases)?;
                            memory
                                .push(&mut rows, (family, row))
                                .map_err(storage::<D>)?;
                        }
                        for graph in new_graphs {
                            memory
                                .push(
                                    &mut graphs,
                                    *aliases.get(graph).ok_or_else(|| {
                                        bad::<D>("unbound graph during native replay")
                                    })?,
                                )
                                .map_err(storage::<D>)?;
                        }
                        for &(s, p, o) in lowering {
                            memory
                                .push(&mut defaults, lookup_row::<D>((s, p, o, None), &aliases)?)
                                .map_err(storage::<D>)?;
                        }
                        let revision = Revision {
                            source,
                            rows: &rows,
                            graphs: &graphs,
                            defaults: &defaults,
                        };
                        let input =
                            super::try_ontology_identity(&revision, control).map_err(|error| {
                                PositiveWorldError::Entail(EntailError::Canonicalization(error))
                            })?;
                        if *initial {
                            let (native, mut parsed) = Reasoner::build_positive_prepared(
                                &revision,
                                Some(input),
                                schema,
                                native_stop.take(),
                            )?;
                            drop(std::mem::take(&mut parsed.initial_order));
                            reasoner = Some(native);
                            ingress = Some(parsed);
                        }
                        let native = reasoner
                            .as_mut()
                            .ok_or_else(|| bad::<D>("append precedes native source preparation"))?;
                        for &(slot, id) in bindings {
                            work.charge::<D::Error, D::Evidence>(1)?;
                            let term =
                                source.term_value_with_storage(id, EntailError::from, |_| {
                                    EntailError::Storage(StorageError::AllocationFailed)
                                })?;
                            let actual = native.bind_positive(term).map_err(|error| {
                                PositiveWorldError::Entail(error.with_read_error())
                            })?;
                            if actual != slot {
                                return Err(bad::<D>(
                                    "recorded positive source slot differs from native replay",
                                ));
                            }
                        }
                        if !initial {
                            native
                                .append_positive(
                                    ingress.as_mut().ok_or_else(|| {
                                        bad::<D>("native append ingress is absent")
                                    })?,
                                    lowering,
                                    Some(input),
                                )
                                .map_err(|error| {
                                    PositiveWorldError::Entail(error.with_read_error())
                                })?;
                        }
                    }
                }
            }
            let mut reasoner =
                reasoner.ok_or_else(|| bad::<D>("native positive source preparation is absent"))?;
            reasoner
                .prepare(question)
                .map_err(|error| PositiveWorldError::Entail(error.with_read_error()))?;
            let context = reasoner
                .proof_context()
                .map_err(|error| PositiveWorldError::Entail(error.with_read_error()))?;
            work.charge::<D::Error, D::Evidence>(0)?;
            Ok(context)
        })();
        if let Some(error) = account.error {
            return Err(PositiveWorldError::Entail(EntailError::SourceRead(error)));
        }
        if let Some(attempted) = account.refused {
            return Err(PositiveWorldError::Limit {
                resource: "metadata bytes",
                attempted: attempted as u128,
                limit: limits.metadata_bytes as u128,
            });
        }
        result
    }
}

fn lookup_row<D: FallibleDatasetView>(
    row: Row,
    aliases: &OrderedTable<u32, D::Id>,
) -> Checked<D, Row<D::Id>> {
    let id = |slot| {
        aliases
            .get(&slot)
            .copied()
            .ok_or_else(|| bad::<D>("positive row uses an unbound source slot"))
    };
    Ok((
        id(row.0)?,
        id(row.1)?,
        id(row.2)?,
        row.3.map(id).transpose()?,
    ))
}

fn resolve_row<D: FallibleDatasetView>(
    row: Row,
    aliases: &OrderedTable<u32, D::Id>,
    used: &mut OrderedTable<u32, bool>,
) -> Checked<D, Row<D::Id>> {
    let resolved = lookup_row::<D>(row, aliases)?;
    for slot in [Some(row.0), Some(row.1), Some(row.2), row.3]
        .into_iter()
        .flatten()
    {
        *used
            .get_mut(&slot)
            .ok_or_else(|| bad::<D>("positive row source membership is absent"))? = true;
    }
    Ok(resolved)
}

struct ReplayStorage<R: purrdf_core::WorkspaceReservation> {
    reservation: R,
    limit: usize,
    error: Option<R::Error>,
    refused: Option<usize>,
}
impl<R: purrdf_core::WorkspaceReservation> Admission for ReplayStorage<R> {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if bytes > self.limit {
            self.refused = Some(bytes);
            return Err(StorageError::AdmissionFailed);
        }
        if let Err(error) = self
            .reservation
            .resize(u64::try_from(bytes).map_err(|_| StorageError::SizeOverflow)?)
        {
            if self.error.is_none() {
                self.error = Some(error);
            }
            return Err(StorageError::AdmissionFailed);
        }
        Ok(())
    }
}

/// A historical projection of validated consumer rows; all physical capabilities
/// remain the original provider's. It owns no producer dataset or theory.
struct Revision<'a, D: DatasetView> {
    source: &'a D,
    rows: &'a [(u8, Row<D::Id>)],
    graphs: &'a [D::Id],
    defaults: &'a [Row<D::Id>],
}

impl<D: DatasetView> Revision<'_, D> {
    fn family(&self, selected: u8) -> impl Iterator<Item = QuadIds<D::Id>> + '_ {
        self.rows
            .iter()
            .filter_map(move |&(family, row)| (family == selected).then_some(quad(row)))
    }
}
impl<D: DatasetView> DatasetView for Revision<'_, D> {
    type Id = D::Id;
    type ReadError = D::ReadError;
    type TermGuard<'a>
        = D::TermGuard<'a>
    where
        Self: 'a;
    type ProbePlan = ();
    fn read_error(&self) -> Option<Self::ReadError> {
        self.source.read_error()
    }
    fn resolve(&self, id: Self::Id) -> Result<Self::TermGuard<'_>, Self::ReadError> {
        self.source.resolve(id)
    }
    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<Self::Id>, Self::ReadError> {
        self.source.term_id_by_value(value)
    }
    fn quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.defaults
            .iter()
            .copied()
            .chain(
                self.rows
                    .iter()
                    .filter_map(|&(family, row)| (family == 0 && row.3.is_some()).then_some(row)),
            )
            .map(quad)
    }
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.family(1)
    }
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.family(2)
    }
    fn named_graphs(&self) -> impl Iterator<Item = Self::Id> + '_ {
        self.graphs.iter().copied()
    }
    fn capabilities(&self) -> purrdf_core::RdfStoreCapabilities {
        self.source.capabilities()
    }
    fn term_count(&self) -> u64 {
        self.source.term_count()
    }
    fn probe_plan(&self, _s: bool, _p: bool, _o: bool, _g: purrdf_core::GraphMatch<Self::Id>) {}
    fn quads_for_pattern_with_plan(
        &self,
        _plan: &(),
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: purrdf_core::GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.quads().filter(move |q| {
            s.is_none_or(|s| q.s == s)
                && p.is_none_or(|p| q.p == p)
                && o.is_none_or(|o| q.o == o)
                && g.matches(q.g)
        })
    }
    fn supports_named_graphs(&self) -> bool {
        self.source.supports_named_graphs()
    }
    fn require_named_graphs(&self) -> Result<(), Self::ReadError> {
        self.source.require_named_graphs()
    }
    fn reserve_workspace(
        &self,
        bytes: u64,
    ) -> Result<impl purrdf_core::WorkspaceReservation<Error = Self::ReadError> + '_, Self::ReadError>
    {
        self.source.reserve_workspace(bytes)
    }
    fn reserve_owned_workspace(
        &self,
        bytes: u64,
    ) -> Option<Result<purrdf_core::OwnedWorkspaceReservation<Self::ReadError>, Self::ReadError>>
    {
        self.source.reserve_owned_workspace(bytes)
    }
    fn max_owned_term_bytes(&self) -> Option<u64> {
        self.source.max_owned_term_bytes()
    }
    fn storage_live_budget(&self) -> Option<u64> {
        self.source.storage_live_budget()
    }
    fn triple_term_nesting_bound(&self) -> Option<usize> {
        self.source.triple_term_nesting_bound()
    }
}

fn quad<I>(row: Row<I>) -> QuadIds<I> {
    QuadIds {
        s: row.0,
        p: row.1,
        o: row.2,
        g: row.3,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::reasoner::{PositiveWorld, ServiceProof};
    use purrdf_core::term_fixture::{build_page, iri};
    use purrdf_iri::vocab::{rdf, rdfs};

    fn control() -> CanonicalizationControl<'static> {
        CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None)
    }

    /// A producer-supplied preparation program cannot change source membership,
    /// native slots, lowering order or the exact verified preparation stamp.
    #[test]
    fn hostile_layout_membership_and_coordinate_programs_are_refused() {
        let limits = PositiveWorldLimits {
            history_steps: 16,
            source_terms: 256,
            source_rows: 256,
            question_bytes: 4096,
            metadata_bytes: 1_048_576,
            metadata_work: 1_000_000,
        };
        let source = build_page(&[
            (iri("A"), TermValue::iri(rdfs::SUB_CLASS_OF), iri("B")),
            (iri("a"), TermValue::iri(rdf::TYPE), iri("A")),
            (iri("b"), TermValue::iri(rdf::TYPE), iri("B")),
        ])
        .unwrap();
        let world = PositiveWorld::with_proofs(source.clone(), limits, &mut control()).unwrap();
        let proof = world.consistency().unwrap().proof().unwrap().clone();
        let encoded = proof.encode();
        let mut envelope = Reader::new(&encoded);
        envelope.frame().unwrap();
        envelope.frame().unwrap();
        let layout = PreparationLayout::decode(&mut envelope, limits).unwrap();
        let source_step = layout.steps[0].as_ref().clone();
        let mut reader = Reader::new(&source_step);
        assert_eq!(reader.byte().unwrap(), 0);
        let bindings = reader.length().unwrap();
        let mut slots = Vec::new();
        for _ in 0..bindings {
            slots.push(source_step.len() - reader.bytes.len());
            reader.u32().unwrap();
            crate::reasoner::proof::decode_term(&mut reader).unwrap();
        }
        let count_offset = source_step.len() - reader.bytes.len();
        let row_count = reader.length().unwrap();
        let mut row_ranges = Vec::new();
        for _ in 0..row_count {
            let start = source_step.len() - reader.bytes.len();
            reader.byte().unwrap();
            row(&mut reader).unwrap();
            row_ranges.push(start..source_step.len() - reader.bytes.len());
        }
        for _ in 0..reader.length().unwrap() {
            reader.u32().unwrap();
        }
        let lowering_count = reader.length().unwrap();
        assert!(lowering_count > 1);
        let lowering_offset = source_step.len() - reader.bytes.len();
        let mut mutations = Vec::new();
        let mut duplicate_slot = source_step.clone();
        let first = duplicate_slot[slots[0]..slots[0] + 4].to_vec();
        duplicate_slot[slots[1]..slots[1] + 4].copy_from_slice(&first);
        mutations.push(duplicate_slot);
        let mut swapped_slots = source_step.clone();
        for byte in 0..4 {
            swapped_slots.swap(slots[0] + byte, slots[1] + byte);
        }
        mutations.push(swapped_slots);
        let mut duplicate_row = source_step.clone();
        duplicate_row.splice(
            row_ranges[0].end..row_ranges[0].end,
            source_step[row_ranges[0].clone()].iter().copied(),
        );
        duplicate_row[count_offset..count_offset + 8]
            .copy_from_slice(&((row_count + 1) as u64).to_le_bytes());
        mutations.push(duplicate_row);
        let mut extra_row = source_step.clone();
        let mut extra = source_step[row_ranges[0].clone()].to_vec();
        extra[0] = 1;
        extra_row.splice(row_ranges[0].end..row_ranges[0].end, extra);
        extra_row[count_offset..count_offset + 8]
            .copy_from_slice(&((row_count + 1) as u64).to_le_bytes());
        mutations.push(extra_row);
        let mut omitted_family = source_step.clone();
        omitted_family[row_ranges[0].start] = 2;
        mutations.push(omitted_family);
        let mut swapped_lowering = source_step;
        for byte in 0..12 {
            swapped_lowering.swap(lowering_offset + byte, lowering_offset + 12 + byte);
        }
        mutations.push(swapped_lowering);
        for bytes in mutations {
            let mut forged = layout.clone();
            forged.steps[0] = Shared::try_new(bytes).unwrap();
            assert!(
                forged
                    .prepare(
                        &source,
                        &Question::Consistency,
                        limits,
                        SchemaPreparationBudget::default(),
                        None,
                        &mut control()
                    )
                    .is_err()
            );
        }
        // An empty successful syntactic preparation leaves native clauses alone,
        // but it is still part of the exact consumer-verified preparation stamp.
        let mut changed_history = layout;
        changed_history
            .steps
            .push(Shared::try_new(vec![2, 0]).unwrap());
        let context = changed_history
            .prepare(
                &source,
                &Question::Consistency,
                limits,
                SchemaPreparationBudget::default(),
                None,
                &mut control(),
            )
            .unwrap();
        assert!(
            proof
                .verify_prepared(&Question::Consistency, None, &context)
                .is_err()
        );
        assert!(
            proof
                .verify_prepared_causal(&Question::Consistency, None, &context)
                .is_err()
        );
        let mut empty_history = limits;
        empty_history.history_steps = 0;
        assert!(ServiceProof::decode_positive(&encoded, empty_history).is_err());
        let mut absent_terms = limits;
        absent_terms.source_terms = 0;
        assert!(ServiceProof::decode_positive(&encoded, absent_terms).is_err());
        let mut too_deep =
            PreparationLayout::new(DatasetStateDigest::from_view(&source, &mut control()).unwrap());
        let term = purrdf_core::term_fixture::triple_chain(super::super::MAX_NESTING + 1);
        assert!(
            matches!(too_deep.push(Counts { terms: 1, ..Counts::default() }, limits, 0, |out| {
            out.push(0);
            length(out, 1);
            out.extend_from_slice(&0_u32.to_le_bytes());
            crate::reasoner::proof::encode_term(out, &term);
            for _ in 0..3 { length(out, 0); }
        }), Err(EntailError::PositiveUpdate(refusal)) if matches!(*refusal, crate::reasoner::PositiveUpdateRefusal::ProofEncoding { .. }))
        );
    }
}
