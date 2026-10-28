// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Opaque geographic sessions over the one strict Rust request/response boundary.

use std::os::raw::c_char;

use crate::{
    buffer::PurrdfBuffer,
    cstr_to_str,
    error::PurrdfError,
    handles::{free_handle, into_handle},
    opt_cstr_to_str,
    status::PurrdfStatus,
};

/// Immutable geographic profile/session. Release with `purrdf_geo_session_free`.
#[derive(Debug)]
pub struct PurrdfGeoSession(pub(crate) purrdf_validate::geo::GeoSession);

/// Prepare a strict version-one profile, or the standard profile when null.
///
/// # Safety
/// `profile` must be null or a live NUL-terminated UTF-8 string. `out_session`
/// and a nonnull `out_error` must be writable. The caller owns the returned handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_geo_session_create(
    profile: *const c_char,
    out_session: *mut *mut PurrdfGeoSession,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if out_session.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "out_session is null",
                ));
            }
            *out_session = std::ptr::null_mut();
            let session = opt_cstr_to_str(profile)?
                .map(purrdf_validate::geo::GeoSession::from_profile_str)
                .transpose()
                .map_err(|error| PurrdfError::new(PurrdfStatus::GeoError, error.to_string()))?
                .unwrap_or_default();
            *out_session = into_handle(PurrdfGeoSession(session));
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Execute a strict version-one request and return the engine's canonical response.
///
/// A semantic refusal returns its complete response `error` record with status
/// `GEO_ERROR`. An operational refusal returns that status and the original
/// error text while leaving `out_response` null. Free every nonnull response
/// and error handle. No partial result is emitted.
///
/// # Safety
/// `session` must be a live handle, `request` a live NUL-terminated UTF-8 string,
/// and output pointers writable. Handles may be read concurrently; free requires
/// exclusive ownership after every call has finished.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_geo_session_call(
    session: *const PurrdfGeoSession,
    request: *const c_char,
    out_response: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        response_call(
            session,
            request,
            out_response,
            out_error,
            "null geographic session call argument",
            |handle, text| handle.0.try_response(text),
        )
    }
}

/// Pointer validation, unwind boundary, owned response and typed refusal status
/// have one implementation for every opaque geographic response producer.
unsafe fn response_call<T>(
    handle: *const T,
    request: *const c_char,
    out_response: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
    null_message: &'static str,
    response: impl FnOnce(&T, &str) -> Result<(String, bool), purrdf_validate::geo::GeoCallError>,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if handle.is_null() || request.is_null() || out_response.is_null() {
                return Err(PurrdfError::new(PurrdfStatus::NullPointer, null_message));
            }
            *out_response = std::ptr::null_mut();
            let (text, success) = response(&*handle, cstr_to_str(request)?)
                .map_err(|error| PurrdfError::new(PurrdfStatus::GeoError, error.to_string()))?;
            *out_response = into_handle(PurrdfBuffer(text.into_bytes()));
            if !success {
                return Err(PurrdfError::new(
                    PurrdfStatus::GeoError,
                    "geographic operation refused; inspect the typed response error record",
                ));
            }
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Release a geographic session; null is a no-op.
///
/// # Safety
/// `session` must be null or an exclusively owned live handle not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_geo_session_free(session: *mut PurrdfGeoSession) {
    unsafe { free_handle(session) }
}

/// Borrow a live session's profile; a null optional query context means standard CRS84.
///
/// # Safety
/// A nonnull handle must stay live and immutable for the returned borrow's lifetime.
pub(crate) unsafe fn profile<'a>(
    session: *const PurrdfGeoSession,
) -> &'a purrdf_validate::geo::GeoProfile {
    if session.is_null() {
        &purrdf_validate::geo::STANDARD_PROFILE
    } else {
        unsafe { (*session).0.profile() }
    }
}

/// Immutable reusable point-cell index. Release with `purrdf_geo_point_index_free`.
#[derive(Debug)]
pub struct PurrdfGeoPointIndex(purrdf_validate::geo::GeoPointIndex);

/// Prepare complete ordered buckets using the session's explicit reference/profile.
///
/// # Safety
/// `session` must be a live immutable handle and `request` a live UTF-8 C string.
/// `out_index` and nonnull `out_error` must be writable. The output handle is owned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_geo_point_index_create(
    session: *const PurrdfGeoSession,
    request: *const c_char,
    out_index: *mut *mut PurrdfGeoPointIndex,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if session.is_null() || request.is_null() || out_index.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null point index preparation argument",
                ));
            }
            *out_index = std::ptr::null_mut();
            let index = (*session)
                .0
                .point_index(cstr_to_str(request)?)
                .map_err(|error| PurrdfError::new(PurrdfStatus::GeoError, error.to_string()))?;
            *out_index = into_handle(PurrdfGeoPointIndex(index));
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Execute a strict physical/reported search and return its canonical response.
/// Semantic refusal returns its complete typed response through `out_response`;
/// operational refusal leaves that pointer null and preserves the error cause.
///
/// # Safety
/// `index` must be a live immutable handle, `request` a live UTF-8 C string and
/// output pointers writable. Read calls may run concurrently; free is exclusive.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_geo_point_index_call(
    index: *const PurrdfGeoPointIndex,
    request: *const c_char,
    out_response: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    unsafe {
        response_call(
            index,
            request,
            out_response,
            out_error,
            "null point index search argument",
            |handle, text| handle.0.try_response(text),
        )
    }
}

/// Free a point index; null is a no-op.
///
/// # Safety
/// The handle must be null or exclusively owned, live and not previously freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_geo_point_index_free(index: *mut PurrdfGeoPointIndex) {
    unsafe { free_handle(index) }
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};

    use purrdf_lex::json;
    use purrdf_validate::geo::GeoSession;

    use super::*;
    use crate::buffer::{purrdf_buffer_data, purrdf_buffer_free};
    use crate::error::{purrdf_error_code, purrdf_error_free, purrdf_error_message};

    const ORIGIN_DISTANCE: &str = r#"{"version":1,"operation":"distance","a":{"longitude":"0","latitude":"0"},"b":{"longitude":"0","latitude":"0"}}"#;
    const INDEX: &str = r#"{"version":1,"operation":"point-index","grid":"wgs84","level":2,"points":[{"key":"7","point":{"longitude":"0","latitude":"0"}},{"key":"11","point":{"longitude":"1","latitude":"0"}}]}"#;
    const SEARCH: &str = r#"{"version":1,"operation":"search-physical","center":{"longitude":"0","latitude":"0"},"radius_metres":"0"}"#;

    /// Consume the actual C buffer through its public borrowed-byte accessor.
    unsafe fn take_response(buffer: *mut PurrdfBuffer) -> String {
        let mut bytes = std::ptr::null();
        let mut len = 0;
        unsafe {
            assert!(!buffer.is_null());
            assert_eq!(
                purrdf_buffer_data(buffer, &raw mut bytes, &raw mut len),
                PurrdfStatus::Ok as i32
            );
            let text = std::str::from_utf8(std::slice::from_raw_parts(bytes, len))
                .expect("native UTF-8 response")
                .to_owned();
            purrdf_buffer_free(buffer);
            text
        }
    }

    #[test]
    fn session_abi_preserves_canonical_success_and_complete_semantic_refusal() {
        let native = GeoSession::default();
        let mut session = std::ptr::null_mut();
        let mut error = std::ptr::null_mut();
        unsafe {
            assert_eq!(
                purrdf_geo_session_create(std::ptr::null(), &raw mut session, &raw mut error),
                PurrdfStatus::Ok as i32
            );
            assert!(error.is_null());
            for source in [
                ORIGIN_DISTANCE,
                r#"{"version":1,"operation":"not-an-operation"}"#,
            ] {
                let request = CString::new(source).unwrap();
                let (expected, complete) = native.try_response(source).unwrap();
                let mut response = std::ptr::null_mut();
                let status = purrdf_geo_session_call(
                    session,
                    request.as_ptr(),
                    &raw mut response,
                    &raw mut error,
                );
                assert_eq!(
                    status,
                    if complete {
                        PurrdfStatus::Ok
                    } else {
                        PurrdfStatus::GeoError
                    } as i32
                );
                assert_eq!(take_response(response), expected);
                if complete {
                    assert!(error.is_null());
                } else {
                    assert_eq!(purrdf_error_code(error), PurrdfStatus::GeoError as i32);
                    purrdf_error_free(error);
                    error = std::ptr::null_mut();
                }
            }
            purrdf_geo_session_free(session);
        }
    }

    #[test]
    fn operational_refusal_has_original_cause_and_no_response_allocation() {
        let profile = r#"{"version":1,"limits":{"max_work_items":1}}"#;
        let native = GeoSession::from_profile_str(profile).unwrap();
        let expected = native
            .try_response(ORIGIN_DISTANCE)
            .unwrap_err()
            .to_string();
        let profile = CString::new(profile).unwrap();
        let request = CString::new(ORIGIN_DISTANCE).unwrap();
        let mut session = std::ptr::null_mut();
        let mut error = std::ptr::null_mut();
        let mut response = std::ptr::null_mut();
        unsafe {
            assert_eq!(
                purrdf_geo_session_create(profile.as_ptr(), &raw mut session, &raw mut error),
                PurrdfStatus::Ok as i32
            );
            assert_eq!(
                purrdf_geo_session_call(
                    session,
                    request.as_ptr(),
                    &raw mut response,
                    &raw mut error,
                ),
                PurrdfStatus::GeoError as i32
            );
            assert!(
                response.is_null(),
                "a refused account publishes no partial envelope"
            );
            assert_eq!(purrdf_error_code(error), PurrdfStatus::GeoError as i32);
            assert_eq!(
                CStr::from_ptr(purrdf_error_message(error))
                    .to_str()
                    .unwrap(),
                expected
            );
            purrdf_error_free(error);
            purrdf_geo_session_free(session);
        }
    }

    #[test]
    fn index_abi_retains_profile_and_original_buckets_after_session_is_freed() {
        let native = GeoSession::default();
        let expected = native
            .point_index(INDEX)
            .unwrap()
            .try_response(SEARCH)
            .unwrap()
            .0;
        let request = CString::new(INDEX).unwrap();
        let search = CString::new(SEARCH).unwrap();
        let mut session = std::ptr::null_mut();
        let mut index = std::ptr::null_mut();
        let mut error = std::ptr::null_mut();
        let mut response = std::ptr::null_mut();
        unsafe {
            assert_eq!(
                purrdf_geo_session_create(std::ptr::null(), &raw mut session, &raw mut error),
                PurrdfStatus::Ok as i32
            );
            assert_eq!(
                purrdf_geo_point_index_create(
                    session,
                    request.as_ptr(),
                    &raw mut index,
                    &raw mut error
                ),
                PurrdfStatus::Ok as i32
            );
            purrdf_geo_session_free(session);
            assert_eq!(
                purrdf_geo_point_index_call(
                    index,
                    search.as_ptr(),
                    &raw mut response,
                    &raw mut error
                ),
                PurrdfStatus::Ok as i32
            );
            assert!(error.is_null());
            let text = take_response(response);
            assert_eq!(text, expected);
            let parsed = json::read(&text).unwrap();
            assert_eq!(
                parsed["result"]["keys"],
                json::Value::Array(vec![json::Value::from("7")])
            );
            purrdf_geo_point_index_free(index);
        }
    }

    #[test]
    fn explicit_session_axes_reach_the_actual_query_json_entry() {
        use purrdf_core::RdfDatasetBuilder;
        use purrdf_core::vocab::ogc;

        let dataset =
            crate::handles::PurrdfDataset::new(RdfDatasetBuilder::new().freeze().unwrap());
        let query = CString::new(format!(
            "ASK {{ FILTER(<{}>(\"<{}> POINT(0 1)\"^^<{}>,\"POINT(1 0)\"^^<{}>,<{}>) = 0) }}",
            ogc::geof::DISTANCE,
            ogc::EPSG4326,
            ogc::geo::WKT_LITERAL,
            ogc::geo::WKT_LITERAL,
            ogc::uom::METRE,
        ))
        .unwrap();
        let profile = CString::new(format!(
            r#"{{"version":1,"references":[{{"crs":"{}","ellipsoid":"wgs84","axes":"lat-lon"}}]}}"#,
            ogc::EPSG4326,
        ))
        .unwrap();
        let mut session = std::ptr::null_mut();
        let mut response = std::ptr::null_mut();
        let mut error = std::ptr::null_mut();
        unsafe {
            let query_call = |session, response, error| {
                crate::query::purrdf_query_json_with_geo(
                    session,
                    &raw const dataset,
                    query.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    &mut crate::receipt::Output::default().0,
                    response,
                    error,
                )
            };
            assert_ne!(
                query_call(std::ptr::null(), &raw mut response, &raw mut error),
                0
            );
            assert!(response.is_null());
            assert!(
                CStr::from_ptr(purrdf_error_message(error))
                    .to_str()
                    .unwrap()
                    .contains("unregistered")
            );
            purrdf_error_free(error);
            error = std::ptr::null_mut();
            assert_eq!(
                purrdf_geo_session_create(profile.as_ptr(), &raw mut session, &raw mut error),
                PurrdfStatus::Ok as i32
            );
            assert_eq!(query_call(session, &raw mut response, &raw mut error), 0);
            assert!(error.is_null());
            let text = take_response(response);
            assert_eq!(
                json::read(&text).unwrap()["boolean"],
                json::Value::from(true)
            );
            purrdf_geo_session_free(session);
        }
    }
}
