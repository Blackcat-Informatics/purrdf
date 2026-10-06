// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Admission for the single immutable dataset projection algorithm.

use core::{cmp::Ordering, ops::ControlFlow};
use purrdf_core::{DatasetView, TermGuard, TermRef, TermValue, try_fold_nested};

use super::{GeoIndexConfig, Keyed, resolve_value};
use crate::context::WorkProgress;
use crate::{GeoError, GeoVocab, GeometryLiteral, MetricContext, MetricWorkObserver, Rat};

pub(super) struct ProjectionWork<'a, 'observer> {
    admission: Option<(&'a mut MetricContext, WorkProgress<'observer>)>,
}

impl ProjectionWork<'_, '_> {
    pub(super) fn plain() -> Self {
        Self { admission: None }
    }

    pub(super) fn tick(&mut self, items: u64) -> Result<(), GeoError> {
        if let Some((context, progress)) = self.admission.as_mut() {
            context.charge_work(items)?;
            progress.context_poll(context)?;
        }
        Ok(())
    }

    pub(super) fn storage(&mut self, bytes: u64) -> Result<(), GeoError> {
        if let Some((context, progress)) = self.admission.as_mut() {
            context.admit_workspace(bytes)?;
            progress.context_poll(context)?;
        }
        Ok(())
    }

    pub(super) fn reserve<T>(&mut self, count: usize) -> Result<(), GeoError> {
        self.storage((count as u64).checked_mul(size_of::<T>() as u64).ok_or(
            GeoError::ArithmeticOverflow("projection collection storage"),
        )?)
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

    pub(super) fn map_entry<'map, Id: Ord>(
        &mut self,
        map: &'map mut std::collections::BTreeMap<Id, Vec<Keyed>>,
        id: Id,
    ) -> Result<&'map mut Vec<Keyed>, GeoError> {
        self.map_lookup(map.len())?;
        let count = map.len();
        match map.entry(id) {
            std::collections::btree_map::Entry::Occupied(entry) => Ok(entry.into_mut()),
            std::collections::btree_map::Entry::Vacant(entry) => {
                self.output_count(
                    count
                        .checked_add(1)
                        .ok_or(GeoError::ArithmeticOverflow("projection map size"))?,
                )?;
                // Sixteen slots overbound an eleven-key B-tree node, its child
                // pointers and headers, before the insertion allocates it.
                let bytes = (size_of::<Id>() as u64)
                    .checked_add(size_of::<Vec<Keyed>>() as u64)
                    .and_then(|bytes| bytes.checked_mul(16))
                    .and_then(|bytes| bytes.checked_add(1024))
                    .ok_or(GeoError::ArithmeticOverflow("projection map node"))?;
                self.storage(bytes)?;
                Ok(entry.insert(Vec::new()))
            }
        }
    }

    pub(super) fn term(&mut self, value: &TermValue) -> Result<(), GeoError> {
        if let Some((context, progress)) = self.admission.as_mut() {
            super::geographic::term_inventory(value, context, progress)?;
        }
        Ok(())
    }

    pub(super) fn config(&mut self, config: &GeoIndexConfig) -> Result<(), GeoError> {
        self.reserve::<TermValue>(config.serializations.len())?;
        for property in &config.serializations {
            self.term(property)?;
        }
        if let super::GraphSelector::Named(term) = &config.graph {
            self.term(term)?;
        }
        Ok(())
    }

    pub(super) fn resolve<D: DatasetView>(
        &mut self,
        dataset: &D,
        id: D::Id,
    ) -> Result<TermValue, GeoError> {
        if self.admission.is_some() {
            try_fold_nested(
                id,
                self,
                |work, id| {
                    work.tick(1)?;
                    // Each entered node reserves its owned TermValue and an
                    // overbound for the core fold/compare work-list frames before
                    // the shared iterative walker can expand a nested triple.
                    work.storage(
                        (size_of::<TermValue>() as u64)
                            .checked_mul(3)
                            .and_then(|bytes| {
                                bytes.checked_add((size_of::<D::Id>() as u64).checked_mul(16)?)
                            })
                            .and_then(|bytes| bytes.checked_add(384))
                            .ok_or(GeoError::ArithmeticOverflow("expanded RDF term storage"))?,
                    )?;
                    let guard = dataset
                        .resolve(id)
                        .map_err(|error| GeoError::source_read(error.to_string()))?;
                    let term = guard.term();
                    let bytes = match term {
                        TermRef::Iri(text) => text.len(),
                        TermRef::Blank { label, .. } => label.len(),
                        TermRef::Literal {
                            lexical,
                            datatype,
                            language,
                            ..
                        } => {
                            let datatype = dataset
                                .resolve(datatype)
                                .map_err(|error| GeoError::source_read(error.to_string()))?;
                            let TermRef::Iri(iri) = datatype.term() else {
                                return Err(GeoError::config(
                                    "dataset literal datatype does not resolve to an IRI",
                                ));
                            };
                            lexical
                                .len()
                                .checked_add(iri.len())
                                .and_then(|bytes| bytes.checked_add(language.map_or(0, str::len)))
                                .ok_or(GeoError::ArithmeticOverflow("expanded literal strings"))?
                        }
                        TermRef::Triple { s, p, o } => {
                            return Ok(purrdf_core::Nested::Triple(s, p, o));
                        }
                    };
                    work.storage(
                        (bytes as u64)
                            .checked_mul(4)
                            .ok_or(GeoError::ArithmeticOverflow("owned RDF term strings"))?,
                    )?;
                    work.tick((bytes as u64).div_ceil(16))?;
                    Ok(purrdf_core::Nested::Leaf(()))
                },
                |_, _, (), (), ()| Ok(()),
            )?;
        }
        let value = resolve_value(dataset, id)?;
        self.tick(0)?;
        Ok(value)
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
                let mut observer = SourceObserver { context, progress };
                crate::carrier::geometry_arg_metered(vocab, value, policy, &mut observer)
                    .map(crate::carrier::ParsedGeometry::into_literal)
            }
        }
    }

    pub(super) fn write(
        &mut self,
        literal: &GeometryLiteral,
        scale: u32,
    ) -> Result<String, GeoError> {
        match self.admission.as_mut() {
            None => Ok(crate::wkt::write(literal, scale)),
            Some((context, progress)) => {
                crate::wkt::write_admitted(literal, scale, context, progress)
            }
        }
    }

    pub(super) fn append(&mut self, out: &mut String, text: &str) -> Result<(), GeoError> {
        if self.admission.is_some() {
            let needed = out
                .len()
                .checked_add(text.len())
                .ok_or(GeoError::ArithmeticOverflow("projection key storage"))?;
            if needed > out.capacity() {
                self.storage((needed - out.capacity()) as u64)?;
                out.reserve_exact(needed - out.len());
            }
            self.tick((text.len() as u64).div_ceil(16))?;
        }
        out.push_str(text);
        Ok(())
    }

    pub(super) fn exact_ordinate(&mut self, out: &mut String, value: &Rat) -> Result<(), GeoError> {
        if let Some((context, progress)) = self.admission.as_mut() {
            let (fields, bytes) =
                crate::operation::rational_fields_admitted(value, context, progress)?;
            self.storage(bytes)?;
            self.append(out, "|")?;
            self.append(
                out,
                core::str::from_utf8(&fields[0]).expect("canonical decimal integer"),
            )?;
            self.append(out, "/")?;
            self.append(
                out,
                core::str::from_utf8(&fields[1]).expect("canonical decimal integer"),
            )?;
            let (context, _) = self.admission.as_mut().expect("admitted projection");
            context.release_workspace(bytes)?;
        } else {
            out.push('|');
            out.push_str(&value.numerator().to_string());
            out.push('/');
            out.push_str(&value.denominator().to_string());
        }
        Ok(())
    }

    pub(super) fn clone_keyed(&mut self, keyed: &Keyed) -> Result<Keyed, GeoError> {
        if let Some((context, progress)) = self.admission.as_mut() {
            let mut limits = *context.policy().limits();
            limits.max_work_items = limits.max_work_items.saturating_sub(context.work_items());
            limits.max_workspace_bytes = context.remaining_workspace();
            let mut observer = SourceObserver { context, progress };
            crate::prepared::source_inventory(
                keyed.literal.geometry(),
                &limits,
                Some(&mut observer),
            )?;
        }
        self.storage(keyed.literal.crs().retained_text_bytes() as u64)?;
        self.storage(keyed.key.capacity() as u64)?;
        self.tick((keyed.key.len() as u64).div_ceil(16))?;
        Ok(keyed.clone())
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
        if self.admission.is_some() {
            let result = value.visit_terms(|term| {
                let bytes = term.text_payload_bytes();
                let Some(bytes) = bytes else {
                    return ControlFlow::Break(GeoError::ArithmeticOverflow(
                        "projection term scan",
                    ));
                };
                let Some(items) = (bytes as u64).div_ceil(16).checked_add(1) else {
                    return ControlFlow::Break(GeoError::ArithmeticOverflow(
                        "projection term work",
                    ));
                };
                match self.tick(items) {
                    Ok(()) => ControlFlow::Continue(()),
                    Err(error) => ControlFlow::Break(error),
                }
            });
            if let ControlFlow::Break(error) = result {
                return Err(error);
            }
        }
        Ok(())
    }

    pub(super) fn compare_terms(
        &mut self,
        a: &TermValue,
        b: &TermValue,
    ) -> Result<Ordering, GeoError> {
        self.scan_term(a)?;
        self.scan_term(b)?;
        Ok(a.cmp(b))
    }

    pub(super) fn finish(&mut self, complete: bool) -> Result<(), GeoError> {
        if let Some((context, progress)) = self.admission.as_mut() {
            if complete {
                if let Err(error) = progress.context_poll(context) {
                    context.release_transient_workspace()?;
                    return Err(error);
                }
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
        }
    }
}

struct SourceObserver<'a, 'observer> {
    context: &'a mut MetricContext,
    progress: &'a mut WorkProgress<'observer>,
}
impl MetricWorkObserver for SourceObserver<'_, '_> {
    fn charge_chunk(&mut self, work: u64, storage: u64) -> Result<(), GeoError> {
        self.context.charge_work(work)?;
        self.context.admit_workspace(storage)?;
        self.progress.context_poll(self.context)
    }
}

#[cfg(test)]
mod tests;
