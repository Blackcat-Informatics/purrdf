// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The additive complete-state identity is reachable through the umbrella facade.

#![cfg(not(target_arch = "wasm32"))]

use std::convert::Infallible;

use purrdf::{DatasetStateDigest, DatasetStateError, RdfDatasetBuilder};

#[test]
fn complete_state_identity_and_typed_refusal_are_public() {
    let dataset = RdfDatasetBuilder::new()
        .freeze()
        .expect("the empty dataset is valid");
    let result: Result<DatasetStateDigest, DatasetStateError<Infallible, ()>> =
        DatasetStateDigest::from_view(&dataset);
    let digest = result.expect("the complete empty dataset is admitted");
    assert_eq!(digest.as_bytes().len(), 32);
    assert_eq!(digest.to_string(), digest.to_hex());
}
