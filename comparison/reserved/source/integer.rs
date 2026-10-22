// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Certified numerical faces over the workspace's existing exact owners.

pub(crate) use crate::bigint::scratch;
pub use crate::bigint::{BigInt as Int, LimbScratch, LimbScratchError};
pub use crate::exact::{
    DecimalConstructionError, ExactArithmeticCost, ExactOperation, GeometricRational as Rat,
    RationalComparisonBody, RationalComparisonError, RationalNativeComparisonBody,
};
