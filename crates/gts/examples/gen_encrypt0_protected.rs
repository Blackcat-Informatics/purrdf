// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Regenerate the caller protected-header COSE_Encrypt0 vectors.

use purrdf_gts::cose::{Encrypt0Error, encrypt0_with_protected};
use purrdf_gts::wire;
use purrdf_lex::{
    cbor::Value,
    json::{self, Object},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::array::from_fn(|index| u8::try_from(index).expect("32-byte key index"));
    let kid = "did:court";
    let plaintext = b"verified id record";
    let cases = [
        (
            "integer",
            vec![(Value::from(-70_000_i64), Value::Bytes(vec![0, 1, 255]))],
        ),
        (
            "text",
            vec![(Value::from("object"), Value::from("urn:sealed:α"))],
        ),
        ("alg-collision", vec![(Value::from(1), Value::from(3))]),
    ];
    for (index, (name, extra)) in cases.into_iter().enumerate() {
        // Separate deterministic fixture nonces under the same fixed key.
        let iv = [u8::try_from(0xb0 + index).expect("three fixture nonces"); 12];
        let mut record = Object::new()
            .with(
                "extra_protected",
                purrdf_hash::hex::encode(&wire::canonical(&Value::Map(extra.clone()))),
            )
            .with("iv", purrdf_hash::hex::encode(&iv))
            .with("key", purrdf_hash::hex::encode(&key))
            .with("kid", kid)
            .with("plaintext", purrdf_hash::hex::encode(plaintext));
        match encrypt0_with_protected(plaintext, kid, &key, &iv, &extra) {
            Ok(cose) => {
                assert_ne!(name, "alg-collision", "algorithm override must fail");
                record = record.with("cose", purrdf_hash::hex::encode(&cose));
            }
            Err(Encrypt0Error::Malformed) if name == "alg-collision" => {
                record = record.with("error", "malformed");
            }
            Err(error) => return Err(error.into()),
        }
        let path =
            purrdf_testkit::paths::workspace_root!().join(format!("vectors/encrypt0/{name}.json"));
        std::fs::write(path, format!("{}\n", json::write_pretty(&record.into())))?;
    }
    Ok(())
}
