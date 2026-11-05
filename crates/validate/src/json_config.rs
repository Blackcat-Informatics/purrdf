// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One bounded JSON/record decoder for closed native configuration boundaries.

use purrdf_core::RdfDiagnostic;
use purrdf_lex::allocation::{Ceiling, Memory, Resident, StorageError, retain_native_cause};
use purrdf_lex::json::record::{
    DecodeError, FromJsonNative, Record, RecordAllocation, RecordMemory, Within,
};
use purrdf_lex::json::{self, Value};
use std::cell::RefCell;

pub(crate) fn failure(
    code: &str,
    error: impl purrdf_hash::error::NativeError + 'static,
    storage: Option<StorageError>,
) -> RdfDiagnostic {
    let result = || {
        let mut resident = Resident;
        let diagnostic = match storage {
            Some(storage) => RdfDiagnostic::from(storage),
            None => {
                RdfDiagnostic::try_error_with_memory(code, &error, &mut Memory::new(&mut resident))?
            }
        };
        let retained = diagnostic
            .retained_bytes()
            .ok_or(StorageError::SizeOverflow)?;
        let mut memory = Memory::resume(&mut resident, retained);
        diagnostic.with_native_cause(retain_native_cause(error)?, &mut memory)
    };
    result().unwrap_or_else(RdfDiagnostic::from)
}

pub(crate) fn field<T: FromJsonNative>(
    record: &mut Record<'_>,
    name: &'static str,
    default: T,
    allocation: &dyn RecordAllocation,
) -> Result<T, DecodeError> {
    match record.raw(name)? {
        None => Ok(default),
        Some(value) => T::from_json_native(value, Some(allocation))
            .map_err(|error| error.within_native(name, Some(allocation))),
    }
}

pub(crate) fn portable(
    record: &mut Record<'_>,
    name: &'static str,
    default: usize,
    allocation: &dyn RecordAllocation,
) -> Result<usize, DecodeError> {
    let default = u32::try_from(default).map_err(|_| {
        DecodeError::custom_native(
            "native default exceeds portable input-options range",
            Some(allocation),
        )
    })?;
    Ok(field(record, name, default, allocation)? as usize)
}

pub(crate) fn decode<T>(
    bytes: &[u8],
    code: &str,
    maximum_bytes: usize,
    maximum_allocation: usize,
    limits: json::Limits,
    oversized: &'static str,
    decode: impl FnOnce(
        &Value,
        &dyn RecordAllocation,
        &RefCell<Memory<'_, Ceiling>>,
    ) -> Result<T, DecodeError>,
) -> Result<T, RdfDiagnostic> {
    if bytes.len() > maximum_bytes {
        return Err(failure(code, DecodeError::custom(oversized), None));
    }
    let json_failure = |error: json::Error| {
        let storage = match error.kind() {
            json::ErrorKind::Storage(cause)
            | json::ErrorKind::Escape(purrdf_lex::json_escape::JsonEscapeErrorKind::Storage(
                cause,
            )) => Some(cause),
            _ => None,
        };
        failure(code, error, storage)
    };
    let mut admission = Ceiling(maximum_allocation);
    let memory = RefCell::new(Memory::new(&mut admission));
    let mut reader = json::Reader::from_slice(bytes, limits).map_err(json_failure)?;
    let value = reader
        .read_value_with_memory(&mut memory.borrow_mut())
        .map_err(json_failure)?;
    reader
        .finish_with_memory(&mut memory.borrow_mut())
        .map_err(json_failure)?;
    decode(&value, &RecordMemory::new(&memory), &memory).map_err(|error| {
        let storage = error.storage_error();
        failure(code, error, storage)
    })
}
