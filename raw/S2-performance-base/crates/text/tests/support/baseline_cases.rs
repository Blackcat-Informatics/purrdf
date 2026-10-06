// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete baseline artifact cases for native Rust conformance.
//! These bytes are test fixtures only; shipping runtime libraries embed no lexicon.

use purrdf_text::segment::{Dictionary, baseline};
use purrdf_text::{Analyzer, AnalyzerProfile};

/// Validate every complete artifact and the explicit standard-profile resolver.
pub(crate) fn baseline_artifact_resolution() {
    let artifacts: &[(&[u8], &str, &str, usize)] = &[
        (
            include_bytes!(
                "../../lexicons/artifacts/cjdict-64998fa3ecf9f4ed8fa230a7a4e632890bdd7e6dd1620d922c68c5aeee4c715f.cbor"
            ),
            baseline::CJDICT_PHYSICAL,
            baseline::CJDICT_SEMANTIC,
            315_956,
        ),
        (
            include_bytes!(
                "../../lexicons/artifacts/thaidict-193fbd2c750ff0404c2aa0c23ca18023f09e70e8fd35521912182b4457674ec0.cbor"
            ),
            baseline::THAIDICT_PHYSICAL,
            baseline::THAIDICT_SEMANTIC,
            26_383,
        ),
        (
            include_bytes!(
                "../../lexicons/artifacts/laodict-6ac62b45dfe409604fd2518ea7c703061ae8f2d947a5fae228417c80fecc999f.cbor"
            ),
            baseline::LAODICT_PHYSICAL,
            baseline::LAODICT_SEMANTIC,
            30_505,
        ),
        (
            include_bytes!(
                "../../lexicons/artifacts/khmerdict-90710c9a661c3b43c13a6ecdc7c56661b2864ba04ae9097ea49ed09c43512e4b.cbor"
            ),
            baseline::KHMERDICT_PHYSICAL,
            baseline::KHMERDICT_SEMANTIC,
            81_025,
        ),
        (
            include_bytes!(
                "../../lexicons/artifacts/burmesedict-f0fdb5352e8ea4fc11d41140258a6e269baf330510692f07e244a819f77f368a.cbor"
            ),
            baseline::BURMESEDICT_PHYSICAL,
            baseline::BURMESEDICT_SEMANTIC,
            41_120,
        ),
    ];
    for &(bytes, physical, semantic, count) in artifacts {
        let id = purrdf_hash::hex::decode_32(physical).expect("physical identity");
        let dictionary = Dictionary::from_artifact(id, bytes).expect("full artifact");
        assert_eq!(dictionary.len(), count);
        assert_eq!(
            purrdf_hash::hex::encode(&dictionary.fingerprint()),
            semantic
        );
        assert_eq!(dictionary.artifact_bytes(), bytes);
    }
    let bytes: Vec<_> = artifacts.iter().map(|&(bytes, _, _, _)| bytes).collect();
    let standard =
        Analyzer::resolve(AnalyzerProfile::standard(), &bytes).expect("all five artifacts");
    assert_eq!(standard.terms("中文").expect("query"), ["中文"]);
    let reversed: Vec<_> = bytes.iter().rev().copied().collect();
    assert_eq!(
        standard,
        Analyzer::resolve(AnalyzerProfile::standard(), &reversed).expect("order independent")
    );
    assert!(Analyzer::resolve(AnalyzerProfile::standard(), &bytes[..4]).is_err());
    let mut duplicated = bytes;
    duplicated.push(duplicated[0]);
    assert!(Analyzer::resolve(AnalyzerProfile::standard(), &duplicated).is_err());
    assert!(Analyzer::with_profile(AnalyzerProfile::standard()).is_err());
}
