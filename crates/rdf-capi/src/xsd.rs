// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The XSD value space at the C boundary: the canonical lexical form of a typed value
//! and the value-space comparison of two typed values, both exact for integers and
//! decimals of any length.
//!
//! C has no integer wider than its widest built-in type and no exact decimal, so a host
//! that normalizes or orders numeric literals by converting them loses digits. These
//! entry points keep the value in the engine and hand back text or an ordering.

use std::cmp::Ordering;
use std::os::raw::c_char;

use purrdf_rs::xsd::{XsdValue, parse_by_iri, value_cmp};

use crate::buffer::PurrdfBuffer;
use crate::cstr_to_str;
use crate::error::PurrdfError;
use crate::handles::into_handle;
use crate::status::PurrdfStatus;

/// The value of `lexical` read as the datatype IRI `datatype`: `InvalidArgument` when
/// the IRI is not an XSD datatype the engine maps, `ParseError` when `lexical` is not
/// in its lexical space.
fn typed_value(lexical: &str, datatype: &str) -> Result<XsdValue, PurrdfError> {
    match parse_by_iri(lexical, datatype) {
        Ok(Some(value)) => Ok(value),
        Ok(None) => Err(PurrdfError::new(
            PurrdfStatus::InvalidArgument,
            format!("<{datatype}> is not an XSD datatype the engine maps"),
        )),
        Err(error) => Err(PurrdfError::new(
            PurrdfStatus::ParseError,
            error.to_string(),
        )),
    }
}

/// Write the XSD canonical lexical form of `lexical`, read as the datatype IRI
/// `datatype`, to `*out_buffer` as UTF-8 with no terminating NUL (`"+007"` as
/// `xsd:integer` is `7`, `"1.50"` as `xsd:decimal` is `1.5`). Integers and decimals are
/// exact at any length.
///
/// Returns `PURRDF_STATUS_INVALID_ARGUMENT` when `datatype` is not an XSD datatype the
/// engine maps, and `PURRDF_STATUS_PARSE_ERROR` when `lexical` is not in its lexical
/// space; `*out_buffer` is left untouched on any error. Release the buffer with
/// `purrdf_buffer_free`.
///
/// # Safety
/// `lexical` and `datatype` must be NUL-terminated C strings; `out_buffer` must be
/// writable; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_xsd_canonical_lexical(
    lexical: *const c_char,
    datatype: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if lexical.is_null() || datatype.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_xsd_canonical_lexical",
                ));
            }
            let value = typed_value(cstr_to_str(lexical)?, cstr_to_str(datatype)?)?;
            *out_buffer = into_handle(PurrdfBuffer(value.canonical_lexical().into_bytes()));
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Compare two typed values in the XSD value space. On success `*out_comparable` is `1`
/// and `*out_order` is `-1` when the left value is the smaller, `0` when they are equal
/// and `1` when it is the larger; or `*out_comparable` is `0` and `*out_order` is `0`
/// when the two values are incomparable (a `NaN`, or two value-space families such as a
/// number and a string). Numeric datatypes compare across each other, exactly at any
/// length.
///
/// Returns `PURRDF_STATUS_INVALID_ARGUMENT` when either datatype is not an XSD datatype
/// the engine maps, and `PURRDF_STATUS_PARSE_ERROR` when either lexical form is not in
/// its datatype's lexical space; the out-params are left untouched on any error.
///
/// # Safety
/// The four string arguments must be NUL-terminated C strings; `out_comparable` and
/// `out_order` must be writable; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_xsd_value_compare(
    left_lexical: *const c_char,
    left_datatype: *const c_char,
    right_lexical: *const c_char,
    right_datatype: *const c_char,
    out_comparable: *mut u8,
    out_order: *mut i32,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if left_lexical.is_null()
                || left_datatype.is_null()
                || right_lexical.is_null()
                || right_datatype.is_null()
                || out_comparable.is_null()
                || out_order.is_null()
            {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_xsd_value_compare",
                ));
            }
            let left = typed_value(cstr_to_str(left_lexical)?, cstr_to_str(left_datatype)?)?;
            let right = typed_value(cstr_to_str(right_lexical)?, cstr_to_str(right_datatype)?)?;
            let ordering = value_cmp(&left, &right);
            *out_comparable = u8::from(ordering.is_some());
            *out_order = match ordering {
                Some(Ordering::Less) => -1,
                Some(Ordering::Greater) => 1,
                Some(Ordering::Equal) | None => 0,
            };
            Ok(PurrdfStatus::Ok)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use purrdf_rs::xsd::datatype::{XSD_DECIMAL, XSD_DOUBLE, XSD_INTEGER, XSD_STRING};

    use super::*;
    use crate::buffer::purrdf_buffer_free;
    use crate::error::purrdf_error_free;

    /// `i128::MAX + 1`, the first integer past the bounded representation.
    const PAST_I128: &str = "170141183460469231731687303715884105728";
    /// A sixty-digit integer.
    const SIXTY: &str = "123456789012345678901234567890123456789012345678901234567890";
    /// [`SIXTY`] plus one.
    const SIXTY_NEXT: &str = "123456789012345678901234567890123456789012345678901234567891";

    fn c(text: &str) -> CString {
        CString::new(text).expect("no interior NUL")
    }

    /// The canonical lexical form, or the status of the refusal.
    fn canonical(lexical: &str, datatype: &str) -> Result<String, PurrdfStatus> {
        let (lexical, datatype) = (c(lexical), c(datatype));
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = unsafe {
            purrdf_xsd_canonical_lexical(
                lexical.as_ptr(),
                datatype.as_ptr(),
                &raw mut buffer,
                &raw mut error,
            )
        };
        if status == PurrdfStatus::Ok as i32 {
            assert!(error.is_null());
            let text = unsafe { String::from_utf8((*buffer).0.clone()).expect("UTF-8") };
            unsafe { purrdf_buffer_free(buffer) };
            Ok(text)
        } else {
            assert!(buffer.is_null());
            let code = unsafe { (*error).code };
            assert_eq!(code as i32, status);
            unsafe { purrdf_error_free(error) };
            Err(code)
        }
    }

    /// The `(comparable, order)` pair, or the status of the refusal.
    fn compare(left: (&str, &str), right: (&str, &str)) -> Result<(u8, i32), PurrdfStatus> {
        let strings = [c(left.0), c(left.1), c(right.0), c(right.1)];
        let mut comparable: u8 = 9;
        let mut order: i32 = 9;
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        let status = unsafe {
            purrdf_xsd_value_compare(
                strings[0].as_ptr(),
                strings[1].as_ptr(),
                strings[2].as_ptr(),
                strings[3].as_ptr(),
                &raw mut comparable,
                &raw mut order,
                &raw mut error,
            )
        };
        if status == PurrdfStatus::Ok as i32 {
            assert!(error.is_null());
            Ok((comparable, order))
        } else {
            assert_eq!((comparable, order), (9, 9), "out-params untouched on error");
            let code = unsafe { (*error).code };
            assert_eq!(code as i32, status);
            unsafe { purrdf_error_free(error) };
            Err(code)
        }
    }

    #[test]
    fn integers_past_i128_are_canonical_and_exact() {
        assert_eq!(canonical(PAST_I128, XSD_INTEGER).as_deref(), Ok(PAST_I128));
        assert_eq!(canonical(SIXTY, XSD_INTEGER).as_deref(), Ok(SIXTY));
        assert_eq!(
            canonical(&format!("+000{SIXTY}"), XSD_INTEGER).as_deref(),
            Ok(SIXTY)
        );
    }

    #[test]
    fn long_decimals_are_canonical_and_exact() {
        let forty = "0.1000000000000000000000000000000000000001";
        assert_eq!(canonical(forty, XSD_DECIMAL).as_deref(), Ok(forty));
        assert_eq!(canonical("1.50", XSD_DECIMAL).as_deref(), Ok("1.5"));
    }

    #[test]
    fn a_malformed_or_unmapped_lexical_is_refused_beside_its_valid_neighbour() {
        assert_eq!(canonical("12x", XSD_INTEGER), Err(PurrdfStatus::ParseError));
        assert_eq!(canonical("12", XSD_INTEGER).as_deref(), Ok("12"));
        assert_eq!(
            canonical("12", "http://example.org/datatype"),
            Err(PurrdfStatus::InvalidArgument)
        );
    }

    #[test]
    fn long_integers_compare_exactly() {
        assert_eq!(
            compare((SIXTY, XSD_INTEGER), (SIXTY_NEXT, XSD_INTEGER)),
            Ok((1, -1))
        );
        assert_eq!(
            compare((SIXTY_NEXT, XSD_INTEGER), (SIXTY, XSD_INTEGER)),
            Ok((1, 1))
        );
        assert_eq!(
            compare((SIXTY, XSD_INTEGER), (&format!("0{SIXTY}"), XSD_INTEGER)),
            Ok((1, 0))
        );
        let plus_one = format!("1{}1", "0".repeat(41));
        let plus_two = format!("1{}2", "0".repeat(41));
        assert_eq!(
            compare((&plus_one, XSD_INTEGER), (&plus_two, XSD_INTEGER)),
            Ok((1, -1))
        );
        assert_eq!(
            compare(
                (&plus_one, XSD_INTEGER),
                (&format!("{plus_one}.0"), XSD_DECIMAL)
            ),
            Ok((1, 0))
        );
    }

    #[test]
    fn malformed_and_incomparable_values_beside_their_valid_neighbours() {
        assert_eq!(
            compare(("12x", XSD_INTEGER), ("12", XSD_INTEGER)),
            Err(PurrdfStatus::ParseError)
        );
        assert_eq!(
            compare(("12", XSD_INTEGER), ("12", XSD_INTEGER)),
            Ok((1, 0))
        );
        assert_eq!(
            compare(("12", XSD_INTEGER), ("12", "http://example.org/datatype")),
            Err(PurrdfStatus::InvalidArgument)
        );
        assert_eq!(compare(("NaN", XSD_DOUBLE), ("1", XSD_DOUBLE)), Ok((0, 0)));
        assert_eq!(compare(("1", XSD_INTEGER), ("1", XSD_STRING)), Ok((0, 0)));
        assert_eq!(compare(("1", XSD_INTEGER), ("1", XSD_DOUBLE)), Ok((1, 0)));
    }

    #[test]
    fn null_arguments_are_refused() {
        let integer = c(XSD_INTEGER);
        let (one, two) = (c("1"), c("2"));
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut comparable: u8 = 0;
        let mut order: i32 = 0;
        unsafe {
            assert_eq!(
                purrdf_xsd_canonical_lexical(
                    std::ptr::null(),
                    integer.as_ptr(),
                    &raw mut buffer,
                    std::ptr::null_mut(),
                ),
                PurrdfStatus::NullPointer as i32
            );
            assert_eq!(
                purrdf_xsd_value_compare(
                    integer.as_ptr(),
                    integer.as_ptr(),
                    integer.as_ptr(),
                    integer.as_ptr(),
                    std::ptr::null_mut(),
                    &raw mut order,
                    std::ptr::null_mut(),
                ),
                PurrdfStatus::NullPointer as i32
            );
            assert_eq!(
                purrdf_xsd_value_compare(
                    one.as_ptr(),
                    integer.as_ptr(),
                    two.as_ptr(),
                    integer.as_ptr(),
                    &raw mut comparable,
                    &raw mut order,
                    std::ptr::null_mut(),
                ),
                PurrdfStatus::Ok as i32
            );
        }
        assert!(buffer.is_null());
        assert_eq!((comparable, order), (1, -1));
    }
}
