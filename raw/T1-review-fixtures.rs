// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Independent review of external expected answers only; never links purrdf-hash.
use std::{fs, io::Write, process::{Command, Stdio}};

fn decode(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0);
    text.as_bytes().chunks_exact(2).map(|pair| {
        u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()
    }).collect()
}

fn records(path: &str) -> Vec<(usize, usize, Vec<u8>)> {
    let text = fs::read_to_string(path).unwrap();
    text.lines().filter(|line| !line.starts_with('#') && !line.trim().is_empty()).map(|line| {
        let fields: Vec<_> = line.split_whitespace().collect();
        assert_eq!(fields.len(), 3);
        (fields[0].parse().unwrap(), fields[1].parse().unwrap(), decode(fields[2]))
    }).collect()
}

fn main() {
    let raw = ".stage/purrdf-gts-composite-ml-dsa-65-ed25519/raw";
    let vectors = "crates/hash-conformance/tests/vectors";
    let nist = records(&format!("{vectors}/shake_nist_vectors.txt"));
    assert_eq!(nist.len(), 4);
    for (strength, length, expected) in nist {
        let pdf = format!("{raw}/shake{strength}_msg{}.pdf", length * 8);
        let output = Command::new("pdftotext").args([&pdf, "-"]).output().unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        let last = text.rsplit_once("Output val is").unwrap().1;
        let oracle: Vec<_> = last.split_whitespace().map(|value| u8::from_str_radix(value, 16).unwrap()).collect();
        assert_eq!(oracle.len(), 512);
        assert_eq!(expected, oracle);
        println!("NIST SHAKE{strength} input={length}: all 512 output bytes match freshly extracted PDF");
    }
    let boundary = records(&format!("{vectors}/shake_boundary_vectors.txt"));
    assert_eq!(boundary.len(), 16);
    for (strength, length, expected) in boundary {
        let mut child = Command::new("openssl").args(["dgst", &format!("-shake{strength}"), "-xoflen", &expected.len().to_string(), "-binary"])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        child.stdin.take().unwrap().write_all(&vec![0xa3; length]).unwrap();
        let actual = child.wait_with_output().unwrap();
        assert!(actual.status.success());
        assert_eq!(expected, actual.stdout);
        println!("OpenSSL SHAKE{strength} input={length} output={}: every byte matches", expected.len());
    }
    println!("PASS: 4 complete NIST answers and all 16 OpenSSL answers independently verified");
}
