// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared exact coordinate rendering and governed carrier output traversal.

use crate::{GeoError, Geometry, GeometryBody, Rat};

enum Job<'a> {
    Geometry(&'a Geometry),
    Text(&'static str),
}

pub(crate) struct CarrierWriter<'a, 'observer> {
    admission: Option<(
        &'a mut crate::MetricContext,
        &'a mut crate::context::WorkProgress<'observer>,
    )>,
    output_bytes: u64,
    job_bytes: u64,
    exact_ordinates: bool,
}

impl<'a, 'observer> CarrierWriter<'a, 'observer> {
    pub(crate) fn plain() -> Self {
        Self {
            admission: None,
            output_bytes: 0,
            job_bytes: 0,
            exact_ordinates: false,
        }
    }

    pub(crate) fn admitted(
        context: &'a mut crate::MetricContext,
        progress: &'a mut crate::context::WorkProgress<'observer>,
    ) -> Self {
        Self {
            admission: Some((context, progress)),
            output_bytes: 0,
            job_bytes: 0,
            exact_ordinates: false,
        }
    }

    pub(crate) fn preserving_exact(mut self) -> Self {
        self.exact_ordinates = true;
        self
    }

    /// Use the writer's existing native owner for an original failure payload.
    pub(crate) fn with_progress<T>(
        &mut self,
        body: impl FnOnce(&mut crate::context::WorkProgress<'_>) -> T,
    ) -> T {
        match self.admission.as_mut() {
            Some((_, progress)) => body(progress),
            None => body(&mut crate::context::WorkProgress::integer(None)),
        }
    }

    fn capacity(&mut self, out: &mut String, additional: usize) -> Result<(), GeoError> {
        use purrdf_lex::allocation::{Memory, Resident, StorageError};
        let needed = out
            .len()
            .checked_add(additional)
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
        let previous = out.capacity();
        if needed <= previous {
            return Ok(());
        }
        let destination = previous
            .checked_mul(2)
            .map(|doubled| doubled.max(needed))
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
        if let Some((context, progress)) = self.admission.as_mut() {
            let charged = u64::try_from(destination)
                .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
            context.admit_workspace(charged)?;
            let result = progress.context_poll(context).and_then(|()| {
                progress
                    .with_storage(previous, |memory| memory.reserve_string(out, destination))
                    .map_err(GeoError::Storage)
            });
            let released = if result.is_ok() {
                self.output_bytes = u64::try_from(out.capacity())
                    .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
                u64::try_from(previous)
                    .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?
            } else {
                charged
            };
            let refund = context.release_workspace(released);
            result.and(refund)
        } else {
            let mut resident = Resident;
            Memory::resume(&mut resident, previous)
                .reserve_string(out, destination)
                .map_err(GeoError::Storage)
        }
    }

    pub(crate) fn text(&mut self, out: &mut String, text: &str) -> Result<(), GeoError> {
        self.capacity(out, text.len())?;
        if let Some((context, progress)) = self.admission.as_mut() {
            context.charge_work(text.len() as u64)?;
            progress.context_poll(context)?;
        }
        out.push_str(text);
        Ok(())
    }

    pub(crate) fn ordinate(
        &mut self,
        out: &mut String,
        value: &Rat,
        scale: u32,
    ) -> Result<(), GeoError> {
        let scale = if self.exact_ordinates {
            let needed = if let Some((context, progress)) = self.admission.as_mut() {
                let cost = purrdf_xsd::integer::ExactArithmeticCost::finite_decimal_scale(
                    value.denominator().bit_len(),
                )
                .ok_or(GeoError::ArithmeticOverflow(
                    "exact carrier scale admission",
                ))?;
                context.prepare_integer_scratch_for_cost_observed(cost, progress)?;
                progress.exact_context(context, cost, |context| {
                    let scratch = context
                        .integer_scratch()
                        .cloned()
                        .unwrap_or_else(purrdf_xsd::integer::LimbScratch::inline);
                    value
                        .finite_decimal_scale_in(&scratch)
                        .map_err(GeoError::NumericalScratch)
                })?
            } else {
                value.finite_decimal_scale()
            };
            let Some(needed) = needed else {
                let original =
                    self.with_progress(|progress| GeoError::diagnostic_number(value, progress))?;
                return Err(GeoError::Unsupported(
                    crate::UnsupportedRefusal::NonTerminatingDecimal(original),
                ));
            };
            u32::try_from(needed)
                .map_err(|_| GeoError::ArithmeticOverflow("exact carrier decimal scale"))?
        } else {
            scale
        };
        let cost = purrdf_xsd::integer::ExactArithmeticCost::rational_decimal(
            value.numerator().bit_len(),
            value.denominator().bit_len(),
            scale,
        )
        .ok_or(GeoError::ArithmeticOverflow(
            "carrier decimal rendering admission",
        ))?;
        let coefficient = if let Some((context, progress)) = self.admission.as_mut() {
            context.prepare_integer_scratch_for_cost_observed(cost, progress)?;
            progress.exact_context(context, cost, |context| {
                let scratch = context
                    .integer_scratch()
                    .cloned()
                    .unwrap_or_else(purrdf_xsd::integer::LimbScratch::inline);
                value
                    .round_to_scale_in(scale, &scratch)
                    .map_err(GeoError::NumericalScratch)
            })?
        } else {
            value.round_to_scale(scale)
        };
        let layout = coefficient
            .decimal_render_layout(scale)
            .map_err(GeoError::NumericalScratch)?;
        self.capacity(out, layout.output_bytes())?;
        let output_capacity = out.capacity();
        let mut convert = |memory: &mut purrdf_lex::allocation::Memory<
            '_,
            dyn purrdf_lex::allocation::Admission + '_,
        >| {
            purrdf_xsd::exact::append_scaled_integer_with_memory(&coefficient, scale, out, memory)
        };
        let rendered = if let Some((context, progress)) = self.admission.as_mut() {
            let temporary = u64::try_from(layout.temporary_bytes()).map_err(|_| {
                GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow)
            })?;
            context.admit_workspace(temporary)?;
            let result = progress.context_poll(context).and_then(|()| {
                progress
                    .with_storage(output_capacity, convert)
                    .map_err(decimal_error)
            });
            let refund = context.release_workspace(temporary);
            result.and(refund)
        } else {
            let mut resident = purrdf_lex::allocation::Resident;
            let storage: &mut dyn purrdf_lex::allocation::Admission = &mut resident;
            convert(&mut purrdf_lex::allocation::Memory::resume(
                storage,
                output_capacity,
            ))
            .map_err(decimal_error)
        };
        drop(coefficient);
        rendered
    }

    fn job<'g>(&mut self, jobs: &mut Vec<Job<'g>>, job: Job<'g>) -> Result<(), GeoError> {
        if let Some((context, progress)) = self.admission.as_mut() {
            context.charge_work(1)?;
            progress.context_poll(context)?;
        }
        if jobs.len() == jobs.capacity() {
            use purrdf_lex::allocation::{Memory, Resident, StorageError};
            let previous = core::alloc::Layout::array::<Job<'g>>(jobs.capacity())
                .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?
                .size();
            let required = jobs
                .capacity()
                .checked_mul(2)
                .and_then(|doubled| jobs.len().checked_add(1).map(|length| doubled.max(length)))
                .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
            let destination = core::alloc::Layout::array::<Job<'g>>(required)
                .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?
                .size();
            if let Some((context, progress)) = self.admission.as_mut() {
                let charged = u64::try_from(destination)
                    .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?;
                context.admit_workspace(charged)?;
                let result = progress.context_poll(context).and_then(|()| {
                    progress
                        .with_storage(previous, |memory| memory.reserve(jobs, required))
                        .map_err(GeoError::Storage)
                });
                let released = if result.is_ok() {
                    self.job_bytes = charged;
                    u64::try_from(previous)
                        .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?
                } else {
                    charged
                };
                let refund = context.release_workspace(released);
                result.and(refund)?;
            } else {
                let mut resident = Resident;
                Memory::resume(&mut resident, previous)
                    .reserve(jobs, required)
                    .map_err(GeoError::Storage)?;
            }
        }
        jobs.push(job);
        Ok(())
    }

    pub(crate) fn finish(&mut self, completed: bool) -> Result<(), GeoError> {
        let Some((context, progress)) = self.admission.as_mut() else {
            return Ok(());
        };
        let output = (|| {
            if completed {
                return Ok(());
            }
            // write_with has destroyed its refused output String already.
            let bytes = usize::try_from(self.output_bytes)
                .map_err(|_| GeoError::ArithmeticOverflow("carrier output release"))?;
            let physical = progress
                .with_storage(bytes, |memory| memory.release_bytes(bytes))
                .map_err(GeoError::Storage);
            let logical = context.release_workspace(self.output_bytes);
            self.output_bytes = 0;
            physical.and(logical)
        })();
        // Decimal coefficients and traversal jobs have died at this boundary.
        // The returned carrier owns only its String; an automatic numerical
        // arena is no longer part of that output's original storage grant.
        let arena = context.release_unused_integer_scratch();
        progress.settle_result(output, arena)
    }

    pub(crate) fn tree(
        &mut self,
        out: &mut String,
        geometry: &Geometry,
        mut head: impl FnMut(
            &mut String,
            &Geometry,
            &mut Self,
        ) -> Result<Option<&'static str>, GeoError>,
    ) -> Result<(), GeoError> {
        let mut jobs = Vec::new();
        let result = (|| {
            self.job(&mut jobs, Job::Geometry(geometry))?;
            while let Some(job) = jobs.pop() {
                match job {
                    Job::Text(text) => self.text(out, text)?,
                    Job::Geometry(geometry) => {
                        if let Some(close) = head(out, geometry, self)? {
                            let GeometryBody::GeometryCollection(members) = geometry.body() else {
                                unreachable!("only a collection schedules child geometries")
                            };
                            self.job(&mut jobs, Job::Text(close))?;
                            for (index, member) in members.iter().enumerate().rev() {
                                self.job(&mut jobs, Job::Geometry(member))?;
                                if index > 0 {
                                    self.job(&mut jobs, Job::Text(","))?;
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        })();
        let bytes = core::alloc::Layout::array::<Job<'_>>(jobs.capacity())
            .map_err(|_| GeoError::ArithmeticOverflow("carrier traversal release"))?
            .size();
        let refund = if let Some((context, progress)) = self.admission.as_mut() {
            let physical = progress
                .with_storage(bytes, |memory| memory.release_vec(jobs))
                .map_err(GeoError::Storage);
            let logical = context.release_workspace(self.job_bytes);
            self.job_bytes = 0;
            physical.and(logical)
        } else {
            drop(jobs);
            Ok(())
        };
        result.and(refund)
    }

    pub(crate) fn sequence<T>(
        &mut self,
        out: &mut String,
        items: &[T],
        open: &'static str,
        close: &'static str,
        mut each: impl FnMut(&mut String, &T, &mut Self) -> Result<(), GeoError>,
    ) -> Result<(), GeoError> {
        self.text(out, open)?;
        for (index, item) in items.iter().enumerate() {
            if index > 0 {
                self.text(out, ",")?;
            }
            each(out, item, self)?;
        }
        self.text(out, close)
    }
}

fn decimal_error(error: purrdf_xsd::exact::DecimalTextError) -> GeoError {
    match error {
        purrdf_xsd::exact::DecimalTextError::Numeric(error) => GeoError::NumericalScratch(error),
        purrdf_xsd::exact::DecimalTextError::Storage(error) => GeoError::Storage(error),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ExecutionLimits, ExecutionPolicy, GeoError, GeometryLiteral, MetricContext,
        MetricWorkObserver,
    };

    #[test]
    fn exact_decimal_refusal_retains_the_original_nonterminating_rational() {
        use purrdf_testkit::error_capture::Capture;

        let input = crate::Rat::new(crate::Int::one(), crate::Int::from_i64(3)).unwrap();
        let mut output = String::new();
        let error = super::CarrierWriter::plain()
            .preserving_exact()
            .ordinate(&mut output, &input, 15)
            .unwrap_err();
        drop(input);
        assert_eq!(output, "");
        let cloned = error.clone();
        drop(error);
        let fields = Capture::read(&cloned).child("cause");
        assert_eq!(fields.code, "geo-nonterminating-decimal");
        let GeoError::Unsupported(crate::UnsupportedRefusal::NonTerminatingDecimal(value)) = cloned
        else {
            panic!("the exact carrier must retain the original numerical refusal");
        };
        assert_eq!(
            *value,
            crate::Rat::new(crate::Int::one(), crate::Int::from_i64(3)).unwrap()
        );
        super::CarrierWriter::plain()
            .preserving_exact()
            .ordinate(
                &mut output,
                &crate::Rat::new(crate::Int::one(), crate::Int::from_i64(4)).unwrap(),
                15,
            )
            .unwrap();
        assert_eq!(output, "0.25");
    }

    #[derive(Default)]
    struct Receipt {
        work: u64,
        bytes: u64,
        calls: u64,
        stop: Option<u64>,
    }
    impl MetricWorkObserver for Receipt {
        fn charge_chunk(&mut self, work: u64, bytes: u64) -> Result<(), GeoError> {
            self.work += work;
            self.bytes += bytes;
            self.calls += 1;
            if self.stop == Some(self.calls) {
                Err(GeoError::Cancelled)
            } else {
                Ok(())
            }
        }
    }

    fn write(
        literal: &GeometryLiteral,
        json: bool,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<String, GeoError> {
        if json {
            crate::geojson::write_in_context_metered(literal, literal.crs(), 15, context, observer)
        } else {
            crate::wkt::write_in_context_metered(literal, 15, context, observer)
        }
    }

    #[test]
    fn both_carriers_preserve_bytes_charge_actual_phases_and_release_refused_output() {
        for (json, source) in [
            (
                false,
                "GEOMETRYCOLLECTION ZM(POINT(1.23456789 2 3 4),LINESTRING(0 0 7 8,1 1 9 10),MULTIPOINT(EMPTY))",
            ),
            (
                true,
                "GEOMETRYCOLLECTION Z(POINT(1.23456789 2 3),LINESTRING(0 0 7,1 1 9),MULTIPOINT EMPTY)",
            ),
        ] {
            let literal =
                crate::wkt::parse(source, crate::standard_vocabulary().default_wkt_crs()).unwrap();
            let expected = if json {
                crate::geojson::write(&literal, literal.crs(), 15).unwrap()
            } else {
                crate::wkt::write(&literal, 15)
            };
            let mut context = MetricContext::wgs84().unwrap();
            context.set_preparation_work(17).unwrap();
            context.set_retained_workspace(256).unwrap();
            let mut receipt = Receipt::default();
            let output = write(&literal, json, &mut context, &mut receipt).unwrap();
            assert_eq!(output, expected);
            assert_eq!(receipt.work, context.work_items());
            assert_eq!(receipt.bytes, context.workspace_peak());
            context.release_workspace(output.capacity() as u64).unwrap();
            context.begin(1).unwrap();
            assert_eq!(
                context.remaining_workspace(),
                context.policy().limits().max_workspace_bytes - 256
            );

            let mut receipt = Receipt {
                stop: Some(3),
                ..Receipt::default()
            };
            assert_eq!(
                write(&literal, json, &mut context, &mut receipt),
                Err(GeoError::Cancelled)
            );
            context.begin(1).unwrap();
            assert_eq!(
                context.remaining_workspace(),
                context.policy().limits().max_workspace_bytes - 256
            );
            for (work, memory) in [
                (1, ExecutionLimits::GEOMETRY.max_workspace_bytes),
                (ExecutionLimits::GEOMETRY.max_work_items, 1),
            ] {
                let policy = ExecutionPolicy::new(ExecutionLimits {
                    max_work_items: work,
                    max_workspace_bytes: memory,
                    ..ExecutionLimits::GEOMETRY
                })
                .unwrap();
                let mut context =
                    MetricContext::new(crate::GeographicReference::wgs84(), policy).unwrap();
                let refusal = write(&literal, json, &mut context, &mut Receipt::default());
                assert!(matches!(
                    refusal,
                    Err(GeoError::WorkExhausted { .. } | GeoError::MemoryExhausted { .. })
                ));
                context.begin(1).unwrap();
                assert_eq!(
                    context.remaining_workspace(),
                    context.policy().limits().max_workspace_bytes
                );
            }
        }
    }
}
