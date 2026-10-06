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
/// A numerical/reference refusal is returned in the response's `error` record
/// with status `GEO_ERROR`. `out_response` still owns that complete response;
/// both response and optional error handle must be freed. No partial result is emitted.
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
            |handle, text| handle.0.response(text),
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
    response: impl FnOnce(&T, &str) -> (String, bool),
) -> i32 {
    unsafe {
        ffi_try!(out_error, {
            if handle.is_null() || request.is_null() || out_response.is_null() {
                return Err(PurrdfError::new(PurrdfStatus::NullPointer, null_message));
            }
            *out_response = std::ptr::null_mut();
            let (text, success) = response(&*handle, cstr_to_str(request)?);
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
/// Refusal still returns the complete typed response through `out_response`.
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
            |handle, text| handle.0.response(text),
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
