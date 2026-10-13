// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit offline operation models and chains; no zones, epochs or heights inferred.

use purrdf_geo_kernel::{
    CoordinateOperation, OperationChain, OperationPoint, PreparationBudget, PreparedEllipsoid, Rat,
    operation::{
        Applicability, BilinearGrid, BilinearGridInverse, CoordinateUnit, GridNode,
        HelmertParameters, HelmertRates, Hemisphere, MetricSourceDomain, OperationModel,
        OperationReference, Polynomial2d, Polynomial2dInverse, PolynomialTerm, RotationConvention,
        RotationLaw, Similarity2d, TransverseMercator, ZoneFamily,
    },
};
use purrdf_hash::hex::Digest32;
use purrdf_lex::json::{
    Object, Value,
    record::{DecodeError, Record},
};

use super::{
    GeoCallError,
    decode::Decode,
    profile::{exact_decimal, fixed_hex},
};

pub(super) fn chain(
    value: &Value,
    budget: &mut PreparationBudget,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<OperationChain, GeoCallError> {
    let operations = decode.items(value, |value| operation(value, budget, decode))?;
    OperationChain::compile_in_budget(operations, budget).map_err(Into::into)
}

fn operation(
    value: &Value,
    budget: &mut PreparationBudget,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<CoordinateOperation, GeoCallError> {
    let mut fields = decode.record(value, "explicit coordinate operation")?;
    let source = fields.required_with("source", |value| reference(value, decode))?;
    let target = fields.required_with("target", |value| reference(value, decode))?;
    let model = fields.required_with("model", |value| model(value, budget, decode))?;
    fields.deny_unknown()?;
    CoordinateOperation::compile_in_budget(source, target, model, budget).map_err(Into::into)
}

fn reference(
    value: &Value,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<OperationReference, DecodeError> {
    let mut fields = decode.record(value, "coordinate operation reference")?;
    let realization =
        Digest32::new(fields.required_with("realization", |value| fixed_hex::<32>(value, decode))?);
    let unit = match fields.tag("unit", &["degrees", "metres"])? {
        "degrees" => CoordinateUnit::Degrees,
        _ => CoordinateUnit::Metres,
    };
    let swapped_axes = fields.required_native("swapped_axes")?;
    fields.deny_unknown()?;
    Ok(OperationReference {
        realization,
        unit,
        swapped_axes,
    })
}

pub(super) fn point(
    value: &Value,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<OperationPoint, GeoCallError> {
    let mut fields = decode.record(value, "exact operation point")?;
    let point = OperationPoint {
        x: fields.required_with("x", |value| decode.decimal(value))?,
        y: fields.required_with("y", |value| decode.decimal(value))?,
        z: fields.optional_with("z_metres", |value| decode.decimal(value))?,
        epoch: fields.optional_with("epoch_decimal_year", |value| decode.decimal(value))?,
    };
    fields.deny_unknown()?;
    Ok(point)
}

fn exact_array<const N: usize>(
    value: &Value,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<[Rat; N], GeoCallError> {
    decode
        .items(value, |value| decode.decimal(value))?
        .try_into()
        .map_err(|_| {
            decode
                .custom(format_args!("expected exactly {N} decimal coordinates"))
                .into()
        })
}

fn model(
    value: &Value,
    budget: &mut PreparationBudget,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<OperationModel, GeoCallError> {
    let mut fields = decode.record(value, "coordinate operation model")?;
    let law = fields.tag(
        "law",
        &[
            "gcj-rational-harmonic-v1",
            "gcj-rational-harmonic-inverse-v1",
            "bd09ll-v1",
            "bd09ll-inverse-v1",
            "baidu-mercator-analytic-v1",
            "ellipsoidal-mercator-v1",
            "web-mercator",
            "transverse-mercator",
            "transverse-mercator-inverse",
            "geographic-to-geocentric",
            "mercator-to-geographic",
            "geocentric-to-geographic",
            "helmert-small-angle-v1",
            "helmert-euler-rz-ry-rx-v1",
            "similarity2d-v1",
            "polynomial2d",
            "polynomial2d-inverse",
            "bilinear-grid",
            "bilinear-grid-inverse",
        ],
    )?;
    let model = match law {
        "gcj-rational-harmonic-v1" => OperationModel::GcjRationalHarmonicV1(
            fields.required_with("applicability", |value| {
                applicability(value, budget, decode)
            })?,
        ),
        "gcj-rational-harmonic-inverse-v1" => OperationModel::GcjRationalHarmonicInverseV1(
            fields.required_with("applicability", |value| {
                applicability(value, budget, decode)
            })?,
        ),
        "bd09ll-v1" => OperationModel::Bd09LlV1,
        "bd09ll-inverse-v1" => OperationModel::Bd09LlInverseV1,
        "baidu-mercator-analytic-v1" => OperationModel::BaiduMercatorAnalyticV1,
        "ellipsoidal-mercator-v1" => OperationModel::EllipsoidalMercator {
            semimajor: fields.required_with("semimajor_metres", |value| decode.decimal(value))?,
            eccentricity_squared: fields
                .required_with("eccentricity_squared", |value| decode.decimal(value))?,
        },
        "web-mercator" => OperationModel::WebMercator {
            radius: fields.required_with("radius_metres", |value| decode.decimal(value))?,
        },
        "transverse-mercator" | "transverse-mercator-inverse" => {
            let projection = decode.boxed(TransverseMercator {
                ellipsoid: fields
                    .required_with("ellipsoid", |value| ellipsoid(value, budget, decode))?,
                family: match fields.tag("family", &["gauss-kruger-3", "gauss-kruger-6", "utm"])? {
                    "gauss-kruger-3" => ZoneFamily::GaussKruger3,
                    "gauss-kruger-6" => ZoneFamily::GaussKruger6,
                    _ => ZoneFamily::Utm,
                },
                zone: fields.required_native("zone")?,
                central_meridian: fields
                    .required_with("central_meridian_degrees", |value| decode.decimal(value))?,
                scale: fields.required_with("scale", |value| decode.decimal(value))?,
                false_easting: fields
                    .required_with("false_easting_metres", |value| decode.decimal(value))?,
                false_northing: fields
                    .required_with("false_northing_metres", |value| decode.decimal(value))?,
                hemisphere: match fields.tag("hemisphere", &["north", "south"])? {
                    "north" => Hemisphere::North,
                    _ => Hemisphere::South,
                },
                zone_prefix: fields.required_native("zone_prefix")?,
            })?;
            if law == "transverse-mercator" {
                OperationModel::TransverseMercator(projection)
            } else {
                OperationModel::TransverseMercatorInverse(projection)
            }
        }
        "geographic-to-geocentric" => OperationModel::GeographicToGeocentric {
            ellipsoid: fields
                .required_with("ellipsoid", |value| ellipsoid(value, budget, decode))?,
        },
        "geocentric-to-geographic" => OperationModel::GeocentricToGeographic {
            ellipsoid: fields
                .required_with("ellipsoid", |value| ellipsoid(value, budget, decode))?,
        },
        "mercator-to-geographic" => OperationModel::MercatorToGeographic {
            radius: fields.required_with("radius_metres", |value| decode.decimal(value))?,
            eccentricity_squared: fields
                .required_with("eccentricity_squared", |value| decode.decimal(value))?,
            square_domain: fields.required_native("square_domain")?,
        },
        "helmert-small-angle-v1" | "helmert-euler-rz-ry-rx-v1" => {
            OperationModel::Helmert(decode.boxed(HelmertParameters {
                translation: fields.required_with("translation_metres", |value| {
                    exact_array::<3>(value, decode)
                })?,
                rotation_arcseconds: fields.required_with("rotation_arcseconds", |value| {
                    exact_array::<3>(value, decode)
                })?,
                scale_ppm: fields.required_with("scale_ppm", |value| decode.decimal(value))?,
                pivot:
                    fields.required_with("pivot_metres", |value| exact_array::<3>(value, decode))?,
                law: if law == "helmert-small-angle-v1" {
                    RotationLaw::HelmertSmallAngleV1
                } else {
                    RotationLaw::HelmertEulerRzRyRxV1
                },
                convention: convention(&mut fields)?,
                rates: fields.optional_with("rates", |value| rates(value, decode))?,
                inverse: fields.required_native("inverse")?,
            })?)
        }
        "similarity2d-v1" => OperationModel::Similarity2d(Similarity2d {
            translation: fields.required_with("translation_metres", |value| {
                exact_array::<2>(value, decode)
            })?,
            scale: fields.required_with("scale", |value| decode.decimal(value))?,
            rotation_degrees: fields
                .required_with("rotation_degrees", |value| decode.decimal(value))?,
            convention: convention(&mut fields)?,
            inverse: fields.required_native("inverse")?,
        }),
        "polynomial2d" | "polynomial2d-inverse" => {
            let polynomial = polynomial(&mut fields, budget, decode)?;
            if law == "polynomial2d" {
                OperationModel::Polynomial2d(polynomial)
            } else {
                OperationModel::Polynomial2dInverse(decode.boxed(Polynomial2dInverse {
                    polynomial,
                    source_domain: fields.required_with("source_domain", |value| {
                        source_domain(value, budget, decode)
                    })?,
                })?)
            }
        }
        _ => {
            let grid = grid(&mut fields, budget, decode)?;
            if law == "bilinear-grid" {
                OperationModel::BilinearGrid(grid)
            } else {
                OperationModel::BilinearGridInverse(decode.boxed(BilinearGridInverse {
                    grid,
                    source_domain: fields.required_with("source_domain", |value| {
                        source_domain(value, budget, decode)
                    })?,
                })?)
            }
        }
    };
    fields.deny_unknown()?;
    Ok(model)
}

fn polynomial(
    fields: &mut Record<'_>,
    budget: &mut PreparationBudget,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Polynomial2d, GeoCallError> {
    fields.tag("basis", &["monomial"])?;
    fields.tag("order", &["x-power-y-power"])?;
    Polynomial2d::new_in_budget(
        fields.required_native("degree")?,
        fields.required_with("origin_metres", |value| exact_array::<2>(value, decode))?,
        fields.required_with("normalization_metres", |value| {
            exact_array::<2>(value, decode)
        })?,
        fields.required_with("terms", |value| {
            decode.items(value, |value| polynomial_term(value, decode))
        })?,
        budget,
    )
    .map_err(Into::into)
}
fn grid(
    fields: &mut Record<'_>,
    budget: &mut PreparationBudget,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<BilinearGrid, GeoCallError> {
    BilinearGrid::new_in_budget(
        fields.required_with("origin_metres", |value| exact_array::<2>(value, decode))?,
        fields.required_with("spacing_metres", |value| exact_array::<2>(value, decode))?,
        fields.required_native("columns")?,
        fields.required_native("rows")?,
        fields.required_with("nodes", |value| {
            decode.items(value, |value| grid_node(value, decode))
        })?,
        budget,
    )
    .map_err(Into::into)
}
fn source_domain(
    value: &Value,
    budget: &mut PreparationBudget,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<MetricSourceDomain, GeoCallError> {
    let mut fields = decode.record(value, "explicit closed metre source rectangle")?;
    let min_x = fields.required_with("min_x_metres", |value| decode.decimal(value))?;
    let max_x = fields.required_with("max_x_metres", |value| decode.decimal(value))?;
    let min_y = fields.required_with("min_y_metres", |value| decode.decimal(value))?;
    let max_y = fields.required_with("max_y_metres", |value| decode.decimal(value))?;
    fields.deny_unknown()?;
    MetricSourceDomain::new_in_budget([min_x, min_y], [max_x, max_y], budget).map_err(Into::into)
}

fn applicability(
    value: &Value,
    budget: &mut PreparationBudget,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Applicability, GeoCallError> {
    let mut fields = decode.record(value, "explicit exact closed applicability")?;
    let bounds = Applicability::new_in_budget(
        fields.required_with("west", |value| decode.decimal(value))?,
        fields.required_with("east", |value| decode.decimal(value))?,
        fields.required_with("south", |value| decode.decimal(value))?,
        fields.required_with("north", |value| decode.decimal(value))?,
        budget,
    )?;
    fields.deny_unknown()?;
    Ok(bounds)
}

pub(super) fn ellipsoid(
    value: &Value,
    budget: &mut PreparationBudget,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<PreparedEllipsoid, GeoCallError> {
    let mut fields = decode.record(value, "explicit ellipsoid")?;
    let ellipsoid = match fields.tag("profile", &["wgs84", "cgcs2000", "custom"])? {
        "wgs84" => PreparedEllipsoid::wgs84(),
        "cgcs2000" => PreparedEllipsoid::cgcs2000(),
        _ => PreparedEllipsoid::new_in_budget(
            fields.required_with("semimajor_metres", |value| decode.decimal(value))?,
            fields.required_with("inverse_flattening", |value| decode.decimal(value))?,
            budget,
        )?,
    };
    fields.deny_unknown()?;
    Ok(ellipsoid)
}

fn convention(fields: &mut Record<'_>) -> Result<RotationConvention, DecodeError> {
    Ok(
        match fields.tag("convention", &["position-vector", "coordinate-frame"])? {
            "position-vector" => RotationConvention::PositionVector,
            _ => RotationConvention::CoordinateFrame,
        },
    )
}

fn rates(value: &Value, decode: &Decode<'_, '_, '_, '_>) -> Result<HelmertRates, GeoCallError> {
    let mut fields = decode.record(value, "explicit Helmert rates")?;
    let rates = HelmertRates {
        epoch: fields.required_with("epoch_decimal_year", |value| decode.decimal(value))?,
        translation: fields.required_with("translation_metres_per_year", |value| {
            exact_array::<3>(value, decode)
        })?,
        rotation_arcseconds: fields.required_with("rotation_arcseconds_per_year", |value| {
            exact_array::<3>(value, decode)
        })?,
        scale_ppm: fields.required_with("scale_ppm_per_year", |value| decode.decimal(value))?,
    };
    fields.deny_unknown()?;
    Ok(rates)
}

fn polynomial_term(
    value: &Value,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<PolynomialTerm, GeoCallError> {
    let mut fields = decode.record(value, "explicit polynomial monomial")?;
    let term = PolynomialTerm {
        x_power: fields.required_native("x_power")?,
        y_power: fields.required_native("y_power")?,
        x_coefficient: fields
            .required_with("x_coefficient_metres", |value| decode.decimal(value))?,
        y_coefficient: fields
            .required_with("y_coefficient_metres", |value| decode.decimal(value))?,
    };
    fields.deny_unknown()?;
    Ok(term)
}

fn grid_node(
    value: &Value,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Option<GridNode>, GeoCallError> {
    if matches!(value, Value::Null) {
        return Ok(None);
    }
    let mut fields = decode.record(value, "explicit bilinear displacement node")?;
    let node = GridNode {
        x: fields.required_with("x_metres", |value| decode.decimal(value))?,
        y: fields.required_with("y_metres", |value| decode.decimal(value))?,
    };
    fields.deny_unknown()?;
    Ok(Some(node))
}

pub(super) fn chain_json(
    chain: &OperationChain,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    decode.array(chain.operations().iter(), |operation| {
        operation_json(operation, decode)
    })
}

fn reference_json(
    reference: OperationReference,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    Ok(Object::new()
        .with_memory_cell(
            "realization",
            decode.format(&reference.realization)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "unit",
            match reference.unit {
                CoordinateUnit::Degrees => "degrees",
                CoordinateUnit::Metres => "metres",
            },
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "swapped_axes",
            reference.swapped_axes,
            decode.storage.memory(),
        )?
        .into())
}

fn operation_json(
    operation: &CoordinateOperation,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    Ok(Object::new()
        .with_memory_cell(
            "source",
            reference_json(operation.source(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "target",
            reference_json(operation.target(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "model",
            model_json(operation.model(), decode)?,
            decode.storage.memory(),
        )?
        .into())
}

fn array_json(values: &[Rat], decode: &Decode<'_, '_, '_, '_>) -> Result<Value, GeoCallError> {
    decode.array(values.iter(), |value| {
        exact_decimal(value, decode).map(Value::String)
    })
}
fn convention_name(value: RotationConvention) -> &'static str {
    match value {
        RotationConvention::PositionVector => "position-vector",
        RotationConvention::CoordinateFrame => "coordinate-frame",
    }
}

fn ellipsoid_json(
    ellipsoid: &PreparedEllipsoid,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    if ellipsoid == &PreparedEllipsoid::wgs84() {
        return Ok(Object::new()
            .with_memory_cell("profile", "wgs84", decode.storage.memory())?
            .into());
    }
    if ellipsoid == &PreparedEllipsoid::cgcs2000() {
        return Ok(Object::new()
            .with_memory_cell("profile", "cgcs2000", decode.storage.memory())?
            .into());
    }
    Ok(Object::new()
        .with_memory_cell("profile", "custom", decode.storage.memory())?
        .with_memory_cell(
            "semimajor_metres",
            exact_decimal(ellipsoid.semimajor(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "inverse_flattening",
            exact_decimal(ellipsoid.inverse_flattening(), decode)?,
            decode.storage.memory(),
        )?
        .into())
}

fn model_json(
    model: &OperationModel,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    let fields = match model {
        OperationModel::GcjRationalHarmonicV1(bounds)
        | OperationModel::GcjRationalHarmonicInverseV1(bounds) => {
            let law = if matches!(model, OperationModel::GcjRationalHarmonicV1(_)) {
                "gcj-rational-harmonic-v1"
            } else {
                "gcj-rational-harmonic-inverse-v1"
            };
            Object::new()
                .with_memory_cell("law", law, decode.storage.memory())?
                .with_memory_cell(
                    "applicability",
                    Object::new()
                        .with_memory_cell(
                            "west",
                            exact_decimal(&bounds.west, decode)?,
                            decode.storage.memory(),
                        )?
                        .with_memory_cell(
                            "east",
                            exact_decimal(&bounds.east, decode)?,
                            decode.storage.memory(),
                        )?
                        .with_memory_cell(
                            "south",
                            exact_decimal(&bounds.south, decode)?,
                            decode.storage.memory(),
                        )?
                        .with_memory_cell(
                            "north",
                            exact_decimal(&bounds.north, decode)?,
                            decode.storage.memory(),
                        )?,
                    decode.storage.memory(),
                )?
        }
        OperationModel::Bd09LlV1 => {
            Object::new().with_memory_cell("law", "bd09ll-v1", decode.storage.memory())?
        }
        OperationModel::Bd09LlInverseV1 => {
            Object::new().with_memory_cell("law", "bd09ll-inverse-v1", decode.storage.memory())?
        }
        OperationModel::BaiduMercatorAnalyticV1 => Object::new().with_memory_cell(
            "law",
            "baidu-mercator-analytic-v1",
            decode.storage.memory(),
        )?,
        OperationModel::EllipsoidalMercator {
            semimajor,
            eccentricity_squared,
        } => Object::new()
            .with_memory_cell("law", "ellipsoidal-mercator-v1", decode.storage.memory())?
            .with_memory_cell(
                "semimajor_metres",
                exact_decimal(semimajor, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "eccentricity_squared",
                exact_decimal(eccentricity_squared, decode)?,
                decode.storage.memory(),
            )?,
        OperationModel::WebMercator { radius } => Object::new()
            .with_memory_cell("law", "web-mercator", decode.storage.memory())?
            .with_memory_cell(
                "radius_metres",
                exact_decimal(radius, decode)?,
                decode.storage.memory(),
            )?,
        OperationModel::TransverseMercator(projection)
        | OperationModel::TransverseMercatorInverse(projection) => Object::new()
            .with_memory_cell(
                "law",
                if matches!(model, OperationModel::TransverseMercator(_)) {
                    "transverse-mercator"
                } else {
                    "transverse-mercator-inverse"
                },
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "ellipsoid",
                ellipsoid_json(&projection.ellipsoid, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "family",
                match projection.family {
                    ZoneFamily::GaussKruger3 => "gauss-kruger-3",
                    ZoneFamily::GaussKruger6 => "gauss-kruger-6",
                    ZoneFamily::Utm => "utm",
                },
                decode.storage.memory(),
            )?
            .with_memory_cell("zone", projection.zone, decode.storage.memory())?
            .with_memory_cell(
                "central_meridian_degrees",
                exact_decimal(&projection.central_meridian, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "scale",
                exact_decimal(&projection.scale, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "false_easting_metres",
                exact_decimal(&projection.false_easting, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "false_northing_metres",
                exact_decimal(&projection.false_northing, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "hemisphere",
                match projection.hemisphere {
                    Hemisphere::North => "north",
                    Hemisphere::South => "south",
                },
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "zone_prefix",
                projection.zone_prefix,
                decode.storage.memory(),
            )?,
        OperationModel::GeographicToGeocentric { ellipsoid } => Object::new()
            .with_memory_cell("law", "geographic-to-geocentric", decode.storage.memory())?
            .with_memory_cell(
                "ellipsoid",
                ellipsoid_json(ellipsoid, decode)?,
                decode.storage.memory(),
            )?,
        OperationModel::GeocentricToGeographic { ellipsoid } => Object::new()
            .with_memory_cell("law", "geocentric-to-geographic", decode.storage.memory())?
            .with_memory_cell(
                "ellipsoid",
                ellipsoid_json(ellipsoid, decode)?,
                decode.storage.memory(),
            )?,
        OperationModel::MercatorToGeographic {
            radius,
            eccentricity_squared,
            square_domain,
        } => Object::new()
            .with_memory_cell("law", "mercator-to-geographic", decode.storage.memory())?
            .with_memory_cell(
                "radius_metres",
                exact_decimal(radius, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "eccentricity_squared",
                exact_decimal(eccentricity_squared, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell("square_domain", *square_domain, decode.storage.memory())?,
        OperationModel::Helmert(parameters) => Object::new()
            .with_memory_cell(
                "law",
                match parameters.law {
                    RotationLaw::HelmertSmallAngleV1 => "helmert-small-angle-v1",
                    RotationLaw::HelmertEulerRzRyRxV1 => "helmert-euler-rz-ry-rx-v1",
                },
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "translation_metres",
                array_json(&parameters.translation, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "rotation_arcseconds",
                array_json(&parameters.rotation_arcseconds, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "scale_ppm",
                exact_decimal(&parameters.scale_ppm, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "pivot_metres",
                array_json(&parameters.pivot, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "convention",
                convention_name(parameters.convention),
                decode.storage.memory(),
            )?
            .with_memory_cell("inverse", parameters.inverse, decode.storage.memory())?
            .with_memory_cell(
                "rates",
                parameters
                    .rates
                    .as_ref()
                    .map(|rates| {
                        Ok::<_, GeoCallError>(
                            Object::new()
                                .with_memory_cell(
                                    "epoch_decimal_year",
                                    exact_decimal(&rates.epoch, decode)?,
                                    decode.storage.memory(),
                                )?
                                .with_memory_cell(
                                    "translation_metres_per_year",
                                    array_json(&rates.translation, decode)?,
                                    decode.storage.memory(),
                                )?
                                .with_memory_cell(
                                    "rotation_arcseconds_per_year",
                                    array_json(&rates.rotation_arcseconds, decode)?,
                                    decode.storage.memory(),
                                )?
                                .with_memory_cell(
                                    "scale_ppm_per_year",
                                    exact_decimal(&rates.scale_ppm, decode)?,
                                    decode.storage.memory(),
                                )?,
                        )
                    })
                    .transpose()?,
                decode.storage.memory(),
            )?,
        OperationModel::Similarity2d(parameters) => Object::new()
            .with_memory_cell("law", "similarity2d-v1", decode.storage.memory())?
            .with_memory_cell(
                "translation_metres",
                array_json(&parameters.translation, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "scale",
                exact_decimal(&parameters.scale, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "rotation_degrees",
                exact_decimal(&parameters.rotation_degrees, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "convention",
                convention_name(parameters.convention),
                decode.storage.memory(),
            )?
            .with_memory_cell("inverse", parameters.inverse, decode.storage.memory())?,
        OperationModel::Polynomial2d(polynomial) => {
            polynomial_json(polynomial, "polynomial2d", decode)?
        }
        OperationModel::Polynomial2dInverse(parameters) => {
            polynomial_json(&parameters.polynomial, "polynomial2d-inverse", decode)?
                .with_memory_cell(
                    "source_domain",
                    source_domain_json(&parameters.source_domain, decode)?,
                    decode.storage.memory(),
                )?
        }
        OperationModel::BilinearGrid(grid) => grid_json(grid, "bilinear-grid", decode)?,
        OperationModel::BilinearGridInverse(parameters) => {
            grid_json(&parameters.grid, "bilinear-grid-inverse", decode)?.with_memory_cell(
                "source_domain",
                source_domain_json(&parameters.source_domain, decode)?,
                decode.storage.memory(),
            )?
        }
    };
    Ok(fields.into())
}

fn source_domain_json(
    domain: &MetricSourceDomain,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    Ok(Object::new()
        .with_memory_cell(
            "min_x_metres",
            exact_decimal(&domain.lower()[0], decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "max_x_metres",
            exact_decimal(&domain.upper()[0], decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "min_y_metres",
            exact_decimal(&domain.lower()[1], decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "max_y_metres",
            exact_decimal(&domain.upper()[1], decode)?,
            decode.storage.memory(),
        )?
        .into())
}
fn polynomial_json(
    polynomial: &Polynomial2d,
    law: &str,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Object, GeoCallError> {
    let terms = decode.array(polynomial.terms().iter(), |term| {
        Ok(Object::new()
            .with_memory_cell("x_power", term.x_power, decode.storage.memory())?
            .with_memory_cell("y_power", term.y_power, decode.storage.memory())?
            .with_memory_cell(
                "x_coefficient_metres",
                exact_decimal(&term.x_coefficient, decode)?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "y_coefficient_metres",
                exact_decimal(&term.y_coefficient, decode)?,
                decode.storage.memory(),
            )?
            .into())
    })?;
    Ok(Object::new()
        .with_memory_cell("law", law, decode.storage.memory())?
        .with_memory_cell("basis", "monomial", decode.storage.memory())?
        .with_memory_cell("order", "x-power-y-power", decode.storage.memory())?
        .with_memory_cell("degree", polynomial.degree(), decode.storage.memory())?
        .with_memory_cell(
            "origin_metres",
            array_json(polynomial.origin(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "normalization_metres",
            array_json(polynomial.scale(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell("terms", terms, decode.storage.memory())?)
}
fn grid_json(
    grid: &BilinearGrid,
    law: &str,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Object, GeoCallError> {
    let nodes = decode.array(grid.nodes().iter(), |node| {
        node.as_ref()
            .map(|node| {
                Ok::<_, GeoCallError>(
                    Object::new()
                        .with_memory_cell(
                            "x_metres",
                            exact_decimal(&node.x, decode)?,
                            decode.storage.memory(),
                        )?
                        .with_memory_cell(
                            "y_metres",
                            exact_decimal(&node.y, decode)?,
                            decode.storage.memory(),
                        )?
                        .into(),
                )
            })
            .transpose()
            .map(|value| value.unwrap_or(Value::Null))
    })?;
    Ok(Object::new()
        .with_memory_cell("law", law, decode.storage.memory())?
        .with_memory_cell(
            "origin_metres",
            array_json(grid.origin(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "spacing_metres",
            array_json(grid.spacing(), decode)?,
            decode.storage.memory(),
        )?
        .with_memory_cell("columns", grid.dimensions().0, decode.storage.memory())?
        .with_memory_cell("rows", grid.dimensions().1, decode.storage.memory())?
        .with_memory_cell("nodes", nodes, decode.storage.memory())?)
}
