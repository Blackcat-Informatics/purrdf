// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact, validated longitude and latitude in decimal degrees.
//!
//! Validation precedes every numerical conversion. In particular, a rational
//! just outside a pole is refused even when binary64 conversion rounds it to
//! the pole. Construction preserves the supplied rational coordinates.

use crate::{GeoError, Rat};

/// Reduce any exact degree lift into `[origin,origin+360)` using integer
/// remainder. No floating approximation or bounded input range is required.
pub(crate) fn normalize_degrees(angle: &Rat, origin: i64) -> Rat {
    angle.modulo_integer(origin, 360).expect("positive period")
}

/// Exact signed shortest longitude difference in `(-180,180]`; both inputs
/// have already been validated in the closed source-longitude range.
pub(crate) fn longitude_difference(first: &Rat, second: &Rat) -> Rat {
    let mut difference = second.sub(first);
    if difference > Rat::from_i64(180) {
        difference = difference.sub(&Rat::from_i64(360));
    }
    if difference <= Rat::from_i64(-180) {
        difference = difference.add(&Rat::from_i64(360));
    }
    difference
}

/// Longitude and latitude, in that order, with their original exact values.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LonLat {
    longitude: Rat,
    latitude: Rat,
}

impl LonLat {
    /// Validate longitude in `[-180, 180]` and latitude in `[-90, 90]`.
    ///
    /// # Errors
    ///
    /// Returns [`GeoError::CoordinateOutOfRange`] for an original coordinate
    /// outside its closed range. Values are never wrapped or clamped.
    pub fn new(longitude: Rat, latitude: Rat) -> Result<Self, GeoError> {
        Self::validate_original(&longitude, &latitude)?;
        Ok(Self {
            longitude,
            latitude,
        })
    }

    /// Validate borrowed original coordinates through the same exact range law.
    pub(crate) fn validate_original(longitude: &Rat, latitude: &Rat) -> Result<(), GeoError> {
        validate_coordinate("longitude", longitude, 180)?;
        validate_coordinate("latitude", latitude, 90)
    }

    /// Decode finite binary64 coordinates as exact dyadic rationals, then
    /// validate those rationals. This performs no floating-point arithmetic.
    ///
    /// # Errors
    ///
    /// Refuses NaN, infinity, and out-of-range coordinates with typed errors.
    pub fn from_f64(longitude: f64, latitude: f64) -> Result<Self, GeoError> {
        Self::new(
            coordinate_from_f64("longitude", longitude)?,
            coordinate_from_f64("latitude", latitude)?,
        )
    }

    /// The supplied longitude, without normalization.
    #[must_use]
    pub const fn longitude(&self) -> &Rat {
        &self.longitude
    }

    /// The supplied latitude, without normalization.
    #[must_use]
    pub const fn latitude(&self) -> &Rat {
        &self.latitude
    }

    /// Whether the original latitude is exactly one of the two poles.
    #[must_use]
    pub fn is_pole(&self) -> bool {
        self.latitude == Rat::from_i64(90) || self.latitude == Rat::from_i64(-90)
    }

    /// Whether two coordinates denote the same point on a geographic surface.
    ///
    /// Longitude is immaterial at an exact pole; the two antimeridian
    /// spellings denote the same meridian. The original values remain intact.
    #[must_use]
    pub fn same_location(&self, other: &Self) -> bool {
        self.latitude == other.latitude
            && (self.is_pole() || same_longitude(&self.longitude, &other.longitude))
    }
}

/// Exact longitude-cut aliases compare their canonical signed rational values.
/// Equality needs no products, reductions or clones of original coordinate limbs.
pub(crate) fn same_longitude(a: &Rat, b: &Rat) -> bool {
    a == b
        || ((a == &Rat::from_i64(180) || a == &Rat::from_i64(-180))
            && (b == &Rat::from_i64(180) || b == &Rat::from_i64(-180)))
}

fn validate_coordinate(axis: &'static str, value: &Rat, bound: i64) -> Result<(), GeoError> {
    if value < &Rat::from_i64(-bound) || value > &Rat::from_i64(bound) {
        return Err(GeoError::CoordinateOutOfRange {
            axis,
            value: value.clone(),
        });
    }
    Ok(())
}

/// The exact rational value of a finite binary64 coordinate.
fn coordinate_from_f64(axis: &'static str, value: f64) -> Result<Rat, GeoError> {
    Rat::from_binary64(value).ok_or_else(|| GeoError::NonFiniteCoordinate {
        axis,
        bits: value.to_bits(),
    })
}

#[cfg(test)]
mod tests {
    use super::LonLat;
    use crate::{GeoError, Int, Rat};

    fn decimal(value: &str) -> Rat {
        Rat::parse_decimal(value).expect("a test decimal")
    }

    #[test]
    fn original_range_is_checked_before_binary64_rounding() {
        for (axis, longitude, latitude) in [
            ("longitude", "180.000000000000000000000000000001", "0"),
            ("longitude", "-180.000000000000000000000000000001", "0"),
            ("latitude", "0", "90.000000000000000000000000000001"),
            ("latitude", "0", "-90.000000000000000000000000000001"),
        ] {
            assert!(matches!(
                LonLat::new(decimal(longitude), decimal(latitude)),
                Err(GeoError::CoordinateOutOfRange { axis: actual, .. }) if actual == axis
            ));
        }
        let near_pole = LonLat::new(decimal("180"), decimal("89.999999999999999999999999999999"))
            .expect("inside the original range");
        assert_eq!(near_pole.latitude().to_f64(), 90.0);
        assert!(!near_pole.is_pole());
    }

    #[test]
    fn carrier_coordinates_are_preserved_at_seams_and_poles() {
        let east = LonLat::new(decimal("180"), decimal("0")).expect("valid");
        let west = LonLat::new(decimal("-180"), decimal("0")).expect("valid");
        assert!(east.same_location(&west));
        assert_eq!(east.longitude(), &decimal("180"));
        assert_eq!(west.longitude(), &decimal("-180"));
        let north = LonLat::new(decimal("37"), decimal("90")).expect("valid");
        let north_other = LonLat::new(decimal("-123"), decimal("90")).expect("valid");
        assert!(north.same_location(&north_other));
        assert_eq!(north.longitude(), &decimal("37"));
        let south = LonLat::new(decimal("37"), decimal("-90")).expect("valid");
        assert!(!north.same_location(&south));
    }

    #[test]
    fn lifted_degrees_reduce_by_exact_remainder_at_the_half_open_cut() {
        for (source, origin, expected) in [
            ("540", -180, "-180"),
            ("-900", -180, "-180"),
            ("1080.125", -180, "0.125"),
            ("-1080.125", 0, "359.875"),
            (
                "180.000000000000000000000000000001",
                -180,
                "-179.999999999999999999999999999999",
            ),
        ] {
            assert_eq!(
                super::normalize_degrees(&decimal(source), origin),
                decimal(expected)
            );
        }
    }

    #[test]
    fn ieee_input_is_decoded_without_a_decimal_intermediate() {
        let point = LonLat::from_f64(0.1, f64::from_bits(1)).expect("finite in range");
        assert_eq!(
            point.longitude(),
            &Rat::new(Int::from_u64(3_602_879_701_896_397), Int::one().shl(55))
                .expect("positive denominator")
        );
        assert_eq!(
            point.latitude(),
            &Rat::new(Int::one(), Int::one().shl(1074)).expect("positive denominator")
        );
        assert_eq!(
            LonLat::from_f64(-0.0, 0.0).expect("zero is finite"),
            LonLat::new(Rat::zero(), Rat::zero()).expect("zero is in range")
        );
    }

    #[test]
    fn non_finite_ieee_values_have_typed_refusals() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(matches!(
                LonLat::from_f64(value, 0.0),
                Err(GeoError::NonFiniteCoordinate {
                    axis: "longitude",
                    ..
                })
            ));
            assert!(matches!(
                LonLat::from_f64(0.0, value),
                Err(GeoError::NonFiniteCoordinate {
                    axis: "latitude",
                    ..
                })
            ));
        }
    }
}
