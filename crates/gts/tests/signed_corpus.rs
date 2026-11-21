// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The frozen signed-vector corpus, `vectors/signed/`, against the verifier.
//!
//! Every vector is a GTS file with two signed frames, the key and `kid` that
//! sign them, and the status the format owner expects with and without that key
//! (and the `kid` the verifier reports). The corpus is governed upstream in
//! `gmeow-gts` and is byte-frozen here; this test never edits it.
//!
//! [`EXPECTED_DIFFERENCES`] is the one escape hatch and it is held to XPASS
//! discipline: a listed vector must STILL disagree, so a difference cannot be
//! resolved upstream and left stale here, and an unlisted vector must agree.
//! Each entry names the reason the engine does not follow the vector.
//!
//! The run prints a `SIGNED-VECTORS:` scoreboard line before any assertion, so
//! a red run still reports what it measured.

use purrdf_ed25519::VerifyingKey;
use purrdf_gts::cose::verify_signatures;
use purrdf_gts::reader::read;

/// Vectors the engine knowingly does not follow, with the reason for each.
const EXPECTED_DIFFERENCES: &[(&str, &str)] = &[
    // The vectors place unrelated values under COSE labels 5, 6 and 7. RFC 9052
    // section 3.1 types label 5 and 6 as byte strings and does not define 7, and
    // the engine follows the RFC. Whether the format means to depart from it is
    // the format owner's decision, and the specification must say so first.
    (
        "sigalg-depth-8.json",
        "header labels 5 to 7 hold non-byte-string values; RFC 9052 types them",
    ),
    (
        "sigalg-header-null-bool-values.json",
        "header labels 5 to 7 hold non-byte-string values; RFC 9052 types them",
    ),
    (
        "sigalg-int-max-value.json",
        "header label 5 holds an integer; RFC 9052 types it as a byte string",
    ),
    (
        "sigalg-int-min-value.json",
        "header label 5 holds an integer; RFC 9052 types it as a byte string",
    ),
    (
        "sigalg-text-valid-multibyte.json",
        "header label 5 holds a text string; RFC 9052 types it as a byte string",
    ),
    // RFC 9052 section 3.1 makes `kid` optional, and the engine documents and
    // tests that an envelope without one verifies with a supplied key. The
    // specification does not require a `kid`; only the vector does.
    (
        "sigalg-missing-kid.json",
        "the specification does not require a kid and RFC 9052 makes it optional",
    ),
    // The specification does not order the signature-length check and key
    // resolution: the engine refuses a wrong-length EdDSA signature as invalid
    // before any key is looked up.
    (
        "sigalg-eddsa-short.json",
        "a short EdDSA signature is invalid before key resolution; the vector orders them the other way",
    ),
];

fn vectors_dir() -> std::path::PathBuf {
    purrdf_testkit::paths::workspace_root!()
        .join("vectors")
        .join("signed")
}

/// `None` when the engine agrees with the vector, else every disagreement.
fn disagreement(name: &str) -> Option<String> {
    let text = std::fs::read_to_string(vectors_dir().join(name)).expect("read vector");
    let json = purrdf_lex::json::read(&text).expect("vector is JSON");
    let hex = |key: &str| purrdf_hash::hex::decode(json[key].as_str().expect(key)).expect(key);
    let public: [u8; 32] = hex("pub").try_into().expect("32-byte key");
    let kid = json["kid"].as_str().expect("kid").to_owned();
    let data = hex("gts");
    let key = VerifyingKey::from_bytes(&public).expect("public key");
    let reported = json["reported_kid"].as_str();
    let (with_key, without_key) = if name == "basic.json" {
        ("valid", "unverified")
    } else {
        (
            json["status_with_key"].as_str().expect("status_with_key"),
            json["status_without_key"]
                .as_str()
                .expect("status_without_key"),
        )
    };
    let mut problems: Vec<String> = Vec::new();
    for (label, want, resolve_key) in [
        ("with key", with_key, true),
        ("without key", without_key, false),
    ] {
        let mut graph = read(&data, false, None);
        if graph.signatures.len() != 2 {
            problems.push(format!(
                "{label}: {} signatures, want 2",
                graph.signatures.len()
            ));
        }
        verify_signatures(&mut graph.signatures, |candidate| {
            (resolve_key && candidate == kid).then_some(key)
        });
        for signature in &graph.signatures {
            if signature.status != want {
                problems.push(format!("{label}: status {}, want {want}", signature.status));
            }
            if let Some(reported) = reported
                && signature.kid.as_deref() != Some(reported)
            {
                problems.push(format!("{label}: kid {:?}, want {reported}", signature.kid));
            }
        }
    }
    problems.dedup();
    (!problems.is_empty()).then(|| problems.join("; "))
}

/// Every carried signed vector agrees with the verifier except the listed
/// differences, and each listed difference still disagrees.
#[test]
fn the_signed_corpus_agrees_with_the_verifier_except_the_listed_differences() {
    let mut names: Vec<String> = std::fs::read_dir(vectors_dir())
        .expect("vectors/signed")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    names.sort();
    let results: Vec<(String, Option<String>)> = names
        .iter()
        .map(|name| (name.clone(), disagreement(name)))
        .collect();
    let agreed = results
        .iter()
        .filter(|(_, problem)| problem.is_none())
        .count();
    println!(
        "SIGNED-VECTORS: agreed {agreed} total {} diverging {}",
        results.len(),
        results.len() - agreed
    );
    assert_eq!(
        results.len(),
        69,
        "the carried signed corpus is exactly 69 vectors"
    );
    let listed = |name: &str| {
        EXPECTED_DIFFERENCES
            .iter()
            .any(|(listed, _)| *listed == name)
    };
    for (name, problem) in &results {
        match (problem, listed(name)) {
            (Some(problem), false) => {
                panic!("{name} disagrees and is not a listed difference: {problem}")
            }
            (None, true) => panic!("{name} now agrees: remove it from EXPECTED_DIFFERENCES"),
            _ => {}
        }
    }
    assert_eq!(
        results
            .iter()
            .filter(|(_, problem)| problem.is_some())
            .count(),
        EXPECTED_DIFFERENCES.len()
    );
}
