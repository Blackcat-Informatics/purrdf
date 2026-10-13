// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original strict-record construction, sharing the native carrier account.

use std::cell::RefCell;

use purrdf_geo_kernel::{
    ExecutionPolicy, GeoError, MetricWorkObserver, PreparationBudget, Rat,
    carrier::{CarrierStorage, InputAdmission},
};
use purrdf_lex::{
    allocation::{Admission, StorageError},
    json::{
        self, Value,
        record::{DecodeError, Record, RecordAllocation, Within},
    },
};

use super::GeoCallError;

struct Capacity {
    limit: u64,
    admitted: usize,
    failure: Option<GeoError>,
}
impl Admission for Capacity {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if self.failure.is_some() && bytes > self.admitted {
            return Err(StorageError::AdmissionFailed);
        }
        let count = u64::try_from(bytes).map_err(|_| StorageError::SizeOverflow)?;
        if count > self.limit {
            self.failure
                .get_or_insert(GeoError::MemoryExhausted { limit: self.limit });
            return Err(StorageError::AdmissionFailed);
        }
        self.admitted = bytes;
        Ok(())
    }
}

/// Stack borrows of the original lexical and exact-construction account.
/// Returned ordinary typed values transfer to their explicit Rust caller;
/// their arbitrary integers retain the original immutable arena owners.
pub(super) struct Decode<'d, 'observer, 'storage, 'account> {
    pub(super) storage: &'d CarrierStorage<'account>,
    allocation: &'d dyn RecordAllocation,
    admission: &'d RefCell<InputAdmission<'observer, 'storage, 'account>>,
    parsed: &'d [(&'d str, Rat)],
}
impl Decode<'_, '_, '_, '_> {
    pub(super) fn array<I: ExactSizeIterator>(
        &self,
        values: I,
        mut encode: impl FnMut(I::Item) -> Result<Value, GeoCallError>,
    ) -> Result<Value, GeoCallError> {
        let mut output = Vec::new();
        self.storage
            .native(|memory| memory.reserve(&mut output, values.len()))
            .map_err(GeoError::Storage)?;
        for value in values {
            output.push(encode(value)?);
        }
        Ok(Value::Array(output))
    }

    pub(super) fn owned_array<T>(
        &self,
        values: Vec<T>,
        encode: impl FnMut(T) -> Result<Value, GeoCallError>,
    ) -> Result<Value, GeoCallError> {
        let bytes = core::alloc::Layout::array::<T>(values.capacity())
            .map_err(|_| StorageError::SizeOverflow)?
            .size();
        let result = self.array(values.into_iter(), encode);
        // The owning input iterator and all unconsumed cells have died on
        // both outcomes; only the new JSON array survives under this account.
        let cleanup = self
            .storage
            .native(|memory| memory.release_bytes(bytes))
            .map_err(GeoError::Storage)
            .map_err(Into::into);
        match result {
            Err(original) => Err(original),
            Ok(value) => cleanup.map(|()| value),
        }
    }

    pub(super) fn format(&self, value: &impl std::fmt::Display) -> Result<String, GeoCallError> {
        self.storage
            .native(|memory| memory.format(value))
            .map_err(GeoError::Storage)
            .map_err(Into::into)
    }

    pub(super) fn hex(&self, bytes: &[u8]) -> Result<String, GeoCallError> {
        self.storage
            .native(|memory| {
                let required = bytes
                    .len()
                    .checked_mul(2)
                    .ok_or(StorageError::SizeOverflow)?;
                let mut text = String::new();
                memory.reserve_string(&mut text, required)?;
                purrdf_hash::hex::encode_into(bytes, &mut text);
                Ok::<_, StorageError>(text)
            })
            .map_err(GeoError::Storage)
            .map_err(Into::into)
    }

    pub(super) fn integer_text(
        &self,
        value: &purrdf_geo_kernel::Int,
    ) -> Result<String, GeoCallError> {
        self.admission
            .borrow_mut()
            .integer_text(value)
            .map_err(Into::into)
    }

    pub(super) fn certificate(
        &self,
        body: impl FnOnce(&mut InputAdmission<'_, '_, '_>) -> Result<Vec<u8>, GeoError>,
    ) -> Result<String, GeoCallError> {
        let bytes = body(&mut self.admission.borrow_mut())?;
        let result = self.hex(&bytes);
        let cleanup = self
            .storage
            .native(|memory| memory.release_vec(bytes))
            .map_err(GeoError::Storage)
            .map_err(Into::into);
        match result {
            Err(original) => Err(original),
            Ok(value) => cleanup.map(|()| value),
        }
    }

    pub(super) fn message(&self, error: &GeoCallError) -> Result<String, GeoCallError> {
        match error {
            GeoCallError::Engine(error) => self
                .storage
                .native(|memory| error.message_with_memory(memory))
                .map_err(Into::into),
            GeoCallError::Decode(error) => self.format(error),
        }
    }

    pub(super) fn decimal_text(
        &self,
        value: &Rat,
        scale: Option<u32>,
        rounding: purrdf_xsd::ieee::ratio::Rounding,
    ) -> Result<String, GeoCallError> {
        self.admission
            .borrow_mut()
            .decimal_text(value, scale, rounding)
            .map_err(Into::into)
    }

    pub(super) fn write(&self, value: &Value) -> Result<String, GeoCallError> {
        self.storage
            .native(|memory| json::write_with_memory(value, json::Format::COMPACT, memory))
            .map_err(GeoError::Storage)
            .map_err(Into::into)
    }

    pub(super) fn record<'a>(
        &'a self,
        value: &'a Value,
        expecting: &str,
    ) -> Result<Record<'a>, DecodeError> {
        Record::new_native(value, expecting, Some(self.allocation))
    }

    pub(super) fn decimal(&self, value: &Value) -> Result<Rat, GeoCallError> {
        let text = value
            .as_str()
            .ok_or_else(|| self.invalid_type(value, "exact decimal string"))?;
        if let Ok(index) = self
            .parsed
            .binary_search_by(|(lexical, _)| lexical.cmp(&text))
        {
            return Ok(self.parsed[index].1.clone());
        }
        self.admission.borrow_mut().decimal(text)?.ok_or_else(|| {
            self.invalid_value(value, "finite exact decimal string")
                .into()
        })
    }

    pub(super) fn crs(&self, value: &Value) -> Result<purrdf_geo_kernel::Crs, GeoCallError> {
        let iri = value
            .as_str()
            .ok_or_else(|| self.invalid_type(value, "absolute coordinate reference IRI"))?;
        self.crs_text(iri)
    }

    pub(super) fn crs_text(&self, iri: &str) -> Result<purrdf_geo_kernel::Crs, GeoCallError> {
        self.admission.borrow_mut().crs(iri).map_err(Into::into)
    }

    pub(super) fn location(
        &self,
        longitude: Rat,
        latitude: Rat,
    ) -> Result<purrdf_geo_kernel::LonLat, GeoCallError> {
        self.admission
            .borrow_mut()
            .location(longitude, latitude)
            .map_err(Into::into)
    }

    pub(super) fn binary_location(
        &self,
        longitude: f64,
        latitude: f64,
    ) -> Result<purrdf_geo_kernel::LonLat, GeoCallError> {
        let mut admission = self.admission.borrow_mut();
        let longitude = admission.binary64("longitude", longitude)?;
        let latitude = admission.binary64("latitude", latitude)?;
        admission.location(longitude, latitude).map_err(Into::into)
    }

    pub(super) fn closed_box(
        &self,
        west: Rat,
        south: Rat,
        east: Rat,
        north: Rat,
    ) -> Result<purrdf_geo_kernel::cells::ClosedBox, GeoCallError> {
        self.admission
            .borrow_mut()
            .closed_box(west, south, east, north)
            .map_err(Into::into)
    }

    pub(super) fn threshold(
        &self,
        lexical: &str,
        datatype: purrdf_xsd::XsdDatatype,
    ) -> Result<purrdf_geo_kernel::XsdDoubleMetres, GeoCallError> {
        purrdf_xsd::ieee::environment::check().map_err(GeoError::FloatEnvironment)?;
        let _scope = purrdf_xsd::ieee::Binary64Scope::enter();
        let promoted = self.storage.native(|memory| {
            let baseline = memory.admitted_bytes();
            let result = (|| {
                let parsed =
                    purrdf_xsd::value::try_parse_with_memory(lexical, datatype, false, memory)
                        .map_err(GeoError::Storage)?;
                let purrdf_xsd::value::ParsedValue::Value(value) = parsed else {
                    return Ok(None);
                };
                purrdf_xsd::numeric::numeric_to_f64_admitted(&value, &mut |layout| {
                    memory
                        .add_bytes(layout.required_bytes())
                        .map_err(purrdf_xsd::integer::LimbScratchError::Storage)
                })
                .map_err(GeoError::NumericalScratch)
            })();
            // Parsed coefficients and every native promotion temporary have
            // died. The output is one inline IEEE encoding, never an owner.
            let released = memory
                .admitted_bytes()
                .checked_sub(baseline)
                .ok_or(GeoError::Storage(StorageError::SizeOverflow))
                .and_then(|bytes| memory.release_bytes(bytes).map_err(GeoError::Storage));
            match result {
                Err(original) => Err(original),
                Ok(value) => released.map(|()| value),
            }
        })?;
        let value = promoted.ok_or_else(|| {
            self.custom(format_args!(
                "distance threshold must be a valid <{}> numeric value",
                datatype.iri()
            ))
        })?;
        purrdf_geo_kernel::XsdDoubleMetres::new(value).map_err(Into::into)
    }

    pub(super) fn literal(
        &self,
        literal: purrdf_core::RdfLiteral,
    ) -> Result<purrdf_core::TermValue, GeoCallError> {
        let result = self
            .storage
            .native(|memory| purrdf_core::TermValue::from_literal_with_memory(&literal, memory))
            .map_err(GeoError::Storage)
            .map_err(Into::into);
        let cleanup = self.storage.native(|memory| {
            memory.release_string(literal.lexical_form)?;
            if let Some(text) = literal.datatype {
                memory.release_string(text)?;
            }
            if let Some(text) = literal.language {
                memory.release_string(text)?;
            }
            Ok::<(), StorageError>(())
        });
        match result {
            Err(original) => Err(original),
            Ok(value) => cleanup
                .map(|()| value)
                .map_err(GeoError::Storage)
                .map_err(Into::into),
        }
    }

    pub(super) fn items<T, E: Within>(
        &self,
        value: &Value,
        read: impl FnMut(&Value) -> Result<T, E>,
    ) -> Result<Vec<T>, E> {
        self.storage.items(value, read)
    }

    pub(super) fn boxed<T>(&self, value: T) -> Result<Box<T>, GeoCallError> {
        self.storage
            .native(|memory| memory.boxed(value))
            .map_err(GeoError::Storage)
            .map_err(Into::into)
    }

    pub(super) fn shared<T>(
        &self,
        value: T,
    ) -> Result<purrdf_lex::allocation::Shared<T>, GeoCallError> {
        self.storage
            .native(|memory| {
                memory
                    .add_bytes(purrdf_lex::allocation::Shared::<T>::allocation_layout().size())?;
                purrdf_lex::allocation::Shared::try_new(value)
                    .map_err(|_| StorageError::AllocationFailed)
            })
            .map_err(GeoError::Storage)
            .map_err(Into::into)
    }

    pub(super) fn custom(&self, message: impl std::fmt::Display) -> DecodeError {
        DecodeError::custom_native(message, Some(self.allocation))
    }

    pub(super) fn invalid_type(&self, value: &Value, expected: &str) -> DecodeError {
        DecodeError::invalid_type_native(value, expected, Some(self.allocation))
    }

    pub(super) fn invalid_value(&self, value: &Value, expected: &str) -> DecodeError {
        DecodeError::invalid_value_native(value, expected, Some(self.allocation))
    }

    pub(super) fn unknown_variant(&self, value: &str, expected: &[&str]) -> DecodeError {
        DecodeError::unknown_variant_native(value, expected, Some(self.allocation))
    }

    pub(super) fn receipt(&self) -> Result<PreparationBudget, GeoError> {
        let receipt = self.admission.borrow().receipt();
        let mut budget = PreparationBudget::new(self.admission.borrow().policy());
        budget.retain(receipt.work_items(), receipt.workspace_bytes())?;
        Ok(budget)
    }
}

struct Progress<'a, 'observer, 'storage, 'account>(
    &'a RefCell<InputAdmission<'observer, 'storage, 'account>>,
);
impl json::ReadObserver for Progress<'_, '_, '_, '_> {
    fn advance(&mut self, bytes: usize) -> bool {
        json::ReadObserver::advance(&mut *self.0.borrow_mut(), bytes)
    }
}

/// Read the original document and construct its typed record under one account.
/// All JSON limits are lifted here; caller policy and actual checked buffer
/// layouts govern the read instead of an invented per-node resource ceiling.
pub(super) fn record<T>(
    text: &str,
    policy: ExecutionPolicy,
    observer: Option<&mut dyn MetricWorkObserver>,
    read: impl FnOnce(&Value, &Decode<'_, '_, '_, '_>) -> Result<T, GeoCallError>,
) -> Result<(T, PreparationBudget), GeoCallError> {
    record_outcome(text, policy, observer, read, |result, _decode| result)
}

/// Keep the original parse/result/error account alive through its native host
/// envelope. The same reader and typed body serve plain records and responses;
/// lexical refusals can be encoded without copying or detaching their payload.
pub(super) fn record_outcome<T, R>(
    text: &str,
    policy: ExecutionPolicy,
    observer: Option<&mut dyn MetricWorkObserver>,
    read: impl FnOnce(&Value, &Decode<'_, '_, '_, '_>) -> Result<T, GeoCallError>,
    finish: impl FnOnce(Result<T, GeoCallError>, &Decode<'_, '_, '_, '_>) -> Result<R, GeoCallError>,
) -> Result<(R, PreparationBudget), GeoCallError> {
    let mut capacity = Capacity {
        limit: policy.limits().max_workspace_bytes,
        admitted: 0,
        failure: None,
    };
    let result = (|| {
        let storage = CarrierStorage::new(&mut capacity);
        let admission =
            RefCell::new(InputAdmission::new(text.len(), policy, observer)?.with_storage(&storage));
        let allocation = storage.record_memory();
        let mut progress = Progress(&admission);
        let mut reader = json::Reader::new_observed(
            text,
            json::Limits {
                max_depth: usize::MAX,
                max_values: u64::MAX,
                max_string_bytes: usize::MAX,
                unique_members: false,
            },
            &mut progress,
        );
        let decoded = storage.native(|memory| {
            let read = reader
                .read_value_with_memory(memory)
                .and_then(|value| reader.finish_with_memory(memory).map(|()| value));
            let cleanup = reader.release_with_memory(memory);
            match read {
                Err(original) => Err(original),
                Ok(value) => cleanup
                    .map(|()| value)
                    .map_err(|error| json::Error::new(json::ErrorKind::Storage(error), text.len())),
            }
        });
        let interrupted = admission.borrow_mut().take_error();
        let result = match (interrupted, decoded) {
            (Some(original), decoded) => {
                drop(decoded);
                Err(original.into())
            }
            (None, Ok(value)) => {
                let parsed = exact_fields(&value, &storage, &admission)?;
                let decode = Decode {
                    storage: &storage,
                    allocation: &allocation,
                    admission: &admission,
                    parsed: &parsed,
                };
                finish(read(&value, &decode), &decode).and_then(|value| {
                    decode
                        .receipt()
                        .map(|budget| (value, budget))
                        .map_err(Into::into)
                })
            }
            (None, Err(error)) => {
                let error = match error.kind() {
                    json::ErrorKind::Storage(_)
                    | json::ErrorKind::Escape(
                        purrdf_lex::json_escape::JsonEscapeErrorKind::Storage(_),
                    ) => Err(DecodeError::from(error).into()),
                    _ => Err(DecodeError::custom_native(error, Some(&allocation)).into()),
                };
                let decode = Decode {
                    storage: &storage,
                    allocation: &allocation,
                    admission: &admission,
                    parsed: &[],
                };
                finish(error, &decode).and_then(|value| {
                    decode
                        .receipt()
                        .map(|budget| (value, budget))
                        .map_err(Into::into)
                })
            }
        };
        match admission.borrow_mut().take_error() {
            Some(original) => {
                drop(result);
                Err(original.into())
            }
            None => result,
        }
    })();
    match capacity.failure {
        Some(original) => {
            drop(result);
            Err(original.into())
        }
        None => result,
    }
}

/// The same typed body over a pre-existing caller-owned value. This door adds
/// no implicit profile ceiling to the resident public from_value contracts.
pub(super) fn value<T>(
    value: &Value,
    read: impl FnOnce(&Value, &Decode<'_, '_, '_, '_>) -> Result<T, GeoCallError>,
) -> Result<T, GeoCallError> {
    let mut capacity = Capacity {
        limit: u64::MAX,
        admitted: 0,
        failure: None,
    };
    let storage = CarrierStorage::new(&mut capacity);
    let admission = RefCell::new(
        InputAdmission::new(0, ExecutionPolicy::unbounded(), None)?.with_storage(&storage),
    );
    let allocation = storage.record_memory();
    let parsed = exact_fields(value, &storage, &admission)?;
    let decode = Decode {
        storage: &storage,
        allocation: &allocation,
        admission: &admission,
        parsed: &parsed,
    };
    read(value, &decode)
}

/// Construct the original typed output under caller-selected admission. The
/// ordinary returned Rust value is transferred to its caller; no resident
/// builder is used before the actual checked native allocations.
pub(super) fn produce<T>(
    policy: ExecutionPolicy,
    observer: Option<&mut dyn MetricWorkObserver>,
    body: impl FnOnce(&Decode<'_, '_, '_, '_>) -> Result<T, GeoCallError>,
) -> Result<T, GeoCallError> {
    let mut capacity = Capacity {
        limit: policy.limits().max_workspace_bytes,
        admitted: 0,
        failure: None,
    };
    let result = (|| {
        let storage = CarrierStorage::new(&mut capacity);
        let admission =
            RefCell::new(InputAdmission::new(0, policy, observer)?.with_storage(&storage));
        let allocation = storage.record_memory();
        let decode = Decode {
            storage: &storage,
            allocation: &allocation,
            admission: &admission,
            parsed: &[],
        };
        let result = body(&decode);
        let error = admission.borrow_mut().take_error();
        match error {
            Some(original) => {
                drop(result);
                Err(original.into())
            }
            None => result,
        }
    })();
    match capacity.failure {
        Some(original) => {
            drop(result);
            Err(original.into())
        }
        None => result,
    }
}

fn exact_fields<'v>(
    value: &'v Value,
    storage: &CarrierStorage<'_>,
    admission: &RefCell<InputAdmission<'_, '_, '_>>,
) -> Result<Vec<(&'v str, Rat)>, GeoCallError> {
    let mut pending = Vec::new();
    let mut parsed = Vec::new();
    storage
        .native(|memory| memory.push(&mut pending, (value, false)))
        .map_err(GeoError::Storage)?;
    while let Some((value, numeric)) = pending.pop() {
        admission.borrow_mut().admit_items(1)?;
        match value {
            Value::Array(items) => storage
                .native(|memory| {
                    memory.extend(
                        &mut pending,
                        items.iter().rev().map(|value| (value, numeric)),
                    )
                })
                .map_err(GeoError::Storage)?,
            Value::Object(fields) => storage
                .native(|memory| {
                    memory.extend(
                        &mut pending,
                        fields.iter().rev().map(|(field, value)| {
                            (
                                value,
                                super::decimal_field(
                                    field,
                                    fields.get("kind").and_then(Value::as_str),
                                    fields.get("encoding").and_then(Value::as_str),
                                ),
                            )
                        }),
                    )
                })
                .map_err(GeoError::Storage)?,
            Value::String(text) if numeric => {
                // Native arena construction precedes typed model compilation.
                // All lexical roles are the existing closed record vocabulary.
                if let Some(value) = admission.borrow_mut().decimal(text)? {
                    storage
                        .native(|memory| memory.push(&mut parsed, (text.as_str(), value)))
                        .map_err(GeoError::Storage)?;
                }
            }
            _ => {}
        }
    }
    storage
        .native(|memory| memory.release_vec(pending))
        .map_err(GeoError::Storage)?;
    parsed.sort_unstable_by_key(|(a, _)| *a);
    Ok(parsed)
}
