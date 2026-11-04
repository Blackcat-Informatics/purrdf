// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact YAML source bytes through admitted bridge storage and native snapshots.

use purrdf_core::{DatasetHandle, RdfDiagnostic};
use purrdf_lex::allocation::{Ceiling, Memory, Resident, StorageError};
use purrdf_validate::yaml_source::{self, Error, MAX_PROFILE_BYTES, YamlError};
use wasm_bindgen::prelude::*;

use crate::{dataset::Dataset, host, operation};

/// A native export root and a dataset that can be moved out once.
#[wasm_bindgen]
#[derive(Debug)]
pub struct YamlSourceExport {
    root: String,
    dataset: Option<Dataset>,
}

#[wasm_bindgen]
impl YamlSourceExport {
    /// The original computed source-carrier root.
    #[wasm_bindgen(getter)]
    pub fn root(&self) -> String {
        self.root.clone()
    }

    /// Move the dataset out once; freeing this envelope does not free that dataset.
    #[wasm_bindgen(js_name = takeDataset)]
    pub fn take_dataset(&mut self) -> Option<Dataset> {
        self.dataset.take()
    }
}

/// The original diagnostic and native cause, retaining their native admission.
#[wasm_bindgen]
#[derive(Debug)]
pub struct YamlSourceCause {
    diagnostic: RdfDiagnostic,
}

#[wasm_bindgen]
impl YamlSourceCause {
    /// Exact typed fields inspected from the original cause, without a host enum.
    #[wasm_bindgen(getter)]
    pub fn json(&self) -> Result<String, JsError> {
        let cause = self
            .diagnostic
            .native_cause()
            .ok_or_else(|| JsError::new("YAML source diagnostic has no native cause"))?;
        purrdf_validate::native_error::to_json(cause)
            .map(|value| value.to_string())
            .map_err(|error| JsError::new(&error.to_string()))
    }
}

fn diagnostic_to_js(diagnostic: RdfDiagnostic) -> JsValue {
    let attach = || -> Result<JsValue, JsValue> {
        let copy = diagnostic
            .clone_with_memory(&mut Memory::new(&mut Resident))
            .map_err(|error| operation::diagnostic_to_js(error.into()))?;
        let refused = operation::diagnostic_to_js(copy);
        if let Some(cause) = diagnostic.native_cause() {
            let fields = purrdf_validate::native_error::to_json(cause)
                .map_err(|error| JsError::new(&error.to_string()))?;
            if !host::reflect_set(
                &refused,
                &JsValue::from_str("nativeCauseJson"),
                &JsValue::from_str(&fields.to_string()),
            )? {
                return Err(JsError::new("cannot attach native YAML source cause fields").into());
            }
            let original: JsValue = YamlSourceCause { diagnostic }.into();
            if !host::reflect_set(&refused, &JsValue::from_str("nativeCause"), &original)? {
                return Err(JsError::new("cannot retain original native YAML source cause").into());
            }
        }
        Ok(refused)
    };
    attach().unwrap_or_else(core::convert::identity)
}

fn source_error(error: Error) -> JsValue {
    diagnostic_to_js(error.into_diagnostic())
}

fn storage_error(error: StorageError) -> JsValue {
    source_error(YamlError::from(error).into())
}

/// A separate bridge allocation, retained throughout the native operation.
struct BridgeBytes {
    bytes: Vec<u8>,
    _admission: Ceiling,
}

impl BridgeBytes {
    fn capture(value: &JsValue, maximum: usize, resource: &'static str) -> Result<Self, JsValue> {
        let captured = host::capture_bytes(value)?;
        let length = host::byte_length(&captured)?;
        // Never accept an i32 ABI coercion of the host's possibly wider length.
        // Every profile bound is portable u32; layout checks remain separate.
        if !length.is_finite()
            || length.fract() != 0.0
            || length < 0.0
            || length > f64::from(u32::MAX)
        {
            return Err(operation::coded_error(
                "YAML source buffer byte length exceeds the portable unsigned range",
                "yaml-source-byte-length",
            ));
        }
        // The finite, integral, nonnegative length is already bounded by u32::MAX.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let length = length as u32 as usize;
        if length > maximum {
            return Err(source_error(
                YamlError::Limit {
                    resource,
                    limit: maximum,
                }
                .into(),
            ));
        }
        let mut admission = Ceiling(maximum);
        let mut bytes = Vec::new();
        Memory::new(&mut admission)
            .reserve(&mut bytes, length)
            .map_err(storage_error)?;
        bytes.resize(length, 0);
        host::copy_bytes(&captured, &mut bytes)?;
        Ok(Self {
            bytes,
            _admission: admission,
        })
    }
}

/// Export original bytes under an explicit source IRI and UTF-8 profile JSON.
///
/// Profile admission and decoding precede source capture. The buffers cross as
/// handles; only admitted, fallibly allocated native storage receives a copy.
/// Host arrays/strings, bridge copies, native phases and dataset publication have
/// separate accounts. Shared buffers are refused. String-glue allocation is
/// outside bridge admission.
///
/// # Errors
/// Throws host buffer refusals or the original typed native diagnostic.
#[wasm_bindgen(js_name = yamlSourceExport)]
pub fn yaml_source_export(
    source: &JsValue,
    source_iri: &str,
    profile_json: &JsValue,
) -> Result<YamlSourceExport, JsValue> {
    let profile_bytes =
        BridgeBytes::capture(profile_json, MAX_PROFILE_BYTES, "profile JSON bytes")?;
    let profile = yaml_source::profile(&profile_bytes.bytes).map_err(source_error)?;
    let source = BridgeBytes::capture(
        source,
        profile.bounds().syntax.syntax.max_input_bytes,
        "source bytes",
    )?;
    let exported =
        yaml_source::export(source_iri, &source.bytes, &profile).map_err(source_error)?;
    let mut owner = Resident;
    Memory::new(&mut owner)
        .add_bytes(DatasetHandle::allocation_layout::<Resident>().size())
        .map_err(storage_error)?;
    let dataset = DatasetHandle::try_from_admitted(exported.dataset, owner)
        .map_err(|_| storage_error(StorageError::AllocationFailed))?;
    Ok(YamlSourceExport {
        root: exported.root,
        dataset: Some(Dataset::from_frozen(dataset)?),
    })
}

/// Reconstruct a current native snapshot into independently owned original bytes.
///
/// The profile bridge, mutable-dataset snapshot and native decode/reparse phases
/// remain separate accounts. The result is copied into a host-owned Uint8Array.
///
/// # Errors
/// Throws host buffer, native snapshot or verified reconstruction refusals.
#[wasm_bindgen(js_name = yamlSourceReconstruct)]
pub fn yaml_source_reconstruct(
    dataset: &Dataset,
    root: &str,
    profile_json: &JsValue,
) -> Result<JsValue, JsValue> {
    let profile_bytes =
        BridgeBytes::capture(profile_json, MAX_PROFILE_BYTES, "profile JSON bytes")?;
    let profile = yaml_source::profile(&profile_bytes.bytes).map_err(source_error)?;
    let snapshot = dataset.view().freeze().map_err(diagnostic_to_js)?;
    let source =
        yaml_source::reconstruct(snapshot.as_ref(), root, &profile).map_err(source_error)?;
    host::owned_bytes(source.as_bytes())
}
