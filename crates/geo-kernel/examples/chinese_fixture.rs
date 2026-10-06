// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only independent-field qualification of the public coordinate fixture.
//! Input is the pinned MIT-licensed data, supplied through stdin. No provider
//! implementation is loaded. Empirical agreement is distinct from residual and
//! coordinate-error certificates; it supplies no surveying accuracy claim.

#![allow(missing_docs)]

use std::io::{self, Read};
use std::time::Instant;

use purrdf_geo_kernel::{
    CoordinateOperation, GeographicReference, LonLat, MetricContext, PreparedGeodesic, Rat,
    operation::{
        Applicability, CoordinateUnit, OperationModel, OperationPoint, OperationReference,
    },
};
use purrdf_hash::{Domain, blake3::Hasher, frame::frame_le_into};
use purrdf_lex::json::{self, Value};
use purrdf_xsd::{
    BigInt,
    math::{CoordinateMath, FixedInterval, MathLimits},
};

const REFERENCE: Domain = Domain::new(b"purrdf-geo-kernel/fixture-reference/v1");

fn reference(name: &str, unit: CoordinateUnit) -> OperationReference {
    let mut hasher = Hasher::new();
    frame_le_into(&mut hasher, REFERENCE.as_bytes());
    frame_le_into(&mut hasher, name.as_bytes());
    OperationReference {
        realization: purrdf_hash::hex::Digest32::new(*hasher.finalize().as_bytes()),
        unit,
        swapped_axes: false,
    }
}

fn rat(text: &str) -> Rat {
    Rat::parse_decimal(text).expect("frozen probe decimal")
}

fn coordinate(record: &Value, key: &str) -> Result<OperationPoint, Box<dyn std::error::Error>> {
    let values = record["coords"][key]
        .as_array()
        .ok_or("fixture coordinate pair absent")?;
    if values.len() != 2 {
        return Err("fixture coordinate requires two axes".into());
    }
    let exact = |index: usize| -> Result<Rat, Box<dyn std::error::Error>> {
        Rat::parse_decimal(
            values[index]
                .as_number()
                .ok_or("fixture ordinate is not numeric")?
                .lexeme(),
        )
        .ok_or_else(|| "invalid exact fixture ordinate".into())
    };
    Ok(OperationPoint {
        x: exact(0)?,
        y: exact(1)?,
        z: None,
        epoch: None,
    })
}

fn angular_error(
    predicted: &OperationPoint,
    expected: &OperationPoint,
    prepared: &PreparedGeodesic,
    context: &mut MetricContext,
) -> Result<Rat, Box<dyn std::error::Error>> {
    let a = LonLat::new(predicted.x.clone(), predicted.y.clone())?;
    let b = LonLat::new(expected.x.clone(), expected.y.clone())?;
    let (_, proof) = prepared.distance_with_proof(&a, &b, context)?;
    Ok(proof.upper.exact().clone())
}

fn projected_error(
    predicted: &OperationPoint,
    expected: &OperationPoint,
) -> Result<Rat, Box<dyn std::error::Error>> {
    let mut math = CoordinateMath::new(MathLimits::DEFAULT)?;
    let fixed = |value: &Rat, math: &mut CoordinateMath| {
        FixedInterval::from_ratio(
            &BigInt::from(value.numerator().clone()),
            &BigInt::from(value.denominator().clone()),
            math,
        )
    };
    let dx = fixed(&predicted.x.sub(&expected.x), &mut math)?;
    let dy = fixed(&predicted.y.sub(&expected.y), &mut math)?;
    let norm = dx
        .square(&mut math)?
        .add(&dy.square(&mut math)?, &mut math)?
        .sqrt(&mut math)?;
    Ok(Rat::new(
        norm.upper().as_integer().clone(),
        purrdf_geo_kernel::Int::one().shl(math.limits().precision_bits),
    )
    .expect("positive fixed scale"))
}

fn report(name: &str, mut errors: Vec<Rat>, gate: &Rat) -> Result<(), Box<dyn std::error::Error>> {
    errors.sort_unstable();
    let maximum = errors.last().ok_or("empty fixture")?;
    let median = if errors.len().is_multiple_of(2) {
        errors[errors.len() / 2 - 1]
            .add(&errors[errors.len() / 2])
            .div(&rat("2"))
            .expect("positive divisor")
    } else {
        errors[errors.len() / 2].clone()
    };
    println!(
        "model={name} rows={} maximum_upper_metres={} median_upper_metres={} gate_metres={}",
        errors.len(),
        maximum.to_decimal_string(9),
        median.to_decimal_string(9),
        gate.to_decimal_string(9)
    );
    if maximum > gate {
        return Err("empirical fixture agreement gate failed".into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let data = json::read(&input)?;
    let rows = data.as_array().ok_or("fixture root is not an array")?;
    if rows.len() != 664 {
        return Err("pinned fixture requires every one of its 664 rows".into());
    }
    let geographic = CoordinateUnit::Degrees;
    let projected = CoordinateUnit::Metres;
    let gcj = CoordinateOperation::compile(
        reference("WGS84", geographic),
        reference("GCJ02", geographic),
        OperationModel::GcjRationalHarmonicV1(Applicability::new(
            rat("72.004"),
            rat("137.8347"),
            rat("0.8293"),
            rat("55.8271"),
        )?),
    )?;
    let bd = CoordinateOperation::compile(
        reference("GCJ02", geographic),
        reference("BD09", geographic),
        OperationModel::Bd09LlV1,
    )?;
    let mc = CoordinateOperation::compile(
        reference("BD09", geographic),
        reference("BD09MC", projected),
        OperationModel::BaiduMercatorAnalyticV1,
    )?;
    let mut context = MetricContext::wgs84()?;
    context.prepare_arithmetic()?;
    let prepared = PreparedGeodesic::prepare(GeographicReference::wgs84(), &mut context)?;
    let mut gcj_errors = Vec::with_capacity(rows.len());
    let mut bd_errors = Vec::with_capacity(rows.len());
    let mut mc_errors = Vec::with_capacity(rows.len());
    let started = Instant::now();
    for row in rows {
        // Matching fields are supplied independently. One model's empirical
        // discrepancy is never propagated into the next model's gate.
        let wgs = coordinate(row, "WGS84")?;
        let gcj_expected = coordinate(row, "GCJ02")?;
        let bd_expected = coordinate(row, "BD09")?;
        let mc_expected = coordinate(row, "BD09MC")?;
        gcj_errors.push(angular_error(
            gcj.apply(&wgs, &mut context)?.point(),
            &gcj_expected,
            &prepared,
            &mut context,
        )?);
        bd_errors.push(angular_error(
            bd.apply(&gcj_expected, &mut context)?.point(),
            &bd_expected,
            &prepared,
            &mut context,
        )?);
        mc_errors.push(projected_error(
            mc.apply(&bd_expected, &mut context)?.point(),
            &mc_expected,
        )?);
    }
    report("GCJ", gcj_errors, &rat("1"))?;
    report("BD09LL", bd_errors, &rat("0.02"))?;
    report("BaiduMercatorAnalytic", mc_errors, &rat("0.20"))?;
    println!(
        "rows={} elapsed_ns={}",
        rows.len(),
        started.elapsed().as_nanos()
    );
    Ok(())
}
