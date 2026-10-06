// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete reachable compiled-model storage, separate from mathematical identity.

use super::{
    BilinearGridInverse, CoordinateOperation, HelmertParameters, OperationChain, OperationModel,
    Polynomial2dInverse, TransverseMercator,
};
use crate::{GeoError, PreparationBudget};
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};

impl OperationModel {
    /// Upper count for the complete borrowed original-parameter metadata walk.
    /// Grid holes need no parameter limbs but retain their original node slots.
    pub(crate) fn original_operand_count(&self) -> Option<u64> {
        let polynomial = |terms: usize| (terms as u64).checked_mul(2)?.checked_add(4);
        let grid = |nodes: usize| (nodes as u64).checked_mul(2)?.checked_add(4);
        match self {
            Self::GcjRationalHarmonicV1(_) | Self::GcjRationalHarmonicInverseV1(_) => Some(4),
            Self::WebMercator { .. } => Some(1),
            Self::EllipsoidalMercator { .. } | Self::MercatorToGeographic { .. } => Some(2),
            Self::GeographicToGeocentric { .. } | Self::GeocentricToGeographic { .. } => Some(5),
            Self::TransverseMercator(_) | Self::TransverseMercatorInverse(_) => Some(9),
            Self::Helmert(parameters) => Some(if parameters.rates.is_some() { 18 } else { 10 }),
            Self::Similarity2d(_) => Some(4),
            Self::Polynomial2d(value) => polynomial(value.terms().len()),
            Self::BilinearGrid(value) => grid(value.nodes().len()),
            Self::Polynomial2dInverse(parameters) => {
                polynomial(parameters.polynomial.terms().len())?.checked_add(4)
            }
            Self::BilinearGridInverse(parameters) => {
                grid(parameters.grid.nodes().len())?.checked_add(4)
            }
            Self::Bd09LlV1 | Self::Bd09LlInverseV1 | Self::BaiduMercatorAnalyticV1 => Some(0),
        }
    }

    pub(super) fn admit_compile(
        &self,
        budget: &mut PreparationBudget,
    ) -> Result<ExactArithmeticCost, GeoError> {
        let count = self
            .original_operand_count()
            .ok_or(GeoError::ArithmeticOverflow(
                "compiled parameter metadata work",
            ))?;
        budget.retain(
            count.checked_add(1).ok_or(GeoError::ArithmeticOverflow(
                "compiled parameter metadata work",
            ))?,
            0,
        )?;
        let mut cost = ExactArithmeticCost::for_operation(ExactOperation::Linear, 1, 1)
            .ok_or(GeoError::ArithmeticOverflow("compiled parameter admission"))?;
        let mut bytes = Some(256_u64);
        let mut maximum = 1;
        let mut overflow = false;
        self.visit_original_operands(&mut |value| {
            maximum = maximum.max(crate::numerical::rational_operand_bits(value));
            match crate::numerical::rational_field_cost(value) {
                Ok((render, text)) => {
                    bytes = bytes.and_then(|bytes| bytes.checked_add(text));
                    if let Some(next) = cost.followed_by(render) {
                        cost = next;
                    } else {
                        overflow = true;
                    }
                }
                Err(_) => {
                    overflow = true;
                }
            }
        });
        if overflow {
            return Err(GeoError::ArithmeticOverflow("compiled parameter rendering"));
        }
        // Every actual parameter field is bounded by four per original operand,
        // with sixteen fixed model/reference fields. Grid holes retain one field
        // even though the original-operand visitor has no rational for that slot.
        let fields = count
            .checked_mul(4)
            .and_then(|n| n.checked_add(16))
            .ok_or(GeoError::ArithmeticOverflow("compiled parameter fields"))?;
        let text = bytes
            .and_then(|bytes| bytes.checked_add(fields.checked_mul(8)?))
            .ok_or(GeoError::ArithmeticOverflow("compiled parameter bytes"))?;
        cost = cost
            .followed_by(
                ExactArithmeticCost::for_operation(
                    ExactOperation::Linear,
                    text.checked_mul(8)
                        .ok_or(GeoError::ArithmeticOverflow("compiled parameter bits"))?,
                    if matches!(
                        self,
                        Self::TransverseMercator(_) | Self::TransverseMercatorInverse(_)
                    ) {
                        16
                    } else {
                        8
                    },
                )
                .ok_or(GeoError::ArithmeticOverflow("compiled parameter hashing"))?,
            )
            .ok_or(GeoError::ArithmeticOverflow("compiled parameter work"))?;
        cost = cost
            .followed_by(self.compile_validation_cost(maximum)?)
            .ok_or(GeoError::ArithmeticOverflow(
                "compiled parameter validation",
            ))?;
        // Original model, complete Vec<Vec<u8>> capacity, rendered buffers and
        // decimal digit-group scratch coexist until the preimage is dropped.
        cost.workspace_bytes = cost
            .workspace_bytes
            .checked_add(self.retained_bytes()?)
            .and_then(|bytes| {
                bytes.checked_add(fields.checked_mul(2 * size_of::<Vec<u8>>() as u64)?)
            })
            .and_then(|bytes| bytes.checked_add(text.checked_mul(2)?))
            .ok_or(GeoError::ArithmeticOverflow("compiled parameter scratch"))?;
        Ok(cost)
    }

    fn compile_validation_cost(&self, maximum: u64) -> Result<ExactArithmeticCost, GeoError> {
        let mut cost = ExactArithmeticCost::for_operation(ExactOperation::Linear, maximum, 1)
            .ok_or(GeoError::ArithmeticOverflow("compiled validation work"))?;
        let mut append = |operation, bits, count| -> Result<(), GeoError> {
            cost = cost
                .followed_by(
                    ExactArithmeticCost::for_operation(operation, bits, count)
                        .ok_or(GeoError::ArithmeticOverflow("compiled validation work"))?,
                )
                .ok_or(GeoError::ArithmeticOverflow("compiled validation work"))?;
            Ok(())
        };
        match self {
            Self::GcjRationalHarmonicV1(_) | Self::GcjRationalHarmonicInverseV1(_) => {
                append(ExactOperation::Linear, maximum, 8)?;
                append(ExactOperation::RationalCompare, maximum.max(8), 12)?;
            }
            Self::TransverseMercator(parameters) | Self::TransverseMercatorInverse(parameters) => {
                // Eccentricity depends only on inverse flattening, rather than
                // the wider prepared metric factors. Reciprocal exchanges its
                // limbs; 2-f grows by at most two bits; f*(2-f) by twice that width.
                let flattening_bits = crate::numerical::rational_operand_bits(
                    parameters.ellipsoid.inverse_flattening(),
                );
                let input_bits = flattening_bits
                    .checked_add(2)
                    .ok_or(GeoError::ArithmeticOverflow("projection validation width"))?;
                let output_bits = input_bits
                    .checked_mul(2)
                    .ok_or(GeoError::ArithmeticOverflow("projection validation width"))?;
                append(ExactOperation::Linear, flattening_bits, 2)?;
                append(ExactOperation::RationalAdd, input_bits, 1)?;
                append(ExactOperation::RationalMultiply, input_bits, 1)?;
                append(ExactOperation::RationalCompare, output_bits, 1)?;
                append(
                    ExactOperation::RationalCompare,
                    crate::numerical::rational_operand_bits(&parameters.scale),
                    1,
                )?;
                append(
                    ExactOperation::RationalCompare,
                    crate::numerical::rational_operand_bits(&parameters.central_meridian).max(20),
                    3,
                )?;
            }
            Self::BilinearGridInverse(_) => {
                let bits = maximum
                    .checked_mul(2)
                    .and_then(|bits| bits.checked_add(32))
                    .ok_or(GeoError::ArithmeticOverflow("grid validation width"))?;
                append(ExactOperation::RationalMultiply, bits, 2)?;
                append(ExactOperation::RationalAdd, bits, 2)?;
                append(ExactOperation::RationalCompare, bits, 4)?;
            }
            Self::WebMercator { .. } | Self::Similarity2d(_) | Self::Helmert(_) => {
                append(ExactOperation::RationalCompare, maximum.max(21), 1)?;
            }
            Self::EllipsoidalMercator { .. } | Self::MercatorToGeographic { .. } => {
                append(ExactOperation::RationalCompare, maximum, 3)?;
            }
            _ => {}
        }
        Ok(cost)
    }

    fn retained_bytes(&self) -> Result<u64, GeoError> {
        let extra = match self {
            Self::TransverseMercator(_) | Self::TransverseMercatorInverse(_) => {
                Some(size_of::<TransverseMercator>() as u64)
            }
            Self::Helmert(_) => Some(size_of::<HelmertParameters>() as u64),
            Self::Polynomial2d(value) => value.container_bytes(),
            Self::BilinearGrid(value) => value.container_bytes(),
            Self::Polynomial2dInverse(parameters) => parameters
                .polynomial
                .container_bytes()
                .and_then(|bytes| bytes.checked_add(size_of::<Polynomial2dInverse>() as u64)),
            Self::BilinearGridInverse(parameters) => parameters
                .grid
                .container_bytes()
                .and_then(|bytes| bytes.checked_add(size_of::<BilinearGridInverse>() as u64)),
            _ => Some(0),
        };
        let mut bytes = extra.and_then(|bytes| {
            bytes.checked_add(size_of::<Self>() as u64 + 2 * size_of::<usize>() as u64)
        });
        self.visit_original_operands(&mut |value| {
            bytes = bytes.and_then(|bytes| bytes.checked_add(value.allocated_bytes() as u64));
        });
        bytes.ok_or(GeoError::ArithmeticOverflow("compiled model storage"))
    }
}

impl CoordinateOperation {
    /// Complete reachable immutable model storage, including explicit node
    /// capacity, boxed bundles, shared ownership headers and exact limb heaps.
    /// It is a conservative admission bound, not an allocator measurement.
    ///
    /// # Errors
    /// Refuses storage-count overflow. Shared model aliases may be counted again.
    pub fn retained_workspace_bytes(&self) -> Result<u64, GeoError> {
        self.model.retained_bytes()
    }
}

impl OperationChain {
    /// Complete chain container and reachable model storage. Numerical worker
    /// scratch and caller coordinates are separate invocation allocations.
    ///
    /// # Errors
    /// Refuses checked storage-count overflow.
    pub fn retained_workspace_bytes(&self) -> Result<u64, GeoError> {
        let mut bytes = (self.operations.len() as u64)
            .checked_mul(size_of::<CoordinateOperation>() as u64)
            .and_then(|bytes| {
                bytes.checked_add(size_of::<Self>() as u64 + 2 * size_of::<usize>() as u64)
            })
            .ok_or(GeoError::ArithmeticOverflow("compiled chain storage"))?;
        for operation in self.operations.iter() {
            bytes = bytes
                .checked_add(operation.retained_workspace_bytes()?)
                .ok_or(GeoError::ArithmeticOverflow("compiled chain storage"))?;
        }
        Ok(bytes)
    }

    pub(crate) fn storage_walk_work(&self) -> Result<u64, GeoError> {
        self.operations.iter().try_fold(1_u64, |work, operation| {
            operation
                .model
                .original_operand_count()
                .and_then(|count| count.checked_add(1))
                .and_then(|count| work.checked_add(count))
                .ok_or(GeoError::ArithmeticOverflow(
                    "compiled parameter metadata work",
                ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rat;
    use crate::operation::{CoordinateUnit, OperationReference, Polynomial2d, PolynomialTerm};
    use purrdf_hash::hex::Digest32;

    #[test]
    fn configuration_refuses_large_parameters_before_rendering_or_allocating() {
        let coefficient = Rat::new(crate::Int::one().shl(16_000), crate::Int::one()).unwrap();
        let model = OperationModel::Polynomial2d(
            Polynomial2d::new(
                1,
                [Rat::zero(), Rat::zero()],
                [Rat::one(), Rat::one()],
                vec![PolynomialTerm {
                    x_power: 1,
                    y_power: 0,
                    x_coefficient: coefficient,
                    y_coefficient: Rat::zero(),
                }],
            )
            .unwrap(),
        );
        let reference = OperationReference {
            realization: Digest32::new([1; 32]),
            unit: CoordinateUnit::Metres,
            swapped_axes: false,
        };
        let policy = crate::ExecutionPolicy::new(crate::ExecutionLimits {
            max_work_items: 1024,
            ..crate::ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut budget = PreparationBudget::new(policy);
        let measurement = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result =
            CoordinateOperation::compile_in_budget(reference, reference, model, &mut budget);
        let measured = measurement.close();
        assert!(matches!(
            result,
            Err(GeoError::WorkExhausted { limit: 1024 })
        ));
        assert_eq!(measured.allocations, 0);
        assert_eq!(measured.requested_bytes, 0);
        assert!(budget.work_items() > 0);
    }

    #[test]
    fn adequate_configuration_policies_keep_exact_compiled_content() {
        let reference = OperationReference {
            realization: Digest32::new([1; 32]),
            unit: CoordinateUnit::Degrees,
            swapped_axes: false,
        };
        let first =
            CoordinateOperation::compile(reference, reference, OperationModel::Bd09LlV1).unwrap();
        let mut budget = PreparationBudget::new(
            crate::ExecutionPolicy::new(crate::ExecutionLimits {
                max_work_items: 1_000_000,
                ..crate::ExecutionLimits::GEOMETRY
            })
            .unwrap(),
        );
        let second = CoordinateOperation::compile_in_budget(
            reference,
            reference,
            OperationModel::Bd09LlV1,
            &mut budget,
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.id(), second.id());
        assert_eq!(first.law_id(), second.law_id());
        assert!(budget.work_items() > 0);
        assert_eq!(budget.workspace_bytes(), 0);
    }

    #[test]
    fn original_capacity_changes_storage_evidence_without_changing_content() {
        let operation = |capacity| {
            let mut terms = Vec::with_capacity(capacity);
            terms.push(PolynomialTerm {
                x_power: 1,
                y_power: 0,
                x_coefficient: Rat::one(),
                y_coefficient: Rat::zero(),
            });
            CoordinateOperation::compile(
                OperationReference {
                    realization: Digest32::new([1; 32]),
                    unit: CoordinateUnit::Metres,
                    swapped_axes: false,
                },
                OperationReference {
                    realization: Digest32::new([2; 32]),
                    unit: CoordinateUnit::Metres,
                    swapped_axes: false,
                },
                OperationModel::Polynomial2d(
                    Polynomial2d::new(
                        1,
                        [Rat::zero(), Rat::zero()],
                        [Rat::one(), Rat::one()],
                        terms,
                    )
                    .unwrap(),
                ),
            )
            .unwrap()
        };
        let compact = operation(1);
        let retained = operation(4096);
        assert_eq!(compact, retained);
        assert_eq!(compact.id(), retained.id());
        assert!(
            retained.retained_workspace_bytes().unwrap()
                > compact.retained_workspace_bytes().unwrap() + 64 * 1024
        );
        let mut profile = crate::GeoProfile::standard();
        profile
            .register_operation_in_budget(
                crate::Crs::new("http://example.org/operation").unwrap(),
                crate::Crs::new("http://example.org/source").unwrap(),
                crate::Crs::new("http://example.org/target").unwrap(),
                OperationChain::compile(vec![retained]).unwrap(),
                &mut PreparationBudget::new(
                    crate::ExecutionPolicy::new(crate::ExecutionLimits {
                        max_work_items: u64::MAX,
                        ..crate::ExecutionLimits::GEOMETRY
                    })
                    .unwrap(),
                ),
            )
            .unwrap();
        let policy = crate::ExecutionPolicy::new(crate::ExecutionLimits {
            max_work_items: u64::MAX,
            max_workspace_bytes: 64 * 1024,
            ..crate::ExecutionLimits::GEOMETRY
        })
        .unwrap();
        assert!(matches!(
            profile.query_identity_in_budget(&mut PreparationBudget::new(policy)),
            Err(GeoError::MemoryExhausted { limit: 65_536 })
        ));
    }
}
