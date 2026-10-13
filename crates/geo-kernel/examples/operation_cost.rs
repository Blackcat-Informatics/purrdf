// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reproducible report-only native projection work and elapsed-cost probe.

use purrdf_geo_kernel::operation::{
    CoordinateOperation, CoordinateUnit, Hemisphere, OperationModel, OperationPoint,
    OperationReference, TransverseMercator, ZoneFamily,
};
use purrdf_geo_kernel::{
    ExecutionLimits, ExecutionPolicy, GeographicReference, MetricContext, PreparedEllipsoid, Rat,
};
use purrdf_hash::hex::Digest32;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parse = |text: &str| Rat::parse_decimal(text).expect("frozen public probe parameter");
    let reference = |id, unit| OperationReference {
        realization: Digest32::new([id; 32]),
        unit,
        swapped_axes: false,
    };
    let forward = CoordinateOperation::compile(
        reference(1, CoordinateUnit::Degrees),
        reference(2, CoordinateUnit::Metres),
        OperationModel::TransverseMercator(Box::new(TransverseMercator {
            ellipsoid: PreparedEllipsoid::cgcs2000(),
            family: ZoneFamily::Utm,
            zone: 50,
            central_meridian: parse("117"),
            scale: parse("0.9996"),
            false_easting: parse("500000"),
            false_northing: Rat::zero(),
            hemisphere: Hemisphere::North,
            zone_prefix: false,
        })),
    )?;
    let inverse = forward.inverse()?;
    let source = OperationPoint {
        x: parse("117.7"),
        y: parse("35"),
        z: Some(parse("12.5")),
        epoch: None,
    };
    for max_work_items in [262_144, 2_000_000, 20_000_000] {
        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_work_items,
            ..ExecutionLimits::GEOMETRY
        })?;
        let mut context = MetricContext::new(GeographicReference::cgcs2000(), policy)?;
        let clock = std::time::Instant::now();
        let projected = forward.apply(&source, &mut context)?;
        println!(
            "forward limit={max_work_items} work={} peak={} elapsed_ns={}",
            context.work_items(),
            context.workspace_peak(),
            clock.elapsed().as_nanos()
        );
        let clock = std::time::Instant::now();
        let result = inverse.apply(projected.point(), &mut context);
        println!(
            "inverse limit={max_work_items} work={} peak={} elapsed_ns={} result={}",
            context.work_items(),
            context.workspace_peak(),
            clock.elapsed().as_nanos(),
            match result {
                Ok(_) => "complete".to_owned(),
                Err(error) => error.to_string(),
            }
        );
    }
    Ok(())
}
