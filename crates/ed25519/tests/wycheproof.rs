// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Project Wycheproof's Ed25519 verification vectors (vendored byte-frozen in
//! `vectors/wycheproof/`, see its `PROVENANCE.md` for the pinned commit and
//! SHA-256), every group and every case.
//!
//! * `valid`: [`VerifyingKey::verify_strict`] must accept.
//! * `invalid`: it must refuse. A signature that is not 64 bytes is refused by
//!   the length check of `Signature::try_from` before verification runs.
//! * `acceptable`: the pinned file contains none. A re-vendor that introduces
//!   one fails this test until the crate's chosen behaviour for its flags is
//!   written down here: the crate is strict, so an `acceptable` case would be
//!   asserted refused (non-canonical S, non-canonical R or A, small-order A or
//!   R are all refused by design).
//!
//! The upstream file carries public keys only (no `key.sk`), so it grades the
//! verify side; the sign side is covered by the RFC 8032 vectors and the frozen
//! differential.

use purrdf_ed25519::{Signature, VerifyingKey};
use purrdf_lex::json;
use purrdf_testkit::paths::workspace_root;

fn hex_field(text: &str) -> Vec<u8> {
    purrdf_hash::hex::decode(text).expect("hex field")
}

fn accepts(key: &[u8], message: &[u8], signature: &[u8]) -> bool {
    let Ok(key) = VerifyingKey::try_from(key) else {
        return false;
    };
    let Ok(signature) = Signature::try_from(signature) else {
        return false;
    };
    key.verify_strict(message, &signature).is_ok()
}

#[test]
fn every_wycheproof_ed25519_case() {
    let path = workspace_root().join("vectors/wycheproof/ed25519_test.json");
    let text = std::fs::read_to_string(&path).expect("vendored Wycheproof file");
    let document = json::read(&text).expect("well-formed JSON");

    let declared = document["numberOfTests"]
        .as_number()
        .unwrap()
        .as_u64()
        .unwrap();
    let groups = document["testGroups"].as_array().expect("testGroups");

    let (mut total, mut valid, mut invalid) = (0u64, 0u64, 0u64);
    let mut flags_seen = std::collections::BTreeMap::<String, u64>::new();
    let mut failures = Vec::new();

    for group in groups {
        let key = hex_field(group["publicKey"]["pk"].as_str().expect("pk"));
        for case in group["tests"].as_array().expect("tests") {
            let id = case["tcId"].as_number().unwrap().as_u64().unwrap();
            let message = hex_field(case["msg"].as_str().expect("msg"));
            let signature = hex_field(case["sig"].as_str().expect("sig"));
            let got = accepts(&key, &message, &signature);
            let result = case["result"].as_str().expect("result");
            for flag in case["flags"].as_array().expect("flags") {
                *flags_seen
                    .entry(flag.as_str().expect("flag").to_owned())
                    .or_default() += 1;
            }
            total += 1;
            let want = match result {
                "valid" => {
                    valid += 1;
                    true
                }
                "invalid" => {
                    invalid += 1;
                    false
                }
                other => panic!(
                    "tcId {id}: unexpected result {other:?}; state the strict policy for its flags"
                ),
            };
            if got != want {
                failures.push(format!(
                    "tcId {id} ({}): want {result}, got accept={got}",
                    case["comment"].as_str().unwrap_or("")
                ));
            }
        }
    }

    println!(
        "wycheproof ed25519: {} groups, {total} cases ({valid} valid, {invalid} invalid); flags {flags_seen:?}",
        groups.len()
    );
    assert!(
        failures.is_empty(),
        "{} failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(total, declared, "numberOfTests");
    assert_eq!((groups.len(), total, valid, invalid), (78, 151, 88, 63));
}
