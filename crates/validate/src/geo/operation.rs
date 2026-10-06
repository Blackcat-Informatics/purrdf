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
    record::{DecodeError, Record, items_with},
};

use super::{
    GeoCallError,
    profile::{decimal, exact_decimal, fixed_hex},
};

pub(super) fn chain(
    value: &Value,
    budget: &mut PreparationBudget,
) -> Result<OperationChain, GeoCallError> {
    let operations = items_with(value, |value| operation(value, budget))?;
    OperationChain::compile_in_budget(operations, budget).map_err(Into::into)
}

fn operation(
    value: &Value,
    budget: &mut PreparationBudget,
) -> Result<CoordinateOperation, GeoCallError> {
    let mut fields = Record::new(value, "explicit coordinate operation")?;
    let source = fields.required_with("source", reference)?;
    let target = fields.required_with("target", reference)?;
    let model = fields.required_with("model", |value| model(value, budget))?;
    fields.deny_unknown()?;
    CoordinateOperation::compile_in_budget(source, target, model, budget).map_err(Into::into)
}

fn reference(value: &Value) -> Result<OperationReference, DecodeError> {
    let mut fields = Record::new(value, "coordinate operation reference")?;
    let realization = Digest32::new(fields.required_with("realization", fixed_hex::<32>)?);
    let unit = match fields.tag("unit", &["degrees", "metres"])? {
        "degrees" => CoordinateUnit::Degrees,
        _ => CoordinateUnit::Metres,
    };
    let swapped_axes = fields.required("swapped_axes")?;
    fields.deny_unknown()?;
    Ok(OperationReference {
        realization,
        unit,
        swapped_axes,
    })
}

pub(super) fn point(value: &Value) -> Result<OperationPoint, DecodeError> {
    let mut fields = Record::new(value, "exact operation point")?;
    let point = OperationPoint {
        x: fields.required_with("x", decimal)?,
        y: fields.required_with("y", decimal)?,
        z: fields.optional_with("z_metres", decimal)?,
        epoch: fields.optional_with("epoch_decimal_year", decimal)?,
    };
    fields.deny_unknown()?;
    Ok(point)
}

fn exact_array<const N: usize>(value: &Value) -> Result<[Rat; N], DecodeError> {
    items_with(value, decimal)?
        .try_into()
        .map_err(|_| DecodeError::custom(format!("expected exactly {N} decimal coordinates")))
}

fn model(value: &Value, budget: &mut PreparationBudget) -> Result<OperationModel, GeoCallError> {
    let mut fields = Record::new(value, "coordinate operation model")?;
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
            fields.required_with("applicability", |value| applicability(value, budget))?,
        ),
        "gcj-rational-harmonic-inverse-v1" => OperationModel::GcjRationalHarmonicInverseV1(
            fields.required_with("applicability", |value| applicability(value, budget))?,
        ),
        "bd09ll-v1" => OperationModel::Bd09LlV1,
        "bd09ll-inverse-v1" => OperationModel::Bd09LlInverseV1,
        "baidu-mercator-analytic-v1" => OperationModel::BaiduMercatorAnalyticV1,
        "ellipsoidal-mercator-v1" => OperationModel::EllipsoidalMercator {
            semimajor: fields.required_with("semimajor_metres", decimal)?,
            eccentricity_squared: fields.required_with("eccentricity_squared", decimal)?,
        },
        "web-mercator" => OperationModel::WebMercator {
            radius: fields.required_with("radius_metres", decimal)?,
        },
        "transverse-mercator" | "transverse-mercator-inverse" => {
            let projection = Box::new(TransverseMercator {
                ellipsoid: fields.required_with("ellipsoid", |value| ellipsoid(value, budget))?,
                family: match fields.tag("family", &["gauss-kruger-3", "gauss-kruger-6", "utm"])? {
                    "gauss-kruger-3" => ZoneFamily::GaussKruger3,
                    "gauss-kruger-6" => ZoneFamily::GaussKruger6,
                    _ => ZoneFamily::Utm,
                },
                zone: fields.required("zone")?,
                central_meridian: fields.required_with("central_meridian_degrees", decimal)?,
                scale: fields.required_with("scale", decimal)?,
                false_easting: fields.required_with("false_easting_metres", decimal)?,
                false_northing: fields.required_with("false_northing_metres", decimal)?,
                hemisphere: match fields.tag("hemisphere", &["north", "south"])? {
                    "north" => Hemisphere::North,
                    _ => Hemisphere::South,
                },
                zone_prefix: fields.required("zone_prefix")?,
            });
            if law == "transverse-mercator" {
                OperationModel::TransverseMercator(projection)
            } else {
                OperationModel::TransverseMercatorInverse(projection)
            }
        }
        "geographic-to-geocentric" => OperationModel::GeographicToGeocentric {
            ellipsoid: fields.required_with("ellipsoid", |value| ellipsoid(value, budget))?,
        },
        "geocentric-to-geographic" => OperationModel::GeocentricToGeographic {
            ellipsoid: fields.required_with("ellipsoid", |value| ellipsoid(value, budget))?,
        },
        "mercator-to-geographic" => OperationModel::MercatorToGeographic {
            radius: fields.required_with("radius_metres", decimal)?,
            eccentricity_squared: fields.required_with("eccentricity_squared", decimal)?,
            square_domain: fields.required("square_domain")?,
        },
        "helmert-small-angle-v1" | "helmert-euler-rz-ry-rx-v1" => {
            OperationModel::Helmert(Box::new(HelmertParameters {
                translation: fields.required_with("translation_metres", exact_array::<3>)?,
                rotation_arcseconds: fields
                    .required_with("rotation_arcseconds", exact_array::<3>)?,
                scale_ppm: fields.required_with("scale_ppm", decimal)?,
                pivot: fields.required_with("pivot_metres", exact_array::<3>)?,
                law: if law == "helmert-small-angle-v1" {
                    RotationLaw::HelmertSmallAngleV1
                } else {
                    RotationLaw::HelmertEulerRzRyRxV1
                },
                convention: convention(&mut fields)?,
                rates: fields.optional_with("rates", rates)?,
                inverse: fields.required("inverse")?,
            }))
        }
        "similarity2d-v1" => OperationModel::Similarity2d(Similarity2d {
            translation: fields.required_with("translation_metres", exact_array::<2>)?,
            scale: fields.required_with("scale", decimal)?,
            rotation_degrees: fields.required_with("rotation_degrees", decimal)?,
            convention: convention(&mut fields)?,
            inverse: fields.required("inverse")?,
        }),
        "polynomial2d" | "polynomial2d-inverse" => {
            let polynomial = polynomial(&mut fields, budget)?;
            if law == "polynomial2d" {
                OperationModel::Polynomial2d(polynomial)
            } else {
                OperationModel::Polynomial2dInverse(Box::new(Polynomial2dInverse {
                    polynomial,
                    source_domain: fields
                        .required_with("source_domain", |value| source_domain(value, budget))?,
                }))
            }
        }
        _ => {
            let grid = grid(&mut fields, budget)?;
            if law == "bilinear-grid" {
                OperationModel::BilinearGrid(grid)
            } else {
                OperationModel::BilinearGridInverse(Box::new(BilinearGridInverse {
                    grid,
                    source_domain: fields
                        .required_with("source_domain", |value| source_domain(value, budget))?,
                }))
            }
        }
    };
    fields.deny_unknown()?;
    Ok(model)
}

fn polynomial(
    fields: &mut Record<'_>,
    budget: &mut PreparationBudget,
) -> Result<Polynomial2d, GeoCallError> {
    fields.tag("basis", &["monomial"])?;
    fields.tag("order", &["x-power-y-power"])?;
    Polynomial2d::new_in_budget(
        fields.required("degree")?,
        fields.required_with("origin_metres", exact_array::<2>)?,
        fields.required_with("normalization_metres", exact_array::<2>)?,
        fields.required_with("terms", |value| items_with(value, polynomial_term))?,
        budget,
    )
    .map_err(Into::into)
}
fn grid(
    fields: &mut Record<'_>,
    budget: &mut PreparationBudget,
) -> Result<BilinearGrid, GeoCallError> {
    BilinearGrid::new_in_budget(
        fields.required_with("origin_metres", exact_array::<2>)?,
        fields.required_with("spacing_metres", exact_array::<2>)?,
        fields.required("columns")?,
        fields.required("rows")?,
        fields.required_with("nodes", |value| items_with(value, grid_node))?,
        budget,
    )
    .map_err(Into::into)
}
fn source_domain(
    value: &Value,
    budget: &mut PreparationBudget,
) -> Result<MetricSourceDomain, GeoCallError> {
    let mut fields = Record::new(value, "explicit closed metre source rectangle")?;
    let min_x = fields.required_with("min_x_metres", decimal)?;
    let max_x = fields.required_with("max_x_metres", decimal)?;
    let min_y = fields.required_with("min_y_metres", decimal)?;
    let max_y = fields.required_with("max_y_metres", decimal)?;
    fields.deny_unknown()?;
    MetricSourceDomain::new_in_budget([min_x, min_y], [max_x, max_y], budget).map_err(Into::into)
}

fn applicability(
    value: &Value,
    budget: &mut PreparationBudget,
) -> Result<Applicability, GeoCallError> {
    let mut fields = Record::new(value, "explicit exact closed applicability")?;
    let bounds = Applicability::new_in_budget(
        fields.required_with("west", decimal)?,
        fields.required_with("east", decimal)?,
        fields.required_with("south", decimal)?,
        fields.required_with("north", decimal)?,
        budget,
    )?;
    fields.deny_unknown()?;
    Ok(bounds)
}

pub(super) fn ellipsoid(
    value: &Value,
    budget: &mut PreparationBudget,
) -> Result<PreparedEllipsoid, GeoCallError> {
    let mut fields = Record::new(value, "explicit ellipsoid")?;
    let ellipsoid = match fields.tag("profile", &["wgs84", "cgcs2000", "custom"])? {
        "wgs84" => PreparedEllipsoid::wgs84(),
        "cgcs2000" => PreparedEllipsoid::cgcs2000(),
        _ => PreparedEllipsoid::new_in_budget(
            fields.required_with("semimajor_metres", decimal)?,
            fields.required_with("inverse_flattening", decimal)?,
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

fn rates(value: &Value) -> Result<HelmertRates, DecodeError> {
    let mut fields = Record::new(value, "explicit Helmert rates")?;
    let rates = HelmertRates {
        epoch: fields.required_with("epoch_decimal_year", decimal)?,
        translation: fields.required_with("translation_metres_per_year", exact_array::<3>)?,
        rotation_arcseconds: fields
            .required_with("rotation_arcseconds_per_year", exact_array::<3>)?,
        scale_ppm: fields.required_with("scale_ppm_per_year", decimal)?,
    };
    fields.deny_unknown()?;
    Ok(rates)
}

fn polynomial_term(value: &Value) -> Result<PolynomialTerm, DecodeError> {
    let mut fields = Record::new(value, "explicit polynomial monomial")?;
    let term = PolynomialTerm {
        x_power: fields.required("x_power")?,
        y_power: fields.required("y_power")?,
        x_coefficient: fields.required_with("x_coefficient_metres", decimal)?,
        y_coefficient: fields.required_with("y_coefficient_metres", decimal)?,
    };
    fields.deny_unknown()?;
    Ok(term)
}

fn grid_node(value: &Value) -> Result<Option<GridNode>, DecodeError> {
    if matches!(value, Value::Null) {
        return Ok(None);
    }
    let mut fields = Record::new(value, "explicit bilinear displacement node")?;
    let node = GridNode {
        x: fields.required_with("x_metres", decimal)?,
        y: fields.required_with("y_metres", decimal)?,
    };
    fields.deny_unknown()?;
    Ok(Some(node))
}

pub(super) fn chain_json(chain: &OperationChain) -> Result<Value, GeoCallError> {
    Ok(Value::Array(
        chain
            .operations()
            .iter()
            .map(operation_json)
            .collect::<Result<_, _>>()?,
    ))
}

fn reference_json(reference: OperationReference) -> Value {
    Object::new()
        .with("realization", reference.realization.to_string())
        .with(
            "unit",
            match reference.unit {
                CoordinateUnit::Degrees => "degrees",
                CoordinateUnit::Metres => "metres",
            },
        )
        .with("swapped_axes", reference.swapped_axes)
        .into()
}

fn operation_json(operation: &CoordinateOperation) -> Result<Value, GeoCallError> {
    Ok(Object::new()
        .with("source", reference_json(operation.source()))
        .with("target", reference_json(operation.target()))
        .with("model", model_json(operation.model())?)
        .into())
}

fn array_json(values: &[Rat]) -> Result<Value, GeoCallError> {
    Ok(Value::Array(
        values
            .iter()
            .map(|value| exact_decimal(value).map(Value::String))
            .collect::<Result<_, _>>()?,
    ))
}
fn convention_name(value: RotationConvention) -> &'static str {
    match value {
        RotationConvention::PositionVector => "position-vector",
        RotationConvention::CoordinateFrame => "coordinate-frame",
    }
}

fn ellipsoid_json(ellipsoid: &PreparedEllipsoid) -> Result<Value, GeoCallError> {
    if ellipsoid == &PreparedEllipsoid::wgs84() {
        return Ok(Object::new().with("profile", "wgs84").into());
    }
    if ellipsoid == &PreparedEllipsoid::cgcs2000() {
        return Ok(Object::new().with("profile", "cgcs2000").into());
    }
    Ok(Object::new()
        .with("profile", "custom")
        .with("semimajor_metres", exact_decimal(ellipsoid.semimajor())?)
        .with(
            "inverse_flattening",
            exact_decimal(ellipsoid.inverse_flattening())?,
        )
        .into())
}

fn model_json(model: &OperationModel) -> Result<Value, GeoCallError> {
    let fields = match model {
        OperationModel::GcjRationalHarmonicV1(bounds)
        | OperationModel::GcjRationalHarmonicInverseV1(bounds) => {
            let law = if matches!(model, OperationModel::GcjRationalHarmonicV1(_)) {
                "gcj-rational-harmonic-v1"
            } else {
                "gcj-rational-harmonic-inverse-v1"
            };
            Object::new().with("law", law).with(
                "applicability",
                Object::new()
                    .with("west", exact_decimal(&bounds.west)?)
                    .with("east", exact_decimal(&bounds.east)?)
                    .with("south", exact_decimal(&bounds.south)?)
                    .with("north", exact_decimal(&bounds.north)?),
            )
        }
        OperationModel::Bd09LlV1 => Object::new().with("law", "bd09ll-v1"),
        OperationModel::Bd09LlInverseV1 => Object::new().with("law", "bd09ll-inverse-v1"),
        OperationModel::BaiduMercatorAnalyticV1 => {
            Object::new().with("law", "baidu-mercator-analytic-v1")
        }
        OperationModel::EllipsoidalMercator {
            semimajor,
            eccentricity_squared,
        } => Object::new()
            .with("law", "ellipsoidal-mercator-v1")
            .with("semimajor_metres", exact_decimal(semimajor)?)
            .with("eccentricity_squared", exact_decimal(eccentricity_squared)?),
        OperationModel::WebMercator { radius } => Object::new()
            .with("law", "web-mercator")
            .with("radius_metres", exact_decimal(radius)?),
        OperationModel::TransverseMercator(projection)
        | OperationModel::TransverseMercatorInverse(projection) => Object::new()
            .with(
                "law",
                if matches!(model, OperationModel::TransverseMercator(_)) {
                    "transverse-mercator"
                } else {
                    "transverse-mercator-inverse"
                },
            )
            .with("ellipsoid", ellipsoid_json(&projection.ellipsoid)?)
            .with(
                "family",
                match projection.family {
                    ZoneFamily::GaussKruger3 => "gauss-kruger-3",
                    ZoneFamily::GaussKruger6 => "gauss-kruger-6",
                    ZoneFamily::Utm => "utm",
                },
            )
            .with("zone", projection.zone)
            .with(
                "central_meridian_degrees",
                exact_decimal(&projection.central_meridian)?,
            )
            .with("scale", exact_decimal(&projection.scale)?)
            .with(
                "false_easting_metres",
                exact_decimal(&projection.false_easting)?,
            )
            .with(
                "false_northing_metres",
                exact_decimal(&projection.false_northing)?,
            )
            .with(
                "hemisphere",
                match projection.hemisphere {
                    Hemisphere::North => "north",
                    Hemisphere::South => "south",
                },
            )
            .with("zone_prefix", projection.zone_prefix),
        OperationModel::GeographicToGeocentric { ellipsoid } => Object::new()
            .with("law", "geographic-to-geocentric")
            .with("ellipsoid", ellipsoid_json(ellipsoid)?),
        OperationModel::GeocentricToGeographic { ellipsoid } => Object::new()
            .with("law", "geocentric-to-geographic")
            .with("ellipsoid", ellipsoid_json(ellipsoid)?),
        OperationModel::MercatorToGeographic {
            radius,
            eccentricity_squared,
            square_domain,
        } => Object::new()
            .with("law", "mercator-to-geographic")
            .with("radius_metres", exact_decimal(radius)?)
            .with("eccentricity_squared", exact_decimal(eccentricity_squared)?)
            .with("square_domain", *square_domain),
        OperationModel::Helmert(parameters) => Object::new()
            .with(
                "law",
                match parameters.law {
                    RotationLaw::HelmertSmallAngleV1 => "helmert-small-angle-v1",
                    RotationLaw::HelmertEulerRzRyRxV1 => "helmert-euler-rz-ry-rx-v1",
                },
            )
            .with("translation_metres", array_json(&parameters.translation)?)
            .with(
                "rotation_arcseconds",
                array_json(&parameters.rotation_arcseconds)?,
            )
            .with("scale_ppm", exact_decimal(&parameters.scale_ppm)?)
            .with("pivot_metres", array_json(&parameters.pivot)?)
            .with("convention", convention_name(parameters.convention))
            .with("inverse", parameters.inverse)
            .with(
                "rates",
                parameters
                    .rates
                    .as_ref()
                    .map(|rates| {
                        Ok::<_, GeoCallError>(
                            Object::new()
                                .with("epoch_decimal_year", exact_decimal(&rates.epoch)?)
                                .with(
                                    "translation_metres_per_year",
                                    array_json(&rates.translation)?,
                                )
                                .with(
                                    "rotation_arcseconds_per_year",
                                    array_json(&rates.rotation_arcseconds)?,
                                )
                                .with("scale_ppm_per_year", exact_decimal(&rates.scale_ppm)?),
                        )
                    })
                    .transpose()?,
            ),
        OperationModel::Similarity2d(parameters) => Object::new()
            .with("law", "similarity2d-v1")
            .with("translation_metres", array_json(&parameters.translation)?)
            .with("scale", exact_decimal(&parameters.scale)?)
            .with(
                "rotation_degrees",
                exact_decimal(&parameters.rotation_degrees)?,
            )
            .with("convention", convention_name(parameters.convention))
            .with("inverse", parameters.inverse),
        OperationModel::Polynomial2d(polynomial) => polynomial_json(polynomial, "polynomial2d")?,
        OperationModel::Polynomial2dInverse(parameters) => {
            polynomial_json(&parameters.polynomial, "polynomial2d-inverse")?.with(
                "source_domain",
                source_domain_json(&parameters.source_domain)?,
            )
        }
        OperationModel::BilinearGrid(grid) => grid_json(grid, "bilinear-grid")?,
        OperationModel::BilinearGridInverse(parameters) => {
            grid_json(&parameters.grid, "bilinear-grid-inverse")?.with(
                "source_domain",
                source_domain_json(&parameters.source_domain)?,
            )
        }
    };
    Ok(fields.into())
}

fn source_domain_json(domain: &MetricSourceDomain) -> Result<Value, GeoCallError> {
    Ok(Object::new()
        .with("min_x_metres", exact_decimal(&domain.lower()[0])?)
        .with("max_x_metres", exact_decimal(&domain.upper()[0])?)
        .with("min_y_metres", exact_decimal(&domain.lower()[1])?)
        .with("max_y_metres", exact_decimal(&domain.upper()[1])?)
        .into())
}
fn polynomial_json(polynomial: &Polynomial2d, law: &str) -> Result<Object, GeoCallError> {
    Ok(Object::new()
        .with("law", law)
        .with("basis", "monomial")
        .with("order", "x-power-y-power")
        .with("degree", polynomial.degree())
        .with("origin_metres", array_json(polynomial.origin())?)
        .with("normalization_metres", array_json(polynomial.scale())?)
        .with(
            "terms",
            Value::Array(
                polynomial
                    .terms()
                    .iter()
                    .map(|term| {
                        Ok::<_, GeoCallError>(
                            Object::new()
                                .with("x_power", term.x_power)
                                .with("y_power", term.y_power)
                                .with("x_coefficient_metres", exact_decimal(&term.x_coefficient)?)
                                .with("y_coefficient_metres", exact_decimal(&term.y_coefficient)?)
                                .into(),
                        )
                    })
                    .collect::<Result<_, _>>()?,
            ),
        ))
}
fn grid_json(grid: &BilinearGrid, law: &str) -> Result<Object, GeoCallError> {
    Ok(Object::new()
        .with("law", law)
        .with("origin_metres", array_json(grid.origin())?)
        .with("spacing_metres", array_json(grid.spacing())?)
        .with("columns", grid.dimensions().0)
        .with("rows", grid.dimensions().1)
        .with(
            "nodes",
            Value::Array(
                grid.nodes()
                    .iter()
                    .map(|node| {
                        node.as_ref()
                            .map(|node| {
                                Ok::<_, GeoCallError>(
                                    Object::new()
                                        .with("x_metres", exact_decimal(&node.x)?)
                                        .with("y_metres", exact_decimal(&node.y)?)
                                        .into(),
                                )
                            })
                            .transpose()
                            .map(|value| value.unwrap_or(Value::Null))
                    })
                    .collect::<Result<_, _>>()?,
            ),
        ))
}
