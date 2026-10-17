// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The same complete boundary artifact on native and actual WASM.

fn all_shared_dl_proof_vectors_match_the_current_native_boundary() {
    let cases = purrdf_validate::regime::dl_proof_golden_vectors().unwrap();
    assert_eq!(cases.len(), 16, "the complete service and schema inventory");
    purrdf_validate::regime::check_dl_proof_golden_vectors().unwrap();
}

purrdf_testkit::harness_main!(all_shared_dl_proof_vectors_match_the_current_native_boundary);
