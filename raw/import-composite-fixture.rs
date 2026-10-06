// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//! One-time extraction of primary published composite example bytes.

use purrdf_testkit::vectors::Recorder;

fn main() {
    let path = std::env::args().nth(1).expect("pinned draft text path");
    let text = std::fs::read_to_string(path).unwrap();
    let start = text.find("Figure 9: ML-DSA-65-ES256").unwrap();
    let end = text[start..].find("Figure 10: ML-DSA-65-Ed25519").unwrap() + start;
    let mut literals = Vec::new();
    let mut current = None::<String>;
    for line in text[start..end].lines() {
        let line = line.trim();
        let rest = if let Some(position) = line.find("h'") {
            assert!(current.is_none());
            current = Some(String::new());
            &line[position + 2..]
        } else if current.is_some() {
            line
        } else {
            continue;
        };
        let hex = rest.split('\'').next().unwrap();
        if !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            current.as_mut().unwrap().push_str(hex);
        } else if !rest.starts_with('\'') {
            // Pagination headers are not hexadecimal byte content.
            continue;
        }
        if rest.contains('\'') {
            literals.push(current.take().unwrap());
        }
    }
    assert!(current.is_none());
    eprintln!(
        "literal byte lengths: {:?}",
        literals.iter().map(|s| s.len() / 2).collect::<Vec<_>>()
    );
    assert_eq!(
        literals.iter().map(|s| s.len() / 2).collect::<Vec<_>>(),
        [32, 32, 8, 1984, 64, 8, 0, 29, 127, 8, 29, 3373]
    );
    assert_eq!(literals[2], literals[5]);
    assert_eq!(literals[2], literals[9]);
    assert_eq!(literals[7], literals[10]);
    assert_eq!(literals[4], format!("{}{}", literals[0], literals[1]));
    let mut recorder = Recorder::new();
    recorder.comment("SPDX-FileCopyrightText: 2026 IETF Trust and the persons identified as document authors").unwrap();
    recorder
        .comment(
            "Pinned IETF JOSE/COSE composite signatures draft-04 Figure 10; published bytes only",
        )
        .unwrap();
    recorder
        .comment("SPDX-License-Identifier: BSD-3-Clause; see IETF-NOTICE.txt and PROVENANCE.md")
        .unwrap();
    recorder
        .header(
            "source",
            "https://www.ietf.org/archive/id/draft-ietf-jose-pq-composite-sigs-04.txt",
        )
        .unwrap();
    recorder
        .header(
            "fields",
            "mldsa-seed ed25519-seed kid public-key private-seeds payload representative signature",
        )
        .unwrap();
    recorder
        .record(&[
            &literals[0],
            &literals[1],
            &literals[2],
            &literals[3],
            &literals[4],
            &literals[7],
            &literals[8],
            &literals[11],
        ])
        .unwrap();
    std::fs::create_dir_all("crates/gts/tests/composite").unwrap();
    std::fs::write(
        "crates/gts/tests/composite/ietf-cose-example.txt",
        recorder.render(),
    )
    .unwrap();
}
