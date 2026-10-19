// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Admission for the single immutable dataset projection algorithm.

use super::{GeoIndexConfig, Keyed};
use crate::context::{Retained, WorkProgress};
use crate::{GeoError, GeoVocab, GeometryLiteral, MetricContext, MetricWorkObserver, Rat};
use core::{cmp::Ordering, ops::ControlFlow};
use purrdf_core::{DatasetView, FastMap, TermLookupError, TermValue};
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};

pub(super) struct ProjectionWork<'a, 'observer> {
    admission: Option<(&'a mut MetricContext, WorkProgress<'observer>)>,
    resident: usize,
}

pub(super) use crate::context::native_memory;

impl ProjectionWork<'_, '_> {
    pub(super) fn plain() -> Self {
        Self {
            admission: None,
            resident: 0,
        }
    }

    fn memory<T>(
        &mut self,
        body: impl FnOnce(&mut Memory<'_, dyn Admission + '_>) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        match self.admission.as_mut() {
            Some((context, progress)) => native_memory(context, progress, &mut self.resident, body),
            None => {
                let mut resident = Resident;
                let mut memory = Memory::resume(&mut resident as &mut dyn Admission, self.resident);
                let result = body(&mut memory);
                self.resident = memory.admitted_bytes();
                result
            }
        }
    }
    pub(super) fn tick(&mut self, items: u64) -> Result<(), GeoError> {
        if let Some((context, progress)) = self.admission.as_mut() {
            context.charge_work(items)?;
            progress.context_poll(context)?;
        }
        Ok(())
    }
    pub(super) fn reserve<T>(&mut self, values: &mut Vec<T>, count: usize) -> Result<(), GeoError> {
        self.memory(|memory| memory.reserve(values, count).map_err(GeoError::Storage))
    }
    pub(super) fn push<T>(&mut self, values: &mut Vec<T>, value: T) -> Result<(), GeoError> {
        self.memory(|memory| memory.push(values, value).map_err(GeoError::Storage))
    }
    pub(super) fn release_vec<T>(&mut self, values: Vec<T>) -> Result<(), GeoError> {
        self.memory(|memory| memory.release_vec(values).map_err(GeoError::Storage))
    }
    pub(super) fn release_string(&mut self, value: String) -> Result<(), GeoError> {
        self.memory(|memory| memory.release_string(value).map_err(GeoError::Storage))
    }
    pub(super) fn release_vector_layout<T>(&mut self, capacity: usize) -> Result<(), GeoError> {
        let bytes = core::alloc::Layout::array::<T>(capacity)
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?
            .size();
        self.memory(|memory| memory.release_bytes(bytes).map_err(GeoError::Storage))
    }
    pub(super) fn release_term(&mut self, value: TermValue) -> Result<(), GeoError> {
        self.memory(|memory| value.release_with_memory(memory).map_err(GeoError::Storage))
    }
    pub(super) fn release_keyed(&mut self, keyed: Keyed) -> Result<(), GeoError> {
        self.release_string(keyed.key)?;
        match self.admission.as_mut() {
            Some(_) => self.memory(|memory| {
                keyed
                    .literal
                    .into_geometry()
                    .release_with_memory(memory)
                    .map_err(GeoError::Storage)
            }),
            None => {
                drop(keyed.literal);
                Ok(())
            }
        }
    }
    pub(super) fn output_count(&self, count: usize) -> Result<(), GeoError> {
        if let Some((context, _)) = &self.admission {
            let limit = context.policy().limits().max_output_elements;
            if count as u64 > limit {
                return Err(GeoError::OutputExhausted { limit });
            }
        }
        Ok(())
    }
    pub(super) fn map_lookup(&mut self, nodes: usize) -> Result<(), GeoError> {
        self.tick(
            u64::from(nodes.bit_width())
                .checked_add(1)
                .ok_or(GeoError::ArithmeticOverflow("projection map comparisons"))?,
        )
    }
    pub(super) fn map_entry<'map, Id: Eq + core::hash::Hash>(
        &mut self,
        map: &'map mut FastMap<Id, Vec<Keyed>>,
        id: Id,
    ) -> Result<&'map mut Vec<Keyed>, GeoError> {
        self.map_lookup(map.len())?;
        if !map.contains_key(&id) {
            let next = map
                .len()
                .checked_add(1)
                .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
            self.output_count(next)?;
            self.memory(|memory| {
                purrdf_core::hash::reserve_map_with_memory(map, next, memory)
                    .map_err(GeoError::Storage)
            })?;
        }
        Ok(map.entry(id).or_default())
    }
    pub(super) fn release_map<K, V>(&mut self, map: FastMap<K, V>) -> Result<(), GeoError> {
        let bytes = purrdf_core::hash::hash_table_allocation_bound::<(K, V)>(map.capacity())
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
        drop(map);
        self.memory(|memory| memory.release_bytes(bytes).map_err(GeoError::Storage))
    }
    pub(super) fn config(&mut self, config: &GeoIndexConfig) -> Result<GeoIndexConfig, GeoError> {
        self.memory(|memory| config.copy_with_memory(memory))
    }
    pub(super) fn iri(&mut self, text: impl core::fmt::Display) -> Result<TermValue, GeoError> {
        self.memory(|memory| {
            memory
                .format(&text)
                .map(TermValue::Iri)
                .map_err(GeoError::Storage)
        })
    }
    pub(super) fn relation_iri(
        &mut self,
        vocab: &GeoVocab,
        relation: super::SpatialRelation,
    ) -> Result<TermValue, GeoError> {
        self.memory(|memory| {
            memory
                .format(&format_args!(
                    "{}{}",
                    vocab.core_namespace(),
                    relation.local_name()
                ))
                .map(TermValue::Iri)
                .map_err(GeoError::Storage)
        })
    }
    pub(super) fn source_error(
        &mut self,
        error: impl purrdf_hash::error::NativeError + 'static,
    ) -> GeoError {
        match self.admission.as_mut() {
            Some((_, progress)) => GeoError::native_source_with_progress(error, progress),
            None => GeoError::native_source(error),
        }
    }
    pub(super) fn foreign_term_id(&mut self) -> GeoError {
        match self.admission.as_mut() {
            Some((_, progress)) => Retained::produce(
                progress,
                |_| Ok(crate::IndexBindingError::ForeignTermId),
                |_, _| Ok(()),
            )
            .map_or_else(|error| error, GeoError::IndexBinding),
            None => GeoError::resident_failure(
                crate::IndexBindingError::ForeignTermId,
                GeoError::IndexBinding,
            ),
        }
    }
    pub(super) fn invalid_serialization(
        &mut self,
        property: &TermValue,
        object: &TermValue,
    ) -> GeoError {
        match self.admission.as_mut() {
            None => {
                let mut resident = Resident;
                crate::IndexSerializationError::capture(
                    property,
                    object,
                    &mut Memory::new(&mut resident),
                    |_| Ok(()),
                )
                .unwrap_or_else(|error| error)
            }
            Some((context, progress)) => {
                let work = context.work_items();
                let peak = context.workspace_peak();
                progress
                    .with_native_storage(|memory, observer| {
                        // Preserve the existing computation prefix before visiting
                        // original term nodes. Capture uses the core's one admitted
                        // iterative copier, never a rendered diagnostic traversal.
                        observer.charge_chunk(work, peak)?;
                        let mut nested = WorkProgress::new(Some(observer));
                        let mut observer = SourceObserver {
                            context,
                            progress: &mut nested,
                        };
                        let cause = crate::IndexSerializationError::capture(
                            property,
                            object,
                            memory,
                            |_| observer.charge_chunk(1, 0),
                        )?;
                        Err::<GeoError, _>(cause)
                    })
                    .unwrap_or_else(|error| error)
            }
        }
    }
    pub(super) fn resolve<D: DatasetView<ReadError: purrdf_hash::error::StructuredError>>(
        &mut self,
        dataset: &D,
        id: D::Id,
    ) -> Result<TermValue, GeoError> {
        enum Failure<E> {
            Lookup(TermLookupError<E>),
            Storage(StorageError),
        }
        let mut failure = None;
        let value = self.memory(|memory| {
            match dataset.term_value_with_memory(id, memory, Failure::Lookup, |error, _| {
                Failure::Storage(error)
            }) {
                Ok(value) => Ok(Some(value)),
                Err(error) => {
                    failure = Some(error);
                    Ok(None)
                }
            }
        })?;
        match failure {
            None => {
                self.tick(1)?;
                Ok(value.expect("successful original term conversion"))
            }
            Some(Failure::Storage(error)) => Err(GeoError::Storage(error)),
            Some(Failure::Lookup(TermLookupError::Read(error))) => Err(self.source_error(error)),
            Some(Failure::Lookup(TermLookupError::ForeignId)) => Err(self.foreign_term_id()),
        }
    }
    pub(super) fn geometry_arg(
        &mut self,
        vocab: &GeoVocab,
        value: &TermValue,
    ) -> Result<GeometryLiteral, GeoError> {
        match self.admission.as_mut() {
            None => crate::carrier::geometry_arg(vocab, value),
            Some((context, progress)) => {
                let used =
                    context.policy().limits().max_workspace_bytes - context.remaining_workspace();
                let policy = context
                    .policy()
                    .remaining_after(context.work_items(), used)?;
                progress.with_native_storage(|memory, observer| {
                    let mut nested = WorkProgress::new(Some(observer));
                    let mut observer = SourceObserver {
                        context,
                        progress: &mut nested,
                    };
                    memory.with_account(|storage, admitted| {
                        let storage = crate::carrier::CarrierStorage::resume(storage, admitted);
                        let result = crate::carrier::geometry_arg_with_storage(
                            vocab,
                            value,
                            policy,
                            &mut observer,
                            &storage,
                        )
                        .map(crate::carrier::ParsedGeometry::into_literal);
                        (result, storage.admitted_bytes())
                    })
                })
            }
        }
    }
    pub(super) fn write(
        &mut self,
        literal: &GeometryLiteral,
        scale: u32,
    ) -> Result<String, GeoError> {
        match self.admission.as_mut() {
            None => {
                let output = crate::wkt::write(literal, scale);
                self.resident = self
                    .resident
                    .checked_add(output.capacity())
                    .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
                Ok(output)
            }
            Some((context, progress)) => {
                crate::wkt::write_admitted(literal, scale, context, progress)
            }
        }
    }
    pub(super) fn append(&mut self, out: &mut String, text: &str) -> Result<(), GeoError> {
        self.memory(|memory| memory.push_str(out, text).map_err(GeoError::Storage))?;
        self.tick((text.len() as u64).div_ceil(16))
    }
    pub(super) fn exact_ordinate(&mut self, out: &mut String, value: &Rat) -> Result<(), GeoError> {
        if let Some((context, progress)) = self.admission.as_mut() {
            let (fields, bytes) =
                crate::operation::rational_fields_admitted(value, context, progress)?;
            let result = (|| {
                self.append(out, "|")?;
                self.append(out, &fields[0])?;
                self.append(out, "/")?;
                self.append(out, &fields[1])
            })();
            drop(fields);
            let (context, _) = self.admission.as_mut().expect("admitted projection");
            let cleanup = context.release_workspace(bytes);
            match result {
                Err(original) => Err(original),
                Ok(()) => cleanup,
            }
        } else {
            self.append(out, "|")?;
            let numerator = value.numerator().to_string();
            let denominator = value.denominator().to_string();
            self.append(out, &numerator)?;
            self.append(out, "/")?;
            self.append(out, &denominator)
        }
    }
    pub(super) fn clone_keyed(&mut self, keyed: &Keyed) -> Result<Keyed, GeoError> {
        let literal = match self.admission.as_mut() {
            None => keyed.literal.clone(),
            Some((context, progress)) => {
                let mut limits = *context.policy().limits();
                limits.max_work_items = limits.max_work_items.saturating_sub(context.work_items());
                limits.max_workspace_bytes = context.remaining_workspace();
                let inventory = {
                    let mut observer = SourceObserver { context, progress };
                    crate::prepared::source_inventory(
                        keyed.literal.geometry(),
                        &limits,
                        Some(&mut observer),
                    )?
                };
                context.prepare_integer_scratch_for_observed(
                    inventory.maximum_original_operand_bits(),
                    progress,
                )?;
                let scratch = context
                    .integer_scratch()
                    .expect("prepared original coordinate-copy arena")
                    .clone();
                progress
                    .with_storage(0, |memory| {
                        keyed.literal.geometry().copy_with_memory(memory, &scratch)
                    })
                    .map(|geometry| GeometryLiteral::new(keyed.literal.crs().clone(), geometry))?
            }
        };
        let key = self.memory(|memory| memory.string(&keyed.key).map_err(GeoError::Storage))?;
        self.tick((key.len() as u64).div_ceil(16))?;
        Ok(Keyed { key, literal })
    }
    pub(super) fn compare_keys(&mut self, a: &Keyed, b: &Keyed) -> Result<Ordering, GeoError> {
        self.tick(
            (a.key.len() as u64)
                .checked_add(b.key.len() as u64)
                .ok_or(GeoError::ArithmeticOverflow("projection key comparison"))?
                .div_ceil(16)
                .checked_add(1)
                .ok_or(GeoError::ArithmeticOverflow("projection comparison work"))?,
        )?;
        Ok(a.key.cmp(&b.key))
    }
    pub(super) fn scan_term(&mut self, value: &TermValue) -> Result<(), GeoError> {
        self.absorb_term(value, |_| {})
    }
    pub(super) fn absorb_term(
        &mut self,
        value: &TermValue,
        mut absorb: impl FnMut(&TermValue),
    ) -> Result<(), GeoError> {
        match self.admission.as_mut() {
            None => {
                let ControlFlow::Continue(()) =
                    value.visit_terms(|term| -> ControlFlow<core::convert::Infallible> {
                        absorb(term);
                        ControlFlow::Continue(())
                    });
                Ok(())
            }
            Some((context, progress)) => progress.with_native_storage(|memory, observer| {
                let mut nested = WorkProgress::new(Some(observer));
                let flow = value
                    .visit_terms_with_memory(
                        |term, _| {
                            let bytes = term
                                .text_payload_bytes()
                                .ok_or(StorageError::SizeOverflow)?;
                            let work = (bytes as u64)
                                .div_ceil(16)
                                .checked_add(1)
                                .ok_or(StorageError::SizeOverflow)?;
                            let result = context
                                .charge_work(work)
                                .and_then(|()| nested.context_poll(context));
                            if let Err(error) = result {
                                return Ok(ControlFlow::Break(error));
                            }
                            absorb(term);
                            Ok(ControlFlow::Continue(()))
                        },
                        memory,
                    )
                    .map_err(GeoError::Storage)?;
                match flow {
                    ControlFlow::Continue(()) => Ok(()),
                    ControlFlow::Break(error) => Err(error),
                }
            }),
        }
    }
    pub(super) fn compare_terms(
        &mut self,
        a: &TermValue,
        b: &TermValue,
    ) -> Result<Ordering, GeoError> {
        self.scan_term(a)?;
        self.scan_term(b)?;
        self.memory(|memory| a.try_cmp_with_memory(b, memory).map_err(GeoError::Storage))
    }
    pub(super) fn publish<T>(&mut self, value: T) -> Result<Retained<T>, GeoError> {
        match self.admission.as_mut() {
            Some((context, progress)) => {
                context.admit_workspace(Retained::<T>::required_bytes(progress)?)?;
                progress.context_poll(context)?;
                Retained::publish_original(value, progress)
            }
            None => Retained::try_new(value),
        }
    }
    pub(super) fn fail(&mut self, error: GeoError) -> GeoError {
        match self.admission.as_mut() {
            Some((_, progress)) => progress.retain_failure(error),
            None => error,
        }
    }
    pub(super) fn finish(&mut self, complete: bool) -> Result<(), GeoError> {
        if let Some((context, progress)) = self.admission.as_mut() {
            // Projection values have been published into their original native
            // owners or destroyed. Do not promote an unused arithmetic arena
            // into the immutable index's preparation receipt.
            context.release_unused_integer_scratch()?;
            if complete {
                progress.context_poll(context)?;
                context.retain_current_preparation()?;
            } else {
                context.release_transient_workspace()?;
            }
        }
        Ok(())
    }
}
impl<'a, 'observer> ProjectionWork<'a, 'observer> {
    pub(super) fn admitted(
        context: &'a mut MetricContext,
        observer: Option<&'observer mut dyn MetricWorkObserver>,
    ) -> Self {
        Self {
            admission: Some((context, WorkProgress::new(observer))),
            resident: 0,
        }
    }
}

struct SourceObserver<'a, 'observer> {
    context: &'a mut MetricContext,
    progress: &'a mut WorkProgress<'observer>,
}
impl MetricWorkObserver for SourceObserver<'_, '_> {
    fn retain_failure(&mut self, error: GeoError) -> GeoError {
        self.progress.retain_failure(error)
    }
    fn native_storage(
        &mut self,
        producer: &mut dyn crate::NativeStorageProducer,
    ) -> Result<(), GeoError> {
        self.progress
            .with_native_storage(|memory, observer| producer.produce(memory, observer))
    }
    fn retain_current(
        &mut self,
        control: core::alloc::Layout,
    ) -> Result<(purrdf_lex::allocation::SharedOwned<()>, usize), GeoError> {
        self.progress.retain_current(control)
    }
    fn retained_owner_layout(&self) -> core::alloc::Layout {
        self.progress.retained_owner_layout()
    }

    fn retain_produced(
        &mut self,
        control: core::alloc::Layout,
        producer: &mut dyn crate::context::RetainedAllocationProducer,
    ) -> Result<purrdf_lex::allocation::SharedOwned<()>, GeoError> {
        self.progress.retain_produced(control, producer)
    }

    fn allocation(&mut self) -> Option<(&mut dyn Admission, usize)> {
        self.progress.allocation()
    }

    fn retain_text(&mut self, text: &str) -> Result<purrdf_lex::allocation::SharedText, GeoError> {
        self.progress.retain_text(text)
    }

    fn charge_chunk(&mut self, work: u64, storage: u64) -> Result<(), GeoError> {
        self.context.charge_work(work)?;
        self.context.admit_workspace(storage)?;
        self.progress.context_poll(self.context)
    }

    fn integer_scratch_layout(&self, buffers: usize, limbs: usize) -> Result<usize, GeoError> {
        self.progress.integer_scratch_layout(buffers, limbs)
    }

    fn prepare_integer_scratch(
        &mut self,
        buffers: usize,
        limbs: usize,
    ) -> Result<purrdf_xsd::integer::LimbScratch, GeoError> {
        self.progress.prepare_integer_scratch(buffers, limbs)
    }

    fn binary64_layout(&self) -> usize {
        self.progress.binary64_layout()
    }

    fn prepare_binary64(
        &mut self,
        math: &mut purrdf_xsd::math::CoordinateMath,
    ) -> Result<purrdf_xsd::math::PreparedBinary64, purrdf_xsd::math::MathError> {
        self.progress.forward_binary64(math)
    }

    fn taylor_scratch_layout(&self, coefficients: usize, jets: usize) -> Result<usize, GeoError> {
        self.progress.taylor_scratch_layout(coefficients, jets)
    }

    fn prepare_taylor_scratch(
        &mut self,
        coefficients: usize,
        jets: usize,
        zero: purrdf_xsd::math::FixedInterval,
    ) -> Result<purrdf_xsd::math::TaylorScratch, GeoError> {
        self.progress
            .prepare_taylor_scratch(coefficients, jets, zero)
    }
}

#[cfg(test)]
mod tests;
