// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only complete warmed geodesic caller-buffer allocation receipts.

use purrdf_geo_kernel::{
    ExecutionLimits, ExecutionPolicy, GeoError, GeographicReference, LonLat, MetricContext,
    PreparedGeodesic, Rat, geodesic::DirectOutputGrid,
};
use std::{hint::black_box, time::Instant};

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn measure(
    name: &str,
    count: usize,
    mut run: impl FnMut() -> Result<(), GeoError>,
) -> Result<(), GeoError> {
    run()?;
    let allocations = purrdf_alloc_probe::CurrentThreadWindow::open();
    let started = Instant::now();
    let result = run();
    let elapsed = started.elapsed().as_nanos();
    let receipt = allocations.close();
    result?;
    println!("operation={name} count={count} elapsed_ns={elapsed} allocations={receipt:?}");
    Ok(())
}

fn main() -> Result<(), GeoError> {
    let mut arguments = std::env::args().skip(1);
    let adversarial = match arguments.next().as_deref() {
        None => false,
        Some("adversarial") => true,
        Some(_) => {
            return Err(GeoError::config(
                "usage: geodesic_allocations [adversarial]",
            ));
        }
    };
    if arguments.next().is_some() {
        return Err(GeoError::config(
            "usage: geodesic_allocations [adversarial]",
        ));
    }
    let decimal = |value: &str| Rat::parse_decimal(value).expect("published exact decimal");
    // CC0: C. F. F. Karney, GeodTest, Zenodo record32156, first public row.
    let first = LonLat::new(Rat::zero(), decimal("36.530042355041"))?;
    let last = LonLat::new(
        decimal("5.762344694676510456"),
        decimal("-48.164270779097768864"),
    )?;
    let reference = GeographicReference::wgs84();
    let policy = ExecutionPolicy::new(ExecutionLimits {
        max_work_items: u64::MAX,
        ..ExecutionLimits::GEOMETRY
    })?;
    let mut context = MetricContext::new(reference.clone(), policy)?;
    context.prepare_arithmetic()?;
    let prepared = PreparedGeodesic::prepare(reference, &mut context)?;
    let inverse = prepared.inverse(&first, &last, &mut context)?;
    let azimuth = inverse
        .forward_azimuth()
        .expect("distinct published points");
    let length = inverse.distance().value();
    println!(
        "integer_size={} context_size={}",
        size_of::<purrdf_geo_kernel::Int>(),
        size_of::<MetricContext>()
    );
    if adversarial {
        return measure_adversarial(&prepared, &mut context);
    }
    for count in [1, 4, 16, 256, 4096] {
        let mut distance = vec![None; count];
        measure("distance", count, || {
            prepared.distance_batch_borrowed(
                std::iter::repeat_n((&first, &last), count),
                &mut distance,
                &mut context,
            )?;
            black_box(&distance);
            Ok(())
        })?;
        let mut inverses = vec![None; count];
        measure("inverse_metadata", count, || {
            prepared.inverse_batch_borrowed(
                std::iter::repeat_n((&first, &last), count),
                &mut inverses,
                &mut context,
            )?;
            black_box(&inverses);
            Ok(())
        })?;
        let mut direct = vec![None; count];
        measure("direct", count, || {
            prepared.direct_batch_borrowed(
                std::iter::repeat_n((&first, azimuth, length), count),
                &mut direct,
                DirectOutputGrid::DEGREE15,
                &mut context,
            )?;
            black_box(&direct);
            Ok(())
        })?;
        let mut zeros = vec![None; count];
        measure("zero_inverse_metadata", count, || {
            prepared.inverse_batch_borrowed(
                std::iter::repeat_n((&first, &first), count),
                &mut zeros,
                &mut context,
            )?;
            black_box(&zeros);
            Ok(())
        })?;
        measure("mixed_zero_inverse_metadata", count, || {
            prepared.inverse_batch_borrowed(
                (0..count).map(|index| (&first, if index % 2 == 0 { &first } else { &last })),
                &mut zeros,
                &mut context,
            )?;
            black_box(&zeros);
            Ok(())
        })?;
    }
    Ok(())
}

fn measure_adversarial(
    prepared: &PreparedGeodesic,
    context: &mut MetricContext,
) -> Result<(), GeoError> {
    let decimal = |value: &str| Rat::parse_decimal(value).expect("published exact decimal");
    // CC0 GeodTest rows1,450001,457455. Exact source coordinates
    // exercise the distinct112/224/448-bit inverse proof admission paths.
    for (name, first, last, longitude) in [
        (
            "ordinary112",
            "36.530042355041",
            "-48.164270779097768864",
            "5.762344694676510456",
        ),
        (
            "vertex224",
            "21.004101257892",
            "-21.004101257877442853",
            "179.43641220015808594",
        ),
        (
            "vertex448",
            "7.326494029601",
            "-7.326494029600942956",
            "179.401396884582420845",
        ),
    ] {
        let a = LonLat::new(Rat::zero(), decimal(first))?;
        let b = LonLat::new(decimal(longitude), decimal(last))?;
        let (_, proof) = prepared.inverse_with_proof(&a, &b, context)?;
        let precision = proof.distance.precision_bits;
        drop(proof);
        measure(name, 1, || {
            black_box(prepared.inverse(&a, &b, context)?);
            Ok(())
        })?;
        let proof_name = format!("{name}_published_proof");
        measure(&proof_name, 1, || {
            let (answer, proof) = prepared.inverse_with_proof(&a, &b, context)?;
            black_box(answer);
            black_box(proof);
            Ok(())
        })?;
        println!(
            "operation={name} precision_bits={precision} retained_workspace={} arena_available={}",
            context.retained_workspace_bytes(),
            context
                .integer_scratch()
                .map_or(0, purrdf_xsd::integer::LimbScratch::available)
        );
    }
    Ok(())
}
