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

    fn capacity(&mut self, out: &mut String, additional: usize) -> Result<(), GeoError> {
        if self.admission.is_none() {
            return Ok(());
        }
        let needed = out
            .len()
            .checked_add(additional)
            .ok_or(GeoError::ArithmeticOverflow("carrier output capacity"))?;
        if needed > out.capacity() {
            if let Some((context, progress)) = self.admission.as_mut() {
                let growth = (needed - out.capacity()) as u64;
                context.admit_workspace(growth)?;
                self.output_bytes = self
                    .output_bytes
                    .checked_add(growth)
                    .ok_or(GeoError::ArithmeticOverflow("carrier output storage"))?;
                progress.context_poll(context)?;
            }
            out.try_reserve_exact(needed - out.len())
                .map_err(|_| GeoError::MemoryExhausted {
                    limit: self
                        .admission
                        .as_ref()
                        .expect("admitted writer")
                        .0
                        .policy()
                        .limits()
                        .max_workspace_bytes,
                })?;
        }
        Ok(())
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
                progress.exact(context, cost, || Ok(value.finite_decimal_scale()))?
            } else {
                value.finite_decimal_scale()
            }
            .ok_or_else(|| {
                GeoError::Unsupported(
                    "an exact decimal carrier cannot represent a nonterminating rational"
                        .to_owned(),
                )
            })?;
            u32::try_from(needed)
                .map_err(|_| GeoError::ArithmeticOverflow("exact carrier decimal scale"))?
        } else {
            scale
        };
        if self.admission.is_none() {
            out.push_str(&value.to_decimal_string(scale));
            return Ok(());
        }
        let cost = purrdf_xsd::integer::ExactArithmeticCost::rational_decimal(
            value.numerator().bit_len(),
            value.denominator().bit_len(),
            scale,
        )
        .ok_or(GeoError::ArithmeticOverflow(
            "carrier decimal rendering admission",
        ))?;
        // This conservative complete rendering allowance also bounds the
        // returned decimal bytes. Reserve output before executing the original
        // arithmetic body, so its temporary spelling and retained output fit
        // together in the same unchanged invocation admission.
        self.capacity(
            out,
            usize::try_from(cost.workspace_bytes)
                .map_err(|_| GeoError::ArithmeticOverflow("carrier decimal capacity"))?,
        )?;
        let (context, progress) = self.admission.as_mut().expect("admitted writer");
        progress.exact(context, cost, || {
            out.push_str(&value.to_decimal_string(scale));
            Ok(())
        })
    }

    fn job<'g>(&mut self, jobs: &mut Vec<Job<'g>>, job: Job<'g>) -> Result<(), GeoError> {
        if let Some((context, progress)) = self.admission.as_mut() {
            context.charge_work(1)?;
            if jobs.len() == jobs.capacity() {
                let growth = size_of::<Job<'g>>() as u64;
                context.admit_workspace(growth)?;
                self.job_bytes = self
                    .job_bytes
                    .checked_add(growth)
                    .ok_or(GeoError::ArithmeticOverflow("carrier writer work list"))?;
                progress.context_poll(context)?;
                jobs.try_reserve_exact(1)
                    .map_err(|_| GeoError::MemoryExhausted {
                        limit: context.policy().limits().max_workspace_bytes,
                    })?;
            }
            progress.context_poll(context)?;
        }
        jobs.push(job);
        Ok(())
    }

    pub(crate) fn finish(&mut self, completed: bool) -> Result<(), GeoError> {
        if let Some((context, _)) = self.admission.as_mut() {
            context.release_workspace(
                self.job_bytes
                    .checked_add(if completed { 0 } else { self.output_bytes })
                    .ok_or(GeoError::ArithmeticOverflow(
                        "carrier writer storage release",
                    ))?,
            )?;
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use crate::{
        ExecutionLimits, ExecutionPolicy, GeoError, GeometryLiteral, MetricContext,
        MetricWorkObserver,
    };

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
