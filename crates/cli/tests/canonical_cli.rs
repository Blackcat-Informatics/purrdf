// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The CLI's explicit canonical-search policy, refusal evidence and output boundary.

mod support;

#[test]
fn finite_refusal_and_unlimited_neighbor_publish_separate_receipts_before_output() {
    let directory = purrdf_testkit::temp_dir!().expect("temporary directory");
    let evidence = directory.path().join("receipts.json");
    let evidence = evidence.to_str().expect("UTF-8 temporary path");
    let input = "_:a <http://example.org/p> _:b .\n_:b <http://example.org/p> _:a .\n";
    let refused = support::pipe(
        &[
            "--canonical-search",
            "0",
            "--canonicalization-receipts",
            evidence,
            "convert",
            "--from",
            "ntriples",
            "--canonical",
            "-",
            "-",
        ],
        input,
    );
    assert_eq!(
        refused.status.code(),
        Some(1),
        "{}",
        support::stderr(&refused)
    );
    assert!(
        refused.stdout.is_empty(),
        "a refused request publishes no canonical bytes"
    );
    let finite = std::fs::read_to_string(evidence).expect("refusal receipts");
    assert!(finite.contains("\"finite\":\"0\""), "{finite}");
    let accepted = support::pipe(
        &[
            "--canonical-search",
            "unlimited",
            "--canonicalization-receipts",
            evidence,
            "convert",
            "--from",
            "ntriples",
            "--canonical",
            "-",
            "-",
        ],
        input,
    );
    assert!(accepted.status.success(), "{}", support::stderr(&accepted));
    assert!(support::stdout(&accepted).starts_with("_:c14n"));
    let unlimited = std::fs::read_to_string(evidence).expect("success receipts");
    assert!(
        unlimited.contains("unlimited") && unlimited.contains("\"completed\":true"),
        "{unlimited}"
    );
    assert_ne!(finite, unlimited, "evidence belongs to one request");
    let directory_destination = directory.path().to_str().expect("UTF-8 temporary path");
    let failed_evidence = support::pipe(
        &[
            "--canonical-search",
            "0",
            "--canonicalization-receipts",
            directory_destination,
            "convert",
            "--from",
            "ntriples",
            "--canonical",
            "-",
            "-",
        ],
        input,
    );
    assert_eq!(failed_evidence.status.code(), Some(1));
    assert_eq!(failed_evidence.stdout, [] as [u8; 0]);
    let diagnostic = support::stderr(&failed_evidence);
    assert!(
        diagnostic.contains("cannot publish --canonicalization-receipts"),
        "{diagnostic}"
    );
    assert!(
        diagnostic.starts_with(support::stderr(&refused).trim()),
        "original refusal retained: {diagnostic}"
    );
}

#[test]
fn receipt_alias_is_refused_before_reading_or_truncating_input() {
    let directory = purrdf_testkit::temp_dir!().expect("temporary directory");
    let input = directory.path().join("source.nt");
    let original = b"_:a <http://example.org/p> _:a .\n";
    std::fs::write(&input, original).expect("source fixture");
    let path = input.to_str().expect("UTF-8 temporary path");
    let result = support::run(&[
        "--canonical-search",
        "unlimited",
        "--canonicalization-receipts",
        path,
        "convert",
        "--canonical",
        path,
        "-",
    ]);
    assert_eq!(
        result.status.code(),
        Some(2),
        "{}",
        support::stderr(&result)
    );
    assert!(support::stderr(&result).contains("aliases"));
    assert_eq!(result.stdout, [] as [u8; 0]);
    assert_eq!(std::fs::read(&input).expect("source preserved"), original);
}
