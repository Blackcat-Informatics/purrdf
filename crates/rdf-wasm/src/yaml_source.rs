// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact YAML source bytes through admitted bridge storage and native snapshots.

use purrdf_core::DatasetHandle;
use purrdf_lex::allocation::{Memory, Resident, StorageError};
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

fn source_error(error: Error) -> JsValue {
    operation::native_diagnostic_to_js(error.into_diagnostic())
}

fn storage_error(error: StorageError) -> JsValue {
    source_error(YamlError::from(error).into())
}

fn capture(
    value: &JsValue,
    maximum: usize,
    resource: &'static str,
) -> Result<host::CapturedBytes, JsValue> {
    host::CapturedBytes::capture(value, maximum).map_err(|error| match error {
        host::ByteCaptureError::Host(error) => error,
        host::ByteCaptureError::Length => operation::coded_error(
            "YAML source buffer byte length exceeds the portable unsigned range",
            "yaml-source-byte-length",
        ),
        host::ByteCaptureError::Limit { maximum } => source_error(
            YamlError::Limit {
                resource,
                limit: maximum,
            }
            .into(),
        ),
        host::ByteCaptureError::Storage(error) => storage_error(error),
    })
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
    let profile_bytes = capture(profile_json, MAX_PROFILE_BYTES, "profile JSON bytes")?;
    let profile = yaml_source::profile(&profile_bytes.bytes).map_err(source_error)?;
    let source = capture(
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
    let profile_bytes = capture(profile_json, MAX_PROFILE_BYTES, "profile JSON bytes")?;
    let profile = yaml_source::profile(&profile_bytes.bytes).map_err(source_error)?;
    let snapshot = dataset
        .view()
        .freeze()
        .map_err(operation::native_diagnostic_to_js)?;
    let source =
        yaml_source::reconstruct(snapshot.as_ref(), root, &profile).map_err(source_error)?;
    host::owned_bytes(source.as_bytes())
}
