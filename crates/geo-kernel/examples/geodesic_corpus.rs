// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only streamed public GeodTest native geodesic qualification.
#![allow(missing_docs)]

use purrdf_geo_kernel::{
    ExecutionLimits, ExecutionPolicy, GeographicReference, LonLat, Metres, MetricContext,
    PreparedGeodesic, Rat,
};
use purrdf_hash::Backend as _;
use purrdf_xsd::math::FloatProductBackend;
use std::{
    io::{self, BufRead},
    time::Instant,
};

fn positive_maximum(maximum: u64) -> Result<(), &'static str> {
    if maximum == 0 {
        Err("corpus qualification requires a positive maximum_rows")
    } else {
        Ok(())
    }
}

fn read_rows(
    reader: impl BufRead,
    maximum: u64,
    mut visit: impl FnMut(u64, [Rat; 10]) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    positive_maximum(maximum)?;
    let mut lines = reader.lines();
    for row in 1..=maximum {
        let line = lines
            .next()
            .ok_or("corpus input ended before maximum_rows")??;
        let mut fields = line.split_whitespace();
        let mut values = std::array::from_fn(|_| Rat::zero());
        for value in &mut values {
            *value = Rat::parse_decimal(fields.next().ok_or("GeodTest requires ten fields")?)
                .ok_or("invalid exact corpus decimal")?;
        }
        if fields.next().is_some() {
            return Err("GeodTest requires ten fields".into());
        }
        visit(row, values)?;
    }
    Ok(())
}

struct Completion {
    rows: u64,
    primary: u64,
    inverse: u64,
    direct: u64,
    witness: u64,
}
impl Completion {
    fn validate(&self, maximum: u64, mode: &str) -> Result<(), &'static str> {
        positive_maximum(maximum)?;
        let inverse = mode == "inverse" || mode == "all";
        let direct = mode == "direct" || mode == "all";
        if self.rows != maximum
            || self.primary != if mode == "direct" { 0 } else { maximum }
            || self.inverse != if inverse { maximum } else { 0 }
            || self.direct != if direct { maximum } else { 0 }
            || self.witness != if inverse { maximum } else { 0 }
        {
            Err("corpus qualification did not complete every requested mode record")
        } else {
            Ok(())
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let maximum = args
        .next()
        .map_or(Ok(500_000), |value| value.parse::<u64>())?;
    let work = args
        .next()
        .map_or(Ok(ExecutionLimits::GEOMETRY.max_work_items), |value| {
            value.parse::<u64>()
        })?;
    let mode = args.next().unwrap_or_else(|| "distance".into());
    let backend = args
        .next()
        .map(|name| {
            FloatProductBackend::ALL
                .iter()
                .copied()
                .find(|path| path.name() == name)
                .ok_or("unknown numerical backend")
        })
        .transpose()?;
    if !["distance", "inverse", "direct", "all"].contains(&mode.as_str()) || args.next().is_some() {
        return Err("usage: geodesic_corpus [maximum_rows] [work_limit] [distance|inverse|direct|all] [portable|sse2|avx2|avx512|neon|simd128] < public-data".into());
    }
    positive_maximum(maximum)?;
    let inverse = mode == "inverse" || mode == "all";
    let direct = mode == "direct" || mode == "all";
    let reference = GeographicReference::wgs84();
    let mut context = MetricContext::new(
        reference.clone(),
        ExecutionPolicy::new(ExecutionLimits {
            max_work_items: work,
            ..ExecutionLimits::GEOMETRY
        })?,
    )?;
    if let Some(backend) = backend {
        context.set_binary64_backend(backend)?;
    }
    context.prepare_arithmetic()?;
    let prepared = PreparedGeodesic::prepare(reference, &mut context)?;
    let tolerance = Rat::parse_decimal("0.0000006").expect("exact tolerance");
    let mut rows = 0_u64;
    let mut complete = 0_u64;
    let mut fallbacks = 0_u64;
    let mut refusals = 0_u64;
    let mut mismatches = 0_u64;
    let mut inverse_rows = 0_u64;
    let mut direct_rows = 0_u64;
    let mut direct_refusals = 0_u64;
    let mut direct_mismatches = 0_u64;
    let mut inverse_witness_rows = 0_u64;
    let mut inverse_witness_mismatches = 0_u64;
    let inverse_witness_tolerance = Rat::parse_decimal("0.000003").expect("exact allowance");
    // The corpus was generated directly from exact lat1/azi1/s12. Its printed
    // endpoint coordinates have finite uncertainty, amplified near conjugate
    // inverse branches. Report all nominal metadata disagreements; qualify
    // inverse azimuths by independently propagating their completed source law.
    let mut metadata_maxima: [Rat; 5] = std::array::from_fn(|_| Rat::zero());
    let mut metadata_nominal_excesses = [0_u64; 5];
    let metadata_nominal_allowances = [
        "0.000000000001",
        "0.000000000001",
        "0.000000000001",
        "0.0000006",
        "0.02",
    ]
    .map(|text| Rat::parse_decimal(text).expect("exact nominal allowance"));
    let full_turn = Rat::from_i64(360);
    let direct_tolerance = Rat::parse_decimal("0.000001").expect("exact tolerance");
    let mut work_total = 0_u64;
    let mut workspace_peak = 0_u64;
    let started = Instant::now();
    read_rows(io::stdin().lock(), maximum, |row, fields| {
        let exact = |index: usize| fields[index].clone();
        let a = LonLat::new(exact(1), exact(0))?;
        let b = LonLat::new(exact(4), exact(3))?;
        let expected = exact(6);
        let mut inverse_witness_work = 0_u64;
        rows = row;
        let distance_result =
            if mode == "direct" {
                None
            } else if inverse {
                Some(prepared.inverse_with_proof(&a, &b, &mut context).and_then(
                    |(result, proof)| {
                        let forward = result.forward_azimuth().ok_or_else(|| {
                            purrdf_geo_kernel::GeoError::domain(
                                "inverse corpus requires a nonzero branch",
                            )
                        })?;
                        let final_azimuth = result.final_azimuth().ok_or_else(|| {
                            purrdf_geo_kernel::GeoError::domain(
                                "inverse corpus requires a nonzero branch",
                            )
                        })?;
                        inverse_rows += 1;
                        for (field, actual) in [
                            forward,
                            final_azimuth,
                            result.arc_degrees(),
                            result.reduced_length().exact(),
                            result.geodesic_quadrilateral_area().exact(),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let expected = exact([2, 5, 7, 8, 9][field]);
                            let mut disagreement = actual.sub(&expected).abs();
                            if field < 2 {
                                let complement = full_turn.sub(&disagreement).abs();
                                disagreement = disagreement.min(complement);
                            }
                            metadata_nominal_excesses[field] +=
                                u64::from(disagreement > metadata_nominal_allowances[field]);
                            metadata_maxima[field] =
                                metadata_maxima[field].clone().max(disagreement);
                        }
                        let entry_work = context.work_items();
                        let entry_peak = context.workspace_peak();
                        let endpoint =
                            prepared.direct(&a, forward, result.distance().value(), &mut context);
                        inverse_witness_work = entry_work.checked_add(context.work_items()).ok_or(
                            purrdf_geo_kernel::GeoError::ArithmeticOverflow("corpus witness work"),
                        )?;
                        let witness = endpoint.and_then(|endpoint| {
                            prepared.distance(endpoint.endpoint(), &b, &mut context)
                        });
                        match witness {
                            Ok(residual) => {
                                inverse_witness_rows += 1;
                                inverse_witness_mismatches += u64::from(
                                    residual.value().exact() > &inverse_witness_tolerance,
                                );
                            }
                            Err(error) => {
                                inverse_witness_mismatches += 1;
                                if inverse_witness_mismatches <= 16 {
                                    eprintln!("row={rows} inverse_witness_refusal={error}");
                                }
                            }
                        }
                        workspace_peak = workspace_peak.max(entry_peak);
                        std::hint::black_box(result.certificate_bytes());
                        Ok((result.distance().clone(), proof.distance))
                    },
                ))
            } else {
                Some(prepared.distance_with_proof(&a, &b, &mut context))
            };
        match distance_result {
            Some(Ok((result, proof))) => {
                complete += 1;
                fallbacks += u64::from(proof.precision_bits > 53);
                mismatches += u64::from(result.value().exact().sub(&expected).abs() > tolerance);
                if maximum <= 16 {
                    println!(
                        "row={rows} precision={} work={} width={}",
                        proof.precision_bits,
                        context.work_items(),
                        proof
                            .upper
                            .exact()
                            .sub(proof.lower.exact())
                            .to_decimal_string(18)
                    );
                }
            }
            Some(Err(error)) => {
                refusals += 1;
                if refusals <= 16 {
                    eprintln!("row={rows} refusal={error}");
                }
            }
            None => {}
        }
        if mode != "direct" {
            work_total = work_total
                .checked_add(inverse_witness_work)
                .ok_or("work sum overflow")?
                .checked_add(context.work_items())
                .ok_or("work sum overflow")?;
            workspace_peak = workspace_peak.max(context.workspace_peak());
        }
        if direct {
            match prepared.direct(&a, &exact(2), &Metres::new(expected), &mut context) {
                Ok(result) => {
                    direct_rows += 1;
                    if maximum <= 16 {
                        println!("row={rows} direct_work={}", context.work_items());
                    }
                    work_total = work_total
                        .checked_add(context.work_items())
                        .ok_or("work sum overflow")?;
                    workspace_peak = workspace_peak.max(context.workspace_peak());
                    let endpoint = result.endpoint();
                    let residual = prepared.distance(endpoint, &b, &mut context)?;
                    // lat1/azi1/s12 are exact corpus construction inputs. The
                    // printed endpoint has finite decimal uncertainty, so compare
                    // the propagated endpoint physically rather than by spelling.
                    direct_mismatches += u64::from(residual.value().exact() > &direct_tolerance);
                    std::hint::black_box(result.certificate_bytes());
                }
                Err(error) => {
                    direct_refusals += 1;
                    if direct_refusals <= 16 {
                        eprintln!("row={rows} direct_refusal={error}");
                    }
                }
            }
            work_total = work_total
                .checked_add(context.work_items())
                .ok_or("work sum overflow")?;
            workspace_peak = workspace_peak.max(context.workspace_peak());
        }
        if rows.is_multiple_of(10_000) {
            eprintln!(
                "processed_rows={rows} complete={complete} fallbacks={fallbacks} refusals={refusals} mismatches={mismatches}"
            );
        }
        Ok(())
    })?;
    let elapsed = started.elapsed().as_nanos();
    println!(
        "backend={} mode={mode} rows={rows} complete={complete} fallbacks={fallbacks} refusals={refusals} mismatches={mismatches} inverse_rows={inverse_rows} inverse_witness_rows={inverse_witness_rows} inverse_witness_mismatches={inverse_witness_mismatches} direct_rows={direct_rows} direct_refusals={direct_refusals} direct_mismatches={direct_mismatches} elapsed_ns={elapsed} work_total={work_total} workspace_peak={workspace_peak}",
        context.binary64_backend().name()
    );
    if inverse {
        for (field, name) in [
            "forward_azimuth",
            "final_azimuth",
            "arc",
            "reduced_length",
            "quadrilateral_area",
        ]
        .into_iter()
        .enumerate()
        {
            println!(
                "nominal_metadata={name} excesses={} maximum_disagreement={} allowance={}",
                metadata_nominal_excesses[field],
                metadata_maxima[field].to_decimal_string(18),
                metadata_nominal_allowances[field].to_decimal_string(18),
            );
        }
    }
    Completion {
        rows,
        primary: complete,
        inverse: inverse_rows,
        direct: direct_rows,
        witness: inverse_witness_rows,
    }
    .validate(maximum, &mode)?;
    if refusals != 0
        || mismatches != 0
        || inverse_witness_mismatches != 0
        || direct_refusals != 0
        || direct_mismatches != 0
    {
        return Err("corpus qualification failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_rejects_zero_empty_truncated_and_malformed_input_but_keeps_explicit_prefixes() {
        const ROW: &str = "0 0 90 0 1 90 111319.49079327357 1 111313.80114861293 0\n";
        let mut visited = 0;
        assert!(
            read_rows(io::Cursor::new(ROW), 0, |_, _| {
                visited += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(visited, 0);
        assert!(read_rows(io::Cursor::new(""), 1, |_, _| Ok(())).is_err());
        assert!(
            read_rows(io::Cursor::new(ROW), 2, |_, _| {
                visited += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(visited, 1);
        for malformed in [
            "0 0 90 0 1 90 111319 1 111313\n",
            "0 0 90 0 1 90 111319 1 111313 invalid\n",
            "0 0 90 0 1 90 111319 1 111313 0 extra\n",
        ] {
            assert!(
                read_rows(io::Cursor::new(malformed), 1, |_, _| {
                    panic!("malformed row reached a numerical entry")
                })
                .is_err()
            );
        }
        let source = format!("{ROW}{ROW}invalid trailing data outside the requested prefix\n");
        visited = 0;
        read_rows(io::Cursor::new(source), 2, |row, values| {
            visited += 1;
            assert_eq!(row, visited);
            assert_eq!(values[6], Rat::parse_decimal("111319.49079327357").unwrap());
            Ok(())
        })
        .unwrap();
        assert_eq!(visited, 2);
    }

    #[test]
    fn completion_requires_every_record_of_the_selected_mode() {
        for mode in ["distance", "inverse", "direct", "all"] {
            let inverse = mode == "inverse" || mode == "all";
            let direct = mode == "direct" || mode == "all";
            let mut counts = Completion {
                rows: 2,
                primary: if mode == "direct" { 0 } else { 2 },
                inverse: if inverse { 2 } else { 0 },
                direct: if direct { 2 } else { 0 },
                witness: if inverse { 2 } else { 0 },
            };
            counts.validate(2, mode).unwrap();
            counts.rows = 1;
            assert!(counts.validate(2, mode).is_err());
            counts.rows = 2;
            if inverse {
                counts.witness = 1;
            } else if direct {
                counts.direct = 1;
            } else {
                counts.primary = 1;
            }
            assert!(counts.validate(2, mode).is_err());
            assert!(counts.validate(0, mode).is_err());
        }
    }
}
