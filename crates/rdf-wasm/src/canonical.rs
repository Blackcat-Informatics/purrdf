// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit synchronous canonical-search requests and immutable receipt envelopes.

use purrdf_core::CanonicalizationControl;
use purrdf_validate::canonical::{execute, native_canonical_cause, parse_policy, receipts_json};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = Reflect, js_name = set)]
    fn set_property(target: &JsValue, key: &JsValue, value: &JsValue) -> Result<bool, JsValue>;
}

/// An original native cause that retains its immutable payload and admission owner.
#[wasm_bindgen]
#[derive(Debug)]
pub struct CanonicalCause {
    cause: purrdf_core::CanonError,
}

#[wasm_bindgen]
impl CanonicalCause {
    /// Exact fields read from the original owned native cause.
    #[wasm_bindgen(getter)]
    pub fn json(&self) -> Result<String, JsError> {
        purrdf_validate::native_error::to_json(&self.cause)
            .map(|value| value.to_string())
            .map_err(|error| JsError::new(&error.to_string()))
    }
}

impl From<purrdf_core::CanonError> for CanonicalCause {
    fn from(cause: purrdf_core::CanonError) -> Self {
        Self { cause }
    }
}

/// The original host value and this request's immutable execution-receipt JSON.
#[wasm_bindgen]
#[derive(Debug)]
pub struct CanonicalResult {
    value: JsValue,
    canonicalization_json: String,
}

#[wasm_bindgen]
impl CanonicalResult {
    /// The operation's original value. Canonical document contents are unchanged.
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> JsValue {
        self.value.clone()
    }

    /// Ordered per-run execution evidence, with exact decimal-string counters.
    #[wasm_bindgen(getter, js_name = canonicalizationJson)]
    pub fn canonicalization_json(&self) -> String {
        self.canonicalization_json.clone()
    }
}

impl CanonicalResult {
    pub(crate) const fn new(value: JsValue, canonicalization_json: String) -> Self {
        Self {
            value,
            canonicalization_json,
        }
    }
}

pub(crate) fn run<T, E>(
    policy_text: &str,
    cancel: Option<&crate::query::CancellationToken>,
    operation: impl FnOnce(&mut CanonicalizationControl<'_>) -> Result<T, E>,
) -> Result<(T, String), JsValue>
where
    E: std::error::Error + 'static,
{
    run_with(policy_text, cancel, operation, |error| {
        JsError::new(&error.to_string()).into()
    })
}

pub(crate) fn run_with<T, E>(
    policy_text: &str,
    cancel: Option<&crate::query::CancellationToken>,
    operation: impl FnOnce(&mut CanonicalizationControl<'_>) -> Result<T, E>,
    map_error: impl FnOnce(E) -> JsValue,
) -> Result<(T, String), JsValue>
where
    E: std::error::Error + 'static,
{
    let policy = parse_policy(policy_text).map_err(|error| error_to_js(&error, "[]"))?;
    let executed = execute(
        policy,
        cancel.map(|signal| signal as &dyn purrdf_core::StopSignal),
        operation,
    );
    let receipts = receipts_json(&executed.receipts).to_string();
    match executed.result {
        Ok(value) => Ok((value, receipts)),
        Err(error) => {
            let cause = native_canonical_cause(&error).cloned();
            Err(attach_evidence(map_error(error), &receipts, cause.as_ref()))
        }
    }
}

fn error_to_js(error: &(dyn std::error::Error + 'static), receipts: &str) -> JsValue {
    attach_evidence(
        JsError::new(&error.to_string()).into(),
        receipts,
        native_canonical_cause(error),
    )
}

pub(crate) fn attach_evidence(
    refused: JsValue,
    receipts: &str,
    cause: Option<&purrdf_core::CanonError>,
) -> JsValue {
    let attach = || -> Result<(), JsValue> {
        if !set_property(
            &refused,
            &JsValue::from_str("canonicalizationJson"),
            &JsValue::from_str(receipts),
        )? {
            return Err(JsError::new("cannot attach canonicalization execution evidence").into());
        }
        if let Some(cause) = cause {
            let fields = purrdf_validate::native_error::to_json(cause)
                .map_err(|failure| JsError::new(&failure.to_string()))?;
            if !set_property(
                &refused,
                &JsValue::from_str("nativeCauseJson"),
                &JsValue::from_str(&fields.to_string()),
            )? {
                return Err(JsError::new("cannot attach native canonicalization cause").into());
            }
            let original: JsValue = CanonicalCause {
                cause: cause.clone(),
            }
            .into();
            if !set_property(&refused, &JsValue::from_str("nativeCause"), &original)? {
                return Err(
                    JsError::new("cannot retain original native canonicalization cause").into(),
                );
            }
        }
        Ok(())
    };
    match attach() {
        Ok(()) => refused,
        Err(failure) => failure,
    }
}
